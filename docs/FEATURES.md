# Features

> **Status vocabulary:** `Implemented` (code + verification exists), `Partial`
> (some code, incomplete or unverified), `Planned` (designed, not built),
> `Proposed` (an idea), `Not started`.
>
> **Last updated:** 2026-11-06
>
> This inventory is the ground truth for "does this exist?". If code and this
> table disagree, the code wins for behavior and this table must be corrected.

---

## 1. Platform and shell

| ID | Feature | Release | Status | Evidence |
|---|---|---|---|---|
| F-001 | Tauri 2 desktop application launches | 0 | ✅ Implemented | `src-tauri/src/main.rs`, `lib.rs` |
| F-002 | React 19 + TS + Vite renderer | 0 | ✅ Implemented | `src/main.tsx`, `package.json` |
| F-003 | Production build + NSIS installer | 0 | ✅ Implemented | `bundle/nsis/*_x64-setup.exe` |
| F-004 | Strict CSP, no remote origins | 0 | ✅ Implemented | `tauri.conf.json` |
| F-005 | Minimal capability set (`core:default`) | 0 | ✅ Implemented | `capabilities/default.json` |
| F-006 | Welcome screen + design tokens | 0 | ✅ Implemented | `src/App.tsx`, `src/App.css` |
| F-007 | Application shell, navigation, routing | 0 | ⏳ Not started | — |
| F-008 | Error boundary + fatal-startup screen | 0 | ⏳ Not started | `lib.rs` uses `expect()` |
| F-009 | Theming (light/dark/system) | 5 | ⏳ Not started | — |
| F-010 | Accessible keyboard navigation | 0 | ⏳ Not started | — |
| F-011 | Window state persistence | 5 | ⏳ Not started | — |

## 2. IPC and application core

| ID | Feature | Release | Status | Evidence |
|---|---|---|---|---|
| F-020 | Typed command/event boundary | 0 | ⏳ Not started | `lib.rs` has no handlers |
| F-021 | Stable DTOs + schema version | 0 | ⏳ Not started | — |
| F-022 | Error taxonomy + correlation IDs | 0 | ⏳ Not started | — |
| F-023 | Job registry + lifecycle state machine | 0 | ⏳ Not started | — |
| F-024 | Cancellation tokens per job | 0 | ⏳ Not started | — |
| F-025 | Bounded concurrency + resource budgets | 1 | ⏳ Not started | — |
| F-026 | Structured logging + rotation | 0 | ⏳ Not started | — |
| F-027 | Startup reconciliation of interrupted jobs | 1 | ⏳ Not started | — |
| F-028 | Runtime settings store | 1 | ⏳ Not started | — |

## 3. System overview (Release 0)

| ID | Feature | Status | Notes |
|---|---|---|---|
| F-030 | OS version, edition, build, architecture | ⏳ Not started | Needs `RtlGetVersion` or equivalent |
| F-031 | CPU model, physical/logical cores | ⏳ Not started | — |
| F-032 | Memory total/available | ⏳ Not started | — |
| F-033 | Disk volumes, capacity, free space, type | ⏳ Not started | — |
| F-034 | Network adapters + addresses | ⏳ Not started | No connectivity test |
| F-035 | Unavailable-field reporting | ⏳ Not started | Required, not optional |
| F-036 | Manual refresh + last-updated time | ⏳ Not started | — |
| F-037 | Bounded periodic refresh + pause | ⏳ Not started | — |
| F-038 | Stale-response protection | ⏳ Not started | — |

## 4. Processes, startup, services (Release 2)

| ID | Feature | Status |
|---|---|---|
| F-040 | Process list (PID, name, path, memory, CPU) | ⏳ Not started |
| F-041 | Stable process identity (PID + start time) | ⏳ Not started |
| F-042 | Process hierarchy | ⏳ Not started |
| F-043 | Access-limited indicator per process | ⏳ Not started |
| F-044 | Startup entries inventory | ⏳ Not started |
| F-045 | Service inventory (read-only) | ⏳ Not started |
| F-046 | Explanation of source/privilege/limits | ⏳ Not started |
| F-047 | Terminate process (gated, confirmed, audited) | 🚫 Not planned until gated |
| F-048 | Start/stop service (gated, confirmed, audited) | 🚫 Not planned until gated |

## 5. Storage (Release 1)

| ID | Feature | Status |
|---|---|---|
| F-050 | Drive discovery + selection | ⏳ Not started |
| F-051 | Cancellable directory traversal | ⏳ Not started |
| F-052 | Per-entry outcome reporting | ⏳ Not started |
| F-053 | Large-file summary | ⏳ Not started |
| F-054 | Extension/type summary | ⏳ Not started |
| F-055 | Age filters | ⏳ Not started |
| F-056 | Duplicate candidates (size + partial fingerprint) | ⏳ Not started |
| F-057 | Optional bounded hashing (SHA-256/BLAKE3) | ⏳ Not started |
| F-058 | Paginated/virtualized results | ⏳ Not started |
| F-059 | Storage recommendations (no auto-cleanup) | ⏳ Not started |
| F-060 | Scan session history | ⏳ Not started |
| F-061 | Treemap/sunburst visualization + text alternative | ⏳ Not started |

## 6. Security intelligence (Release 3)

| ID | Feature | Status |
|---|---|---|
| F-070 | Signature metadata via WinVerifyTrust | ⏳ Not started |
| F-071 | Evidence-based suspicious indicators | ⏳ Not started |
| F-072 | Classification states (known/suspicious/unknown/not scanned) | ⏳ Not started |
| F-073 | Rule versioning + provenance | ⏳ Not started |
| F-074 | Windows Defender status (supported interfaces only) | 🔬 Research |
| F-075 | Quarantine with integrity + ACLs | 🚫 Gated |
| F-076 | Restore with verification | 🚫 Gated |
| F-077 | Audit history for actions | 🚫 Gated |
| F-078 | Optional reputation lookup (consent) | 💭 Proposed |

## 7. Diagnostics and network (Release 4)

| ID | Feature | Status |
|---|---|---|
| F-080 | Diagnostics view (versions, availability) | ⏳ Not started |
| F-081 | Redacted support bundle | ⏳ Not started |
| F-082 | Event Log summaries | ⏳ Not started |
| F-083 | Crash/application-error correlation | ⏳ Not started |
| F-084 | Active connections + listening ports | ⏳ Not started |
| F-085 | Process ownership of connections | ⏳ Not started |
| F-086 | DNS configuration view | ⏳ Not started |
| F-087 | Historical resource trends | ⏳ Not started |

## 8. Maintenance and automation (Release 5)

| ID | Feature | Status |
|---|---|---|
| F-090 | User-defined schedules | ⏳ Not started |
| F-091 | Resource/battery-aware scan policy | ⏳ Not started |
| F-092 | Exportable redacted reports | ⏳ Not started |
| F-093 | Reversible maintenance workflows | ⏳ Not started |
| F-094 | Explainable health summary | ⏳ Not started |

## 9. Non-functional capabilities

| ID | Capability | Status |
|---|---|---|
| N-001 | Zero network egress | ✅ Holds today (no client) |
| N-002 | No telemetry | ✅ Holds today |
| N-003 | Unelevated operation | ✅ Holds today |
| N-004 | Reversible destructive actions | 🚫 No destructive action exists |
| N-005 | Installer signing | ⏳ Not started |
| N-006 | Auto-update | ⏳ Not started |
| N-007 | Crash-free session tracking (local) | ⏳ Not started |
| N-008 | Accessibility baseline | ⏳ Not started |

## 10. Summary

| Status | Count |
|---|---|
| Implemented | 9 (6 features + 3 baseline properties) |
| Not started / planned | 63 |
| Gated (requires security work) | 6 |
| Research / proposed | 2 |

Only the shell, build, installer, CSP, capability set, and welcome screen are real,
plus the current absence of network access, telemetry, and elevation. Everything
user-facing that inspects the machine is still ahead.
