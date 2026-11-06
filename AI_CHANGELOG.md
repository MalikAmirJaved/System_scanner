# AI_CHANGELOG

> Record of **AI-assisted architectural and structural changes**. Product-facing
> version history lives in [`docs/CHANGELOG.md`](docs/CHANGELOG.md); decisions
> live in [`docs/DECISIONS.md`](docs/DECISIONS.md).
>
> Only record changes that affect structure, boundaries, contracts, or process.
> Routine edits do not belong here.
> **Last updated:** 2026-11-06

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
