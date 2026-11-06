# DevOps

> **Status:** **No CI/CD exists.** There is no workflow, no pipeline, no automated
> checks, and no release automation. This document specifies the target pipeline.
> **Last updated:** 2026-11-06

---

## 1. Current state

| Capability | Status |
|---|---|
| CI provider | None |
| Automated tests | None |
| Automated lint/format | None |
| Release automation | None |
| Artifact storage | None |
| SBOM | None |
| Dependency alerts | None |
| Branch protection | None |

Everything below is a proposal.

## 2. Pipeline design

Target: GitHub Actions on `windows-latest`, because the product is Windows-only
and a Linux runner cannot build the MSVC target.

```text
PR ──► [frontend] ─┐
   ──► [rust]      ├──► [package] ──► artifacts
   ──► [security]  ┘
tag ──► [release] ──► signed installer + SBOM + checksums
```

### 2.1 Frontend job

```yaml
frontend:
  runs-on: windows-latest
  steps:
    - uses: actions/checkout@v4
    - uses: actions/setup-node@v4
      with:
        node-version: 24
        cache: npm
    - run: npm ci
    - run: npm run build          # tsc + vite build
    # - run: npm test             # once Vitest exists
    # - run: npm run lint         # once ESLint exists
```

### 2.2 Rust job

```yaml
rust:
  runs-on: windows-latest
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
      with:
        components: rustfmt, clippy
    - uses: Swatinem/rust-cache@v2
      with:
        workspaces: src-tauri
    - run: cargo fmt --all -- --check
      working-directory: src-tauri
    - run: cargo clippy --all-targets -- -D warnings
      working-directory: src-tauri
    - run: cargo test
      working-directory: src-tauri
```

### 2.3 Package job

```yaml
package:
  needs: [frontend, rust]
  runs-on: windows-latest
  steps:
    - uses: actions/checkout@v4
    - uses: actions/setup-node@v4
      with: { node-version: 24, cache: npm }
    - uses: dtolnay/rust-toolchain@stable
    - run: npm ci
    - run: npm run tauri build
    - uses: actions/upload-artifact@v4
      with:
        name: installer
        path: src-tauri/target/release/bundle/nsis/*.exe
```

### 2.4 Security job

```yaml
security:
  runs-on: windows-latest
  steps:
    - uses: actions/checkout@v4
    - run: npm audit --omit=dev
    - uses: rustsec/audit-check@v2
      with:
        token: ${{ secrets.GITHUB_TOKEN }}
        working-directory: src-tauri
    # SBOM: cargo cyclonedx + npm sbom, archived per release
```

### 2.5 Release job (tags only)

```text
- verify the tag matches version in package.json / tauri.conf.json / Cargo.toml
- build + sign
- generate checksums (SHA-256) and SBOM
- create a draft GitHub release with artifacts
- require manual approval to publish
```

## 3. Toolchain pinning

| Tool | Pin method |
|---|---|
| Node | `actions/setup-node` version + `package-lock.json` |
| Rust | `rust-toolchain.toml` (**to be added**) |
| Tauri CLI | dev dependency (currently `^2`, resolved 2.12.4) |
| npm | `npm ci` only in CI |
| OS image | `windows-latest`, reviewed on major changes |

Add `rust-toolchain.toml` before CI is enabled so local and CI compilers match.

## 4. Caching

- Node: `setup-node` npm cache.
- Rust: `Swatinem/rust-cache` keyed on `Cargo.lock` + `rust-toolchain.toml`.
- Tauri: cache `~/.cargo` and `src-tauri/target`; do not cache `dist/`.
- Never cache signing material.

## 5. Secrets

| Secret | Use | Rule |
|---|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` | Updater signature | CI secret only, never repo |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Key password | CI secret only |
| Authenticode certificate | Code signing | CI secret store, hardware-backed if possible |
| `GITHUB_TOKEN` | Release creation | Default scoped token |

Rules:

- No secret in logs; mask and avoid echoing.
- No secrets available to PRs from forks.
- Rotate on contributor departure or suspected exposure.
- Documented recovery: [`INCIDENT_RESPONSE.md`](INCIDENT_RESPONSE.md).

## 6. Branch protection (proposed)

```text
main:
  - require PR review (1)
  - require status checks: frontend, rust, security
  - require up-to-date branch
  - disallow force push and deletion
  - require linear history (optional)
```

## 7. Local parity

A developer must be able to reproduce every CI check locally:

```powershell
npm ci
npm run build
cd src-tauri
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Commands are canonical in [`../build.md`](../build.md). If CI runs something not
documented there, the build doc is incomplete.

## 8. Versioning automation

Version appears in three files and must stay in sync:

```text
package.json          "version"
src-tauri/tauri.conf.json  "version"
src-tauri/Cargo.toml  version
```

Add a check (script or CI step) that fails when they diverge. Release tagging:
`vMAJOR.MINOR.PATCH`.

## 9. Artifact retention

| Artifact | Retention |
|---|---|
| PR build artifacts | 14 days |
| Release installers | Indefinite (per release) |
| SBOM + checksums | Indefinite |
| Test logs | 30 days |
| Signed provenance | Indefinite |

## 10. Definition of a healthy pipeline

- Fails fast on formatting/lint before spending time on a full build.
- Produces the installer as an artifact on every main build.
- Never publishes without a signature, checksum, and SBOM.
- Does not rely on a developer's machine state.
- Runs in under ~20 minutes warm.

## 11. Gaps to close

See [`BACKLOG.md`](BACKLOG.md): add workflows, `rust-toolchain.toml`, test runner,
linters, SBOM, signing, version-sync check, and Dependabot.
