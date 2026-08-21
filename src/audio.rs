use anyhow::{anyhow, Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, SampleRate, StreamConfig, SupportedStreamConfig};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, SyncSender, TrySendError};
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub struct AudioInfo {
    pub device_name: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub requested_sample_rate: u32,
}

fn handled_format(format: SampleFormat) -> bool {
    matches!(format, SampleFormat::F32 | SampleFormat::I16 | SampleFormat::U16)
}

fn choose_config(device: &cpal::Device, requested_sample_rate: u32) -> Result<SupportedStreamConfig> {
    let default = device.default_input_config()?;
    let mut best = None;
    let mut best_score = 0u8;

    match device.supported_input_configs() {
        Ok(configs) => {
            for range in configs {
                if !handled_format(range.sample_format())
                    || requested_sample_rate < range.min_sample_rate().0
                    || requested_sample_rate > range.max_sample_rate().0
                {
                    continue;
                }

                let score = u8::from(range.sample_format() == default.sample_format()) * 2
                    + u8::from(range.channels() == default.channels());

                if best.is_none() || score > best_score {
                    best = Some(range.with_sample_rate(SampleRate(requested_sample_rate)));
                    best_score = score;
                }
            }
        }
        Err(_) if handled_format(default.sample_format()) => return Ok(default),
        Err(error) => return Err(error.into()),
    }

    if let Some(config) = best {
        return Ok(config);
    }

    if handled_format(default.sample_format()) {
        return Ok(default);
    }

    for range in device.supported_input_configs()? {
        if !handled_format(range.sample_format()) {
            continue;
        }

        let default_rate = default.sample_rate().0;
        let rate = default_rate
            .max(range.min_sample_rate().0)
            .min(range.max_sample_rate().0);
        return Ok(range.with_sample_rate(SampleRate(rate)));
    }

    Err(anyhow!("input device has no supported PCM format"))
}

fn send_loudest_channel<T, F>(
    data: &[T],
    channels: usize,
    selected_channel: &mut Option<usize>,
    tx: &SyncSender<Vec<f32>>,
    convert: F,
) where
    T: Copy,
    F: Fn(T) -> f32 + Copy,
{
    if data.is_empty() || channels == 0 {
        return;
    }

    let mut energy = vec![0.0f64; channels];

    for (index, sample) in data.iter().copied().enumerate() {
        let value = convert(sample);
        let channel = index % channels;
        energy[channel] += f64::from(value * value);
    }

    let strongest = energy
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.total_cmp(b.1))
        .map(|(index, _)| index)
        .unwrap_or(0);
    let selected = match *selected_channel {
        Some(current) if energy[current] >= energy[strongest] * 0.1 => current,
        _ => strongest,
    };
    *selected_channel = Some(selected);

    let mut mono = Vec::with_capacity(data.len() / channels + 1);
    mono.extend(
        data.iter()
            .copied()
            .skip(selected)
            .step_by(channels)
            .map(convert),
    );

    match tx.try_send(mono) {
        Ok(()) | Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => {}
    }
}

pub fn run_audio_capture<F, R>(
    mut process_data: F,
    on_ready: R,
    requested_sample_rate: u32,
    running: Arc<AtomicBool>,
) -> Result<()>
where
    F: FnMut(&[f32], f32) + Send + 'static,
    R: FnOnce(AudioInfo) + Send + 'static,
{
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| anyhow!("no input device is available"))?;
    let device_name = device.name().unwrap_or_else(|_| "Unknown input".to_string());
    let supported_config = choose_config(&device, requested_sample_rate)?;
    let sample_format = supported_config.sample_format();
    let sample_rate = supported_config.sample_rate().0;
    let channels = supported_config.channels();
    let config: StreamConfig = supported_config.into();
    let (audio_tx, audio_rx) = mpsc::sync_channel::<Vec<f32>>(4);
    let (error_tx, error_rx) = mpsc::channel::<String>();
    let channel_count = usize::from(channels);

    let stream = match sample_format {
        SampleFormat::F32 => {
            let tx = audio_tx.clone();
            let error_tx = error_tx.clone();
            let mut selected_channel = None;
            device.build_input_stream(
                &config,
                move |data: &[f32], _| {
                    send_loudest_channel(data, channel_count, &mut selected_channel, &tx, |x| x)
                },
                move |error| {
                    let _ = error_tx.send(error.to_string());
                },
                None,
            )?
        }
        SampleFormat::I16 => {
            let tx = audio_tx.clone();
            let error_tx = error_tx.clone();
            let mut selected_channel = None;
            device.build_input_stream(
                &config,
                move |data: &[i16], _| {
                    send_loudest_channel(
                        data,
                        channel_count,
                        &mut selected_channel,
                        &tx,
                        |x| x as f32 / 32768.0,
                    )
                },
                move |error| {
                    let _ = error_tx.send(error.to_string());
                },
                None,
            )?
        }
        SampleFormat::U16 => {
            let tx = audio_tx.clone();
            let error_tx = error_tx.clone();
            let mut selected_channel = None;
            device.build_input_stream(
                &config,
                move |data: &[u16], _| {
                    send_loudest_channel(
                        data,
                        channel_count,
                        &mut selected_channel,
                        &tx,
                        |x| (x as f32 - 32768.0) / 32768.0,
                    )
                },
                move |error| {
                    let _ = error_tx.send(error.to_string());
                },
                None,
            )?
        }
        _ => return Err(anyhow!("unsupported input sample format: {sample_format:?}")),
    };

    drop(audio_tx);
    drop(error_tx);
    stream.play().context("failed to start the input stream")?;

    on_ready(AudioInfo {
        device_name,
        sample_rate,
        channels,
        requested_sample_rate,
    });

    while running.load(Ordering::Acquire) {
        if let Ok(error) = error_rx.try_recv() {
            return Err(anyhow!("audio stream error: {error}"));
        }

        match audio_rx.recv_timeout(Duration::from_millis(50)) {
            Ok(data) => process_data(&data, sample_rate as f32),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                return Err(anyhow!("audio input stream stopped unexpectedly"));
            }
        }
    }

    Ok(())
}
