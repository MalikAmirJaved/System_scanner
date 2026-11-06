# Risk Register

> **Status:** Active · **Last updated:** 2026-11-06
>
> Likelihood/Impact: Low / Medium / High. Owner is a role, not a person, until
> ownership is assigned. Accepted risks are recorded, not hidden.

---

## 1. Product and business risks

| ID | Risk | L | I | Mitigation | Owner | Status |
|---|---|---|---|---|---|---|
| R-01 | Product is mistaken for an antivirus and trusted as one | High | High | Explicit terminology, evidence + confidence, in-app disclaimers, no "infected" verdicts | Product | Open |
| R-02 | Users expect a "clean my PC" button and bounce | High | Medium | Clear recommendations + explanations; no unsafe automation | Product | Open |
| R-03 | Undecided audience (learning vs consumer vs internal) leads to waste | High | High | Resolve plan §12 decision 1 before Release 1 | Product | **Open — blocking** |
| R-04 | No license blocks public distribution | High | High | Choose a license before any public release | Legal | Open |
| R-05 | No revenue/distribution model | Medium | Medium | Decide before Release 1 scope grows | Product | Open |
| R-06 | Scope creep into kernel/EDR territory | Medium | High | Explicit non-goals; ADR required to change | Eng | Controlled |

## 2. Technical risks

| ID | Risk | L | I | Mitigation | Owner | Status |
|---|---|---|---|---|---|---|
| R-10 | Filesystem traversal exhausts memory on multi-million-file volumes | High | High | Streaming, bounded channels, aggregation, memory budget, fixtures | Eng | Design only |
| R-11 | UI freezes during scans | High | High | Workers off the UI thread, coalesced events, virtualization | Eng | Design only |
| R-12 | TOCTOU / reparse-point abuse turns a scan into a write primitive | Medium | Critical | Identity verification, handle-based ops, protected paths, threat model, tests | Security | Not started |
| R-13 | SQLite corruption or interrupted migration loses history | Medium | High | WAL, transactions, pre-migration backup, recovery path | Eng | Not started |
| R-14 | `panic = "abort"` turns a recoverable bug into a crash | Medium | Medium | No unwrap on input, typed errors, interrupted-session recovery | Eng | Accepted |
| R-15 | Win32 API differences across Windows builds produce wrong data | High | Medium | Explicit `Unsupported`, version tests, VM matrix | Eng | Open |
| R-16 | WMI/Event Log calls hang the UI | Medium | High | Timeouts, cancellation, `spawn_blocking`, bounded queries | Eng | Design only |
| R-17 | Dependency bloat reintroduces a large attack surface | Medium | Medium | Dependency policy, `cargo-deny`, review | Eng | Open |
| R-18 | Unsigned installer trains users to click through SmartScreen | High | High | Sign early, publish checksums, document warnings honestly | Release | Open |
| R-19 | No test suite means silent regressions | High | High | Test runner + required tests per change | Eng | **Open** |
| R-20 | Performance claims are unmeasured | High | Medium | Baseline fixtures + published methodology | Eng | Open |

## 3. Security risks

| ID | Risk | L | I | Mitigation | Owner | Status |
|---|---|---|---|---|---|---|
| R-30 | Compromised webview reaches the OS | Low | Critical | No OS access in UI, minimal capabilities, validation in Rust | Security | Partial |
| R-31 | Malicious filename/path causes injection or traversal | Medium | High | Never build SQL/shell strings from input; canonicalize; reject traversal | Security | Not started |
| R-32 | Quarantine store tampered with or used to hide malware | Low | High | Integrity hashes, ACLs, audit chain | Security | Not started |
| R-33 | Update channel compromised | Low | Critical | Signed manifests, TLS, protected keys | Release | Not started |
| R-34 | Dependency advisory unpatched | Medium | High | Advisory scanning, Dependabot, release gate | Eng | Open |
| R-35 | Sensitive data leaks via logs or support bundle | Medium | High | Redaction by construction, preview before export | Security | Not started |
| R-36 | Elevation prompt social-engineered | Medium | High | Narrow scope, explanation, no auto-elevate, audit | Security | Design only |
| R-37 | App itself becomes an attack vector on a compromised host | Low | High | Least privilege, no auto-actions, evidence-only output | Security | Partial |

## 4. Operational risks

| ID | Risk | L | I | Mitigation | Owner | Status |
|---|---|---|---|---|---|---|
| R-40 | No CI means a broken build ships | High | Medium | Add Windows CI before Release 0 exit | Eng | Open |
| R-41 | No upgrade testing breaks user data | Medium | High | Upgrade matrix, migration backups | Eng | Open |
| R-42 | No support channel for security reports | Medium | High | Publish contact + advisory process | Ops | Open |
| R-43 | No rollback plan for a harmful build | Low | Critical | Release policy + stop-distribution authority | Release | Draft |
| R-44 | Users cannot self-diagnose failures | High | Medium | Diagnostics view + support bundle | Eng | Not started |
| R-45 | Windows 10 end-of-support changes the matrix | Medium | Medium | Decide support policy explicitly | Product | Open |

## 5. Compliance and privacy risks

| ID | Risk | L | I | Mitigation | Owner | Status |
|---|---|---|---|---|---|---|
| R-50 | Unexpected personal data retention (paths, usernames) | Medium | High | Data classification, retention limits, redaction | Privacy | Open |
| R-51 | GDPR/CCPA obligations if any data leaves the device | Low | High | Local-only baseline; consent gates | Privacy | Open |
| R-52 | Third-party license violations in dependencies | Medium | Medium | SBOM + license review + `cargo-deny` | Legal | Open |
| R-53 | Anti-cheat/AV false-positive flags the app | Medium | Medium | Sign, avoid injection/driver behavior, submit for review | Release | Open |

## 6. Risk acceptance log

| Date | Risk | Decision | Rationale |
|---|---|---|---|
| 2026-11-06 | R-14 (`panic=abort`) | Accepted | Binary size + simplicity; mitigated by explicit error handling |
| 2026-11-06 | CSP `style-src 'unsafe-inline'` | Accepted | Needed by current styling approach; revisit if XSS surface grows |

## 7. Review cadence

- Re-score at every release gate.
- Any "Critical" impact risk requires an owner and a dated mitigation before a
  public release.
- New risks from a feature must be added in the same PR that introduces it.
