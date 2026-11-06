# Incident Response

> **Status:** Process draft. **No security contact, on-call rotation, or advisory
> tooling exists.** **Last updated:** 2026-11-06

---

## 1. Scope

Incidents include:

- a vulnerability in the app or its dependencies;
- a malicious or harmful release;
- compromise of signing keys, CI, or the distribution channel;
- user data loss or corruption caused by the app;
- a privacy breach (unexpected egress or data exposure);
- a security-relevant false negative/positive that could cause harm.

## 2. Roles

| Role | Responsibility | Assigned |
|---|---|---|
| Incident lead | Coordinates, decides, communicates | **TBD** |
| Engineering | Reproduces, fixes, verifies | TBD |
| Release/signing owner | Revokes keys, rebuilds, publishes | TBD |
| Communications | Advisory, user messaging | TBD |

Until assigned, the repository owner acts in all roles. This is a tracked gap
([`KNOWN_ISSUES.md`](KNOWN_ISSUES.md), [`RISKS.md`](RISKS.md) R-42).

## 3. Severity

| Level | Definition | Response target |
|---|---|---|
| **S0 Critical** | Active exploitation, code execution, signing key compromise, harmful build distributed | Immediate; stop distribution |
| **S1 High** | Data loss/corruption, privilege escalation, silent egress | 24–72 h |
| **S2 Medium** | Limited-impact security bug, audit bypass | 7 days |
| **S3 Low** | Hardening gap, defense-in-depth issue | Next release |

## 4. Intake

1. Report received privately ([`../SECURITY.md`](../SECURITY.md)).
2. Acknowledge within 3 business days (target).
3. Triage: severity, affected versions, exploitability, user impact.
4. Create a private tracking record; avoid public detail until fixed.
5. Assign an owner and a target date.

## 5. Response procedure

```text
1. Triage and confirm          -> severity, scope, affected versions
2. Contain                      -> disable feature, stop distribution, revoke key
3. Fix                          -> minimal patch + regression test
4. Verify                       -> reproduce pre-fix, confirm post-fix
5. Release                      -> signed build, advisory, upgrade guidance
6. Communicate                  -> what happened, who is affected, what to do
7. Post-mortem                  -> root cause, detection gap, prevention
```

### 5.1 Containment options

| Situation | Immediate action |
|---|---|
| Harmful build distributed | Remove artifacts, publish advisory, mark release as revoked |
| Signing key compromised | Revoke certificate, rotate keys, rebuild, notify users |
| CI compromise | Disable workflows, rotate secrets, audit artifacts |
| Data-loss bug | Disable the feature via release or config, publish recovery steps |
| Unexpected egress | Disable the feature, disclose exactly what was sent |

There is no remote kill switch and no telemetry channel. Containment depends on
distribution control and user communication — a strong argument for signing and an
updater.

## 6. Communication templates

**Advisory (fixed):**

```text
Title:  System Scanner <version> security update
Impact: <who is affected and how>
Fixed:  <version>
Action: Upgrade to <version>. If you cannot, <mitigation>.
Detail: <technical summary after users can update>
Credit: <reporter, if they consent>
```

**Harmful build:**

```text
Do not install System Scanner <version>. If installed, <steps>.
A replacement build will be published at <location>.
```

Rules:

- Do not downplay impact.
- Do not disclose exploit details before a fix is available.
- State uncertainty honestly.
- Keep a single canonical advisory location.

## 7. Post-mortem

Required for S0/S1. Include:

- timeline (detection → containment → fix → release);
- root cause;
- why existing controls/tests did not catch it;
- corrective actions with owners and dates;
- documentation updates (threat model, risks, tests).

Store with the incident record; add non-sensitive lessons to
[`DECISIONS.md`](DECISIONS.md) or [`RISKS.md`](RISKS.md).

## 8. Evidence handling

- Preserve logs, build artifacts, and the affected binary before rebuilding.
- Do not collect user file contents during investigation.
- Redact user identifiers in any shared report.

## 9. Drills

Before a public release, run one tabletop exercise:

1. "A signed build contains a data-deletion bug" — who stops distribution?
2. "The signing key leaked" — what is rotated, in what order?
3. "A user's DB is corrupt after upgrade" — what recovery guidance is given?

Record outcomes and gaps.

## 10. Readiness checklist

```text
[ ] Private vulnerability reporting enabled
[ ] Published security contact
[ ] Named incident lead and release/signing owner
[ ] Signing keys in a secret store with rotation procedure
[ ] Artifact revocation procedure documented
[ ] Advisory template ready
[ ] Users can determine their version
[ ] Backup/restore guidance published
[ ] Post-mortem template ready
```

Most items are currently **unchecked**; this is expected pre-alpha.
