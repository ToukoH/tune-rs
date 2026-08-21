const TARGET_SAMPLE_RATE: f32 = 12_000.0;
const FRAME_SIZE: usize = 3072;
const HOP_SIZE: usize = 384;
const MIN_FREQUENCY: f32 = 55.0;
const MAX_FREQUENCY: f32 = 1200.0;
const ABSOLUTE_RMS_FLOOR: f32 = 0.00002;
const YIN_THRESHOLD: f32 = 0.18;
const ACQUIRE_CONFIDENCE: f32 = 0.62;
const TRACK_CONFIDENCE: f32 = 0.34;
const TRACK_WINDOW_CENTS: f32 = 60.0;
const TRACK_MEMORY_FRAMES: u16 = 28;

#[derive(Clone, Copy)]
pub enum PitchState {
    TooQuiet { level_db: f32 },
    Uncertain { level_db: f32 },
    Pitch {
        frequency: f32,
        confidence: f32,
        level_db: f32,
    },
}

pub struct PitchDetector {
    effective_sample_rate: f32,
    decimation: usize,
    decimation_sum: f32,
    decimation_count: usize,
    buffer: Vec<f32>,
    difference: Vec<f32>,
    last_frequency: Option<f32>,
    missed_frames: u16,
}

impl PitchDetector {
    pub fn new(input_sample_rate: f32) -> Self {
        let decimation = (input_sample_rate / TARGET_SAMPLE_RATE)
            .round()
            .max(1.0) as usize;

        Self {
            effective_sample_rate: input_sample_rate / decimation as f32,
            decimation,
            decimation_sum: 0.0,
            decimation_count: 0,
            buffer: Vec::with_capacity(FRAME_SIZE + HOP_SIZE),
            difference: Vec::new(),
            last_frequency: None,
            missed_frames: TRACK_MEMORY_FRAMES,
        }
    }

    pub fn push(&mut self, samples: &[f32]) -> Option<PitchState> {
        for &sample in samples {
            if !sample.is_finite() {
                continue;
            }

            self.decimation_sum += sample;
            self.decimation_count += 1;

            if self.decimation_count == self.decimation {
                self.buffer
                    .push(self.decimation_sum / self.decimation as f32);
                self.decimation_sum = 0.0;
                self.decimation_count = 0;
            }
        }

        let mut latest = None;

        while self.buffer.len() >= FRAME_SIZE {
            let analysis = analyze_frame(
                &self.buffer[..FRAME_SIZE],
                self.effective_sample_rate,
                &mut self.difference,
            );
            latest = Some(self.classify(analysis));
            self.buffer.drain(..HOP_SIZE);
        }

        latest
    }

    fn classify(&mut self, analysis: Analysis) -> PitchState {
        if analysis.rms < ABSOLUTE_RMS_FLOOR || !analysis.rms.is_finite() {
            self.miss();
            return PitchState::TooQuiet {
                level_db: analysis.level_db,
            };
        }

        let Some(frequency) = analysis.frequency else {
            self.miss();
            return PitchState::Uncertain {
                level_db: analysis.level_db,
            };
        };

        let tracking = self
            .last_frequency
            .filter(|_| self.missed_frames < TRACK_MEMORY_FRAMES)
            .map(|previous| cents_between(previous, frequency).abs() <= TRACK_WINDOW_CENTS)
            .unwrap_or(false);
        let required_confidence = if tracking {
            TRACK_CONFIDENCE
        } else {
            ACQUIRE_CONFIDENCE
        };

        if analysis.confidence < required_confidence {
            self.miss();
            return PitchState::Uncertain {
                level_db: analysis.level_db,
            };
        }

        self.last_frequency = Some(frequency);
        self.missed_frames = 0;

        PitchState::Pitch {
            frequency,
            confidence: analysis.confidence,
            level_db: analysis.level_db,
        }
    }

    fn miss(&mut self) {
        self.missed_frames = self.missed_frames.saturating_add(1);
        if self.missed_frames >= TRACK_MEMORY_FRAMES {
            self.last_frequency = None;
        }
    }
}

struct Analysis {
    frequency: Option<f32>,
    confidence: f32,
    rms: f32,
    level_db: f32,
}

fn analyze_frame(frame: &[f32], sample_rate: f32, difference: &mut Vec<f32>) -> Analysis {
    let mean = frame.iter().copied().sum::<f32>() / frame.len() as f32;
    let rms = (frame
        .iter()
        .map(|&sample| {
            let centered = sample - mean;
            centered * centered
        })
        .sum::<f32>()
        / frame.len() as f32)
        .sqrt();
    let level_db = if rms > 0.0 && rms.is_finite() {
        20.0 * rms.log10()
    } else {
        -120.0
    };

    if !rms.is_finite() || rms < ABSOLUTE_RMS_FLOOR {
        return Analysis {
            frequency: None,
            confidence: 0.0,
            rms,
            level_db,
        };
    }

    let min_tau = (sample_rate / MAX_FREQUENCY).floor().max(2.0) as usize;
    let max_tau = (sample_rate / MIN_FREQUENCY).ceil() as usize;
    let max_tau = max_tau.min(frame.len().saturating_sub(3));

    if min_tau >= max_tau {
        return Analysis {
            frequency: None,
            confidence: 0.0,
            rms,
            level_db,
        };
    }

    difference.clear();
    difference.resize(max_tau + 1, 0.0);
    let comparison_len = frame.len() - max_tau;
    let inverse_rms = 1.0 / rms;

    for tau in 1..=max_tau {
        let mut sum = 0.0f32;

        for index in 0..comparison_len {
            let delta = (frame[index] - frame[index + tau]) * inverse_rms;
            sum += delta * delta;
        }

        difference[tau] = sum;
    }

    let mut running_sum = 0.0f32;
    difference[0] = 1.0;

    for tau in 1..=max_tau {
        running_sum += difference[tau];
        difference[tau] = if running_sum > 1e-12 {
            difference[tau] * tau as f32 / running_sum
        } else {
            1.0
        };
    }

    let mut candidate = None;
    let mut tau = min_tau;

    while tau < max_tau {
        if difference[tau] < YIN_THRESHOLD {
            while tau + 1 <= max_tau && difference[tau + 1] < difference[tau] {
                tau += 1;
            }
            candidate = Some(tau);
            break;
        }
        tau += 1;
    }

    let tau = candidate.unwrap_or_else(|| {
        (min_tau..=max_tau)
            .min_by(|&a, &b| difference[a].total_cmp(&difference[b]))
            .unwrap_or(min_tau)
    });
    let confidence = (1.0 - difference[tau]).clamp(0.0, 1.0);
    let refined_tau = if tau > 1 && tau < max_tau {
        let left = difference[tau - 1];
        let center = difference[tau];
        let right = difference[tau + 1];
        let denominator = left - 2.0 * center + right;

        if denominator.abs() > 1e-12 {
            tau as f32 + 0.5 * (left - right) / denominator
        } else {
            tau as f32
        }
    } else {
        tau as f32
    };
    let frequency = sample_rate / refined_tau;
    let frequency = if frequency.is_finite()
        && (MIN_FREQUENCY..=MAX_FREQUENCY).contains(&frequency)
    {
        Some(frequency)
    } else {
        None
    };

    Analysis {
        frequency,
        confidence,
        rms,
        level_db,
    }
}

fn cents_between(a: f32, b: f32) -> f32 {
    1200.0 * (b / a).log2()
}
