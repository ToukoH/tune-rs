use std::io::{self, Write};

pub fn clear_screen() {
    // ANSI escape code
    print!("\x1B[2J\x1B[H");
}

pub fn display_message(message: &str) {
    clear_screen();
    println!("{}", message);
    io::stdout().flush().unwrap();
}

pub fn display_tuning(note: &str, cents: f32) {
    clear_screen();

    let gauge_width = 41;
    let clamped_cents = cents.max(-50.0).min(50.0);
    let marker_pos = (((clamped_cents + 50.0) / 100.0) * (gauge_width as f32 - 1.0))
        .round() as usize;

    let mut gauge = vec!['-'; gauge_width];
    if marker_pos < gauge_width {
        gauge[marker_pos] = '|';
    }
    let gauge_str: String = gauge.into_iter().collect();

    println!("Note: {}", note);
    println!("Deviation: {:.2} cents", cents);
    println!("Tuning:");
    println!("[-50] {} [50]", gauge_str);

    io::stdout().flush().unwrap();
}
