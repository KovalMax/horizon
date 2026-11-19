use chrono::{SecondsFormat, Utc};
use log::{Level, LevelFilter, Log, Metadata, Record};

struct StderrLogger;

static LOGGER: StderrLogger = StderrLogger;

impl Log for StderrLogger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record<'_>) {
        if self.enabled(record.metadata()) {
            let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
            eprintln!(
                "{timestamp} {} [{}] {}",
                record.level(),
                record.target(),
                record.args()
            );
        }
    }

    fn flush(&self) {}
}

pub fn init() {
    log::set_logger(&LOGGER).expect("API logger should only be initialized once");
    log::set_max_level(LevelFilter::Info);
}
