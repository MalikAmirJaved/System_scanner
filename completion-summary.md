## Summary

Two Release 0 blockers finished in this session.

### B-006 — Diagnostics view + redacted support bundle (already committed: `9dc9d14`)

- `redact.rs` — pure in-path redaction of user-profile, UNC, e-mail, IPv4/IPv6, MAC, SIDs, and literal account/machine names; quote-aware tokenizer; 7 tests
- `domain/diagnostics.rs` — bundle text rendering, UTC timestamp without date crate, 4 tests
- `dto/diagnostics.rs` — wire DTO (`DIAGNOSTICS_SCHEMA_VERSION = 1`), 2 tests; missing `log`, `log.tail`, `os.*` paired with `unavailable` entries; `os.arch` always reported
- `ports/diagnostics.rs` + `adapters/windows/diagnostics.rs` — identity (`GetUserNameW`/`GetComputerNameW`) and token elevation via `Win32_Security` on existing `windows-sys` dependency; no new crate
- `logging.rs` — bounded `summary` and `tail` readers (256 KiB cap, whole-line start); 4 tests; `redact_detail` now delegates to `crate::redact::redact_text`
- `commands/diagnostics.rs` — `get_diagnostics` runs on `spawn_blocking`, renders a `SupportBundle` with environment facts, returns no absolute path, writes no file, uploads nothing
- `src/services/diagnostics.ts` — typed IPC wrapper (`getDiagnostics`)
- `src/components/Diagnostics.tsx` — idle/loading/error/ready states, environment facts, unavailable fields, read-only preview textarea, copy + copied/failed states; 3 tests
- `src/components/Diagnostics.test.tsx` — on-demand load, error/retry, copy; 3 tests
- Added as fourth panel in `src/App.tsx`; no router or navigation added

### B-012 / F-008 — Fatal-startup error screen + renderer error boundary (new in this session)

- `src-tauri/src/startup.rs` — `report_fatal` builds a redacted, length-bounded message (version, detail, three next steps), shows a native `MessageBoxW` dialog, exits with status 1; unit-tested
- `src-tauri/src/adapters/windows/dialog.rs` — narrow `MessageBoxW` call using existing `Win32_UI_WindowsAndMessaging` feature on `windows-sys`; no new crate
- `src/main.tsx` — `<App />` wrapped in `<ErrorBoundary>`; render failure shows a reload screen with error message and log-file reference instead of blank window
- `src/components/ErrorBoundary.tsx` — React error boundary; 1 test; componentDidCatch logs to dev console only
- Replaced the final `expect("error while running tauri application")` in `lib.rs` with `startup::report_fatal`; Cargo.toml enabled `Win32_UI_WindowsAndMessaging` feature (no new dependency)

### Docs updated
API.md, FEATURES.md, BACKLOG.md, KNOWN_ISSUES.md, DECISIONS.md, MONITORING.md, ROADMAP.md, AI_CHANGELOG.md, AI_CONTEXT.md, CHANGELOG.md, DEPENDENCIES.md, DEVOPS.md, TESTING.md, build.md, README.md, ARCHITECTURE.md

### Final counts
- **54 Rust tests** (added `startup`, `redact`)
- **20 frontend tests** (6 files: `ipc`, `BackendStatus`, `ThemeToggle`, `SystemOverview`, `ProcessList`, `Diagnostics`, `ErrorBoundary`)
- **70 total** (54 + 20)