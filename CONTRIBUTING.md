# Contributing

> **Status:** Active · **Last updated:** 2026-11-06
>
> This project is pre-alpha. Expect interfaces to move. Read
> [`AGENTS.md`](AGENTS.md) too if you contribute with AI assistance — the same
> rules apply to humans using AI.

---

## 1. Before you start

1. Read [`README.md`](README.md) for the product and honest current state.
2. Read [`docs/plan.md`](docs/plan.md) for sequencing and constraints.
3. Read [`ARCHITECTURE.md`](ARCHITECTURE.md) before adding a module.
4. Check [`docs/BACKLOG.md`](docs/BACKLOG.md) and [`docs/KNOWN_ISSUES.md`](docs/KNOWN_ISSUES.md).
5. For anything destructive, privileged, or networked, open an issue **first**.

## 2. Environment setup

Prerequisites and the verified toolchain table: [`build.md`](build.md) §1.

```powershell
npm install
npm run tauri dev
```

If the app does not launch, work through [`build.md`](build.md) §9 before
changing code.

## 3. Branch and commit conventions

**Branches**

```text
feat/<short-scope>       # new capability
fix/<short-scope>        # bug fix
docs/<short-scope>       # documentation only
refactor/<short-scope>   # no behavior change
chore/<short-scope>      # tooling, deps, build
```

**Commits** — Conventional Commits, imperative mood, scoped when useful:

```text
feat(system): add read-only system snapshot command
fix(storage): clamp query_files limit to 500
docs(security): document TOCTOU policy for quarantine
chore(deps): bump tauri to 2.12.4
```

Rules:

- One logical change per commit.
- The message says *why*, not just *what*.
- Never commit secrets, signing keys, tokens, or machine-specific paths.
- Never commit `node_modules/`, `dist/`, `src-tauri/target/`, or generated schemas.

## 4. Code standards

### TypeScript / React

- TypeScript `strict` is on; do not weaken it or add `any` to silence errors.
- No unused locals/parameters (enforced by `tsconfig.json`).
- Components render; they do not call `invoke()` directly. Use `src/services`.
- Keep UI state local unless it is genuinely shared. No global store for one screen.
- Prefer semantic HTML and accessible names; every interactive element must be
  keyboard reachable.
- Do not introduce a UI framework or state library without an ADR.

### Rust

- `cargo fmt` and `cargo clippy --all-targets -- -D warnings` must pass.
- Domain code must not import `windows`, `tauri`, or SQL.
- Adapters return explicit outcome enums; do not collapse failures into defaults.
- No `unwrap()`/`expect()` on input-derived data. Use typed errors.
- `unsafe` only in `adapters/windows`, with a `// SAFETY:` comment and a test.
- Prefer bounded channels and cooperative cancellation over detached tasks.

### Styling

- Plain CSS with custom properties, matching the existing token approach.
- No inline styles that bypass the design tokens for color/spacing.
- Status must never be conveyed by color alone.

## 5. Tests are part of the change

- New domain logic → unit tests.
- New IPC command → validation, error, and authorization tests.
- New adapter → contract test behind a trait; Windows-specific behavior isolated.
- Bug fix → a regression test that fails before the fix.

No test runner exists yet; if you add one, update [`build.md`](build.md),
[`TESTING.md`](TESTING.md), and `package.json` in the same change.

## 6. Documentation obligations

Update the document that would become wrong:

| Change | Document |
|---|---|
| Command/script | [`build.md`](build.md) |
| IPC contract | [`API.md`](API.md) |
| Schema | [`DATABASE.md`](DATABASE.md), [`docs/MIGRATIONS.md`](docs/MIGRATIONS.md) |
| Permission/capability | [`SECURITY.md`](SECURITY.md), [`docs/ACCESS_CONTROL.md`](docs/ACCESS_CONTROL.md) |
| Feature status | [`docs/FEATURES.md`](docs/FEATURES.md), [`docs/CHANGELOG.md`](docs/CHANGELOG.md) |
| Architectural decision | [`docs/DECISIONS.md`](docs/DECISIONS.md) |
| Performance-relevant code | [`docs/PERFORMANCE.md`](docs/PERFORMANCE.md) |

Do not mark planned work as complete. Add dated entries rather than rewriting
history to look better.

## 7. Pull request checklist

```text
[ ] Scope is one logical change; description explains why
[ ] UI does not touch the OS; new IPC input is validated in Rust
[ ] Domain logic has no Windows/Tauri/SQL imports
[ ] No new dependency without justification and DEPENDENCIES.md entry
[ ] Tests added or updated at the appropriate level
[ ] npm run build passes
[ ] cargo fmt --check passes
[ ] cargo clippy --all-targets -D warnings passes
[ ] cargo test passes
[ ] No secrets, tokens, or personal paths in code, logs, or fixtures
[ ] User-facing errors are actionable and do not leak sensitive detail
[ ] Docs updated per section 6
[ ] Screenshots/GIFs for visible UI changes
[ ] Reversal/rollback considered for destructive behavior
```

## 8. Review standards

Reviewers check, in order:

1. **Correctness and honesty** — does it do what it says, and does the description
   match reality?
2. **Security boundary** — untrusted input, privileges, capabilities, paths.
3. **Resource bounds** — memory, concurrency, result size, event rate.
4. **Failure behavior** — partial results, cancellation, error messages.
5. **Tests and evidence** — the claim is verifiable.
6. **Maintainability** — naming, module placement, duplication.

A review comment that says "this could be simpler" is valid. A PR that adds a
dependency to avoid 30 lines is usually not.

## 9. Responsible disclosure

Security issues go through [`SECURITY.md`](SECURITY.md), never a public PR or
issue.

## 10. Licensing

No license has been selected. By contributing before a license exists, you agree
that your contribution may be relicensed when the project selects one. This must
be resolved before public distribution — see [`docs/COMPLIANCE.md`](docs/COMPLIANCE.md).
