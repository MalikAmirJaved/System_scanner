# Testing Strategy

> **Status:** **No tests exist yet.** No runner is configured for either language.
> This document defines the target strategy and the minimum bar for new work.
> **Last updated:** 2026-11-06

---

## 1. Current state

| Level | Tool | Configured | Tests |
|---|---|---|---|
| Rust unit/integration | `cargo test` | Yes (harness only) | 0 |
| Frontend unit | Vitest (proposed) | No | 0 |
| Frontend component | Testing Library (proposed) | No | 0 |
| E2E | tauri-driver / WebDriver (proposed) | No | 0 |
| Performance | custom harness (proposed) | No | 0 |
| Security | manual review + targeted tests | No | 0 |

`cargo test` compiles and runs zero tests. Do not report "tests pass" as evidence
of correctness while this table shows zero.

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
cargo test            -> 0 tests, harness compiled
npm run build         -> passed
npm test              -> not configured
```

Never summarize "all tests pass" when the suite is empty.
