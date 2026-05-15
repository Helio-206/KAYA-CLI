use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tracing_subscriber::fmt::writer::MakeWriter;
use tracing_subscriber::EnvFilter;

pub fn init_tracing(log_level: &str, data_dir: &Path) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level));
    let writer = SharedLogWriter::new(data_dir).unwrap_or_else(|_| SharedLogWriter::sink());
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(writer)
        .with_ansi(false)
        .with_target(true)
        .try_init();
}

fn log_file_path(data_dir: &Path) -> PathBuf {
    data_dir.join("logs").join("kaya.log")
}

#[derive(Clone)]
struct SharedLogWriter {
    destination: LogDestination,
}

#[derive(Clone)]
enum LogDestination {
    File(Arc<Mutex<File>>),
    Sink,
}

impl SharedLogWriter {
    fn new(data_dir: &Path) -> io::Result<Self> {
        let log_path = log_file_path(data_dir);
        if let Some(parent) = log_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)?;
        Ok(Self {
            destination: LogDestination::File(Arc::new(Mutex::new(file))),
        })
    }

    fn sink() -> Self {
        Self {
            destination: LogDestination::Sink,
        }
    }
}

struct SharedLogGuard {
    destination: LogDestination,
}

impl<'a> MakeWriter<'a> for SharedLogWriter {
    type Writer = SharedLogGuard;

    fn make_writer(&'a self) -> Self::Writer {
        SharedLogGuard {
            destination: self.destination.clone(),
        }
    }
}

impl Write for SharedLogGuard {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match &self.destination {
            LogDestination::File(file) => {
                let mut handle = file
                    .lock()
                    .map_err(|_| io::Error::other("failed to lock log file"))?;
                handle.write(buf)
            }
            LogDestination::Sink => Ok(buf.len()),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match &self.destination {
            LogDestination::File(file) => {
                let mut handle = file
                    .lock()
                    .map_err(|_| io::Error::other("failed to lock log file"))?;
                handle.flush()
            }
            LogDestination::Sink => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{log_file_path, SharedLogWriter};
    use std::fs;
    use std::io::Write;

    #[test]
    fn log_file_path_uses_profile_data_dir() {
        let root = std::path::Path::new("/tmp/kaya-profile");
        assert_eq!(log_file_path(root), root.join("logs").join("kaya.log"));
    }

    #[test]
    fn shared_log_writer_appends_to_file() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let writer = SharedLogWriter::new(temp_dir.path()).expect("writer");
        let mut guard = tracing_subscriber::fmt::writer::MakeWriter::make_writer(&writer);
        writeln!(guard, "voice debug line").expect("write log line");
        guard.flush().expect("flush log line");

        let contents = fs::read_to_string(log_file_path(temp_dir.path())).expect("read log file");
        assert!(contents.contains("voice debug line"));
    }
}
