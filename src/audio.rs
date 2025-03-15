use std::time::Duration;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig};

pub fn start_audio_capture_with_callback<F>(
    mut process_data: F,
    cli_sample_rate: f32,
) -> Result<(), anyhow::Error>
where
    F: FnMut(&[f32], f32) + Send + 'static,
{
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .expect("Failed to get input device");

    let default_config = device.default_input_config()?;
    let sample_format = default_config.sample_format();
    let mut config: StreamConfig = default_config.into();

    config.sample_rate = cpal::SampleRate(cli_sample_rate as u32);
    let sample_rate = cli_sample_rate;

    println!("Input device: {}", device.name()?);
    println!("Input config: {:?}", config);

    let err_fn = |err| eprintln!("An error occurred on the audio stream: {}", err);

    let stream = match sample_format {
        SampleFormat::F32 => device.build_input_stream(
            &config,
            move |data: &[f32], _| {
                process_data(data, sample_rate);
            },
            err_fn,
            None,
        )?,
        SampleFormat::I16 => device.build_input_stream(
            &config,
            move |data: &[i16], _| {
                let data_f32: Vec<f32> =
                    data.iter().map(|&x| x as f32 / 32768.0).collect();
                process_data(&data_f32, sample_rate);
            },
            err_fn,
            None,
        )?,
        SampleFormat::U16 => device.build_input_stream(
            &config,
            move |data: &[u16], _| {
                let data_f32: Vec<f32> = data
                    .iter()
                    .map(|&x| (x as f32 - 32768.0) / 32768.0)
                    .collect();
                process_data(&data_f32, sample_rate);
            },
            err_fn,
            None,
        )?,
        _ => return Err(anyhow::anyhow!("Unsupported sample format")),
    };

    stream.play()?;
    println!("Streaming");
    loop {
        std::thread::sleep(Duration::from_secs(1));
    }
}
