# Testing Strategy

> **Status:** **Runners are configured; coverage is still narrow.** 125 Rust tests
> and 76 frontend tests cover the IPC contract (envelope, error taxonomy, DTO
> shapes, runtime payload guards), the snapshot and process domain rules, both
> native adapters' structural invariants, logging/redaction, startup messages,
> schema creation, v1 to v2 migration and runner refusal paths, pre-migration
> snapshot/verify/restore and retention, the traversal (outcomes, budgets,
> cancellation, long/Unicode paths, junction loops, a live denied-ACL fixture),
> the scan job supervisor and session rows, root validation, six UI panels
> (including the scan panel, its event/status reducer, and its service wrapper),
> and the render error boundary. **Every UI and IPC test uses mocked IPC.** No test
> launches the Tauri app, so event delivery, capabilities, and real cancellation
> are unverified end to end (KI-052). This document defines the target strategy
> and the minimum bar for new work.
> **Last updated:** 2026-11-17

---

## 1. Current state

| Level | Tool | Configured | Tests |
|---|---|---|---|
| Rust unit/integration | `cargo test` | Yes | 125 (`ipc`, `error`, `logging`, `redact`, `startup`, `dto::*`, `domain::*`, `adapters::windows::*`, `adapters::persistence::*`, `adapters::filesystem::*`, `commands::*`, `app::*`) |
| Frontend unit | Vitest 5 + jsdom | Yes | 37 (`services/{ipc,validate,contracts,scan}.test.ts`, `lib/scanView.test.ts`) |
| Frontend component | Testing Library 16 | Yes | 26 (`BackendStatus`, `ThemeToggle`, `SystemOverview`, `ProcessList`, `Diagnostics`, `ScanPanel`, `ErrorBoundary`) |
| E2E | tauri-driver / WebDriver (proposed) | No | 0 |
| Performance | custom harness (proposed) | No | 0 |
| Security | manual review + targeted tests | No | 0 |

What the existing tests actually establish, and nothing more:

- the `CommandResult` envelope serializes as `{status:"ok"|"err", ...}`;
- error codes serialize as `snake_case` and `detail` is omitted when `None`;
- correlation IDs are unique per error;
- `AppInfo` serializes with camelCase keys and `schemaVersion`;
- the frontend adapter unwraps `ok` and throws `IpcError` on `err`;
- every response guard accepts the documented shape, rejects a renamed, retyped,
  or out-of-range field with the field path, rejects an unknown envelope status
  and an undocumented error code, enforces `schemaVersion`, and drops unknown
  fields;
- a malformed `scan:*` payload reaches `onInvalid` instead of the handler, and
  the scan panel shows `contract_violation`, then re-reads the status;
- the status component renders loading, ready, error, and recovers on retry;
- the system panel renders each collected group, replaces a missing group with its
  `unavailable` reason, and recovers after a failed refresh;
- the process panel renders rows and the paged range, requests the next offset,
  keeps denied fields visible with their reason, recovers on refresh, and states
  when the list hit its safety cap;
- the theme toggle writes `data-theme`, persists the choice, and restores it;
- the log writer enforces its file cap, rotates without losing the active file,
  and limits a tail to a whole-line boundary;
- the redactor replaces user-profile and other absolute paths, UNC paths,
  e-mail/IP/MAC/SID shapes, and literal account and machine names;
- the diagnostics panel waits for an explicit request, renders versions and
  environment facts, shows the redacted preview, reports errors, retries, and
  confirms a copy;
- the fatal-startup message names the version, includes redacted and bounded
  detail, and lists next steps;
- the error boundary renders children normally and replaces a crashed tree with
  a reload screen;
- opening a fresh database creates `scan_sessions` at the latest schema version
  with foreign keys enabled, a file database uses WAL and creates its parent
  directory, reopening applies nothing new, a newer schema is refused, and an
  edited released migration is detected by checksum;
- a database with a pending migration is snapshotted first, the snapshot passes
  `PRAGMA quick_check`, contains the pre-migration schema and rows, and can be
  reopened to reach the latest schema with its rows intact; an up-to-date
  database is not snapshotted, an unwritable backup directory does not block the
  migration, old snapshots are pruned to five while unrelated files survive, and
  a corrupt file fails verification;
- the migration list starts at 1, strictly increases, ends at `SCHEMA_VERSION`,
  and every declared checksum matches its SQL;
- traversal counts each outcome exactly once and only `ok` entries contribute
  bytes; limits are clamped and a zero entry cap means "unbounded";
- a nested tree, an empty root, a missing root, and a file root each produce the
  expected entries and counters on the real filesystem;
- the depth limit reports the skipped directory without hiding it and does not
  emit entries below it;
- an entry budget stops the walk at the cap and sets `truncated` (not
  `cancelled`), and cancellation stops a larger tree early while keeping partial
  counters;
- Unicode names survive the round trip, and a path longer than 260 characters is
  created and traversed;
- a Windows junction loop is reported once as a skipped reparse point and is not
  descended into;
- invalid traversal requests (no roots, relative root, too many roots, worker
  count, channel capacity) are rejected before any thread starts.

They do **not** establish that the UI is visually correct under WebView2; layout
and theme appearance still need a manual smoke test.

Commands: `npm test` (run once), `npm run test:watch`, `cd src-tauri; cargo test`.

## 2. Strategy

The product's risk is concentrated at boundaries: the OS, the filesystem, IPC,
and persistence. Tests are therefore weighted toward **domain logic** and
**adapter contracts**, not snapshot-heavy UI tests.

```text
        E2E (few)            packaged app, install/upgrade, offline, read-only flow
      Integration            pipeline + SQLite + fixtures, cancellation, permissions
    Adapter contracts        Windows API behind traits, repository queries, migrations
  Unit (many)                domain rules, policies, error mapping, DTOs, parsers
```

### 2.1 Unit tests (Rust)

Target: fast, no Windows, no DB, no UI.

- job state machine transitions (queued → running → cancelling → cancelled);
- path and input validation (length, traversal, reserved names, UNC);
- protected-path policy decisions;
- retention calculations;
- evidence classification (positive, negative, missing evidence, conflicting);
- error-code mapping and redaction;
- DTO serialization shape (field names, enum tags, nullability).

### 2.2 Adapter contract tests

Define a trait per capability and test the contract, then the Windows
implementation separately.

```rust
pub trait ProcessCollector {
    fn list(&self, limit: usize) -> Result<ProcessPage, CollectorError>;
}
```

Contract tests assert: ordering is stable, limits are respected, access-denied is
reported (not swallowed), and identity is stable across calls.

### 2.3 Integration tests

Use temporary directories and generated fixtures. Required scenarios:

| Scenario | Assertion |
|---|---|
| Empty directory | Completes, zero records, no error |
| Permission-restricted folder | Reports `denied` count, continues |
| File deleted mid-scan | Reports `disappeared`, does not fail the scan |
| Junction/reparse loop | Does not loop forever; policy applied |
| Unicode + long path (> 260) | Recorded correctly |
| Locked file | Enrichment marked `failed`, scan continues |
| Cancellation at each phase | Stops within the latency budget, state is consistent |
| Interrupted session (kill process) | Session recoverable or safely discardable |
| Duplicate candidates | Size filter first, hash only when eligible |
| DB disk-full simulation | Session marked failed, prior data intact |

Covered today by the B-022 walker and the B-023/B-025 job layer: empty directory,
junction/reparse loop, Unicode + long path (> 260), cancellation at start and
mid-walk through the job supervisor, permission denied (live `icacls` fixture,
Windows only), missing root, entry-budget truncation, busy second scan, shutdown
cancel, progress `seq` and pacing, and request validation. File deleted
mid-scan, locked file, interrupted session, duplicate candidates, and DB
disk-full still require scan sessions or an enrichment stage and are untested.
The job tests use temporary fixture trees only; no real user data is scanned, and
they are wall-clock-tolerant (polling with a 10 s ceiling), not timing-exact.

The prototype deliberately has **no new crate**: enumeration uses `std::fs`, and
`walkdir` is no longer planned (`docs/DEPENDENCIES.md` §6).

### 2.4 Frontend tests

- Service adapters: mock IPC, assert success and every error class renders an
  actionable message.
- Critical components: keyboard navigation, focus visibility, loading/empty/
  partial/error states, confirmation dialogs naming the exact target.
- Virtualized tables: sorting/filtering without loading the full dataset.
- No snapshot tests as the only coverage for a workflow.

### 2.5 E2E (packaged app)

Run on Windows VMs against the real installer:

```text
install -> launch -> dashboard populated -> refresh -> cancel a scan
-> close -> upgrade -> verify data -> uninstall -> verify cleanup
offline run (network disabled) -> no errors, no egress attempts
non-admin run -> read-only features work, no elevation prompt
```

## 3. Fixtures

- Generated programmatically, never committed as large binaries.
- Deterministic names, sizes, and timestamps.
- Include: empty dirs, deep nesting, long paths, Unicode, junctions, sparse files,
  zero-byte files, > 4 GB logical size (sparse), and permission-denied dirs.
- Fixture generator lives in the test tree and documents its parameters.

## 4. Tooling plan

| Need | Choice | Notes |
|---|---|---|
| Rust tests | built-in `cargo test` | `#[cfg(windows)]` for adapter tests |
| Temp dirs | `tempfile` | add to `[dev-dependencies]` |
| Frontend unit | Vitest | native Vite integration |
| Component tests | `@testing-library/react` + `jsdom` | accessibility queries first |
| Mocking IPC | a local module mock, not a global | keeps `src/services` testable |
| E2E | `tauri-driver` + WebdriverIO | Windows-only in CI |
| Coverage | `cargo-llvm-cov`, `vitest --coverage` | report, do not gate on a number yet |

Adding any of these requires updating [`build.md`](build.md) and
[`docs/DEPENDENCIES.md`](docs/DEPENDENCIES.md).

## 5. Quality gates

A change is not complete until:

```text
[ ] npm run build passes (type-check + production build)
[ ] cargo fmt --all -- --check passes
[ ] cargo clippy --all-targets -- -D warnings passes
[ ] cargo test passes
[ ] New behavior has a test that fails without the change
[ ] Error paths are tested, not just the happy path
[ ] No test depends on machine-specific paths, user names, or timing luck
```

Release gates are stricter and live in [`docs/RELEASE_POLICY.md`](docs/RELEASE_POLICY.md).

## 6. Windows test matrix (proposed)

| Dimension | Values |
|---|---|
| OS | Windows 11 24H2+; Windows 10 22H2 (if supported) |
| Privilege | Standard user; elevated (control paths only) |
| Locale/paths | en-US; a non-ASCII path fixture |
| Display | 100% and 150% scaling; minimum window size |
| Storage | NTFS system drive; removable; network share (if supported) |
| Antivirus | Defender enabled; third-party AV best-effort |

Formal support matrix is an open decision (`docs/plan.md` §12).

## 7. Flakiness policy

- A test that fails intermittently is a defect, not a nuisance. Quarantine it with
  an issue link; do not silently retry forever.
- No `sleep()`-based synchronization in tests; wait on state or use timeouts.
- Timing assertions use generous ceilings and measure relative to a baseline.

## 8. What not to test

- Framework behavior (React rendering, Tauri window creation) beyond a smoke test.
- Third-party library internals.
- Visual pixel differences as a primary gate.

## 9. Reporting

When reporting test results, state exactly what ran:

```text
cargo test            -> 78 tests passed (ipc, error, logging, redact, startup,
                         dto::{app_info, system_snapshot, process_list,
                         diagnostics}, domain::{system, process, diagnostics,
                         traversal}, adapters::windows::{system_info, processes},
                         adapters::persistence::{migrations, sqlite},
                         adapters::filesystem::traversal,
                         commands::{app, system, processes, diagnostics})
npm test              -> 20 tests passed (7 files: ipc, BackendStatus, ThemeToggle,
                         SystemOverview, ProcessList, Diagnostics, ErrorBoundary)
npm run build         -> passed (tsc + vite)
```

Never summarize "all tests pass" without stating the counts and what they cover —
ninety-eight tests that mostly assert read-only contracts say nothing about scan
sessions, crash recovery, or the UI that will consume traversal.
