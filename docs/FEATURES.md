# Features

> **Status vocabulary:** `Implemented` (code + verification exists), `Partial`
> (some code, incomplete or unverified), `Planned` (designed, not built),
> `Proposed` (an idea), `Not started`.
>
> **Last updated:** 2026-11-16
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
| F-006 | Application shell + design tokens | 0 | ✅ Implemented | `src/App.tsx`, `src/styles/globals.css` |
| F-007 | In-app navigation and routing | 0 | ⏳ Not started | — |
| F-008 | Error boundary + fatal-startup screen | 0 | ✅ Implemented — renderer boundary shows a reload screen; a failed Tauri startup shows a native dialog with a redacted detail, then exits 1 | `src/components/ErrorBoundary.tsx`, `src-tauri/src/startup.rs` |
| F-009 | Theming (light/dark/system) | 5 | 🟡 Partial — manual light/dark toggle persisted in `localStorage`; no OS-preference follow | `src/components/ThemeToggle.tsx`, `src/styles/globals.css` |
| F-010 | Accessible keyboard navigation | 0 | ⏳ Not started — focus-visible styling and semantic table/landmarks only | `src/styles/globals.css` |
| F-011 | Window state persistence | 5 | ⏳ Not started | — |
| F-012 | Responsive panel layout + component primitives | 0 | ✅ Implemented — 1/2/3-column Tailwind grid, shared `Button`, light/dark color tokens | `src/App.tsx`, `src/components/ui/button.tsx`, `src/styles/globals.css` |

## 2. IPC and application core

| ID | Feature | Release | Status | Evidence |
|---|---|---|---|---|
| F-020 | Typed command/event boundary | 0 | 🟡 Partial — commands done, events not started | `src-tauri/src/ipc.rs`, `src-tauri/src/commands/`, `src/services/ipc.ts` |
| F-021 | Stable DTOs + schema version | 0 | ✅ Implemented | `src-tauri/src/dto/app_info.rs` (`DTO_SCHEMA_VERSION = 1`) |
| F-022 | Error taxonomy + correlation IDs | 0 | ✅ Implemented (envelope-level; `unsupported` and `internal` are reachable from the snapshot command) | `src-tauri/src/error.rs`, `src/services/ipc.ts` |
| F-023 | Job registry + lifecycle state machine | 0 | ⏳ Not started | — |
| F-024 | Cancellation tokens per job | 0 | ⏳ Not started | — |
| F-025 | Bounded concurrency + resource budgets | 1 | ⏳ Not started | — |
| F-026 | Structured logging + rotation | 0 | ✅ Implemented — `tracing` to `app_log_dir()`, 10 MB × 5 files, redaction guard; command outcomes logged without payloads | `src-tauri/src/logging.rs`, `ipc.rs` |
| F-027 | Startup reconciliation of interrupted jobs | 1 | ⏳ Not started | — |
| F-028 | Runtime settings store | 1 | ⏳ Not started | — |
| F-029 | SQLite persistence foundation (driver, schema v1, migration runner) | 1 | 🟡 Partial — `rusqlite` opens `app_data_dir()/system-scanner.db` at startup with WAL/foreign keys, applies forward-only checksummed migrations, and creates `scan_sessions`; no repository, command, or row writer exists | `src-tauri/src/adapters/persistence/`, `src-tauri/migrations/0001_initial.sql` |

## 3. System overview (Release 0)

| ID | Feature | Status | Notes |
|---|---|---|---|
| F-030 | OS version, edition, build, architecture | ✅ Implemented | Registry `CurrentVersion` + `GetNativeSystemInfo`; `adapters/windows/system_info.rs` |
| F-031 | CPU model, physical/logical cores | ✅ Implemented | Registry `ProcessorNameString`, `GetActiveProcessorCount`, `GetLogicalProcessorInformationEx` |
| F-032 | Memory total/available | ✅ Implemented | `GlobalMemoryStatusEx` |
| F-033 | Disk volumes, capacity, free space, type | ✅ Implemented | Network drives report `unsupported` sizes to stay bounded |
| F-034 | Network adapters + addresses | ✅ Implemented | `GetAdaptersAddresses`; no connectivity test |
| F-035 | Unavailable-field reporting | ✅ Implemented | `unavailable[]` with `access_denied` / `unsupported` / `failed` |
| F-036 | Manual refresh + last-updated time | ✅ Implemented | `SystemOverview.tsx` |
| F-037 | Bounded periodic refresh + pause | ⏳ Not started | — |
| F-038 | Stale-response protection | ✅ Implemented | One in-flight refresh; superseded responses are discarded |

## 4. Processes, startup, services (Release 2)

| ID | Feature | Status |
|---|---|---|
| F-040 | Process list (PID, name, path, memory, CPU) | 🟡 Partial — PID, parent PID, name, threads, working set, and start time implemented; image path and CPU usage are not collected |
| F-041 | Stable process identity (PID + start time) | ✅ Implemented — `key` is `pid-startTime`, or an explicit `pid:<pid>` when the start time is denied |
| F-042 | Process hierarchy | 🟡 Partial — parent PID is exposed per row; no tree view |
| F-043 | Access-limited indicator per process | ✅ Implemented — per-row `unavailable[]` with `access_denied` / `failed` |
| F-044 | Startup entries inventory | ⏳ Not started | |
| F-045 | Service inventory (read-only) | ⏳ Not started | |
| F-046 | Explanation of source/privilege/limits | ⏳ Not started | |
| F-047 | Terminate process (gated, confirmed, audited) | 🚫 Not planned until gated | |
| F-048 | Start/stop service (gated, confirmed, audited) | 🚫 Not planned until gated |

Evidence for the partial and implemented rows: `src-tauri/src/{domain/process.rs,
ports/processes.rs, adapters/windows/processes.rs, dto/process_list.rs,
commands/processes.rs}`, `src/components/ProcessList.tsx`, `API.md` §3.3.

## 5. Storage (Release 1)

| ID | Feature | Status |
|---|---|---|
| F-050 | Drive discovery + selection | ⏳ Not started |
| F-051 | Cancellable directory traversal | 🟡 Partial — bounded worker prototype in `adapters/filesystem`, cooperative cancel, depth/entry budgets; no IPC, UI, or persistence wiring |
| F-052 | Per-entry outcome reporting | 🟡 Partial — `ok`/`skipped`/`denied`/`disappeared`/`unsupported`/`failed` produced by the traversal prototype; not rendered or persisted |
| F-053 | Large-file summary | ⏳ Not started |
| F-054 | Extension/type summary | ⏳ Not started |
| F-055 | Age filters | ⏳ Not started |
| F-056 | Duplicate candidates (size + partial fingerprint) | ⏳ Not started |
| F-057 | Optional bounded hashing (SHA-256/BLAKE3) | ⏳ Not started |
| F-058 | Paginated/virtualized results | ⏳ Not started |
| F-059 | Storage recommendations (no auto-cleanup) | ⏳ Not started |
| F-060 | Scan session history | ⏳ Not started |
| F-061 | Treemap/sunburst visualization + text alternative | ⏳ Not started |

F-051 and F-052 ship only as the B-022 prototype: a bounded, cancellable walker
with an explicit outcome per entry. It is not exposed over IPC, has no UI, writes
no rows, and does not follow reparse points. Drive discovery, summaries, and
duplicate detection remain not started.

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
| F-080 | Diagnostics view (versions, availability) | ✅ Implemented — in-shell panel (`Diagnostics.tsx`, `get_diagnostics`): app/Tauri versions, OS build/arch, elevation, log size, unavailable fields. No feature-availability probe and no routing |
| F-081 | Redacted support bundle | ✅ Implemented — on-request preview and copy (`domain/diagnostics.rs`, `redact.rs`), path/identity/network-address redaction, 256 KiB log tail. Saved-file export is not implemented |
| F-082 | Event Log summaries | ⏳ Not started |
| F-083 | Crash/application-error correlation | ⏳ Not started |
| F-084 | Active connections + listening ports | ⏳ Not started |
| F-085 | Process ownership of connections | ⏳ Not started |
| F-086 | DNS configuration view | ⏳ Not started |
| F-087 | Historical resource trends | ⏳ Not started |

F-080 and F-081 ship the Release 0 subset (versions, environment, log preview).
Event Log summaries, crash correlation, and connection features remain Release 4.

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
| Implemented | 26 (23 features + 3 baseline properties) |
| Partial | 7 (F-009 OS-follow theming; F-020 event channel; F-029 persistence foundation; F-040 process fields; F-042 hierarchy view; F-051 traversal prototype; F-052 outcome reporting) |
| Not started / planned | 37 |
| Gated (requires security work) | 5 |
| Research / proposed | 2 |

Implemented since the scaffold: the typed IPC envelope, the error taxonomy with
correlation IDs, four versioned DTOs, four read-only commands (`get_app_info`,
`get_system_snapshot`, `list_processes`, `get_diagnostics`), the
port/domain/Windows-adapter split for system information, processes, and
diagnostics, and UI views for backend status, the system snapshot, the paged
process inventory, and on-request diagnostics with a redacted support-bundle
preview. Panels have loading, error, retry, refresh, paging, and per-field
unavailable reasons. The renderer uses Tailwind CSS v4 design tokens, a
responsive panel grid that reflows from one to three columns, and a persisted
light/dark toggle. The backend writes bounded, redacted local logs for command
outcomes and startup, and redacts the preview before returning it. A render
failure is caught by a boundary that replaces the dashboard with a reload screen,
and a startup failure that prevents a window from existing shows a native dialog
and exits non-zero instead of panicking. Persistence groundwork is in place: a
bundled-SQLite connection opens at startup, applies forward-only checksummed
migrations, and creates schema v1 (`scan_sessions`) — but no feature writes or
reads rows yet. A traversal prototype now streams filesystem entries through a
bounded channel with a fixed worker count, cooperative cancellation, depth/entry
budgets, and an explicit outcome per entry; it is not yet reachable from the UI.

The event channel, job registry, cancellation, saved-file bundle export, image
paths, and everything that touches storage are still ahead.
