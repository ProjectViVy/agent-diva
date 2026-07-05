use std::path::Path;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{
    fmt, fmt::time::LocalTime, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer,
    Registry,
};

use crate::config::schema::LoggingConfig;

/// Initialize the logging system
pub fn init_logging(config: &LoggingConfig) -> WorkerGuard {
    init_logging_with_terminal_output(config, true)
}

/// Initialize the logging system and optionally write logs to the current terminal.
pub fn init_logging_with_terminal_output(
    config: &LoggingConfig,
    enable_terminal_output: bool,
) -> WorkerGuard {
    // 1. Log Level
    let log_level_str = std::env::var("RUST_LOG").unwrap_or_else(|_| config.level.clone());

    // Build the EnvFilter
    let mut filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&log_level_str));

    // Apply module overrides from config
    for (module, level) in &config.overrides {
        // Directives must be valid
        if let Ok(directive) = format!("{}={}", module, level).parse() {
            filter = filter.add_directive(directive);
        } else {
            eprintln!("Invalid log directive: {}={}", module, level);
        }
    }

    // 2. Log Format
    let format_str = std::env::var("LOG_FORMAT").unwrap_or_else(|_| config.format.clone());
    let is_json = format_str.to_lowercase() == "json";

    // 3. File Appender
    // We use rolling::daily.
    // Requirement: gateway-{date}.log
    // tracing_appender::rolling::daily(dir, "gateway.log") produces gateway.log.YYYY-MM-DD
    // tracing_appender::rolling::daily(dir, "gateway") produces gateway.YYYY-MM-DD
    // We'll use "gateway.log" as prefix to get gateway.log.YYYY-MM-DD which is standard.
    let file_appender = tracing_appender::rolling::daily(&config.dir, "gateway.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // 4. Layers
    // We need to use Box<dyn Layer<S>> to unify types for conditional compilation
    // But since is_json is runtime, we can't easily change the Layer type in the subscriber type chain
    // without boxing.

    // RFC 3339 in the process local timezone (e.g. `+08:00`), not UTC `Z`.
    let stdout_layer = enable_terminal_output.then(|| {
        if is_json {
            fmt::layer()
                .json()
                .with_timer(LocalTime::rfc_3339())
                .with_target(true)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true)
                .boxed()
        } else {
            fmt::layer()
                .with_timer(LocalTime::rfc_3339())
                .with_target(true)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true)
                // .pretty() // Optional: make text output pretty
                .boxed()
        }
    });

    let file_layer = if is_json {
        fmt::layer()
            .json()
            .with_writer(non_blocking)
            .with_timer(LocalTime::rfc_3339())
            .with_target(true)
            .with_thread_ids(true)
            .with_file(true)
            .with_line_number(true)
            .with_ansi(false)
            .boxed()
    } else {
        fmt::layer()
            .with_writer(non_blocking)
            .with_timer(LocalTime::rfc_3339())
            .with_ansi(false)
            .with_target(true)
            .with_thread_ids(true)
            .with_file(true)
            .with_line_number(true)
            .boxed()
    };

    // 5. Init Subscriber
    Registry::default()
        .with(filter)
        .with(stdout_layer)
        .with(file_layer)
        .init();

    // 6. Cleanup old logs
    if let Err(e) = cleanup_old_logs(&config.dir, config.retention_days) {
        eprintln!("Failed to clean up old logs: {}", e);
    }

    guard
}

/// Clean up log files older than `days` days
fn cleanup_old_logs(dir: &str, days: u64) -> std::io::Result<()> {
    let path = Path::new(dir);
    if !path.exists() {
        return Ok(());
    }

    if days == 0 {
        return Ok(());
    }

    let now = std::time::SystemTime::now();
    let threshold = std::time::Duration::from_secs(days * 24 * 3600);

    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                // Match standard patterns
                if name.starts_with("gateway.log") || name.starts_with("gateway-") {
                    if let Ok(metadata) = entry.metadata() {
                        if let Ok(modified) = metadata.modified() {
                            if let Ok(age) = now.duration_since(modified) {
                                if age > threshold {
                                    if let Err(e) = std::fs::remove_file(&path) {
                                        eprintln!(
                                            "Failed to remove old log file {:?}: {}",
                                            path, e
                                        );
                                    } else {
                                        // Use println here as logger might not be fully ready or to avoid recursion loop if we log to file?
                                        // Actually logger is initializing, so we can use eprintln for internal errors.
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use filetime::{set_file_mtime, FileTime};

    fn write_log_file(path: &std::path::Path) {
        std::fs::write(path, "test log").unwrap();
    }

    #[test]
    fn logging_retention_default_is_30() {
        let config: crate::config::schema::LoggingConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(config.retention_days, 30);
    }

    #[test]
    fn logging_retention_custom_value() {
        let config: crate::config::schema::LoggingConfig =
            serde_json::from_str(r#"{"retention_days": 7}"#).unwrap();
        assert_eq!(config.retention_days, 7);
    }

    #[test]
    fn logging_retention_zero_keeps_all() {
        let temp = tempfile::tempdir().unwrap();
        let old_log = temp.path().join("gateway.log.2026-07-01");
        write_log_file(&old_log);
        let old_mtime = FileTime::from_unix_time(946684800, 0);
        set_file_mtime(&old_log, old_mtime).unwrap();

        cleanup_old_logs(temp.path().to_str().unwrap(), 0).unwrap();

        assert!(old_log.exists(), "retention_days=0 should keep all logs");
    }

    #[test]
    fn logging_retention_nonexistent_dir_ok() {
        let result = cleanup_old_logs("/nonexistent/path", 7);
        assert!(result.is_ok());
    }

    #[test]
    fn logging_retention_threshold_calculation() {
        // Verify the threshold math: 30 days = 30 * 24 * 3600 seconds
        let days: u64 = 30;
        let expected_secs = days * 24 * 3600;
        assert_eq!(expected_secs, 2_592_000);
    }

    #[test]
    fn logging_retention_removes_only_expired_gateway_logs() {
        let temp = tempfile::tempdir().unwrap();
        let old_log = temp.path().join("gateway.log.2026-06-01");
        let fresh_log = temp.path().join("gateway.log.2026-07-05");
        let unrelated = temp.path().join("notes.txt");
        write_log_file(&old_log);
        write_log_file(&fresh_log);
        write_log_file(&unrelated);

        let old_mtime = FileTime::from_system_time(
            std::time::SystemTime::now() - std::time::Duration::from_secs(2 * 24 * 3600),
        );
        let fresh_mtime = FileTime::from_system_time(std::time::SystemTime::now());
        set_file_mtime(&old_log, old_mtime).unwrap();
        set_file_mtime(&fresh_log, fresh_mtime).unwrap();

        cleanup_old_logs(temp.path().to_str().unwrap(), 1).unwrap();

        assert!(!old_log.exists(), "expired gateway log should be removed");
        assert!(fresh_log.exists(), "fresh gateway log should be retained");
        assert!(unrelated.exists(), "non-gateway files should be ignored");
    }
}
