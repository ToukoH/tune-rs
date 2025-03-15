mod audio;
mod cli;
mod dsp;
mod tuner;
mod graphics;

use cli::parse_args;

fn main() -> Result<(), anyhow::Error> {
    let config = parse_args();
    println!(
        "Starting tuner with configured sample rate: {} and tolerance: {} cents",
        config.sample_rate, config.tolerance
    );

    let cli_sample_rate = config.sample_rate;

    audio::start_audio_capture_with_callback(
        move |data, sample_rate| {
            if data.is_empty() {
                return;
            }

            const RMS_THRESHOLD: f32 = 0.001;
            const FFT_MAG_THRESHOLD: f32 = 0.001;
            
            let rms: f32 = (data.iter().map(|&x| x * x).sum::<f32>() / data.len() as f32).sqrt();

            if rms < RMS_THRESHOLD {
                graphics::display_message("Play a note");
                return;
            }

            let magnitudes = dsp::compute_fft(data);
            let (max_index, max_magnitude) = magnitudes
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                .unwrap();

            if *max_magnitude < FFT_MAG_THRESHOLD {
                graphics::display_message("No convincing dominant frequency...");
                return;
            }

            let fft_size = data.len() as f32;
            let dominant_freq = (max_index as f32) * (sample_rate / fft_size);

            if let Some((note, cents)) = tuner::detect_note(dominant_freq) {
                graphics::display_tuning(&note, cents);
            } else {
                graphics::display_message("No valid note detected");
            }
        },
        cli_sample_rate,
    )?;

    Ok(())
}
