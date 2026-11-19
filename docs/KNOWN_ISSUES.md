# Known Issues & Limitations

> **Status:** Active · **Last updated:** 2026-11-19
>
> This is a list of **real** limitations, not a wish list. Severity reflects user
> impact. Each item should link to a backlog ID where one exists.

---

## 1. Critical / blocking for release

| ID | Issue | Impact | Workaround | Tracked |
|---|---|---|---|---|
| KI-001 | ✅ Resolved 2026-11-19 — startup and service collection exist | `get_startup_inventory` reports six autostart sources and the `SERVICE_WIN32` service list, with per-source and per-field limitation reasons; the remaining coverage gaps (scheduled tasks, drivers, 32-bit registry view, shortcut targets) are KI-058 | — | B-043 |
| KI-002 | Seven read-only IPC commands exist; the UI can drive a scan but has no storage views or service actions | A scan shows counters only: no file list, sizes by folder, or treemap, and no folder picker | None | B-050; no storage-view item exists yet |
| KI-003 | Persistence is write-only and minimal | Schema v2 holds `scan_sessions`; a scan writes one counters-only row, but no IPC command reads history, no UI shows it, and nothing stores per-entry data | None needed; browsing is unaffected | B-024 (per-entry), B-030 |
| KI-004 | No code signing | SmartScreen warns; tampering undetectable | Install anyway, verify hash manually | B-080 |
| KI-005 | No license selected | Cannot be legally redistributed | Internal use only | B-084 |
| KI-006 | Tests cover IPC, runtime payload validation, the snapshot/process/diagnostics contracts, redaction, persistence migrations, the traversal prototype, and five UI panels, and the scan view reducer | Process actions are untested because they do not exist; the scan panel is tested against mocked IPC only (never against the running app); no visual/theme test exists | Manual checks only | B-002, B-008, B-021, B-022, B-090 |
| KI-007 | CI has never run on GitHub | A workflow file is not a gate; broken builds can still merge | Run the gates locally per `build.md` | B-010, B-092 |

## 2. High

| ID | Issue | Impact | Notes |
|---|---|---|---|
| KI-012 | The support bundle has no saved-file export | The preview can be copied, but support must paste it instead of attaching a file | F-092 / B-064 |
| KI-013 | Cancellation exists only for the scan job | `cancel_scan` and window-close cancel work for one in-memory scan; there is no general job registry and no cancel for other work. A killed process leaves its session row open until the next startup marks it `interrupted`; nothing resumes it | Design in `SYSTEM_DESIGN.md` |
| KI-049 | Scan roots are caller-supplied and only shape-checked plus a device-namespace block | Device and NT-namespace roots are refused, but any other absolute path, including UNC shares and removable media, can be walked read-only; reparse points are never followed; only counters are returned | The B-040 review (2026-11-17) accepted this for read-only use; network/removable root policy is still an open decision (`THREAT_MODEL.md` §11.7). The Storage panel accepts typed absolute paths | B-041 |
| KI-055 | ✅ Resolved 2026-11-19 — a directory can no longer be swapped between the reparse check and enumeration | A directory is now opened **once** without following its final link, and the volume serial + 128-bit file id read from that handle are compared with the id the parent recorded; enumeration uses the same handle, so the checked object is the enumerated object. A mismatch is reported as `disappeared`. Children are classified from the enumeration record instead of a second by-path stat | Identity is unverifiable where a volume supplies no file id (KI-056); roots that are plain files are still classified by path, read-only | B-041, `THREAT_MODEL.md` §11.8 |
| KI-056 | Identity checks are unverifiable where the volume supplies no file id | The handle-based check cannot prove the opened object is the recorded one, so the walk continues instead of refusing; no content is read today | Expected for `FileIdInfo`-unsupported volumes and for the non-Windows fallback, which records no id at all. Must become a typed `unsupported` outcome before any content read or write on such a volume | B-041; `THREAT_MODEL.md` §11.8 |
| KI-057 | The protected-path policy is implemented but nothing consults it | `domain/policy.rs` decides deny-by-default over the system roots with a stable reason and is unit-tested, but no command calls it: no command changes state, and the read-only walker must stay able to scan system directories (counters only) | It cannot be bypassed while no write path exists. The first state-changing command must call `evaluate` and report the denial before acting, never after | B-041, B-047, B-048; `THREAT_MODEL.md` §11.8 |
| KI-050 | `scan_sessions` rows have no retention limit and no delete control | Each scan adds a counters-only row (no paths) that stays forever; the live job state still resets on restart | Delete the database file with the app closed; a retention policy and UI need the `docs/plan.md` §12 decision |
| KI-051 | Session persistence is best-effort | A failed `begin`/`finish` write is logged and the scan still reports normally, so a scan can exist with no row, or a row can stay `running` until the next startup marks it `interrupted` | Intentional: the app must work without a database; check the log for `scan session` warnings |
| KI-014 | No DB corruption recovery, and restore is manual | A corrupt database is preserved and the app continues without persistence, but nothing renames it aside, offers export, or restores a backup. Pre-migration snapshots now exist (B-033) in `backups/` (newest five), but there is no restore command, no scheduled/daily backup, and snapshots are best-effort, so a failed snapshot does not stop a migration. Workaround: with the app closed, copy a snapshot over `system-scanner.db` | B-033, `RECOVERY_PLAN.md` |
| KI-015 | No updater | Security fixes cannot reach users | `DEPLOYMENT.md` §7 |

## 3. Medium

| ID | Issue | Impact | Notes |
|---|---|---|---|
| KI-020 | CSP allows `style-src 'unsafe-inline'` | Style injection if an XSS exists | Accepted risk; revisit |
| KI-021 | Release uses `panic = "abort"` | No unwinding; a panic kills the app | Intentional for size; requires disciplined error handling |
| KI-022 | Only NSIS x64 bundle | No MSI, no ARM64, no portable | `tauri.conf.json` |
| KI-023 | No accessibility verification | Unknown keyboard/screen-reader quality | B-066 |
| KI-024 | Only one screen | No navigation or routing; all six panels share one view | Accepted: single-dashboard design (B-004) |
| KI-025 | No OS-preference theming | Light/dark is manual only; the app ignores the Windows light/dark setting | F-009 |
| KI-026 | `gen/schemas` is gitignored | Capability autocomplete missing on fresh clones | Regenerated by build |
| KI-027 | `rust-toolchain.toml` pins `stable`, not an exact compiler version | Local and CI can drift within stable | B-011 |
| KI-058 | The startup/service inventory is incomplete by design | Scheduled tasks, driver services, the 32-bit `WOW6432Node` registry view, and the target of a Startup shortcut are not read; a service whose configuration is denied shows no start type or binary path. The inventory cannot start, stop, or disable anything | Read-only by ADR-028; service control waits for B-049's confirmation/audit work. A denied field is always named in `unavailable`, so a gap is never rendered as a value | B-043, ADR-028 |
| KI-028 | Version is duplicated in 3 files | Drift risk | B-083 |
| KI-029 | No `.editorconfig` | Line-ending/style drift on Windows | Optional |
| KI-036 | 10 moderate advisories in the dev dependency tree (`minimatch` ReDoS via ESLint/`typescript-eslint`) | Affects lint tooling only, not the shipped binary; `npm audit --omit=dev` is clean | Accept until upstream releases; do not force-downgrade ESLint |

## 4. Low

| ID | Issue | Impact | Notes |
|---|---|---|---|
| KI-030 | `build.md` marks planned commands clearly; they fail if run | Confusing only if misread | Intentional honesty |
| KI-031 | `src/assets/react.svg`, `public/vite.svg` are template leftovers | Minor clutter | Safe to remove when branding is final |
| KI-032 | No favicon branding | Cosmetic | — |
| KI-033 | `dist/` and `src-tauri/target/` exist in the working tree | Disk usage; ignored by git | `gitignore` covers them |
| KI-034 | Default window size 960×720 may be small for tables | Now applicable: the process table scrolls horizontally below 540 px | Revisit with the app shell (B-004) |
| KI-037 | Prettier does not check Markdown or CSS | Prose and hand-authored CSS can drift in style | Intentional: `.prettierignore` keeps authored files byte-stable |
| KI-038 | Network drives report `unsupported` size/label/filesystem | Network volumes show no capacity; deliberate to avoid multi-second blocking on dead mappings | ADR-012; query the volume manually in Explorer |
| KI-039 | The native adapters are verified on one machine | Edition-, architecture-, and media-specific behavior is not covered by CI hardware | The B-032 baseline now measures the traversal walker on one machine (`PERFORMANCE.md` §1.2); every other adapter is still unmeasured. Add a manual step to the release checklist |
| KI-040 | The process list has no command lines, CPU usage, or session IDs | Rows cannot show arguments, CPU hogs are not visible, and two processes of the same image are not attributable to a logon session | Image paths arrived with B-042 (ADR-027); the rest stay out deliberately — command lines are a privacy decision, CPU needs sampling, and a session ID needs a hand-written import (ADR-013) |
| KI-041 | Working set is `access_denied` for processes owned by another account | SYSTEM and other-user rows show no memory figure | Expected without elevation; the app will not request elevation for a read-only view |
| KI-042 | The process list is sorted by PID and has no search or filter | Locating a process by name means paging through the list | Deliberate: no unvalidated sort enum. A client-side filter over the current page would be misleading, so it waits for a Rust-side query |
| KI-043 | Support-bundle redaction is best-effort | A pattern the redactor does not recognize (for example a bare personal name or an unusual path form) could reach the preview | No log call site may rely on redaction alone; the preview is user-initiated and reviewable before copying. Extend rules with tests when a new field is logged |
| KI-044 | Traversal results are counters only | The Storage scan panel shows aggregate counts; no entry is stored or listed, so F-051/F-052 remain partial | Per-entry persistence waits on the retention decision (B-024) |
| KI-052 | The scan panel has not been exercised in the running app | Its tests mock the IPC layer, so event delivery over real Tauri, the capability for `listen`, and a real cancel are unverified end to end. `core:default` is confirmed from the generated manifest to include `allow-listen` | Run `npm run tauri dev`, scan a small folder, cancel a second scan, and watch the log |
| KI-053 | Scan roots are typed, not picked | A mistyped or relative path is refused only after the request; there is no picker, no drive list (F-050), and no remembered roots | Folder picker needs a dialog plugin and capability review |
| KI-045 | Reparse points are skipped, never followed | Files behind a junction, symlink, or OneDrive/cloud placeholder are counted as `skipped` and their contents are not scanned; totals can understate disk usage | Deliberate for prototype safety (ADR-019). Following needs a visited set by volume + file ID |
| KI-046 | Denied-directory fixture is Windows-only and unproven in CI | B-023 added a live test that denies listing via `icacls` and restores it on drop; it passed locally on one machine, but CI has never run and the test is `#[cfg(windows)]` | Run `cargo test` locally; confirm on the first CI run |
| KI-048 | A failed directory-stream read is counted but has no entry | `read_dir` item errors increment `failed` without a `FileEntry`, so `entries` can be lower than the sum of outcomes and the path is unknown | Rare; surface per-directory when persistence (B-024) needs exact accounting |
| KI-047 | Traversal order is nondeterministic | Parallel workers race on one queue; entry order varies between runs, so ordered exports or golden files would be unstable | Documented; sort explicitly if a feature requires stable order |
| KI-054 | IPC payload guards are hand-maintained (replaces KI-011 after B-090) | A DTO change is only reflected when its guard changes: a rename or type change fails loudly as `contract_violation`, but a *new* field stays invisible to the UI | Each guard sits beside the type it mirrors and has a valid plus a drifted test (`src/services/contracts.test.ts`); no generated schema | ADR-022 |

## 5. Known non-issues (documented to prevent "fixes")

| Behavior | Why it is intentional |
|---|---|
| No network access | Product promise; not a bug |
| No admin requirement | Least privilege |
| Heuristics do not produce "malware" verdicts | Evidence-first design |
| No deletion features | Safety; not yet designed |
| No telemetry | Privacy |
| `#![allow]`-free modules with no dead code | Warnings are errors in CI; suppression is scoped and justified |

## 6. Environment-specific limitations

| Environment | Limitation |
|---|---|
| Windows 10 | WebView2 may need bootstrapping; APIs vary by build |
| Non-NTFS volumes | File IDs may be unavailable; dedup falls back to path/size |
| Network drives | Traversal may be slow or unavailable; not yet supported |
| Removable media | May disappear mid-scan; must be handled as `disappeared` |
| Long paths | Requires `LongPathsEnabled` for some operations |
| Non-admin sessions | Some process/service metadata will be access-limited |

## 7. Reporting a new issue

Include:

1. app version and Windows build;
2. exact steps and expected vs actual;
3. whether it requires elevation;
4. log excerpt with paths redacted;
5. whether it is reproducible on a clean profile.

Security issues: [`../SECURITY.md`](../SECURITY.md), not this file.
