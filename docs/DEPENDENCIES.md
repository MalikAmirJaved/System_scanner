# Dependencies

> **Status:** Inventory of direct dependencies as resolved on 2026-11-16.
> License review and advisory gating are **not yet automated**.
> **Related:** [`../TECH_STACK.md`](../TECH_STACK.md), [`DECISIONS.md`](DECISIONS.md)

---

## 1. Runtime JavaScript dependencies

| Package | Declared | Resolved | License | Purpose |
|---|---|---|---|---|
| `react` | `^19.1.0` | `19.3.1` | MIT | UI library |
| `react-dom` | `^19.1.0` | `19.3.1` | MIT | DOM renderer |
| `@tauri-apps/api` | `^2` | `2.12.4` | Apache-2.0 OR MIT | Typed IPC/event API — used by `src/services/ipc.ts` |
| `class-variance-authority` | `^0.7.1` | `0.7.1` | Apache-2.0 | Typed component variants for the shared `Button` |
| `clsx` | `^2.1.1` | `2.1.1` | MIT | Conditional class names in `cn()` |
| `tailwind-merge` | `^3.7.0` | `3.7.0` | MIT | Tailwind conflict resolution in `cn()` |
| `@radix-ui/react-slot` | `^1.5.0` | `1.5.0` | MIT | `asChild` composition for `Button` |
| `lucide-react` | `^1.72.0` | `1.72.0` | ISC | Icon SVGs (decorative, `aria-hidden`) |

Production tree: 13 packages total including transitive (`scheduler`, `react-is`,
etc.).

## 2. Development JavaScript dependencies

| Package | Declared | Resolved | License | Purpose |
|---|---|---|---|---|
| `@tauri-apps/cli` | `^2` | `2.12.4` | Apache-2.0 OR MIT | Dev/build/bundle orchestration |
| `vite` | `^8.0.16` | `8.3.6` | MIT | Dev server + bundler |
| `@vitejs/plugin-react` | `^6.0.2` | `6.2.1` | MIT | React transform/HMR |
| `tailwindcss` | `^4.3.5` | `4.3.5` | MIT | Utility CSS engine + `@theme` tokens |
| `@tailwindcss/vite` | `^4.3.5` | `4.3.5` | MIT | Tailwind integration for Vite |
| `typescript` | `~6.0.3` | `6.0.3` | Apache-2.0 | Type checking |
| `@types/react` | `^19.1.8` | `19.3.0` | MIT | React types |
| `@types/react-dom` | `^19.1.6` | `19.3.0` | MIT | React DOM types |
| `vitest` | `^5.1.0` | `5.1.0` | MIT | Frontend unit test runner |
| `jsdom` | `^31.0.0` | `31.0.0` | MIT | DOM environment for tests |
| `@testing-library/react` | `^16.3.3` | `16.3.3` | MIT | Component test rendering |
| `@testing-library/jest-dom` | `^7.0.1` | `7.0.1` | MIT | DOM assertions (`toBeInTheDocument`) |
| `eslint` | `^10.14.0` | `10.14.0` | MIT | Linter |
| `@eslint/js` | `^10.0.1` | `10.0.1` | MIT | ESLint recommended rules |
| `typescript-eslint` | `^8.72.1` | `8.72.1` | MIT | TypeScript parser + rules |
| `eslint-plugin-react-hooks` | `^7.1.1` | `7.1.1` | MIT | Hook rules (incl. `set-state-in-effect`) |
| `eslint-plugin-react-refresh` | `^0.5.7` | `0.5.7` | MIT | HMR-safety warnings |
| `globals` | `^17.14.0` | `17.14.0` | MIT | Browser globals for ESLint |
| `prettier` | `^3.9.10` | `3.9.10` | MIT | Formatter |

Dev-only advisories: 10 moderate (`minimatch` ReDoS, reachable only through the
lint toolchain). Production tree is clean under `npm audit --omit=dev`.
See [`KNOWN_ISSUES.md`](KNOWN_ISSUES.md) KI-036.

## 3. Rust dependencies

| Crate | Declared | Resolved | License | Purpose |
|---|---|---|---|---|
| `tauri` | `2` | `2.12.5` | MIT OR Apache-2.0 | Desktop runtime, IPC, windowing |
| `tauri-build` (build) | `2` | see `Cargo.lock` | MIT OR Apache-2.0 | Build-time codegen |
| `serde` | `1.0.229` (`derive`) | `1.0.229` | MIT OR Apache-2.0 | DTO/error serialization |
| `serde_json` | `1.0.154` | `1.0.154` | MIT OR Apache-2.0 | Envelope and DTO tests |
| `thiserror` | `2.0.21` | `2.0.21` | MIT OR Apache-2.0 | `AppError` `Error` impl |
| `tracing` | `0.1` | `0.1.44` | MIT | Structured events (already transitive via Tauri; now declared directly) |
| `tracing-subscriber` | `0.3` (`default-features = false`, `fmt`, `std`) | `0.3.23` | MIT | File subscriber for `tracing` |
| `rusqlite` | `0.43.0` (`bundled`) | `0.43.0` | MIT | SQLite access layer for `adapters/persistence` (ADR-018) |
| `windows-sys` (Windows target only) | `0.61.2` | `0.61.2` | MIT OR Apache-2.0 | Win32 registry, SCM/service, memory, volume, adapter, process, diagnostics, and last-resort dialog bindings for `adapters/windows` (B-002, B-003, B-006, B-012, B-043) |

`windows-sys` features enabled: `Win32_Foundation`,
`Win32_NetworkManagement_IpHelper`, `Win32_NetworkManagement_Ndis`,
`Win32_Networking_WinSock`, `Win32_Security`, `Win32_Storage_FileSystem`,
`Win32_System_Diagnostics_ToolHelp`, `Win32_System_ProcessStatus`,
`Win32_System_Registry`, `Win32_System_SystemInformation`,
`Win32_System_Services`, `Win32_System_Threading`, `Win32_System_WindowsProgramming`,
`Win32_UI_WindowsAndMessaging`. The security feature adds token-elevation reads for
diagnostics, `Win32_System_Services` adds the read-only SCM enumeration and
per-service configuration queries for B-043, and the UI feature adds the
`MessageBoxW` startup dialog; none adds a crate. The dependency is target-scoped so `domain/` and its tests still build on
non-Windows hosts.

- `Cargo.lock` currently resolves **420 packages** (full transitive tree).
- Logging adds `tracing-subscriber` plus four small transitive crates; `tracing`
  itself was already in the tree through Tauri.
- Diagnostics added no package: token elevation and identity reads reuse
  `windows-sys`, and timestamp formatting/redaction are in-process.
- The startup dialog added no package: `MessageBoxW` reuses `windows-sys`, and no
  dialog plugin is installed.
- Persistence (B-020) added `rusqlite` and eight packages: `libsqlite3-sys`
  (SQLite C sources, MIT), `cc` (build-time C compiler driver, MIT OR Apache-2.0),
  `vcpkg` (MIT OR Apache-2.0), `hashlink`, `rsqlite-vfs`, `sqlite-wasm-rs`,
  `fallible-iterator`, `fallible-streaming-iterator`. Building now needs a C
  toolchain — the MSVC Build Tools already required for this project. The
  `bundled` feature compiles SQLite into the binary, so no system SQLite is
  required at runtime.
- The workspace crate `system-scanner` declares **no license field**, which means
  all rights reserved by default. This must be fixed before distribution.

## 4. Toolchain dependencies (not bundled)

| Tool | Version verified | Notes |
|---|---|---|
| Node.js | 24.20.0 | LTS line preferred; pin in CI |
| npm | 11.19.0 | `npm ci` in CI |
| Rust stable | 1.99.1 (2026-10-13) | Edition 2024; `rust-toolchain.toml` pins the channel |
| rustfmt / clippy | 1.10.0 / 0.1.99 | `rustup` components, declared in `rust-toolchain.toml` |
| C compiler (MSVC) | present | Required to build bundled SQLite |
| Tauri CLI | 2.12.4 | via devDependency |
| WebView2 | Evergreen | OS runtime dependency |
| MSVC Build Tools | present | Windows SDK + C++ workload |

## 5. Platform dependencies

| Dependency | Required | Failure behavior |
|---|---|---|
| Windows 10/11 x64 | Yes | Not supported elsewhere |
| WebView2 Runtime | Yes | Installer bootstraps; clear error if missing |
| NTFS (for file IDs) | Preferred | Fall back to path/size identity |
| Visual C++ runtime | via MSVC | Bundled by OS/redistributable |

## 6. Planned additions (not installed)

| Dependency | Would be used for | Gate |
|---|---|---|
| `walkdir` **or** custom traversal | Directory enumeration | **Decided: custom `std` traversal** (B-022); `walkdir` not added |
| `sha2`, `blake3` | Hashing | Release 1 (B-028) |
| `tempfile` (dev) | Test fixtures | First filesystem test |
| `zod` (optional) | Runtime DTO validation in the renderer | Only if hand-written guards prove insufficient (B-090) |

Each addition requires an entry here **in the same change**, plus a justification.
Rotation is implemented in-process (`logging.rs`), so `tracing-appender` is not
planned.

## 7. Dependency policy

1. **Minimum viable surface.** Every dependency must enable a feature that cannot
   be reasonably built otherwise.
2. **One capability, one library.** No second HTTP client, state manager, date
   library, or logging facade.
3. **Pinned lockfiles.** `package-lock.json` and `Cargo.lock` are committed.
4. **No wildcard versions** in manifests.
5. **License compatibility** checked before adding; no copyleft in the distributed
   binary without a legal decision.
6. **Maintenance check**: recent release, responsive issues, no abandoned project.
7. **Advisory check** before each release.
8. **No post-install scripts** from unvetted packages in the frontend tree.

## 8. Update cadence

| Class | Cadence |
|---|---|
| Security advisories | Immediately, patch release |
| Tauri patch/minor | Monthly or on a security fix |
| React/Vite/TypeScript | Per minor, after CI is green |
| Rust stable | Quarterly, verified with full test run |
| Dev-only tooling | As needed |

Major upgrades require an ADR and a migration note in [`CHANGELOG.md`](CHANGELOG.md).

## 9. Supply chain

| Control | Status |
|---|---|
| Lockfiles committed | ✅ |
| npm audit | Available; `--omit=dev` clean; not yet in CI |
| `cargo audit` | Not installed |
| `cargo-deny` (licenses/advisories/bans) | Not configured |
| SBOM (CycloneDX) | Not generated |
| Artifact signing | Not configured |
| Provenance/checksums | Not published |

Target state: CI fails on a high/critical advisory, publishes an SBOM per release,
and archives checksums.

## 10. License summary (current tree)

| License | Packages (direct) |
|---|---|
| MIT | react, react-dom, vite, plugin-react, vitest, jsdom, testing-library, eslint stack, prettier, tailwindcss, @tailwindcss/vite, clsx, tailwind-merge, @radix-ui/react-slot, rusqlite, libsqlite3-sys (bundled SQLite), tracing, tracing-subscriber, @types/* |
| Apache-2.0 | typescript, class-variance-authority |
| ISC | lucide-react |
| Apache-2.0 OR MIT | @tauri-apps/api, @tauri-apps/cli, tauri, serde, serde_json, thiserror, cc, vcpkg |

SQLite itself is public domain. The bundled build links it into the binary; that
is compatible with distribution, but a full transitive license audit is still
required.

**This is not a legal review.** A full transitive license audit is required before
distribution — see [`COMPLIANCE.md`](COMPLIANCE.md).

## 11. Removal criteria

Remove a dependency when:

- its feature is removed;
- the standard library or an existing crate covers it;
- it is unmaintained or has an unfixed advisory;
- it duplicates another dependency's role.
