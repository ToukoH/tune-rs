use crossterm::{
    cursor::MoveTo,
    execute,
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
    Result,
};
use std::io::{stdout, Write};

pub fn init_terminal() -> Result<()> {
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, Clear(ClearType::All))?;
    terminal::enable_raw_mode()?;
    Ok(())
}

pub fn restore_terminal() -> Result<()> {
    let mut stdout = stdout();
    terminal::disable_raw_mode()?;
    execute!(stdout, LeaveAlternateScreen)?;
    Ok(())
}

pub fn update_tuning(note: &str, cents: f32) -> Result<()> {
    let gauge_width = 41;
    let clamped_cents = cents.max(-50.0).min(50.0);
    let marker_pos = (((clamped_cents + 50.0) / 100.0) * (gauge_width as f32 - 1.0))
        .round() as usize;

    let mut gauge = vec!['-'; gauge_width];
    if marker_pos < gauge_width {
        gauge[marker_pos] = '|';
    }
    let gauge_str: String = gauge.into_iter().collect();

    let mut stdout = stdout();
    execute!(stdout, Clear(ClearType::All), MoveTo(0, 0))?;
    writeln!(stdout, "Note: {}", note)?;
    writeln!(stdout, "Deviation: {:.2} cents", cents)?;
    writeln!(stdout, "Tuning:")?;
    writeln!(stdout, "[-50] {} [50]", gauge_str)?;
    stdout.flush()?;

    Ok(())
}

pub fn update_message(message: &str) -> Result<()> {
    let mut stdout = stdout();
    execute!(stdout, Clear(ClearType::All), MoveTo(0, 0))?;
    writeln!(stdout, "{}", message)?;
    stdout.flush()?;
    Ok(())
}
