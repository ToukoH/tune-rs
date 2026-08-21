mod audio;
mod cli;
mod dsp;
mod graphics;
mod tuner;

use cli::parse_args;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use dsp::{PitchDetector, PitchState};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, TryRecvError};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tuner::PitchSmoother;

enum UiEvent {
    Ready(audio::AudioInfo),
    Pitch(PitchState),
    Error(String),
}

struct LastPitch {
    note: String,
    frequency: f32,
    target_frequency: f32,
    cents: f32,
    confidence: f32,
    at: Instant,
}

fn is_quit_key(key: KeyEvent) -> bool {
    key.code == KeyCode::Esc
        || key.code == KeyCode::Char('q')
        || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
}

fn main() -> anyhow::Result<()> {
    let config = parse_args();
    let mut terminal = graphics::Terminal::new()?;
    terminal.draw_starting()?;

    let running = Arc::new(AtomicBool::new(true));
    let audio_running = Arc::clone(&running);
    let (ui_tx, ui_rx) = mpsc::channel::<UiEvent>();
    let requested_sample_rate = config.sample_rate;

    let audio_handle = std::thread::spawn(move || {
        let ready_tx = ui_tx.clone();
        let pitch_tx = ui_tx.clone();
        let error_tx = ui_tx;
        let mut detector = None;

        let result = audio::run_audio_capture(
            move |samples, sample_rate| {
                let detector = detector.get_or_insert_with(|| PitchDetector::new(sample_rate));
                if let Some(state) = detector.push(samples) {
                    let _ = pitch_tx.send(UiEvent::Pitch(state));
                }
            },
            move |info| {
                let _ = ready_tx.send(UiEvent::Ready(info));
            },
            requested_sample_rate,
            audio_running,
        );

        if let Err(error) = result {
            let _ = error_tx.send(UiEvent::Error(error.to_string()));
        }
    });

    let mut audio_info = None;
    let mut smoother = PitchSmoother::new();
    let mut fatal_error = None;
    let mut last_pitch: Option<LastPitch> = None;

    while running.load(Ordering::Acquire) {
        loop {
            let update = match ui_rx.try_recv() {
                Ok(update) => update,
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    if running.load(Ordering::Acquire) {
                        fatal_error = Some("audio worker stopped unexpectedly".to_string());
                        running.store(false, Ordering::Release);
                    }
                    break;
                }
            };

            match update {
                UiEvent::Ready(info) => {
                    audio_info = Some(info);
                    terminal.draw_waiting("Listening for a string", audio_info.as_ref(), -120.0)?;
                }
                UiEvent::Pitch(PitchState::TooQuiet { level_db }) => {
                    if let Some(previous) = last_pitch.as_ref() {
                        if previous.at.elapsed() <= Duration::from_millis(650) {
                            terminal.draw_pitch(
                                &previous.note,
                                previous.frequency,
                                previous.target_frequency,
                                previous.cents,
                                previous.confidence * 0.8,
                                config.tolerance,
                                level_db,
                                audio_info.as_ref(),
                            )?;
                            continue;
                        }
                    }
                    smoother.reset();
                    last_pitch = None;
                    terminal.draw_waiting("Pluck a string", audio_info.as_ref(), level_db)?;
                }
                UiEvent::Pitch(PitchState::Uncertain { level_db }) => {
                    if let Some(previous) = last_pitch.as_ref() {
                        if previous.at.elapsed() <= Duration::from_millis(650) {
                            terminal.draw_pitch(
                                &previous.note,
                                previous.frequency,
                                previous.target_frequency,
                                previous.cents,
                                previous.confidence * 0.85,
                                config.tolerance,
                                level_db,
                                audio_info.as_ref(),
                            )?;
                            continue;
                        }
                    }
                    terminal.draw_waiting("Listening for a stable pitch", audio_info.as_ref(), level_db)?;
                }
                UiEvent::Pitch(PitchState::Pitch {
                    frequency,
                    confidence,
                    level_db,
                }) => {
                    let frequency = smoother.update(frequency);
                    if let Some(reading) = tuner::detect_note(frequency) {
                        terminal.draw_pitch(
                            &reading.name,
                            frequency,
                            reading.target_frequency,
                            reading.cents,
                            confidence,
                            config.tolerance,
                            level_db,
                            audio_info.as_ref(),
                        )?;
                        last_pitch = Some(LastPitch {
                            note: reading.name,
                            frequency,
                            target_frequency: reading.target_frequency,
                            cents: reading.cents,
                            confidence,
                            at: Instant::now(),
                        });
                    }
                }
                UiEvent::Error(error) => {
                    fatal_error = Some(error);
                    running.store(false, Ordering::Release);
                }
            }
        }

        if event::poll(Duration::from_millis(20))? {
            if let Event::Key(key) = event::read()? {
                if is_quit_key(key) {
                    running.store(false, Ordering::Release);
                }
            }
        }
    }

    running.store(false, Ordering::Release);

    if audio_handle.join().is_err() && fatal_error.is_none() {
        fatal_error = Some("audio worker panicked".to_string());
    }

    drop(terminal);

    if let Some(error) = fatal_error {
        return Err(anyhow::anyhow!(error));
    }

    Ok(())
}
