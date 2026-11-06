# Integrations

> **Status:** No integration is implemented. The app currently uses only Tauri's
> window/runtime APIs. **Last updated:** 2026-11-06

---

## 1. Integration principles

1. **Documented APIs only.** Prefer Microsoft-documented interfaces over internal
   NT APIs.
2. **Adapters isolate the platform.** Domain code never learns which API answered.
3. **Every call is fallible.** Access denied, unsupported, timeout, and disappearing
   targets are normal outcomes.
4. **No silent egress.** A network integration requires consent and disclosure.
5. **No elevation by default.** An integration that requires admin is scoped to a
   single operation.

## 2. Windows integrations

| Integration | Release | Purpose | Privilege | Failure mode | Status |
|---|---|---|---|---|---|
| Win32 system info (`GetSystemInfo`, `GlobalMemoryStatusEx`, `GetVersionEx`/RtlGetVersion) | 0 | OS/CPU/memory | User | `Unsupported`, partial | Planned |
| `GetLogicalDrives` / `GetDiskFreeSpaceEx` | 0 | Disks, capacity | User | Per-drive outcome | Planned |
| IP Helper API (`GetAdaptersAddresses`) | 0 | Network adapters | User | Empty + reason | Planned |
| Toolhelp / `NtQuerySystemInformation` process snapshot | 0 | Process list | User | `ACCESS_DENIED` per process | Planned |
| Registry (`RegOpenKeyEx`) | 2 | Startup entries | User | `ACCESS_DENIED`, continue | Planned |
| Service Control Manager | 2 | Service inventory | User (read) | `ACCESS_DENIED` | Planned |
| Process termination | 2 | Control (gated) | User/Admin | Explicit failure + audit | **Not planned until gated** |
| Signature verification (`WinVerifyTrust`) | 3 | Authenticode status | User | `unknown`, never `clean` | Planned |
| Windows Defender (`MpCmdRun`/WMI/Defender APIs) | 3 | Status/scan | User/Admin | `UNSUPPORTED` + explanation | Research |
| Event Log (`EvtQuery`, `wevtapi`) | 4 | Diagnostics | User | Partial + reason | Planned |
| WMI/CIM | 4 | Correlated diagnostics | User | Timeout, never block UI | Planned |
| Quarantine storage | 3 | Isolation | User | Disk full, ACL error | Planned, gated |

### 2.1 Win32 / `windows` crate

- Purpose: primary system data source.
- Constraints: `unsafe` in the adapter only; explicit error conversion; no blocking
  calls on the async reactor.
- Testing: contract tests behind traits, plus Windows-only integration tests.

### 2.2 WMI / CIM

- Often the easiest way to read Windows configuration, but slow and fallible.
- Rules: timeout + cancellation where possible; never on the UI thread; treat
  partial results as normal; do not assume the service is running.

### 2.3 Registry

- Read-only in the initial scope. Reads may be access-denied or redirected by
  WOW64; document the view (32/64-bit) for each key.
- Never write to the registry without a separate, gated, reversible design.

### 2.4 Windows Defender

- Only documented interfaces. Research required before promising behavior.
- Never claim "your PC is protected" from a status field alone.
- A failed query is `UNSUPPORTED`/`unknown`, never "off".

### 2.5 Event Log

- Reading can be slow and permission-sensitive; bound the query window and count.
- No full-log export by default (privacy).

## 3. Runtime and platform integrations

| Integration | Purpose | Status | Notes |
|---|---|---|---|
| WebView2 | Renderer | Implemented | Evergreen; bootstrapped on Win10 |
| NSIS | Installer | Implemented | x64, unsigned |
| Tauri updater | Updates | Not implemented | Requires signed manifests |
| Windows code signing (Authenticode) | Trust | Not implemented | Blocks public distribution |
| SmartScreen reputation | Install trust | Not implemented | Expect warnings initially |

## 4. External services

**None.** There is no server, no cloud, no third-party API, and no account system.

If a reputation or update service is added, it must be documented here with:

- endpoint(s), protocol, and payload;
- consent and disclosure;
- offline behavior;
- retention and privacy impact;
- failure and rate-limit behavior.

## 5. Integration failure policy

| Failure | Behavior |
|---|---|
| API unavailable on this Windows version | `UNSUPPORTED` with an explanation |
| Access denied | Report the count, continue, offer elevation guidance |
| Timeout | Abort that collector, mark partial, keep the rest |
| Target disappeared | Record the outcome, continue |
| Service not running | Empty result with an explicit reason |
| Invalid data returned | Validate, discard the record, log at debug, count as `failed` |

## 6. Security review triggers

An integration requires a threat-model update when it:

- can modify system state (files, registry, services, processes);
- requires elevation;
- sends data off the device;
- reads sensitive content (documents, credentials, browser data — none are in scope);
- adds a third-party binary or driver.

## 7. Testing integrations

- Contract test the trait with a fake adapter.
- Integration-test the real adapter on Windows CI or a VM.
- Record the Windows build used; APIs differ across versions.
- Never assert exact localized strings from the OS.

## 8. Open questions

1. Which Defender interfaces are supported and documented for third-party use?
2. Is WMI acceptable for Release 4, or should everything be Win32?
3. Should Event Log reading be opt-in due to privacy sensitivity?
4. Is MSIX relevant for distribution, or is NSIS sufficient?
