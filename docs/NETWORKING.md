# Networking

> **Status:** The application has **zero network capability**. No HTTP client is
> included in Rust or JavaScript, and the CSP restricts the webview to `'self'`.
> **Last updated:** 2026-11-06

---

## 1. Baseline posture

| Property | State |
|---|---|
| Outbound requests from app code | None |
| Telemetry | None |
| Account/licence check | None |
| Update check | None (no updater) |
| Remote assets (fonts, scripts, images) | Blocked by CSP |
| HTTP client dependency | None |
| Network plugin capability | Not granted |

This is a product feature, not an accident. Verify it on every release:

```powershell
# Inspect dependencies for network-capable crates/packages
Select-String -Path src-tauri/Cargo.lock -Pattern "reqwest|hyper|ureq|curl"
Select-String -Path package-lock.json -Pattern '"axios"|"node-fetch"|"undici"'
```

Expected: no matches in application dependencies (build tooling may differ).

## 2. Local development networking

| Port | Purpose | Binding |
|---|---|---|
| `1420` | Vite dev server (Tauri `devUrl`) | loopback by default |
| `1421` | Vite HMR WebSocket when `TAURI_DEV_HOST` is set | loopback / dev host |

Rules:

- `strictPort: true` — fail rather than silently change ports.
- `TAURI_DEV_HOST` is for device/remote debugging only; never set in production.
- `src-tauri` is excluded from the file watcher.
- The dev server exists only in development; release builds load `frontendDist`.

## 3. WebView2 considerations

- The webview may contact Microsoft endpoints for its own runtime update/maintenance,
  independent of application code. This is an OS component behavior, not telemetry
  from this app, and should be disclosed accurately if it matters to users.
- The app must function with WebView2's network access blocked.
- Any future "offline mode" claim must be tested with networking disabled.

## 4. Content Security Policy

Current:

```text
default-src 'self';
img-src 'self' asset: data:;
style-src 'self' 'unsafe-inline';
font-src 'self' data:
```

Implications:

- No `connect-src` override → falls back to `default-src 'self'`, so the webview
  cannot call arbitrary origins.
- No remote scripts, styles, frames, or fonts.
- `style-src 'unsafe-inline'` is an accepted, documented risk.

Rules:

- Never add a remote origin to the CSP to make a library work.
- Never add `unsafe-eval`.
- If an opt-in feature needs an origin, add a narrow `connect-src` for that host
  only, and review it in [`THREAT_MODEL.md`](THREAT_MODEL.md).

## 5. Future optional network features

Only with explicit product approval and a privacy review.

| Candidate | Data sent | Default | Requirements |
|---|---|---|---|
| Hash reputation lookup | File hash only | Off, opt-in per lookup | Consent UI, endpoint disclosure, TLS, offline fallback, rate limit |
| Update check/manifest | App version, OS build | Off until updater exists | Signed manifest, pinned host, no identifiers |
| Crash reporting | Redacted crash data | Off | Previewable payload, explicit consent |

Never send: file contents, full paths, usernames, machine names, process lists.

### 5.1 Consent requirements

```text
[ ] User action is explicit (not bundled into install)
[ ] Shows exactly what will be sent, before sending
[ ] Can be revoked; revocation stops future calls
[ ] Works identically offline with a clear "unavailable" state
[ ] Endpoint and retention documented
```

## 6. Protocol rules (when networking is added)

- TLS 1.2+ only; validate certificates; no pinned-but-unmaintained certs.
- Timeouts: connect ≤ 5 s, total ≤ 15 s; cancellable.
- Bounded request/response size (e.g., 64 KB response).
- Retry only idempotent requests, max 2, with jitter.
- No redirects to unexpected hosts.
- Respect the Windows system proxy where applicable; document behavior.
- No custom CAs, no disabled verification, ever.

## 7. Enterprise and firewall considerations

- If a feature ever requires egress, document the host and port so IT can allowlist.
- Offline installs and air-gapped machines must remain fully functional.
- Proxy authentication failure must degrade to "unavailable", not hang.

## 8. Verification checklist (each release)

```text
[ ] No HTTP client in Cargo.lock/package-lock (application deps)
[ ] CSP unchanged or reviewed
[ ] No update/telemetry code paths
[ ] Launch, scan, and export with networking disabled
[ ] No unexpected listening sockets
[ ] Dev-server settings absent from release config
```

Check listening sockets while the app runs:

```powershell
Get-NetTCPConnection -State Listen |
  Where-Object OwningProcess -in (Get-Process system-scanner).Id
```

Expected: none.

## 9. Open questions

1. Is a hosted reputation service ever in scope?
2. Should the updater check on launch or only on explicit user action?
3. How should offline "update check failed" be communicated without alarm?
