# AI_CONTEXT

> **Purpose:** the minimum context an AI agent needs to work correctly in this
> repository. Read this before touching code.
> **Last updated:** 2026-11-17

---

## 1. What this project is

**System Scanner** — a Windows-first, local-first desktop application for system
inspection, storage intelligence, and evidence-based security visibility.

- Not a cloud service. No account, no telemetry, no server.
- Not an antivirus. It never claims malware without evidence.
- Currently a **Tauri 2 + React 19 + TypeScript** desktop shell at **Release 0**.

Authoritative plan: [`docs/plan.md`](docs/plan.md).
Product intent: [`PRD.md`](PRD.md).
Architecture: [`ARCHITECTURE.md`](ARCHITECTURE.md).

## 2. Verified current state (2026-11-16)

| Fact | Value | Source |
|---|---|---|
| App version | `0.1.0` | `package.json`, `tauri.conf.json` |
| Identifier | `com.systemscanner.desktop` | `tauri.conf.json` |
| Product name | `System Scanner` | `tauri.conf.json` |
| Window | 960×720, min 420×560, centered | `tauri.conf.json` |
| Bundle target | NSIS x64 only | `tauri.conf.json` |
| CSP | `default-src 'self'; img-src 'self' asset: data:; style-src 'self' 'unsafe-inline'; font-src 'self' data:` | `tauri.conf.json` |
| Capabilities | `core:default` only, window `main` | `capabilities/default.json` |
| Runtime JS deps | `react`, `react-dom`, `@tauri-apps/api`, Tailwind v4 styling primitives (`cva`, `clsx`, `tailwind-merge`, Radix `Slot`, `lucide-react`) | `package.json` |
| Styles | Tailwind CSS v4 via `@tailwindcss/vite`; tokens in `src/styles/globals.css` | `vite.config.ts` |
| Rust deps | `tauri 2`, `serde`, `serde_json`, `thiserror`, `tracing`/`tracing-subscriber`, `rusqlite 0.43` (bundled SQLite), `windows-sys 0.61` (Windows target) | `Cargo.toml` |
| Release profile | LTO, `codegen-units=1`, `opt-level=3`, `panic=abort`, `strip` | `Cargo.toml` |
| Rust files | `main.rs`, `lib.rs`, `ipc.rs`, `error.rs`, `logging.rs`, `redact.rs`, `startup.rs`, `dto/`, `commands/`, `domain/`, `ports/`, `app/`, `adapters/windows/`, `adapters/persistence/`, `adapters/filesystem/` | `src-tauri/src/` |
| IPC commands | `get_app_info`, `get_system_snapshot`, `list_processes`, `get_diagnostics`, plus `start_scan`, `cancel_scan`, `get_scan_status` (B-025). All read-only; the scan commands are called by `ScanPanel` through `src/services/scan.ts` | `src-tauri/src/commands/` |
| IPC events | `scan:progress`, `scan:completed`, `scan:failed`, `scan:cancelled` (aggregate counters only, paced to 10/s) | `API.md` §4.1, `src-tauri/src/app/jobs.rs` |
| Wire format | envelope `{status, data|error}`; DTO fields camelCase; codes snake_case | `API.md` §1–§2 |
| Frontend IPC | `src/services/ipc.ts` is the only `invoke` call site; every response and scan event is validated in the renderer by hand-written guards (`src/services/validate.ts`, B-090) | `src/services/` |
| Database | SQLite schema **v2** (`scan_sessions`) via `rusqlite`; file at `app_data_dir()/system-scanner.db`, opened at startup. One counters-only row is written per scan through the `ScanSessionStore` port (best-effort); leftover open rows become `interrupted` at startup. A verified `VACUUM INTO` snapshot goes to `backups/` before pending migrations (newest five, best-effort; B-033). **No paths stored; no command reads rows back; restore is manual** | `src-tauri/src/adapters/persistence/`, `src-tauri/migrations/` |
| Traversal | Bounded worker prototype with cancellation, depth/entry budgets, an outcome per entry with stable codes, and a derived `Completeness` verdict (B-023); each directory is opened once without following its final link and enumerated through that handle, with a volume-serial + 128-bit file-ID check against the parent-recorded identity (B-041; mismatch → `disappeared`); driven by an in-memory scan job (B-025) shown in the Storage scan panel; **persisted only as a counters-only session row**. `domain/policy.rs` holds the protected-path policy (B-041): deny-by-default over the system roots with a reason per denial, **implemented but not consulted by any command** (KI-057) | `src-tauri/src/adapters/filesystem/directory.rs`, `src-tauri/src/adapters/filesystem/traversal.rs`, `src-tauri/src/domain/traversal.rs` |
| Tests | 150 Rust + 1 opt-in baseline (`--ignored`) + 76 frontend (`npm test`, `cargo test`) | `src/**/*.test.ts(x)`, `#[cfg(test)]` |
| CI | `.github/workflows/ci.yml` (**never run on GitHub**) | `.github/` |
| Frontend | shell header + backend status + system snapshot + paged process list + storage scan panel (roots text box, start/cancel, live counts, completeness verdict; no folder picker) + on-request diagnostics in a responsive 1/2/3-column grid, inside an `ErrorBoundary` | `src/App.tsx`, `src/components/`, `src/components/ui/button.tsx` |

If any of these change, update this table in the same commit.

## 3. What does not exist (do not assume it does)

- `get_system_snapshot` reads OS, CPU, memory, disks, and network adapters.
- `list_processes` reads PID, parent PID, name, thread count, start time, and
  working set. It does **not** read image paths, command lines, CPU usage, or
  session IDs (ADR-013), and it is not sortable or searchable.
- Neither command reads services, startup entries, files, or event logs.
- Jobs exist only for the scan: one at a time. Live state is in memory; the final
  counters are also written as one session row. No generic job registry, no
  history view, and no resumption.
- No startup/service collectors and no storage views. The Storage scan panel
  starts, cancels, and shows aggregate counters from `scan:*` events, but there
  is no folder picker (roots are typed), no file list or treemap, and file-level
  data is neither persisted nor queryable (`get_scan_summary`, `query_files` do
  not exist).
- No scan history, repositories, or IPC command that reads the database.
  `adapters/persistence` exists and a finished scan writes one `scan_sessions`
  row, but **no command reads rows back**, there is no history UI, no per-entry
  storage, no retention policy, and no delete control. A pre-migration snapshot
  is written to `backups/`, but there is no restore command, scheduled backup,
  or backup UI.
- No saved-file diagnostics export and no log viewer or level filter. The
  in-app preview/copy of a redacted support bundle exists (B-006); logs remain
  local files.
- No app shell navigation or routing — one screen with five panels (backend
  status, system snapshot, process inventory, storage scan, diagnostics).
- No runtime schema generation: the response guards are hand-written and must be
  updated with each DTO change (ADR-022). A renamed field fails as
  `contract_violation`; a new field is invisible until a guard copies it.
- No auto-updater, code signing, or telemetry.
- No network access of any kind.

## 4. Terminology (use these words precisely)

| Term | Meaning |
|---|---|
| **Observation** | A fact collected from the system, with a source |
| **Finding** | An observation plus an interpretation, with confidence and rule version |
| **Evidence** | The data supporting a finding, with provenance |
| **Known / Suspicious / Unknown / Not scanned** | The only allowed finding states; never "infected" |
| **Partial** | The operation completed with skipped/denied/failed items |
| **Session** | A persisted scan or job record |
| **Job** | A running operation with an ID, state, and cancellation token |
| **Protected path** | A location the app will not modify without a specific policy decision |
| **Budget** | A hard limit on memory, concurrency, result size, event rate, or disk |
| **Quarantine** | Reversible isolation of a file with integrity metadata |

Avoid: "clean", "infected", "optimize", "fix" for anything that is not verified.

## 5. Where things go

| I want to… | Put it in |
|---|---|
| Render a screen | `src/features/<domain>/` (planned); presentational pieces in `src/components/` |
| Share a presentational component | `src/components/` (implemented); shared primitives in `src/components/ui/` |
| Add or change a design token | `src/styles/globals.css` (`@theme` / `@theme inline`) |
| Log a backend event | `tracing::info!`/`warn!` at the call site; subscriber and rotation live in `src-tauri/src/logging.rs` |
| Redact diagnostic text | `src-tauri/src/redact.rs` (shape rules + literal secrets); add a test for every new field that reaches a bundle |
| Report a startup or render failure | `src-tauri/src/startup.rs` (native dialog, exits 1) and `src/components/ErrorBoundary.tsx` (reload screen); never sprinkle `expect()` back in |
| Call Rust from the UI | `src/services/` (implemented) — never in a component |
| Define an IPC command | `src-tauri/src/commands/` (implemented) |
| Orchestrate jobs/cancellation | `src-tauri/src/app/` (planned) |
| Business rules, policies | `src-tauri/src/domain/` (implemented for the snapshot and process paging) — no Windows, no SQL |
| Declare a capability trait | `src-tauri/src/ports/` (implemented) |
| Call Win32/WMI/registry | `src-tauri/src/adapters/windows/` (implemented) |
| Enumerate files/directories | `src-tauri/src/adapters/filesystem/` (prototype B-022); keep policy and counters in `domain/traversal.rs` |
| Query SQLite | `src-tauri/src/adapters/persistence/` (implemented — direct adapters only so far; add a port when a use case needs one) |
| Export a report | `src-tauri/src/reporting/` (planned) |

`src/`, `commands/`, `dto/`, `domain/`, `ports/`, `adapters/windows/`,
`adapters/persistence/`, and `adapters/filesystem/` exist. Create the remaining
directories when the first feature needs them; do not scaffold empty modules.

## 6. Non-negotiable invariants

1. UI never touches the OS.
2. Domain never imports Windows APIs, Tauri, or SQL.
3. All IPC input is untrusted and validated in Rust.
4. No destructive action without confirmation, audit, and recovery.
5. No telemetry, no silent network, no account.
6. Unknown is a valid result; never fabricate defaults.
7. Every long-running operation is cancellable and bounded.
8. Documentation must distinguish implemented from planned.

Full rule set: [`AI_RULES.md`](AI_RULES.md).

## 7. Build and verify

Commands: [`build.md`](build.md). Minimum verification:

```powershell
npm run build
cd src-tauri; cargo fmt --all -- --check; cargo clippy --all-targets -- -D warnings; cargo test
```

## 8. Open decisions that block design

From [`docs/plan.md`](docs/plan.md) §12:

1. Audience: learning prototype, consumer product, or internal tool?
2. Supported Windows versions/architectures; is Windows 10 required?
3. Strictly single-user/local, or future fleet management?
4. What data is retained, for how long, and where?
5. Any external reputation lookup? Under what consent?
6. Distribution channel and code-signing ownership?
7. Reference hardware and dataset for "production-ready" performance?
8. Which features require administrator privileges?

**Until answered: local-only, read-only, non-elevated.**

## 9. Common mistakes to avoid

| Mistake | Correct behavior |
|---|---|
| Adding a file scanner "because Release 1 exists in the plan" | Release 1 is not implemented; follow the task you were given |
| Calling a heuristic result "malware" | Use `suspicious` + confidence + evidence |
| Putting `std::fs` calls in `domain` | Add a port and a filesystem adapter |
| Sending 100k rows to the UI | Aggregate in Rust; page results |
| Adding `tokio`/`serde` "just in case" | Add only with a feature that needs it, plus a dependency entry |
| Logging full paths by default | Redact or hash before logging |
| Marking the checklist item done because code compiles | Evidence required: test, measurement, or review |
