use log::{Metadata, Record};

use crate::models::CaptureMessage;

pub struct ExLog;

impl log::Log for ExLog {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            crate::print_message(&CaptureMessage::Log {
                level: record.metadata().level(),
                message: format!("{}", record.args()),
            });
        }
    }

    fn flush(&self) {}
}
