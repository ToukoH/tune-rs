use clap::{Arg, Command};

pub struct Config {
    pub sample_rate: f32,
    pub tolerance: f32,
}

pub fn parse_args() -> Config {
    let matches = Command::new("Guitar Tuner")
        .version("0.1")
        .about("Command-line note tuner")
        .arg(
            Arg::new("sample_rate")
                .short('r')
                .long("sample_rate")
                .default_value("44100")
                .value_parser(clap::value_parser!(f32))
                .help("Audio sample rate"),
        )
        .arg(
            Arg::new("tolerance")
                .short('t')
                .long("tolerance")
                .default_value("1.0")
                .value_parser(clap::value_parser!(f32))
                .help("Tolerance in cents"),
        )
        .get_matches();

    let sample_rate = *matches.get_one::<f32>("sample_rate").unwrap();
    let tolerance = *matches.get_one::<f32>("tolerance").unwrap();

    Config {
        sample_rate,
        tolerance,
    }
}
