mod audio;
mod cli;
mod dsp;
mod tuner;

use cli::parse_args;

fn main() -> Result<(), anyhow::Error> {
    let config = parse_args();
    println!(
        "Starting tuner with configured sample rate: {} and tolerance: {} cents",
        config.sample_rate, config.tolerance
    );

    let tolerance = config.tolerance;
    let cli_sample_rate = config.sample_rate;

    audio::start_audio_capture_with_callback(
        move |data, sample_rate| {
            if data.is_empty() {
                return;
            }

            let magnitudes = dsp::compute_fft(data);
            let (max_index, _) = magnitudes
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                .unwrap();
            let fft_size = data.len() as f32;
            let dominant_freq = (max_index as f32) * (sample_rate / fft_size);

            if let Some((note, cents)) = tuner::detect_note(dominant_freq) {
                if cents.abs() <= tolerance {
                    println!(
                        "Detected note: {} is in tune (deviation: {:.2} cents)",
                        note, cents
                    );
                } else {
                    println!(
                        "Detected note: {} is out of tune (deviation: {:.2} cents)",
                        note, cents
                    );
                }
            } else {
                println!("No valid note detected");
            }
        },
        cli_sample_rate,
    )?;

    Ok(())
}
