#![allow(dead_code)]

use std::io::Write;

pub struct MasterLogger;
impl log::Log for MasterLogger {
    fn enabled(&self, md: &log::Metadata) -> bool {
        let t = md.target();
        if t.starts_with("hyper_util") {
            return false;
        }
        if md.level() > log::Level::Debug {
            return false;
        }

        true
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let level = match record.level() {
            log::Level::Trace => ["\x1b[36m", "T", "Trace"],
            log::Level::Debug => ["\x1b[35m", "D", "Debug"],
            log::Level::Info => ["\x1b[32m", "I", "Info"],
            log::Level::Warn => ["\x1b[33m", "W", "Warn"],
            log::Level::Error => ["\x1b[31m", "E", "Error"],
        };
        let _ = writeln!(
            std::io::stderr(),
            "{{{}{}\x1b[0m}}[{}{}\x1b[0m]{{{}{}\x1b[32m:\x1b[93m{}\x1b[0m}}: {}",
            level[0],
            sys_now(),
            level[0],
            level[1],
            level[0],
            record.target(),
            record.line().unwrap_or_default(),
            record.args(),
        );
    }

    fn flush(&self) {}
}

pub fn sys_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
