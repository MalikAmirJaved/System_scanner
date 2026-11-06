# Tech Stack

> **Status:** Current stack (implemented) + planned additions · **Last updated:** 2026-11-06
>
> Versions below were read from `package.json`, `src-tauri/Cargo.toml`, and the
> installed toolchains. Anything marked **Proposed** is not a dependency yet.

---

## 1. Summary

| Layer | Technology | Version | Status |
|---|---|---|---|
| Desktop shell | Tauri | `tauri 2`, `tauri-cli 2.12.4` | Implemented |
| Core language | Rust | stable `1.99.1`, edition 2024 | Implemented |
| UI library | React | `^19.1.0` | Implemented |
| UI language | TypeScript | `~6.0.3` (strict) | Implemented |
| Bundler / dev server | Vite | `^8.0.16` | Implemented |
| React plugin | `@vitejs/plugin-react` | `^6.0.2` | Implemented |
| Tauri JS API | `@tauri-apps/api` | `^2` | Implemented (unused so far) |
| Styles | Plain CSS + custom properties | — | Implemented |
| Persistence | SQLite | — | **Proposed** |
| DB access | `rusqlite` or `sqlx` | — | **Undecided** |
| Serialization | `serde` / `serde_json` | — | **Proposed** (transitively present) |
| Errors | `thiserror` | — | **Proposed** |
| Logging | `tracing` + `tracing-subscriber` | — | **Proposed** |
| Hashing | `sha2` / `blake3` | — | **Proposed** |
| Windows bindings | `windows` / `windows-sys` | — | **Proposed** |
| Frontend tests | Vitest + Testing Library | — | **Proposed** |
| E2E | WebDriver / tauri-driver | — | **Proposed** |
| Installer | NSIS (Tauri bundler) | — | Implemented |
| CI | GitHub Actions (Windows) | — | **Proposed** |

## 2. Reasons for the core choices

### Tauri 2 (not Electron)

| Criterion | Tauri 2 | Electron |
|---|---|---|
| Installer size | ~1.4 MB (verified here) | 80–150 MB |
| Memory overhead | Uses OS WebView2 | Bundles Chromium |
| OS access | Rust commands, narrow IPC | Full Node in renderer (large surface) |
| Security model | Capability-based permissions | Manual, easy to over-grant |
| Ecosystem maturity | Good, smaller | Very large |

**Decision:** Tauri. The product's trust proposition depends on a small, auditable
binary and a narrow OS boundary. Electron's Node-in-renderer model is the opposite
of what a system-inspection tool should expose.

**Trade-offs accepted:** WebView2 dependency on Windows; fewer ready-made plugins;
Rust ramp-up cost; debugging spans two languages.

### React 19 + TypeScript + Vite (not Next.js)

- This is a **local desktop UI**, not a website. Next.js's SSR, routing server,
  and image pipeline add no value and complicate the Tauri build.
- Vite's dev server and Tauri's `beforeDevCommand`/`frontendDist` integration are
  the documented happy path.
- TypeScript `strict` is enabled; `noUnusedLocals`/`noUnusedParameters` keep IPC
  DTO adapters honest.

### Rust

- Memory safety without a GC is material when the app must stay responsive while
  enumerating millions of directory entries.
- Direct, well-typed access to Win32 via the `windows` crate.
- `panic = "abort"` + `strip` + LTO in release (`Cargo.toml`) keeps the binary small
  and predictable.

### SQLite (proposed)

- Single-user, local, embedded, transactional, zero-admin.
- Handles the expected history volume with correct indexes and retention.
- Backup is a file copy or `VACUUM INTO`.

**Open:** access layer. `rusqlite` (synchronous, simple, explicit) vs `sqlx`
(async, compile-time query checking, heavier). Decide before the first migration;
record in [`docs/DECISIONS.md`](docs/DECISIONS.md).

## 3. Rejected alternatives

| Option | Why rejected |
|---|---|
| WPF / WinUI 3 | Strong Windows integration, but weaker cross-cutting UI patterns, heavier XAML surface, and a smaller pool of contributors for this project |
| Qt / C++ | Excellent native performance; higher memory-safety risk and slower iteration |
| Electron + React | Footprint and renderer privilege surface |
| Next.js | Server rendering and routing server are irrelevant locally |
| Microservices | No deployment or scaling requirement justifies them |
| Electron store / Redux for scan state | Rust is authoritative for job state; duplicating it in the UI causes drift |
| A second HTTP client | One client, opt-in only; see [`docs/NETWORKING.md`](docs/NETWORKING.md) |

## 4. Version policy

- **Rust:** stable channel; edition 2024 requires a recent toolchain. Pin the
  exact version in CI.
- **Node:** LTS; `package-lock.json` is committed. Do not upgrade majors casually.
- **Tauri:** stay on the latest 2.x patch line. A 3.x move requires an ADR and a
  capability/CSP review.
- **TypeScript:** currently `~6.0.3` (patch updates only within the minor).
- **Frontend runtime deps:** keep to React, React DOM, and `@tauri-apps/api` until
  a feature proves it needs more.

## 5. Tooling not yet configured

| Tool | Purpose | Blocker |
|---|---|---|
| ESLint + `eslint-plugin-react-hooks` | Catch hook and unused-code issues | No config |
| Prettier | Consistent formatting | No config |
| `rustfmt.toml` / `clippy.toml` | Explicit Rust style | Defaults are in use |
| Vitest | Unit tests | No runner |
| GitHub Actions | Windows CI | No workflow |
| `cargo-audit`, Dependabot | Advisory monitoring | Not enabled |
| `cargo-deny` | License/advisory/allowlist gate | Not enabled |

Adding any of these is a documented change (see [`build.md`](build.md) and
[`docs/DEPENDENCIES.md`](docs/DEPENDENCIES.md)).

## 6. Platform requirements

| Requirement | Detail |
|---|---|
| OS | Windows 10/11 x64 (formal matrix **undecided**) |
| WebView2 | Evergreen runtime |
| Installer | NSIS x64, unsigned today |
| Rust target | `x86_64-pc-windows-msvc` |
| Build host | Windows with MSVC C++ workload + Windows SDK |

## 7. Related

- [`ARCHITECTURE.md`](ARCHITECTURE.md) — how these pieces fit
- [`docs/DEPENDENCIES.md`](docs/DEPENDENCIES.md) — full dependency inventory
- [`docs/DECISIONS.md`](docs/DECISIONS.md) — ADRs behind these choices
- [`build.md`](build.md) — commands
