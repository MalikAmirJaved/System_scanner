# Disaster Recovery

> **Status:** No data exists yet. This plan covers machine loss, corruption, and
> key compromise once persistence ships. **Last updated:** 2026-11-06
>
> **Distinction:** [`RECOVERY_PLAN.md`](RECOVERY_PLAN.md) covers in-app failures
> (corrupt DB, interrupted scan). This document covers losing the machine or the
> ability to ship.

---

## 1. Scenarios

| Scenario | Impact | RTO | RPO |
|---|---|---|---|
| Machine lost/stolen | Local history and quarantine gone | N/A (new machine) | Last backup |
| Disk failure | Same | Reinstall + restore | Last backup |
| Ransomware / destructive malware | Data encrypted or deleted | Hours | Last offline backup |
| OS reinstall | App data removed | < 30 min | Last backup |
| Profile corruption | App data unreadable | < 30 min | Last backup |
| Corrupt database | History inaccessible | < 10 min | Daily backup |
| Signing key loss | Cannot publish trustworthy updates | Days | Key escrow |
| CI compromise | Artifacts untrusted | Hours | Rebuild from clean state |
| Repository loss | Source lost | Hours | Remote mirror |

## 2. What is recoverable

| Item | Recoverable from |
|---|---|
| App binary | Rebuild from source / reinstall |
| Scan history | Backup (`VACUUM INTO`) |
| Quarantine items | Backup of `quarantine/` + manifest |
| Settings | Backup of `settings.json` |
| Source code | Git remote |
| Signing identity | Escrowed certificate / reissue |
| Release artifacts | Archive with checksums |

Not recoverable: unsaved in-flight scan state (by design), logs beyond retention.

## 3. User recovery procedure

```text
1. Reinstall System Scanner (signed installer, verified checksum).
2. Do NOT launch yet if restoring — close it.
3. Restore to %LOCALAPPDATA%\com.systemscanner.desktop\:
     app.db, settings.json, quarantine\ (+ manifest), optionally logs\
4. Launch. Verify schema version and history.
5. If the app reports a schema mismatch, install the matching version first.
6. Verify a quarantined item's hash before relying on it.
```

Never restore individual WAL/SHM files over a running database. Use the backed-up
`app.db` produced by `VACUUM INTO`.

## 4. Maintainer recovery

| Loss | Procedure |
|---|---|
| Signing key | Revoke certificate, publish advisory, reissue, sign new build, notify users |
| CI secrets | Rotate all secrets, audit recent runs/artifacts, rebuild |
| Repository | Restore from remote/mirror, verify tags and release hashes |
| Release artifacts | Rebuild from the tagged commit in a clean environment; compare hashes where possible |
| Domain/hosting | Keep a documented fallback channel; publish advisory |

## 5. Backup strategy (offline)

- **3-2-1**: three copies, two media, one off-site/offline.
- Backups must be verifiable and encrypted at rest (user responsibility if not
  implemented by the app).
- Test restore quarterly; a backup is unverified until restored.
- Keep one immutable/offline copy to survive ransomware.

Details for app-level backups: [`BACKUP_POLICY.md`](BACKUP_POLICY.md).

## 6. Ransomware considerations

- The app does not protect user files; it is a scanner, not a backup tool.
- Quarantine is not a backup: it stores potentially malicious files.
- If the data directory is encrypted, restore from an offline backup; do not pay.
- Document that backups on the same volume do not survive disk encryption.

## 7. Drill schedule

| Drill | Frequency | Owner |
|---|---|---|
| Restore DB backup | Quarterly | Maintainer |
| Restore quarantine item | Quarterly | Maintainer |
| Rebuild from tag + compare | Per release | Release owner |
| Key-loss tabletop | Before public release | Incident lead |
| Clean-machine install | Per preview release | QA |

Record results and gaps here:

| Date | Drill | Result | Notes |
|---|---|---|---|
| — | — | Not run | No persistence yet |

## 8. Communication during a disaster

- Publish status where users already look (release page / repository).
- State what is affected, what is safe, and what users should do.
- Do not speculate on cause before it is known.
- Follow up with a post-mortem for any data-loss event.

## 9. Open questions

1. Does the app offer an automatic backup destination outside the profile?
2. Are quarantine backups opt-in?
3. Who holds signing key escrow?
4. Is there a documented fallback distribution channel?
