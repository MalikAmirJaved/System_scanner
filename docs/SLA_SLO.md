# SLA / SLO

> **Status:** No targets are measured. All values are **proposed**. This is a
> desktop application: there is no server availability SLO.
> **Last updated:** 2026-11-06

---

## 1. What applies to a desktop product

There is no uptime, no request latency, no multi-tenant availability. The
meaningful reliability dimensions are:

| Dimension | Question |
|---|---|
| **Launch reliability** | Does the app open and become usable? |
| **Task success** | Does a scan/refresh complete, cancel, or fail predictably? |
| **Responsiveness** | Does the UI stay interactive during work? |
| **Data integrity** | Is history preserved across crashes and upgrades? |
| **Recoverability** | Can the user recover from a failure? |
| **Crash-free rate** | How often does the process terminate unexpectedly? |

## 2. Proposed SLOs

| ID | Objective | Target | Measurement |
|---|---|---|---|
| SLO-01 | Launch to usable dashboard | ≤ 1.5 s p95 | Instrumented timer, cold start, SSD |
| SLO-02 | Refresh completes | ≤ 2 s p95 | Event payload duration |
| SLO-03 | Cancellation latency | ≤ 500 ms p95 | Cancel request → terminal state |
| SLO-04 | UI frame time during scan | ≤ 16 ms p95 | Frame timing / long-task observer |
| SLO-05 | Scan completion rate | ≥ 95% of started scans reach a terminal state | Local counters |
| SLO-06 | Crash-free sessions | ≥ 99.5% | Local clean-shutdown marker |
| SLO-07 | Data integrity after crash | 100% of committed batches readable | Recovery test |
| SLO-08 | Migration success | 100% from supported versions | Test matrix |
| SLO-09 | Installer footprint | ≤ 5 MB | Artifact size |
| SLO-10 | Idle memory | ≤ 150 MB | Process RSS |
| SLO-11 | Peak scan memory (1M entries) | ≤ 300 MB | Process RSS |
| SLO-12 | Support bundle creation | ≤ 10 s | Diagnostics timer |

These are **unbudgeted until measured**. Do not publish them as commitments.

## 3. Measurement

- Local only; no telemetry. Users may export a bundle voluntarily.
- Store measurements with commit SHA, machine class, and fixture size.
- Report median and p95, not averages alone.
- Baseline machines: minimum (4-core/8 GB/SATA SSD), typical (8-core/16 GB/NVMe),
  weak (2-core/4 GB/HDD).

## 4. Error budget (conceptual)

Because there is no service, budgets apply per release:

```text
If crash-free sessions < 99.5% in preview -> block stable promotion.
If scan completion < 95% -> investigate before new features.
If a data-integrity failure occurs -> stop release, treat as S1.
```

## 5. Exclusions

Not counted against SLOs:

- unsupported Windows versions/builds;
- hardware failure, disk disconnection, corrupted filesystem;
- antivirus interference;
- network shares and removable media (until explicitly supported);
- user cancellation;
- OS-level policy blocking the app.

## 6. User-facing expectations

State honestly in documentation:

- the app is local and has no service dependency;
- scans are best-effort and may be partial;
- history may be lost on crash up to the last batch;
- the app does not protect against already-compromised administrators.

## 7. Reporting

| Audience | Artifact |
|---|---|
| User | Diagnostics view (local) |
| Support | Redacted bundle |
| Maintainers | Release checklist with measured results |

## 8. Review cadence

- Re-baseline every minor release.
- Revisit targets after the first Release 1 measurements.
- Any target that cannot be measured is removed, not kept as decoration.

## 9. Open questions

1. What machine defines the p95 baseline?
2. Is 1M entries the target workload or 10M?
3. Should crash-free rate be a hard gate for stable?
4. Is idle memory under 150 MB realistic with WebView2?
