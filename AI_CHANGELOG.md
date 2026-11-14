# AI_CHANGELOG

> Record of **AI-assisted architectural and structural changes**. Product-facing
> version history lives in [`docs/CHANGELOG.md`](docs/CHANGELOG.md); decisions
> live in [`docs/DECISIONS.md`](docs/DECISIONS.md).
>
> Only record changes that affect structure, boundaries, contracts, or process.
> Routine edits do not belong here.
> **Last updated:** 2026-11-14

---

## Format

```text
## YYYY-MM-DD — <short title>
- Agent/tool: <name if known>
- Scope: <files/modules>
- Change: <what structurally changed>
- Why: <driving requirement or defect>
- Risk: <what could break>
- Evidence: <commands run / tests / review>
- Follow-up: <tracked items>
```

---

## 2026-11-14 — Diagnostics view + redacted support bundle (B-006)

- **Agent/tool:** OpenCode (DeepSeek V4.1 Flash Free).
- **Scope:** `src-tauri/src/{redact.rs,logging.rs,dto,domain,ports,adapters/windows,commands}`, `Cargo.toml`; renderer `Diagnostics` panel, service, tests; docs.
- **Change:**
  - Added a pure `redact.rs` module: user-profile and other drive-rooted paths,
    UNC paths, e-mail addresses, IPv4/IPv6, MAC addresses, SIDs, and literal
    account/machine names are replaced. `logging::redact_detail` now delegates to
    it, so error details and support bundles share one policy.
  - Added `domain/diagnostics.rs` (bundle text and a dependency-free UTC
    timestamp) and `dto/diagnostics.rs` (`DIAGNOSTICS_SCHEMA_VERSION = 1`).
  - Added `ports/diagnostics.rs` and a Windows adapter using `GetUserNameW`,
    `GetComputerNameW`, and token elevation (`Win32_Security` feature on the
    existing `windows-sys` dependency).
  - `logging.rs` gained bounded `summary` and `tail` readers; the tail starts on a
    whole line.
  - `get_diagnostics` builds the report on a blocking thread and logs only its
    outcome. It returns no absolute path and writes no file.
  - The renderer gained an on-request `Diagnostics` panel with environment facts,
    unavailable fields, a read-only preview, and copy-to-clipboard. It is a
    fourth panel in the existing shell; no router or navigation was added.
- **Why:** B-006 closes the Release 0 support requirement without telemetry. The
  preview is the only way a user shares diagnostics, and it is inspectable before
  copying.
- **Risk:**
  - Redaction is best-effort and pattern-based (KI-043); a bare personal name or
    an unrecognized path form can survive. The preview stays user-initiated and
    reviewable.
  - There is no saved-file export yet, so support receives pasted text.
  - The elevation probe reads the process token; it does not request elevation.
- **Evidence:** `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D
  warnings`, `cargo test` (52 passed), `npm test` (18 passed), `npm run build`,
  `npm run lint`, `npm run format:check`, `npm audit --omit=dev` (0).
- **Follow-up:** saved-file bundle export (F-092/B-064), a log level filter, and
  DB statistics once persistence exists.

---

## 2026-11-14 — Local structured logging with size-based rotation (B-007)

- **Agent/tool:** OpenCode (DeepSeek V4.1 Flash Free).
- **Scope:** `src-tauri/src/logging.rs`, `lib.rs`, `ipc.rs`, `error.rs`,
  `commands/{app,system,processes}.rs`, `Cargo.toml`/`Cargo.lock`; docs.
- **Change:**
  - Added a `tracing` + `tracing-subscriber` subscriber that writes to Tauri's
    `app_log_dir()`. `logging.rs` owns rotation: 10 MB per file, five files,
    oldest removed first; rotation is tested with small limits.
  - `lib.rs` initializes logging in `setup` and logs the app version and log
    path. Initialization failure is non-fatal and reported on stderr.
  - Every command logs `command`, `outcome`, and (on failure) the stable code and
    correlation ID through `CommandResult::log_outcome`. Payloads and inputs are
    never logged; error `detail` is logged with `redact_detail`, which replaces
    user-profile names and drive-rooted paths.
  - `ErrorCode` gained a stable `as_str()`/`Display` so logs and the wire share
    one spelling.
- **Why:** B-007 was the last unblocked Release 0 diagnostics blocker; without
  logs, field failures cannot be diagnosed and a support bundle (B-006) has no
  source.
- **Risk:**
  - Logs are plain text on disk; no secret is logged by policy, but a future call
    site could add one. The redaction helper only covers path-shaped tokens.
  - `tracing-subscriber` pulls four small transitive crates; defaults were
    disabled (no ANSI/log-bridge) to keep the tree small.
  - A global subscriber can only be installed once; tests use scoped subscribers
    instead of `init`.
- **Evidence:** `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D
  warnings`, `cargo test` (37 passed), `npm test` (15 passed), `npm run build`.
- **Follow-up:** B-006 diagnostics view + redacted support bundle; optional
  retention/size visibility in the UI.

---

## 2026-11-14 — Renderer reskin: Tailwind CSS v4 + responsive panel grid

- **Agent/tool:** OpenCode (DeepSeek V4.1 Flash Free).
- **Scope:** `src/App.tsx`, `src/components/*`, `src/components/ui/button.tsx`,
  `src/lib/utils.ts`, `src/styles/globals.css`, `vite.config.ts`, `package.json`;
  removed `src/App.css`. Rust, domain, ports, adapters, and IPC untouched.
- **Change:**
  - Replaced the hand-written `App.css` with Tailwind CSS v4 configured through
    `@tailwindcss/vite`; the palette, radius scale, focus ring, and light/dark
    token sets now live in `src/styles/globals.css` (`@theme` / `@theme inline`).
  - Added a shared `Button` primitive (`cva` variants, `cn()` merge, Radix
    `Slot` for `asChild`, Lucide icons) under `src/components/ui/`.
  - Apps shell now renders three panels in a 1 → 2 → 3 column responsive grid:
    backend status, system snapshot, and a full-width paged process table.
  - Added a persisted light/dark toggle (`ThemeToggle`) that selects the
    `[data-theme="dark"]` token set and remembers the choice in `localStorage`.
  - Preserved all prior behavior: refresh/retry, single in-flight refresh,
    paging with the Rust-supplied offset/limit, per-field `Unavailable (reason)`
    text, truncation and denial notes, and the empty-list message.
  - Tests were updated to target roles/labels and to assert the restored
    behaviors (range caption, refresh recovery, safety-cap note).
- **Why:** The Release 0 UI grew ad-hoc CSS and panels duplicated control
  styling. The reskin gives one token layer and one responsive contract before
  more panels are added.
- **Risk:**
  - Tailwind utilities fail silently when mistyped; reviewers must check class
    strings (`data-theme('…')?`-style pseudo-syntax is invalid).
  - Adding seven direct dependencies expands the frontend tree; recorded in
    `docs/DEPENDENCIES.md` and ADR-014.
  - The app now ships a manual light/dark toggle persisted in `localStorage`;
    OS-preference following (F-009 system mode) is still missing.
- **Evidence:** `npm test -- --run` (15 passed, 5 files), `npm run build`
  (tsc + Vite), `npm run lint`, `npm run format:check`; Rust gates unchanged
  (32 passed).
- **Follow-up:** F-009 OS-preference following, F-010 accessibility review, and a
  visual smoke test under WebView2 (not part of the automated gates).

---

## 2026-11-13 — Process inventory vertical slice (paged port → adapter → DTO → UI)

- **Agent/tool:** OpenCode (DeepSeek V4.1 Flash Free).
- **Scope:** `src-tauri/src/{ports/probe.rs, ports/processes.rs,
  domain/process.rs, adapters/windows/processes.rs, dto/process_list.rs,
  dto/unavailable.rs, commands/processes.rs, commands/mod.rs, lib.rs}`,
  `src-tauri/Cargo.toml`, `src/services/processes.ts`, `src/lib/*`,
  `src/components/ProcessList.{tsx,test.tsx}`, `src/App.{tsx,css}`.
- **Change:**
  - Added the second vertical slice: `ProcessInfoPort` → `domain/process.rs` →
    Windows adapter → `ProcessListDto` → `list_processes` → `ProcessList`.
  - Paging is a domain rule (`PageRequest::new`), so `limit`/`offset` are
    rejected before any OS work; the command only maps the rejection to
    `invalid_input`.
  - Identity is `pid-startTime`, or `pid:<pid>` when the start time is denied,
    and the difference is visible on the wire (ADR-013).
  - Moved `ProbeError` to `ports/probe.rs` and the shared
    `UnavailableFieldDto`/`take` helpers to `dto/unavailable.rs`, because both
    features now need them. `now_ms` moved to `commands/mod.rs` for the same
    reason.
  - Extracted `formatBytes`/`formatTime`/`formatDateTime` and
    `summarizeError` into `src/lib/` so the two panels share them instead of
    keeping copies.
- **Why:** B-003 is the next P0 and the first command with untrusted input. It
  also establishes that a partially readable row is still a row, which every
  later process view depends on.
- **Risk:**
  - Verified on one machine (KI-039); edition- and account-specific access
    behaviour is untested.
  - No image paths, command lines, CPU usage, or session IDs (KI-040), and no
    sort/search (KI-042). A user may expect those from a process list.
  - Working set is denied for processes owned by other accounts (KI-041).
  - Two `windows-sys` features were added to the audited surface.
- **Evidence:** `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D
  warnings`, `cargo test` (32 passed), `npm test -- --run` (13 passed),
  `npm run lint`, `npm run format:check`, `npm run build`.
- **Follow-up:** B-004 (app shell), B-006/B-007 (diagnostics + logging),
  B-090 (runtime DTO validation), B-092 (first green CI run), B-042 (hierarchy
  and Rust-side search).

---

## 2026-11-13 — System snapshot vertical slice (port → adapter → DTO → UI)

- **Agent/tool:** OpenCode (DeepSeek V4.1 Flash Free).
- **Scope:** `src-tauri/src/{ports/*, domain/*, adapters/windows/*,
  dto/system_snapshot.rs, commands/system.rs, error.rs, lib.rs}`,
  `src-tauri/Cargo.toml`, `src/services/system.ts`,
  `src/components/SystemOverview.{tsx,test.tsx}`, `src/App.{tsx,css}`.
- **Change:**
  - Created the layering the architecture rules already described: a
    platform-neutral `domain/` model with explicit availability reasons, a
    one-capability `ports/` trait, and a `windows-sys` adapter containing every
    `unsafe` block.
  - Added `get_system_snapshot` (no input, read-only, blocking work on a
    blocking thread) returning a versioned `SystemSnapshotDto`. Wholly
    unavailable groups are omitted and named in `unavailable[]`; leaf gaps keep
    their group and carry a field path and reason.
  - Added `SystemOverview` with loading, error, refresh, last-updated, and
    per-field unavailable display. A single in-flight refresh plus an `active`
    guard prevents stale responses from winning.
  - Removed the module-level `#[allow(dead_code)]` from `error.rs`; the
    allowance is now scoped to the published `ErrorCode` taxonomy (B-091).
  - Added `windows-sys` 0.61.2 as a `cfg(windows)` dependency (ADR-012).
- **Why:** B-002 is the first Release 0 value and the first real test of the
  domain/OS boundary. Building it before the app shell proves the contract
  without locking in navigation or feature state.
- **Risk:**
  - Win32 collection is verified on one machine (KI-039); edition- and
    architecture-specific behavior is not covered.
  - Network drives deliberately report `unsupported` sizes/labels (KI-038); a
    user may read that as missing data.
  - Nine `windows-sys` feature flags are enabled; each addition expands the
    audited surface.
  - CI still has not run on GitHub (B-092), so none of this is gated remotely.
- **Evidence:** `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D
  warnings`, `cargo test` (18 passed), `npm run build`, `npm test` (9 passed),
  `npm run lint`, `npm run format:check`.
- **Follow-up:** B-004 (app shell/dashboard), B-005 completion at app-shell level,
  B-090 (runtime DTO validation), B-092 (first green CI run), B-003 (processes).

---

## 2026-11-06 — IPC vertical slice implemented (Rust boundary → React)

- **Agent/tool:** OpenCode (DeepSeek V4.1 Flash Free).
- **Scope:** `src-tauri/src/{lib.rs, ipc.rs, error.rs, dto/*, commands/*}`,
  `src/services/*`, `src/components/BackendStatus.tsx`,
  `package.json`, `tsconfig.node.json`, `eslint.config.js`, `vitest.config.ts`,
  `.prettierrc.json`, `.prettierignore`, `rust-toolchain.toml`,
  `.github/workflows/ci.yml`, `src/App.{tsx,css}`.
- **Change:**
  - Established the renderer → IPC → Rust path with one read-only command
    (`get_app_info`) returning a versioned DTO through the `CommandResult`
    envelope.
  - Introduced the stable error taxonomy (`ErrorCode`) and `AppError` with
    correlation IDs as the only error shape crossing the boundary.
  - Added the single frontend `invoke` call site (`src/services/ipc.ts`) and a
    domain module per command; components no longer touch Tauri directly.
  - Changed the wire format of DTO fields from the originally specified
    `snake_case` to `camelCase` (`#[serde(rename_all = "camelCase")]`), removing
    the hand-written TypeScript mapping layer. Recorded as ADR-011.
  - Added the test/lint/format toolchain, a `rust-toolchain.toml`, and a
    three-job Windows CI workflow.
- **Why:** Everything else in Release 0 (system collection, processes, jobs,
  persistence) depends on a typed, stable, testable boundary. Building the data
  collectors first would have locked in an untested error/shape contract.
- **Risk:**
  - `error.rs` is only exercised by tests, so it carries a scoped
    `#[allow(dead_code)]` on the module. This hides dead code there until the
    first fallible command lands (B-091) — deliberately time-boxed.
  - The CI workflow is untested on GitHub (B-092). Local runs of every step pass,
    but a workflow file is not a gate.
  - `camelCase` diverges from the original `API.md` design; anyone relying on the
    old spec would break. Documented rather than silently changed.
- **Evidence:** `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D
  warnings`, `cargo test` (8 passed), `npm run build`, `npm test` (6 passed),
  `npm run lint`, `npm run format:check`.
- **Follow-up:** B-002 (`get_system_snapshot`), B-004 (app shell), B-090 (runtime
  DTO validation), B-091 (remove the dead-code allow), B-092 (first green CI run).

---

## 2026-11-06 — Documentation system established

- **Scope:** repository root and `docs/` (documentation only; no application code
  was added or modified).
- **Change:**
  - Created the full engineering/product/security documentation set.
  - Moved the authoritative plan from `plan.md` to `docs/plan.md`; left a pointer
    stub at `plan.md` so existing links keep working.
  - Created `build.md` as the single source of truth for commands, with each entry
    marked Verified or Planned.
  - Redesigned `README.md` around the honest current state rather than the target
    product.
- **Why:** The repository had only the default Tauri template README and a plan
  file; there was no shared contract for commands, architecture, security, or AI
  workflow. Documentation drift was guaranteed without one.
- **Risk:** Documentation now describes a target architecture that does not exist
  yet. Mitigated by a status banner in every document and a "what does not exist"
  section in `AI_CONTEXT.md`.
- **Evidence:**
  - Verified toolchain versions: Node `24.20.0`, npm `11.19.0`, Rust `1.99.1`,
    Tauri CLI `2.12.4`.
  - Verified installer artifact:
    `src-tauri/target/release/bundle/nsis/System Scanner_0.1.0_x64-setup.exe`.
  - Confirmed no IPC commands in `src-tauri/src/lib.rs`, no DB, no tests, no CI.
- **Follow-up:**
  - Decide DB access layer (`rusqlite` vs `sqlx`) — `docs/DECISIONS.md` ADR-005.
  - Add CI, linters, and a test runner — `docs/BACKLOG.md`.
  - Resolve license, signing, and supported-version matrix.

---

## Template for the next entry

```text
## YYYY-MM-DD — <title>
- Agent/tool:
- Scope:
- Change:
- Why:
- Risk:
- Evidence:
- Follow-up:
```
