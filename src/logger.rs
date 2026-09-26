use log::LevelFilter;
use std::io::{IsTerminal, Write};

pub fn init(cli: &crate::cli::Cli) {
    let level = if cli.quiet { LevelFilter::Error } else if cli.verbose { LevelFilter::Debug } else { LevelFilter::Info };
    let color = std::io::stderr().is_terminal();
    let mut builder = env_logger::Builder::from_default_env();
    builder.filter_level(level);
    builder.format(move |buf, record| {
        if color { writeln!(buf, "[{}] {}", record.level(), record.args()) }
        else { writeln!(buf, "[{}] {}", record.level(), record.args()) }
    });
    let _ = builder.try_init();
}
