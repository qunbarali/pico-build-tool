use log::{LevelFilter, Level};
use std::io::Write;

pub fn init(cli: &crate::cli::Cli) {
    let level = if cli.quiet {
        LevelFilter::Error
    } else if cli.verbose {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };

    env_logger::Builder::from_default_env()
        .filter_level(level)
        .format(|buf, record| {
            let level_color = match record.level() {
                Level::Error => "\x1b[31m",
                Level::Warn => "\x1b[33m",
                Level::Info => "\x1b[32m",
                Level::Debug => "\x1b[36m",
                Level::Trace => "\x1b[35m",
            };
            let reset = "\x1b[0m";
            writeln!(buf, "[{}{}{}] {}", level_color, record.level(), reset, record.args())
        })
        .try_init()
        .ok();
}
