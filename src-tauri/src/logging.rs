use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use tracing_subscriber::fmt::MakeWriter;

/// Current log file size cap. Five files of this size bound the log directory
/// at 50 MB, which is small enough to rotate often and large enough to diagnose
/// a startup or refresh failure.
pub const MAX_FILE_BYTES: u64 = 10 * 1024 * 1024;

/// Total files kept, including the active one.
pub const MAX_FILES: usize = 5;

pub const LOG_FILE_NAME: &str = "system-scanner.log";

#[derive(Debug, thiserror::Error)]
pub enum LoggingError {
    #[error("the log directory could not be created")]
    CreateDir(#[source] io::Error),
    #[error("the log file could not be opened")]
    Open(#[source] io::Error),
    #[error("a global tracing subscriber is already installed")]
    AlreadyInstalled,
}

/// File writer that rotates by size and keeps a fixed number of archives.
///
/// Cloning shares one file handle and one size counter, which is what
/// `tracing_subscriber` needs from `MakeWriter`.
#[derive(Clone)]
pub struct RotatingWriter {
    inner: Arc<Inner>,
}

struct Inner {
    dir: PathBuf,
    max_bytes: u64,
    max_files: usize,
    state: Mutex<State>,
}

struct State {
    file: Option<File>,
    len: u64,
}

impl RotatingWriter {
    pub fn new(dir: &Path) -> Result<Self, LoggingError> {
        Self::with_limits(dir, MAX_FILE_BYTES, MAX_FILES)
    }

    /// Test seam for exercising rotation without writing 50 MB.
    pub fn with_limits(dir: &Path, max_bytes: u64, max_files: usize) -> Result<Self, LoggingError> {
        fs::create_dir_all(dir).map_err(LoggingError::CreateDir)?;

        let max_bytes = max_bytes.max(1);
        let max_files = max_files.max(2);
        let path = dir.join(LOG_FILE_NAME);
        let file = open_append(&path)?;
        let len = file.metadata().map_err(LoggingError::Open)?.len();

        Ok(Self {
            inner: Arc::new(Inner {
                dir: dir.to_path_buf(),
                max_bytes,
                max_files,
                state: Mutex::new(State {
                    file: Some(file),
                    len,
                }),
            }),
        })
    }

    pub fn current_path(&self) -> PathBuf {
        self.inner.dir.join(LOG_FILE_NAME)
    }

    fn append(&self, buf: &[u8]) -> io::Result<()> {
        let mut state = self
            .inner
            .state
            .lock()
            .map_err(|_| io::Error::other("log state lock is poisoned"))?;

        if state.len > 0 && state.len + buf.len() as u64 > self.inner.max_bytes {
            self.rotate(&mut state)?;
        }

        state
            .file
            .as_mut()
            .ok_or_else(|| io::Error::other("log file is not open"))?
            .write_all(buf)?;
        state.len += buf.len() as u64;
        Ok(())
    }

    fn rotate(&self, state: &mut State) -> io::Result<()> {
        // Windows will not rename the active file while this handle is open, so
        // close it first. It is reopened below on both the success and failure
        // paths.
        if let Some(file) = state.file.as_ref() {
            file.sync_all()?;
        }
        drop(state.file.take());

        let rotate_result = self.rotate_files();

        let active = self.inner.dir.join(LOG_FILE_NAME);
        state.file =
            Some(open_append(&active).map_err(|error| io::Error::other(error.to_string()))?);
        state.len = state
            .file
            .as_ref()
            .and_then(|file| file.metadata().ok())
            .map_or(0, |metadata| metadata.len());

        rotate_result
    }

    fn rotate_files(&self) -> io::Result<()> {
        // Oldest first: remove `.max-1`, then shift every archive up by one.
        let oldest = self.inner.dir.join(archive_name(self.inner.max_files - 1));
        if oldest.exists() {
            fs::remove_file(&oldest)?;
        }

        for index in (1..self.inner.max_files - 1).rev() {
            let from = self.inner.dir.join(archive_name(index));
            if from.exists() {
                fs::rename(from, self.inner.dir.join(archive_name(index + 1)))?;
            }
        }

        let current = self.inner.dir.join(LOG_FILE_NAME);
        if current.exists() {
            fs::rename(&current, self.inner.dir.join(archive_name(1)))?;
        }

        Ok(())
    }
}

fn archive_name(index: usize) -> String {
    format!("system-scanner.{index}.log")
}

fn open_append(path: &Path) -> Result<File, LoggingError> {
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(LoggingError::Open)
}

impl Write for RotatingWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.append(buf)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        let mut state = self
            .inner
            .state
            .lock()
            .map_err(|_| io::Error::other("log state lock is poisoned"))?;
        state
            .file
            .as_mut()
            .ok_or_else(|| io::Error::other("log file is not open"))?
            .flush()
    }
}

impl<'a> MakeWriter<'a> for RotatingWriter {
    type Writer = RotatingWriter;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

/// Installs the process-wide subscriber writing to `dir`.
///
/// Returns the active log path so the caller can report it. Failure is not
/// fatal: a desktop app must still start if its log directory is unavailable.
pub fn init(dir: &Path) -> Result<PathBuf, LoggingError> {
    let writer = RotatingWriter::new(dir)?;
    let path = writer.current_path();

    tracing_subscriber::fmt()
        .with_writer(writer)
        .with_ansi(false)
        .with_target(true)
        .with_max_level(tracing::Level::INFO)
        .try_init()
        .map_err(|_| LoggingError::AlreadyInstalled)?;

    Ok(path)
}

/// Best-effort path redaction for diagnostic strings.
///
/// Full paths are never logged on purpose; this is the guard for text that
/// reaches the log through an error detail. A user-profile path keeps its
/// suffix but loses the account name; any other drive-rooted token becomes
/// `<path>`.
pub fn redact_detail(input: &str) -> String {
    input
        .split(' ')
        .map(redact_token)
        .collect::<Vec<_>>()
        .join(" ")
}

fn redact_token(token: &str) -> String {
    if !is_absolute_windows_path(token) {
        return token.to_owned();
    }

    if let Some(rest) = after_user_segment(token) {
        return format!("%USERPROFILE%\\{rest}");
    }

    "<path>".to_owned()
}

fn is_absolute_windows_path(token: &str) -> bool {
    let bytes = token.as_bytes();
    (bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'\\' | b'/'))
        || token.starts_with("\\\\")
}

fn after_user_segment(token: &str) -> Option<&str> {
    let marker = token.find("\\Users\\")?;
    let rest = &token[marker + "\\Users\\".len()..];
    let separator = rest.find('\\')?;
    Some(&rest[separator + 1..])
}

impl fmt::Debug for RotatingWriter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RotatingWriter")
            .field("dir", &self.inner.dir)
            .field("max_bytes", &self.inner.max_bytes)
            .field("max_files", &self.inner.max_files)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(label: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|elapsed| elapsed.as_nanos())
                .unwrap_or(0);
            let path = std::env::temp_dir().join(format!(
                "system-scanner-{label}-{}-{nanos}",
                std::process::id()
            ));
            fs::create_dir_all(&path).expect("temp dir must be creatable");
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn log_files(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .expect("log dir must be readable")
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn rotation_keeps_a_bounded_number_of_files() {
        let dir = TempDir::new("rotation");
        let writer = RotatingWriter::with_limits(&dir.0, 32, 3).unwrap();

        for _ in 0..6 {
            let handle = writer.clone();
            handle.append(b"0123456789abcdef\n").unwrap();
        }

        let names = log_files(&dir.0);
        assert_eq!(
            names.len(),
            3,
            "max_files includes the active log: {names:?}"
        );
        assert!(names.contains(&LOG_FILE_NAME.to_owned()));
    }

    #[test]
    fn writes_survive_a_rotation_boundary() {
        let dir = TempDir::new("content");
        let writer = RotatingWriter::with_limits(&dir.0, 16, 3).unwrap();

        for _ in 0..4 {
            writer.clone().append(b"abcdefghij\n").unwrap();
        }

        let active = fs::read_to_string(dir.0.join(LOG_FILE_NAME)).unwrap();
        let archive = fs::read_to_string(dir.0.join(archive_name(1))).unwrap();

        assert!(active.contains("abcdefghij"));
        assert!(archive.contains("abcdefghij"));
    }

    #[test]
    fn subscriber_writes_structured_events_to_the_file() {
        let dir = TempDir::new("event");
        let writer = RotatingWriter::with_limits(&dir.0, 4096, 3).unwrap();

        let subscriber = tracing_subscriber::fmt()
            .with_writer(writer)
            .with_ansi(false)
            .finish();

        tracing::subscriber::with_default(subscriber, || {
            tracing::info!(
                command = "get_app_info",
                outcome = "ok",
                "command completed"
            );
        });

        let content = fs::read_to_string(dir.0.join(LOG_FILE_NAME)).unwrap();
        assert!(content.contains("command=\"get_app_info\""), "{content}");
        assert!(content.contains("outcome=\"ok\""), "{content}");
        assert!(content.contains("command completed"), "{content}");
    }

    #[test]
    fn redacts_profile_names_and_other_absolute_paths() {
        assert_eq!(
            redact_detail("failed to open C:\\Users\\ada\\AppData\\Local\\x.txt"),
            "failed to open %USERPROFILE%\\AppData\\Local\\x.txt"
        );
        assert_eq!(
            redact_detail("scanned D:\\data\\secret.txt"),
            "scanned <path>"
        );
        assert_eq!(redact_detail("no path here"), "no path here");
    }
}
