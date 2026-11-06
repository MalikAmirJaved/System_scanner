# Product Requirements Document (PRD)

> **Status:** Proposed baseline · **Version:** 0.1 · **Last updated:** 2026-11-06
> **Source of truth:** [`docs/plan.md`](docs/plan.md) — this PRD summarizes product
> intent; the plan governs engineering sequencing and acceptance gates.
>
> **Implementation status:** Release 0 shell only. No collection, scanning,
> persistence, or security features exist. See [`docs/FEATURES.md`](docs/FEATURES.md).

---

## 1. Problem

Windows users have plenty of system data and very little trustworthy
interpretation. Existing tools tend to fall into two failure modes:

1. **Opaque optimizers** that delete files, "clean" the registry, and report a
   score with no evidence — often causing harm and always preventing learning.
2. **Expert-only utilities** that expose raw counters without explaining what
   matters, what is normal, or what is safe to do next.

Users also increasingly distrust tools that require an account, upload telemetry,
or phone home to produce a local result.

## 2. Product

A Windows-first, **local-first** desktop application that helps a user:

- understand what their machine is (hardware, OS, disks, network);
- see what is running and what starts automatically;
- find where storage is going, without guessing;
- review security-relevant evidence with honest confidence levels;
- perform **reversible**, explicitly confirmed maintenance.

No account. No telemetry by default. No verdict without evidence.

## 3. Target users

| Persona | Need | Success looks like |
|---|---|---|
| Everyday user with a slow/full PC | "Why is my disk full / PC slow?" | Understands the top causes in under 2 minutes without breaking anything |
| Power user / developer | Precise, exportable system facts | Can filter, sort, export, and verify findings |
| IT support technician (single machine) | Fast, trustworthy diagnosis | Produces a redacted diagnostic bundle a colleague can read |
| Privacy-conscious user | No cloud, no account | Confirms zero network egress in normal use |

**Non-users (explicit non-goals):** fleet/enterprise administration, multi-tenant
SaaS, real-time EDR, kernel-level protection, cross-platform parity in V1.

## 4. Business objectives

This is a **desktop product with no server-side revenue dependency**. Objectives
are therefore about trust, adoption, and sustainability rather than uptime.

| ID | Objective | Measure | Status |
|---|---|---|---|
| BO-1 | Earn trust through verifiable evidence | 100% of findings expose source + confidence | Planned |
| BO-2 | Work fully offline | Core flows succeed with networking disabled | Planned |
| BO-3 | Avoid harm | Zero irreversible actions without confirmation + recovery | Planned |
| BO-4 | Stay responsive at scale | UI remains interactive during a multi-million-file scan | Planned |
| BO-5 | Sustainable footprint | Small installer, bounded memory, small dependency set | Partial (1.4 MB installer) |
| BO-6 | Low support burden | Actionable local diagnostics reduce "it doesn't work" reports | Planned |

> Revenue model, pricing, and distribution ownership are **open decisions**
> (`docs/plan.md` §12). Do not design monetization features before they are decided.

## 5. Scope by release

Detailed acceptance criteria: [`docs/ROADMAP.md`](docs/ROADMAP.md) and
[`docs/plan.md`](docs/plan.md) §14.

### Release 0 — Foundation
Read-only system overview (OS, CPU, memory, disks, adapters), process list,
manual + bounded periodic refresh, diagnostics view, installable build.
**No verdicts, no deletion, no elevation, no network.**

### Release 1 — Storage intelligence
Drive discovery, cancellable traversal, large-file/type/age summaries, duplicate
candidates via size + partial fingerprint, optional bounded hashing, paginated
results. **Recommendations only.**

### Release 2 — Processes, startup, services
Process hierarchy, startup inventory, service inventory, explain limitations.
Optional user-confirmed control only after threat model + audit + recovery.

### Release 3 — Security intelligence
Evidence-based indicators, signature/hash/reputation status, Defender status,
quarantine/restore with integrity and audit. **Never call a heuristic "malware".**

### Release 4 — Diagnostics and network
Event Log summaries, crash correlation, adapters/DNS/connections/ports with
process ownership, historical trends with retention limits.

### Release 5 — Managed maintenance
Schedules and policies with resource awareness, exportable redacted reports,
reversible maintenance workflows, explainable health summary.

## 6. Functional requirements (Release 0 baseline)

| ID | Requirement | Priority | Status |
|---|---|---|---|
| FR-001 | App launches on clean Windows 10/11 x64 install | P0 | Partial (builds; not VM-tested) |
| FR-002 | Read-only system snapshot via typed Rust command | P0 | Not started |
| FR-003 | Process list with stable identity + refresh semantics | P0 | Not started |
| FR-004 | Manual refresh with in-progress state and last-updated time | P0 | Not started |
| FR-005 | Bounded periodic refresh with visible interval + pause | P1 | Not started |
| FR-006 | Explicit unavailable/permission-restricted field states | P0 | Not started |
| FR-007 | Diagnostics view with redacted, shareable log export | P0 | Not started |
| FR-008 | No admin requirement for any Release 0 flow | P0 | Not started |
| FR-009 | No unexpected network requests | P0 | Partial (no client exists) |
| FR-010 | Install/uninstall leaves no orphan state | P1 | Partial (installer only) |

## 7. Non-functional requirements

| Area | Requirement |
|---|---|
| Performance | Launch to usable dashboard within an agreed budget; refresh never blocks rendering (budgets in [`docs/PERFORMANCE.md`](docs/PERFORMANCE.md)) |
| Scale | Bounded memory during scans; paged IPC; virtualized tables |
| Reliability | Interrupted scans leave recoverable or safely discardable state |
| Security | UI compromise cannot directly reach the OS; minimal capabilities; CSP |
| Privacy | No telemetry by default; any export is previewable and redacted |
| Accessibility | Keyboard navigation, visible focus, contrast, non-color-only status |
| Compatibility | Supported Windows versions published before release |

## 8. Out of scope (all releases unless re-decided)

- Kernel driver or filesystem filter driver.
- Automatic malware conviction from heuristics.
- Automatic deletion/cleanup without confirmation.
- Cloud sync, accounts, fleet management.
- Cross-platform macOS/Linux builds.

## 9. Assumptions

1. Users can install a signed (eventually) desktop app.
2. WebView2 is available (evergreen on Windows 11; bootstrapped on Windows 10).
3. The app runs as the logged-in user; elevation is the exception.
4. SQLite is sufficient for local history at expected volumes.
5. Users accept a one-time scan cost in exchange for bounded resource use.

## 10. Success metrics

Measured locally unless the user explicitly opts into sharing:

- Time to first useful system view after launch.
- Percentage of scans completed vs cancelled vs failed.
- Partial-result rate (access denied, disappeared) surfaced honestly.
- Crash-free sessions.
- Memory peak by scan size.
- Support bundles successfully self-diagnosed.

## 11. Dependencies and constraints

- Windows API availability varies by version and privilege; every field must have
  a defined "unavailable" state.
- Code signing, distribution channel, and supported-version matrix are unresolved.
- No licensing decision has been made (blocks public redistribution).

## 12. Open product decisions

Tracked in [`docs/plan.md`](docs/plan.md) §12. Until resolved: local-only,
read-only, non-elevated. Do not build features that presume an answer.
