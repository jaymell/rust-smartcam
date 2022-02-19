use crate::config;
use crate::config::Config;
use chrono::{DateTime, Utc};
use log::{Metadata, Record, SetLoggerError};
use once_cell::sync::Lazy;
use std::sync::Arc;
use std::time::SystemTime;

struct SimpleLogger(Arc<Config>);

impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.0.log_level.level()
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let now: DateTime<Utc> = SystemTime::now().into();
            println!(
                "[{}] {} - {}",
                now.to_rfc3339(),
                record.level(),
                record.args()
            );
        }
    }

    fn flush(&self) {}
}

static LOGGER: Lazy<Arc<SimpleLogger>> = Lazy::new(|| {
    let config = config::load_config(None);
    Arc::new(SimpleLogger(Arc::clone(&config)))
});

pub fn init() -> Result<(), SetLoggerError> {
    let config = config::load_config(None);
    log::set_logger(LOGGER.as_ref()).map(|_| log::set_max_level(config.log_level.level_filter()))
}
