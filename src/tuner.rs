const A4_FREQUENCY: f32 = 440.0;
const NOTE_NAMES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

pub struct NoteReading {
    pub name: String,
    pub cents: f32,
    pub target_frequency: f32,
}

pub struct PitchSmoother {
    values: [f32; 5],
    len: usize,
    next: usize,
}

impl PitchSmoother {
    pub fn new() -> Self {
        Self {
            values: [0.0; 5],
            len: 0,
            next: 0,
        }
    }

    pub fn reset(&mut self) {
        self.len = 0;
        self.next = 0;
    }

    pub fn update(&mut self, frequency: f32) -> f32 {
        if !frequency.is_finite() || frequency <= 0.0 {
            return frequency;
        }

        let value = frequency.log2();

        if self.len > 0 {
            let center = self.median_log();
            let distance_cents = (value - center).abs() * 1200.0;
            if distance_cents > 150.0 {
                self.reset();
            }
        }

        self.values[self.next] = value;
        self.next = (self.next + 1) % self.values.len();
        self.len = (self.len + 1).min(self.values.len());

        self.median_log().exp2()
    }

    fn median_log(&self) -> f32 {
        let mut values = [0.0f32; 5];
        values[..self.len].copy_from_slice(&self.values[..self.len]);
        values[..self.len].sort_by(|a, b| a.total_cmp(b));
        values[self.len / 2]
    }
}

pub fn detect_note(frequency: f32) -> Option<NoteReading> {
    if !frequency.is_finite() || frequency <= 0.0 {
        return None;
    }

    let midi = 69.0 + 12.0 * (frequency / A4_FREQUENCY).log2();

    if !midi.is_finite() {
        return None;
    }

    let nearest = midi.round() as i32;
    let cents = (midi - nearest as f32) * 100.0;
    let note_index = nearest.rem_euclid(12) as usize;
    let octave = nearest.div_euclid(12) - 1;
    let target_frequency = A4_FREQUENCY * 2.0f32.powf((nearest as f32 - 69.0) / 12.0);

    Some(NoteReading {
        name: format!("{}{}", NOTE_NAMES[note_index], octave),
        cents,
        target_frequency,
    })
}
