//! Scan job supervisor: one traversal at a time, cancellable, in memory.
//!
//! ```text
//! start -> validate -> claim the single slot -> Traversal::start
//!       -> drain thread: count entries, pace progress, then finalize
//! cancel -> Running -> Cancelling -> CancelHandle::cancel
//! ```
//!
//! Only aggregate counters are kept or emitted. Entries are counted and dropped
//! as they arrive, so memory does not grow with the tree. Nothing is persisted
//! (B-024) and no path leaves this module.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use crate::adapters::filesystem::traversal::{
    CancelHandle, Traversal, TraversalError, TraversalRequest,
};
use crate::domain::job::{JobState, ProgressGate};
use crate::domain::traversal::{Completeness, TraversalLimits, TraversalSummary};
use crate::dto::scan::{
    ScanCancelledEvent, ScanCompletedEvent, ScanCountersDto, ScanEvent, ScanFailedEvent,
    ScanProgressEvent, ScanStatusDto,
};
use crate::error::{AppError, ErrorCode};
use crate::ports::sessions::{ScanSessionStore, SessionFinish, SessionStart};

/// Receives events from the drain thread. It must not block for long: a slow
/// sink stalls the walk through the bounded channel (that is the backpressure).
pub type EventSink = Arc<dyn Fn(ScanEvent) + Send + Sync>;

/// Why a job was cancelled; published in `scan:cancelled`.
const REASON_USER: &str = "user_requested";
const REASON_SHUTDOWN: &str = "shutdown";

/// Version of the traversal policy recorded with each session (reparse points
/// never followed, depth 64, entry ceiling). Bump when that policy changes.
const POLICY_VERSION: &str = "traversal-1";

static JOB_SEQ: AtomicU64 = AtomicU64::new(1);

/// Resource budgets for one scan. Chosen in Rust; never taken from IPC.
#[derive(Clone, Copy, Debug)]
pub struct ScanTuning {
    pub workers: usize,
    pub channel_capacity: usize,
    /// Minimum gap between `scan:progress` events (100 ms = at most 10/s).
    pub progress_interval_ms: u64,
}

impl Default for ScanTuning {
    fn default() -> Self {
        Self {
            workers: 4,
            channel_capacity: 1024,
            progress_interval_ms: 100,
        }
    }
}

struct JobRecord {
    id: String,
    state: JobState,
    started: Instant,
    finished_elapsed_ms: Option<u64>,
    counters: TraversalSummary,
    seq: u64,
    cancel: CancelHandle,
    cancel_reason: &'static str,
    completeness: Option<Completeness>,
    error_code: Option<ErrorCode>,
}

/// Owns the single scan slot. Cheap to clone; all clones share one slot.
#[derive(Clone)]
pub struct Supervisor {
    slot: Arc<Mutex<Option<JobRecord>>>,
    tuning: ScanTuning,
    store: Option<Arc<dyn ScanSessionStore>>,
}

impl Default for Supervisor {
    fn default() -> Self {
        Self::new()
    }
}

impl Supervisor {
    pub fn new() -> Self {
        Self::with_tuning(ScanTuning::default())
    }

    pub fn with_tuning(tuning: ScanTuning) -> Self {
        Self {
            slot: Arc::new(Mutex::new(None)),
            tuning,
            store: None,
        }
    }

    /// Record sessions in `store`. Persistence is best-effort: a failed write is
    /// logged and the scan carries on, because the app also runs without a
    /// database.
    #[must_use]
    pub fn with_store(mut self, store: Arc<dyn ScanSessionStore>) -> Self {
        self.store = Some(store);
        self
    }

    /// Validate, claim the slot, and start the walk. Returns the job ID at once.
    ///
    /// Fails with `invalid_input` for a bad request and `busy` while another
    /// scan is queued, running, or cancelling.
    pub fn start(
        &self,
        roots: Vec<PathBuf>,
        max_entries: u64,
        sink: EventSink,
    ) -> Result<String, AppError> {
        let request = TraversalRequest {
            roots,
            limits: TraversalLimits {
                max_entries: Some(max_entries),
                ..TraversalLimits::default()
            },
            workers: self.tuning.workers,
            channel_capacity: self.tuning.channel_capacity,
        };
        request.validate().map_err(invalid_input)?;

        let mut slot = lock(&self.slot);
        if slot.as_ref().is_some_and(|job| job.state.is_active()) {
            return Err(AppError::new(ErrorCode::Busy));
        }

        let traversal = Traversal::start(request).map_err(invalid_input)?;
        let id = next_job_id();
        let started = Instant::now();

        let mut record = JobRecord {
            id: id.clone(),
            state: JobState::Queued,
            started,
            finished_elapsed_ms: None,
            counters: TraversalSummary::default(),
            seq: 0,
            cancel: traversal.cancel_handle(),
            cancel_reason: REASON_USER,
            completeness: None,
            error_code: None,
        };
        transition(&mut record, JobState::Running);
        *slot = Some(record);
        drop(slot);

        // Written before the drain thread exists, so `finish` can never reach the
        // store ahead of `begin`. The options hold budgets only, never a path.
        if let Some(store) = &self.store {
            let options = serde_json::json!({
                "maxEntries": max_entries,
                "workers": self.tuning.workers,
                "channelCapacity": self.tuning.channel_capacity,
            });
            let _ = store.begin(&SessionStart {
                id: id.clone(),
                started_at_ms: unix_ms(),
                options_json: options.to_string(),
                engine_version: env!("CARGO_PKG_VERSION"),
                policy_version: POLICY_VERSION,
            });
        }

        let shared = Arc::clone(&self.slot);
        let interval = self.tuning.progress_interval_ms;
        let job_id = id.clone();
        let store = self.store.clone();
        let spawned = thread::Builder::new()
            .name("scan-drain".into())
            .spawn(move || drain(shared, job_id, traversal, sink, interval, started, store));

        if spawned.is_err() {
            // The closure (and the traversal inside it) was dropped, which stops
            // the workers. Record the failure instead of leaving a phantom job.
            let mut slot = lock(&self.slot);
            if let Some(job) = slot.as_mut().filter(|job| job.id == id) {
                job.error_code = Some(ErrorCode::Internal);
                transition(job, JobState::Failed);
            }
            drop(slot);
            if let Some(store) = &self.store {
                let _ = store.finish(
                    &id,
                    &SessionFinish {
                        status: JobState::Failed.code(),
                        finished_at_ms: unix_ms(),
                        completeness: None,
                        counters: TraversalSummary::default(),
                        cancel_reason: None,
                        error_code: Some(ErrorCode::Internal.as_str()),
                    },
                );
            }
            return Err(AppError::new(ErrorCode::Internal));
        }

        tracing::info!(job_id = %id, "scan started");
        Ok(id)
    }

    /// Request cancellation. Returns `true` when the job is (now) cancelling.
    /// Idempotent; `false` means there is nothing to cancel.
    pub fn cancel(&self, job_id: Option<&str>) -> bool {
        self.request_cancel(job_id, REASON_USER)
    }

    /// Stop any running scan because the window is closing.
    pub fn shutdown(&self) {
        let _ = self.request_cancel(None, REASON_SHUTDOWN);
    }

    /// Authoritative state of the current or most recent job.
    pub fn status(&self, job_id: Option<&str>) -> Result<ScanStatusDto, AppError> {
        let slot = lock(&self.slot);
        let job = slot
            .as_ref()
            .filter(|job| job_id.is_none_or(|wanted| wanted == job.id))
            .ok_or_else(|| AppError::new(ErrorCode::NotFound))?;

        Ok(ScanStatusDto {
            job_id: job.id.clone(),
            state: job.state.code(),
            seq: job.seq,
            elapsed_ms: job
                .finished_elapsed_ms
                .unwrap_or_else(|| elapsed_ms(job.started)),
            counters: ScanCountersDto::from(&job.counters),
            completeness: job.completeness.map(Completeness::code),
            error_code: job.error_code.map(ErrorCode::as_str),
        })
    }

    fn request_cancel(&self, job_id: Option<&str>, reason: &'static str) -> bool {
        let mut slot = lock(&self.slot);
        let Some(job) = slot
            .as_mut()
            .filter(|job| job_id.is_none_or(|wanted| wanted == job.id))
        else {
            return false;
        };

        match job.state {
            JobState::Running => {
                job.cancel_reason = reason;
                transition(job, JobState::Cancelling);
                job.cancel.cancel();
                tracing::info!(job_id = %job.id, reason, "scan cancel requested");
                true
            }
            JobState::Cancelling => true,
            _ => false,
        }
    }
}

/// Runs on its own thread: counts entries, paces progress, finalizes the job.
fn drain(
    slot: Arc<Mutex<Option<JobRecord>>>,
    id: String,
    traversal: Traversal,
    sink: EventSink,
    interval_ms: u64,
    started: Instant,
    store: Option<Arc<dyn ScanSessionStore>>,
) {
    let mut gate = ProgressGate::new(interval_ms);
    let mut seen = TraversalSummary::default();
    let mut seq = 0u64;

    for entry in traversal.events() {
        seen.record(&entry);
        let elapsed = elapsed_ms(started);
        if !gate.due(elapsed) {
            continue;
        }

        seq += 1;
        {
            let mut guard = lock(&slot);
            if let Some(job) = guard.as_mut().filter(|job| job.id == id) {
                job.counters = seen.clone();
                job.seq = seq;
            }
        }
        sink(ScanEvent::Progress(ScanProgressEvent {
            job_id: id.clone(),
            seq,
            phase: "traverse",
            files_seen: seen.files,
            bytes_seen: seen.bytes,
            skipped: seen.skipped,
            elapsed_ms: elapsed,
        }));
    }

    let result = traversal.join();
    let elapsed = elapsed_ms(started);

    let (event, finish) = {
        let mut guard = lock(&slot);
        let Some(job) = guard.as_mut().filter(|job| job.id == id) else {
            return;
        };
        job.finished_elapsed_ms = Some(elapsed);
        job.seq = seq;

        match result {
            Ok(summary) => {
                let completeness = summary.completeness();
                job.counters = summary.clone();
                job.completeness = Some(completeness);
                let cancelled = completeness == Completeness::Cancelled;
                let state = if cancelled {
                    JobState::Cancelled
                } else {
                    JobState::Completed
                };
                transition(job, state);
                let finish = SessionFinish {
                    status: state.code(),
                    finished_at_ms: unix_ms(),
                    completeness: Some(completeness.code()),
                    counters: summary.clone(),
                    cancel_reason: cancelled.then_some(job.cancel_reason),
                    error_code: None,
                };
                let event = if cancelled {
                    ScanEvent::Cancelled(ScanCancelledEvent {
                        job_id: id.clone(),
                        reason: job.cancel_reason,
                        summary: ScanCountersDto::from(&summary),
                        elapsed_ms: elapsed,
                    })
                } else {
                    ScanEvent::Completed(ScanCompletedEvent::new(
                        id.clone(),
                        &summary,
                        completeness,
                        elapsed,
                    ))
                };
                (event, finish)
            }
            Err(_) => {
                let error = AppError::new(ErrorCode::Internal);
                job.error_code = Some(error.code);
                transition(job, JobState::Failed);
                let finish = SessionFinish {
                    status: JobState::Failed.code(),
                    finished_at_ms: unix_ms(),
                    completeness: None,
                    counters: seen,
                    cancel_reason: None,
                    error_code: Some(error.code.as_str()),
                };
                (
                    ScanEvent::Failed(ScanFailedEvent::new(id.clone(), &error)),
                    finish,
                )
            }
        }
    };

    // Stored before the event is sent, so a listener that reads the database
    // after `scan:completed` sees the final row. A failed write is logged by the
    // adapter and does not change the job outcome.
    if let Some(store) = &store {
        let _ = store.finish(&id, &finish);
    }

    tracing::info!(job_id = %id, event = event.name(), "scan finished");
    sink(event);
}

fn transition(job: &mut JobRecord, next: JobState) {
    match job.state.advance(next) {
        Ok(state) => job.state = state,
        Err(error) => tracing::error!(
            from = error.from.code(),
            to = error.to.code(),
            "invalid job transition ignored"
        ),
    }
}

/// Map a traversal validation failure to a fixed message. The error text of
/// `TraversalError` can contain the offending path, so it is never forwarded.
fn invalid_input(error: TraversalError) -> AppError {
    let detail = match error {
        TraversalError::NoRoots => "at least one root is required",
        TraversalError::TooManyRoots(_) => "too many roots",
        TraversalError::RelativeRoot(_) => "roots must be absolute paths",
        TraversalError::RootTooLong => "a root path is too long",
        TraversalError::Workers(_) | TraversalError::ChannelCapacity(_) => {
            return AppError::new(ErrorCode::Internal);
        }
        TraversalError::WorkerPanicked => return AppError::new(ErrorCode::Internal),
    };
    AppError::new(ErrorCode::InvalidInput).with_detail(detail)
}

fn next_job_id() -> String {
    let seq = JOB_SEQ.fetch_add(1, Ordering::Relaxed);
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis())
        .unwrap_or(0);
    format!("scan-{millis:x}-{seq:04x}")
}

/// Unix milliseconds UTC; `0` if the clock is before the epoch.
pub fn unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|elapsed| i64::try_from(elapsed.as_millis()).ok())
        .unwrap_or(0)
}

fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use std::time::Duration;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(label: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|elapsed| elapsed.as_nanos())
                .unwrap_or(0);
            let path = std::env::temp_dir().join(format!(
                "system-scanner-jobs-{label}-{}-{nanos}",
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

    type Events = Arc<Mutex<Vec<ScanEvent>>>;

    fn collector(delay_ms: u64) -> (EventSink, Events) {
        let events: Events = Arc::new(Mutex::new(Vec::new()));
        let target = Arc::clone(&events);
        let sink: EventSink = Arc::new(move |event| {
            let slow = delay_ms > 0 && matches!(event, ScanEvent::Progress(_));
            lock(&target).push(event);
            if slow {
                thread::sleep(Duration::from_millis(delay_ms));
            }
        });
        (sink, events)
    }

    fn tree(root: &Path, directories: usize, files: usize) {
        for directory in 0..directories {
            let path = root.join(format!("dir-{directory}"));
            fs::create_dir_all(&path).expect("fixture dir");
            for file in 0..files {
                fs::write(path.join(format!("file-{file}.bin")), [0u8; 2]).expect("fixture file");
            }
        }
    }

    fn wait_terminal(supervisor: &Supervisor, id: &str) -> ScanStatusDto {
        for _ in 0..2_000 {
            let status = supervisor.status(Some(id)).expect("job must exist");
            if matches!(status.state, "completed" | "failed" | "cancelled") {
                return status;
            }
            thread::sleep(Duration::from_millis(5));
        }
        panic!("job did not reach a terminal state");
    }

    fn wait_for_progress(events: &Events) {
        for _ in 0..2_000 {
            if lock(events)
                .iter()
                .any(|event| matches!(event, ScanEvent::Progress(_)))
            {
                return;
            }
            thread::sleep(Duration::from_millis(5));
        }
        panic!("no progress event arrived");
    }

    fn fast() -> ScanTuning {
        ScanTuning {
            workers: 2,
            channel_capacity: 64,
            progress_interval_ms: 0,
        }
    }

    /// Tiny channel and a slow sink keep a 300-entry walk alive long enough to
    /// cancel it mid-flight without sleeping on a timer.
    fn stalled() -> ScanTuning {
        ScanTuning {
            workers: 2,
            channel_capacity: 1,
            progress_interval_ms: 0,
        }
    }

    use crate::ports::sessions::SessionStoreError;

    #[derive(Default)]
    struct RecordingStore {
        calls: Mutex<Vec<String>>,
        starts: Mutex<Vec<SessionStart>>,
        finishes: Mutex<Vec<(String, SessionFinish)>>,
        fail: bool,
    }

    impl ScanSessionStore for RecordingStore {
        fn begin(&self, start: &SessionStart) -> Result<(), SessionStoreError> {
            lock(&self.calls).push("begin".into());
            lock(&self.starts).push(start.clone());
            if self.fail {
                Err(SessionStoreError)
            } else {
                Ok(())
            }
        }

        fn finish(&self, id: &str, finish: &SessionFinish) -> Result<(), SessionStoreError> {
            lock(&self.calls).push("finish".into());
            lock(&self.finishes).push((id.to_owned(), finish.clone()));
            if self.fail {
                Err(SessionStoreError)
            } else {
                Ok(())
            }
        }

        fn mark_interrupted(&self, _now_ms: i64) -> Result<usize, SessionStoreError> {
            Ok(0)
        }
    }

    #[test]
    fn a_finished_scan_is_recorded_begin_then_finish_without_paths() {
        let temp = TempDir::new("store");
        let root = temp.0.join("root");
        tree(&root, 2, 3);
        let store = Arc::new(RecordingStore::default());
        let supervisor = Supervisor::with_tuning(fast()).with_store(store.clone());
        let (sink, _) = collector(0);

        let id = supervisor
            .start(vec![root], 5_000, sink)
            .expect("scan must start");
        wait_terminal(&supervisor, &id);
        // The status flips just before the store write; wait for the write.
        for _ in 0..2_000 {
            if !lock(&store.finishes).is_empty() {
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }

        assert_eq!(*lock(&store.calls), ["begin", "finish"]);
        let starts = lock(&store.starts);
        assert_eq!(starts[0].id, id);
        assert!(starts[0].options_json.contains("\"maxEntries\":5000"));
        assert!(
            !starts[0].options_json.contains("system-scanner-jobs"),
            "no path may be stored"
        );
        let finishes = lock(&store.finishes);
        assert_eq!(finishes[0].0, id);
        assert_eq!(finishes[0].1.status, "completed");
        assert_eq!(finishes[0].1.completeness, Some("complete"));
        assert_eq!(finishes[0].1.counters.files, 6);
        assert_eq!(finishes[0].1.cancel_reason, None);
    }

    #[test]
    fn a_cancelled_scan_is_recorded_with_its_reason() {
        let temp = TempDir::new("store-cancel");
        let root = temp.0.join("root");
        tree(&root, 30, 10);
        let store = Arc::new(RecordingStore::default());
        let supervisor = Supervisor::with_tuning(stalled()).with_store(store.clone());
        let (sink, events) = collector(20);

        let id = supervisor
            .start(vec![root], 10_000, sink)
            .expect("scan must start");
        wait_for_progress(&events);
        supervisor.cancel(Some(&id));
        wait_terminal(&supervisor, &id);
        for _ in 0..2_000 {
            if !lock(&store.finishes).is_empty() {
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }

        let finishes = lock(&store.finishes);
        assert_eq!(finishes[0].1.status, "cancelled");
        assert_eq!(finishes[0].1.completeness, Some("cancelled"));
        assert_eq!(finishes[0].1.cancel_reason, Some("user_requested"));
    }

    #[test]
    fn a_failing_store_never_breaks_the_scan() {
        let temp = TempDir::new("store-fail");
        let root = temp.0.join("root");
        tree(&root, 2, 2);
        let store = Arc::new(RecordingStore {
            fail: true,
            ..RecordingStore::default()
        });
        let supervisor = Supervisor::with_tuning(fast()).with_store(store);
        let (sink, events) = collector(0);

        let id = supervisor
            .start(vec![root], 1_000, sink)
            .expect("a store failure must not refuse the scan");
        let status = wait_terminal(&supervisor, &id);

        assert_eq!(status.state, "completed");
        assert_eq!(status.counters.files, 4);
        for _ in 0..2_000 {
            if matches!(lock(&events).last(), Some(ScanEvent::Completed(_))) {
                return;
            }
            thread::sleep(Duration::from_millis(5));
        }
        panic!("the completed event must still be sent");
    }

    #[test]
    fn a_real_database_ends_the_session_so_nothing_is_left_open() {
        use crate::adapters::persistence::Database;

        let temp = TempDir::new("store-real");
        let root = temp.0.join("root");
        tree(&root, 2, 2);
        let database =
            Arc::new(Database::open(&temp.0.join("test.db")).expect("database must open"));
        let supervisor = Supervisor::with_tuning(fast()).with_store(database.clone());
        let (sink, events) = collector(0);

        let id = supervisor
            .start(vec![root], 1_000, sink)
            .expect("scan must start");
        wait_terminal(&supervisor, &id);
        for _ in 0..2_000 {
            if matches!(lock(&events).last(), Some(ScanEvent::Completed(_))) {
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }

        // `finish` runs before the event, so the row is already closed. An
        // unfinished `begin` would be reported here as one interrupted session.
        assert_eq!(database.mark_interrupted(unix_ms()).unwrap(), 0);
    }

    #[test]
    fn a_clean_scan_completes_and_reports_complete() {
        let temp = TempDir::new("clean");
        let root = temp.0.join("root");
        tree(&root, 3, 4);
        let supervisor = Supervisor::with_tuning(fast());
        let (sink, events) = collector(0);

        let id = supervisor
            .start(vec![root], 10_000, sink)
            .expect("scan must start");
        let status = wait_terminal(&supervisor, &id);

        assert_eq!(status.state, "completed");
        assert_eq!(status.completeness, Some("complete"));
        assert_eq!(status.counters.files, 12);
        assert_eq!(status.counters.directories, 4);
        assert_eq!(status.counters.bytes, 24);

        let events = lock(&events);
        match events.last().expect("a terminal event") {
            ScanEvent::Completed(done) => {
                assert_eq!(done.job_id, id);
                assert!(!done.partial);
                assert_eq!(done.summary.files, 12);
            }
            other => panic!("expected completed, got {other:?}"),
        }
    }

    #[test]
    fn a_missing_root_completes_as_partial_not_failed() {
        let temp = TempDir::new("missing");
        let supervisor = Supervisor::with_tuning(fast());
        let (sink, events) = collector(0);

        let id = supervisor
            .start(vec![temp.0.join("nope")], 100, sink)
            .expect("scan must start");
        let status = wait_terminal(&supervisor, &id);

        assert_eq!(status.state, "completed");
        assert_eq!(status.completeness, Some("partial"));
        assert_eq!(status.counters.disappeared, 1);
        assert!(matches!(
            lock(&events).last(),
            Some(ScanEvent::Completed(done)) if done.partial
        ));
    }

    #[test]
    fn invalid_roots_are_rejected_before_a_job_exists() {
        let supervisor = Supervisor::with_tuning(fast());
        let (sink, _) = collector(0);

        let relative = supervisor
            .start(
                vec![PathBuf::from("secret-relative-dir")],
                10,
                Arc::clone(&sink),
            )
            .expect_err("relative roots must be refused");
        assert_eq!(relative.code, ErrorCode::InvalidInput);
        assert_eq!(
            relative.detail.as_deref(),
            Some("roots must be absolute paths")
        );
        assert!(!relative.detail.unwrap_or_default().contains("secret"));

        let none = supervisor
            .start(Vec::new(), 10, sink)
            .expect_err("empty roots must be refused");
        assert_eq!(none.code, ErrorCode::InvalidInput);

        let missing = supervisor.status(None).expect_err("no job was created");
        assert_eq!(missing.code, ErrorCode::NotFound);
    }

    #[test]
    fn a_second_scan_is_busy_until_the_first_finishes() {
        let temp = TempDir::new("busy");
        let root = temp.0.join("root");
        tree(&root, 30, 10);
        let supervisor = Supervisor::with_tuning(stalled());
        let (sink, events) = collector(20);

        let first = supervisor
            .start(vec![root.clone()], 10_000, Arc::clone(&sink))
            .expect("first scan must start");
        wait_for_progress(&events);

        let refused = supervisor
            .start(vec![root.clone()], 10_000, Arc::clone(&sink))
            .expect_err("second scan must be refused");
        assert_eq!(refused.code, ErrorCode::Busy);
        assert!(refused.retryable);

        assert!(supervisor.cancel(Some(&first)));
        wait_terminal(&supervisor, &first);

        let second = supervisor
            .start(vec![root], 10_000, sink)
            .expect("the slot is free after a terminal state");
        assert_ne!(first, second);
        supervisor.cancel(None);
        wait_terminal(&supervisor, &second);
    }

    #[test]
    fn cancelling_mid_walk_ends_cancelled_with_partial_counts() {
        let temp = TempDir::new("cancel");
        let root = temp.0.join("root");
        tree(&root, 30, 10);
        let supervisor = Supervisor::with_tuning(stalled());
        let (sink, events) = collector(20);

        let id = supervisor
            .start(vec![root], 10_000, sink)
            .expect("scan must start");
        wait_for_progress(&events);

        assert!(supervisor.cancel(Some(&id)));
        assert!(
            supervisor.cancel(Some(&id)),
            "a repeated cancel is accepted, not an error"
        );
        let status = wait_terminal(&supervisor, &id);

        assert_eq!(status.state, "cancelled");
        assert_eq!(status.completeness, Some("cancelled"));
        assert!(
            status.counters.entries < 331,
            "walk must stop early: {}",
            status.counters.entries
        );
        match lock(&events).last().expect("a terminal event") {
            ScanEvent::Cancelled(event) => {
                assert_eq!(event.reason, "user_requested");
                assert_eq!(event.job_id, id);
            }
            other => panic!("expected cancelled, got {other:?}"),
        }

        assert!(
            !supervisor.cancel(Some(&id)),
            "a terminal job cannot be cancelled again"
        );
    }

    #[test]
    fn shutdown_cancels_the_running_scan_with_its_own_reason() {
        let temp = TempDir::new("shutdown");
        let root = temp.0.join("root");
        tree(&root, 30, 10);
        let supervisor = Supervisor::with_tuning(stalled());
        let (sink, events) = collector(20);

        let id = supervisor
            .start(vec![root], 10_000, sink)
            .expect("scan must start");
        wait_for_progress(&events);

        supervisor.shutdown();
        wait_terminal(&supervisor, &id);

        assert!(matches!(
            lock(&events).last(),
            Some(ScanEvent::Cancelled(event)) if event.reason == "shutdown"
        ));
    }

    #[test]
    fn progress_sequence_starts_at_one_and_only_increases() {
        let temp = TempDir::new("seq");
        let root = temp.0.join("root");
        tree(&root, 5, 5);
        let supervisor = Supervisor::with_tuning(fast());
        let (sink, events) = collector(0);

        let id = supervisor
            .start(vec![root], 10_000, sink)
            .expect("scan must start");
        let status = wait_terminal(&supervisor, &id);

        let seqs: Vec<u64> = lock(&events)
            .iter()
            .filter_map(|event| match event {
                ScanEvent::Progress(progress) => Some(progress.seq),
                _ => None,
            })
            .collect();

        assert!(!seqs.is_empty());
        assert_eq!(seqs[0], 1);
        assert!(seqs.windows(2).all(|pair| pair[1] == pair[0] + 1));
        assert_eq!(status.seq, *seqs.last().expect("non-empty"));
    }

    #[test]
    fn progress_is_paced_by_the_interval() {
        let temp = TempDir::new("pace");
        let root = temp.0.join("root");
        tree(&root, 10, 10);
        let supervisor = Supervisor::with_tuning(ScanTuning {
            progress_interval_ms: 60_000,
            ..fast()
        });
        let (sink, events) = collector(0);

        let id = supervisor
            .start(vec![root], 10_000, sink)
            .expect("scan must start");
        wait_terminal(&supervisor, &id);

        let progress = lock(&events)
            .iter()
            .filter(|event| matches!(event, ScanEvent::Progress(_)))
            .count();
        assert_eq!(progress, 1, "only the first entry is due inside the window");
    }

    #[test]
    fn the_entry_budget_truncates_and_is_reported() {
        let temp = TempDir::new("budget");
        let root = temp.0.join("root");
        tree(&root, 4, 4);
        let supervisor = Supervisor::with_tuning(fast());
        let (sink, _) = collector(0);

        let id = supervisor
            .start(vec![root], 5, sink)
            .expect("scan must start");
        let status = wait_terminal(&supervisor, &id);

        assert_eq!(status.state, "completed");
        assert_eq!(status.completeness, Some("truncated"));
        assert_eq!(status.counters.entries, 5);
    }

    #[test]
    fn status_and_cancel_handle_unknown_jobs_without_panicking() {
        let supervisor = Supervisor::with_tuning(fast());
        assert!(!supervisor.cancel(None));
        assert_eq!(
            supervisor.status(Some("scan-unknown")).unwrap_err().code,
            ErrorCode::NotFound
        );

        let temp = TempDir::new("unknown");
        let (sink, _) = collector(0);
        let id = supervisor
            .start(vec![temp.0.clone()], 100, sink)
            .expect("scan must start");
        wait_terminal(&supervisor, &id);

        assert_eq!(
            supervisor.status(Some("scan-other")).unwrap_err().code,
            ErrorCode::NotFound
        );
        assert!(!supervisor.cancel(Some("scan-other")));
    }
}
