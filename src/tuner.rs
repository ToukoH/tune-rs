pub fn detect_note(freq: f32) -> Option<(String, f32)> {
    if freq <= 0.0 {
        return None;
    }

    let midi = 69.0 + 12.0 * (freq / 440.0).log2();
    let midi_round = midi.round();
    let cents = (midi - midi_round) * 100.0;
    let note_names = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];

    let note_index = ((midi_round as i32) % 12).rem_euclid(12);
    let octave = (midi_round as i32) / 12 - 1;

    let note_name = format!("{}{}", note_names[note_index as usize], octave);
    Some((note_name, cents))
}
