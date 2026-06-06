use std::io::{self, Write};
use std::path::Path;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{
    fmt,
    fmt::{time::LocalTime, writer::MakeWriter},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Layer, Registry,
};

use crate::{config::schema::LoggingConfig, redaction::redact_secrets};

/// Initialize the logging system
pub fn init_logging(config: &LoggingConfig) -> WorkerGuard {
    init_logging_with_terminal_output(config, true)
}

/// Initialize the logging system and optionally write logs to the current terminal.
pub fn init_logging_with_terminal_output(
    config: &LoggingConfig,
    enable_terminal_output: bool,
) -> WorkerGuard {
    let log_level_str = std::env::var("RUST_LOG").unwrap_or_else(|_| config.level.clone());

    let mut filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&log_level_str));

    for (module, level) in &config.overrides {
        if let Ok(directive) = format!("{}={}", module, level).parse() {
            filter = filter.add_directive(directive);
        } else {
            eprintln!("Invalid log directive: {}={}", module, level);
        }
    }

    let format_str = std::env::var("LOG_FORMAT").unwrap_or_else(|_| config.format.clone());
    let is_json = format_str.to_lowercase() == "json";

    let file_appender = tracing_appender::rolling::daily(&config.dir, "gateway.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    let file_writer = RedactingMakeWriter::new(non_blocking);

    let stdout_layer = enable_terminal_output.then(|| {
        let stdout_writer = RedactingMakeWriter::new(std::io::stdout);
        if is_json {
            fmt::layer()
                .json()
                .with_writer(stdout_writer)
                .with_timer(LocalTime::rfc_3339())
                .with_target(true)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true)
                .boxed()
        } else {
            fmt::layer()
                .with_writer(stdout_writer)
                .with_timer(LocalTime::rfc_3339())
                .with_target(true)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true)
                .boxed()
        }
    });

    let file_layer = if is_json {
        fmt::layer()
            .json()
            .with_writer(file_writer)
            .with_timer(LocalTime::rfc_3339())
            .with_target(true)
            .with_thread_ids(true)
            .with_file(true)
            .with_line_number(true)
            .with_ansi(false)
            .boxed()
    } else {
        fmt::layer()
            .with_writer(file_writer)
            .with_timer(LocalTime::rfc_3339())
            .with_ansi(false)
            .with_target(true)
            .with_thread_ids(true)
            .with_file(true)
            .with_line_number(true)
            .boxed()
    };

    Registry::default()
        .with(filter)
        .with(stdout_layer)
        .with(file_layer)
        .init();

    if let Err(e) = cleanup_old_logs(&config.dir, 7) {
        eprintln!("Failed to clean up old logs: {}", e);
    }

    guard
}

#[derive(Clone)]
pub struct RedactingMakeWriter<M> {
    inner: M,
}

impl<M> RedactingMakeWriter<M> {
    pub fn new(inner: M) -> Self {
        Self { inner }
    }
}

impl<'a, M> MakeWriter<'a> for RedactingMakeWriter<M>
where
    M: MakeWriter<'a>,
{
    type Writer = RedactingWriter<M::Writer>;

    fn make_writer(&'a self) -> Self::Writer {
        RedactingWriter::new(self.inner.make_writer())
    }
}

pub struct RedactingWriter<W: Write> {
    inner: W,
    buffer: Vec<u8>,
}

impl<W: Write> RedactingWriter<W> {
    fn new(inner: W) -> Self {
        Self {
            inner,
            buffer: Vec::new(),
        }
    }

    fn flush_buffer(&mut self) -> io::Result<()> {
        if self.buffer.is_empty() {
            return self.inner.flush();
        }

        let buffered = String::from_utf8_lossy(&self.buffer);
        let redacted = redact_secrets(&buffered);
        self.inner.write_all(redacted.as_bytes())?;
        self.buffer.clear();
        self.inner.flush()
    }
}

impl<W: Write> Write for RedactingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buffer.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.flush_buffer()
    }
}

impl<W: Write> Drop for RedactingWriter<W> {
    fn drop(&mut self) {
        let _ = self.flush_buffer();
    }
}

/// Clean up log files older than `days` days
fn cleanup_old_logs(dir: &str, days: u64) -> std::io::Result<()> {
    let path = Path::new(dir);
    if !path.exists() {
        return Ok(());
    }

    let now = std::time::SystemTime::now();
    let threshold = std::time::Duration::from_secs(days * 24 * 3600);

    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
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
    use super::{RedactingMakeWriter, Write};
    use std::sync::{Arc, Mutex};
    use tracing_subscriber::fmt::writer::MakeWriter;

    #[derive(Clone, Default)]
    struct SharedBuffer(Arc<Mutex<Vec<u8>>>);

    impl SharedBuffer {
        fn contents(&self) -> String {
            String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
        }
    }

    struct SharedBufferWriter(Arc<Mutex<Vec<u8>>>);

    impl Write for SharedBufferWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl<'a> MakeWriter<'a> for SharedBuffer {
        type Writer = SharedBufferWriter;

        fn make_writer(&'a self) -> Self::Writer {
            SharedBufferWriter(self.0.clone())
        }
    }

    #[test]
    fn redacting_writer_scrubs_buffered_output() {
        let sink = SharedBuffer::default();
        let writer_factory = RedactingMakeWriter::new(sink.clone());
        let mut writer = writer_factory.make_writer();

        writer
            .write_all(br#"Authorization: Bearer sk-secret api_key: "ghp_demo""#)
            .unwrap();
        writer.flush().unwrap();

        let output = sink.contents();
        assert!(output.contains("***REDACTED***"));
        assert!(!output.contains("sk-secret"));
        assert!(!output.contains("ghp_demo"));
    }
}
