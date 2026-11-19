# Decisions (ADR Index)

> Architecture Decision Records. One decision per entry: context, options,
> decision, consequences, revisit trigger.
>
> Status: `Accepted`, `Proposed`, `Superseded`, `Rejected`.
> **Last updated:** 2026-11-19

---

## ADR-001 — Desktop shell: Tauri 2

**Status:** Accepted · 2026-11-06

**Context.** A Windows system-inspection tool needs OS access, a responsive UI, a
small trusted computing base, and a defensible security boundary. The installer
must be small enough for a utility users install without deliberation.

**Options.**
1. Electron + React — mature, but bundles Chromium and exposes Node in the renderer.
2. WPF/WinUI 3 — native and fast, but a narrower UI ecosystem and heavier XAML surface.
3. Tauri 2 + React — OS WebView, Rust backend, capability-based IPC.

**Decision.** Tauri 2.

**Consequences.**
- Installer ~1.4 MB (measured); memory overhead lower than Electron.
- Requires WebView2 and a Rust toolchain.
- Debugging spans TypeScript and Rust.
- The capability model forces an explicit, small permission surface.

**Revisit if:** WebView2 availability becomes a distribution blocker, or a
required OS API cannot be reached from Rust.

---

## ADR-002 — UI: React 19 + TypeScript + Vite, no SSR framework

**Status:** Accepted · 2026-11-06

**Context.** The UI is a local desktop renderer, not a website.

**Options.** Next.js, Remix, plain Vite, Svelte, Vue.

**Decision.** React 19 + TypeScript (strict) + Vite.

**Consequences.**
- No server rendering, routing server, or image pipeline to pay for.
- Vite's dev server integrates with Tauri's `beforeDevCommand`/`frontendDist`.
- React's ecosystem and accessibility tooling are available.
- Bundle size is acceptable for a desktop app.

**Revisit if:** the team's expertise shifts, or a bundler-level need appears.

---

## ADR-003 — Local-first, no account, no telemetry

**Status:** Accepted · 2026-11-06

**Context.** Users of system tools are sensitive about what leaves their machine;
a scanner that uploads paths or process lists is a privacy hazard.

**Decision.** Core functionality is fully offline. No account, no analytics, no
crash upload, no update ping until an updater is explicitly approved. Any future
egress requires per-feature consent, disclosure, and a preview of the payload.

**Consequences.**
- No server-side analytics for product decisions; metrics are local and opt-in.
- Support relies on user-provided, redacted bundles.
- Some features (reputation lookup) are deferred or opt-in only.
- Strong trust differentiator and a simpler threat model.

**Revisit only** with an explicit product decision recorded here.

---

## ADR-004 — Modular monolith, not microservices

**Status:** Accepted · 2026-11-06

**Context.** There is one desktop process serving one user. There is no deployment
topology that would benefit from service boundaries.

**Decision.** One application, one Rust engine, explicit module boundaries
(commands → app → domain ← ports ← adapters).

**Consequences.**
- Refactoring is cheap; no network contracts between modules.
- Boundaries must be enforced by review and dependency rules, since the compiler
  will not stop a careless import.
- Scaling is vertical (concurrency, streaming), not horizontal.

**Revisit if:** a hosted fleet/reputation service is approved — that is a separate
product with its own architecture.

---

## ADR-005 — Persistence: SQLite, access layer undecided

**Status:** Superseded by ADR-018 · 2026-11-06

**Context.** Local history needs transactions, indexes, and zero administration.

**Options.** SQLite via `rusqlite`; SQLite via `sqlx`; a custom file format.

**Lean.** `rusqlite` + hand-written migrations + `spawn_blocking`, WAL mode.
Rationale: one local writer, explicit SQL, smallest dependency surface, no async
complexity in the persistence layer.

**Resolution.** ADR-018 accepts `rusqlite` 0.43 with bundled SQLite, one
`Mutex<Connection>`, and hand-written forward-only migrations. The original lean
held; see ADR-018 for the implementation and consequences.

**Consequences either way.** Repository layer required; migration tests required;
DB must remain backup-able with `VACUUM INTO`.

**Revisit by:** first persistence milestone (Backlog B-020) — resolved.

---

## ADR-006 — Read-only first; destructive features gated

**Status:** Accepted · 2026-11-06

**Context.** Almost every real-world harm from "cleaner" tools comes from
irreversible actions taken on weak evidence.

**Decision.** Ship read-only inspection first. Any destructive or privileged
operation requires, before implementation: a threat model, protected-path policy,
identity verification, explicit confirmation, audit trail, and a recovery path.

**Consequences.**
- Slower feature roadmap.
- Much smaller blast radius and support burden.
- Quarantine is preferred over deletion.

**Revisit:** never for the baseline; individual actions may be approved through a
separate ADR with security evidence.

---

## ADR-007 — Release profile: LTO, strip, panic=abort

**Status:** Accepted · 2026-11-06

**Context.** A utility should be small and predictable; unwinding adds binary size
and can mask programming errors.

**Decision.** `codegen-units = 1`, `lto = true`, `opt-level = 3`, `panic = "abort"`,
`strip = true`.

**Consequences.**
- Smaller binary and installer.
- A panic terminates the process; no `catch_unwind` recovery.
- Interrupted sessions must be recoverable at startup.
- Error handling must be explicit; panics are bugs.

**Revisit if:** a legitimate need for in-process recovery appears.

---

## ADR-008 — Installer: NSIS x64 only

**Status:** Accepted · 2026-11-06

**Context.** Tauri supports NSIS, MSI, and portable targets.

**Decision.** NSIS x64 as the only bundle target for now.

**Consequences.**
- Small per-user installer, no WiX toolchain.
- No enterprise MSI deployment yet.
- No ARM64 or portable build.
- Unsigned today — SmartScreen warnings are expected.

**Revisit when:** enterprise distribution or ARM64 becomes a requirement.

---

## ADR-009 — Documentation structure and status vocabulary

**Status:** Accepted · 2026-11-06

**Context.** The repository had a template README and a plan, with no shared
contract for commands, architecture, security, or AI behavior. Documentation that
describes planned features as real is worse than no documentation.

**Decision.**
- `build.md` is the single source of truth for commands.
- `docs/plan.md` is the authoritative plan; root `plan.md` is a pointer.
- Every document carries a status banner.
- A fixed vocabulary is used: `Implemented`, `Partial`, `Planned`, `Proposed`,
  `Not started`.
- `docs/FEATURES.md` is the ground truth for "does this exist?".

**Consequences.**
- Documentation must be updated with code changes (enforced in review).
- Slightly more maintenance; far less drift and false expectation.

**Revisit if:** the documentation set becomes too heavy for the project's size.

---

## ADR-010 — No shell-outs for core functionality

**Status:** Accepted · 2026-11-06

**Context.** It is tempting to call `wmic`, `powershell`, or `sc.exe` to get system
data quickly.

**Decision.** Use documented Windows APIs through Rust adapters. Shelling out is
prohibited for core functionality.

**Consequences.**
- No dependence on localized output or deprecated tools (`wmic` is deprecated).
- Better performance and structured errors.
- More Rust/FFI work up front.
- A narrow exception (e.g., a documented Defender CLI) requires an ADR and must
  handle missing tools, localization, and quoting safely.

---

## ADR-011 — IPC DTO fields are `camelCase` on the wire

**Status:** Accepted · 2026-11-06

**Context.** `API.md` originally specified `snake_case` DTO fields on the wire with a
camelCase mapping in a TypeScript adapter. Implementing the first command showed
that the mapping layer is pure duplication: every new field has to be added in
Rust, in the adapter, and in a TS type.

**Options.**
1. `snake_case` on the wire, hand-written adapters in `src/services`.
2. `camelCase` on the wire via `#[serde(rename_all = "camelCase")]`, types written
   once in TypeScript.
3. Generate TypeScript types from the Rust DTOs at build time.

**Decision.** Option 2 for DTO fields and error payloads. Commands, events, and
error *codes* stay `snake_case`, matching Windows/JSON conventions and the
stability of the command names.

**Consequences.**
- No mapping layer and no drift between Rust and TS field names.
- Rust field names stay idiomatic `snake_case`; only the serialized form changes.
- A future type generator (option 3) is not blocked; it would emit camelCase too.
- Anyone reading older notes or the original spec must be told the wire format
  changed. `API.md` §1 records this deviation explicitly.

**Revisit if:** IPC consumers outside this UI appear, or a Windows API payload must
be forwarded verbatim.

---

## ADR-012 — Win32 access via `windows-sys` behind a narrow port

**Status:** Accepted · 2026-11-13

**Context.** `get_system_snapshot` (B-002) is the first feature that reads the
machine. Invariant 3 forbids `domain/` from importing Windows APIs, and
`AI_SECURITY_RULES.md` §9 confines `unsafe` to `adapters/windows`.

**Options.**
1. `windows-sys` — raw, Microsoft-maintained bindings, no abstraction layer.
2. `windows` — generated safe-ish wrappers, larger API surface and compile cost.
3. `sysinfo` — cross-platform convenience crate that owns collection decisions.
4. WMI — powerful but slow, string-typed, and heavier to bound.

**Decision.** Option 1 for the first adapter. `domain/system.rs` defines the
platform-neutral model (including `Availability` and reason codes),
`ports/system_info.rs` declares one narrow trait, and
`adapters/windows/system_info.rs` is the only module that imports `windows-sys`.
The dependency is target-scoped (`cfg(windows)`), so the domain and its tests
build on non-Windows hosts. Network drives are deliberately not queried for
label or size because those calls can block on a dead mapping; the fields are
reported as `unsupported` and the snapshot stays bounded.

**Consequences.**
- All `unsafe` and all handle/registry details stay in one auditable file.
- Domain, DTO, and UI tests need no Windows APIs; one structural smoke test runs
  on Windows for the adapter.
- We own the FFI error mapping and the field-availability logic.
- Nine `windows-sys` feature flags are enabled; adding a feature is a dependency
  change and must be recorded in `docs/DEPENDENCIES.md`.
- Collection is intentionally narrower than `sysinfo` would provide: no
  temperature, GPU, or process data, and no background enumeration.

**Revisit if:** a second and third domain need a broad Win32 surface, a maintained
safe wrapper covers the required subset, or WMI becomes the better source for a
specific field.

---

## ADR-013 — Process identity, server-side paging, and field scope

**Status:** Accepted · 2026-11-13

**Context.** `list_processes` (B-003) is the first command with untrusted input.
It must page through a list Windows can change under it, and it must not imply
that a PID is an identity — Windows reuses PIDs.

**Options.**

1. **Identity:** PID alone; a GUID per row; PID + process start time.
2. **Paging:** send the whole table and page in the UI; page in Rust; stream
   events with a job id.
3. **Fields:** everything `PROCESSENTRY32W` and `OpenProcess` can reach (paths,
   command lines, session IDs) versus a narrow, always-honest subset.

**Decision.**

- Identity is **PID + start time**: `key` is `"<pid>-<startedAtMs>"`. When the
  start time cannot be read, the key is `"pid:<pid>"`, which is explicitly
  snapshot-local. A fabricated GUID would look stable and be worse.
- Paging happens **in Rust** (`offset` / `limit`), sorted by PID, with `limit`
  capped at 500 and `offset` at 1,000,000. Rejected input returns
  `invalid_input` before the process table is touched. No `sort` parameter yet:
  an unvalidated sort enum would be input surface with no test.
- Fields are limited to what is cheap and honest: PID, parent PID, name, thread
  count, start time, working set. **Image paths, command lines, CPU usage, and
  session IDs are excluded.** Paths and command lines are a privacy decision;
  CPU usage needs sampling; `ProcessIdToSessionId` is not bound by
  `windows-sys` 0.61, and a hand-written DLL import is not worth it for one
  column.
- A row Windows refuses to open is **kept** with identifiers and a per-field
  `unavailable` reason. The inventory is capped at 8192 processes and reports
  `truncated` instead of pretending the list is complete.

**Consequences.**

- Refreshes keep row keys stable, so the UI can highlight the same process later
  without PID-reuse mistakes.
- Deep offsets still cost one full enumeration; acceptable at desktop scale
  (~300 processes) and bounded by the cap.
- Two Win32 features were added (`Win32_System_Diagnostics_ToolHelp`,
  `Win32_System_ProcessStatus`); further fields need their own dependency change.
- Image paths were left out of this decision as a privacy question; ADR-027
  later added them as local-only display metadata, and the process view can now
  answer "which executable is this?". Command lines, CPU usage, and session IDs
  remain open (KI-040).

**Revisit if:** a Rust-side search/sort is needed (B-042), a support path needs
paths in a bundle (ADR-027 forbids it today), or CPU sampling is designed with a
bounded interval.

---

## ADR-014 — UI styling: Tailwind CSS v4 with local shadcn-style primitives

**Status:** Accepted · 2026-11-14

**Context.** The Release 0 renderer had one hand-written stylesheet
(`src/App.css`, ~10 KB) for a welcome screen; the three data panels reused class
names but had no shared control primitives. The next slices add more panels,
tables, and states. Continuing with ad-hoc CSS would grow a second, undocumented
design system and make responsive behavior inconsistent.

**Options.**

1. Keep plain CSS and add conventions by hand.
2. Adopt a component kit (MUI, Ant Design) with its own theme provider.
3. Adopt Tailwind CSS v4 plus shadcn-style components copied into the repo.
4. Adopt a CSS-in-JS runtime (styled-components, Emotion).

**Decision.**

- **Tailwind CSS v4** through `@tailwindcss/vite`. Tokens live in
  `src/styles/globals.css` using `@theme` / `@theme inline` and CSS variables;
  there is no `tailwind.config.js` palette to keep in sync.
- **shadcn-style primitives** live in `src/components/ui/` and are owned by this
  repository. `Button` uses `class-variance-authority` for variants,
  `tailwind-merge` + `clsx` through `cn()` so caller overrides win, Radix `Slot`
  for `asChild`, and `lucide-react` for decorative icons.
- **Responsive contract:** the shell grid is one column, then two at `sm`, then
  three at `lg`; the process table spans the full width because it needs it.
- Light and dark token sets are selected by `[data-theme="dark"]`; a small
  `ThemeToggle` writes the attribute and persists the choice in `localStorage`.
  Following the OS preference is not implemented (F-009 stays partial).

**Consequences.**

- One token layer and one button primitive replace per-panel CSS classes.
- Utility classes appear in JSX; reviewers must check class strings, and invalid
  utilities fail silently at runtime (they simply emit no CSS).
- Seven direct dependencies were added (two dev, five runtime); they are recorded
  in `docs/DEPENDENCIES.md` and are small, maintained, permissively licensed.
- No styling runtime is shipped; the bundle carries static CSS.

**Revisit if:** the panel count grows enough that copied primitives duplicate a
real component library, a design-token pipeline (Figma → CSS) is adopted, or the
theme toggle (F-009) needs a provider that changes the current structure.

---

## ADR-015 — Local logs: `tracing` with size-based rotation, no telemetry

**Status:** Accepted · 2026-11-14

**Context.** B-007 requires diagnosable failures without telemetry or a network
path. Tauri already pulls `tracing` transitively; the app had no subscriber, so
command outcomes and startup state were invisible in the field.

**Options.**

1. Keep `eprintln!` / ad-hoc file writes.
2. Add `tracing-appender` for rotation.
3. Add `tracing-subscriber` and own a small rotating writer.
4. Ship logs to a remote collector (rejected: violates N-001/N-002).

**Decision.**

- Use **`tracing` + `tracing-subscriber`** (default features off; `fmt`, `std`).
- Own rotation in `logging.rs`: **10 MB per file, five files, 50 MB total**.
  Rotation shifts numbered archives and removes the oldest; the active file is
  `system-scanner.log`. Tests use small limits via `with_limits`.
- Log **command**, **outcome**, stable error **code**, and **correlation ID**.
  Payloads, IPC inputs, file contents, and command lines are never logged.
  Error `detail` is diagnostic-only, logged at info, and passes `redact_detail` (best effort:
  `C:\Users\<name>\…` becomes `%USERPROFILE%\…`, other drive-rooted tokens
  become `<path>`).
- Logging is **initialized in `setup`** against Tauri's `app_log_dir()`; failure
  is non-fatal and reported on stderr. No upload, no telemetry, no new Tauri
  capability.
- `tracing-appender` is unnecessary: its rotation is time-based, and the
  requirement is size-bounded retention.

**Consequences.**

- Field failures now leave a bounded local trace, including the correlation ID
  shown to the user.
- The redaction helper is heuristic; any new log call site still needs review.
- Four small transitive crates are added; the process-wide subscriber means
  tests must use scoped subscribers.
- B-006 (diagnostics view/support bundle) can now read a real log source.

**Revisit if:** structured JSON logs are needed for a support tool, the file cap
proves too small or too large on real machines, or a second process needs to
write concurrently (single-instance assumption).

---

## ADR-016 — Diagnostics bundle: local preview/copy, best-effort redaction

**Status:** Accepted · 2026-11-14

**Context.** B-006 requires a diagnostics view and a support bundle that excludes
paths and PII, without telemetry or a network path. A saved-file export needs a
file-picker flow (a new capability or dialog plugin), and a zip archive would add
a dependency. The app also cannot yet rely on the user finding
`%LOCALAPPDATA%\...\logs`.

**Options.**

1. Return the raw log file and let the user share it.
2. Build a text preview in Rust, redact it, and let the user copy it.
3. Write a `.zip` to a fixed app directory and reveal it in Explorer.
4. Add a dialog/shell plugin and a zip crate for a full export flow.

**Decision.**

- `get_diagnostics` returns environment facts plus a **plain-text preview** built
  in Rust. The log tail is capped at **256 KiB** and starts on a whole line.
- Redaction runs in `src-tauri/src/redact.rs` on the finished text: profile and
  other absolute paths, UNC paths, e-mail addresses, IPv4/IPv6, MAC addresses,
  SIDs, and literal account/machine names. The Windows adapter supplies the two
  names from `GetUserNameW`/`GetComputerNameW`; token elevation is read with the
  existing `windows-sys` dependency (`Win32_Security` feature).
- The preview is **generated only on an explicit action** and copied only on a
  second action. The command writes no file and returns no absolute path.
- The panel is a fourth section in the existing dashboard. No router, dialog
  plugin, shell plugin, or new capability is added.
- No new crate: the UTC formatter is in-process and redaction is hand-written and
  tested.

**Consequences.**

- Support can receive a complete-enough bundle by copy/paste today; a saved file
  remains F-092/B-064.
- Redaction is heuristic (KI-043). Every new field that reaches the bundle needs
  a rule or a test, and the preview remains reviewable before it is shared.
- The elevation probe is read-only. It does not request elevation and does not
  change the "read-only, unelevated" default.

**Revisit if:** a saved-file flow becomes a release requirement, logs move to
structured JSON, or redaction needs to cover a new class of identifier.

---

## ADR-017 — Fatal startup and render failures: native dialog + React boundary

**Status:** Accepted · 2026-11-14

**Context.** `lib.rs` ended with `.expect("error while running tauri application")`.
In a release build (`panic = "abort"`, `windows_subsystem = "windows"`) that means a
failing startup exits without any window, console, or message. A render exception
has the same effect inside WebView2: React unmounts the tree and the user sees a
blank window. B-012 requires an actionable failure, not a silent exit.

**Options.**

1. Keep `expect()` and rely on the log file.
2. Add a Tauri dialog plugin and show the error from the setup hook.
3. Draw a native `MessageBoxW` when the event loop fails, and catch render errors
   in a React error boundary.
4. Try to create a second window purely to display the failure.

**Decision.**

- The startup path calls `startup::report_fatal`, which builds a message (app
  version, redacted and 400-char-bounded detail, three next steps), shows a
  `MessageBoxW` through the existing `windows-sys` dependency, and exits 1.
- The detail is passed through `redact::redact_text` before display, so a startup
  error cannot show a full path even locally.
- The renderer wraps `<App />` in `ErrorBoundary`, which shows a reload screen
  with the error message and the log file name. It performs no IPC and no
  clipboard access.
- No new crate, plugin, or capability. No second window and no dialog-on-dialog
  path.

**Consequences.**

- Option 4 was rejected because window creation is exactly what can fail.
- The message text is unit-tested; the modal dialog itself is verified manually.
- The boundary cannot restore a broken subtree without a reload, which is
  acceptable for a bug this severe.

**Revisit if:** startup gains a recoverable "safe mode", or the app needs the same
screen on a non-Windows target.

---

## ADR-018 — Persistence: `rusqlite` with bundled SQLite and hand-written migrations

**Status:** Accepted · 2026-11-16 · *(backup bullet superseded by ADR-023,
2026-11-17)*

**Context.** Release 1 needs scan sessions, results, and history. `DATABASE.md`
proposed SQLite but no driver, schema, or migration mechanism existed
(`DATABASE.md` §1, `docs/MIGRATIONS.md` §1). The app is a single-user, local,
non-elevated desktop tool with one writer and small data volumes; there is no
server, no connection pool requirement, and no async runtime beyond Tauri's.

**Options.**

1. `rusqlite` with the `bundled` feature — synchronous, compiled-in SQLite.
2. `sqlx` — async, compile-time-checked queries, built-in migrations.
3. A JSON file per domain — no SQL, no indexes, rewrite-on-write.
4. An embedded key-value store (`sled`) — no SQL aggregation for scan results.

**Decision.**

- **`rusqlite` 0.43** with `bundled` SQLite, behind a `Mutex<Connection>` in
  `adapters/persistence/sqlite.rs`. Bundled removes a system SQLite dependency
  and keeps version behavior identical on every machine.
- **Blocking only.** Commands that touch the database must use
  `tauri::async_runtime::spawn_blocking`; the UI thread never waits on SQLite.
- **Pragmas:** `journal_mode=WAL`, `synchronous=NORMAL`, `foreign_keys=ON`,
  `busy_timeout=5000`, matching `DATABASE.md` §4.
- **Migrations are hand-written, forward-only SQL** in `src-tauri/migrations`,
  embedded with `include_str!`. Applied versions and FNV-1a checksums are stored
  in `schema_migrations`. A changed checksum on a released migration, or a
  database newer than the build, refuses to open instead of guessing.
- **One transaction per migration**, so a failure leaves the file at the prior
  version. No automatic pre-migration backup yet: v1 is additive, and the backup
  requirement returns when a migration can lose data (`docs/MIGRATIONS.md` §4).
- **No downgrade support.** An older build refuses a newer schema.
- **Database file:** `app_data_dir()/system-scanner.db`
  (`%APPDATA%\com.systemscanner.desktop\system-scanner.db`). Startup opens it,
  applies migrations, and stores the handle in Tauri state. Failure is logged and
  the app continues read-only; persistence never blocks the current panels.
- **Schema v1 is deliberately narrow:** `scan_sessions` only. File records,
  snapshots, and hashes land with the features that write them, because data
  retention is still open (`docs/plan.md` §12).

**Consequences.**

- `rusqlite`, `libsqlite3-sys`, `cc`, and five small support crates join the
  dependency tree; `Cargo.lock` grows from 412 to 420 packages. Building now
  requires a C toolchain (the MSVC toolchain already required for Windows).
- One connection serializes writes. That is correct for one local user; a second
  writer would need a second connection or a queue.
- The startup path creates an empty schema before any feature writes rows. This
  is observable on disk and must be documented in the data inventory.
- FNV-1a detects accidental edits, not forgery. It is not a security control.
- `sqlx` remains attractive if compile-time query checking becomes valuable, but
  its macros/offline metadata add ceremony for a small local schema.

**Revisit if:** concurrent writers are required, aggregate queries outgrow
hand-written SQL, or the bundled SQLite build becomes a distribution problem.

---

## ADR-019 — Directory traversal: custom `std` walker, no third-party crate

**Status:** Accepted · 2026-11-16

**Context.** B-022 required a traversal prototype before scan orchestration, and
`docs/DEPENDENCIES.md` §6 had a placeholder: `walkdir` *or* a custom walker,
decided after the prototype. Constraints that matter more than convenience:

- cancellation must be observable between entries, not only between directories;
- memory must stay flat when a tree has millions of entries, so entries must
  stream with backpressure rather than be collected;
- reparse points must be detectable and never followed, including junctions and
  cloud placeholders;
- the adapter must stay auditable and compile without extra licenses.

**Options.**

1. `walkdir` — mature, handles depth and symlink policy, yields an iterator.
2. `jwalk` — parallel walk with rayon.
3. Custom walker on `std::fs` with a work queue and bounded channel.

**Decision.** Custom implementation, **no new dependency**.

- `domain/traversal` owns the vocabulary: `EntryKind`, `SkipReason`,
  `EntryOutcome`, `FileEntry`, `TraversalLimits`, `TraversalSummary`, plus the
  pure `classify_io_error` / `classify_kind` functions. It performs no I/O.
- `adapters/filesystem/traversal` owns enumeration: one `VecDeque` work queue
  behind a `Mutex`/`Condvar`, a fixed worker count, and a `SyncSender<FileEntry>`
  with a caller-chosen capacity. A slow consumer blocks producers.
- Cancellation is an `AtomicBool` checked between entries, with the condition
  variable notified so idle workers exit. `join()` drops the receiver, which
  releases any worker blocked in `send`.
- Depth is bounded (default 64, clamped to 4096). An optional entry budget stops
  the walk and sets `truncated`; it is never reported as a complete scan.
- Reparse points are detected from `FILE_ATTRIBUTE_REPARSE_POINT` (Windows) or
  `file_type().is_symlink()` elsewhere and are **reported and skipped**, never
  followed. Roots are classified before they are opened.
- No IPC command, UI, or database write consumes the prototype yet.

**Consequences.**

- `walkdir` is removed from the planned-dependency list. The incremental cost is
  that depth/entry policy, cancellation, and reparse handling are our tests, not
  a library's (see `TESTING.md` §1).
- Traversal order is nondeterministic because workers race on the queue; callers
  must not depend on order. Tests assert counts and membership, not sequence.
- Because reparse points are never followed, a junction or OneDrive placeholder
  is counted as skipped, and content behind it is not scanned. Following would
  require a visited set keyed by volume + file ID to stay cycle-safe; deferred.
- Denied directories are counted and the walk continues. The outcome mapping is
  unit-tested; a live ACL fixture is still a gap.
- Threads are unbounded per traversal by request field but capped at
  `MAX_WORKERS` (16), roots at 64, and channel capacity at 65,536.

**Revisit if:** we need ordered traversal for reproducible exports, we decide to
follow reparse points (visited-set design required), or a benchmark on the
100k/1M fixtures (`B-032`) shows the worker pool is the bottleneck.

---

## ADR-020 — Scan job supervisor: one in-memory slot, drain thread, paced events

**Status:** Accepted · 2026-11-17

**Context.** B-025 had to connect the traversal to IPC without persistence
(B-024) or a UI. Constraints: every long operation is bounded and cancellable,
IPC input is untrusted, events must not flood the renderer, and no path may
reach the UI.

**Decision.** No new dependency.

- `app/jobs.rs` owns a `Mutex<Option<JobRecord>>`: **one scan at a time**; a second
  `start_scan` returns `busy` while the job is queued, running, or cancelling.
  The previous terminal job is kept until replaced so `get_scan_status` stays
  authoritative after the events are gone.
- `domain/job.rs` is the pure state machine (`queued -> running ->
  cancelling -> completed | failed | cancelled`, terminal states immutable).
  `cancelling -> completed` is legal: a walk that finished before it saw the
  cancel is reported as completed, not cancelled.
- One named **drain thread** per job consumes the entry channel, counts entries,
  and drops them, so memory is independent of tree size. Budgets (4 workers, a
  1,024-entry channel, `maxEntries` ceiling of 100,000,000) are chosen in Rust.
- Progress is paced by `ProgressGate` (default 100 ms, at most 10 events/s); the
  first entry is always due and terminal events are always sent. `seq` counts
  progress events per job from 1.
- Events go through an injected `EventSink` closure; the command layer supplies
  `AppHandle::emit`. The supervisor therefore has no Tauri types and is tested
  with a plain collector.
- Cancellation uses a cloneable `CancelHandle` onto the traversal queue.
  `WindowEvent::Destroyed` calls `Supervisor::shutdown` (reason `shutdown`).
- Responses and events carry **aggregate counters only**. Validation errors use
  fixed messages and never echo a path.

**Consequences.**

- Live scan state is lost on restart and the next scan replaces the last result;
  the final counters are persisted separately by ADR-021.
- Roots are caller-supplied and shape-checked, with device and NT-namespace roots
  refused; UNC shares and other absolute paths are accepted (KI-049, B-040).
- Starting a scan stats each root while the slot lock is held; a dead network
  mapping can delay `get_scan_status` for that moment. `start_scan` runs the
  start on a blocking thread so the async runtime is not stalled.
- The drain thread is detached. On process exit it is not joined; shutdown relies
  on cancellation plus process teardown, not a graceful flush (nothing is
  persisted yet).
- A failed `read_dir` stream item is still counted without an entry (KI-048).

**Revisit if:** concurrent scans are required, B-024 needs session rows tied to
job IDs, or event delivery to a slow renderer must be acknowledged rather than
paced.

---

## ADR-021 — Scan sessions: counters-only rows through a port, best-effort

**Status:** Accepted · 2026-11-17

**Context.** B-024 asked for persistence of scan results. Storing file paths and
sizes needs the retention decision still open in `docs/plan.md` §12, and
`AGENTS.md` §8 forbids guessing on data retention. A scan with no recorded
outcome, however, cannot be told apart from one that crashed.

**Decision.** Persist the **session only**, never entries.

- `ports/sessions.rs` defines `ScanSessionStore` (`begin`, `finish`,
  `mark_interrupted`) and plain records. `Database` implements it in
  `adapters/persistence/sessions.rs`; `app/jobs.rs` depends on the trait, not on
  SQLite.
- Migration `0002_scan_session_results.sql` adds the completeness verdict and the
  counters schema v1 lacked. Additive; v1 rows are preserved.
- Stored: ID, module, times, status, completeness, aggregate counters, the
  budgets as JSON, engine/policy version, cancel reason, error code. **Not
  stored:** roots, paths, names. `roots_json` stays `'[]'`.
- `begin` runs before the drain thread starts and `finish` runs before the
  terminal event, so a listener never sees an event ahead of its row. An ended
  row is never overwritten (`WHERE status IN ('queued','running','cancelling')`).
- Writes are **best-effort**: failure is logged by the adapter and never changes
  the job outcome or refuses a scan.
- At startup `mark_interrupted` sets rows left open by a previous run to
  `interrupted` and logs the count. The supervisor is built after the database
  opens, with or without a store.

**Consequences.**

- Rows accumulate with no limit and no delete control (KI-050); a scan can have no
  row or a stale `running` row until the next startup (KI-051).
- No command reads sessions yet, so history is invisible to the UI.
- Job IDs are process-local (`scan-<millis>-<seq>`) and used as the primary key; a
  collision is refused rather than overwritten.
- Per-entry persistence (the rest of B-024) stays blocked on retention and needs
  its own ADR and schema version.

**Revisit if:** retention is decided, a history view needs a read port, or
sessions must be tied to a persisted job registry across restarts.

---

## ADR-022 — IPC response validation: hand-written guards, no schema library

**Status:** Accepted · 2026-11-17

**Context.** `API.md` §8 promised runtime validation and KI-011 recorded that a
DTO rename would render `undefined` instead of failing. The renderer already
declares a TypeScript type per DTO, but types vanish at runtime. The candidates
were a schema library (`zod` and similar), generating TypeScript from the Rust
DTOs, or hand-written guards.

**Decision.** **Hand-written guards**, one per command or event, in
`src/services/validate.ts` plus a `parse*` function beside each mirrored type.

- `invokeCommand(command, parse, args?)` requires the guard, so a new command
  cannot silently skip validation; the compiler enforces it.
- Guards are strict but not normalizing: a missing optional field is
  `undefined`; a present field of the wrong type is rejected rather than
  coerced or defaulted. Known fields are copied, unknown fields dropped.
- `schemaVersion` is checked against the constant this build reads.
- Failures throw `IpcContractError` (`source`, `problem`), which
  `src/lib/errors.ts` maps to `contract_violation`. Messages name paths, never
  payload values.
- Event payloads are validated in `subscribeToScan`; a malformed event is
  reported to the panel and dropped, then the panel re-reads `get_scan_status`.
- Collection caps (page items, disks, adapters, unavailable entries, preview
  characters) bound the memory a bad payload can force.

**Consequences.**

- No new dependency; the footprint stays auditable, per `AGENTS.md` §5.
- The guards are a second declaration of each DTO and must be updated with it.
  There is no generated schema and no compile-time link to the Rust structs; a
  test per command plus one drifted case each is the safety net.
- An additive Rust field is invisible to the UI until a guard copies it. That is
  deliberate (no silent pass-through) but means a feature is not done until both
  sides change.
- `IpcContractError` is not an `IpcError`: the backend succeeded, but this build
  cannot read the answer. Panels show `contract_violation` with the field path.

**Revisit if:** the DTO count grows enough that generated types (for example from
`schemars` or a checked-in JSON Schema) beat hand-maintained guards, or a second
consumer of the IPC contract appears.

---

## ADR-023 — Snapshot before migration: automatic, verified, best-effort, five kept

**Status:** Accepted · 2026-11-17

**Context.** `docs/BACKUP_POLICY.md` §2.1 requires `VACUUM INTO` plus a read-only
`PRAGMA quick_check`, and KI-014 recorded that no pre-migration backup existed.
The runner refuses a newer schema and rolls back a failed migration, but a
migration that succeeds and is still wrong has no way back. Every released
migration (v1, v2) is purely additive, so today the cost of loss is zero.

**Decision.**

- `Database::open` snapshots only when `schema_migrations` exists and its maximum
  version is below `SCHEMA_VERSION`. A fresh file (nothing to protect) and an
  up-to-date file (nothing pending) skip the snapshot.
- One `VACUUM INTO` into `backups/` beside the database, named
  `system-scanner-v{version:03}-{epoch_ms}.db`, so name order is age order. Never
  a file copy and never a sidecar copy.
- The snapshot is reopened read-only and must return `ok` from
  `PRAGMA quick_check`. A snapshot that fails is deleted; it is never left as a
  restore candidate.
- Retention is the newest five snapshots by name; unrelated files are ignored.
  The policy's 30-day half is not implemented.
- The step is **best-effort**: a failure logs `tracing::warn!` and the migration
  proceeds. Blocking startup on a backup is not justified while every migration
  is additive.
- `from_connection` (in-memory and tests) does not snapshot; this is a file
  concern, not a connection concern.

**Consequences.**

- Startup can take one extra database copy while the single connection is held.
  Acceptable for a small local database; revisit if the file reaches hundreds of
  megabytes (B-032).
- A silently wrong migration is now recoverable by copying one file back, but
  only by hand: there is no restore command or UI (KI-014).
- The guarantee is only as good as the additive property. The first non-additive
  migration **must** make the snapshot mandatory and refuse to migrate on
  failure; recorded in `docs/MIGRATIONS.md` §11 and `docs/BACKUP_POLICY.md` §3.
- Restoring an older snapshot re-applies migrations on the next open; the test
  performs exactly that and checks a row survives.

**Revisit if:** the database grows enough that the copy meaningfully delays
startup, a daily/rolling backup is added (retention and scheduling then need one
owner), or a non-additive migration is scheduled.

## ADR-025 — Directory open, identity, and enumeration bind to one handle

**Status:** Accepted · 2026-11-19

**Context.** Threat-model finding F-01 (KI-055) recorded that the walker checked
a directory with `symlink_metadata` and then opened it again with `fs::read_dir`
**by path**. Both calls name a path, so a directory swapped for a junction between
them was followed, and the reparse check did not bind the object enumerated. The
threat model made closing this a precondition for any content read or write.

**Options.**

1. Keep the path-based pair. Smallest change, leaves the race open.
2. Open a handle, verify it, then still enumerate by path. Narrows the window but
   does not close it: the enumeration opens a different object.
3. Open once with `FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS`,
   read identity from that handle, and enumerate through that handle.

**Decision.** Option 3, in `adapters/filesystem/directory.rs`.

- Identity is volume serial + 128-bit file id from `FileIdInfo`, because the
  enumeration record (`FileIdExtdDirectoryInfo`) carries the same 128-bit id, so
  parent-recorded and opened values compare exactly.
- Enumeration through the handle also yields each child's attributes, size,
  timestamps, and id, so the previous per-child `symlink_metadata` call is gone.
- Batches are fixed at 16 KiB of entry records, keeping memory flat.
- Paths are prefixed with `\\?\` (or `\\?\UNC\`) so paths beyond `MAX_PATH` open
  without requiring `LongPathsEnabled`.
- A mismatch is reported as `disappeared`; an unavailable id is unverifiable and
  is never treated as a match (KI-056).

**Consequences.** The filesystem adapter is no longer std-only on Windows; it uses
`windows-sys` (`Win32_Storage_FileSystem`), which was already a Windows-only
dependency with that feature enabled, so no dependency or feature was added.
`unsafe` is confined to that module with `SAFETY:` comments. Cost is two property
reads per directory plus one per batch; children no longer cost a stat each. The
non-Windows fallback records no identity and remains unverifiable.

**Revisit trigger.** Any content read or write (the write path must reuse this
handle, not copy the old pattern), or a need for deterministic entry order.

## ADR-026 — Protected paths are a pure, lexical, deny-by-default domain policy

**Status:** Accepted · 2026-11-19

**Context.** Threat T-02 needs an answer to "may this location be modified?"
before the first write feature, and B-041 pairs it with the handle identity check
(ADR-025). Three forces shape the answer: the domain must compile and test without
Windows (invariant 3), a path is attacker-influenced input (invariant 4), and a
refusal must be explainable rather than a silent skip. Canonicalization — 8.3
short names, reparse resolution, `GetFinalPathNameByHandleW` — needs the OS, so it
cannot live in the domain.

**Options.**

1. Put the policy in a Windows adapter and canonicalize every path there. Most
   accurate, but the rule set becomes untestable off Windows and the domain loses
   its policy.
2. Put the rule in the domain but read environment variables for the roots.
   Simple, but takes ambient state, so a test cannot describe the machine it means.
3. Put the rule in the domain over *caller-supplied* roots, resolve paths
   lexically, and let the adapter supply canonical directories.

**Decision.** Option 3, in `domain/policy.rs`.

- `default_roots(windows_dir, program_files, program_files_x86, program_data,
  local_app_data, app_install, app_data)` takes the machine directories as
  arguments; no environment read, no `std::fs`.
- Comparison is upper-cased, component-aware, and boundary-checked, so
  `C:\Windows10` is not covered by `C:\Windows`. Trailing dots and spaces are
  trimmed, `.`/`..` resolve lexically, `\\?\C:\...` equals `C:\...`.
- Roots are sorted most-specific-first, so a `System32` path reports `system32`
  rather than `system_root`; every denial carries a stable code.
- A path the module cannot resolve to a drive-rooted sequence — relative,
  drive-less, device (`\\.\`), verbatim global, UNC — is **denied** as
  `unresolvable`. A share can point into a protected tree, so it is not ordinary
  user space. A root that cannot be resolved is dropped rather than allowed to
  match everything.
- A path outside every root is `Allowed`; the policy is a ceiling on authority,
  not a grant.

**Consequences.** The policy is honest about being lexical: 8.3 names, reparse
resolution, non-ASCII case folding, ADS, and hard links are not modelled, so it
must be paired with the handle identity check rather than replace it (ADS and hard
links inherit their containing path, which is the safe direction). It has **no
caller** until the first state-changing command exists; it is unreachable while
nothing changes state, and the read-only walker deliberately does not consult it,
because scanning `C:\Windows` for counters is a legitimate use (KI-057). The list
is compiled in, not configurable, which keeps it auditable; making it updatable is
an open question (`docs/ACCESS_CONTROL.md` §9 item 4).

**Revisit trigger.** The first write path (must call `evaluate` and report the
denial before acting), a requirement to configure the list per machine, or a
decision to follow reparse points — lexical resolution is then no longer enough on
its own.

---

## ADR-027 — Process image paths are collected locally, never exported

**Status:** Accepted · 2026-11-19

**Context.** B-042 covers "process hierarchy + access-limited metadata", and F-040
lists the image path as the one missing read-only field. `API.md` §3.3 previously
called image paths "a privacy decision" and omitted them, which left the process
panel unable to answer the first question a user asks about an unfamiliar process:
where is it running from? The constraint is the product's privacy posture (§8 of
`SECURITY.md`): no telemetry, no upload, no bundle containing paths.

**Options.**

1. Keep paths out. Cheapest, but the panel stays unable to distinguish
   `C:\Windows\System32\svchost.exe` from a copy in a user-writable directory.
2. Collect them and include them in the diagnostics bundle when the user copies
   it. More useful for support, but silently widens what leaves the app and
   contradicts the redacted-by-default promise.
3. Collect them for local display only, with no logging and no bundle inclusion.

**Decision.** Option 3.

- `adapters/windows/processes.rs` reads the path with
  `QueryFullProcessImageNameW` on a `PROCESS_QUERY_LIMITED_INFORMATION` handle —
  no `PROCESS_VM_READ`, no elevation, no new capability, no new dependency.
- The field is `imagePath`, optional on the wire, always paired with a per-row
  `unavailable` entry when it could not be read, so a missing path is never
  ambiguous.
- The value is never written to the log (`ipc.rs` logs outcomes, not payloads),
  never placed in the support bundle (`redact.rs` keeps paths out), and never
  sent over the network because no network feature exists and ADR-level policy
  requires consent.
- A path that would fill the 32,768-unit buffer is reported as `failed` instead
  of a truncated string; a wrong path is worse than an honest gap.
- `PROCESS_LIST_SCHEMA_VERSION` becomes `2`, because `depth` is a new required
  field.

**Consequences.** The renderer now holds machine-local absolute paths in memory.
That is the same trust level as the scan-root text already typed by the user, and
the CSP/capability model is unchanged. `docs/FEATURES.md` (F-040/F-042) and
`docs/DATA_PRIVACY.md` record the field and its disposal rule. Command lines stay
out: they routinely contain credentials and URLs, which the image path does not.

**Revisit trigger.** A support bundle that must include executable locations, a
process-detail view that adds command lines, or any feature that would send a path
off the device.

---

## ADR-028 — Startup entries and services are read-only observations with per-source gaps

**Status:** Accepted · 2026-11-19

**Context.** B-043 asks for a "startup/service inventory with limitation
explanations", covering F-044 (startup entries), F-045 (read-only service
inventory), and F-046 (explain the source and its limits). Both answers exist in
Windows, but neither is uniformly readable by a non-elevated user: the SCM can
enumerate services while refusing each service's own configuration, and a
registry key or Startup folder can be denied outright. The product rule is that
unknown is a valid result and nothing may be fabricated (`AI_CONTEXT.md` §6).

**Options.**

1. Shell out to `sc.exe`, `reg.exe`, or `services.msc`. Rejected: ADR-010 bans
   shell-outs, and parsing localized console output is not a contract.
2. Use WMI (`Win32_Service`, `Win32_StartupCommand`) through COM. Rejected for
   this slice: it needs a new dependency or a large hand-written COM surface and
   a slow first call, for data the SCM and registry already provide.
3. Read the narrow Win32 surfaces directly: `EnumServicesStatusExW` for the
   inventory, `QueryServiceConfigW` per service for start type and binary path,
   the four `Run`/`RunOnce` keys, and the two Startup folders.

**Decision.** Option 3, with these rules:

- Read-only. Services are opened with `SERVICE_QUERY_CONFIG` only; no start, stop,
  disable, or terminate exists (those wait for B-049's confirmation/audit work).
- No elevation is requested. Anything denied becomes an `unavailable` reason:
  per source group (`startup[i].entries`) and per service configuration field.
- A fixed set of six sources is always reported, including empty and denied ones,
  so "nothing found" and "could not look" never look the same.
- Deterministic ordering is applied in `domain/startup.rs`, not in the adapter.
- Caps: 4,096 services, 512 startup entries, reported through
  `servicesTruncated` / `startupTruncated`.
- Collected strings (service binary paths, registry value data, folder item
  paths) follow ADR-027: local display only, never logged, never bundled, never
  stored.
- Only `SERVICE_WIN32` services are enumerated; drivers are out of scope for this
  slice.

**Consequences.** The inventory cannot see the 32-bit registry view
(`WOW6432Node`), scheduled tasks, or the target of a Startup shortcut; `KI-058`
records these. A service whose configuration is denied still appears with its
state, which is honest but means the panel can show rows with no path. No new
crate, capability, or elevated code path was added; `windows-sys` gained the
`Win32_System_Services` feature.

**Revisit trigger.** A scheduled-task view, a driver view, an action on a service
or startup entry, or a request to include any of these strings in an export.

---

## Template

```text
## ADR-NNN — Title
Status: Proposed|Accepted|Superseded by ADR-XXX
Date:
Context:
Options:
Decision:
Consequences:
Revisit trigger:
```
