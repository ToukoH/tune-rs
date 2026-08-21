use crate::audio::AudioInfo;
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    execute, queue,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::{self, stdout, Stdout, Write};

pub struct Terminal {
    stdout: Stdout,
}

impl Terminal {
    pub fn new() -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        let mut stdout = stdout();

        if let Err(error) = execute!(stdout, EnterAlternateScreen, Hide, Clear(ClearType::All)) {
            let _ = execute!(stdout, Show, LeaveAlternateScreen);
            let _ = terminal::disable_raw_mode();
            return Err(error);
        }

        Ok(Self { stdout })
    }

    pub fn draw_starting(&mut self) -> io::Result<()> {
        self.draw_waiting("Opening input device", None, -120.0)
    }

    pub fn draw_waiting(
        &mut self,
        message: &str,
        info: Option<&AudioInfo>,
        level_db: f32,
    ) -> io::Result<()> {
        let (width, _) = terminal::size().unwrap_or((80, 24));
        let panel_width = usize::from(width).saturating_sub(2).min(86).max(20);
        let mut lines = Vec::new();
        lines.push(border_top(panel_width, " GUITAR TUNER "));
        lines.push(panel_line(panel_width, ""));
        lines.push(panel_line(panel_width, &center(message, panel_width - 4)));
        lines.push(panel_line(panel_width, ""));
        lines.push(panel_line(
            panel_width,
            &format!("  INPUT  {}", signal_bar(level_db, 22)),
        ));
        lines.push(panel_line(panel_width, ""));
        lines.push(panel_line(
            panel_width,
            &center(&input_label(info), panel_width - 4),
        ));
        lines.push(panel_line(
            panel_width,
            &center("q / Esc / Ctrl-C  quit", panel_width - 4),
        ));
        lines.push(border_bottom(panel_width));
        self.draw_lines(&lines, None)
    }

    pub fn draw_pitch(
        &mut self,
        note: &str,
        frequency: f32,
        target_frequency: f32,
        cents: f32,
        confidence: f32,
        tolerance: f32,
        level_db: f32,
        info: Option<&AudioInfo>,
    ) -> io::Result<()> {
        let (width, _) = terminal::size().unwrap_or((80, 24));
        let panel_width = usize::from(width).saturating_sub(2).min(92).max(20);
        let inner_width = panel_width - 4;
        let status = if cents.abs() <= tolerance {
            "IN TUNE"
        } else if cents < 0.0 {
            "FLAT  ·  tune up"
        } else {
            "SHARP · tune down"
        };
        let gauge_width = inner_width.saturating_sub(10).min(65).max(11);
        let gauge = tuning_gauge(cents, tolerance, gauge_width);
        let scale = tuning_scale(gauge_width);
        let detail = format!(
            "{:+.1} cents   ·   {:.2} Hz   ·   target {:.2} Hz",
            cents, frequency, target_frequency
        );
        let meters = format!(
            "  SIGNAL  {}    LOCK {:>3.0}%",
            signal_bar(level_db, 18),
            confidence * 100.0
        );
        let mut lines = Vec::new();
        lines.push(border_top(panel_width, " GUITAR TUNER "));
        lines.push(panel_line(panel_width, ""));
        lines.push(panel_line(panel_width, &center(note, inner_width)));
        lines.push(panel_line(panel_width, &center(status, inner_width)));
        lines.push(panel_line(panel_width, ""));
        lines.push(panel_line(panel_width, &center(&scale, inner_width)));
        lines.push(panel_line(panel_width, &center(&gauge, inner_width)));
        lines.push(panel_line(panel_width, &center(&detail, inner_width)));
        lines.push(panel_line(panel_width, ""));
        lines.push(panel_line(panel_width, &meters));
        lines.push(panel_line(panel_width, ""));
        lines.push(panel_line(
            panel_width,
            &center(&input_label(info), inner_width),
        ));
        lines.push(panel_line(
            panel_width,
            &center("q / Esc / Ctrl-C  quit", inner_width),
        ));
        lines.push(border_bottom(panel_width));
        self.draw_lines(&lines, Some((note, status, cents, tolerance)))
    }

    fn draw_lines(
        &mut self,
        lines: &[String],
        pitch: Option<(&str, &str, f32, f32)>,
    ) -> io::Result<()> {
        let (terminal_width, terminal_height) = terminal::size().unwrap_or((80, 24));
        let panel_width = lines.first().map(|line| line.chars().count()).unwrap_or(0);
        let x = (usize::from(terminal_width).saturating_sub(panel_width) / 2) as u16;
        let y = (usize::from(terminal_height).saturating_sub(lines.len()) / 2) as u16;

        queue!(self.stdout, MoveTo(0, 0), Clear(ClearType::All))?;

        for (row, line) in lines.iter().enumerate() {
            let color = if row == 0 || row + 1 == lines.len() {
                Color::DarkGrey
            } else if let Some((note, status, cents, tolerance)) = pitch {
                if row == 2 {
                    Color::Cyan
                } else if row == 3 {
                    if cents.abs() <= tolerance {
                        Color::Green
                    } else {
                        Color::Yellow
                    }
                } else if line.contains('●') {
                    if cents.abs() <= tolerance {
                        Color::Green
                    } else {
                        Color::Yellow
                    }
                } else if line.contains(note) || line.contains(status) {
                    Color::White
                } else {
                    Color::Grey
                }
            } else if row == 2 {
                Color::Cyan
            } else {
                Color::Grey
            };

            queue!(
                self.stdout,
                MoveTo(x, y + row as u16),
                SetForegroundColor(color),
                Print(line),
                ResetColor
            )?;
        }

        self.stdout.flush()
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = execute!(self.stdout, ResetColor, Show, LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
    }
}

fn border_top(width: usize, title: &str) -> String {
    let title_len = title.chars().count();
    let fill = width.saturating_sub(title_len + 2);
    let left = fill / 2;
    let right = fill - left;
    format!("╭{}{}{}╮", "─".repeat(left), title, "─".repeat(right))
}

fn border_bottom(width: usize) -> String {
    format!("╰{}╯", "─".repeat(width.saturating_sub(2)))
}

fn panel_line(width: usize, content: &str) -> String {
    let inner = width.saturating_sub(4);
    let content = truncate(content, inner);
    let len = content.chars().count();
    format!("│ {}{} │", content, " ".repeat(inner.saturating_sub(len)))
}

fn center(text: &str, width: usize) -> String {
    let text = truncate(text, width);
    let len = text.chars().count();
    let left = width.saturating_sub(len) / 2;
    let right = width.saturating_sub(len + left);
    format!("{}{}{}", " ".repeat(left), text, " ".repeat(right))
}

fn truncate(text: &str, width: usize) -> String {
    let count = text.chars().count();
    if count <= width {
        return text.to_string();
    }
    if width <= 1 {
        return "…".chars().take(width).collect();
    }
    let mut shortened: String = text.chars().take(width - 1).collect();
    shortened.push('…');
    shortened
}

fn input_label(info: Option<&AudioInfo>) -> String {
    match info {
        Some(info) if info.sample_rate != info.requested_sample_rate => format!(
            "{}  ·  {} Hz  ·  {} ch  ·  requested {} Hz",
            info.device_name, info.sample_rate, info.channels, info.requested_sample_rate
        ),
        Some(info) => format!(
            "{}  ·  {} Hz  ·  {} ch",
            info.device_name, info.sample_rate, info.channels
        ),
        None => "Waiting for microphone".to_string(),
    }
}

fn signal_bar(level_db: f32, width: usize) -> String {
    let normalized = ((level_db + 90.0) / 72.0).clamp(0.0, 1.0);
    let filled = (normalized * width as f32).round() as usize;
    format!(
        "{}{} {:>5.1} dB",
        "█".repeat(filled.min(width)),
        "░".repeat(width.saturating_sub(filled)),
        level_db.max(-120.0)
    )
}

fn tuning_scale(width: usize) -> String {
    let mut chars = vec![' '; width];
    put_label(&mut chars, 0, "-50");
    put_label(&mut chars, width / 4, "-25");
    put_label(&mut chars, width / 2, "0");
    put_label(&mut chars, width * 3 / 4, "+25");
    put_label(&mut chars, width.saturating_sub(3), "+50");
    chars.into_iter().collect()
}

fn tuning_gauge(cents: f32, tolerance: f32, width: usize) -> String {
    let mut chars = vec!['━'; width];
    let center = width / 2;
    let cents_to_index = |value: f32| {
        (((value.clamp(-50.0, 50.0) + 50.0) / 100.0) * (width - 1) as f32).round() as usize
    };
    let left_tolerance = cents_to_index(-tolerance);
    let right_tolerance = cents_to_index(tolerance);
    let marker = cents_to_index(cents);

    for ch in chars
        .iter_mut()
        .take(right_tolerance.saturating_add(1).min(width))
        .skip(left_tolerance.min(width))
    {
        *ch = '═';
    }

    chars[center] = '┿';
    chars[marker] = '●';
    chars.into_iter().collect()
}

fn put_label(chars: &mut [char], center: usize, label: &str) {
    let label_chars: Vec<char> = label.chars().collect();
    let start = center.saturating_sub(label_chars.len() / 2);
    for (index, ch) in label_chars.into_iter().enumerate() {
        if start + index < chars.len() {
            chars[start + index] = ch;
        }
    }
}
