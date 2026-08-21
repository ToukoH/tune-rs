use clap::{Arg, Command};

pub struct Config {
    pub sample_rate: u32,
    pub tolerance: f32,
}

fn parse_sample_rate(value: &str) -> Result<u32, String> {
    let sample_rate = value
        .parse::<u32>()
        .map_err(|_| "sample rate must be an integer".to_string())?;

    if (8_000..=192_000).contains(&sample_rate) {
        Ok(sample_rate)
    } else {
        Err("sample rate must be between 8000 and 192000 Hz".to_string())
    }
}

fn parse_tolerance(value: &str) -> Result<f32, String> {
    let tolerance = value
        .parse::<f32>()
        .map_err(|_| "tolerance must be a number".to_string())?;

    if tolerance.is_finite() && (0.1..=25.0).contains(&tolerance) {
        Ok(tolerance)
    } else {
        Err("tolerance must be between 0.1 and 25 cents".to_string())
    }
}

pub fn parse_args() -> Config {
    let matches = Command::new("Guitar Tuner")
        .version("0.2")
        .about("Command-line guitar tuner")
        .arg(
            Arg::new("sample_rate")
                .short('r')
                .long("sample-rate")
                .visible_alias("sample_rate")
                .value_name("HZ")
                .default_value("44100")
                .value_parser(parse_sample_rate)
                .help("Preferred audio sample rate"),
        )
        .arg(
            Arg::new("tolerance")
                .short('t')
                .long("tolerance")
                .value_name("CENTS")
                .default_value("1.0")
                .value_parser(parse_tolerance)
                .help("In-tune tolerance in cents"),
        )
        .get_matches();

    Config {
        sample_rate: *matches.get_one::<u32>("sample_rate").unwrap(),
        tolerance: *matches.get_one::<f32>("tolerance").unwrap(),
    }
}
