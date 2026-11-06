# AI_CODE_REVIEW

> How an AI agent reviews its own changes before reporting them.
> **Last updated:** 2026-11-06

---

## 1. Principle

You are not reviewing "does it compile". You are reviewing **"would a careful
maintainer accept this, and can I prove the claims I am about to make?"**

Review the diff, not your memory of writing it.

## 2. Procedure

```text
1. Re-read the task and the original constraints.
2. Inspect the full diff (git diff), file by file.
3. Read each changed file in its final state (not just the diff hunks).
4. Run the verification commands.
5. Walk the checklists below.
6. Fix what fails; re-run.
7. Report with evidence.
```

Do not skip step 3. Diff context hides broken surrounding code.

## 3. Checklist A — Correctness

- [ ] The change does exactly what the task asked, no more.
- [ ] Behavior on empty input, zero results, and missing data is defined.
- [ ] Error paths return typed errors, not panics or silent defaults.
- [ ] Boundaries: off-by-one in limits, pagination, and ranges.
- [ ] Cancellation leaves consistent state (no half-written session).
- [ ] Concurrency: no shared mutable state without a clear owner.
- [ ] Async code cannot deadlock or hold a lock across an `.await`.
- [ ] Numeric types: byte counts and timestamps cannot overflow or truncate.

## 4. Checklist B — Honesty

- [ ] Comments and docs describe what the code **does**, not what is planned.
- [ ] No "implemented" claim for stubs, TODOs, or unregistered commands.
- [ ] Error messages do not promise capability that does not exist.
- [ ] The completion report separates verified from unverified.
- [ ] `docs/FEATURES.md` status matches the code.

## 5. Checklist C — Architecture

- [ ] No OS access in the renderer.
- [ ] No `windows`, `tauri`, or SQL imports in domain code.
- [ ] Dependency direction respected; no cycles.
- [ ] Command is thin: validate → authorize → delegate → map.
- [ ] Persistence is behind a repository; no SQL in workflows.
- [ ] New module documented in [`ARCHITECTURE.md`](ARCHITECTURE.md).
- [ ] No global mutable engine or hidden singleton.

## 6. Checklist D — Security

- [ ] Every IPC field is validated (type, range, length, path).
- [ ] No path traversal, UNC surprise, reparse-point, or TOCTOU gap in new file code.
- [ ] Protected paths denied by default.
- [ ] No secrets, tokens, or personal paths in code, fixtures, or logs.
- [ ] Capabilities/CSP unchanged, or the change is justified and documented.
- [ ] No network egress added.
- [ ] `unsafe` blocks have safety invariants and tests.
- [ ] Privileged operations require confirmation + audit + recovery.

## 7. Checklist E — Resources

- [ ] Result sets are bounded or paged.
- [ ] Queues and channels are bounded.
- [ ] Event emission is throttled/coalesced.
- [ ] Memory does not grow with the number of files (streaming, not accumulating).
- [ ] Hashing and scanning have size/concurrency caps.
- [ ] Logs are bounded and redacted.
- [ ] Long transactions are avoided; batches are sized by measurement.

## 8. Checklist F — Tests

- [ ] New behavior has a test that fails without the change.
- [ ] Bug fix has a regression test.
- [ ] Error paths tested, not only success.
- [ ] IPC validation tested with malformed/oversized/unauthorized input.
- [ ] No test depends on machine-specific paths or wall-clock timing.
- [ ] Tests are deterministic and fast; no `sleep()` synchronization.

## 9. Checklist G — Frontend

- [ ] Components do not call `invoke()` directly.
- [ ] Loading, empty, partial, error, and stale states all render.
- [ ] Keyboard navigable; visible focus; screen-reader labels present.
- [ ] Status is not conveyed by color alone.
- [ ] Large tables virtualized; no full dataset in component state.
- [ ] Destructive confirmation names the exact target and consequence.
- [ ] No dependency added for styling/state without justification.

## 10. Checklist H — Documentation

- [ ] [`build.md`](build.md) updated for new commands/scripts.
- [ ] [`API.md`](API.md) updated for new commands/events/DTOs.
- [ ] [`DATABASE.md`](DATABASE.md) + [`docs/MIGRATIONS.md`](docs/MIGRATIONS.md) for schema.
- [ ] [`docs/FEATURES.md`](docs/FEATURES.md) + [`docs/CHANGELOG.md`](docs/CHANGELOG.md) for status.
- [ ] [`docs/DECISIONS.md`](docs/DECISIONS.md) for architectural choices.
- [ ] [`AI_CHANGELOG.md`](AI_CHANGELOG.md) for structural AI-assisted changes.

## 11. Self-review questions

Ask these explicitly and answer in one line each:

1. What could this break that I did not test?
2. What happens if the user cancels mid-operation?
3. What happens if the OS denies access?
4. What happens if the file disappears between two calls?
5. What is the worst-case memory and time for this code path?
6. If this input were malicious, what is the worst it could do?
7. Is there any claim in my summary that I cannot point to evidence for?

If any answer is "I don't know", that is the next thing to investigate.

## 12. Common self-review failures in this repo

| Failure | Fix |
|---|---|
| Adding a scanner because Release 1 is planned | Re-scope to the task |
| `unwrap()` on a Win32 result | Return `ACCESS_DENIED`/`UNSUPPORTED` |
| Returning `0` for an unavailable field | Add it to `unavailable[]` with a reason |
| Loading all files into React state | Aggregate in Rust, page results |
| Logging a full path at `info` | Redact or log at `debug` with a hash |
| Reporting "tests pass" with zero tests | State the count |
| Adding `serde`/`tokio` with no consumer | Remove; add when needed |
| Leaving a stale doc claim | Update the doc in the same change |

## 13. Report template

Use the structure from [`AI_WORKFLOW.md`](AI_WORKFLOW.md) §8. The review is
complete only when the **Verified** section contains real command output and the
**Not verified** section is honest.
