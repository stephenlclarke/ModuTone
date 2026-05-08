// Phase: 10
// Content-free file logger with rotation.
//
// This logger must NEVER log user content (input text, output text,
// refinement instructions, assembled prompts). Only structured metadata
// (IDs, counts, durations, error codes) may be logged.
//
// Config:
//   - Max file size: 5 MB (SizeTrigger)
//   - Max files: 3 (FixedWindowRoller: app.log, app.1.log, app.2.log)
//   - Log directory: {app_data_dir}/logs/
//   - Format: [{timestamp}] [{level}] {message}
//   - Level: Info for file appender

use std::path::Path;

use log::LevelFilter;
use log4rs::config::{Appender, Config, Root};
use log4rs::encode::pattern::PatternEncoder;

const LOG_DIR: &str = "logs";
const LOG_FILE: &str = "app.log";
const MAX_LOG_SIZE_BYTES: u64 = 5 * 1024 * 1024; // 5 MB
const MAX_LOG_FILES: u32 = 3;
const LOG_PATTERN: &str = "[{d(%Y-%m-%dT%H:%M:%S%.3fZ)(utc)}] [{l}] {m}{n}";

/// Initialize the file logger with rotation.
///
/// Creates `{data_dir}/logs/app.log` with size-based rotation.
/// Returns `Err` if log initialization fails (non-fatal — caller may continue).
pub fn init_logger(data_dir: &Path) -> Result<(), String> {
    let log_dir = data_dir.join(LOG_DIR);
    std::fs::create_dir_all(&log_dir)
        .map_err(|e| format!("Failed to create log directory: {}", e))?;

    let log_file = log_dir.join(LOG_FILE);
    let archive_pattern = log_dir.join("app.{}.log").to_string_lossy().into_owned();

    // Build rolling file appender with size trigger + fixed window roller
    let roller = log4rs::append::rolling_file::policy::compound::roll::fixed_window::FixedWindowRoller::builder()
        .build(&archive_pattern, MAX_LOG_FILES)
        .map_err(|e| format!("Failed to build log roller: {}", e))?;

    let trigger = log4rs::append::rolling_file::policy::compound::trigger::size::SizeTrigger::new(
        MAX_LOG_SIZE_BYTES,
    );

    let policy = log4rs::append::rolling_file::policy::compound::CompoundPolicy::new(
        Box::new(trigger),
        Box::new(roller),
    );

    let rolling_appender = log4rs::append::rolling_file::RollingFileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(LOG_PATTERN)))
        .build(log_file, Box::new(policy))
        .map_err(|e| format!("Failed to build rolling file appender: {}", e))?;

    let config = Config::builder()
        .appender(Appender::builder().build("file", Box::new(rolling_appender)))
        .build(Root::builder().appender("file").build(LevelFilter::Info))
        .map_err(|e| format!("Failed to build log config: {}", e))?;

    log4rs::init_config(config).map_err(|e| format!("Failed to initialize logger: {}", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_logger_creates_log_file() {
        let dir = tempfile::TempDir::new().unwrap();
        init_logger(dir.path()).unwrap();

        // Log something so the file gets created
        log::info!("test log entry");

        let log_path = dir.path().join("logs").join("app.log");
        assert!(log_path.exists(), "app.log should be created");
    }

    #[test]
    fn log_pattern_is_valid() {
        // Structural test: verify the log pattern string is well-formed
        // by constructing a PatternEncoder (will panic if pattern is invalid)
        let _encoder = PatternEncoder::new(LOG_PATTERN);
    }
}
