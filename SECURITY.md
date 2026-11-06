# Security Policy

> **Status:** Pre-alpha. No security review has been performed. No release is
> supported for production use. **Last updated:** 2026-11-06

---

## 1. Supported versions

| Version | Supported | Notes |
|---|---|---|
| 0.1.x | Development only | Unsigned, unverified, pre-alpha |

No version is currently eligible for a security support commitment. This table
must be updated before the first public release.

## 2. Reporting a vulnerability

**Do not open a public issue for a security problem.**

Preferred channel (once the repository is public): GitHub **private
vulnerability reporting** for this repository.

If private reporting is unavailable, contact the maintainer directly through the
address published in the repository profile. A dedicated security contact is
**not yet established** — this is a tracked gap in
[`docs/INCIDENT_RESPONSE.md`](docs/INCIDENT_RESPONSE.md) and
[`docs/KNOWN_ISSUES.md`](docs/KNOWN_ISSUES.md).

Please include:

- affected version and Windows build;
- reproduction steps and expected vs actual behavior;
- impact assessment (data loss, privilege escalation, code execution, privacy);
- whether the issue requires elevation, local access, or a malicious file.

**Response targets (aspirational, not yet staffed):** acknowledge within 3
business days, initial assessment within 10, fix or mitigation plan within 30 for
high severity.

We will not pursue legal action against good-faith research that respects user
data, avoids service disruption, and is disclosed privately first.

## 3. Scope

In scope:

- the Tauri IPC boundary and capability configuration;
- the Rust core, Windows adapters, and filesystem handling;
- persistence, quarantine, and export paths;
- installer, update path, and bundled assets;
- any network egress.

Out of scope:

- vulnerabilities in Windows itself, WebView2, or third-party drivers;
- physical access attacks;
- issues requiring an already-compromised administrator account, unless the app
  makes that compromise materially worse.

## 4. Security model (summary)

The product is a **local, single-user desktop application** that inspects a
machine which may already be compromised. Therefore:

- The UI is treated as potentially compromised; it holds no authority.
- File names, paths, process names, and metadata are attacker-controlled input.
- The OS boundary (Win32) is fallible and must fail closed.
- Nothing destructive runs without explicit confirmation and a recovery path.

Full analysis: [`docs/THREAT_MODEL.md`](docs/THREAT_MODEL.md).

## 5. Controls implemented today

| Control | Evidence | Notes |
|---|---|---|
| Minimal webview capability set | `src-tauri/capabilities/default.json` → `core:default` only | No fs/shell/http plugins |
| Content Security Policy restricting remote origins | `src-tauri/tauri.conf.json` | `default-src 'self'`; no `connect-src` to the internet |
| No HTTP client dependency | `src-tauri/Cargo.toml`, `package.json` | Zero network capability in code |
| No shell execution | no shell plugin, no `Command::new` | |
| Release binary hardening | `Cargo.toml`: LTO, strip, `panic = "abort"` | Note: abort means no unwind recovery |
| No telemetry | no analytics dependency | |
| Minimal dependency set | 3 runtime JS deps; 1 Rust dep | Small attack surface |
| No elevation manifest | `tauri.conf.json` | Runs as the invoking user |

## 6. Known gaps (must close before any public release)

| Gap | Risk | Tracked |
|---|---|---|
| No code signing | Installer tampering, SmartScreen warnings | `docs/DEPLOYMENT.md` |
| No updater / signed update channel | Users stay on vulnerable builds | `docs/RELEASE_POLICY.md` |
| No dependency advisory gate in CI | Silent vulnerable transitive deps | `docs/DEVOPS.md` |
| No SBOM | Unverifiable supply chain | `docs/DEVOPS.md` |
| No threat-model review of file operations | Path traversal / TOCTOU risk when writes land | `docs/THREAT_MODEL.md` |
| No audit trail implementation | No accountability for control operations | `docs/AUDIT_LOGGING.md` |
| No security contact published | Slow disclosure | `docs/INCIDENT_RESPONSE.md` |
| CSP allows `style-src 'unsafe-inline'` | Style injection if XSS exists | Revisit when styling changes |
| No fuzzing of IPC parsers | Parser crashes | `TESTING.md` |

## 7. Security requirements for contributors

1. Treat every IPC payload as hostile; validate before use.
2. Never `unwrap()`/`expect()` on input-derived data in a command path.
3. Keep `unsafe` inside Windows adapters, documented and reviewed.
4. Do not log secrets, file contents, full command lines, or unnecessary paths.
5. Do not add a plugin without reviewing its permission surface.
6. Any new filesystem write path requires: protected-path policy, identity
   verification, TOCTOU handling, and audit.
7. Do not weaken the CSP to make a library work; fix the library or wrap it.
8. Report dependency advisories immediately; do not "wait for the next release".

Enforceable AI-specific rules: [`AI_SECURITY_RULES.md`](AI_SECURITY_RULES.md).

## 8. Privacy posture

- No account, no telemetry, no analytics, no crash upload by default.
- Any optional network feature requires explicit consent, a preview of what
  leaves the device, and documentation.
- Local diagnostic export is previewable and redacted by default.

Details: [`docs/DATA_PRIVACY.md`](docs/DATA_PRIVACY.md).

## 9. Disclosure and incident handling

See [`docs/INCIDENT_RESPONSE.md`](docs/INCIDENT_RESPONSE.md) for severity levels,
containment, stop-distribution criteria, and user communication.

## 10. Review cadence

| Trigger | Action |
|---|---|
| Any new IPC command | Boundary and authorization review |
| Any new capability/plugin | Capability minimization review |
| Any filesystem write/delete | Threat model update + recovery test |
| Any network egress | Privacy + consent review |
| Each release | Advisory scan, SBOM, signing verification |

Record outcomes in [`docs/DECISIONS.md`](docs/DECISIONS.md) and
[`AI_CHANGELOG.md`](AI_CHANGELOG.md).
