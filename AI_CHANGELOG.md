# AI_CHANGELOG

> Record of **AI-assisted architectural and structural changes**. Product-facing
> version history lives in [`docs/CHANGELOG.md`](docs/CHANGELOG.md); decisions
> live in [`docs/DECISIONS.md`](docs/DECISIONS.md).
>
> Only record changes that affect structure, boundaries, contracts, or process.
> Routine edits do not belong here.
> **Last updated:** 2026-11-19

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

## 2026-11-19 — Handle-based directory access closes F-01 (B-041 identity half)

- **Agent/tool:** Buffy (DeepSeek V4.1 Flash), Freebuff.
- **Scope:** new `src-tauri/src/adapters/filesystem/directory.rs`;
  `src-tauri/src/adapters/filesystem/{mod,traversal}.rs`;
  `src-tauri/src/domain/traversal.rs` (`FileIdentity`, `IdentityCheck`,
  `compare_identity`); `src-tauri/Cargo.toml` unchanged;
  `docs/{THREAT_MODEL,KNOWN_ISSUES,BACKLOG,FEATURES,DECISIONS}.md`,
  `ARCHITECTURE.md`, `SECURITY.md`, `AI_CONTEXT.md`, `docs/CHANGELOG.md`.
- **Change:** Directory access now goes through one handle. `DirectoryHandle::open`
  uses `CreateFileW` with `FILE_FLAG_OPEN_REPARSE_POINT |
  FILE_FLAG_BACKUP_SEMANTICS`, reads the volume serial and 128-bit file id
  (`FileIdInfo`) from that handle, and enumerates children through it
  (`FileIdExtdDirectoryInfo`, 16 KiB batches). The walker records a child's id in
  the work item, compares it with the opened id, and reports a mismatch as
  `disappeared`; the per-child by-path `symlink_metadata` is removed. A new
  module boundary is documented (ARCHITECTURE §4.5) and the decision recorded as
  ADR-025.
- **Why:** Threat-model finding F-01 / KI-055: checking a directory by path and
  then enumerating by path let a junction swapped in between be followed. The
  review made closing it a precondition for the first content read or write
  (hashing, duplicates).
- **Risk:** `unsafe` Win32 calls sit in the enumeration path; a wrong buffer or
  offset interpretation would misreport entries. Mitigated by `SAFETY:` comments,
  tests for batch refills (1,200- and 1,500-entry directories), identity
  stability, junction reporting, and the pre-existing denied/Unicode/long-path
  tests. Non-Windows behavior is unchanged but compiles only; it records no
  identity.
- **Evidence:** `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D
  warnings`, `cargo test` (134 passed), `npm run build` — all green on Windows.
- **Follow-up:** protected-path allowlist policy is still the other half of
  B-041; KI-056 (unverifiable identity) must become a typed outcome before content
  reads; F-02 (UNC/network/removable root policy) remains an open product
  decision. Documentation defect noted: the 2026-11-17 entry cites ADR-024, but
  `docs/DECISIONS.md` contains no ADR-024, so this change uses ADR-025 rather than
  reusing the number.

## 2026-11-17 — File-operation threat-model review (B-040)

- **Agent/tool:** DeepSeek V4.1 Flash (opencode).
- **Scope:** `docs/THREAT_MODEL.md` (new §11, T-14, status header),
  `SECURITY.md` (§6), `docs/ACCESS_CONTROL.md` (§4, §9), `docs/KNOWN_ISSUES.md`
  (KI-049 updated, KI-055 added), `docs/RISKS.md` (R-12), `docs/BACKLOG.md`,
  `docs/CHANGELOG.md`, `docs/DECISIONS.md` (ADR-024). No source code.
- **Change:** The first review of an **implemented** operation surface. It records
  what the scan path actually does (validated roots, one scan at a time,
  reparse points skipped, counters only, counters-only DB row), seven findings
  (F-01–F-07), a new threat (T-14), and a residual-risk table. Conclusion: the
  read-only surface is acceptable for pre-alpha; every content-read or write path
  must first replace the path-based check-then-open walk with a handle-based open
  and a volume-serial + file-ID identity check. UNC/network/removable root policy
  is explicitly left as an open product decision.
- **Why:** B-040 gates all P2 security work; the previous document was a draft
  that predated the IPC wiring and said no review had happened.
- **Risk:** Documentation only. The review could be read as broader assurance than
  it is; §11.1 and the status header limit it to a paper review of read-only code.
- **Evidence:** Source and tests read for `commands/scan.rs`, `dto/scan.rs`,
  `app/jobs.rs`, `adapters/filesystem/traversal.rs`, `domain/traversal.rs`, and
  `adapters/persistence/`; doc format check run.
- **Follow-up:** B-041 (protected paths + handle-based identity), the F-02 root
  policy decision, KI-055, and extending §11 before the first write path.

---

- **Agent/tool:** DeepSeek V4.1 Flash (opencode).
- **Scope:** `src-tauri/src/adapters/persistence/sqlite.rs` (constant, error
  variants, `configure`, `backup_before_migration`, snapshot helpers, tests);
  status docs.
- **Change:** `Database::open` now takes one verified snapshot when an existing
  file reports a schema version below `SCHEMA_VERSION`, before the migration
  runner touches it. `VACUUM INTO` writes
  `backups/system-scanner-v{version:03}-{epoch_ms}.db`; the file is reopened
  read-only and must pass `PRAGMA quick_check`. A failed snapshot is deleted;
  the newest five are pruned by file-name order; unrelated files are untouched.
  Fresh files (no `schema_migrations`) and up-to-date files are not snapshotted.
  `from_connection` (in-memory and tests) keeps configure+migrate only.
- **Why:** KI-014 recorded that no pre-migration backup existed, and
  `docs/BACKUP_POLICY.md` §2.1/§3 require `VACUUM INTO` with verification.
  B-033 was the next unblocked P1 item.
- **Risk:** the snapshot runs on the startup path while the single connection is
  held, so a slow disk delays startup by one copy; it is best-effort, so a
  migration can still proceed without a snapshot. `VACUUM INTO` binds a lossy
  UTF-8 path (app-data paths), and the destination is chosen so it never
  overwrites an existing file. No IPC, capability, DTO, or dependency change.
- **Evidence:** `cargo fmt --all -- --check` clean, `cargo clippy --all-targets --
  -D warnings` clean, `cargo test` 125 passed (5 new: snapshot+restore, no
  snapshot when current, backup failure tolerated, retention, corrupt
  verification); `npm run build`/`test`/`lint`/`format:check` unchanged and green.
- **Follow-up:** make the snapshot mandatory before the first non-additive
  migration; add a restore command and a user-visible backup size (KI-014, ADR-023).

---

- **Agent/tool:** DeepSeek V4.1 Flash (opencode).
- **Scope:** new `src/services/validate.ts`, `src/services/unavailable.ts`,
  `src/services/contracts.test.ts`, `src/services/validate.test.ts`;
  `src/services/{ipc,app,system,processes,diagnostics,scan}.ts`;
  `src/lib/errors.ts`; `src/components/ScanPanel.tsx` and its test.
- **Change:** the renderer now validates every command response and every
  `scan:*` event at runtime. `invokeCommand(command, parse, args?)` takes a
  **required** guard, so the type checker finds an unvalidated call site.
  Guards check the envelope, the error payload and its code enum, the DTO
  (`schemaVersion`, enums, counters, collection caps), copy known fields only,
  and reject a present field of the wrong type instead of defaulting it.
  Failures throw `IpcContractError` (`source`, `problem`), mapped to
  `contract_violation` by `summarizeError`. `subscribeToScan` gained an
  `onInvalid` callback; the panel reports the failure and re-reads
  `get_scan_status` so a dropped terminal event cannot stall it.
- **Why:** KI-011 — a DTO rename rendered `undefined` instead of failing, which
  contradicts invariant 1 and `API.md` §8. B-090 was the highest open P0.
- **Risk:** guards duplicate each DTO declaration; a new Rust field is invisible
  until its guard copies it (KI-054). `IpcContractError` intentionally does not
  extend `IpcError`, so any caller that special-cases `IpcError` keeps working
  but must not assume every thrown value is one. No Rust, capability, dependency,
  or wire-format change.
- **Evidence:** `npm test` 76 passed (12 files), `npm run lint` clean,
  `npm run format:check` clean, `npm run build` (`tsc` + Vite) succeeded,
  `cargo test` 120 passed (unchanged; no Rust edit).
- **Follow-up:** keep the guards in step with DTO changes; revisit generated
  types if the DTO count grows (ADR-022).

---

## 2026-11-17 — Storage scan panel (first UI caller of the scan commands)

- **Agent/tool:** Cursor agent.
- **Scope:** new `src/services/scan.ts`, `src/lib/scanView.ts`,
  `src/components/ScanPanel.tsx` and their tests; `src/App.tsx`; status docs.
- **Change:** a panel that starts, cancels, and watches one scan. A pure reducer
  (`applyEvent`, `applyStatus`) enforces event ordering: a job is adopted only
  while starting, other jobs' events and non-newer `seq` are dropped, and
  `get_scan_status` is applied after start, after cancel, and on mount.
  `subscribeToScan` is the first use of the Tauri event API; it unsubscribes
  partial subscriptions on failure.
- **Why:** the backend scan had no way to be used; this makes it testable by a
  person and fixes the event/status contract the UI relies on.
- **Risk:** no capability change (`core:default` includes `allow-listen`,
  checked in the generated manifest). The panel shows counters only. Tests mock
  IPC, so real event delivery and cancel are unverified (KI-052). Typed roots, no
  picker (KI-053). `emit` is also allowed to the page by `core:default`; nothing
  in the backend listens to UI-emitted events.
- **Evidence:** see the session report.
- **Follow-up:** run the app (KI-052), a picker with a capability review, a
  history read port, and per-entry persistence after the retention decision.

---

## 2026-11-17 — Scan root hardening: device namespaces refused (KI-049)

- **Agent/tool:** Cursor agent.
- **Scope:** `src-tauri/src/dto/scan.rs` (`is_device_namespace`, two tests);
  security/access/API/known-issue docs.
- **Change:** `StartScanRequest::parse` rejects `\\.\`, `\??\`, and `\\?\` paths
  that are not a drive or UNC form, after normalizing `/` to `\`. No dependency,
  capability, or IPC shape change.
- **Why:** untrusted IPC supplies roots; closing the device namespace before any UI
  can call the command is the cheapest part of KI-049.
- **Risk:** `\\?\Volume{GUID}\` is a legitimate volume path and is now refused
  (conservative). UNC shares remain accepted. Reserved DOS names in normal paths
  and root ACLs are not checked; Windows resolves them.
- **Evidence:** see the session report.
- **Follow-up:** B-040 decision on UNC and an allow-list tied to a folder picker.

---

## 2026-11-17 — Scan session persistence, counters only (B-024 minimal)

- **Agent/tool:** Cursor agent.
- **Scope:** new `ports/sessions.rs`, `adapters/persistence/sessions.rs`,
  `migrations/0002_scan_session_results.sql`; `migrations.rs` (`SCHEMA_VERSION`
  2); `sqlite.rs` (`connection` visibility, v1 -> v2 test); `app/jobs.rs`
  (optional store, `unix_ms` made public); `lib.rs` (supervisor built in `setup`
  after the database, `mark_interrupted`, `try_state` in the window hook); docs.
- **Change:** a counters-only session row per scan behind a port, written
  best-effort; startup marks leftover open rows `interrupted`. `database` is now
  managed as `Arc<Database>` (nothing read the old `Database` state).
- **Why:** a scan with no recorded outcome cannot be told from a crash, and the
  retention decision (plan §12) blocks storing paths, so only the session is
  stored.
- **Risk:** rows accumulate without limit (KI-050); a failed write leaves no row
  or a stale `running` row (KI-051); the job ID is process-local and is the
  primary key. `files_denied` also counts denied directories.
- **Evidence:** see the session report (fmt, clippy, cargo test, npm build/test).
- **Follow-up:** retention decision, a read port and history view, per-entry
  persistence (rest of B-024) with its own ADR and schema version.

---

## 2026-11-17 — Scan job orchestration, backend only (B-025)

- **Agent/tool:** Cursor agent.
- **Scope:** new `src-tauri/src/app/{mod,jobs}.rs`, `domain/job.rs`,
  `dto/scan.rs`, `commands/scan.rs`; `lib.rs` (managed `Supervisor`, window
  `Destroyed` hook, three handlers); `adapters/filesystem/traversal.rs`
  (`CancelHandle`, removed `Traversal::cancel`); removed the scoped
  `#[allow(dead_code)]` on `domain::traversal` and `adapters::filesystem`; docs.
- **Change:** one in-memory scan slot with a pure state machine, a drain thread
  that counts and drops entries, progress paced by `ProgressGate`, an injected
  `EventSink`, and `start_scan`/`cancel_scan`/`get_scan_status`. Added a new
  layer, `app/`, for orchestration (previously "planned"). Removed unused
  `FileEntry::is_directory`; the per-entry `code()`/`skip_reason()` helpers keep a
  narrowly scoped `#[allow(dead_code)]` until B-024 persists entries.
- **Why:** it is the first step that connects the traversal to IPC, and it
  settles job and event shapes before the schema and UI depend on them.
- **Risk:** untrusted IPC now supplies filesystem roots (KI-049); device and UNC
  roots are not rejected. Scan state is lost on restart (KI-050). The drain
  thread is detached at exit. Job tests poll with wall-clock tolerance.
- **Evidence:** see the session report (fmt, clippy, cargo test, npm build/test).
- **Follow-up:** B-024 (session rows, schema v2), B-004/B-050 (UI and picker),
  B-040 (root policy), B-032 (stress baseline), KI-048.

---

## 2026-11-17 — Traversal outcome taxonomy finished (B-023)

- **Agent/tool:** Cursor agent.
- **Scope:** `src-tauri/src/domain/traversal.rs`,
  `src-tauri/src/adapters/filesystem/traversal.rs`; status docs.
- **Change:** Added `code()` on `EntryOutcome`, `SkipReason`, and a new
  `Completeness` enum; added `TraversalSummary::unread()` and `completeness()`
  (cancelled > truncated > partial > complete). Added a Windows live test that
  denies directory listing with `icacls` and restores it on drop.
- **Why:** B-024 and B-025 need stable codes and an honest coverage verdict
  before schema and IPC shapes are fixed.
- **Risk:** The verdict is derived, so it cannot drift from the counters. The
  `icacls` test mutates an ACL inside its own temp directory only; a Drop guard
  removes the deny entry. No dependency, IPC, UI, or database change.
- **Evidence:** see the commands recorded in the session report.
- **Follow-up:** KI-048 (stream read errors have no entry), KI-046 (CI proof).

---

## 2026-11-16 — Traversal prototype: bounded workers + outcome taxonomy (B-022)

- **Agent/tool:** OpenCode (DeepSeek V4.1 Flash Free).
- **Scope:** `src-tauri/src/domain/traversal.rs`,
  `src-tauri/src/adapters/filesystem/{mod,traversal}.rs`,
  `src-tauri/src/{domain,adapters}/mod.rs`; status/design docs.
- **Change:**
  - Added a third adapter family, `adapters/filesystem`, and its first read-only
    capability: a streaming walker with a direct `std` implementation.
  - Added `domain/traversal` with `EntryKind`, `SkipReason`, `EntryOutcome`,
    `FileEntry`, `TraversalLimits`, and `TraversalSummary`. It performs no I/O and
    imports no platform crate; `std::io::ErrorKind` is the only std surface.
  - The walker uses one work queue protected by a mutex/condvar, a fixed worker
    count, a `SyncSender`/`Receiver` pair with a caller-chosen capacity, and an
    atomic entry budget. Files stream one at a time, and the queue grows only with
    the pending-directory frontier, not with the total file count.
  - Cancellation is checked between entries and communicated to waiting workers.
    An entry budget stops the walk and is reported as `truncated`, never as a
    complete scan.
  - Reparse points are detected from Windows file attributes and never followed;
    roots are classified before opening so a junction root is not traversed
    either. Following them would need a visited set by volume + file ID.
  - No IPC command, event, UI, log line, or database write was added. The module
    carries a scoped `#[allow(dead_code)]` until B-025 wires scan jobs.
- **Why:** B-022 is the highest-risk Release 1 component; validating boundedness,
  cancellation, and reparse behaviour before building scan orchestration keeps
  later failures cheaper.
- **Risk:** Parallel traversal is nondeterministic in entry order; callers must
  not rely on it. Extra `symlink_metadata` calls occur per entry and per directory.
  OneDrive placeholders and other reparse points are skipped rather than read.
- **Evidence:** `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D
  warnings`, `cargo test` (78 passed), frontend gates unchanged (20 passed).
- **Follow-up:** B-024 (batch writer), B-025 (job orchestration + IPC), B-030
  (interrupted-session reconciliation), threat model B-040, live ACL fixture.

---

## 2026-11-16 — Persistence foundation: schema v1 + migration runner (B-020, B-021)

- **Agent/tool:** OpenCode (DeepSeek V4.1 Flash Free).
- **Scope:** `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`,
  `src-tauri/migrations/0001_initial.sql`,
  `src-tauri/src/adapters/persistence/{mod,migrations,sqlite}.rs`,
  `src-tauri/src/adapters/mod.rs`, `src-tauri/src/lib.rs`; `DATABASE.md`,
  `docs/MIGRATIONS.md`, `docs/DECISIONS.md` (ADR-018), dependency/status docs.
- **Change:**
  - Added a third adapter family, `adapters/persistence`, as the only module that
    may open a database. Domain code still imports no SQL or driver type.
  - Added `rusqlite` 0.43 with `bundled` SQLite behind a `Mutex<Connection>`.
    Opening configures WAL/NORMAL/foreign keys/busy timeout, then applies pending
    migrations.
  - Added hand-written forward-only migrations embedded with `include_str!` and a
    `schema_migrations` bookkeeping table (version, name, FNV-1a checksum,
    applied timestamp). A newer schema or a changed released migration refuses to
    open without modifying the file.
  - Schema v1 contains `scan_sessions` and two indexes only. Results, snapshots,
    hashes, findings, quarantine, and audit tables are deferred to the features
    that write them.
  - `lib.rs` opens the database at `app_data_dir()/system-scanner.db`, logs the
    schema version, and manages the handle. Failure is logged; startup continues.
- **Why:** B-020/B-021 block scan history and every later persistence feature.
- **Risk:** A C toolchain is now required to build (bundled SQLite). One
  connection serializes writes. The app creates a database file on first launch
  even though nothing writes rows yet.
- **Evidence:** `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D
  warnings`, `cargo test` (62 passed), `npm test -- --run` (20 passed),
  `npm run build`, `npm run lint`, `npm run format:check`,
  `npm audit --omit=dev` (0 vulnerabilities).
- **Follow-up:** B-024 (batch writer), B-025 (job orchestration), B-030
  (interrupted-session reconciliation), B-033 (backup), retention decision
  (`docs/plan.md` §12).

---

## 2026-11-14 — Fatal-startup dialog + renderer error boundary (B-012)

- **Agent/tool:** OpenCode (DeepSeek V4.1 Flash Free).
- **Scope:** `src-tauri/src/{lib.rs,startup.rs,adapters/windows/dialog.rs}`,
  `Cargo.toml`; `src/main.tsx`, `src/components/ErrorBoundary.tsx`; docs.
- **Change:**
  - Replaced the final `expect()` in `lib.rs` with `startup::report_fatal`: the
    last-resort path builds a message (version, redacted and length-bounded
    detail, next steps), shows a native `MessageBoxW` dialog, and exits 1.
  - Added `startup.rs` (pure message assembly + platform dispatch) and a narrow
    `adapters/windows/dialog.rs`. This is the only UI the Rust core draws itself,
    and only when Tauri has no window.
  - Added `ErrorBoundary` and wrapped the app in `main.tsx`, so a render failure
    shows a reload screen instead of unmounting to a blank window.
  - Enabled the existing `windows-sys` feature `Win32_UI_WindowsAndMessaging`.
    No new crate; `Cargo.lock` is unchanged.
- **Why:** B-012/F-008. A panic message and a silent exit are not actionable, and
  a blank window looks like a hang.
- **Risk:**
  - The dialog is modal and blocking; it is only reachable after the event loop
    failed, so it cannot block normal use.
  - The dialog and the boundary are not exercised by an automated test that opens
    a real window; message formatting is unit-tested, the dialog call is not.
  - The boundary cannot recover the failed subtree without a reload.
- **Evidence:** `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D
  warnings`, `cargo test` (54 passed), `npm test` (20 passed), `npm run build`,
  `npm run lint`, `npm run format:check`, `npm audit --omit=dev` (0).
- **Follow-up:** manually trigger a startup failure and confirm the dialog text
  and layout under WebView2; a crash log with a correlation ID remains open.

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
