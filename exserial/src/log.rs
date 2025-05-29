use log::{Level, Metadata, Record};

use crate::models::CaptureMessage;

struct ExLog;

impl log::Log for ExLog {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            crate::print_message(CaptureMessage::Log {
                level: record.metadata().level(),
                message: format!("{}", record.args()),
            });
        }
    }

    fn flush(&self) {}
}
