mod audio;
mod cli;
mod dsp;
mod graphics;
mod tuner;

use cli::parse_args;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::error::Error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

fn main() -> Result<(), Box<dyn Error>> {
    graphics::init_terminal()?;

    let config = parse_args();
    println!(
        "Starting tuner with configured sample rate: {} and tolerance: {} cents",
        config.sample_rate, config.tolerance
    );
    let cli_sample_rate = config.sample_rate;

    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();

    let audio_handle = std::thread::spawn(move || {
        if let Err(e) =
            audio::start_audio_capture_with_callback(
                move |data, sample_rate| {
                    if data.is_empty() {
                        return;
                    }
                    // Calculate RMS amplitude.
                    let rms: f32 = (data.iter().map(|&x| x * x).sum::<f32>() / data.len() as f32)
                        .sqrt();
                    const RMS_THRESHOLD: f32 = 0.001;
                    if rms < RMS_THRESHOLD {
                        let _ = graphics::update_message("Signal too weak...");
                        return;
                    }
                    let magnitudes = dsp::compute_fft(data);
                    let (max_index, max_magnitude) = magnitudes
                        .iter()
                        .enumerate()
                        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                        .unwrap();
                    const FFT_MAG_THRESHOLD: f32 = 0.001;
                    if *max_magnitude < FFT_MAG_THRESHOLD {
                        let _ = graphics::update_message("No convincing dominant frequency...");
                        return;
                    }
                    let fft_size = data.len() as f32;
                    let dominant_freq = (max_index as f32) * (sample_rate / fft_size);
                    if let Some((note, cents)) = tuner::detect_note(dominant_freq) {
                        let _ = graphics::update_tuning(&note, cents);
                    } else {
                        let _ = graphics::update_message("No valid note detected");
                    }
                },
                cli_sample_rate,
            ) {
            eprintln!("Audio error: {}", e);
        }
        running_clone.store(false, Ordering::SeqCst);
    });

    while running.load(Ordering::SeqCst) {
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(KeyEvent { code, modifiers, .. }) = event::read()? {
                if (code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL))
                    || code == KeyCode::Esc
                    || (code == KeyCode::Char('q'))
                {
                    running.store(false, Ordering::SeqCst);
                }
            }
        }
    }

    audio_handle.join().unwrap();

    graphics::restore_terminal()?;
    println!("Exiting...");
    Ok(())
}
