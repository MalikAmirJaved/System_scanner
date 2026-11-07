# Changelog

All notable changes to this project are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versioning follows [Semantic Versioning](https://semver.org/) once a public release
exists. Until then, `0.x` versions may change behavior without notice.

> AI-assisted **architectural** changes are recorded separately in
> [`../AI_CHANGELOG.md`](../AI_CHANGELOG.md).

---

## [Unreleased]

### Added
- **Typed IPC boundary (implemented).** `src-tauri/src/ipc.rs`
  (`CommandResult` envelope), `src-tauri/src/error.rs` (`ErrorCode` taxonomy,
  `AppError` with per-error correlation IDs), `src-tauri/src/dto/app_info.rs`
  (`AppInfo`, `DTO_SCHEMA_VERSION = 1`), and the read-only command
  `get_app_info`.
- **Frontend IPC adapter.** `src/services/ipc.ts` (single `invoke` call site,
  `IpcError`), `src/services/app.ts`, and `src/components/BackendStatus.tsx` with
  loading, ready, error, and retry states wired into the welcome screen.
- **Tooling.** Vitest 5 + jsdom + Testing Library, ESLint 10 flat config,
  Prettier 3, and `rust-toolchain.toml` (stable channel + `rustfmt`/`clippy`).
- **CI workflow.** `.github/workflows/ci.yml` — `frontend`, `rust`, and `package`
  jobs on `windows-latest`. Written and verified step-by-step locally; it has
  **not** run on GitHub yet.
- `npm test`, `npm run test:watch`, `npm run lint`, `npm run format`,
  `npm run format:check`.
- ADR-011: IPC DTO fields are `camelCase` on the wire.

### Changed
- `API.md` §1–§3 now describe the implemented envelope and `get_app_info` instead
  of a proposed contract; error codes are documented in their serialized
  `snake_case` form.
- `build.md` §4 no longer marks `cargo fmt`/`cargo clippy` as verified without the
  components being installed. That claim was incorrect and is corrected, not
  quietly removed.
- `README.md` rewritten around the actual implementation state, with a
  documentation map and an honest capability table.
- `docs/plan.md`: authoritative plan moved from the repository root; `plan.md`
  is now a pointer.

### Documentation
- Full documentation set: architecture, product, security, operations, and AI
  workflow documents (`ARCHITECTURE.md`, `PRD.md`, `TECH_STACK.md`, `DATABASE.md`,
  `API.md`, `SECURITY.md`, `CONTRIBUTING.md`, `TESTING.md`, `AGENTS.md`,
  `AI_*.md`, and `docs/*`).
- `build.md`: command reference for the current toolchain, now including the new
  test/lint/format scripts.

### Fixed
- Toolchain documentation claimed `rustfmt`/`clippy` were available and verified;
  neither component was installed. Both are now installed and declared in
  `rust-toolchain.toml`.

### Security
- No network capability, no telemetry, no privilege escalation added.
- `get_app_info` returns no machine-identifying data.
- 10 moderate advisories exist in the **dev-only** lint toolchain (`minimatch`
  ReDoS). `npm audit --omit=dev` is clean; see `docs/KNOWN_ISSUES.md` KI-036.

---

## [0.1.0] — 2026-11-06

Initial scaffold. **Not a functional system scanner.**

### Added
- Tauri 2 desktop application (`src-tauri/`), Rust edition 2024.
- React 19 + TypeScript + Vite renderer (`src/`).
- Welcome screen with design tokens and a preview footer (`src/App.tsx`,
  `src/App.css`).
- Strict CSP and a minimal Tauri capability set (`core:default` only).
- NSIS x64 bundle configuration and generated installer.
- Release profile: LTO, `codegen-units = 1`, `opt-level = 3`, `panic = "abort"`,
  `strip = true`.

### Known limitations
- No IPC commands, system collection, process list, storage scan, persistence,
  diagnostics, tests, or CI.
- Installer is unsigned.
- No license selected.

[Unreleased]: #unreleased
[0.1.0]: #010--2026-11-06
