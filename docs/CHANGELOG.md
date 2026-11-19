# Changelog

All notable changes to this project are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versioning follows [Semantic Versioning](https://semver.org/) once a public release
exists. Until then, `0.x` versions may change behavior without notice.

> AI-assisted **architectural** changes are recorded separately in
> [`../AI_CHANGELOG.md`](../AI_CHANGELOG.md).

---

## [Unreleased]

### Added (persistence)
- **Verified snapshot before a schema migration (B-033).** When an existing
  database reports a schema version below this build's, startup writes a
  consistent copy with `VACUUM INTO` to `backups/system-scanner-v{version:03}-{epoch_ms}.db`,
  reopens it read-only, and requires `PRAGMA quick_check` to return `ok`. A
  snapshot that fails is deleted; the newest five are kept. Fresh and up-to-date
  files are not snapshotted. The step is best-effort: every released migration is
  additive, so a full disk or denied directory logs a warning instead of blocking
  startup. Restore remains manual and is exercised only by tests. No new command,
  capability, or dependency.

### Fixed
- **Every IPC response and scan event is validated in the renderer (B-090).**
  `invokeCommand` now requires a guard for the command's DTO, so an unvalidated
  call site does not compile. Guards check the envelope status, the `AppError`
  payload and its code enum, `schemaVersion`, enum values, counters as
  non-negative integers, and collection caps; only known fields are copied and a
  present field of the wrong type is rejected rather than defaulted. A mismatch
  throws `IpcContractError` (`source`, field path, expected type), which panels
  show as `contract_violation` instead of rendering `undefined`; messages never
  include payload values. Malformed `scan:*` payloads are reported and dropped,
  and the panel re-reads `get_scan_status`. No dependency added (ADR-022).
  31 new frontend tests (76 total).

### Added (UI)
- **Storage scan panel.** Type one or more absolute folder paths, start a
  read-only scan, watch live file/size/skipped counts, and cancel. A finished scan
  shows a Complete, Partial, Truncated, or Cancelled verdict with what that does
  and does not cover. Counts only: no file names or paths are shown, there is no
  folder picker, and the panel has not yet been exercised in the running app
  (KI-052). New `src/services/scan.ts` is the only wrapper for the scan commands
  and `scan:*` events; no capability change.

### Security
- **Handle-based directory access closes the check-then-open race (B-041).**
  Each directory is opened once with
  `FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS`, its volume serial
  and 128-bit file id are read from that handle, and children are enumerated
  through the same handle in 16 KiB batches. The walker compares the id the
  parent recorded with the id it opened and reports a mismatch as `disappeared`
  instead of scanning the replacement. Children are classified from the
  enumeration record, so the second by-path `symlink_metadata` is gone, and
  `\\?\` prefixes make paths beyond `MAX_PATH` open without `LongPathsEnabled`.
  Counts, verdicts, and the IPC surface are unchanged. Where a volume supplies no
  file id the check is unverifiable, never a match (KI-056). 9 new Rust tests
  (134 total); no dependency or capability change.
- **Threat-model review of the read-only file surface (B-040).** Reviewed the
  implemented scan path end to end: IPC root/limit validation, root
  classification, per-child reparse checks, counters-only output, and the
  database/backup content. No code changed. Findings are in
  `docs/THREAT_MODEL.md` §11. Three matter: arbitrary UNC/network/removable roots
  are still accepted and the policy decision is open; `visit_directory` checks the
  reparse attribute with `symlink_metadata` and then opens with a **by-path**
  `read_dir`, so a directory swapped in between is followed (KI-055) — harmless
  while only counters are produced, blocking before any content read or write; and
  there is no per-I/O timeout, so an unreachable share stalls one worker. T-14
  added, T-02/T-03/T-09 annotated. The read-only surface is accepted for pre-alpha
  only.
- **`start_scan` refuses device and NT-namespace roots** (`\\.\...`, `\??\...`,
  and `\\?\` forms that are not a drive or UNC, such as `\\?\GLOBALROOT\...`).
  Ordinary drives, UNC shares, and extended-length drive/UNC paths are still
  accepted. Further root restrictions remain an open B-040 decision (KI-049).

### Added
- **Scan sessions persisted, counters only (B-024 minimal).** Each scan writes one
  `scan_sessions` row (status, completeness, aggregate counters, budgets; **no
  paths or roots**) through a `ScanSessionStore` port. Schema is now v2
  (`0002_scan_session_results.sql`, additive). Sessions left open by a crash or
  kill are marked `interrupted` at the next startup. Writes are best-effort. No
  command reads sessions, and per-entry persistence is not started. See ADR-021.
- **In-memory scan jobs (B-025; backend only, no UI).** `start_scan`,
  `cancel_scan`, and `get_scan_status` drive one read-only traversal at a time;
  `scan:progress` (paced to 10/s), `scan:completed`, `scan:cancelled`, and
  `scan:failed` carry aggregate counters only. A second scan is refused with
  `busy`, closing the window cancels a running scan, and nothing is persisted.
  No frontend code calls the commands yet. See `API.md` §4.1 and ADR-020.
- **Traversal outcome taxonomy completed (B-023; implemented, not wired).**
  Stable snake_case codes for entry outcomes and skip reasons, a derived
  `Completeness` verdict (`complete`/`partial`/`truncated`/`cancelled`) computed
  from the counters, and a live denied-directory test. No IPC, UI, or database
  change.
- **Bounded traversal prototype (implemented, not wired).** `domain/traversal`
  defines entry kinds, skip reasons, the outcome taxonomy, limits, and counters;
  `adapters/filesystem/traversal` walks roots with a fixed worker pool, an
  explicit work queue, a bounded entry channel (backpressure), cooperative
  cancellation, and depth/entry budgets reported as `truncated`. Reparse points
  (symlinks, junctions, other Windows reparse attributes) are reported and never
  followed, so a junction loop cannot hang the walk. Paths stay in memory: no
  IPC command, UI, log line, or database row uses this yet.
- **Persistence foundation (implemented).** `rusqlite` 0.43 with bundled SQLite
  opens `app_data_dir()/system-scanner.db` at startup, applies forward-only
  migrations, and stores the handle in Tauri state. Schema **v1 adds
  `scan_sessions`** with two indexes; no other table exists. Migrations are
  hand-written SQL embedded via `include_str!`, recorded in `schema_migrations`
  with FNV-1a checksums. A database from a newer build, or an edited released
  migration, refuses to open instead of guessing. Startup failure is logged and
  the app continues; no panel reads the database yet. ADR-018.
- **Fatal-startup dialog + renderer error boundary (implemented).**
  `startup.rs` replaces the final `expect()`: a failed
  `tauri::Builder::run` shows a native message box with the app version, a
  redacted, length-bounded detail, and next steps, then exits with status 1.
  `src/components/ErrorBoundary.tsx` catches a render failure and shows a reload
  screen instead of a blank window. No new crate; the dialog uses the existing
  `windows-sys` dependency (`Win32_UI_WindowsAndMessaging`).
- **Local diagnostics + redacted support bundle (implemented).**
  `get_diagnostics` returns app/Tauri versions, OS build and architecture,
  elevation, log size, and a bounded (256 KiB) log tail. `redact.rs` replaces
  profile/absolute/UNC paths, e-mail addresses, IPv4/IPv6, MAC addresses, SIDs,
  and the current account and machine names before the preview is returned. The
  Diagnostics panel loads only on request and supports copy; it writes no file
  and uploads nothing.
- **Typed IPC boundary (implemented).** `src-tauri/src/ipc.rs`
  (`CommandResult` envelope), `src-tauri/src/error.rs` (`ErrorCode` taxonomy,
  `AppError` with per-error correlation IDs), `src-tauri/src/dto/app_info.rs`
  (`AppInfo`, `DTO_SCHEMA_VERSION = 1`), and the read-only command
  `get_app_info`.
- **Frontend IPC adapter.** `src/services/ipc.ts` (single `invoke` call site,
  `IpcError`), `src/services/app.ts`, and `src/components/BackendStatus.tsx` with
  loading, ready, error, and retry states wired into the welcome screen.
- **Tooling.** Vitest 5 + jsdom + Testing Library, ESLint 10 flat config,
  Prettier 3, and `rust-toolchain.toml` (stable channel + `rustfmt`/`clippy`).
- **CI workflow.** `.github/workflows/ci.yml` — `frontend`, `rust`, and `package`
  jobs on `windows-latest`. Written and verified step-by-step locally; it has
  **not** run on GitHub yet.
- `npm test`, `npm run test:watch`, `npm run lint`, `npm run format`,
  `npm run format:check`.
- ADR-011: IPC DTO fields are `camelCase` on the wire.
- **System snapshot (implemented).** `get_system_snapshot` returns OS, CPU,
  memory, disks, and network adapters through a new `ports/` trait,
  platform-neutral `domain/` model, and `adapters/windows/` Win32 implementation
  (`windows-sys` 0.61.2). Unavailable groups and leaf fields are reported with
  `access_denied` / `unsupported` / `failed`, never as zero or empty values.
- **System overview UI.** `src/services/system.ts` and
  `src/components/SystemOverview.tsx` render the snapshot with loading, error,
  refresh, last-updated time, and per-field unavailable reasons.
- ADR-012: Win32 bindings are confined to an adapter behind a narrow port, and
  network-drive size/label queries are skipped to keep the snapshot bounded.
- **Local logging (implemented).** `tracing` events write to Tauri's
  `app_log_dir()` through a size-rotating writer (10 MB × 5 files). Command
  outcomes log stable codes and correlation IDs without payloads; diagnostic
  detail passes best-effort path redaction. Initialization failure is non-fatal.
  ADR-015.
- **Responsive reskin (implemented).** Tailwind CSS v4 (`@tailwindcss/vite`) with
  CSS-first tokens in `src/styles/globals.css`, a shared `Button` primitive
  (`class-variance-authority`, `cn()`, Radix `Slot`, Lucide icons), and a shell
  grid that reflows from one to three columns. The system, backend, and process
  panels keep refresh/retry, paging, and per-field gap reporting. A persisted
  light/dark toggle selects the `[data-theme="dark"]` tokens. ADR-014.
- **Process inventory (implemented).** `list_processes` returns one page of
  processes through `ports/processes.rs`, `domain/process.rs`, and
  `adapters/windows/processes.rs` (Toolhelp snapshot + per-process query
  handles). Paging is validated in Rust (`limit` 1–500, `offset` ≤ 1,000,000),
  results are ordered by PID, and each row carries a `key` of
  `pid-startTime` or an explicit `pid:<pid>`.
- **Process list UI.** `src/services/processes.ts` and
  `src/components/ProcessList.tsx` render the page with loading, error, refresh,
  previous/next paging, a truncation note, and per-row unavailable reasons.
- ADR-013: process identity is PID plus start time, paging is server-side and
  fixed-order, and image paths, command lines, CPU usage, and session IDs are
  deliberately not collected in this slice.

### Changed
- `API.md` §1–§3 now describe the implemented envelope, `get_app_info`,
  `get_system_snapshot`, and `list_processes` instead of a proposed contract;
  error codes are documented in their serialized `snake_case` form. The proposed
  `include` request for the snapshot and the proposed `sort` request,
  `executablePath`, and `accessLimited` fields for the process list were dropped,
  and the document says so.
- `error.rs`: the module-level `#[allow(dead_code)]` is gone. The allowance is
  now scoped to the published `ErrorCode` taxonomy, which still contains variants
  no feature constructs yet.
- `build.md` §4 no longer marks `cargo fmt`/`cargo clippy` as verified without the
  components being installed. That claim was incorrect and is corrected, not
  quietly removed.
- `README.md` rewritten around the actual implementation state, with a
  documentation map and an honest capability table.
- `docs/plan.md`: authoritative plan moved from the repository root; `plan.md`
  is now a pointer.

### Documentation
- Recorded B-004 (app shell + single dashboard) as complete: `src/App.tsx`,
  `src/main.tsx`, and the five panels already met the acceptance criteria
  (responsive grid, loading/empty/error states per panel, no router). The backlog
  and the F-006 evidence now say so instead of listing it as outstanding.
- Full documentation set: architecture, product, security, operations, and AI
  workflow documents (`ARCHITECTURE.md`, `PRD.md`, `TECH_STACK.md`, `DATABASE.md`,
  `API.md`, `SECURITY.md`, `CONTRIBUTING.md`, `TESTING.md`, `AGENTS.md`,
  `AI_*.md`, and `docs/*`).
- `build.md`: command reference for the current toolchain, now including the new
  test/lint/format scripts.

### Fixed
- Toolchain documentation claimed `rustfmt`/`clippy` were available and verified;
  neither component was installed. Both are now installed and declared in
  `rust-toolchain.toml`.

### Security
- No network capability, no telemetry, no privilege escalation added.
- `get_app_info` returns no machine-identifying data.
- 10 moderate advisories exist in the **dev-only** lint toolchain (`minimatch`
  ReDoS). `npm audit --omit=dev` is clean; see `docs/KNOWN_ISSUES.md` KI-036.

---

## [0.1.0] — 2026-11-06

Initial scaffold. **Not a functional system scanner.**

### Added
- Tauri 2 desktop application (`src-tauri/`), Rust edition 2024.
- React 19 + TypeScript + Vite renderer (`src/`).
- Welcome screen with design tokens and a preview footer (`src/App.tsx`,
  `src/App.css`).
- Strict CSP and a minimal Tauri capability set (`core:default` only).
- NSIS x64 bundle configuration and generated installer.
- Release profile: LTO, `codegen-units = 1`, `opt-level = 3`, `panic = "abort"`,
  `strip = true`.

### Known limitations
- No IPC commands, system collection, process list, storage scan, persistence,
  diagnostics, tests, or CI.
- Installer is unsigned.
- No license selected.

[Unreleased]: #unreleased
[0.1.0]: #010--2026-11-06
