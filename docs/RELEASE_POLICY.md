# Release Policy

> **Status:** Draft. No release process, signing, or versioning automation exists.
> **Last updated:** 2026-11-06

---

## 1. Versioning

Semantic versioning once a public release exists:

```text
MAJOR.MINOR.PATCH
MAJOR  incompatible data/behavior change, or a support commitment
MINOR  new capability, backward compatible
PATCH  fixes only
```

Pre-1.0 (`0.x`) may break compatibility; still document breaks in
[`CHANGELOG.md`](CHANGELOG.md).

Version must match in all three files:

```text
package.json
src-tauri/tauri.conf.json
src-tauri/Cargo.toml
```

A CI check should fail on divergence (Backlog B-083).

## 2. Channels

| Channel | Audience | Cadence | Stability |
|---|---|---|---|
| Internal | Maintainers | On demand | Any |
| Preview | Opt-in testers | Monthly | Feature-complete, may break |
| Stable | Everyone | When gates pass | Tested, signed |

There is no channel infrastructure today; this is the target.

## 3. Release gates

A release is blocked unless **all** apply:

### Build and test
```text
[ ] Formatting, clippy, type-check, tests pass on the Windows CI matrix
[ ] Release build succeeds with pinned toolchains
[ ] Installer builds and is signed
[ ] Version is consistent across manifests
```

### Quality
```text
[ ] No known critical/high exploitable dependency advisory untriaged
[ ] Performance budgets met or regressions justified
[ ] Accessibility checks on core flows
[ ] No new warnings suppressed without justification
```

### Safety
```text
[ ] Any destructive action is audited, confirmed, and recoverable
[ ] Recovery drill performed for changed data paths
[ ] Migrations tested from every supported prior version
```

### Operations
```text
[ ] Install, upgrade, rollback, uninstall tested
[ ] Offline operation verified
[ ] Non-admin operation verified
[ ] SBOM, checksums, provenance archived
[ ] Release notes: changes, limitations, data/privacy notes, known issues
[ ] Rollback plan documented
```

## 4. Approval

| Change type | Approver |
|---|---|
| Patch (fixes) | One maintainer |
| Minor (feature) | One maintainer + test evidence |
| Major / security-sensitive | Two reviewers + security review |
| Signing key use | Release owner only |

No self-approval for security-sensitive changes.

## 5. Release procedure

```text
1. Freeze scope; update CHANGELOG and version files.
2. Run the full local gate (build.md).
3. CI green on Windows.
4. Build installer; sign; generate checksums + SBOM.
5. Smoke test on a clean Windows VM (install, launch, read-only flow).
6. Upgrade test from the previous supported version.
7. Stage to internal -> preview.
8. Publish stable with notes and checksums.
9. Tag the commit; archive artifacts.
```

## 6. Rollback and revocation

| Situation | Action |
|---|---|
| Regression, no data change | Re-publish previous build; keep the new one unpublished |
| Data migration bug | Stop distribution, publish recovery guidance, restore backup path |
| Security issue | Stop distribution immediately; advisory; fixed build |
| Signing compromise | Revoke, rotate, rebuild, notify |

Artifacts are never silently replaced. If a build is withdrawn, say so and explain
why. The release owner has authority to stop distribution without a meeting.

## 7. Deprecation

- Announce a deprecation at least one minor release before removal.
- Keep reading old persisted data for at least one major version where feasible.
- Never silently drop user history.

## 8. Release notes template

```markdown
## System Scanner <version> — YYYY-MM-DD

### Highlights
### Changes
### Fixes
### Security
### Known limitations
### Data & privacy
### Upgrade notes
### Verification
- SHA-256: <installer hash>
- Signed by: <subject>
```

## 9. Post-release

- Monitor the issue tracker for crash/failure reports (no telemetry).
- Track "known issues" against the release.
- Start the next `[Unreleased]` section immediately.

## 10. Current gaps

| Gap | Effort |
|---|---|
| No CI gates | High |
| No signing | High |
| No updater | High |
| No VM test harness | Medium |
| No SBOM | Medium |
| No version-sync check | Low |
| No release checklist automation | Low |

These are prerequisites for the first *public* release, not for local development.
