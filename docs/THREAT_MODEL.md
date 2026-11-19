# Threat Model

> **Status:** Reviewed for the **implemented read-only file surface** (B-040,
> 2026-11-17, §11). Destructive, privileged, and networked operations remain
> **unreviewed** and must not ship. **Last updated:** 2026-11-19

---

## 1. System under analysis

A local Windows desktop application that inspects system state and, eventually,
quarantines files and controls processes/services with user confirmation.

**Deployment:** one installer, one user session, no server.
**Components:** WebView2 renderer, Tauri IPC boundary, Rust core, Windows adapters,
SQLite store, quarantine directory, NSIS installer.

## 2. Assets

| Asset | Why it matters | Where it lives |
|---|---|---|
| User files | Deletion/corruption is irreversible | Filesystem |
| Quarantine contents | May be malware; must not be executable/leaked | `%LOCALAPPDATA%\...\quarantine` |
| Scan history | Reveals user activity and paths | SQLite |
| Logs / support bundles | May contain paths, usernames | `logs/` |
| Signing keys | Compromise enables malicious updates | CI secret store |
| User trust | The product's only real asset | — |
| System integrity | App must not weaken the machine | OS |

## 3. Trust boundaries

```text
[ OS / filesystem / registry / WMI ]  ← untrusted, fallible
        │ adapters
[ Rust core ]                         ← trusted after validation
        │ IPC (untrusted input)
[ Tauri boundary + capabilities ]     ← enforcement point
        │
[ WebView2 renderer ]                 ← assumed compromisable
        │
[ Network ]                           ← absent by design
```

| Boundary | Direction | Trust | Control |
|---|---|---|---|
| Renderer → IPC | inbound | **Untrusted** | Schema validation, capability check, policy check |
| IPC → Renderer | outbound | Trusted data, untrusted sink | DTO allowlist, no handles, size limits |
| Core → Win32 | outbound | OS is fallible/hostile | Explicit error handling, timeouts |
| Core → filesystem (write) | outbound | Hostile names/links | Identity verification, protected paths, TOCTOU handling |
| Core → DB | local | Tamperable by the user/other processes | ACLs, integrity checks, no secrets |
| Build → release | supply chain | Critical | Signing, lockfiles, SBOM, review |

## 4. Adversaries

| Adversary | Capability | Goal |
|---|---|---|
| A1 Local unprivileged user/process | Can read app data dir, modify DB/logs, craft files | Tamper with audit, read quarantine |
| A2 Malware already on the host | Can create hostile files/names/links, hook APIs | Exploit the scanner to escalate or hide |
| A3 Malicious removable media | Controls file names, directory structure, junctions | Trigger traversal bugs, path confusion |
| A4 Compromised renderer (XSS/dependency) | Can call any permitted IPC command | Reach the OS, exfiltrate data |
| A5 Network attacker | Would need egress; none today | Intercept future reputation lookups |
| A6 Supply-chain attacker | Compromised dependency or update channel | Ship code execution to users |
| A7 Social engineer targeting the user | Can make the UI show a scary finding | Get the user to approve a destructive action |

## 5. Abuse cases and mitigations

| ID | Abuse case | Impact | Mitigation | Residual risk |
|---|---|---|---|---|
| T-01 | Malicious filename injects SQL/log/HTML | Injection, spoofed UI | Parameterized queries; escape output; never `dangerouslySetInnerHTML` | Low |
| T-02 | Junction/symlink redirects a write to a protected path | System file destroyed | Do not follow reparse points for writes; verify volume + file ID | Medium — design-time; no write path exists (identity check now available) |
| T-03 | TOCTOU: file replaced between scan and quarantine | Wrong file quarantined/deleted | Open by handle, re-verify identity/hash, operate on handle | Medium — F-01 is the concrete window to close |
| T-04 | Crafted IPC payload (huge limit, `..\` path) | DoS, path escape | Clamp limits, canonicalize, reject traversal, bound payload size | Low |
| T-05 | Compromised renderer invokes privileged command | Unauthorized system change | Capability gating + Rust-side policy + confirmation nonce | Low |
| T-06 | Quarantine file executed by user | Malware runs | Store non-executable (renamed + ACL), document, never auto-open | Low |
| T-07 | Quarantine file replaced/modified | Restore wrong content | SHA-256 on store/restore, fail closed | Low |
| T-08 | Audit log edited to hide an action | Lost accountability | Append-only + hash chain; ACLs | Medium (local admin can always win) |
| T-09 | Reparse loop / deep tree DoS | Hang or OOM | Visited-set by file ID, depth cap, cancellation | Low — reparse points skipped; depth/entry caps (F-07) |
| T-10 | Fake "critical malware" finding tricks user into deleting data | Data loss | Evidence + confidence, no auto-delete, explicit consequence text | Medium |
| T-11 | Dependency compromise | Code execution | Lockfiles, review, advisory gate, signing | Medium |
| T-12 | Unsigned installer replaced in transit | Malware install | Sign + checksums + HTTPS distribution | High until signing |
| T-13 | Support bundle leaks paths/usernames | Privacy breach | Redaction by construction, preview before share | Low |
| T-14 | A directory is swapped for a junction between the reparse check and `read_dir` | Read outside the selected root (today); wrong file read/written later | Handle-based open + identity check (implemented 2026-11-19; F-01 closed, §11.8) | Low — mismatch reported; unverifiable volumes remain (KI-056) |

## 6. STRIDE summary

| Category | Applies to | Primary controls |
|---|---|---|
| **S**poofing | Update channel, installer, findings | Signing, checksums, rule provenance |
| **T**ampering | DB, quarantine, audit, logs | Hashes, ACLs, hash chain, integrity checks |
| **R**epudiation | User actions | Append-only audit with confirmation text |
| **I**nformation disclosure | Logs, bundles, quarantine | Redaction, ACLs, minimal collection |
| **D**enial of service | Traversal, IPC, DB | Bounds, cancellation, timeouts, paging |
| **E**levation of privilege | Renderer→OS, app→admin | Capabilities, validation, no auto-elevation |

## 7. Assumptions

1. The OS enforces user ACLs correctly.
2. WebView2 is patched by the OS/vendor.
3. The user's account is not already fully compromised (a local admin can always
   tamper with local data; we do not claim otherwise).
4. The signing key is stored outside the repository and CI logs.
5. No network egress exists in the baseline.

## 8. Out of scope

- Kernel-mode attacks and rootkits actively subverting user-mode APIs.
- Physical access.
- Multi-user/fleet scenarios.
- Protecting the user from themselves if they ignore clear warnings and confirm a
  destructive action (we minimize this through design but cannot eliminate it).

## 9. Open questions

1. Is the audit hash chain required for a single-user app? (See `AUDIT_LOGGING.md`.)
2. How do we present a quarantine item without enabling accidental execution?
3. What is the policy for scanning network shares and removable media?
   *(Still open after B-040: §11.7.)*
4. Do we need a confirmation nonce to prevent replay of an approved action?
5. How is tampering with the local DB detected and communicated?

## 10. Review triggers

- Any new write/delete path.
- Any elevation.
- Any network egress.
- Any new capability or plugin.
- Before each public release.

---

## 11. Review 2026-11-17 — file operations (B-040)

> **Status:** Complete for the surface that exists in code. This is a **paper
> review against source and tests**, not a penetration test, fuzzing run, or live
> ACL/network fixture. The conclusion applies only to read-only scanning.

### 11.1 Scope and method

Read in full and checked against the tests:

- `start_scan` / `cancel_scan` / `get_scan_status` and the `scan:*` event stream
  (`src-tauri/src/commands/scan.rs`, `src-tauri/src/dto/scan.rs`,
  `src-tauri/src/app/jobs.rs`);
- the traversal adapter and the outcome taxonomy
  (`src-tauri/src/adapters/filesystem/traversal.rs`,
  `src-tauri/src/domain/traversal.rs`);
- the only database writes on the scan path, and the pre-migration snapshot
  (`src-tauri/src/adapters/persistence/`).

Out of scope because they are not implemented: delete, rename, move, ACL change,
quarantine, process/service control, elevation, hashing, signature/reputation
lookup, and any network egress.

### 11.2 Surface as implemented

| Property | Behavior | Where |
|---|---|---|
| Who chooses targets | The renderer sends 1–64 root strings; the user types one absolute path per line. No folder picker, no privileged source, no path from a scan result | `ScanPanel.tsx`, `dto/scan.rs` |
| Shape validation | Non-empty, no NUL, ≤ 32,767 bytes, absolute; `\\.\`, `\??\`, and any `\\?\` form that is not a drive or UNC path rejected; `maxEntries` 1–100,000,000 with the ceiling as default; unknown fields rejected | `dto/scan.rs` |
| Concurrency and budgets | One scan at a time (`busy` otherwise); workers, channel capacity, depth (64) and entry ceiling are chosen in Rust, never from IPC | `app/jobs.rs`, `domain/traversal.rs` |
| Root classification | `symlink_metadata` before queueing; only a real directory is opened; a junction/symlink root is reported as `reparse_point` and skipped | `seed_root` |
| Walk | Per child: `symlink_metadata`, reparse points skipped, depth cap, shared entry budget, cancellation checked between entries | `visit_child`, `Shared::emit` |
| Failure reporting | `denied`, `disappeared`, `unsupported`, `failed` are distinct counters; a directory-stream error increments `failed`; truncation is explicit | `domain/traversal.rs` |
| Data leaving Rust | Aggregate counters only. No path, name, or per-entry row crosses IPC, is persisted, or is logged; the session `options` JSON holds budgets only | `ScanCountersDto`, `jobs.rs` |
| Persistence | One counters-only `scan_sessions` row per scan; startup marks stale rows `interrupted`; pre-migration snapshots use `VACUUM INTO` plus read-only `quick_check`, newest five | `persistence/`, B-033 |
| Privilege and network | Standard user, no elevation, no HTTP client, no shell, no fs/shell plugins | `tauri.conf.json`, `Cargo.toml` |

### 11.3 Findings

#### F-01 — Check-then-open: a directory can be swapped for a junction before `read_dir`

`visit_directory` calls `fs::symlink_metadata(path)` and, when that shows a real
directory, later calls `fs::read_dir(path)`. These are two separate opens **by
path**. `read_dir` does not pass `FILE_FLAG_OPEN_REPARSE_POINT`, so on Windows it
follows a reparse point that replaced the directory between the two calls. The
reparse check therefore does not bind the object that is actually enumerated.
`seed_root` has the same shape (stat, queue, re-stat on visit) and narrows but does
not close the window.

**Impact today:** read-only. The walk can enumerate entries outside the intended
root and add them to counters. Nothing is read, written, hashed, or persisted.
An attacker able to replace a directory under the user's profile already has the
write access needed to do so.

**Impact once content is read or written:** a file outside the selected root
could be hashed, opened, moved, or deleted as if it were inside. T-03 stops being
hypothetical.

**Required before any content read or write:**

- open with `FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS`;
- compare `GetFileInformationByHandle` volume serial + 64-bit file ID with the
  entry the parent recorded, and enumerate through that handle instead of
  re-opening by path;
- report an identity change as `disappeared`/`skipped`, never as success.

**Status: closed 2026-11-19 (see §11.8).** All three requirements are met for
directories: `adapters/filesystem/directory.rs` opens once with both flags, reads
the volume serial and 128-bit file id from that handle (`FileIdInfo`), enumerates
children through the same handle (`FileIdExtdDirectoryInfo`, fixed-size batches),
and the walker compares the parent-recorded id with the opened id before
descending. A mismatch is reported as `disappeared`.

Residual: a volume that cannot supply a file id yields an *unverifiable* check,
which is not treated as a match but also cannot prove one (KI-056). A root that
is a plain file is still classified by path; that is read-only classification and
there is no handle to bind while nothing is read. The write path must not copy
this pattern (see `SECURITY.md` §7.6).

#### F-02 — Arbitrary absolute roots, including UNC and removable media (open decision)

Every absolute path is accepted: local drives, UNC shares, mapped network drives,
and removable media. Device and NT namespaces are refused, but nothing classifies
a path as network or removable.

**Impact today:** a scan of an unreachable share blocks one worker until the OS
returns an error — the UI stays responsive because it polls, but the scan stalls
and there is no per-I/O timeout. A reachable share consumes bandwidth and its
existence plus rough tree shape are reflected in counters. Removable media
supplies hostile names, deep trees, and junctions; the depth/entry caps and
reparse skip-bound those, so there is no traversal escape today.

**Decision needed:** block network paths, warn and require explicit consent, or
allow only roots chosen through a folder picker. This is a product decision, not a
code bug; it stays open (§11.7) rather than being guessed.

#### F-03 — File names and metadata are treated as hostile

Names are counted and classified, never rendered in the UI, interpolated into SQL,
or written to logs. The scan's log statement contains budgets only. Keeping paths
out of logs is a deliberate privacy control, not an accident. Any future path
display, export, or query must escape or parameterize; a name is not a trust
signal.

#### F-04 — Budgets bound memory, not wall-clock time

Four workers, a 1,024-entry channel, depth 64, a 100,000,000-entry ceiling, one
active scan, and cooperative cancellation keep memory flat because entries stream
instead of accumulating. They do not bound duration: a large tree can scan for
hours, the entry ceiling is a safety limit rather than a target, and there is no
per-directory or per-I/O timeout. Cancellation is observed between entries, so a
worker blocked inside `read_dir` on a dead share does not see it until the OS
returns.

#### F-05 — What the database and backup actually contain

One counters-only row per scan, no paths, plus the `options` budgets. Pre-migration
snapshots are written with `VACUUM INTO` under app data, reopened read-only, and
must pass `quick_check`; the newest five are kept. The database is protected by
the user-profile ACL only — any process running as the same user can read it —
which matches assumption A1. No encryption is claimed. A failed snapshot logs a
warning and startup continues, acceptable only while every migration is additive
(ADR-023).

#### F-06 — A scan is not a security verdict

No hash, signature, or classification exists, so no UI can call a file clean or
malicious. `denied`, `skipped`, `disappeared`, and `truncated` prevent a
complete-looking result from being read as full coverage. T-10 (a fake "critical"
finding tricking the user) is unreachable until B-045 and must be reviewed then.

#### F-07 — Reparse points are skipped, not followed

The right default for a counter-only scan: no visited set is needed because
nothing is followed, so a junction loop cannot occur. The cost is that content
behind a junction or cloud placeholder is invisible; it is reported as
`reparse_point`/`skipped`, not silently omitted. Following links for full-volume
coverage would need a visited set keyed by volume + file ID and an explicit
policy; neither exists.

### 11.4 Threat mapping after this review

| Threat | Effect of the review |
|---|---|
| T-02 | No write path exists and reparse points are skipped. The identity check the write path must use is implemented (F-01 closed), and so is the protected-path policy (`domain/policy.rs`), but no command consults either yet. |
| T-03 | Same. F-01 is the concrete window to close before quarantine. |
| T-04 | Mitigated for the current payload: shape and enum checks, bounded roots/entries, unknown fields rejected. |
| T-09 | Partially mitigated: no link following, depth and entry caps, cancellation; no visited set needed while nothing is followed. |
| T-13 | Scan paths never reach logs or IPC. Support-bundle redaction is reviewed separately (B-006). |
| T-14 | New: F-01 recorded as a threat; bounded today, blocking before writes. |

### 11.5 Residual risks and owners

| Risk | Today | Must close before | Owner |
|---|---|---|---|
| F-01 check-then-open race | **Closed 2026-11-19** for directories (§11.8) | — | — |
| F-01 residual: volume supplies no file id | Check reports unverifiable and continues; nothing is read (§11.8) | Any content read or write on such a volume | B-041, KI-056 |
| F-02 UNC/network/removable roots | Accepted, bounded, can stall a worker | Public release, or any claim about network scanning | Open product decision; enforcement would extend B-041 |
| No per-I/O timeout | UI responsive; a scan can stall on a dead share | Network roots allowed by policy | Not scheduled |
| Protected-path policy exists but is not enforced | Module and tests only; unreachable while nothing changes state (§11.8) | First state-changing command | B-041, KI-057 |
| No audit of scans | Read-only; no accountability need yet | First state-changing action | B-046 |
| No fuzzing of the walker or IPC parsers | Unit tests only | Public release | `TESTING.md` gap |

### 11.6 Conclusion

The implemented read-only file surface is **acceptable for development and
pre-alpha use**. It is not a release claim: no destructive operation exists to
threat-model, and none may be added by copying the current walker. Before the
first content read, hash, or write (B-041 onward):

1. ~~close F-01 with handle-based identity checks~~ — done 2026-11-19 (§11.8);
2. decide F-02 (network/removable policy) and implement it in Rust;
3. extend this section for the new operation, including audit and recovery.

### 11.7 Open decisions raised by this review

1. Are UNC/network shares scanned, warned about, or refused — and is a root
   allow-list tied to a folder picker the mechanism?
2. Are removable volumes scanned by default or only after confirmation?
3. Will the walker ever follow reparse points (with a visited set) to reach
   content behind junctions, or is "skip and report" the permanent contract?

### 11.8 Update 2026-11-19 — F-01 closed by handle-based directory access

> **Status:** Implemented and tested on Windows; read-only behavior unchanged.

- New `adapters/filesystem/directory.rs`: `DirectoryHandle::open` uses
  `CreateFileW` with `FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS`
  and `FILE_SHARE_READ | WRITE | DELETE`, and prefixes `\\?\` so paths beyond
  `MAX_PATH` open without relying on `LongPathsEnabled`.
- Identity is the volume serial plus 128-bit file id (`FileIdInfo`) read from the
  opened handle; children are enumerated through that handle
  (`FileIdExtdDirectoryInfo`), which also yields each child's id.
- The walker opens a directory **once**. It compares the identity the parent
  recorded against the opened identity before emitting the directory as `ok` or
  queueing it; `IdentityCheck::Changed` becomes `disappeared`, and a reparse
  attribute seen on the opened handle becomes `skipped`/`reparse_point`.
  Children are classified from the enumeration record, so the per-child
  `symlink_metadata` by path is gone too.
- Bounded memory is unchanged: enumeration is read in 16 KiB batches and at most
  one batch is held. The `.` and `..` records are skipped.
- Tests: adapter identity stability, junction report, missing directory, a
  1,200-entry directory spanning several buffers, and walker-level tests for a
  1,500-file directory and a junction root (`cargo test`, 144 passing).
- The protected-path policy the first write path must consult is implemented in
  `domain/policy.rs`: deny-by-default over the system roots, component-aware,
  case-insensitive, lexical, with a reason per denial; an unresolvable path is
  denied rather than assumed safe. It is **not enforced anywhere yet** — no
  command changes state, and the read-only walker must remain able to scan system
  directories (KI-057).
- Still open: a volume without file ids is unverifiable (KI-056); F-02 root
  policy is a product decision; file roots are classified by path.
