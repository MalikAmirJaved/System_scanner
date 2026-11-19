//! Baseline measurement harness for the traversal walker (B-032).
//!
//! Compiled only for tests, and every test here is `#[ignore]`d: it writes a
//! fixture tree to disk, and at the sizes that matter (100k, 1M) it takes
//! minutes. A normal `cargo test` must never run it. Run it explicitly:
//!
//! ```text
//! cd src-tauri
//! cargo test --release -- --ignored --nocapture perf_baseline
//! ```
//!
//! Environment knobs:
//!
//! | Variable | Meaning | Default |
//! |---|---|---|
//! | `PERF_ENTRIES` | files to generate | `100000` |
//! | `PERF_WORKERS` | walker threads | `min(16, available parallelism)` |
//! | `PERF_CHANNEL` | event channel capacity | `4096` |
//! | `PERF_FIXTURE` | fixture directory | `<temp>/system-scanner-perf-<entries>` |
//! | `PERF_CLEAN` | `1` deletes the fixture afterwards | unset (keep it) |
//!
//! The fixture is identified by a stamp file and reused, so repeated runs
//! measure the walk rather than the generator. Files are zero-length: this
//! measures metadata traversal, which is what a counters-only scan does, not
//! data read.
//!
//! Numbers from this harness are **measurements of one machine**, not budgets.
//! Record them in `docs/PERFORMANCE.md` with the machine spec; that document
//! owns the budgets.

use std::env;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use crate::adapters::filesystem::traversal::{MAX_WORKERS, Traversal, TraversalRequest};
use crate::domain::traversal::{TraversalLimits, TraversalSummary};

/// Files per fixture directory. Keeps directory sizes typical (hundreds, not
/// thousands) while keeping the generator fast.
const FILES_PER_DIR: usize = 200;
/// Fixture directories per parent directory, so the tree is three levels deep.
const DIRS_PER_BATCH: usize = 64;
/// Marks a complete fixture with the file count it holds.
const STAMP: &str = "perf-fixture.stamp";

#[test]
#[ignore = "writes a fixture tree and takes minutes; run explicitly with --ignored"]
fn perf_baseline() {
    let entries = env_usize("PERF_ENTRIES", 100_000);
    let workers = env_usize("PERF_WORKERS", default_workers());
    let capacity = env_usize("PERF_CHANNEL", 4096);

    let root = ensure_fixture(entries);

    let (summary, first_event, elapsed) = run_walk(&root, workers, capacity);
    let rate = summary.entries as f64 / elapsed.as_secs_f64().max(f64::MIN_POSITIVE);
    let per_entry = elapsed.as_secs_f64() * 1e6 / summary.entries.max(1) as f64;

    println!(
        "scan: {} entries ({} files, {} dirs) in {:.3} s",
        summary.entries,
        summary.files,
        summary.directories,
        elapsed.as_secs_f64()
    );
    println!(
        "scan: {rate:.0} entries/s, {per_entry:.1} us/entry, {workers} workers, capacity {capacity}"
    );
    println!(
        "scan: first event after {:.1} ms",
        first_event.as_secs_f64() * 1e3
    );
    println!(
        "scan: os outcomes ok={} skipped={} denied={} disappeared={} unsupported={} failed={} truncated={} cancelled={}",
        summary.entries,
        summary.skipped,
        summary.denied,
        summary.disappeared,
        summary.unsupported,
        summary.failed,
        summary.truncated,
        summary.cancelled
    );

    match peak_working_set_bytes() {
        Some(bytes) => println!(
            "scan: process peak working set {:.1} MiB (includes the fixture generator)",
            mib(bytes)
        ),
        None => println!("scan: process peak working set unavailable on this host"),
    }

    let cancel = cancel_latency(&root, workers, capacity);
    println!(
        "scan: cancellation latency {:.1} ms",
        cancel.as_secs_f64() * 1e3
    );

    // The walk must have covered the fixture honestly: a benchmark that
    // silently stops early would produce a flattering, false number.
    assert!(
        summary.entries >= entries as u64,
        "walk saw {} entries of {entries} generated",
        summary.entries
    );
    assert!(
        !summary.truncated,
        "the entry budget truncated the baseline"
    );
    assert!(!summary.cancelled, "the baseline walk was cancelled");
    assert_eq!(
        (summary.denied, summary.failed, summary.disappeared),
        (0, 0, 0),
        "the fixture produced unexpected failures"
    );

    if env_usize("PERF_CLEAN", 0) == 1 {
        let _ = fs::remove_dir_all(&root);
        println!("fixture: removed {}", root.display());
    } else {
        println!(
            "fixture: kept at {} (PERF_CLEAN=1 removes it)",
            root.display()
        );
    }
}

/// One full walk, returning the summary, the time to the first event, and the
/// total duration.
fn run_walk(
    root: &Path,
    workers: usize,
    capacity: usize,
) -> (TraversalSummary, Duration, Duration) {
    let started = Instant::now();
    let traversal = Traversal::start(request(root, workers, capacity)).expect("start traversal");

    let mut first_event = None;
    for _entry in traversal.events().iter() {
        if first_event.is_none() {
            first_event = Some(started.elapsed());
        }
    }

    let summary = traversal.join().expect("join traversal");
    (summary, first_event.unwrap_or_default(), started.elapsed())
}

/// Time from `cancel()` to the walk actually stopping.
///
/// Measured on a walk that is definitely running: the fixture is large enough
/// that the first event arrives long before the end.
fn cancel_latency(root: &Path, workers: usize, capacity: usize) -> Duration {
    let traversal = Traversal::start(request(root, workers, capacity)).expect("start traversal");

    {
        let mut events = traversal.events().iter();
        let _ = events.next();
    }

    let handle = traversal.cancel_handle();
    let cancelled_at = Instant::now();
    handle.cancel();
    let _ = traversal.join();
    cancelled_at.elapsed()
}

fn request(root: &Path, workers: usize, capacity: usize) -> TraversalRequest {
    TraversalRequest {
        roots: vec![root.to_path_buf()],
        limits: TraversalLimits::default(),
        workers,
        channel_capacity: capacity,
    }
}

fn ensure_fixture(entries: usize) -> PathBuf {
    assert!(entries > 0, "PERF_ENTRIES must be positive");

    let root = env::var_os("PERF_FIXTURE")
        .map(PathBuf::from)
        .unwrap_or_else(|| env::temp_dir().join(format!("system-scanner-perf-{entries}")));
    let stamp = root.join(STAMP);
    let wanted = entries.to_string();

    if fs::read_to_string(&stamp).ok().as_deref() == Some(wanted.as_str()) {
        println!("fixture: reusing {}", root.display());
        return root;
    }

    // An incomplete or differently sized fixture is not measurable.
    if root.exists() {
        fs::remove_dir_all(&root).expect("clear stale fixture");
    }
    fs::create_dir_all(&root).expect("create fixture root");

    let dirs = entries.div_ceil(FILES_PER_DIR);
    let threads = default_workers().min(dirs).max(1);
    let started = Instant::now();

    thread::scope(|scope| {
        for worker in 0..threads {
            let root = &root;
            scope.spawn(move || {
                for index in (worker..dirs).step_by(threads) {
                    generate_dir(root, index, entries);
                }
            });
        }
    });

    fs::write(&stamp, &wanted).expect("write fixture stamp");
    println!(
        "fixture: generated {entries} files in {dirs} dirs in {:.1} s at {}",
        started.elapsed().as_secs_f64(),
        root.display()
    );
    root
}

fn generate_dir(root: &Path, index: usize, entries: usize) {
    let batch = index / DIRS_PER_BATCH;
    let dir = root
        .join(format!("b{batch:04}"))
        .join(format!("d{index:05}"));
    fs::create_dir_all(&dir).expect("create fixture dir");

    let first = index * FILES_PER_DIR;
    let last = (first + FILES_PER_DIR).min(entries);
    for file in first..last {
        File::create(dir.join(format!("f{file:07}.bin"))).expect("create fixture file");
    }
}

fn default_workers() -> usize {
    thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(4)
        .min(MAX_WORKERS)
}

fn env_usize(name: &str, default: usize) -> usize {
    match env::var(name) {
        Ok(value) => value
            .trim()
            .parse()
            .unwrap_or_else(|_| panic!("{name} must be a positive integer; got {value:?}")),
        Err(_) => default,
    }
}

fn mib(bytes: u64) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
}

/// Peak working set of this process. Process-wide, so it includes whatever the
/// test harness and fixture generator held; it is an upper bound, not a
/// measurement of the walk alone.
#[cfg(windows)]
fn peak_working_set_bytes() -> Option<u64> {
    use std::mem::size_of;

    use windows_sys::Win32::System::ProcessStatus::{
        K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
    };
    use windows_sys::Win32::System::Threading::GetCurrentProcess;

    let mut counters = PROCESS_MEMORY_COUNTERS {
        cb: size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
        ..PROCESS_MEMORY_COUNTERS::default()
    };

    // SAFETY: `counters` is live and `cb` is its own size, as documented.
    if unsafe { K32GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, counters.cb) } == 0 {
        return None;
    }

    Some(counters.PeakWorkingSetSize as u64)
}

#[cfg(not(windows))]
fn peak_working_set_bytes() -> Option<u64> {
    None
}
