# Configuration

> **Status:** Only build-time Tauri configuration exists. There is no runtime
> settings system, no config file, and no user preferences. **Last updated:** 2026-11-06

---

## 1. Current configuration surface

### 1.1 `src-tauri/tauri.conf.json`

| Key | Value | Effect |
|---|---|---|
| `productName` | `System Scanner` | Window title, artifact name |
| `version` | `0.1.0` | App version (must match `package.json`, `Cargo.toml`) |
| `identifier` | `com.systemscanner.desktop` | App-data directory, Windows identity |
| `build.devUrl` | `http://localhost:1420` | Dev webview origin |
| `build.frontendDist` | `../dist` | Release assets |
| `build.beforeDevCommand` | `npm run dev` | Dev server |
| `build.beforeBuildCommand` | `npm run build` | Frontend build |
| `app.windows[0]` | 960×720, min 420×560, centered | Window |
| `app.security.csp` | `default-src 'self'; img-src 'self' asset: data:; style-src 'self' 'unsafe-inline'; font-src 'self' data:` | Renderer restrictions |
| `bundle.targets` | `["nsis"]` | Installer format |
| `bundle.icon` | 4 icon paths | App + installer icons |

Never loosen `csp` or add capabilities for convenience. See
[`../AI_SECURITY_RULES.md`](../AI_SECURITY_RULES.md).

### 1.2 `src-tauri/capabilities/default.json`

```json
{
  "identifier": "default",
  "windows": ["main"],
  "permissions": ["core:default"]
}
```

The capability list must contain only permissions required by an implemented
feature. Adding one is a security change.

### 1.3 `vite.config.ts`

| Setting | Value | Reason |
|---|---|---|
| `server.port` | `1420` | Tauri `devUrl` contract |
| `server.strictPort` | `true` | Fail instead of drifting |
| `server.host` | `TAURI_DEV_HOST` or false | Remote debugging only |
| `server.hmr` | ws on `1421` when host set | Device debugging |
| `server.watch.ignored` | `**/src-tauri/**` | Avoid rebuild loops |
| `clearScreen` | `false` | Preserve Rust errors |

### 1.4 `tsconfig.json`

`strict`, `noUnusedLocals`, `noUnusedParameters`, `noFallthroughCasesInSwitch`,
`noEmit`, `moduleResolution: bundler`, `jsx: react-jsx`. Do not weaken these to
silence an error.

### 1.5 `src-tauri/Cargo.toml`

Release profile: `codegen-units=1`, `lto=true`, `opt-level=3`, `panic="abort"`,
`strip=true`. `panic="abort"` is a behavioral constraint (no unwinding recovery).

## 2. Build-time environment variables

| Variable | Used by | Purpose | Production |
|---|---|---|---|
| `TAURI_DEV_HOST` | `vite.config.ts` | Dev server binding + HMR host for remote debugging | Must be unset |
| `TAURI_SIGNING_PRIVATE_KEY` | Tauri bundler (when signing) | Update signature key | CI secret only |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Tauri bundler | Key password | CI secret only |
| `RUST_LOG` | Planned `tracing` init | Log filter | Default `info`; no secrets |
| `CARGO_*` / `RUSTUP_*` | Toolchain | Standard Rust tooling | Not app config |

Rules:

- No `.env` file is committed. If one is introduced, it is gitignored and
  documented here.
- Never read environment variables as a hidden configuration channel in release
  builds; behavior must be reproducible from the config file and UI.
- Never log environment variables.

## 3. Runtime settings (proposed)

No settings system exists. The proposed design:

```text
%LOCALAPPDATA%\com.systemscanner.desktop\settings.json
```

```jsonc
{
  "schema_version": 1,
  "appearance": { "theme": "system", "density": "comfortable" },
  "scan": {
    "concurrency": 4,
    "follow_reparse_points": false,
    "max_hash_file_bytes": 268435456,
    "max_hashes_concurrent": 2,
    "exclude_globs": []
  },
  "refresh": { "periodic_enabled": false, "interval_seconds": 30 },
  "retention": { "snapshots_days": 30, "processes_days": 7, "findings_days": 90 },
  "privacy": { "network_features": false, "diagnostics_include_paths": false },
  "logging": { "level": "info", "max_files": 5 }
}
```

### 3.1 Rules for settings

1. **Validated in Rust.** Invalid values fall back to safe defaults and are
   reported, never silently accepted.
2. **Clamped.** Concurrency, sizes, and retention have hard bounds.
3. **Versioned.** `schema_version` drives migration.
4. **No secrets.** Credentials belong in the OS credential store, not settings.
5. **Visible.** Any setting that changes privacy, privilege, or resource use is
   surfaced in the UI with its consequence.
6. **Exported in diagnostics** (redacted).

### 3.2 Precedence (proposed)

```text
built-in defaults < settings.json < per-session user action (not persisted)
```

No hidden per-machine override. If an enterprise policy is ever needed, it
requires a separate decision and an ADR.

## 4. Data locations

| Data | Path (proposed) | Sensitive |
|---|---|---|
| Settings | `%LOCALAPPDATA%\com.systemscanner.desktop\settings.json` | Low |
| Database | `...\app.db` (+ `-wal`, `-shm`) | High (paths, process data) |
| Logs | `...\logs\app.log*` | Medium (redacted) |
| Quarantine | `...\quarantine\` | High (isolated files) |
| Exports | User-selected path | User-controlled |

- Never write inside `Program Files` or the install directory.
- Apply user-only ACLs to the data directory.
- Document what survives uninstall (undecided).

## 5. Secrets

| Type | Storage |
|---|---|
| Signing key | CI secret store / hardware token |
| Certificate | CI secret store / OS certificate store |
| Future API keys | Windows Credential Manager, never settings.json |

No secret may appear in the repository, logs, crash reports, or support bundles.

## 6. Configuration drift checks (proposed)

```text
[ ] package.json version == tauri.conf.json version == Cargo.toml version
[ ] capabilities list reviewed in the PR
[ ] CSP diff requires explicit approval
[ ] settings schema_version has a migration if changed
[ ] no new env var is required to run the app
```

## 7. Related

- [`../build.md`](../build.md) — commands and toolchain
- [`DEVOPS.md`](DEVOPS.md) — CI secrets
- [`MIGRATIONS.md`](MIGRATIONS.md) — settings and schema versioning
- [`NETWORKING.md`](NETWORKING.md) — network feature flags and consent
