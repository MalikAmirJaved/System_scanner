# Scalability

> **Status:** Design guidance. The product currently runs no scans and stores no
> data. No capacity measurement has been performed.
> **Last updated:** 2026-11-06

---

## 1. What "scalability" means for a desktop app

There is no fleet to scale horizontally. Scale here means:

1. **Data scale** — many files (millions), many processes, years of history.
2. **Work scale** — long scans without freezing the UI or exhausting memory.
3. **Feature scale** — new modules without coupling or regressions.
4. **Machine scale** — weak HDD laptops through NVMe workstations.

Horizontal scaling (more machines, load balancing, sharding) is **not applicable**
to the local product. It becomes relevant only if a hosted reputation or fleet
service is approved — a separate product with a separate architecture.

## 2. Workload model

| Workload | Expected shape | Stress case |
|---|---|---|
| Directory traversal | 100k–2M entries | 10M entries, deep nesting, denied dirs |
| Duplicate detection | size groups + partial hashes | many same-size files |
| Full hashing | opt-in, size-capped | several large files, slow disk |
| Process list | 100–500 processes | 2k+ processes, access denied |
| History queries | recent sessions | 100+ sessions, 10M file rows |
| Event stream | 1–10 events/s | bursty progress on fast NVMe |

## 3. Vertical scaling (primary strategy)

| Resource | Strategy |
|---|---|
| CPU | Bounded worker pool; default `min(4, physical_cores/2)`; user-adjustable |
| Memory | Stream + aggregate; fixed-size channels; hard per-scan budget |
| Disk I/O | Batched writes; avoid per-file transactions; yield to foreground |
| Disk space | Retention caps + pre-scan estimate + visible DB size |
| GPU | Not used |

Rules:

- Concurrency is configurable but clamped (`1..=16` proposed).
- A scan must never allocate memory proportional to file count.
- The UI thread is never a worker.

## 4. Horizontal scaling (explicitly out of scope)

If a hosted service is ever proposed, it must answer:

- What data leaves the device, with what consent?
- Is the endpoint optional and offline-tolerant?
- What is the request rate and payload cap per client?
- How is abuse/rate limiting handled without user accounts?
- What is the retention and deletion policy server-side?

Do not design client code that assumes such a service exists. Baseline must remain
fully functional offline.

## 5. Backpressure and boundedness

```text
traverse (N workers)
   │ bounded channel (capacity C)
   ▼
collector
   │ bounded batch
   ▼
DB writer  ── if slow, producers await ──► traversal slows, memory stays flat
```

| Stage | Bound | On saturation |
|---|---|---|
| Traversal → collect | channel capacity | producers await |
| Collect → DB | batch size | flush sooner |
| DB → UI | page size (≤ 500) | UI requests next page |
| Progress events | ≤ 10/s | coalesce/drop intermediate |
| Log buffer | ring buffer | drop oldest at debug level |

If a stage cannot apply backpressure (e.g., UI events), it **drops** rather than
buffering unboundedly.

## 6. Database growth

Growth is dominated by `file_records`. Controls:

1. Delete-on-session-delete with `ON DELETE CASCADE` for records/hashes.
2. Retention policy per table (see [`../DATABASE.md`](../DATABASE.md) §8).
3. Aggregate rollups for old sessions (counts + top-N) instead of raw rows.
4. `VACUUM`/`PRAGMA incremental_vacuum` after large deletions, off the UI thread.
5. Show estimated and actual DB size in settings/diagnostics.

Budget (proposed, unmeasured): DB ≤ 2 GB by default with a visible warning at 80%.

## 7. Performance scaling rules

- Hash only after size + partial fingerprint reduces the candidate set.
- Prefer `ReadFile` with large sequential buffers over per-byte reads.
- Batch inserts (500–5000 rows) tuned by measurement, not guesswork.
- Index for query patterns; verify with `EXPLAIN QUERY PLAN`.
- Virtualize any table over 200 rows.
- Never send more than one page of records per IPC response.

Details: [`PERFORMANCE.md`](PERFORMANCE.md).

## 8. Feature scalability (architecture)

To add a domain without destabilizing the app:

```text
DTO → command (validate) → app job → domain use case → ports → adapter
   → service adapter → feature UI
```

Checklist for a new module:

- [ ] Own job type and cancellation token (if long-running)
- [ ] Own persistence entities + retention rule
- [ ] Own error codes mapped into the shared taxonomy
- [ ] Bounded result size and event rate
- [ ] No shared mutable global state

## 9. Benchmark plan

| Fixture | Purpose |
|---|---|
| Generated 100k / 1M / 5M entry trees | traversal throughput, memory |
| Deep nesting (200+ levels) | stack safety |
| 10k permission-denied dirs | error-path overhead |
| Many same-size files | duplicate pipeline cost |
| Large sparse files | hashing caps |
| 100 historical sessions | query latency + index validity |
| Concurrent UI interaction | responsiveness under load |

Record on HDD, SATA SSD, and NVMe. Publish methodology in
[`PERFORMANCE.md`](PERFORMANCE.md).

## 10. Scaling limits to publish

Before release, document:

- maximum supported scan scope;
- memory required per 1M entries (measured);
- DB size per 1M records (measured);
- cancellation latency;
- the point at which the app recommends narrowing scope.

Honest limits build more trust than a claim of "unlimited".
