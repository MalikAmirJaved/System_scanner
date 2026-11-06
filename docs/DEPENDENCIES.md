# Dependencies

> **Status:** Inventory of direct dependencies as resolved on 2026-11-06.
> License review and advisory gating are **not yet automated**.
> **Related:** [`../TECH_STACK.md`](../TECH_STACK.md), [`DECISIONS.md`](DECISIONS.md)

---

## 1. Runtime JavaScript dependencies

| Package | Declared | Resolved | License | Purpose |
|---|---|---|---|---|
| `react` | `^19.1.0` | `19.3.1` | MIT | UI library |
| `react-dom` | `^19.1.0` | `19.3.1` | MIT | DOM renderer |
| `@tauri-apps/api` | `^2` | `2.12.4` | Apache-2.0 OR MIT | Typed IPC/event API (not yet used) |

Production tree: 5 packages total including transitive (`scheduler`, etc.).

## 2. Development JavaScript dependencies

| Package | Declared | Resolved | License | Purpose |
|---|---|---|---|---|
| `@tauri-apps/cli` | `^2` | `2.12.4` | Apache-2.0 OR MIT | Dev/build/bundle orchestration |
| `vite` | `^8.0.16` | `8.3.6` | MIT | Dev server + bundler |
| `@vitejs/plugin-react` | `^6.0.2` | `6.2.1` | MIT | React transform/HMR |
| `typescript` | `~6.0.3` | `6.0.3` | Apache-2.0 | Type checking |
| `@types/react` | `^19.1.8` | `19.3.0` | MIT | React types |
| `@types/react-dom` | `^19.1.6` | `19.3.0` | MIT | React DOM types |

## 3. Rust dependencies

| Crate | Declared | Resolved | License | Purpose |
|---|---|---|---|---|
| `tauri` | `2` | `2.12.5` | MIT OR Apache-2.0 | Desktop runtime, IPC, windowing |
| `tauri-build` (build) | `2` | see `Cargo.lock` | MIT OR Apache-2.0 | Build-time codegen |

- `Cargo.lock` currently resolves **407 packages** (full transitive tree).
- The workspace crate `system-scanner` declares **no license field**, which means
  all rights reserved by default. This must be fixed before distribution.

## 4. Toolchain dependencies (not bundled)

| Tool | Version verified | Notes |
|---|---|---|
| Node.js | 24.20.0 | LTS line preferred; pin in CI |
| npm | 11.19.0 | `npm ci` in CI |
| Rust stable | 1.99.1 (2026-10-13) | Edition 2024 |
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
| `serde`, `serde_json` | DTO serialization | First IPC command |
| `thiserror` | Typed errors | First error enum |
| `tracing`, `tracing-subscriber`, `tracing-appender` | Logging | Release 0 diagnostics |
| `rusqlite` **or** `sqlx` | SQLite | ADR-005 |
| `walkdir` **or** custom traversal | Directory enumeration | Decide after prototype |
| `sha2`, `blake3` | Hashing | Release 1 |
| `windows` / `windows-sys` | Win32 bindings | Release 0 adapters |
| `tempfile` (dev) | Test fixtures | First test |
| `vitest`, `@testing-library/react`, `jsdom` | Frontend tests | Test runner |
| `zod` (optional) | Runtime DTO validation | Only if hand-written guards prove insufficient |

Each addition requires an entry here **in the same change**, plus a justification.

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
| npm audit | Available, not in CI |
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
| MIT | react, react-dom, vite, plugin-react, @types/* |
| Apache-2.0 | typescript |
| Apache-2.0 OR MIT | @tauri-apps/api, @tauri-apps/cli, tauri |

**This is not a legal review.** A full transitive license audit is required before
distribution — see [`COMPLIANCE.md`](COMPLIANCE.md).

## 11. Removal criteria

Remove a dependency when:

- its feature is removed;
- the standard library or an existing crate covers it;
- it is unmaintained or has an unfixed advisory;
- it duplicates another dependency's role.
