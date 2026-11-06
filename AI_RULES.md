# AI_RULES — Hard Rules

> These are not suggestions. A change that violates any rule below is rejected,
> even if it compiles and the tests pass.
> **Last updated:** 2026-11-06

---

## A. Honesty and scope

1. **Never describe planned work as implemented.** Use the status vocabulary:
   `Implemented`, `Partial`, `Planned`, `Proposed`, `Not started`.
2. **Do not invent facts about the project.** Check the file before asserting it
   exists, and cite the path.
3. **Do not add features outside the requested task** because the plan mentions
   them. Do the task; note the rest.
4. **Do not silently change scope.** If the task requires an architectural change,
   say so and explain the trade-off before implementing.
5. **Never fabricate test output, benchmark numbers, or review results.** If you
   did not run it, say you did not run it.

## B. Architecture

6. **The renderer never accesses the OS.** No `child_process`, no shell plugin, no
   filesystem plugin, no `eval`, no direct Win32 bridge.
7. **Domain code imports no Windows API, no Tauri, and no SQL.** Windows behavior
   lives in `adapters/windows`; persistence in `adapters/persistence`.
8. **No cycles between modules.** Dependency direction:
   `features → services → commands → app → domain ← ports ← adapters`.
9. **No global mutable engine.** Use explicit state owned by the app layer.
10. **No mega-command.** One command per intent, validated at the boundary.
11. **Persisted and exported data carries a schema version.**
12. **New module = new documented boundary.** Add its responsibility to
    `ARCHITECTURE.md`; do not grow `lib.rs` into a monolith.

## C. Security

13. **Treat all IPC input as hostile.** Validate type, range, length, and path
    before use. UI validation is never the control.
14. **Never `unwrap()`/`expect()` on input-derived data** in a command or worker
    path. Return a typed error.
15. **`unsafe` only in Windows adapters**, with a `// SAFETY:` invariant comment
    and a test or documented review.
16. **Do not weaken the CSP, capabilities, or permission list** to make a library
    work. Fix the integration instead.
17. **No new network egress without explicit approval**, a consent flow, and a
    privacy review.
18. **Never log or commit secrets**, tokens, signing keys, file contents, full
    command lines, or unnecessary personal paths.
19. **Never add a plugin or dependency with broad capabilities** without
    documenting why a narrower option fails.
20. **Never execute shell commands** for core functionality. Use documented APIs.

## D. Safety of operations

21. **No destructive operation without:** explicit confirmation naming the exact
    target, a recovery path, and an audit record.
22. **Prefer quarantine over deletion.** Permanent deletion must never be
    automatic or the default.
23. **Never classify a file as malware from name, extension, path, or one
    heuristic.** Use evidence + confidence + rule version.
24. **Never auto-delete "large", "old", "hidden", or "suspicious" files.**
25. **Protected paths are denied by default**, with an explicit allowlist policy.
26. **Any path that writes files must handle TOCTOU, reparse points, and hard
    links** and must be reviewed before shipping.
27. **No elevation by default.** Elevation is one narrowly scoped operation, with
    a fallback when denied.

## E. Resources and process

28. **Every long-running operation is cancellable and bounded** (memory,
    concurrency, result size, event rate).
29. **Never stream unbounded rows to the UI.** Aggregate or page.
30. **No background task without supervision**; failures must reach the owning
    job state.
31. **No unbounded log growth.** Rotation and redaction are mandatory.
32. **Do not remove or rewrite existing user work** without approval. Investigate
    uncommitted changes first.
33. **Do not run `git push`, force-push, or rewrite history.**
34. **Do not delete build output or docs** as a "cleanup" without approval.
35. **Do not install global tools** or modify the system outside the repository.

## F. Dependencies

36. **No dependency without:** justification, license check, maintenance check,
    and an entry in `docs/DEPENDENCIES.md`.
37. **No duplicate capability** (a second HTTP client, a second state manager, a
    second date library) for one use case.
38. **Prefer the standard library and existing crates.** Small footprint is part
    of the product.

## G. Testing and completion

39. **New behavior requires a test that fails without the change.**
40. **Bug fix requires a regression test.**
41. **Run the verification commands** in `build.md` before reporting completion.
42. **Report exactly what ran**, including empty suites.
43. **A checklist item is complete only with evidence** (code + test + measurement
    or reviewed decision).

## H. Documentation

44. **Update the doc that would become wrong** in the same change. See the table
    in [`AGENTS.md`](AGENTS.md) §6.
45. **Add dated entries; never rewrite history** to make progress look better.
46. **Link, don't duplicate.** Commands live in [`build.md`](build.md); contracts
    in [`API.md`](API.md); schema in [`DATABASE.md`](DATABASE.md).

---

## Escalation

If a task cannot be completed without violating a rule:

1. Stop.
2. State which rule conflicts and why.
3. Offer the smallest compliant alternative.
4. Ask for an explicit decision.

Record the outcome in [`docs/DECISIONS.md`](docs/DECISIONS.md) if it changes an
architectural boundary.
