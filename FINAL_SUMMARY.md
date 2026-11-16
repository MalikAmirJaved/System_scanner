## What was done

### B-006 — Diagnostics view + redacted support bundle (committed `9dc9d14`)

Added a fourth on-request panel to the existing dashboard. The panel is generated only when the user clicks **Generate bundle**; browsing the dashboard never reads the log file.

- **`redact.rs`** — pure in-process redaction of paths, e-mail, IPv4/IPv6, MAC, SIDs, and literal account/machine names. Quote-aware tokenizer so `C:\Users\ada\report.txt` redacts the path but keeps `report.txt` intact. 7 unit tests.
- **`domain/diagnostics.rs`** — renders the support-bundle header text and a dependency-free UTC timestamp from Unix milliseconds. 4 tests.
- **`dto/diagnostics.rs`** — wire DTO (`DIAGNOSTICS_SCHEMA_VERSION = 1`) with `DiagnosticsDto`, `LogSummaryDto`, `BundlePreviewDto`. Missing `log`, `log.tail`, or `os.*` leaf produces an `unavailable` entry; `os.arch` is always reported. 2 tests.
- **`ports/diagnostics.rs` + `adapters/windows/diagnostics.rs`** — identity (`GetUserNameW`/`GetComputerNameW`) and token elevation via `Win32_Security` on the existing `windows-sys` dependency; zero new crates.
- **`logging.rs`** — bounded `summary` and `tail` readers (256 KiB cap, starts on a whole line boundary); `redact_detail` now delegates to `crate::redact::redact_text`; 4 tests.
- **`commands/diagnostics.rs`** — `get_diagnostics` runs on `spawn_blocking`; renders a `SupportBundle` with environment facts; returns no absolute path, writes no file, uploads nothing.
- **`src/services/diagnostics.ts`** — typed IPC wrapper (`getDiagnostics`).
- **`src/components/Diagnostics.tsx`** — idle/loading/error/ready states; environment facts; read-only preview textarea; copy-to-clipboard with success/failed states; 3 tests.
- **`src/components/Diagnostics.test.tsx`** — on-demand load, error/retry, copy; 3 tests.
- Integrated as a fourth full-width panel in `src/App.tsx`; no router or navigation added.

### B-012 / F-008 — Fatal-startup error screen + renderer error boundary

Replaced the final `expect("error while running tauri application")` in `lib.rs` with `startup::report_fatal`, which shows a native `MessageBoxW` dialog and exits status 1. Added a React `ErrorBoundary` that catches render failures and shows a reload screen instead of a blank window.

- **`src-tauri/src/startup.rs`** — `report_fatal` builds a redacted, length-bounded message (app version, detail ≤400 chars, three next steps), calls `MessageBoxW`, then `std::process::exit(1)`. Unit-tested.
- **`src-tauri/src/adapters/windows/dialog.rs`** — thin wrapper around `MessageBoxW` using the `Win32_UI_WindowsAndMessaging` feature on `windows-sys`; no new crate.
- **`src/main.tsx`** — `<App />` wrapped in `<ErrorBoundary>`; render failure shows a reload screen with the error message and the log-file path.
- **`src/components/ErrorBoundary.tsx`** — React error boundary; 1 test; `componentDidCatch` logs to the dev console only.

### Docs updated (status-honest)

| Doc | Change |
|---|---|
| API.md | `get_diagnostics` marked implemented; §3.4 contract |
| FEATURES.md | F-080/F-081 ✅; counts 25 implemented |
| BACKLOG.md | B-006 ✅; B-004 wording |
| KNOWN_ISSUES.md | KI-012 (no saved-file export); KI-043 (best-effort redaction) |
| DECISIONS.md | ADR-016 records preview/copy and redaction policy |
| MONITORING.md | Diagnostics view + bundle status |
| ROADMAP.md | Diagnostics complete; 70 tests; fatal-startup added |
| AI_CHANGELOG.md | Entries for B-006 and B-012 |
| AI_CONTEXT.md | Updated counts, four IPC commands, diagnostics panel |
| CHANGELOG.md | B-006 + B-012 entries |
| DEPENDENCIES.md | windows-sys features; no new crate |
| DEVOPS.md | 18 npm test / 54 cargo test counts |
| TESTING.md | 54 Rust + 20 frontend; expanded descriptions |
| build.md | cargo test 54; npm test 20 + 7 files |
| README.md | four read-only commands; diagnostics + boundary |
| ARCHITECTURE.md | startup.rs + dialog.rs in module map |

### Files

**New** (9): `redact.rs`, `domain/diagnostics.rs`, `dto/diagnostics.rs`, `ports/diagnostics.rs`, `adapters/windows/diagnostics.rs`, `commands/diagnostics.rs`, `startup.rs`, `adapters/windows/dialog.rs`, `ErrorBoundary.tsx` + `ErrorBoundary.test.tsx`

**Modified** (31): `Cargo.toml`, `lib.rs`, `main.tsx`, `adapters/windows/mod.rs`, all doc files, `TESTING.md`, `build.md`, `README.md`, `ARCHITECTURE.md`

### Verification

- `cargo fmt --all -- --check`: 0
- `cargo clippy --all-targets -- -D warnings`: 0
- `cargo test`: 54 passed (0 failed)
- `npm test -- --run`: 20 passed across 7 files (0 failed)
- `npm run build`: 0
- `npm run lint`: 0
- `npm run format:check`: 0
- `npm audit --omit=dev`: 0 vulnerabilities

### Not verified

Manual smoke test in a running Tauri window (starting failure + dialog; render failure + boundary) cannot be automated from this session.

### Next step

The next unblocked P0 is **B-010** (Windows CI workflow never executed on GitHub) or **B-011** (rust-toolchain exact pinning) per BACKLOG.md §12, or begin B-020 persistence work.