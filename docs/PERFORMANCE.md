# Performance

> **Status:** The traversal walker has a first measured baseline (B-032,
> 2026-11-19) on one machine with a **hot-cache** fixture. Every other target
> below is still **proposed**: launch, refresh, process list, DB, hashing, and UI
> frame times have not been measured. **Last updated:** 2026-11-19

---

## 1. Measured facts

### 1.1 Build artifacts (2026-11-06)

| Metric | Value | Source |
|---|---|---|
| NSIS installer size | ~1.41 MB | `bundle/nsis/System Scanner_0.1.0_x64-setup.exe` |
| Frontend JS bundle | ~222 KB (uncompressed) | `dist/index-*.js` |
| Frontend CSS | ~4.4 KB | `dist/index-*.css` |
| Runtime JS dependencies | 3 | `package.json` |
| Rust direct dependencies | 1 (`tauri`) | `Cargo.toml` |

### 1.2 Walker baseline (2026-11-19, B-032)

Fixture: generated trees of zero-length files, 200 files per directory, three
levels deep, on the system temp volume. Release build, default limits (depth 64,
no entry cap), 16 workers, 4,096-entry channel. One run per size; the table in
§8 records the numbers.

| Fixture | Entries walked | Wall time | Throughput | Peak working set | First event | Cancellation |
|---|---|---|---|---|---|---|
| 100,000 files | 100,510 | 0.030 s | ~3.36M entries/s | 12.7 MiB | 1.1 ms | 1.4 ms |
| 1,000,000 files | 1,005,081 | 0.256 s | ~3.93M entries/s | 11.5 MiB | 1.0 ms | 1.2 ms |

Reading these numbers honestly:

- The fixture was **just written**, so all metadata was in the OS cache. This is
  the warm-cache case, not a cold start and not an HDD.
- Peak working set is process-wide (`K32GetProcessMemoryInfo`), so it includes the
  test harness and the fixture generator; it is an upper bound on the walk, and it
  did **not** grow with fixture size (11.5 MiB at 1M vs 12.7 MiB at 100k), which is
  the flat-memory claim holding up rather than an estimate.
- One run per size, not the five runs §4.3 asks for. Treat the figures as an
  order-of-magnitude baseline, not a p95.
- Zero-length files measure **metadata traversal**, which is what a counters-only
  scan does. Nothing here says anything about content read or hashing.

No launch time, refresh time, process-list, hashing, or UI frame figures exist
because those paths are not measured (KI-039).

## 2. Why budgets matter here

The product competes on **not freezing the machine**. A system scanner that makes
a laptop unusable during a scan has failed regardless of accuracy. Budgets are
therefore product requirements, not optimizations.

## 3. Proposed budgets

> Replace with measured values before release. Every number needs a workload and a
> machine definition.

| Metric | Target (proposed) | Workload |
|---|---|---|
| Cold launch → usable dashboard | ≤ 1.5 s p95 | Warm OS cache, SSD |
| Warm launch → usable dashboard | ≤ 600 ms p95 | |
| System refresh duration | ≤ 2 s p95 | Typical machine |
| Process list (500 rows) | ≤ 1 s p95 | |
| Cancellation latency | ≤ 500 ms p95 | At any phase |
| UI frame time during scan | ≤ 16 ms p95 (no long tasks > 100 ms) | Scan active |
| Scan throughput | ≥ 20k entries/s (SSD), ≥ 2k (HDD) | 1M-entry fixture |
| Peak RSS during 1M-entry scan | ≤ 300 MB | Default concurrency |
| DB growth | ≤ 400 bytes/record | Measured after batching |
| IPC event rate | ≤ 10/s per job | Coalesced |
| Time to first progress event | ≤ 300 ms | After start |
| Hashing throughput | ≥ 200 MB/s (NVMe), size-capped | Sequential read |

These are starting points for discussion, not commitments.

**Implemented today:** `get_system_snapshot` runs on a blocking thread, performs
only local registry/Win32 reads, and deliberately skips size/label queries on
network drives, whose calls can block for seconds. `list_processes` enumerates a
Toolhelp snapshot once (capped at 8192 entries) and opens each process only
long enough to read its start time and working set. Neither duration has been
measured on the baseline machines above (KI-039).

**Traversal walker:** measured once at 100k and 1M entries (§1.2, B-032). The
20k entries/s floor is met with a wide margin in the warm-cache case (~3.4–3.9M
entries/s on one 40-thread Xeon workstation under Windows 11 build 26300), and
peak working set stayed ~12 MiB across both sizes, which supports the flat-memory
design. Those numbers do **not** establish the HDD floor (2k entries/s), the
cold-cache case, or a real user tree with denied directories and reparse points:
none of those were measured. The RSS target of 300 MB is not tested at 5M entries.
Do not quote these figures as a p95 or as a cold-start budget.

**Startup snapshot:** before a pending migration, startup copies the database once
with `VACUUM INTO` and verifies it (B-033). The cost is proportional to database
size and has not been measured; it applies only when a migration is pending, not
on every launch.

## 4. Measurement methodology

### 4.1 Baseline machines (proposed)

| Class | Spec |
|---|---|
| Minimum | 4-core, 8 GB RAM, SATA SSD, Windows 11 |
| Typical | 8-core, 16 GB RAM, NVMe |
| Weak | 2-core, 4 GB RAM, HDD (worst case for traversal) |

### 4.2 Fixtures

- **Implemented (B-032):** generated entry trees of any size, 200 zero-length
  files per directory, three levels deep, created in parallel and reused across
  runs. 100k and 1M have been generated; 5M has not.
- Not generated yet: deep nesting (200 levels), long paths, Unicode names, 10k
  denied directories, same-size duplicates, sparse large files, 100 historical
  sessions.

### 4.3 Procedure

```text
1. Reboot, disable indexing/AV interference as documented.
2. Warm the app once; discard.
3. Run the workload 5 times; report median and p95.
4. Record: wall time, CPU time, peak RSS, DB size delta, cancellation latency.
5. Store results with commit SHA in this document.
```

The walker harness implements steps 3–5 for traversal only, and it is opt-in
because it writes a large fixture to disk (`src-tauri/src/adapters/filesystem/`
`baseline.rs`):

```bash
cd src-tauri
cargo test --release -- --ignored --nocapture perf_baseline

# sizes and knobs
PERF_ENTRIES=1000000 cargo test --release -- --ignored --nocapture perf_baseline
# PERF_WORKERS, PERF_CHANNEL, PERF_FIXTURE, PERF_CLEAN=1
```

It prints one row per run: entries walked, wall time, entries/s, µs/entry,
first-event latency, process peak working set, and cancellation latency; it fails if
the walk did not see the whole fixture. It does **not** implement the reboot and
AV conditions in steps 1–2, so a run on a busy machine is not a comparable
measurement. Use `--release`: debug builds measure the unoptimized walk and are
meaningfully slower.

**Baseline machine A** (the only machine measured so far): Intel Family 6 Model
85 (Xeon Scalable), 40 logical processors, ~31.7 GiB RAM, Windows 11 build 26300
(x86_64), NTFS on the system volume treated as SSD/NVMe (the model was not
captured). The walker used 16 workers, its configured cap, not 40.

Never compare numbers from different machines or fixture sizes.

## 5. Optimization rules

Ordered by impact:

1. **Stream, don't accumulate.** Never hold the full file list in memory.
2. **Batch DB writes.** One transaction per batch, not per file.
3. **Filter before hashing.** Size → partial fingerprint → hash.
4. **Coalesce UI events.** Progress is a summary, not a per-file log.
5. **Virtualize tables.** Render ~40 rows regardless of result count.
6. **Page queries.** Return ≤ 500 records per IPC call.
7. **Yield.** Reduce concurrency when the foreground app is active (proposed).
8. **Avoid re-stat.** Capture metadata once per entry.
9. **Use buffered sequential I/O** for hashing (≥ 1 MB buffers).
10. **Index for the actual query**, verified with `EXPLAIN QUERY PLAN`.

## 6. Anti-patterns to reject in review

| Pattern | Cost |
|---|---|
| `Vec<FileRecord>` for every entry | GBs of RAM at 1M+ files |
| `SELECT *` then filter in Rust | Full table scan per keystroke |
| Per-file `INSERT` | Orders of magnitude slower |
| `clone()` of path strings in hot loops | Allocation churn |
| React state update per progress event | Render storms |
| Synchronous Win32 call on the UI thread | Freezes |
| Re-hashing duplicate candidates before filtering | Wasted I/O |
| Unbounded channels | Memory blow-up under load |

## 7. Regression tracking

- Every performance-sensitive change records before/after on the same fixture.
- Results go in the table in §8 with commit SHA and date.
- A regression > 20% on a budgeted metric blocks the change unless justified and
  documented in [`DECISIONS.md`](DECISIONS.md).

## 8. Results log

| Date | Commit | Metric | Fixture | Result | Notes |
|---|---|---|---|---|---|
| 2026-11-06 | `ab941a1` | installer size | — | 1.41 MB | unsigned |
| 2026-11-06 | `ab941a1` | JS bundle | — | 222 KB | welcome screen only |
| 2026-11-16 | — | traversal prototype | ad-hoc temp trees | not run | no 100k/1M fixture; B-032 |
| 2026-11-19 | `825dacd`+baseline harness | walk 100k entries | 100,000 zero-length files, 3 levels, warm cache | 0.030 s, ~3.36M entries/s, 12.7 MiB peak, 1.1 ms first event, 1.4 ms cancel | baseline machine A, release build, 16 workers, one run |
| 2026-11-19 | `825dacd`+baseline harness | walk 1M entries | 1,000,000 zero-length files, 3 levels, warm cache | 0.256 s, ~3.93M entries/s, 11.5 MiB peak, 1.0 ms first event, 1.2 ms cancel | same machine; peak does not grow with fixture size |

## 9. Power and thermals

Proposed for Release 5:

- Detect battery vs AC; default to lower concurrency on battery.
- Stop or throttle scans on low battery.
- No busy-wait loops; no polling faster than 1 s.
- Respect Windows power throttling hints where available.

## 10. Open questions

1. What reference machine defines "production-ready"?
2. Is 1M entries the design target, or 10M?
3. Is the DB budget 2 GB, or user-configurable?
4. Should scans yield automatically when the foreground app is busy?
5. What is the acceptable UI impact on an HDD (the worst case)?
