# AI_WORKFLOW

> How an AI agent should approach work in this repository.
> **Last updated:** 2026-11-06

---

## 1. The loop

```text
1. Understand   -> what is asked, what exists, what must not change
2. Inspect      -> read files, git status, git diff, relevant docs
3. Plan         -> 1-5 concrete steps, files to touch, verification command
4. Implement    -> smallest coherent change, in dependency order
5. Verify       -> build, format, lint, tests, manual behavior
6. Document     -> update the docs that would become wrong
7. Report       -> changed / verified / not verified / gaps
```

Do not compress steps 1–3. Most bad changes in this repository will come from
assuming a planned feature exists.

## 2. Step 1 — Understand

Answer these before editing:

- Which release/feature does this belong to, and is that feature implemented?
- Does it cross a trust boundary (UI → IPC → OS → disk)?
- Is it read-only or does it change system state?
- What are the bounds (size, concurrency, duration)?
- What is the failure behavior if the OS says no?

If the answer to "does it change system state?" is yes, **stop** and check
[`SECURITY.md`](SECURITY.md) §7 and [`AI_SECURITY_RULES.md`](AI_SECURITY_RULES.md).

## 3. Step 2 — Inspect

```powershell
git status --short
git diff
```

Then read, in order:

1. the file(s) you will change;
2. their direct dependencies and callers;
3. the relevant contract doc (`API.md`, `DATABASE.md`, `ARCHITECTURE.md`);
4. the test that covers the area, if any.

Never edit a file you have not read. Never assume a function exists because the
plan describes it.

## 4. Step 3 — Plan

Write the plan in your working response, briefly:

```text
Change: add get_system_snapshot command (read-only)
Files:  src-tauri/src/commands/system.rs, src-tauri/src/lib.rs, src/services/system.ts
Steps:  1) port trait + DTO  2) windows adapter  3) command + validation
        4) register  5) unit tests  6) API.md + FEATURES.md
Verify: cargo test; npm run build; manual launch
Risk:   Win32 call may fail on some editions -> explicit Unsupported outcome
```

Rules for the plan:

- Prefer 1–5 steps; if more, split the task.
- Touch the smallest set of files.
- Identify the exact verification command up front.
- Name the risk you are most likely to get wrong.

## 5. Step 4 — Implement

Order of work for a typical feature:

```text
DTO/contract -> domain logic + tests -> port trait -> adapter -> command
-> register in lib.rs -> frontend service adapter -> UI state -> UI
-> docs
```

Rules:

- One logical change per commit-sized unit.
- Do not reformat unrelated code.
- Do not rename public contracts without a version bump and doc update.
- Do not leave `TODO` in shipped code without a tracked backlog entry.
- If you discover an unrelated bug, report it; do not fix it silently in the same
  change.

## 6. Step 5 — Verify

Minimum, for any code change:

```powershell
npm run build
cd src-tauri
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

For IPC or UI work, also launch the app (`npm run tauri dev`) and exercise the
flow, including one failure path. For persistence work, inspect the DB and test a
restart. For performance-sensitive work, measure — do not estimate.

If a command cannot run (missing tool, no Windows VM), say so explicitly.

## 7. Step 6 — Document

Use the table in [`AGENTS.md`](AGENTS.md) §6. At minimum:

- feature status → [`docs/FEATURES.md`](docs/FEATURES.md);
- user-visible change → [`docs/CHANGELOG.md`](docs/CHANGELOG.md);
- structural change → [`AI_CHANGELOG.md`](AI_CHANGELOG.md);
- decision → [`docs/DECISIONS.md`](docs/DECISIONS.md).

## 8. Step 7 — Report

```markdown
## Changed
- <file>: <what and why>

## Verified
- `npm run build` — passed
- `cargo test` — 3 tests passed (new: job_state_transitions)

## Not verified
- Packaged installer — requires a Windows VM

## Known gaps / next step
- Win32 fallback for Windows 10 not implemented; see BACKLOG item B-12
```

Never write "done" or "production ready" without the evidence above.

## 9. Handling ambiguity

| Situation | Action |
|---|---|
| Two reasonable interpretations | Pick the safer, smaller one; state the assumption |
| Requires a destructive operation | Stop and ask |
| Requires network egress | Stop and ask |
| Requires a new dependency | Justify and ask if non-trivial |
| Conflicts with a documented rule | Stop, cite the rule, offer an alternative |
| Existing code contradicts docs | Trust the code for behavior, fix the doc, note it |

## 10. Context management

- Prefer reading the specific file over searching the whole repo repeatedly.
- When a task spans many files, summarize what you learned before editing.
- Do not re-read files you just wrote; verify them with a targeted check instead.
- If the task grows beyond one coherent change, propose splitting it.

## 11. Stop conditions

Stop and report instead of pushing through when:

- a build fails for reasons unrelated to your change and you cannot isolate it;
- tests fail and the fix would require changing behavior beyond the task;
- you need credentials, signing keys, or a network resource;
- you would have to delete user work or rewrite git history;
- the change would violate one of the invariants in [`AGENTS.md`](AGENTS.md).
