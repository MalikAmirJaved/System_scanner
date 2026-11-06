# AI_SECURITY_RULES

> Security restrictions that apply to every line of AI-generated code in this
> repository. **Last updated:** 2026-11-06 · **Context:** [`SECURITY.md`](SECURITY.md),
> [`docs/THREAT_MODEL.md`](docs/THREAT_MODEL.md)

---

## 1. Threat assumptions

Assume all of the following are true:

1. The webview may be compromised; it has no authority.
2. File names, paths, process names, and metadata are attacker-controlled.
3. Removable media and network shares may be hostile.
4. The system may already be infected; the app must not become a pivot.
5. A local unprivileged attacker can read the app's data directory.
6. Users will click "yes" on a dialog that does not clearly state consequences.

## 2. IPC rules

| Rule | Requirement |
|---|---|
| Validation | Every field validated in Rust: type, range, length, enum membership |
| Paths | Canonicalize, reject traversal, enforce max length, reject device paths, define UNC policy |
| Sizes | Clamp `limit`, `offset`, concurrency, and hash sizes server-side |
| Authorization | Command re-checks policy; capability config alone is not sufficient |
| Errors | Return a stable code + safe message; never leak internal paths or stack traces |
| Logging | Log the correlation ID, not the raw input |
| Panics | No panic on input; `panic = "abort"` means a panic kills the app |

Prohibited:

```rust
// Never: trusts the renderer
let path = PathBuf::from(req.path);
std::fs::remove_file(path)?;

// Never: panics on hostile input
let n: usize = req.limit.parse().unwrap();

// Never: unbounded
let all = collect_everything()?;
```

Required pattern:

```rust
let path = validate_scan_root(&req.path)?;      // typed error, canonicalized
let limit = req.limit.clamp(1, MAX_PAGE);       // bounded
let outcome = adapter.enumerate(&path, limit)?; // explicit outcomes
```

## 3. Filesystem write rules

Any code that creates, moves, modifies, or deletes a file must satisfy **all** of:

1. **Protected paths** denied by default (Windows, Program Files, user profile
   system folders, app's own directory, quarantine store).
2. **Identity verified** before the operation (volume ID + file ID or hash), not
   just the path string.
3. **TOCTOU handled**: open with the narrowest sharing mode, re-verify after open,
   operate on the handle where possible.
4. **Reparse points/symlinks/junctions** not followed for destructive operations
   unless explicitly in scope, and never across volume boundaries.
5. **Confirmation** echoes the exact target and consequence.
6. **Audit record** written before and after, including denial/failure.
7. **Recovery path** exists (quarantine + restore, or a documented undo).
8. **Tests** for: missing file, replaced file, locked file, denied path, disk full,
   and cancellation mid-operation.

If any item is missing, the feature does not ship.

## 4. Privilege rules

- Default process integrity: as-invoked user. No `requireAdministrator` manifest.
- Elevation is per-operation and explicit; explain why, and fail safely if denied.
- Privileged code paths are isolated from read-only collection modules.
- Never auto-elevate, never retry elevation silently, never cache elevation.
- Test: a non-admin user can use every read-only feature.

## 5. Secrets and data

Never commit or log:

- signing keys, certificates, tokens, API keys;
- full file contents or file hashes tied to a person's identity without need;
- full command lines or environment variables;
- usernames, SIDs, machine names, or absolute paths in shared diagnostics.

Diagnostics export must be previewable, redacted by default, and explicit.

## 6. Dependencies and supply chain

1. No new dependency without justification, license check, and maintenance check.
2. Prefer crates with `#![forbid(unsafe_code)]` or a clear safety story.
3. No wildcard versions; use the lockfiles.
4. Do not add a plugin whose default permissions are broader than the feature.
5. Do not vendor code of unknown origin.
6. Report advisories immediately; do not defer to the next release.

## 7. Network rules

- Baseline: **zero network egress**. No analytics, no crash upload, no update
  check unless explicitly approved.
- Any future egress requires: explicit consent UI, a preview of the payload,
  TLS with certificate validation, a documented endpoint, an offline fallback, and
  a privacy review.
- Never send file contents. Never send full paths. Minimize to a hash or a
  user-confirmed selection.
- Never add a generic HTTP capability to the webview.

## 8. Content Security Policy rules

Current policy must not be weakened to accommodate a dependency. If a library
requires `unsafe-eval` or a remote origin, replace the library or isolate the
feature. `style-src 'unsafe-inline'` is a known accepted risk; do not expand it.

## 9. Rust-specific rules

- `unsafe` only in `adapters/windows`, with `// SAFETY:` invariants.
- No `transmute`, no raw pointer arithmetic outside a reviewed FFI wrapper.
- Handle `GetLastError`/`HRESULT` explicitly; convert to typed errors.
- Do not ignore `Result` (`let _ =` is a review flag).
- No `std::process::Command` for core functionality.
- No `panic!`/`unreachable!` on input-derived branches.

## 10. Review gate

Before reporting a security-relevant change complete, answer:

```text
[ ] What is the worst a malicious IPC payload can do here?
[ ] What if the target file is replaced between check and use?
[ ] What if the path is a junction into a protected directory?
[ ] What if the user is non-admin? What if elevation is denied?
[ ] What is written to logs, and could it identify the user?
[ ] Does this add any network capability?
[ ] Is there an audit record and a recovery path?
```

Any "no" or "unknown" blocks completion until resolved.
