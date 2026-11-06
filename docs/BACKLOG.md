# Backlog

> **Status:** Active · **Last updated:** 2026-11-06
>
> Priority: **P0** blocks Release 0 / trust, **P1** needed for V1, **P2** V2+,
> **P3** nice to have. IDs are stable; do not renumber.
> Feature status lives in [`FEATURES.md`](FEATURES.md).

---

## P0 — Release 0 blockers

| ID | Task | Why | Done when |
|---|---|---|---|
| B-001 | Add typed IPC boundary (`CommandResult`, `AppError`) | Everything depends on it | A no-op command round-trips with typed errors |
| B-002 | Implement `get_system_snapshot` (Win32) | Core Release 0 value | OS/CPU/memory/disks/adapters returned; unavailable fields reported |
| B-003 | Implement `list_processes` with stable identity | Core Release 0 value | Paged, bounded, access-limited rows marked |
| B-004 | Build the app shell + dashboard route | UI has only a welcome screen | Navigation + loading/empty/error states |
| B-005 | Refresh lifecycle (manual, in-progress, last-updated) | Prevent stale/frozen UI | No overlapping refreshes; stale responses discarded |
| B-006 | Diagnostics view + redacted support bundle | Support without telemetry | Bundle previews and excludes paths/PII |
| B-007 | Structured logging with rotation | Diagnosability | `tracing` to file, 5×10 MB cap, redaction test |
| B-008 | Add test runner + first tests | Zero tests today | `cargo test` has real tests; `npm test` exists |
| B-009 | Add ESLint + Prettier | Consistency before the codebase grows | `npm run lint` / `format` pass |
| B-010 | Add Windows CI (fmt, clippy, test, build) | No automation today | PRs blocked on failing checks |
| B-011 | Add `rust-toolchain.toml` | Reproducible builds | Local and CI compilers match |
| B-012 | Fatal-startup error screen | `expect()` today | Init failure shows an actionable dialog, not a silent exit |

## P1 — V1 (Release 1)

| ID | Task | Why |
|---|---|---|
| B-020 | Choose DB layer (`rusqlite` vs `sqlx`) + ADR | Blocks persistence |
| B-021 | Initial schema + migration runner + tests | Blocks scan history |
| B-022 | Traversal prototype with bounded workers | Highest-risk component |
| B-023 | Per-entry outcome taxonomy implementation | Honest partial results |
| B-024 | Batch persistence writer | Throughput + crash safety |
| B-025 | Scan job orchestration + cancellation | Responsiveness |
| B-026 | Aggregation queries + indexes + query-plan tests | UI scale |
| B-027 | Duplicate pipeline (size → fingerprint → hash) | Avoid wasted hashing |
| B-028 | Bounded hashing with progress/pause/cancel | Resource control |
| B-029 | Virtualized results table | UI scale |
| B-030 | Interrupted-session reconciliation | Crash recovery |
| B-031 | Retention enforcement + visible DB size | Bounded growth |
| B-032 | Performance baseline on 100k/1M fixtures | Sets real budgets |
| B-033 | Backup via `VACUUM INTO` + restore test | Data safety |

## P2 — V2 (Releases 2–3)

| ID | Task |
|---|---|
| B-040 | Threat model review of file operations (TOCTOU, reparse points) |
| B-041 | Protected-path policy + tests |
| B-042 | Process hierarchy + access-limited metadata |
| B-043 | Startup/service inventory with limitation explanations |
| B-044 | `WinVerifyTrust` signature adapter |
| B-045 | Evidence/classification engine with rule versions |
| B-046 | Audit history (append-only, tamper-evident) |
| B-047 | Quarantine + restore design review |
| B-048 | Quarantine implementation with integrity checks |
| B-049 | Confirmation UX standard for destructive actions |
| B-050 | Elevation flow for narrowly scoped operations |

## P3 — V3 and later

| ID | Task |
|---|---|
| B-060 | Event Log summaries with bounded queries |
| B-061 | Connection/port ownership |
| B-062 | Historical trends + retention UI |
| B-063 | Schedules + battery-aware policy |
| B-064 | Redacted export (CSV/JSON/PDF) |
| B-065 | Explainable health summary |
| B-066 | Accessibility audit |
| B-067 | MSIX / enterprise deployment research |

## Engineering / process debt

| ID | Task | Priority |
|---|---|---|
| B-080 | Code signing + SmartScreen workflow | P1 (public release) |
| B-081 | SBOM generation | P1 |
| B-082 | Dependabot / `cargo-deny` allowlist | P1 |
| B-083 | Version-sync check across 3 files | P2 |
| B-084 | `LICENSE` selection | P1 (blocks redistribution) |
| B-085 | Security contact publication | P1 |
| B-086 | Upgrade-path test harness | P1 |
| B-087 | Splash-free startup measurement | P2 |
| B-088 | Remove generated `gen/schemas` ambiguity (gitignored) | P3 |
| B-089 | Decide whether `dist/` and `target/` stay in the working tree | P3 |

## Explicitly rejected / deferred

| Item | Reason |
|---|---|
| Auto-cleanup feature | Violates safe-by-default |
| Kernel driver | Out of scope for initial releases |
| Cloud sync | No product need; privacy cost |
| Electron migration | No benefit |
| Microservices | No deployment requirement |

## How to pick work

1. P0 in ID order.
2. Do not start P2 security features before the threat model (B-040).
3. Every item must add its evidence to [`FEATURES.md`](FEATURES.md) and an entry to
   [`CHANGELOG.md`](CHANGELOG.md) when complete.
