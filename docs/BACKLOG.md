# Backlog

> **Status:** Active · **Last updated:** 2026-11-19
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
| B-004 | ✅ Build the app shell + dashboard route | One dashboard screen with panels; no navigation or routing | Panels share one dashboard with loading/empty/error states; no router |
| B-005 | ✅ Refresh lifecycle (manual, in-progress, last-updated) | Prevent stale/frozen UI | No overlapping refreshes; stale responses discarded |
| B-006 | ✅ Diagnostics view + redacted support bundle | Support without telemetry | Bundle previews and excludes paths/PII |
| B-007 | ✅ Structured logging with rotation | Diagnosability | `tracing` to file, 5×10 MB cap, redaction test |
| B-008 | ✅ Add test runner + first tests | Zero tests today | `cargo test` has real tests; `npm test` exists |
| B-009 | ✅ Add ESLint + Prettier | Consistency before the codebase grows | `npm run lint` / `format` pass |
| B-010 | 🟡 Add Windows CI (fmt, clippy, test, build) | No automation today | PRs blocked on failing checks |
| B-011 | 🟡 Add `rust-toolchain.toml` | Reproducible builds | Local and CI compilers match |
| B-012 | ✅ Fatal-startup error screen | `expect()` today | Init failure shows an actionable dialog, not a silent exit |

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
| B-020 | ✅ Choose DB layer (`rusqlite` vs `sqlx`) + ADR | Blocks persistence |
| B-021 | ✅ Initial schema + migration runner + tests | Blocks scan history |
| B-022 | ✅ Traversal prototype with bounded workers | Highest-risk component |
| B-023 | ✅ Per-entry outcome taxonomy implementation | Honest partial results |
| B-024 | 🟡 Batch persistence writer. **Minimal slice done** (one counters-only `scan_sessions` row per scan, interrupted-at-startup recovery, schema v2). **Remaining:** per-entry batch writes, which are blocked on the retention decision (`docs/plan.md` §12) | Throughput + crash safety |
| B-025 | ✅ Scan job orchestration + cancellation (in-memory slice) | Responsiveness |
| B-026 | Aggregation queries + indexes + query-plan tests | UI scale |
| B-027 | Duplicate pipeline (size → fingerprint → hash) | Avoid wasted hashing |
| B-028 | Bounded hashing with progress/pause/cancel | Resource control |
| B-029 | Virtualized results table | UI scale |
| B-030 | ✅ Interrupted-session reconciliation (startup marks open rows `interrupted`) | Crash recovery |
| B-031 | Retention enforcement + visible DB size | Bounded growth |
| B-032 | ✅ Performance baseline on 100k/1M fixtures | Sets real budgets |
| B-033 | ✅ Backup via `VACUUM INTO` + restore test (verified pre-migration snapshot, newest five kept, best-effort) | Data safety |

Note on B-020/B-021: schema v1 contains **`scan_sessions` only**. File records,
snapshots, hashes, findings, quarantine, and audit tables are deliberately deferred
to the features that write them, because retention and data-location are still open
decisions (`docs/plan.md` §12). Persistence is opened at startup but no IPC command
reads or writes the database yet.

Note on B-022: the prototype is `domain/traversal` (policy/outcomes) plus
`adapters/filesystem/traversal` (bounded worker pool, bounded channel,
cancellation, depth/entry budgets). It is **not reachable from IPC or the UI** and
writes nothing to the database. It does not follow reparse points, so a junction or
symlink is reported and skipped; following them needs a visited set by volume +
file ID and is deferred. Denied-directory and stream-error handling is covered by
unit mapping, not by a live ACL fixture.

## P2 — V2 (Releases 2–3)

| ID | Task |
|---|---|
| B-040 | ✅ Threat model review of file operations (TOCTOU, reparse points) |
| B-041 | 🟡 Handle-based identity **done** (F-01 closed) and protected-path policy **implemented and tested**, but the policy has no caller: nothing changes state yet, so no command enforces it. **Remaining:** the policy and identity check must be consumed by the first write path (B-047/B-048), with a denial reported per operation |
| B-042 | ✅ Process hierarchy + access-limited metadata |
| B-043 | ✅ Startup/service inventory with limitation explanations |
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
| B-090 | ✅ Runtime-validate IPC responses in `src/services` | P0 |
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
| B-043 | Startup/service inventory: `get_startup_inventory` reports six autostart sources (current-user and all-user `Run` and `RunOnce` keys, both Startup folders) and the `SERVICE_WIN32` service list from `EnumServicesStatusExW`, with start type and binary path from a per-service `QueryServiceConfigW`. A denied source keeps its group with a reason (`startup[i].entries`), a denied service configuration keeps its row and names the field, ordering is decided in the domain, and caps (4,096 services / 512 entries) are visible. Read-only, unelevated, no new crate; paths and commands are local-only (ADR-028) | 2026-11-19 | `src-tauri/src/domain/startup.rs`, `ports/startup.rs`, `adapters/windows/startup.rs`, `dto/startup_inventory.rs`, `commands/startup.rs`; `src/services/startup.ts`, `src/components/StartupInventory.tsx`; 16 new Rust tests (166 total), 82 frontend tests; fmt/clippy/test/build green |
| B-042 | Process hierarchy + image path: rows carry a derived `depth` (ancestors within the snapshot; `0` for a root, a missing parent, or a cycle; chain walk capped at 32; computed over the whole inventory before paging) and an optional `imagePath` read with `QueryFullProcessImageNameW` on a `PROCESS_QUERY_LIMITED_INFORMATION` handle, reported per row in `unavailable` when denied and never logged or bundled (ADR-027). `PROCESS_LIST_SCHEMA_VERSION` is now 2; the panel indents by depth (up to 8 levels) and shows the path under the name | 2026-11-19 | `src-tauri/src/domain/process.rs`, `src-tauri/src/adapters/windows/processes.rs`, `src-tauri/src/dto/process_list.rs`; 6 new Rust tests (150 total), 76 frontend tests; fmt/clippy/test green |
| B-032 | Traversal performance baseline: opt-in `#[ignore]`d harness generates reusable 100k/1M fixture trees (200 zero-length files per directory, three levels), walks them with the real walker at default limits, and reports entries, wall time, entries/s, µs/entry, first-event latency, process peak working set, and cancellation latency; it fails if the walk did not cover the fixture. Measured on one machine (40-thread Xeon, Windows 11 build 26300, release, warm cache): 100,510 entries in 0.030 s and 1,005,081 entries in 0.256 s (~3.4–3.9M entries/s), 11.5–12.7 MiB peak, ~1 ms to first event, 1.2–1.4 ms to cancel. HDD, cold cache, and real trees not measured | 2026-11-19 | `src-tauri/src/adapters/filesystem/baseline.rs`; `docs/PERFORMANCE.md` §1.2/§4.3/§8; `cargo test` 144 + 1 ignored; fmt/clippy green |
| B-041 (policy half) | Protected-path policy: `domain/policy.rs` — pure, no I/O, no platform import. Deny-by-default over SystemRoot, System32, Program Files (x86), ProgramData\Microsoft, LOCALAPPDATA\Microsoft, the app install dir, and the app data dir; component-aware case-insensitive comparison (so `C:\Windows10` is outside `C:\Windows`); trailing dots/spaces trimmed; `.`/`..` resolved lexically; `\\?\` accepted as the same location; most specific root supplies the stable reason code; unresolvable paths (relative, drive-less, device, verbatim global, UNC) denied rather than assumed safe; unusable roots dropped. **Not enforced** — no command changes state yet (KI-057) | 2026-11-19 | `src-tauri/src/domain/policy.rs`; 10 new Rust tests (144 total); fmt/clippy/test green |
| B-041 (identity half) | Handle-based directory access closes F-01/KI-055: one `CreateFileW` open with `FILE_FLAG_OPEN_REPARSE_POINT \| FILE_FLAG_BACKUP_SEMANTICS`, volume serial + 128-bit file ID from that handle, enumeration through the same handle in fixed-size batches, parent-recorded vs opened ID comparison (mismatch → `disappeared`), `\\?\` prefix for long paths. Children no longer re-stat by path. Protected-path allowlist policy remains | 2026-11-19 | `src-tauri/src/adapters/filesystem/directory.rs`, `src-tauri/src/domain/traversal.rs` (`FileIdentity`, `compare_identity`), `src-tauri/src/adapters/filesystem/traversal.rs`; 9 new Rust tests (134 total); fmt/clippy/test/build green |
| B-040 | Threat-model review of the implemented read-only file surface: IPC root/limit checks, root classification, per-child reparse checks, counters-only output, DB/backup content, and the check-then-open window between `symlink_metadata` and `read_dir`. Recorded F-01–F-07 and T-14, mapped T-02/T-03/T-09, and left UNC/network/removable root policy as an open product decision. No code change | 2026-11-17 | `docs/THREAT_MODEL.md` §11; `SECURITY.md` §6; `docs/ACCESS_CONTROL.md` §4; KI-055; R-12 |
| B-004 | App shell + single dashboard: header/main/footer, responsive one/two/three-column grid, `<h1 class="sr-only">`, five panels (backend status, system overview, process list, diagnostics, storage scan) each with explicit loading, empty, and error states, and an `ErrorBoundary` at the root. No router and no navigation, by design | 2026-11-17 | `src/App.tsx`, `src/main.tsx`, `src/components/*.tsx`; frontend tests already cover the panel states (76 tests) |
| B-033 | Pre-migration snapshot: `VACUUM INTO` into `app_data_dir()/backups/` with a version+timestamp name, read-only `PRAGMA quick_check` verification (suspect file deleted on failure), newest five kept, skipped for fresh/up-to-date files, best-effort so startup is never blocked. Restore exercised by reopening the snapshot with `Database::open` | 2026-11-17 | `src-tauri/src/adapters/persistence/sqlite.rs`; 5 new Rust tests (125 total); ADR-023 |
| B-030 | Interrupted-session reconciliation: startup sets rows still `queued`/`running`/`cancelling` to `interrupted` with a `finished_at` | 2026-11-17 | `src-tauri/src/lib.rs`, `src-tauri/src/adapters/persistence/sessions.rs` (`mark_interrupted`); `open_sessions_become_interrupted_and_ended_ones_do_not` |
| B-090 | Runtime validation of every IPC response and scan event: mandatory guard in `invokeCommand`, `schemaVersion`/enum/counter/collection-cap checks, `IpcContractError` → `contract_violation` in panels, malformed events reported and re-synced. No dependency added (ADR-022) | 2026-11-17 | `src/services/validate.ts`, `src/services/{app,system,processes,diagnostics,scan,unavailable}.ts`, `src/lib/errors.ts`; 31 new frontend tests (76 total) |
| B-024 (minimal) | `ScanSessionStore` port + SQLite writer: one counters-only `scan_sessions` row per scan, begin/finish best-effort, startup marks open rows `interrupted`; migration `0002_scan_session_results.sql` (schema v2). No paths, no per-entry rows, no read command | 2026-11-17 | `src-tauri/src/ports/sessions.rs`, `src-tauri/src/adapters/persistence/sessions.rs`, `src-tauri/migrations/0002_scan_session_results.sql`; 11 new Rust tests |
| B-025 | In-memory scan job supervisor: one scan at a time, `start_scan`/`cancel_scan`/`get_scan_status`, paced `scan:*` events (counters only), shutdown cancel. No UI, no picker, no persistence (B-024) | 2026-11-17 | `src-tauri/src/app/jobs.rs`, `src-tauri/src/domain/job.rs`, `src-tauri/src/dto/scan.rs`, `src-tauri/src/commands/scan.rs`; 24 new Rust tests |
| B-023 | Outcome taxonomy finished: stable snake_case codes for outcomes/skip reasons, derived `Completeness` verdict (`complete`/`partial`/`truncated`/`cancelled`), live denied-ACL fixture | 2026-11-17 | `src-tauri/src/domain/traversal.rs`, `src-tauri/src/adapters/filesystem/traversal.rs`; 4 new Rust tests. Not persisted or sent over IPC yet |
| B-022 | Bounded, cancellable traversal prototype: std-only walker, work queue + bounded channel, depth/entry budgets, reparse points never followed | 2026-11-16 | `src-tauri/src/domain/traversal.rs`, `src-tauri/src/adapters/filesystem/traversal.rs`; 16 new Rust tests |
| B-021 | Schema v1 (`scan_sessions`) + forward-only migration runner, checksum drift detection, refuses newer schema | 2026-11-16 | `src-tauri/migrations/0001_initial.sql`, `src-tauri/src/adapters/persistence/`; 8 new Rust tests |
| B-020 | DB layer chosen: `rusqlite` 0.43 `bundled`, one `Mutex<Connection>`, `spawn_blocking` for callers, WAL | 2026-11-16 | ADR-018 in `docs/DECISIONS.md`; `src-tauri/Cargo.toml` |
| B-012 | Fatal-startup dialog (`MessageBoxW`, exit 1) + React error boundary | 2026-11-16 | `src-tauri/src/startup.rs`, `src/components/ErrorBoundary.tsx` |
| B-006 | Diagnostics panel + redacted support bundle preview/copy | 2026-11-15 | `commands/diagnostics.rs`, `redact.rs`, `Diagnostics.tsx` |
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
