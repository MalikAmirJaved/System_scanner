<div align="center">

# System Scanner

**A clearer view of your PC.**

Local-first Windows system intelligence — inspect, understand, and maintain your
machine without an account, a cloud, or a verdict you can't verify.

![status](https://img.shields.io/badge/status-pre--alpha-orange)
![platform](https://img.shields.io/badge/platform-Windows%2010%2F11%20x64-0078D4)
![stack](https://img.shields.io/badge/stack-Tauri%202%20%C2%B7%20Rust%20%C2%B7%20React%2019-2ea44f)
![license](https://img.shields.io/badge/license-not%20selected-lightgrey)

</div>

> **Honest status — read this first.** This repository is at **Release 0
> (foundation)**. The desktop shell, build pipeline, installer, and a styled
> welcome screen exist. System collection, scanning, persistence, IPC commands,
> tests, and CI **do not exist yet**. Everything else in this repository is a
> documented plan, not a shipped feature. The authoritative plan is
> [`docs/plan.md`](docs/plan.md).

---

## What it is

System Scanner is a desktop application for understanding a Windows machine:
what hardware it has, what is running, what is consuming storage, what deserves
a closer look, and what changed over time. It is designed around a simple rule:

> **Evidence before verdict.** A finding shows what was observed, how, with what
> confidence, and what was *not* checked. "Unknown" is a valid, honest result.

It is a local desktop product, not a service. There is no account, no telemetry,
and no network dependency for core functionality.

## Where the project is today

| Capability | State | Evidence |
|---|---|---|
| Tauri 2 + React 19 + TS + Vite shell launches | ✅ Implemented | `src-tauri/`, `src/App.tsx` |
| Production build + NSIS installer | ✅ Implemented | `src-tauri/target/release/bundle/nsis/` |
| Strict CSP, minimal Tauri capability set | ✅ Implemented | `tauri.conf.json`, `src-tauri/capabilities/default.json` |
| Styled welcome screen, design tokens | ✅ Implemented | `src/App.tsx`, `src/App.css` |
| Typed IPC command surface | ⏳ Planned | `src-tauri/src/lib.rs` has no commands |
| System / process collection | ⏳ Planned | Release 0 |
| Storage scan engine | ⏳ Planned | Release 1 |
| SQLite persistence + migrations | ⏳ Planned | Release 1 |
| Tests, lint config, CI | ⏳ Planned | none configured |
| Network egress | 🚫 None by design | no HTTP client dependency |

Full inventory: [`docs/FEATURES.md`](docs/FEATURES.md). Gaps:
[`docs/KNOWN_ISSUES.md`](docs/KNOWN_ISSUES.md).

## Product principles

1. **Local-first** — inspection and history work offline. Any external lookup is
   opt-in, explicit, and disclosed.
2. **Evidence before verdict** — findings carry provenance, confidence, and
   limitations. Missing data is never treated as a positive signal.
3. **Safe by default** — read-only first. Destructive or privileged operations
   require explicit intent, a clear consequence, and a recovery path.
4. **Least privilege** — run unelevated; elevate one narrowly scoped operation
   only when it is genuinely required.
5. **Responsive and bounded** — scans are cancellable, streamed, and governed by
   configurable memory, CPU, and disk budgets.
6. **Recoverable actions** — quarantine and restore precede deletion.
7. **Observable without surveillance** — local diagnostics, no telemetry by default.
8. **Modular evolution** — UI, orchestration, domain logic, Windows adapters, and
   persistence are separated by explicit contracts.

## Tech stack

| Layer | Choice | Version |
|---|---|---|
| Shell | [Tauri](https://tauri.app/) | 2.x (`tauri-cli` 2.12.4) |
| Core | Rust | edition 2024, stable 1.99.1 |
| UI | React + TypeScript | 19.1 / ~6.0.3 |
| Bundler | Vite | ^8.0.16 |
| Persistence | SQLite (**not yet added**) | — |
| Installer | NSIS | x64 |

Rationale, alternatives, and rejected options: [`TECH_STACK.md`](TECH_STACK.md).

## Quick start

**Prerequisites:** Windows 10/11 x64, Node ≥ 20, Rust stable, WebView2, and
Visual Studio Build Tools with the C++ desktop workload. See
[`build.md`](build.md) for the verified toolchain table.

```powershell
npm install          # install frontend dependencies
npm run tauri dev    # launch the desktop app with hot reload
```

Build a release installer:

```powershell
npm run tauri build
# -> src-tauri/target/release/bundle/nsis/System Scanner_0.1.0_x64-setup.exe
```

Every command, flag, artifact path, and troubleshooting entry lives in
[`build.md`](build.md). It is the only place commands are documented.

## Project layout

```text
system-scanner/
├─ src/                     # React UI (rendering only, no OS access)
│  ├─ App.tsx               # welcome screen (current entry point)
│  ├─ App.css               # design tokens + layout
│  └─ main.tsx              # React root
├─ src-tauri/
│  ├─ src/lib.rs            # Tauri builder (no commands yet)
│  ├─ src/main.rs           # binary entry point
│  ├─ capabilities/         # Tauri permission surface (core:default only)
│  ├─ icons/                # app + installer icons
│  ├─ tauri.conf.json       # window, CSP, bundle config
│  └─ Cargo.toml            # Rust dependencies + release profile
├─ public/                  # static assets
├─ docs/                    # engineering, product, security documentation
├─ build.md                 # every command (single source of truth)
├─ AGENTS.md                # rules for AI coding agents
└─ plan.md                  # pointer -> docs/plan.md
```

The target structure — feature folders, domain/adapters split, migrations — is
specified in [`ARCHITECTURE.md`](ARCHITECTURE.md) and
[`docs/SYSTEM_DESIGN.md`](docs/SYSTEM_DESIGN.md). Directories are created when
a feature needs them, not before.

## Documentation map

<details open>
<summary><strong>Start here</strong></summary>

| Document | Purpose |
|---|---|
| [`docs/plan.md`](docs/plan.md) | Authoritative product & engineering plan |
| [`build.md`](build.md) | All verified and planned commands |
| [`AGENTS.md`](AGENTS.md) | Rules every AI agent must follow |
| [`CONTRIBUTING.md`](CONTRIBUTING.md) | Workflow, standards, review checklist |
| [`docs/FEATURES.md`](docs/FEATURES.md) | Feature inventory with real status |
| [`docs/ROADMAP.md`](docs/ROADMAP.md) | Release milestones and version framing |

</details>

<details>
<summary><strong>Engineering</strong></summary>

| Document | Purpose |
|---|---|
| [`ARCHITECTURE.md`](ARCHITECTURE.md) | System architecture and module boundaries |
| [`docs/SYSTEM_DESIGN.md`](docs/SYSTEM_DESIGN.md) | Detailed design, job lifecycle, data flow |
| [`TECH_STACK.md`](TECH_STACK.md) | Technologies, versions, justification |
| [`DATABASE.md`](DATABASE.md) | Schema, indexes, constraints, retention |
| [`API.md`](API.md) | IPC command/event contracts, error envelope |
| [`TESTING.md`](TESTING.md) | Test strategy and quality gates |
| [`docs/ERROR_HANDLING.md`](docs/ERROR_HANDLING.md) | Error taxonomy and failure handling |
| [`docs/PERFORMANCE.md`](docs/PERFORMANCE.md) | Performance budgets and optimization rules |
| [`docs/SCALABILITY.md`](docs/SCALABILITY.md) | Scaling model for a local desktop app |
| [`docs/NETWORKING.md`](docs/NETWORKING.md) | Network posture and protocol rules |
| [`docs/INTEGRATIONS.md`](docs/INTEGRATIONS.md) | Windows APIs and external services |
| [`docs/CONFIGURATION.md`](docs/CONFIGURATION.md) | Env vars, settings, secrets |
| [`docs/MIGRATIONS.md`](docs/MIGRATIONS.md) | Database and schema migration procedure |

</details>

<details>
<summary><strong>Product</strong></summary>

| Document | Purpose |
|---|---|
| [`PRD.md`](PRD.md) | Product requirements and business objectives |
| [`docs/ROADMAP.md`](docs/ROADMAP.md) | V1/V2/V3 and future direction |
| [`docs/FEATURES.md`](docs/FEATURES.md) | Complete feature inventory |
| [`docs/BACKLOG.md`](docs/BACKLOG.md) | Prioritized pending work |
| [`docs/KNOWN_ISSUES.md`](docs/KNOWN_ISSUES.md) | Known bugs and limitations |
| [`docs/CHANGELOG.md`](docs/CHANGELOG.md) | Version history |
| [`docs/DECISIONS.md`](docs/DECISIONS.md) | Architecture decision records |
| [`docs/RISKS.md`](docs/RISKS.md) | Risk register |
| [`docs/DEPENDENCIES.md`](docs/DEPENDENCIES.md) | Dependency inventory and policy |

</details>

<details>
<summary><strong>Security & compliance</strong></summary>

| Document | Purpose |
|---|---|
| [`SECURITY.md`](SECURITY.md) | Security policy and reporting |
| [`docs/THREAT_MODEL.md`](docs/THREAT_MODEL.md) | Assets, attackers, attack surfaces |
| [`docs/ACCESS_CONTROL.md`](docs/ACCESS_CONTROL.md) | Permissions, elevation, authorization |
| [`docs/DATA_PRIVACY.md`](docs/DATA_PRIVACY.md) | Data categories and privacy rules |
| [`docs/AUDIT_LOGGING.md`](docs/AUDIT_LOGGING.md) | Auditable action history |
| [`docs/INCIDENT_RESPONSE.md`](docs/INCIDENT_RESPONSE.md) | Incident handling |
| [`docs/COMPLIANCE.md`](docs/COMPLIANCE.md) | Regulatory and licensing posture |

</details>

<details>
<summary><strong>Operations</strong></summary>

| Document | Purpose |
|---|---|
| [`docs/DEPLOYMENT.md`](docs/DEPLOYMENT.md) | Install, upgrade, uninstall, signing |
| [`docs/DEVOPS.md`](docs/DEVOPS.md) | CI/CD and automation (planned) |
| [`docs/MONITORING.md`](docs/MONITORING.md) | Logs, metrics, diagnostics |
| [`docs/BACKUP_POLICY.md`](docs/BACKUP_POLICY.md) | Backup frequency and verification |
| [`docs/RECOVERY_PLAN.md`](docs/RECOVERY_PLAN.md) | Recovery objectives and procedures |
| [`docs/DISASTER_RECOVERY.md`](docs/DISASTER_RECOVERY.md) | Data loss and corruption recovery |
| [`docs/FAILURE_MODES.md`](docs/FAILURE_MODES.md) | Expected failures and handling |
| [`docs/RELEASE_POLICY.md`](docs/RELEASE_POLICY.md) | Versioning, gates, rollback |
| [`docs/SLA_SLO.md`](docs/SLA_SLO.md) | Reliability targets |

</details>

<details>
<summary><strong>AI collaboration</strong></summary>

| Document | Purpose |
|---|---|
| [`AGENTS.md`](AGENTS.md) | Entry point for AI agents |
| [`AI_CONTEXT.md`](AI_CONTEXT.md) | Project context and invariants |
| [`AI_RULES.md`](AI_RULES.md) | Hard rules that are never violated |
| [`AI_WORKFLOW.md`](AI_WORKFLOW.md) | How to approach a task |
| [`AI_CODE_REVIEW.md`](AI_CODE_REVIEW.md) | Self-review protocol |
| [`AI_ARCHITECTURE_RULES.md`](AI_ARCHITECTURE_RULES.md) | Enforced boundaries |
| [`AI_SECURITY_RULES.md`](AI_SECURITY_RULES.md) | Security restrictions |
| [`AI_TESTING_RULES.md`](AI_TESTING_RULES.md) | Required tests before "done" |
| [`AI_CHANGELOG.md`](AI_CHANGELOG.md) | AI-assisted architectural changes |

</details>

## Contributing

Read [`CONTRIBUTING.md`](CONTRIBUTING.md) before opening a pull request. In short:
keep the domain layer free of Windows APIs, validate all IPC input in Rust, add
tests with the change, and never commit secrets or generated build output.

## Security

Please do not open public issues for vulnerabilities. See
[`SECURITY.md`](SECURITY.md) for the reporting process and the current
limitations of the disclosure channel.

## License

No license has been selected for this repository yet. Until one is added, the
code is **all rights reserved** by default and cannot be redistributed. See
[`docs/COMPLIANCE.md`](docs/COMPLIANCE.md) for the pending decision.
