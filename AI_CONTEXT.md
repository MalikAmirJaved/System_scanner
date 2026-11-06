# AI_CONTEXT

> **Purpose:** the minimum context an AI agent needs to work correctly in this
> repository. Read this before touching code.
> **Last updated:** 2026-11-06

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

## 2. Verified current state (2026-11-06)

| Fact | Value | Source |
|---|---|---|
| App version | `0.1.0` | `package.json`, `tauri.conf.json` |
| Identifier | `com.systemscanner.desktop` | `tauri.conf.json` |
| Product name | `System Scanner` | `tauri.conf.json` |
| Window | 960×720, min 420×560, centered | `tauri.conf.json` |
| Bundle target | NSIS x64 only | `tauri.conf.json` |
| CSP | `default-src 'self'; img-src 'self' asset: data:; style-src 'self' 'unsafe-inline'; font-src 'self' data:` | `tauri.conf.json` |
| Capabilities | `core:default` only, window `main` | `capabilities/default.json` |
| Runtime JS deps | `react`, `react-dom`, `@tauri-apps/api` | `package.json` |
| Rust deps | `tauri 2` only | `Cargo.toml` |
| Release profile | LTO, `codegen-units=1`, `opt-level=3`, `panic=abort`, `strip` | `Cargo.toml` |
| Rust files | `main.rs` (entry), `lib.rs` (builder) | `src-tauri/src/` |
| IPC commands | **none** | `lib.rs` |
| Database | **none** | — |
| Tests | **none** | — |
| CI | **none** | — |
| Frontend | single static welcome screen | `src/App.tsx` |

If any of these change, update this table in the same commit.

## 3. What does not exist (do not assume it does)

- No `invoke()` calls, handlers, or DTOs.
- No collectors, adapters, domain layer, or job orchestrator.
- No SQLite, migrations, repositories, or data directory.
- No logging, tracing, or diagnostics export.
- No tests, linters, formatters, or CI workflows.
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
| Render a screen | `src/features/<domain>/` (planned) |
| Share a presentational component | `src/components/` (planned) |
| Call Rust from the UI | `src/services/` (planned) — never in a component |
| Define an IPC command | `src-tauri/src/commands/` (planned) |
| Orchestrate jobs/cancellation | `src-tauri/src/app/` (planned) |
| Business rules, policies | `src-tauri/src/domain/` (planned) — no Windows, no SQL |
| Declare a capability trait | `src-tauri/src/ports/` (planned) |
| Call Win32/WMI/registry | `src-tauri/src/adapters/windows/` (planned) |
| Query SQLite | `src-tauri/src/adapters/persistence/` (planned) |
| Export a report | `src-tauri/src/reporting/` (planned) |

Right now only `src/` and `src-tauri/src/{main,lib}.rs` exist. Create directories
when the first feature needs them; do not scaffold empty modules.

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
| Logging full paths by default | Redact or hash; log at debug only |
| Marking the checklist item done because code compiles | Evidence required: test, measurement, or review |
