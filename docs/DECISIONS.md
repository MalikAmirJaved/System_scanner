# Decisions (ADR Index)

> Architecture Decision Records. One decision per entry: context, options,
> decision, consequences, revisit trigger.
>
> Status: `Accepted`, `Proposed`, `Superseded`, `Rejected`.
> **Last updated:** 2026-11-06

---

## ADR-001 — Desktop shell: Tauri 2

**Status:** Accepted · 2026-11-06

**Context.** A Windows system-inspection tool needs OS access, a responsive UI, a
small trusted computing base, and a defensible security boundary. The installer
must be small enough for a utility users install without deliberation.

**Options.**
1. Electron + React — mature, but bundles Chromium and exposes Node in the renderer.
2. WPF/WinUI 3 — native and fast, but a narrower UI ecosystem and heavier XAML surface.
3. Tauri 2 + React — OS WebView, Rust backend, capability-based IPC.

**Decision.** Tauri 2.

**Consequences.**
- Installer ~1.4 MB (measured); memory overhead lower than Electron.
- Requires WebView2 and a Rust toolchain.
- Debugging spans TypeScript and Rust.
- The capability model forces an explicit, small permission surface.

**Revisit if:** WebView2 availability becomes a distribution blocker, or a
required OS API cannot be reached from Rust.

---

## ADR-002 — UI: React 19 + TypeScript + Vite, no SSR framework

**Status:** Accepted · 2026-11-06

**Context.** The UI is a local desktop renderer, not a website.

**Options.** Next.js, Remix, plain Vite, Svelte, Vue.

**Decision.** React 19 + TypeScript (strict) + Vite.

**Consequences.**
- No server rendering, routing server, or image pipeline to pay for.
- Vite's dev server integrates with Tauri's `beforeDevCommand`/`frontendDist`.
- React's ecosystem and accessibility tooling are available.
- Bundle size is acceptable for a desktop app.

**Revisit if:** the team's expertise shifts, or a bundler-level need appears.

---

## ADR-003 — Local-first, no account, no telemetry

**Status:** Accepted · 2026-11-06

**Context.** Users of system tools are sensitive about what leaves their machine;
a scanner that uploads paths or process lists is a privacy hazard.

**Decision.** Core functionality is fully offline. No account, no analytics, no
crash upload, no update ping until an updater is explicitly approved. Any future
egress requires per-feature consent, disclosure, and a preview of the payload.

**Consequences.**
- No server-side analytics for product decisions; metrics are local and opt-in.
- Support relies on user-provided, redacted bundles.
- Some features (reputation lookup) are deferred or opt-in only.
- Strong trust differentiator and a simpler threat model.

**Revisit only** with an explicit product decision recorded here.

---

## ADR-004 — Modular monolith, not microservices

**Status:** Accepted · 2026-11-06

**Context.** There is one desktop process serving one user. There is no deployment
topology that would benefit from service boundaries.

**Decision.** One application, one Rust engine, explicit module boundaries
(commands → app → domain ← ports ← adapters).

**Consequences.**
- Refactoring is cheap; no network contracts between modules.
- Boundaries must be enforced by review and dependency rules, since the compiler
  will not stop a careless import.
- Scaling is vertical (concurrency, streaming), not horizontal.

**Revisit if:** a hosted fleet/reputation service is approved — that is a separate
product with its own architecture.

---

## ADR-005 — Persistence: SQLite, access layer undecided

**Status:** Proposed · 2026-11-06

**Context.** Local history needs transactions, indexes, and zero administration.

**Options.** SQLite via `rusqlite`; SQLite via `sqlx`; a custom file format.

**Lean.** `rusqlite` + hand-written migrations + `spawn_blocking`, WAL mode.
Rationale: one local writer, explicit SQL, smallest dependency surface, no async
complexity in the persistence layer.

**Why not decided yet.** No persistence code exists; the choice should be validated
against the first real query set. `sqlx` compile-time checking is attractive if the
schema grows large.

**Consequences either way.** Repository layer required; migration tests required;
DB must remain backup-able with `VACUUM INTO`.

**Revisit by:** first persistence milestone (Backlog B-020).

---

## ADR-006 — Read-only first; destructive features gated

**Status:** Accepted · 2026-11-06

**Context.** Almost every real-world harm from "cleaner" tools comes from
irreversible actions taken on weak evidence.

**Decision.** Ship read-only inspection first. Any destructive or privileged
operation requires, before implementation: a threat model, protected-path policy,
identity verification, explicit confirmation, audit trail, and a recovery path.

**Consequences.**
- Slower feature roadmap.
- Much smaller blast radius and support burden.
- Quarantine is preferred over deletion.

**Revisit:** never for the baseline; individual actions may be approved through a
separate ADR with security evidence.

---

## ADR-007 — Release profile: LTO, strip, panic=abort

**Status:** Accepted · 2026-11-06

**Context.** A utility should be small and predictable; unwinding adds binary size
and can mask programming errors.

**Decision.** `codegen-units = 1`, `lto = true`, `opt-level = 3`, `panic = "abort"`,
`strip = true`.

**Consequences.**
- Smaller binary and installer.
- A panic terminates the process; no `catch_unwind` recovery.
- Interrupted sessions must be recoverable at startup.
- Error handling must be explicit; panics are bugs.

**Revisit if:** a legitimate need for in-process recovery appears.

---

## ADR-008 — Installer: NSIS x64 only

**Status:** Accepted · 2026-11-06

**Context.** Tauri supports NSIS, MSI, and portable targets.

**Decision.** NSIS x64 as the only bundle target for now.

**Consequences.**
- Small per-user installer, no WiX toolchain.
- No enterprise MSI deployment yet.
- No ARM64 or portable build.
- Unsigned today — SmartScreen warnings are expected.

**Revisit when:** enterprise distribution or ARM64 becomes a requirement.

---

## ADR-009 — Documentation structure and status vocabulary

**Status:** Accepted · 2026-11-06

**Context.** The repository had a template README and a plan, with no shared
contract for commands, architecture, security, or AI behavior. Documentation that
describes planned features as real is worse than no documentation.

**Decision.**
- `build.md` is the single source of truth for commands.
- `docs/plan.md` is the authoritative plan; root `plan.md` is a pointer.
- Every document carries a status banner.
- A fixed vocabulary is used: `Implemented`, `Partial`, `Planned`, `Proposed`,
  `Not started`.
- `docs/FEATURES.md` is the ground truth for "does this exist?".

**Consequences.**
- Documentation must be updated with code changes (enforced in review).
- Slightly more maintenance; far less drift and false expectation.

**Revisit if:** the documentation set becomes too heavy for the project's size.

---

## ADR-010 — No shell-outs for core functionality

**Status:** Accepted · 2026-11-06

**Context.** It is tempting to call `wmic`, `powershell`, or `sc.exe` to get system
data quickly.

**Decision.** Use documented Windows APIs through Rust adapters. Shelling out is
prohibited for core functionality.

**Consequences.**
- No dependence on localized output or deprecated tools (`wmic` is deprecated).
- Better performance and structured errors.
- More Rust/FFI work up front.
- A narrow exception (e.g., a documented Defender CLI) requires an ADR and must
  handle missing tools, localization, and quoting safely.

---

## Template

```text
## ADR-NNN — Title
Status: Proposed|Accepted|Superseded by ADR-XXX
Date:
Context:
Options:
Decision:
Consequences:
Revisit trigger:
```
