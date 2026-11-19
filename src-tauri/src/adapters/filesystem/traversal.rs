//! Bounded, cancellable directory traversal prototype.
//!
//! Shape of the pipeline:
//!
//! ```text
//! roots -> work queue (Mutex<VecDeque>) -> N worker threads
//!                                        -> SyncSender<FileEntry> (bounded)
//!                                        -> caller consumes the Receiver
//! ```
//!
//! Properties this prototype is meant to prove:
//!
//! - Memory stays flat with the number of files: entries stream through a
//!   channel whose capacity is fixed up front. The work queue grows only with
//!   the pending-directory frontier, not with the total tree size.
//! - Backpressure is real: a slow consumer blocks producers in `send` instead of
//!   letting the queue grow.
//! - Cancellation is cooperative and observed between entries.
//! - Reparse points are never followed, so a junction loop cannot hang the walk.
//! - Every observed entry gets exactly one outcome; nothing is dropped silently
//!   (the entry budget is reported as `truncated`).
//! - A directory is opened once and never re-opened by path: the handle that was
//!   identity-checked is the handle that is enumerated (`directory.rs`).
//!
//! Driven by the in-memory scan job (`app/jobs.rs`), one scan at a time. Only
//! aggregate counters leave this module; nothing is persisted here.

use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread;
use std::time::UNIX_EPOCH;

use crate::adapters::filesystem::directory::{DirectoryEntry, DirectoryHandle};
use crate::domain::traversal::{
    EntryKind, EntryOutcome, FileEntry, FileIdentity, IdentityCheck, SkipReason, TraversalLimits,
    TraversalSummary, classify_io_error, classify_kind, compare_identity,
};

/// Upper bound on worker threads for one traversal.
pub const MAX_WORKERS: usize = 16;
/// Upper bound on roots in one request.
pub const MAX_ROOTS: usize = 64;
/// Upper bound on the entry channel capacity.
pub const MAX_CHANNEL_CAPACITY: usize = 65_536;
/// Defensive cap on a root path. Windows paths are limited to 32,767 UTF-16
/// units; a UTF-8 byte cap is a conservative proxy at the boundary.
pub const MAX_ROOT_BYTES: usize = 32_767;

/// `FILE_ATTRIBUTE_REPARSE_POINT` from the Windows file attribute set.
///
/// Kept here because it is needed only for the path-based root classification,
/// which uses `std::fs`; the value is part of the documented Win32 attribute
/// set. Directory enumeration itself goes through `directory.rs`.
#[cfg(windows)]
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;

/// Request for one traversal.
#[derive(Clone, Debug)]
pub struct TraversalRequest {
    pub roots: Vec<PathBuf>,
    pub limits: TraversalLimits,
    pub workers: usize,
    pub channel_capacity: usize,
}

impl TraversalRequest {
    /// Validate inputs supplied by a caller. Nothing here is trusted.
    pub fn validate(&self) -> Result<(), TraversalError> {
        if self.roots.is_empty() {
            return Err(TraversalError::NoRoots);
        }
        if self.roots.len() > MAX_ROOTS {
            return Err(TraversalError::TooManyRoots(self.roots.len()));
        }
        for root in &self.roots {
            if !root.is_absolute() {
                return Err(TraversalError::RelativeRoot(root.clone()));
            }
            if root.as_os_str().len() > MAX_ROOT_BYTES {
                return Err(TraversalError::RootTooLong);
            }
        }
        if !(1..=MAX_WORKERS).contains(&self.workers) {
            return Err(TraversalError::Workers(self.workers));
        }
        if !(1..=MAX_CHANNEL_CAPACITY).contains(&self.channel_capacity) {
            return Err(TraversalError::ChannelCapacity(self.channel_capacity));
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TraversalError {
    #[error("a traversal needs at least one root")]
    NoRoots,
    #[error("at most {MAX_ROOTS} roots may be traversed at once; got {0}")]
    TooManyRoots(usize),
    #[error("root path must be absolute: {}", .0.display())]
    RelativeRoot(PathBuf),
    #[error("root path exceeds {MAX_ROOT_BYTES} bytes")]
    RootTooLong,
    #[error("worker count must be between 1 and {MAX_WORKERS}; got {0}")]
    Workers(usize),
    #[error("channel capacity must be between 1 and {MAX_CHANNEL_CAPACITY}; got {0}")]
    ChannelCapacity(usize),
    #[error("a traversal worker panicked")]
    WorkerPanicked,
}

/// A running (or finished) traversal.
///
/// The caller must keep consuming [`Traversal::events`] while the walk runs.
/// Dropping the handle stops the workers; [`Traversal::join`] does that
/// explicitly and returns whatever counters were reached.
pub struct Traversal {
    events: Receiver<FileEntry>,
    queue: Arc<Queue>,
    worker: thread::JoinHandle<Result<TraversalSummary, TraversalError>>,
}

impl Traversal {
    /// Validate the request, seed the work queue, and start the workers.
    ///
    /// Returns immediately; the walk happens on background threads. Roots are
    /// classified before any worker opens them, so a root that is itself a
    /// reparse point is reported and never followed.
    pub fn start(request: TraversalRequest) -> Result<Self, TraversalError> {
        request.validate()?;
        let limits = request.limits.normalized();
        let (tx, events) = mpsc::sync_channel(request.channel_capacity);
        let queue = Arc::new(Queue::new());

        let shared = Arc::new(Shared {
            queue: Arc::clone(&queue),
            tx,
            limits,
            emitted: AtomicU64::new(0),
            truncated: AtomicBool::new(false),
        });

        for root in request.roots {
            seed_root(&shared, root);
        }

        let workers = request.workers;
        let worker = thread::spawn(move || run(shared, workers));

        Ok(Self {
            events,
            queue,
            worker,
        })
    }

    /// Stream of observed entries. Consume it before [`Traversal::join`] to see
    /// everything; the channel is bounded, so a stalled consumer stalls the walk.
    pub fn events(&self) -> &Receiver<FileEntry> {
        &self.events
    }

    /// A cloneable handle that asks the walk to stop at the next entry
    /// boundary. Usable from another thread while the caller keeps draining
    /// [`Traversal::events`].
    pub fn cancel_handle(&self) -> CancelHandle {
        CancelHandle {
            queue: Arc::clone(&self.queue),
        }
    }

    /// Stop the walk if it is still running, then report the counters.
    ///
    /// Any buffered events are discarded: drain [`Traversal::events`] first when
    /// you need them.
    pub fn join(self) -> Result<TraversalSummary, TraversalError> {
        let Self { events, worker, .. } = self;
        drop(events);
        match worker.join() {
            Ok(result) => result,
            Err(_) => Err(TraversalError::WorkerPanicked),
        }
    }
}

/// Cancels one traversal. Cheap to clone; safe to call any number of times,
/// including after the walk has finished.
#[derive(Clone)]
pub struct CancelHandle {
    queue: Arc<Queue>,
}

impl CancelHandle {
    pub fn cancel(&self) {
        self.queue.cancel();
    }
}

/// One unit of work. `leaf` is precomputed for roots that are not directories.
///
/// `expected` is the identity the parent recorded for this directory. It is
/// compared with the identity read from the handle that is actually opened, so
/// a directory swapped for another object is never scanned under the old name.
struct Job {
    path: PathBuf,
    depth: u32,
    leaf: Option<FileEntry>,
    expected: Option<FileIdentity>,
}

#[derive(Default)]
struct QueueState {
    jobs: VecDeque<Job>,
    active: usize,
}

/// Shared work queue with one condition variable.
///
/// A worker that finds no job waits until another worker adds one or until the
/// active count drops to zero. That is the termination signal.
struct Queue {
    state: Mutex<QueueState>,
    signal: Condvar,
    cancelled: AtomicBool,
    stopped: AtomicBool,
    finished: AtomicBool,
}

impl Queue {
    fn new() -> Self {
        Self {
            state: Mutex::new(QueueState::default()),
            signal: Condvar::new(),
            cancelled: AtomicBool::new(false),
            stopped: AtomicBool::new(false),
            finished: AtomicBool::new(false),
        }
    }

    fn is_stopping(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst) || self.stopped.load(Ordering::SeqCst)
    }

    fn push(&self, job: Job) {
        if self.is_stopping() {
            return;
        }
        let mut state = lock(&self.state);
        state.jobs.push_back(job);
        drop(state);
        self.signal.notify_one();
    }

    fn pop(&self) -> Option<Job> {
        let mut state = lock(&self.state);
        loop {
            // Completion is decided before the stop flags: if a cancel arrives
            // just after the last job finished, the walk still completed, and
            // reporting it as `cancelled` would understate the result.
            if state.jobs.is_empty() && state.active == 0 {
                self.finished.store(true, Ordering::SeqCst);
                self.signal.notify_all();
                return None;
            }
            if self.is_stopping() {
                return None;
            }
            if let Some(job) = state.jobs.pop_front() {
                state.active = state.active.saturating_add(1);
                return Some(job);
            }
            state = wait(&self.signal, state);
        }
    }

    /// Called once a worker is done with a job, including on unwind.
    fn finish(&self) {
        let mut state = lock(&self.state);
        state.active = state.active.saturating_sub(1);
        drop(state);
        self.signal.notify_all();
    }

    fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        self.signal.notify_all();
    }

    /// Internal stop (entry budget). Distinct from user cancellation.
    fn stop(&self) {
        self.stopped.store(true, Ordering::SeqCst);
        self.signal.notify_all();
    }
}

struct Shared {
    queue: Arc<Queue>,
    tx: SyncSender<FileEntry>,
    limits: TraversalLimits,
    emitted: AtomicU64,
    truncated: AtomicBool,
}

impl Shared {
    /// Count and send one entry. Returns `false` when the walk must stop.
    fn emit(&self, summary: &mut TraversalSummary, entry: FileEntry) -> bool {
        let slot = self.emitted.fetch_add(1, Ordering::SeqCst);
        if let Some(max) = self.limits.max_entries
            && slot >= max
        {
            self.truncated.store(true, Ordering::SeqCst);
            self.queue.stop();
            return false;
        }
        if self.queue.is_stopping() {
            return false;
        }

        summary.record(&entry);
        self.tx.send(entry).is_ok()
    }
}

/// Keeps the queue's active count correct even if a worker panics.
struct ActiveJob<'a> {
    queue: &'a Queue,
}

impl Drop for ActiveJob<'_> {
    fn drop(&mut self) {
        self.queue.finish();
    }
}

fn run(shared: Arc<Shared>, workers: usize) -> Result<TraversalSummary, TraversalError> {
    let mut handles = Vec::with_capacity(workers);
    for _ in 0..workers {
        let shared = Arc::clone(&shared);
        handles.push(thread::spawn(move || {
            let mut local = TraversalSummary::default();
            walk(&shared, &mut local);
            local
        }));
    }

    let mut summary = TraversalSummary::default();
    let mut panicked = false;
    for handle in handles {
        match handle.join() {
            Ok(local) => summary.merge(local),
            Err(_) => panicked = true,
        }
    }

    summary.truncated |= shared.truncated.load(Ordering::SeqCst);
    summary.cancelled = shared.queue.cancelled.load(Ordering::SeqCst)
        && !shared.queue.finished.load(Ordering::SeqCst);

    if panicked {
        return Err(TraversalError::WorkerPanicked);
    }
    Ok(summary)
}

fn walk(shared: &Shared, summary: &mut TraversalSummary) {
    while let Some(job) = shared.queue.pop() {
        let _active = ActiveJob {
            queue: &shared.queue,
        };
        let keep_going = match job.leaf {
            Some(entry) => shared.emit(summary, entry),
            None => visit_directory(shared, summary, &job.path, job.depth, job.expected),
        };
        if !keep_going {
            break;
        }
    }
}

/// Classify a root before queueing it.
///
/// Only real directories become open jobs. A root that is a junction, a file,
/// or missing is turned into a leaf entry, so `read_dir` never follows it.
fn seed_root(shared: &Shared, root: PathBuf) {
    match fs::symlink_metadata(&root) {
        Ok(metadata) => {
            let reparse = is_reparse_point(&metadata);
            let kind = classify_kind(
                metadata.file_type().is_dir(),
                metadata.file_type().is_file(),
                reparse,
            );

            if kind == EntryKind::Directory {
                // Record the root's identity now, so the open that follows can
                // prove it reached the same object. A failed open here is
                // reported by the visitor, not hidden.
                let expected = DirectoryHandle::open(&root)
                    .ok()
                    .and_then(|opened| opened.identity());
                shared.queue.push(Job {
                    path: root,
                    depth: 0,
                    leaf: None,
                    expected,
                });
                return;
            }

            let outcome = match kind {
                EntryKind::File => EntryOutcome::Ok,
                EntryKind::ReparsePoint => EntryOutcome::Skipped(SkipReason::ReparsePoint),
                _ => EntryOutcome::Unsupported,
            };
            let size = match kind {
                EntryKind::File => Some(metadata.len()),
                _ => None,
            };
            let entry = FileEntry {
                path: root.clone(),
                depth: 0,
                kind,
                outcome,
                size_bytes: size,
                modified_at_ms: modified_at_ms(&metadata),
            };
            shared.queue.push(Job {
                path: root,
                depth: 0,
                leaf: Some(entry),
                expected: None,
            });
        }
        Err(error) => {
            let entry = FileEntry {
                path: root.clone(),
                depth: 0,
                kind: EntryKind::Other,
                outcome: classify_io_error(error.kind()),
                size_bytes: None,
                modified_at_ms: None,
            };
            shared.queue.push(Job {
                path: root,
                depth: 0,
                leaf: Some(entry),
                expected: None,
            });
        }
    }
}

fn visit_directory(
    shared: &Shared,
    summary: &mut TraversalSummary,
    path: &Path,
    depth: u32,
    expected: Option<FileIdentity>,
) -> bool {
    // Open once, without following a link in the final component, and use that
    // one handle for the identity check and for enumeration. Re-opening the
    // path afterwards is what made the old by-path check meaningless (F-01).
    let mut handle = match DirectoryHandle::open(path) {
        Ok(handle) => handle,
        Err(error) => {
            return shared.emit(
                summary,
                FileEntry {
                    path: path.to_path_buf(),
                    depth,
                    kind: EntryKind::Directory,
                    outcome: classify_io_error(error.kind()),
                    size_bytes: None,
                    modified_at_ms: None,
                },
            );
        }
    };

    let info = match handle.info() {
        Ok(info) => info,
        Err(error) => {
            return shared.emit(
                summary,
                FileEntry {
                    path: path.to_path_buf(),
                    depth,
                    kind: EntryKind::Directory,
                    outcome: classify_io_error(error.kind()),
                    size_bytes: None,
                    modified_at_ms: None,
                },
            );
        }
    };

    // Defence in depth: the open asked for the reparse point itself, so this
    // attribute describes the object the handle is bound to.
    if info.is_reparse_point {
        return shared.emit(
            summary,
            FileEntry {
                path: path.to_path_buf(),
                depth,
                kind: EntryKind::ReparsePoint,
                outcome: EntryOutcome::Skipped(SkipReason::ReparsePoint),
                size_bytes: None,
                modified_at_ms: info.modified_at_ms,
            },
        );
    }

    if compare_identity(expected, handle.identity()) == IdentityCheck::Changed {
        // The path now resolves to a different object than the one the parent
        // recorded, so its contents are not the recorded directory's contents.
        return shared.emit(
            summary,
            FileEntry {
                path: path.to_path_buf(),
                depth,
                kind: EntryKind::Directory,
                outcome: EntryOutcome::Disappeared,
                size_bytes: None,
                modified_at_ms: info.modified_at_ms,
            },
        );
    }

    if !shared.emit(
        summary,
        FileEntry {
            path: path.to_path_buf(),
            depth,
            kind: EntryKind::Directory,
            outcome: EntryOutcome::Ok,
            size_bytes: None,
            modified_at_ms: info.modified_at_ms,
        },
    ) {
        return false;
    }

    loop {
        let batch = match handle.read_batch() {
            Ok(batch) => batch,
            Err(_) => {
                // The directory stream failed mid-walk; there is no child path
                // to attribute. Count it honestly and keep the rest.
                summary.failed = summary.failed.saturating_add(1);
                return true;
            }
        };
        if batch.is_empty() {
            return true;
        }

        for entry in batch {
            if shared.queue.is_stopping() {
                return false;
            }
            if !visit_child(shared, summary, depth, path, entry) {
                return false;
            }
        }
    }
}

fn visit_child(
    shared: &Shared,
    summary: &mut TraversalSummary,
    parent_depth: u32,
    parent: &Path,
    entry: DirectoryEntry,
) -> bool {
    let depth = parent_depth.saturating_add(1);
    let path = parent.join(&entry.name);
    let kind = classify_kind(entry.is_dir, entry.is_file, entry.is_reparse_point);

    if kind == EntryKind::ReparsePoint {
        return shared.emit(
            summary,
            FileEntry {
                path,
                depth,
                kind,
                outcome: EntryOutcome::Skipped(SkipReason::ReparsePoint),
                size_bytes: None,
                modified_at_ms: entry.modified_at_ms,
            },
        );
    }

    if kind == EntryKind::Directory {
        if depth > shared.limits.max_depth {
            return shared.emit(
                summary,
                FileEntry {
                    path,
                    depth,
                    kind,
                    outcome: EntryOutcome::Skipped(SkipReason::DepthLimit),
                    size_bytes: None,
                    modified_at_ms: entry.modified_at_ms,
                },
            );
        }
        shared.queue.push(Job {
            path,
            depth,
            leaf: None,
            expected: entry.identity,
        });
        return true;
    }

    if kind == EntryKind::Other {
        return shared.emit(
            summary,
            FileEntry {
                path,
                depth,
                kind,
                outcome: EntryOutcome::Unsupported,
                size_bytes: None,
                modified_at_ms: entry.modified_at_ms,
            },
        );
    }

    shared.emit(
        summary,
        FileEntry {
            path,
            depth,
            kind,
            outcome: EntryOutcome::Ok,
            size_bytes: Some(entry.size_bytes),
            modified_at_ms: entry.modified_at_ms,
        },
    )
}

#[cfg(windows)]
fn is_reparse_point(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn is_reparse_point(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}

fn modified_at_ms(metadata: &fs::Metadata) -> Option<i64> {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .and_then(|elapsed| i64::try_from(elapsed.as_millis()).ok())
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn wait<'a, T>(signal: &Condvar, guard: MutexGuard<'a, T>) -> MutexGuard<'a, T> {
    signal
        .wait(guard)
        .unwrap_or_else(|poisoned| poisoned.into_inner())
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
                "system-scanner-walk-{label}-{}-{nanos}",
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

    fn request(root: &Path) -> TraversalRequest {
        request_with(root, TraversalLimits::default())
    }

    fn request_with(root: &Path, limits: TraversalLimits) -> TraversalRequest {
        TraversalRequest {
            roots: vec![root.to_path_buf()],
            limits,
            workers: 2,
            channel_capacity: 64,
        }
    }

    fn write_file(path: &Path, bytes: usize) {
        fs::write(path, vec![0u8; bytes]).expect("fixture file must be writable");
    }

    fn collect(traversal: Traversal) -> (Vec<FileEntry>, TraversalSummary) {
        let mut entries = Vec::new();
        for entry in traversal.events() {
            entries.push(entry);
        }
        let summary = traversal.join().expect("traversal must finish");
        (entries, summary)
    }

    #[test]
    fn a_nested_tree_is_streamed_with_one_outcome_per_entry() {
        let temp = TempDir::new("nested");
        let root = temp.0.join("root");
        fs::create_dir_all(root.join("sub")).expect("fixture dir");
        fs::create_dir_all(root.join("empty")).expect("fixture dir");
        write_file(&root.join("alpha.txt"), 5);
        write_file(&root.join("sub").join("beta.bin"), 3);
        write_file(&root.join("zero.dat"), 0);

        let (entries, summary) =
            collect(Traversal::start(request(&root)).expect("start must succeed"));

        assert_eq!(summary.entries, 6);
        assert_eq!(summary.directories, 3);
        assert_eq!(summary.files, 3);
        assert_eq!(summary.bytes, 8);
        assert_eq!(summary.skipped, 0);
        assert!(!summary.truncated);
        assert!(!summary.cancelled);
        assert!(
            entries
                .iter()
                .all(|entry| entry.outcome == EntryOutcome::Ok)
        );
        assert!(
            entries
                .iter()
                .any(|entry| entry.path == root.join("sub").join("beta.bin")
                    && entry.size_bytes == Some(3))
        );
        assert!(
            entries
                .iter()
                .any(|entry| entry.path == root.join("zero.dat") && entry.size_bytes == Some(0))
        );
    }

    #[test]
    fn a_directory_larger_than_one_enumeration_buffer_is_walked_completely() {
        let temp = TempDir::new("large-directory");
        let root = temp.0.join("root");
        fs::create_dir_all(&root).expect("fixture dir");
        for index in 0..1500 {
            write_file(&root.join(format!("file-{index:04}.bin")), 1);
        }

        let (entries, summary) =
            collect(Traversal::start(request(&root)).expect("start must succeed"));

        assert_eq!(summary.files, 1500, "every file must be reported");
        assert_eq!(summary.entries, 1501, "the root directory and its files");
        assert_eq!(summary.bytes, 1500);
        assert_eq!(entries.len() as u64, summary.entries);
    }

    #[cfg(windows)]
    #[test]
    fn a_junction_root_is_reported_and_never_enumerated() {
        let temp = TempDir::new("junction-root");
        let target = temp.0.join("target");
        let root = temp.0.join("root-link");
        fs::create_dir_all(&target).expect("fixture dir");
        write_file(&target.join("hidden.bin"), 5);
        let output = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&root)
            .arg(&target)
            .output()
            .expect("cmd must be available");
        assert!(
            output.status.success(),
            "junction fixture must be created: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let (entries, summary) =
            collect(Traversal::start(request(&root)).expect("start must succeed"));

        assert_eq!(entries.len(), 1, "the link is reported once");
        assert_eq!(entries[0].kind, EntryKind::ReparsePoint);
        assert_eq!(
            entries[0].outcome,
            EntryOutcome::Skipped(SkipReason::ReparsePoint)
        );
        assert_eq!(summary.files, 0, "content behind the link is not scanned");
        assert_eq!(summary.bytes, 0);

        fs::remove_dir(&root).expect("junction must be removable");
    }

    #[test]
    fn an_empty_root_reports_only_the_directory() {
        let temp = TempDir::new("empty");
        let root = temp.0.join("root");
        fs::create_dir_all(&root).expect("fixture dir");

        let (entries, summary) =
            collect(Traversal::start(request(&root)).expect("start must succeed"));

        assert_eq!(entries.len(), 1);
        assert_eq!(summary.directories, 1);
        assert_eq!(summary.files, 0);
    }

    #[test]
    fn a_missing_root_is_disappeared_not_a_scan_failure() {
        let temp = TempDir::new("missing");
        let root = temp.0.join("does-not-exist");

        let (entries, summary) =
            collect(Traversal::start(request(&root)).expect("start must succeed"));

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].outcome, EntryOutcome::Disappeared);
        assert_eq!(summary.entries, 1);
        assert_eq!(summary.disappeared, 1);
    }

    #[test]
    fn a_file_root_is_reported_as_a_file() {
        let temp = TempDir::new("file-root");
        let root = temp.0.join("solo.bin");
        write_file(&root, 7);

        let (entries, summary) =
            collect(Traversal::start(request(&root)).expect("start must succeed"));

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].kind, EntryKind::File);
        assert_eq!(entries[0].outcome, EntryOutcome::Ok);
        assert_eq!(summary.files, 1);
        assert_eq!(summary.bytes, 7);
    }

    #[test]
    fn the_depth_limit_skips_deeper_directories_without_hiding_them() {
        let temp = TempDir::new("depth");
        let root = temp.0.join("root");
        let nested = root.join("sub").join("nested");
        fs::create_dir_all(&nested).expect("fixture dir");
        write_file(&nested.join("hidden.txt"), 1);

        let limits = TraversalLimits {
            max_depth: 1,
            max_entries: None,
        };
        let (entries, summary) =
            collect(Traversal::start(request_with(&root, limits)).expect("start must succeed"));

        assert_eq!(summary.entries, 3);
        assert_eq!(summary.skipped, 1);
        assert_eq!(summary.files, 0);
        assert!(entries.iter().any(|entry| entry.path == nested
            && entry.outcome == EntryOutcome::Skipped(SkipReason::DepthLimit)));
        assert!(
            !entries
                .iter()
                .any(|entry| entry.path.ends_with("hidden.txt"))
        );
    }

    #[test]
    fn the_entry_budget_stops_the_walk_and_is_reported_as_truncated() {
        let temp = TempDir::new("budget");
        let root = temp.0.join("root");
        fs::create_dir_all(&root).expect("fixture dir");
        for index in 0..10 {
            write_file(&root.join(format!("file-{index}.bin")), 1);
        }

        let limits = TraversalLimits {
            max_depth: 64,
            max_entries: Some(5),
        };
        let (entries, summary) =
            collect(Traversal::start(request_with(&root, limits)).expect("start must succeed"));

        assert_eq!(entries.len(), 5);
        assert_eq!(summary.entries, 5);
        assert!(summary.truncated);
        assert!(!summary.cancelled);
    }

    #[test]
    fn cancellation_stops_the_walk_and_keeps_partial_counts() {
        let temp = TempDir::new("cancel");
        let root = temp.0.join("root");
        for directory in 0..30 {
            let path = root.join(format!("dir-{directory}"));
            fs::create_dir_all(&path).expect("fixture dir");
            for file in 0..10 {
                write_file(&path.join(format!("file-{file}.bin")), 1);
            }
        }

        let traversal = Traversal::start(TraversalRequest {
            roots: vec![root],
            limits: TraversalLimits::default(),
            workers: 2,
            channel_capacity: 1,
        })
        .expect("start must succeed");
        traversal.cancel_handle().cancel();

        let (entries, summary) = collect(traversal);

        assert!(summary.cancelled);
        assert!(!summary.truncated);
        assert!(
            summary.entries < 331,
            "walk must stop early: {}",
            summary.entries
        );
        assert_eq!(entries.len() as u64, summary.entries);
    }

    #[test]
    fn a_cancel_after_the_last_entry_is_not_reported_as_cancelled() {
        let temp = TempDir::new("cancel-late");
        let root = temp.0.join("root");
        fs::create_dir_all(&root).expect("fixture dir");
        write_file(&root.join("only.bin"), 1);

        let traversal = Traversal::start(request(&root)).expect("start must succeed");
        // Draining to the end of the channel means every worker has finished.
        let mut entries = Vec::new();
        for entry in traversal.events() {
            entries.push(entry);
        }
        traversal.cancel_handle().cancel();

        let summary = traversal.join().expect("traversal must finish");

        assert_eq!(entries.len(), 2, "root directory and one file");
        assert_eq!(summary.entries, 2);
        assert!(
            !summary.cancelled,
            "a completed walk is not a cancelled walk"
        );
        assert!(!summary.truncated);
    }

    #[test]
    fn unicode_names_are_preserved() {
        let temp = TempDir::new("unicode");
        let root = temp.0.join("root");
        let directory = root.join("résumé-日本語");
        fs::create_dir_all(&directory).expect("fixture dir");
        let file = directory.join("naïve.txt");
        write_file(&file, 2);

        let (entries, summary) =
            collect(Traversal::start(request(&root)).expect("start must succeed"));

        assert_eq!(summary.files, 1);
        assert!(
            entries
                .iter()
                .any(|entry| entry.path == file && entry.outcome == EntryOutcome::Ok)
        );
    }

    #[test]
    fn a_path_longer_than_the_legacy_limit_is_traversed() {
        let temp = TempDir::new("long");
        let root = temp.0.join("root");
        fs::create_dir_all(&root).expect("fixture dir");

        let mut deep = root.clone();
        while deep.as_os_str().len() <= 300 {
            deep.push("padding-directory");
        }
        fs::create_dir_all(&deep).expect("long path fixture must be creatable");
        let file = deep.join("deep.txt");
        write_file(&file, 1);

        let (entries, summary) =
            collect(Traversal::start(request(&root)).expect("start must succeed"));

        assert_eq!(summary.files, 1);
        assert!(entries.iter().any(|entry| entry.path == file));
    }

    #[cfg(windows)]
    #[test]
    fn a_junction_loop_is_reported_and_never_followed() {
        let temp = TempDir::new("junction");
        let root = temp.0.join("root");
        fs::create_dir_all(root.join("sub")).expect("fixture dir");
        write_file(&root.join("sub").join("file.bin"), 1);

        let link = root.join("loop");
        let output = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&link)
            .arg(&root)
            .output()
            .expect("cmd must be available");
        assert!(
            output.status.success(),
            "junction fixture must be created: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let (entries, summary) =
            collect(Traversal::start(request(&root)).expect("start must succeed"));

        let junction = entries
            .iter()
            .find(|entry| entry.path == link)
            .expect("junction must be reported once");
        assert_eq!(junction.kind, EntryKind::ReparsePoint);
        assert_eq!(
            junction.outcome,
            EntryOutcome::Skipped(SkipReason::ReparsePoint)
        );
        assert_eq!(summary.entries, 4, "root, sub, file, and the junction");
        assert_eq!(summary.reparse_points, 1);

        fs::remove_dir(&link).expect("junction must be removable");
    }

    /// Denies directory listing to the current user and restores it on drop,
    /// so the temp directory stays deletable even if an assertion fails.
    #[cfg(windows)]
    struct DeniedListing {
        path: PathBuf,
        user: String,
    }

    #[cfg(windows)]
    impl DeniedListing {
        fn apply(path: &Path) -> Self {
            let user = std::env::var("USERNAME").expect("USERNAME must be set");
            let guard = Self {
                path: path.to_path_buf(),
                user,
            };
            let output = std::process::Command::new("icacls")
                .arg(path)
                .arg("/deny")
                .arg(format!("{}:(RD)", guard.user))
                .output()
                .expect("icacls must be available");
            assert!(
                output.status.success(),
                "deny ACE must be applied: {}",
                String::from_utf8_lossy(&output.stdout)
            );
            guard
        }
    }

    #[cfg(windows)]
    impl Drop for DeniedListing {
        fn drop(&mut self) {
            let _ = std::process::Command::new("icacls")
                .arg(&self.path)
                .arg("/remove:d")
                .arg(&self.user)
                .output();
        }
    }

    #[cfg(windows)]
    #[test]
    fn a_directory_the_user_cannot_list_is_denied_and_the_walk_is_partial() {
        use crate::domain::traversal::Completeness;

        let temp = TempDir::new("denied");
        let root = temp.0.join("root");
        let locked = root.join("locked");
        fs::create_dir_all(&locked).expect("fixture dir");
        write_file(&root.join("readable.txt"), 4);
        write_file(&locked.join("secret.txt"), 9);
        let _guard = DeniedListing::apply(&locked);

        let (entries, summary) =
            collect(Traversal::start(request(&root)).expect("start must succeed"));

        let denied = entries
            .iter()
            .find(|entry| entry.path == locked)
            .expect("the locked directory must be reported");
        assert_eq!(denied.outcome, EntryOutcome::Denied);
        assert!(
            !entries
                .iter()
                .any(|entry| entry.path.ends_with("secret.txt"))
        );
        assert_eq!(summary.denied, 1);
        assert_eq!(summary.bytes, 4, "denied contents must not be counted");
        assert_eq!(summary.completeness(), Completeness::Partial);
    }

    #[test]
    fn a_clean_walk_is_complete_and_a_budget_stopped_one_is_truncated() {
        use crate::domain::traversal::Completeness;

        let temp = TempDir::new("completeness");
        let root = temp.0.join("root");
        fs::create_dir_all(&root).expect("fixture dir");
        write_file(&root.join("a.bin"), 1);

        let (_, summary) = collect(Traversal::start(request(&root)).expect("start must succeed"));
        assert_eq!(summary.completeness(), Completeness::Complete);

        let limits = TraversalLimits {
            max_depth: 64,
            max_entries: Some(1),
        };
        let (_, summary) =
            collect(Traversal::start(request_with(&root, limits)).expect("start must succeed"));
        assert_eq!(summary.completeness(), Completeness::Truncated);
    }

    #[test]
    fn requests_are_validated_before_any_thread_starts() {
        let temp = TempDir::new("validate");
        let root = temp.0.join("root");
        fs::create_dir_all(&root).expect("fixture dir");
        let base = request(&root);

        let mut invalid = base.clone();
        invalid.roots.clear();
        assert_eq!(invalid.validate(), Err(TraversalError::NoRoots));

        let mut invalid = base.clone();
        invalid.roots = vec![PathBuf::from("relative\\path")];
        assert!(matches!(
            invalid.validate(),
            Err(TraversalError::RelativeRoot(_))
        ));

        let mut invalid = base.clone();
        invalid.roots = vec![root.clone(); MAX_ROOTS + 1];
        assert_eq!(
            invalid.validate(),
            Err(TraversalError::TooManyRoots(MAX_ROOTS + 1))
        );

        let mut invalid = base.clone();
        invalid.workers = 0;
        assert_eq!(invalid.validate(), Err(TraversalError::Workers(0)));
        invalid.workers = MAX_WORKERS + 1;
        assert_eq!(
            invalid.validate(),
            Err(TraversalError::Workers(MAX_WORKERS + 1))
        );

        let mut invalid = base.clone();
        invalid.channel_capacity = 0;
        assert_eq!(invalid.validate(), Err(TraversalError::ChannelCapacity(0)));
        invalid.channel_capacity = MAX_CHANNEL_CAPACITY + 1;
        assert_eq!(
            invalid.validate(),
            Err(TraversalError::ChannelCapacity(MAX_CHANNEL_CAPACITY + 1))
        );

        assert!(base.validate().is_ok());
    }
}
