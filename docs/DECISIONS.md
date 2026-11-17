# Decisions (ADR Index)

> Architecture Decision Records. One decision per entry: context, options,
> decision, consequences, revisit trigger.
>
> Status: `Accepted`, `Proposed`, `Superseded`, `Rejected`.
> **Last updated:** 2026-11-06

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
- Without paths, the process view cannot answer "which executable is this?".
  That is a known gap (KI-040), not an accident.

**Revisit if:** a Rust-side search/sort is needed (B-042), image paths become
necessary for evidence in a later release, or CPU sampling is designed with a
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

**Status:** Accepted · 2026-11-16

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
