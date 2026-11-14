# Tech Stack

> **Status:** Current stack (implemented) + planned additions · **Last updated:** 2026-11-14
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
| Tauri JS API | `@tauri-apps/api` | `^2` | Implemented (`src/services/ipc.ts` is the only caller) |
| Styles | Tailwind CSS v4 (`@tailwindcss/vite`) + CSS variables | `4.3.5` | Implemented (`src/styles/globals.css`) |
| UI primitives | shadcn-style local components (`cva`, `clsx`, `tailwind-merge`, Radix `Slot`, `lucide-react`) | see `package.json` | Implemented (`src/components/ui/`) |
| Persistence | SQLite | — | **Proposed** |
| DB access | `rusqlite` or `sqlx` | — | **Undecided** (B-020) |
| Serialization | `serde` / `serde_json` | `1.0.229` / `1.0.154` | Implemented |
| Errors | `thiserror` | `2.0.21` | Implemented |
| Logging | `tracing` + `tracing-subscriber`, size-based file rotation (`src-tauri/src/logging.rs`) | `0.1.44` / `0.3.23` | Implemented |
| Hashing | `sha2` / `blake3` | — | **Proposed** |
| Windows bindings | `windows` / `windows-sys` | — | **Proposed** |
| Frontend tests | Vitest + Testing Library | `5.1.0` / `16.3.3` | Implemented |
| E2E | WebDriver / tauri-driver | — | **Proposed** |
| Installer | NSIS (Tauri bundler) | — | Implemented |
| CI | GitHub Actions (Windows) | — | **Workflow committed, never run** |

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

### Tailwind CSS v4 + shadcn-style primitives

- Tailwind v4's Vite plugin performs the CSS-first configuration: tokens live in
  `@theme` / `@theme inline` inside `src/styles/globals.css`, so the palette and
  radius scale have one source of truth instead of a JS config file.
- Components are **copied into the repository** (`src/components/ui/`), not
  imported from a runtime library. There is no hidden theme provider to upgrade,
  and the whole styling surface stays auditable.
- `class-variance-authority` + `tailwind-merge` give the shared `Button` typed
  variants without a prop-styling free-for-all; Radix `Slot` is the only Radix
  dependency and is used for `asChild`.
- Icons come from `lucide-react`; they are decorative SVGs with `aria-hidden`,
  so a screen reader never reads an icon name.

**Trade-offs accepted:** utility classes in markup, a build-time dependency on
Tailwind, and the need to keep tokens (not arbitrary colors) as the palette
source. In exchange the UI ships static CSS and no styling runtime.

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

### Local logs

- `tracing` events go to `app_log_dir()/system-scanner.log`, rotated at 10 MB with
  five files maximum (50 MB bound).
- Commands log their stable code and correlation ID on failure; request payloads
  and inputs are never logged, and diagnostic detail passes best-effort path
  redaction.
- Logs never leave the machine. The in-app diagnostics panel builds a redacted
  preview on request and copies it; saved-file export is still pending (F-092).

## 3. Rejected alternatives

| Option | Why rejected |
|---|---|
| WPF / WinUI 3 | Strong Windows integration, but weaker cross-cutting UI patterns, heavier XAML surface, and a smaller pool of contributors for this project |
| Qt / C++ | Excellent native performance; higher memory-safety risk and slower iteration |
| Electron + React | Footprint and renderer privilege surface |
| Next.js | Server rendering and routing server are irrelevant locally |
| Microservices | No deployment or scaling requirement justifies them |
| Electron store / Redux for scan state | Rust is authoritative for job state; duplicating it in the UI causes drift |
| CSS-in-JS runtime (styled-components, Emotion) | Ships a runtime and a second theming model; Tailwind v4 emits static CSS and shares one token layer |
| A full component kit (MUI, Ant Design) | Large surface and its own theme system for a compact multi-panel desktop UI |
| A second HTTP client | One client, opt-in only; see [`docs/NETWORKING.md`](docs/NETWORKING.md) |

## 4. Version policy

- **Rust:** stable channel; edition 2024 requires a recent toolchain. Pin the
  exact version in CI.
- **Node:** LTS; `package-lock.json` is committed. Do not upgrade majors casually.
- **Tauri:** stay on the latest 2.x patch line. A 3.x move requires an ADR and a
  capability/CSP review.
- **TypeScript:** currently `~6.0.3` (patch updates only within the minor).
- **Frontend runtime deps:** Tailwind CSS v4, its Vite plugin, and the small
  shadcn-style primitive set (`class-variance-authority`, `clsx`,
  `tailwind-merge`, `@radix-ui/react-slot`, `lucide-react`) are accepted as the
  styling contract. Any further component library requires an ADR and a
  dependency entry; do not add a second styling system.

## 5. Tooling status

| Tool | Purpose | Status |
|---|---|---|
| ESLint + `eslint-plugin-react-hooks` | Catch hook and unused-code issues | Configured; `npm run lint` passes |
| Prettier | Consistent formatting | Configured; `npm run format:check` passes (code only, Markdown/CSS excluded by design) |
| `rustfmt.toml` / `clippy.toml` | Explicit Rust style | Defaults are in use; `cargo fmt --check` and `clippy -D warnings` pass |
| Vitest | Unit tests | Configured; `npm test` passes (18 tests) |
| `rust-toolchain.toml` | Reproducible toolchain | Committed; pins `stable` + `rustfmt`/`clippy` (not an exact compiler version; B-011) |
| GitHub Actions | Windows CI | Workflow committed (`.github/workflows/ci.yml`); never executed (B-092) |
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
