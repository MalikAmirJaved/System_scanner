# Backlog

> **Status:** Active · **Last updated:** 2026-11-13
>
> Priority: **P0** blocks Release 0 / trust, **P1** needed for V1, **P2** V2+,
> **P3** nice to have. IDs are stable; do not renumber.
> Feature status lives in [`FEATURES.md`](FEATURES.md).
>
> **Legend:** `✅` complete with evidence · `🟡` partial, see the note · unmarked =
> not started. Evidence for each completed item must also exist in
> [`FEATURES.md`](FEATURES.md) and [`CHANGELOG.md`](CHANGELOG.md).

---

## P0 — Release 0 blockers

| ID | Task | Why | Done when |
|---|---|---|---|
| B-001 | ✅ Add typed IPC boundary (`CommandResult`, `AppError`) | Everything depends on it | A no-op command round-trips with typed errors |
| B-002 | ✅ Implement `get_system_snapshot` (Win32) | Core Release 0 value | OS/CPU/memory/disks/adapters returned; unavailable fields reported |
| B-003 | ✅ Implement `list_processes` with stable identity | Core Release 0 value | Paged, bounded, access-limited rows marked |
| B-004 | Build the app shell + dashboard route | UI has only a welcome screen | Navigation + loading/empty/error states |
| B-005 | ✅ Refresh lifecycle (manual, in-progress, last-updated) | Prevent stale/frozen UI | No overlapping refreshes; stale responses discarded |
| B-006 | Diagnostics view + redacted support bundle | Support without telemetry | Bundle previews and excludes paths/PII |
| B-007 | Structured logging with rotation | Diagnosability | `tracing` to file, 5×10 MB cap, redaction test |
| B-008 | ✅ Add test runner + first tests | Zero tests today | `cargo test` has real tests; `npm test` exists |
| B-009 | ✅ Add ESLint + Prettier | Consistency before the codebase grows | `npm run lint` / `format` pass |
| B-010 | 🟡 Add Windows CI (fmt, clippy, test, build) | No automation today | PRs blocked on failing checks |
| B-011 | 🟡 Add `rust-toolchain.toml` | Reproducible builds | Local and CI compilers match |
| B-012 | Fatal-startup error screen | `expect()` today | Init failure shows an actionable dialog, not a silent exit |

Notes on the partial items:

- **B-010** — `.github/workflows/ci.yml` exists (frontend, rust, package jobs on
  `windows-latest`) and every step was run locally, but the workflow has **never
  executed on GitHub**. It is not a gate until a run is green and branch
  protection requires it. Follow-up: B-092.
- **B-011** — The file pins `channel = "stable"` and the `rustfmt`/`clippy`
  components. It does **not** pin an exact compiler version, so local and CI can
  still drift within stable. Exact pinning is deliberately deferred.

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
| B-090 | Runtime-validate IPC responses in `src/services` | P0 |
| B-091 | ✅ Scope the `error` dead-code allowance to `ErrorCode` | P0 |
| B-092 | Get the first green CI run and enable branch protection | P0 |

## Explicitly rejected / deferred

| Item | Reason |
|---|---|
| Auto-cleanup feature | Violates safe-by-default |
| Kernel driver | Out of scope for initial releases |
| Cloud sync | No product need; privacy cost |
| Electron migration | No benefit |
| Microservices | No deployment requirement |

## Done (most recent first)

| ID | Item | Completed | Evidence |
|---|---|---|---|
| B-003 | `list_processes` via port + Win32 adapter, Rust-validated paging, stable per-row identity, per-row unavailable fields | 2026-11-13 | `src-tauri/src/{domain,ports,adapters/windows,dto,commands}/process*.rs`, `ProcessList.tsx`; 32 Rust + 13 frontend tests |
| B-091 | Module-level `#[allow(dead_code)]` removed; allowance scoped to the `ErrorCode` contract enum | 2026-11-13 | `src-tauri/src/error.rs` (clippy clean) |
| B-005 | Manual refresh, in-progress state, last-updated time, stale-response guard | 2026-11-13 | `src/components/SystemOverview.tsx`, `SystemOverview.test.tsx` |
| B-002 | `get_system_snapshot` via port + Windows adapter, explicit unavailable fields | 2026-11-13 | `src-tauri/src/{ports,domain,adapters,dto,commands}`, 18 Rust + 9 frontend tests |
| B-009 | ESLint 10 flat config + Prettier 3 | 2026-11-06 | `npm run lint`, `npm run format:check` |
| B-008 | Vitest 5 + jsdom + Rust unit tests | 2026-11-06 | `npm test` (6), `cargo test` (8) |
| B-001 | Typed IPC boundary + `get_app_info` | 2026-11-06 | `src-tauri/src/ipc.rs`, `src/services/ipc.ts` |

## How to pick work

1. P0 in ID order.
2. Do not start P2 security features before the threat model (B-040).
3. Every item must add its evidence to [`FEATURES.md`](FEATURES.md) and an entry to
   [`CHANGELOG.md`](CHANGELOG.md) when complete.
