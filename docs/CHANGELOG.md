# Changelog

All notable changes to this project are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versioning follows [Semantic Versioning](https://semver.org/) once a public release
exists. Until then, `0.x` versions may change behavior without notice.

> AI-assisted **architectural** changes are recorded separately in
> [`../AI_CHANGELOG.md`](../AI_CHANGELOG.md).

---

## [Unreleased]

### Added
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
