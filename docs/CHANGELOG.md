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
- Full documentation set: architecture, product, security, operations, and AI
  workflow documents (`ARCHITECTURE.md`, `PRD.md`, `TECH_STACK.md`, `DATABASE.md`,
  `API.md`, `SECURITY.md`, `CONTRIBUTING.md`, `TESTING.md`, `AGENTS.md`,
  `AI_*.md`, and `docs/*`).
- `build.md`: verified command reference for the current toolchain.
- `docs/plan.md`: authoritative plan moved from the repository root; `plan.md`
  is now a pointer.

### Changed
- `README.md` rewritten around the actual implementation state, with a
  documentation map and an honest capability table.

### Fixed
- Nothing yet.

### Security
- No change. No network capability, no telemetry, no privilege escalation.

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
