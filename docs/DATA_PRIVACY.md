# Data Privacy

> **Status:** No data is collected or persisted today. This document defines the
> privacy rules **before** persistence exists.
> **Last updated:** 2026-11-06

---

## 1. Commitment

- **Local by default.** Inspection works offline. Nothing is uploaded.
- **No account.** No identity is required to use the product.
- **No telemetry.** No analytics, crash upload, or usage reporting.
- **No silent collection.** If a feature ever sends data, it says exactly what,
  to whom, and why — and asks first.
- **User control.** The user can inspect and delete local data.

## 2. Data classification

| Class | Examples | Sensitivity | Default handling |
|---|---|---|---|
| A — System facts | OS build, CPU model, RAM | Low | Persisted, exportable |
| B — Paths and names | file paths, process executables, usernames | **High** | Minimize, redact in exports/logs |
| C — Content | file contents, command lines, environment | **Critical** | Never collected |
| D — Derived | hashes, sizes, counts | Medium | Hashes are pseudonymous but identifying |
| E — Actions | user confirmations, quarantine records | Medium | Persisted for safety/audit |
| F — Diagnostics | logs, versions, error codes | Medium | Redacted, bounded, local |

Rules:

- Class C is **never** collected, logged, or exported.
- Class B is collected only when a feature requires it, and is redacted by default
  in any shared artifact.
- A file hash can identify known content; treat it as sensitive and never send it
  without consent.

## 3. Data inventory (proposed)

| Data | Purpose | Retention default | User can delete |
|---|---|---|---|
| System snapshots | Recent history | 30 days | Yes |
| Process snapshots | Recent history | 7 days | Yes |
| File records | Scan results | Until scan deleted / cap | Yes |
| File hashes | Duplicate detection | With file record | Yes |
| Security findings | User dispositions | 90 days | Yes |
| Action history | Restore + accountability | Until uninstall | **No** (see §6) |
| Quarantine objects | Isolation | Until user purges | Yes (restore/purge) |
| Logs | Diagnosis | 5 × 10 MB rolling | Yes |

All values are proposals pending the retention decision (`docs/plan.md` §12).

## 4. What never leaves the device

- File contents.
- Full directory listings.
- Process command lines.
- Usernames, SIDs, machine name, IP addresses.
- Logs, support bundles, database files.

Unless the user explicitly exports a bundle and shares it.

## 5. Optional network features

None exist. If a reputation lookup is ever added:

```text
[ ] Off by default; per-lookup consent
[ ] Payload shown before sending (hash + algorithm only)
[ ] No path, no name, no user identifier
[ ] Works offline with an "unavailable" state
[ ] Documented endpoint, retention, and legal basis
[ ] Revocable
```

## 6. Deletion and user rights

| Request | Implementation |
|---|---|
| Delete scan history | Delete session rows; cascade file records/hashes |
| Delete all data | Settings action + documented data directory |
| Delete a quarantine item | Purge with confirmation, after restore option |
| Delete audit history | **Not offered** while quarantine items exist (needed for restore); documented |
| Export my data | Redacted JSON export (proposed) |

Uninstall behavior is undecided. Until decided, uninstall must not delete data
silently. Document the choice in [`DEPLOYMENT.md`](DEPLOYMENT.md).

## 7. Redaction rules

Applied before writing logs or building a support bundle:

| Field | Redaction |
|---|---|
| Path | Keep drive + basename; hash the middle, or drop |
| Username | Remove or replace with `<user>` |
| Machine name | replace |
| IP / MAC | remove |
| Process command line | never captured |
| File hash | keep only if the user opted in to share |

Redaction is applied **at the call site**, not by a later scrubber.

## 8. Diagnostics and support bundles

- User-initiated only.
- Previewable before sharing.
- Excludes the raw DB, quarantine objects, and file contents.
- States what is included in plain language.

## 9. Children / sensitive environments

The product is not directed at children and collects no personal data by default.
No special category data (health, biometric, political) is processed.

## 10. Regulatory posture

No legal determination has been made. Because processing is local and no data is
transmitted, the GDPR/CCPA surface is limited — but **not zero**, because the app
stores paths and history on disk. See [`COMPLIANCE.md`](COMPLIANCE.md).

## 11. Privacy review triggers

- A new persisted entity.
- A new export format.
- Any network egress.
- A change to logging fields.
- A change to retention.

## 12. Open questions

1. Is file-level history retained at all by default?
2. Where do DB and quarantine live, and what are the ACLs?
3. Does uninstall delete data?
4. Is a redacted export required for V1?
5. Is audit history exempt from deletion, and how is that explained?
