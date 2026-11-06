# AI_TESTING_RULES

> Mandatory tests and evidence before work is considered complete.
> **Last updated:** 2026-11-06 · **Strategy:** [`TESTING.md`](TESTING.md)

---

## 1. Baseline: no change is done without these

```powershell
npm run build
cd src-tauri
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

If a command cannot be run, say why. "It should pass" is not evidence.

## 2. Required tests by change type

| Change | Minimum tests |
|---|---|
| Domain rule / policy | Unit tests: positive, negative, boundary, missing input |
| New IPC command | Validation (valid/invalid/oversized), error mapping, authorization |
| Windows adapter | Contract test behind the trait + one integration test on Windows |
| File traversal change | Empty dir, denied dir, disappeared file, junction/loop, long/Unicode path |
| Hashing | Correct digest, size cap, cancellation, failure status |
| Database query | Result correctness + `EXPLAIN QUERY PLAN` on a realistic fixture |
| Migration | Fresh DB + upgrade from every released schema + interrupted migration |
| Retention | Deletes what it should, preserves what it must (quarantine/audit) |
| UI workflow | Loading, empty, partial, error, cancellation, stale response |
| Accessibility change | Keyboard path + accessible name assertions |
| Bug fix | Regression test that fails before the fix |
| Performance-sensitive code | Measured baseline + post-change comparison, same workload |
| Security-relevant code | Abuse-case test (malformed input, path substitution, privilege denial) |

## 3. Test quality rules

1. **Deterministic.** No wall-clock dependency, no random ordering, no
   machine-specific paths.
2. **Isolated.** Each test creates and cleans its own fixtures (temp dirs).
3. **Fast.** Unit tests in milliseconds; integration tests bounded by a timeout.
4. **Explicit.** One behavior per test; the name states the expectation.
5. **Honest.** A skipped test is reported as skipped, not passing.
6. **No sleeps.** Wait on state or use a bounded poll with a deadline.
7. **No shared mutable fixtures** across tests without synchronization.

## 4. What must be asserted

For collectors and scans, assert the **outcome taxonomy**, not just success:

```text
ok | skipped | denied | disappeared | unsupported | failed
```

A test that only checks "no error" does not prove partial results are reported.

For IPC, assert:

- exact error `code` for each failure class;
- that `detail` does not contain raw paths or stack traces;
- that limits are clamped, not trusted.

For UI, assert:

- the user sees why data is missing, not an empty table;
- destructive confirmations name the target;
- cancellation is reachable by keyboard.

## 5. Fixtures

- Generate programmatically; never commit large binaries.
- Cover: empty dir, deep nesting, long path, Unicode, junction, locked file,
  zero-byte, sparse large file, denied directory.
- Document parameters in the test module.
- Fixture creation failures must fail the test loudly, not skip it.

## 6. Coverage expectations

Coverage is a diagnostic, not a gate yet. Targets once tooling exists:

| Area | Target |
|---|---|
| `domain/` | ≥ 90% line coverage |
| `commands/` | ≥ 80%, with every error code exercised |
| `adapters/` | ≥ 70% plus contract tests |
| UI services | ≥ 80% |
| UI components | critical paths only |

Do not write meaningless assertions to raise a number.

## 7. Untestable claims

If a behavior cannot be tested in the current environment (e.g., a specific
Windows edition, elevation, a huge volume), that is a **known gap**:

1. record it in [`docs/KNOWN_ISSUES.md`](docs/KNOWN_ISSUES.md);
2. add a manual test step in [`docs/DEPLOYMENT.md`](docs/DEPLOYMENT.md) or the
   release checklist;
3. state it in the completion report under "Not verified".

## 8. Definition of done

```text
[ ] Code implemented and reviewed against AI_CODE_REVIEW.md
[ ] Required tests added and passing
[ ] Build, format, clippy pass
[ ] Failure and partial-result paths covered
[ ] Documentation updated (FEATURES, CHANGELOG, API/DB as applicable)
[ ] No new known issue left unrecorded
[ ] Evidence captured in the report (commands + outcomes)
```

## 9. Forbidden shortcuts

| Shortcut | Why it fails review |
|---|---|
| Deleting a failing test | Hides a regression |
| `#[ignore]` without an issue link | Becomes permanent silence |
| Increasing a timeout to mask flakiness | Symptom, not cause |
| Asserting only that a call returns `Ok` | Does not verify behavior |
| Mocking the module under test | Tests the mock, not the code |
| Marking a checklist item done with no test | Violates the evidence rule |
| Writing "tested manually" with no steps recorded | Not reproducible |
