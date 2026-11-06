# Deployment

> **Status:** Build and NSIS bundling work. Signing, auto-update, upgrade testing,
> and release automation do **not** exist. **Last updated:** 2026-11-06

---

## 1. Distribution model (current and proposed)

| Item | Current | Proposed |
|---|---|---|
| Format | NSIS x64 installer | Signed NSIS; MSI/portable only if required |
| Channel | Manual local build | Internal → Preview → Stable |
| Update mechanism | None | Tauri updater with signed manifests |
| Signing | None | EV/OV code-signing certificate |
| Hosting | None | Release artifact host with checksums |
| Data location | None yet | `%LOCALAPPDATA%\com.systemscanner.desktop\` |

Open decisions: distribution channel and signing ownership
([`plan.md`](plan.md) §12). Do not invent a channel before it is decided.

## 2. Build the release

```powershell
npm ci
npm run tauri build
```

Artifacts:

```text
src-tauri/target/release/system-scanner.exe
src-tauri/target/release/bundle/nsis/System Scanner_0.1.0_x64-setup.exe
```

Verified on 2026-11-06: installer ~1.4 MB, unsigned.

## 3. Installer behavior

- Per-user install by default (NSIS), no elevation unless the installer requires
  a machine-wide location.
- Installs the app binary and WebView2 bootstrapper fallback.
- Does not install a service, driver, or scheduled task.
- Uninstall removes program files. **Data deletion policy is undecided** — until
  decided, uninstall must not delete user data silently.

## 4. Data locations (proposed)

```text
%LOCALAPPDATA%\com.systemscanner.desktop\
  app.db              SQLite database
  app.db-wal / -shm   WAL sidecars
  logs\               rotating diagnostic logs
  quarantine\         isolated files + manifest
  exports\            user-generated reports (optional)
```

Rules:

- Never store data in `Program Files` or the repository.
- Apply ACLs so only the current user can read quarantine and logs.
- Document whether data survives uninstall (privacy requirement).

## 5. Upgrade procedure (proposed)

```text
1. Verify new build signature + checksum.
2. Close the running app (or let the installer do it).
3. Install over the existing version (NSIS in-place).
4. On first launch: run migrations inside a transaction.
5. Verify: app version, schema version, existing history readable.
6. If migration fails: keep the old DB, show a recovery notice, do not delete.
```

Requirements before shipping upgrades:

- Test upgrade from **every supported prior version**, not only the last one.
- Test a downgrade/rollback path (schema forward-compatibility or a documented
  "cannot roll back" warning).
- Never silently discard history to make an upgrade succeed.

## 6. Rollback

| Scenario | Action |
|---|---|
| Bad build, no schema change | Reinstall previous installer; data intact |
| Bad build with schema change | Restore DB backup taken before upgrade; reinstall previous |
| Security issue in a build | Stop distribution, publish advisory, provide a fixed build |

Rollback criteria and stop-distribution authority:
[`RELEASE_POLICY.md`](RELEASE_POLICY.md).

## 7. Code signing

Not configured. Required before public distribution:

```powershell
# Proposed CI secrets (never in the repo)
TAURI_SIGNING_PRIVATE_KEY
TAURI_SIGNING_PRIVATE_KEY_PASSWORD
# Certificate for Authenticode (CI secret store)
```

- Sign the app binary and the installer.
- Timestamp signatures.
- Verify with `signtool verify /pa /v <artifact>`.
- Expect SmartScreen warnings for a new, low-reputation certificate; disclose this
  honestly rather than claiming a warning-free install.

## 8. Release procedure (proposed)

```text
[ ] Version bumped in package.json + tauri.conf.json + Cargo.toml (keep in sync)
[ ] CHANGELOG updated
[ ] CI green on the Windows matrix
[ ] Tests, clippy, build pass
[ ] Installer built, signed, checksum recorded
[ ] SBOM generated and archived
[ ] Install smoke test on clean Windows 11 VM
[ ] Upgrade smoke test from previous version
[ ] Offline launch verified
[ ] Non-admin run verified
[ ] Uninstall verified
[ ] Release notes: changes, limitations, data/privacy notes, known issues
[ ] Stage to internal/preview before stable
```

## 9. Offline installation

The product promises offline operation. Therefore:

- The installer must not require a network connection to install the app itself
  (WebView2 bootstrapper may, on old Windows 10 — document this).
- No first-run license check, account, or telemetry ping.
- Verify with networking disabled in a VM.

## 10. Support bundle

Planned: a zip with app version, OS build, redacted logs, feature availability, and
DB/schema version. Must be previewable and redacted before the user shares it.

## 11. Known deployment gaps

| Gap | Impact |
|---|---|
| Unsigned installer | SmartScreen warning, tampering risk |
| No updater | Users cannot receive security fixes |
| No upgrade testing | Migration failures reach users |
| No MSI | Enterprise deployment limited |
| No checksums published | Manual download unverifiable |
| No uninstall data policy | Privacy ambiguity |

Tracked in [`KNOWN_ISSUES.md`](KNOWN_ISSUES.md).
