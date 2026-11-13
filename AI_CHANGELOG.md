# AI_CHANGELOG

> Record of **AI-assisted architectural and structural changes**. Product-facing
> version history lives in [`docs/CHANGELOG.md`](docs/CHANGELOG.md); decisions
> live in [`docs/DECISIONS.md`](docs/DECISIONS.md).
>
> Only record changes that affect structure, boundaries, contracts, or process.
> Routine edits do not belong here.
> **Last updated:** 2026-11-13

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

## 2026-11-13 — System snapshot vertical slice (port → adapter → DTO → UI)

- **Agent/tool:** OpenCode (DeepSeek V4.1 Flash Free).
- **Scope:** `src-tauri/src/{ports/*, domain/*, adapters/windows/*,
  dto/system_snapshot.rs, commands/system.rs, error.rs, lib.rs}`,
  `src-tauri/Cargo.toml`, `src/services/system.ts`,
  `src/components/SystemOverview.{tsx,test.tsx}`, `src/App.{tsx,css}`.
- **Change:**
  - Created the layering the architecture rules already described: a
    platform-neutral `domain/` model with explicit availability reasons, a
    one-capability `ports/` trait, and a `windows-sys` adapter containing every
    `unsafe` block.
  - Added `get_system_snapshot` (no input, read-only, blocking work on a
    blocking thread) returning a versioned `SystemSnapshotDto`. Wholly
    unavailable groups are omitted and named in `unavailable[]`; leaf gaps keep
    their group and carry a field path and reason.
  - Added `SystemOverview` with loading, error, refresh, last-updated, and
    per-field unavailable display. A single in-flight refresh plus an `active`
    guard prevents stale responses from winning.
  - Removed the module-level `#[allow(dead_code)]` from `error.rs`; the
    allowance is now scoped to the published `ErrorCode` taxonomy (B-091).
  - Added `windows-sys` 0.61.2 as a `cfg(windows)` dependency (ADR-012).
- **Why:** B-002 is the first Release 0 value and the first real test of the
  domain/OS boundary. Building it before the app shell proves the contract
  without locking in navigation or feature state.
- **Risk:**
  - Win32 collection is verified on one machine (KI-039); edition- and
    architecture-specific behavior is not covered.
  - Network drives deliberately report `unsupported` sizes/labels (KI-038); a
    user may read that as missing data.
  - Nine `windows-sys` feature flags are enabled; each addition expands the
    audited surface.
  - CI still has not run on GitHub (B-092), so none of this is gated remotely.
- **Evidence:** `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D
  warnings`, `cargo test` (18 passed), `npm run build`, `npm test` (9 passed),
  `npm run lint`, `npm run format:check`.
- **Follow-up:** B-004 (app shell/dashboard), B-005 completion at app-shell level,
  B-090 (runtime DTO validation), B-092 (first green CI run), B-003 (processes).

---

## 2026-11-06 — IPC vertical slice implemented (Rust boundary → React)

- **Agent/tool:** OpenCode (DeepSeek V4.1 Flash Free).
- **Scope:** `src-tauri/src/{lib.rs, ipc.rs, error.rs, dto/*, commands/*}`,
  `src/services/*`, `src/components/BackendStatus.tsx`,
  `package.json`, `tsconfig.node.json`, `eslint.config.js`, `vitest.config.ts`,
  `.prettierrc.json`, `.prettierignore`, `rust-toolchain.toml`,
  `.github/workflows/ci.yml`, `src/App.{tsx,css}`.
- **Change:**
  - Established the renderer → IPC → Rust path with one read-only command
    (`get_app_info`) returning a versioned DTO through the `CommandResult`
    envelope.
  - Introduced the stable error taxonomy (`ErrorCode`) and `AppError` with
    correlation IDs as the only error shape crossing the boundary.
  - Added the single frontend `invoke` call site (`src/services/ipc.ts`) and a
    domain module per command; components no longer touch Tauri directly.
  - Changed the wire format of DTO fields from the originally specified
    `snake_case` to `camelCase` (`#[serde(rename_all = "camelCase")]`), removing
    the hand-written TypeScript mapping layer. Recorded as ADR-011.
  - Added the test/lint/format toolchain, a `rust-toolchain.toml`, and a
    three-job Windows CI workflow.
- **Why:** Everything else in Release 0 (system collection, processes, jobs,
  persistence) depends on a typed, stable, testable boundary. Building the data
  collectors first would have locked in an untested error/shape contract.
- **Risk:**
  - `error.rs` is only exercised by tests, so it carries a scoped
    `#[allow(dead_code)]` on the module. This hides dead code there until the
    first fallible command lands (B-091) — deliberately time-boxed.
  - The CI workflow is untested on GitHub (B-092). Local runs of every step pass,
    but a workflow file is not a gate.
  - `camelCase` diverges from the original `API.md` design; anyone relying on the
    old spec would break. Documented rather than silently changed.
- **Evidence:** `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D
  warnings`, `cargo test` (8 passed), `npm run build`, `npm test` (6 passed),
  `npm run lint`, `npm run format:check`.
- **Follow-up:** B-002 (`get_system_snapshot`), B-004 (app shell), B-090 (runtime
  DTO validation), B-091 (remove the dead-code allow), B-092 (first green CI run).

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
