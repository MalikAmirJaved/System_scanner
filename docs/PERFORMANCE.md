# Performance

> **Status:** No performance work has been measured beyond build artifact sizes.
> All targets below are **proposed** and must be replaced with measurements from
> a documented baseline machine. **Last updated:** 2026-11-06

---

## 1. Measured facts (2026-11-06)

| Metric | Value | Source |
|---|---|---|
| NSIS installer size | ~1.41 MB | `bundle/nsis/System Scanner_0.1.0_x64-setup.exe` |
| Frontend JS bundle | ~222 KB (uncompressed) | `dist/index-*.js` |
| Frontend CSS | ~4.4 KB | `dist/index-*.css` |
| Runtime JS dependencies | 3 | `package.json` |
| Rust direct dependencies | 1 (`tauri`) | `Cargo.toml` |

No launch time, refresh time, scan throughput, or memory figures exist because
those features do not exist.

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

## 4. Measurement methodology

### 4.1 Baseline machines (proposed)

| Class | Spec |
|---|---|
| Minimum | 4-core, 8 GB RAM, SATA SSD, Windows 11 |
| Typical | 8-core, 16 GB RAM, NVMe |
| Weak | 2-core, 4 GB RAM, HDD (worst case for traversal) |

### 4.2 Fixtures

- Generated trees: 100k, 1M, 5M entries.
- Deep nesting (200 levels), long paths, Unicode names.
- 10k denied directories.
- Many same-size files (duplicate pipeline).
- Sparse large files (hashing caps).
- 100 historical sessions (query latency).

### 4.3 Procedure

```text
1. Reboot, disable indexing/AV interference as documented.
2. Warm the app once; discard.
3. Run the workload 5 times; report median and p95.
4. Record: wall time, CPU time, peak RSS, DB size delta, cancellation latency.
5. Store results with commit SHA in this document.
```

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
