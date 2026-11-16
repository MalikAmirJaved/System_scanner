Changed — files and behaviour actually modified:

Rust backend:
  src-tauri/src/redact.rs (new) — best-effort path/identity redaction, 7 tests
  src-tauri/src/domain/diagnostics.rs (new) — bundle text + UTC timestamp, 4 tests
  src-tauri/src/dto/diagnostics.rs (new) — DiagnosticsDto + LogSummaryDto, 2 tests
  src-tauri/src/ports/diagnostics.rs (new) — DiagnosticsPort trait
  src-tauri/src/adapters/windows/diagnostics.rs (new) — identity + elevation via Win32
  src-tauri/src/commands/diagnostics.rs (new) — get_diagnostics wiring
  src-tauri/src/logging.rs — bounded summary/tail, redact_detail now delegates to redact.rs
  src-tauri/src/lib.rs — added mod redact; wired get_diagnostics to invoke handler

Frontend:
  src/services/diagnostics.ts (new) — typed IPC wrapper
  src/components/Diagnostics.tsx (new) — on-demand diagnostics panel, 3 tests
  src/components/Diagnostics.test.tsx (new) — on-demand load, error/retry, copy, 3 tests
  src/App.tsx — Diagnostics added as fourth panel; no router/navigation

Docs updated (status-honest):
  API.md — 3.4 get_diagnostics now "implemented"; command contract
  FEATURES.md — F-080/F-081 implemented; counts 25/4/40 → 25 features
  BACKLOG.md — B-006 ✅; B-004 wording updated
  KNOWN_ISSUES.md — KI-012 added (no saved-file export); KI-043 redaction best-effort
  DECISIONS.md — ADR-016 records preview/copy and best-effort redaction policy
  MONITORING.md — diagnostics view and support bundle status updated
  ROADMAP.md — diagnostics complete; 70 tests; panels count
  AI_CHANGELOG.md — entry for B-006 + B-012
  AI_CONTEXT.md — updated counts/state, four IPC commands, diagnostics panel
  CHANGELOG.md — B-006 + B-012 entries
  DEPENDENCIES.md — windows-sys feature list; no new crate added
  DEVOPS.md — 18 npm test / 52 cargo test → updated counts
  TESTING.md — 54 Rust + 20 frontend; expanded test descriptions
  build.md — cargo test 54 pass; npm test 20 pass + 7 files
  README.md — four read-only commands; diagnostics + boundary
  ARCHITECTURE.md — startup.rs + dialog.rs added to module map

New files (9):
  src-tauri/src/redact.rs
  src-tauri/src/domain/diagnostics.rs
  src-tauri/src/dto/diagnostics.rs
  src-tauri/src/ports/diagnostics.rs
  src-tauri/src/adapters/windows/diagnostics.rs
  src-tauri/src/commands/diagnostics.rs
  src-tauri/src/startup.rs
  src-tauri/src/adapters/windows/dialog.rs
  src/components/ErrorBoundary.tsx
  src/components/ErrorBoundary.test.tsx

Modified files (31):
  src-tauri/Cargo.toml (Win32_Security + Win32_UI_WindowsAndMessaging features)
  src-tauri/src/lib.rs
  src/main.tsx
  src-tauri/src/adapters/windows/mod.rs
  docs/FEATURES.md, BACKLOG.md, KNOWN_ISSUES.md, DECISIONS.md, MONITORING.md,
  ROADMAP.md, AI_CHANGELOG.md, AI_CONTEXT.md, CHANGELOG.md,
  DEPENDENCIES.md, DEVOPS.md, TESTING.md, build.md, README.md, ARCHITECTURE.md

Verified — exact commands + outcome:
  cargo fmt --all -- --check: 0
  cargo clippy --all-targets -- -D warnings: 0
  cargo test: 54 passed (0 failed)
  npm test -- --run: 20 passed (7 files, 0 failed)
  npm run build: 0
  npm run lint: 0
  npm run format:check: 0
  npm audit --omit=dev: 0 vulnerabilities

Not verified — manual smoke test in a real Tauri window cannot be run from this session; record as not verified.

Known gaps / next step:
  - B-010/B-011 (CI on Windows, rust-toolchain pinning) remain open per BACKLOG.md
  - B-004 (app shell) already complete; panel grid is responsive 1/2/3-column
  - Next unblocked P0 after this: B-012 / F-008 completion (done); then B-004 wording tweak, or begin B-020 persistence