# Roadmap

> **Status:** Release 0 in progress · **Last updated:** 2026-11-06
> **Authority:** [`plan.md`](plan.md) governs sequencing; this file tracks status.
>
> A release is "done" only when its exit criteria have evidence. Checked items
> here mean code + verification exist, not that they are planned.

---

## 1. Version framing

The plan defines **Release 0–5**. Product versions map to them as follows:

| Version | Releases | Theme | State |
|---|---|---|---|
| **V1** | Release 0 + 1 | Trustworthy read-only inspection + storage intelligence | In progress |
| **V2** | Release 2 + 3 | Process/startup/service visibility + evidence-based security | Planned |
| **V3** | Release 4 + 5 | Diagnostics, network, history, scheduled maintenance | Planned |
| **Future** | — | Optional hosted reputation, fleet management (requires separate architecture) | Not scoped |

| App version | Content |
|---|---|
| `0.1.x` | Release 0 (foundation) |
| `0.2.x` | Release 1 (storage) |
| `0.3.x` | Release 2 (processes/startup/services) |
| `0.4.x` | Release 3 (security) |
| `0.5.x` | Release 4 (diagnostics/network) |
| `0.6.x` | Release 5 (maintenance) |
| `1.0.0` | Stability + support commitments (undecided) |

Version numbers are a proposal; `1.0.0` should mean "supportable", not "feature
complete".

## 2. Release 0 — Foundation

**Goal:** a launchable, honest, read-only system view with no privilege or network
requirements.

| Work item | Status |
|---|---|
| Tauri 2 + React/TS/Vite scaffold | ✅ Implemented |
| Production build + NSIS installer | ✅ Implemented |
| Minimal capability set + CSP | ✅ Implemented |
| Welcome screen / app shell | ✅ Implemented |
| Typed IPC boundary + DTO + error model | ⏳ Not started |
| System snapshot (OS/CPU/memory/disks/network) | ⏳ Not started |
| Process list with stable identity | ⏳ Not started |
| Refresh lifecycle (manual + bounded periodic) | ⏳ Not started |
| Diagnostics view + redacted support bundle | ⏳ Not started |
| Tests (unit/adapter/UI) | ⏳ Not started |
| CI on Windows | ⏳ Not started |

**Exit criteria**

- Clean install and upgrade smoke tests pass on supported Windows versions.
- No UI freeze during refresh; stale responses cannot overwrite newer data.
- All collection commands have tests and structured errors.
- No unexpected network access; no elevation requirement.

## 3. Release 1 — Storage intelligence

Cancellable traversal, metadata summaries, duplicate candidates, optional bounded
hashing, aggregated paginated views, recommendations only.

**Exit criteria:** bounded and cancellable under stress; resumable or safely
discardable; measured memory/throughput on 1M+ entries.

## 4. Release 2 — Process, startup, service visibility

Hierarchy, startup inventory, services, limitation explanations. Optional
user-confirmed control only after threat model, authorization, audit, recovery.

**Exit criteria:** read-only inventory complete and explained; control features
gated behind a security review and recovery drill.

## 5. Release 3 — Security intelligence

Evidence-based indicators, signature/hash/reputation status, Defender integration
where supported, quarantine/restore with integrity and audit.

**Exit criteria:** security review passed; restore verified; no heuristic labeled
as confirmed malware; no kernel driver.

## 6. Release 4 — Diagnostics and network

Event Log summaries, crash correlation, adapters/DNS/connections/ports, historical
trends with retention.

**Exit criteria:** privacy review for log-derived data; retention enforced;
permissions documented.

## 7. Release 5 — Managed maintenance

Schedules, policies, battery/resource-aware behavior, redacted export, reversible
maintenance, explainable health summary.

**Exit criteria:** all background work has visible status, pause/cancel policy, and
bounded resource use.

## 8. Post-V3 candidates (not scoped)

- Opt-in hash reputation service (requires consent, privacy review, and a server
  product decision).
- Fleet/multi-user administration (requires a different architecture).
- MSIX / enterprise deployment.
- Localization.
- Plugin/extension model (only if a real need appears).

## 9. Explicitly not on the roadmap

- Kernel driver / filesystem minifilter.
- Automatic deletion or "one-click clean".
- Account system, cloud sync.
- Cross-platform builds.

## 10. Decision gates

| Gate | Blocks | Status |
|---|---|---|
| Audience + supported Windows matrix | Release 0 exit | Open |
| Threat model | Any destructive feature | Not started |
| Data retention policy | Persistence | Open |
| Signing + distribution | Public release | Open |
| Performance baseline | Release 1 exit | Not started |
| License | Public distribution | Open |

See [`plan.md`](plan.md) §12 and [`DECISIONS.md`](DECISIONS.md).
