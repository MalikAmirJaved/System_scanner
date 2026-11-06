# Build & Command Reference

> **Status:** Living document · **Last verified:** 2026-11-06 · **App version:** 0.1.0
>
> This is the single source of truth for every command used in this repository.
> Other documents must link here instead of repeating commands, so there is only
> one place to update when the toolchain changes.

Everything marked **Verified** was executed in this repository on the date above.
Everything marked **Planned** does not work yet — the script, config, or code does
not exist. Do not present planned commands as if they were available.

---

## 1. Toolchain

| Tool | Required | Verified locally | Notes |
|---|---|---|---|
| Windows | 10/11 x64 | — | Primary target; not yet formally supported/tested |
| Node.js | ≥ 20 (Tauri 2 baseline) | `v24.20.0` | LTS recommended |
| npm | ≥ 10 | `11.19.0` | Lockfile is `package-lock.json` |
| Rust (stable) | ≥ 1.77 (Tauri 2 baseline) | `1.99.1` (2026-10-13) | Edition 2024 requires a recent stable |
| Tauri CLI | 2.x | `2.12.4` | Installed as a dev dependency, not globally |
| WebView2 Runtime | Evergreen | present | Bundled by Windows 11; installer bootstraps on Windows 10 |
| MSVC Build Tools | VS 2022 + "Desktop development with C++" + Windows SDK | present | Required to link the Rust binary |

> The versions in the "Verified locally" column are informational. Pin exact
> tool versions in CI (see `docs/DEVOPS.md`) before declaring a supported matrix.

## 2. Bootstrap

```powershell
# From the repository root
npm install                 # Verified: installs from package-lock.json
```

Rust dependencies are resolved on the first Cargo invocation:

```powershell
cd src-tauri
cargo fetch                 # Verified: populates the Cargo registry cache
```

## 3. Develop

```powershell
# Full desktop app: Vite dev server + Rust backend, hot reload
npm run tauri dev           # Verified

# Frontend only, in a browser (no Tauri APIs available)
npm run dev                 # Verified, serves http://localhost:1420
```

Notes:

- Vite is pinned to port `1420` with `strictPort: true`; if the port is busy the
  dev server fails instead of silently moving (see `vite.config.ts`).
- HMR uses port `1421` when `TAURI_DEV_HOST` is set (remote/device debugging).
- `src-tauri` is excluded from Vite's file watcher.
- The first `tauri dev` compiles the full Rust dependency tree and can take
  several minutes. Later builds are incremental.

## 4. Quality gates

```powershell
# --- Frontend ---
npm run build               # Verified: `tsc` type-check (noEmit) + `vite build`
npx tsc --noEmit            # Verified: type-check only
npx tsc --noEmit -p tsconfig.node.json   # Verified: config-file type-check

# --- Rust ---
cd src-tauri
cargo check                 # Verified
cargo fmt --all -- --check  # Verified (no rustfmt.toml: defaults apply)
cargo clippy --all-targets -- -D warnings   # Verified
cargo test                  # Verified: currently 0 tests, compiles the harness
```

| Gate | Command | Status |
|---|---|---|
| TypeScript type-check | `npm run build` | Available |
| Frontend unit tests | `npm test` | **Planned** — no test runner configured |
| Frontend lint | `npm run lint` | **Planned** — no ESLint config |
| Rust format | `cargo fmt --all -- --check` | Available |
| Rust lint | `cargo clippy --all-targets -- -D warnings` | Available |
| Rust tests | `cargo test` | Available (no tests yet) |
| Dependency audit (JS) | `npm audit --omit=dev` | Available |
| Dependency audit (Rust) | `cargo audit` | **Planned** — `cargo-audit` not installed |

## 5. Build the desktop app

```powershell
# Production frontend build only -> dist/
npm run build               # Verified

# Full release build + NSIS installer
npm run tauri build         # Verified

# Debug bundle (faster compile, larger binary, devtools on)
npm run tauri build -- --debug

# Build without bundling (binary only)
npm run tauri build -- --no-bundle
```

### Artifacts

| Artifact | Path | Verified |
|---|---|---|
| Executable | `src-tauri/target/release/system-scanner.exe` | Yes |
| NSIS installer | `src-tauri/target/release/bundle/nsis/System Scanner_0.1.0_x64-setup.exe` | Yes (~1.4 MB) |
| Frontend assets | `dist/` | Yes |

The bundle target is `nsis` only (`src-tauri/tauri.conf.json`). MSI/WiX and
portable builds are **not configured**. The installer is **unsigned** — Windows
SmartScreen will warn until a signing certificate is configured (see
`docs/DEPLOYMENT.md`).

### Release profile

`src-tauri/Cargo.toml` sets `codegen-units = 1`, `lto = true`, `opt-level = 3`,
`panic = "abort"`, `strip = true`. Consequence: a Rust panic aborts the process
instead of unwinding, so panic handling must never be the recovery mechanism.

## 6. Cleanup

```powershell
# Frontend artifacts
Remove-Item -Recurse -Force dist -ErrorAction SilentlyContinue

# Rust build artifacts (can be several GB)
cd src-tauri; cargo clean

# Full reset (keeps source and lockfiles)
Remove-Item -Recurse -Force node_modules, dist, src-tauri\target -ErrorAction SilentlyContinue
npm install
```

## 7. Local runtime state

The app currently persists nothing. When persistence lands, data will live under
`%LOCALAPPDATA%\com.systemscanner.desktop\` (Tauri app-data directory) — see
`docs/CONFIGURATION.md` for the planned layout and `docs/BACKUP_POLICY.md` for
backup implications.

```powershell
# Planned: inspect app data after persistence is implemented
explorer "$env:LOCALAPPDATA\com.systemscanner.desktop"
```

## 8. Planned commands (do not run yet)

These are reserved names for scripts that do not exist. Adding them requires a
change to `package.json` and an update to this file in the same commit.

```powershell
npm test                    # Planned: Vitest unit tests
npm run test:e2e            # Planned: Tauri/WebDriver E2E
npm run lint                # Planned: ESLint
npm run format              # Planned: Prettier
npm run tauri icon <path>   # Available today via CLI, not scripted
```

## 9. Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| `Port 1420 is already in use` | Stale dev server | Stop the other process (`Get-NetTCPConnection -LocalPort 1420`) or reboot the terminal |
| `link.exe not found` / `msvc` errors | Missing MSVC C++ workload | Install VS Build Tools with "Desktop development with C++" |
| `error: linker link.exe not found` after Rust upgrade | Toolchain mismatch | `rustup update stable` then `cargo clean` |
| Blank window with `tauri dev` | `frontendDist`/`devUrl` mismatch | Confirm Vite is on `1420` and `beforeDevCommand` is running |
| Installer build fails at NSIS download | Network restriction | Pre-seed NSIS cache or build with `--no-bundle` |
| SmartScreen warning on install | Unsigned binary | Expected until code signing is configured |
| `cargo clippy` warnings after dependency bump | New lints | Fix or explicitly allow with a comment and justification |

## 10. Windows environment quirks

- Run PowerShell as a normal user. Elevation is **not** required to build or run.
- Long paths: enable `LongPathsEnabled` if the repository is deeply nested.
- Antivirus can lock freshly built `.exe` files; exclude `src-tauri/target` if
  builds intermittently fail.
- Release binaries are stripped; attach a debug build when diagnosing crashes.

## 11. Change log for this document

| Date | Change |
|---|---|
| 2026-11-06 | Initial verified command reference (scaffold state) |
