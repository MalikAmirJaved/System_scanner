# Compliance

> **Status:** No legal review has been performed. This document records known
> obligations and open items; it is **not legal advice**.
> **Last updated:** 2026-11-06

---

## 1. Executive summary

| Area | Status | Blocker for public release |
|---|---|---|
| Project license | **None selected** | **Yes** |
| Third-party license audit | Partial (direct deps only) | Yes |
| Privacy law (GDPR/CCPA) | Assessment not started; local-only design limits exposure | No, but document |
| Code signing / SmartScreen | Not configured | Yes (trust) |
| SBOM / supply chain | Not generated | Yes (best practice) |
| Accessibility | Not assessed | Recommended |
| Export control | Standard mass-market software; no crypto beyond OS primitives | Low risk |
| Trademark / branding | "System Scanner" not cleared | Recommended |

## 2. Project license

- The repository has **no LICENSE file**.
- `Cargo.toml` has no `license` field; `package.json` has no `license` field.
- Default legal position: **all rights reserved** — nobody may copy, modify, or
  redistribute it, even if the source is visible.
- Contributors have not signed a CLA or DCO.

**Required decision:** choose a license (MIT/Apache-2.0 for permissive; GPL for
copyleft) and add it to `LICENSE`, `package.json`, and `Cargo.toml`. Record in
[`DECISIONS.md`](DECISIONS.md).

## 3. Third-party licenses

Direct dependencies (verified 2026-11-06):

| License | Packages |
|---|---|
| MIT | react, react-dom, vite, @vitejs/plugin-react, @types/* |
| Apache-2.0 | typescript |
| Apache-2.0 OR MIT | @tauri-apps/api, @tauri-apps/cli, tauri |

- Transitive tree: 407 crates in `Cargo.lock` — **not audited**.
- Obligations for MIT/Apache-2.0: retain copyright and license notices, state
  changes for Apache-2.0 files.
- No copyleft dependency has been identified in the direct set, but the transitive
  set is unaudited.

**Actions:**

```text
[ ] Generate an SBOM (CycloneDX) per release
[ ] Run a license allowlist check (cargo-deny, license-checker)
[ ] Include third-party notices in the installer/about screen
[ ] Re-check on every dependency addition
```

## 4. Privacy law

- The app is local-first; no data is transmitted.
- It stores paths, process metadata, and history on the user's machine — this is
  personal data under GDPR even without transmission.
- No account, so there is no data controller relationship with a service.

**Position (to be reviewed):** processing occurs on the user's device, under their
control, for their own purposes. The app acts as a local tool, not a data
processor. This must be documented in a privacy notice before distribution.

Obligations to document regardless:

- what is stored and where;
- how to delete it;
- retention defaults;
- that no telemetry exists;
- what an opt-in network feature would send.

See [`DATA_PRIVACY.md`](DATA_PRIVACY.md).

## 5. Security and distribution compliance

| Requirement | Status |
|---|---|
| Code signing (Authenticode) | Not configured |
| SmartScreen reputation | Not established |
| Signed update manifests | No updater |
| SBOM + checksums | Not produced |
| Vulnerability disclosure policy | Draft in `SECURITY.md` |
| Dependency advisory monitoring | Not automated |

Windows may warn users about an unsigned, low-reputation installer. The honest
response is to sign, publish checksums, and disclose the warning — not to instruct
users to bypass security controls.

## 6. Accessibility

No formal standard has been adopted. If a public consumer product is intended,
WCAG 2.2 AA for the desktop UI is a reasonable target (keyboard access, contrast,
focus visibility, non-color-only status). No conformance claim is made today.

## 7. Export control

The app uses standard OS cryptographic APIs (hashing) for integrity checks. It does
not implement custom cryptography, encryption products, or key exchange. Typical
mass-market software exemptions are expected to apply; confirm before distribution
in restricted jurisdictions.

## 8. Data retention obligations

If the product is ever used in a regulated environment, retention of logs and
history may create obligations. Current defaults are proposals, not commitments.
See [`DATA_PRIVACY.md`](DATA_PRIVACY.md) §3 and [`BACKUP_POLICY.md`](BACKUP_POLICY.md).

## 9. Branding and trademarks

- "System Scanner" is a generic descriptive name; a trademark search has not been
  performed.
- Do not use Microsoft logos or imply endorsement.
- Windows is a trademark of Microsoft Corporation.

## 10. Pre-release compliance checklist

```text
[ ] LICENSE file added and referenced in manifests
[ ] Third-party notices generated
[ ] SBOM produced and archived
[ ] Privacy notice drafted and linked in-app
[ ] Retention and deletion behavior documented
[ ] Signing configured and verified
[ ] Vulnerability disclosure channel published
[ ] Accessibility self-assessment completed
[ ] Installer does not bundle unlicensed assets or fonts
[ ] No analytics/telemetry SDK present
```

## 11. Open questions

1. Which license?
2. Is this a consumer product (stricter expectations) or an internal tool?
3. Will any data ever leave the device?
4. Is an enterprise deployment model planned (MSI, GPO)?
5. Who owns legal review?
