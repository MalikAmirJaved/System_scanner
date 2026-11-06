# AGENTS.md — Instructions for AI Coding Agents

> **Scope:** every AI agent (or human acting with AI assistance) working in this
> repository. **Last updated:** 2026-11-06.
>
> Read this file fully before your first edit. It routes to the detailed rule
> files and defines the non-negotiables. If a rule here conflicts with an
> explicit user instruction, ask the user — do not silently pick a side.

---

## 0. Orientation (do this first, every session)

1. Read [`AI_CONTEXT.md`](AI_CONTEXT.md) — what this project is and what exists today.
2. Read [`docs/plan.md`](docs/plan.md) — the authoritative roadmap and constraints.
3. Read [`docs/FEATURES.md`](docs/FEATURES.md) — what is *actually* implemented.
4. Read [`docs/KNOWN_ISSUES.md`](docs/KNOWN_ISSUES.md) — known traps.
5. Inspect the working tree (`git status`, `git diff`) before editing. Uncommitted
   changes may be the user's in-progress work.

Do not skip step 3. The most common failure mode in this repository is treating a
planned feature as an existing one.

## 1. The five invariants

These are absolute. Violating one is a failed task, regardless of whether the
code compiles.

1. **Never claim unimplemented behavior as implemented.** Docs, comments, commit
   messages, and chat summaries must distinguish `Implemented`, `Planned`,
   `Proposed`, and `Not started`. A TODO is not a feature.
2. **The UI never touches the OS.** All system access goes through typed Tauri
   IPC to Rust. No `child_process`, no shell plugin, no filesystem plugin in the
   frontend.
3. **Domain logic never imports Windows APIs.** Windows-specific behavior lives
   behind adapters implementing narrow traits. The domain must compile and test
   without Windows.
4. **All IPC input is untrusted.** Validate paths, sizes, counts, and enum values
   in Rust. UI-side validation is usability only.
5. **Nothing destructive ships without a recovery path.** No delete, terminate,
   or service control before quarantine/rollback, audit, and explicit
   confirmation exist.

## 2. Task workflow

Follow [`AI_WORKFLOW.md`](AI_WORKFLOW.md). Summary:

```text
understand -> inspect -> plan -> implement in small steps -> verify -> document -> report
```

Before writing code, state in one sentence what you are changing and why. If you
cannot, you do not understand the task yet.

## 3. Rules by concern

| Concern | File | Load when |
|---|---|---|
| Hard prohibitions and required behaviors | [`AI_RULES.md`](AI_RULES.md) | Always |
| Step-by-step approach to a task | [`AI_WORKFLOW.md`](AI_WORKFLOW.md) | Always |
| Reviewing your own diff | [`AI_CODE_REVIEW.md`](AI_CODE_REVIEW.md) | Before reporting completion |
| Layering, dependencies, module boundaries | [`AI_ARCHITECTURE_RULES.md`](AI_ARCHITECTURE_RULES.md) | Any code change |
| IPC, secrets, paths, privileges, dependencies | [`AI_SECURITY_RULES.md`](AI_SECURITY_RULES.md) | Any code change |
| Required tests and evidence | [`AI_TESTING_RULES.md`](AI_TESTING_RULES.md) | Before claiming done |
| Record of structural changes | [`AI_CHANGELOG.md`](AI_CHANGELOG.md) | After architectural work |

## 4. Commands agents may run

All commands are documented in [`build.md`](build.md). Agents may run read-only
inspection freely, and these verification commands:

```powershell
npm run build                                   # type-check + frontend build
cargo fmt --all -- --check                      # in src-tauri
cargo clippy --all-targets -- -D warnings       # in src-tauri
cargo test                                      # in src-tauri
npm audit --omit=dev
```

Agents **must not**:

- run `git push`, force-push, rewrite history, or change remotes;
- delete `node_modules`, `dist`, `src-tauri/target`, or `docs/` without approval;
- install global tools or change the system PATH;
- execute binaries produced by this project against real user data during
  development unless the task explicitly requires it;
- modify files outside the repository;
- add a dependency without stating why and checking that it is already vendored
  or available.

## 5. Dependency policy

Adding a dependency requires, in the same change:

1. A one-line justification in the PR/commit description.
2. Confirmation it is maintained and its license is acceptable.
3. An entry in [`docs/DEPENDENCIES.md`](docs/DEPENDENCIES.md).
4. No duplicate of an existing capability (e.g., do not add a second HTTP client
   or a second state manager for one screen).

Prefer the standard library and existing crates over new ones. This product's
value proposition includes a small, auditable footprint.

## 6. Documentation obligations

A change is incomplete until the docs that would become wrong are updated:

| Change | Update |
|---|---|
| New/changed command or script | [`build.md`](build.md) |
| New IPC command, event, or DTO | [`API.md`](API.md) |
| Schema or migration | [`DATABASE.md`](DATABASE.md), [`docs/MIGRATIONS.md`](docs/MIGRATIONS.md) |
| New capability or permission | [`SECURITY.md`](SECURITY.md), [`docs/ACCESS_CONTROL.md`](docs/ACCESS_CONTROL.md) |
| Feature status change | [`docs/FEATURES.md`](docs/FEATURES.md), [`docs/CHANGELOG.md`](docs/CHANGELOG.md) |
| Architectural decision | [`docs/DECISIONS.md`](docs/DECISIONS.md), [`AI_CHANGELOG.md`](AI_CHANGELOG.md) |
| Test strategy or fixtures | [`TESTING.md`](TESTING.md) |
| Performance-relevant change | [`docs/PERFORMANCE.md`](docs/PERFORMANCE.md) |

Never rewrite history in documentation to make progress look better. Add a dated
entry instead.

## 7. Reporting completion

Your final message must separate:

- **Changed** — files and behavior actually modified.
- **Verified** — exact commands run and their outcome.
- **Not verified** — what you could not run, and why.
- **Known gaps / next step** — honest residual work.

Do not use "done", "fully implemented", or "production ready" unless the
evidence exists in this session.

## 8. When blocked

Ask one focused question with a recommended default. Do not guess on:

- destructive behavior, privilege requirements, or data retention;
- network egress or third-party data sharing;
- supported Windows versions or distribution channel;
- licensing.

These are recorded as open decisions in [`docs/plan.md`](docs/plan.md) §12.
