use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::UNIX_EPOCH;

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
/// reaches the log through an error detail. See [`crate::redact`] for the rules.
pub fn redact_detail(input: &str) -> String {
    crate::redact::redact_text(input)
}

/// Aggregate size of the log directory, for the diagnostics view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogSummary {
    pub size_bytes: u64,
    pub file_count: usize,
    pub modified_ms: Option<u64>,
}

/// Bounded tail of the active log file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogTail {
    pub text: String,
    pub truncated: bool,
    pub bytes: usize,
}

/// Summarises the active log and its archives.
///
/// Returns `None` when the directory does not exist yet. That is different from
/// an empty directory: a first run has no logs, a failed setup has no directory.
pub fn summary(dir: &Path) -> io::Result<Option<LogSummary>> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };

    let mut size_bytes = 0;
    let mut file_count = 0;

    for entry in entries {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();

        if !(name.starts_with("system-scanner") && name.ends_with(".log")) {
            continue;
        }

        let metadata = entry.metadata()?;
        if metadata.is_file() {
            file_count += 1;
            size_bytes += metadata.len();
        }
    }

    let modified_ms = fs::metadata(dir.join(LOG_FILE_NAME))
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|elapsed| u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX));

    Ok(Some(LogSummary {
        size_bytes,
        file_count,
        modified_ms,
    }))
}

/// Reads at most `max_bytes` from the end of the active log.
///
/// A tail that starts in the middle of a line drops that partial line so the
/// preview never begins with a broken record.
pub fn tail(dir: &Path, max_bytes: usize) -> io::Result<Option<LogTail>> {
    let mut file = match File::open(dir.join(LOG_FILE_NAME)) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };

    let length = file.metadata()?.len();
    let limit = max_bytes.max(1) as u64;
    let start = length.saturating_sub(limit);
    let truncated = start > 0;

    if truncated {
        file.seek(SeekFrom::Start(start))?;
    }

    let mut buffer = Vec::with_capacity((length - start) as usize);
    file.take(limit).read_to_end(&mut buffer)?;

    let mut text = String::from_utf8_lossy(&buffer).into_owned();
    if truncated {
        match text.find('\n') {
            Some(index) => {
                text.drain(..=index);
            }
            None => text.clear(),
        }
    }

    Ok(Some(LogTail {
        bytes: text.len(),
        text,
        truncated,
    }))
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
    fn summary_counts_active_and_rotated_files() {
        let dir = TempDir::new("summary");
        fs::write(dir.0.join(LOG_FILE_NAME), b"12345").unwrap();
        fs::write(dir.0.join(archive_name(1)), b"678").unwrap();
        fs::write(dir.0.join("unrelated.txt"), b"ignored").unwrap();

        let summary = summary(&dir.0).unwrap().unwrap();

        assert_eq!(summary.file_count, 2);
        assert_eq!(summary.size_bytes, 8);
        assert!(summary.modified_ms.is_some());
    }

    #[test]
    fn tail_is_bounded_and_drops_the_partial_first_line() {
        let dir = TempDir::new("tail");
        fs::write(
            dir.0.join(LOG_FILE_NAME),
            b"old line\nnew line\nlast line\n",
        )
        .unwrap();

        let tail = tail(&dir.0, 16).unwrap().unwrap();

        assert!(tail.truncated);
        assert!(tail.text.contains("last line"), "{}", tail.text);
        assert!(tail.bytes <= 16);
    }

    #[test]
    fn tail_of_a_small_file_is_complete() {
        let dir = TempDir::new("small-tail");
        fs::write(dir.0.join(LOG_FILE_NAME), b"one\ntwo\n").unwrap();

        let tail = tail(&dir.0, 1024).unwrap().unwrap();

        assert!(!tail.truncated);
        assert_eq!(tail.text, "one\ntwo\n");
    }

    #[test]
    fn a_missing_log_directory_is_reported_as_absent() {
        let dir = TempDir::new("missing");
        fs::remove_dir_all(&dir.0).unwrap();

        assert!(summary(&dir.0).unwrap().is_none());
        assert!(tail(&dir.0, 64).unwrap().is_none());
    }
}
