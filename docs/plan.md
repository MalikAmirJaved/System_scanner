# System Scanner — Production-Grade Product & Engineering Plan

**Document status:** Proposed baseline  
**Product direction:** Windows-first, local-first system intelligence and maintenance desktop application  
**Initial platform:** Windows 10/11 x64; support matrix to be finalized before release  
**Core stack:** Rust, Tauri 2, React, TypeScript, Vite, SQLite  

---

## 1. Product vision

Build a trustworthy Windows system-inspection platform that helps users understand system health, investigate resource and storage use, review security-relevant evidence, and perform carefully controlled maintenance. The product should be useful without an account or network connection, explain what it observes, and never treat a heuristic as proof of malware.

The application is a desktop product, not a distributed service. “High scalability” means that its architecture can support large disks, many files and processes, long-running scans, multiple scan modules, historical data, and future features without blocking the UI, exhausting memory, or coupling every feature to one another. A future hosted reputation service is an optional, separately scoped product decision—not a prerequisite for the local scanner.

### Product principles

1. **Local-first:** inspection and history work offline; external lookups are opt-in, explicit, and disclosed.
2. **Evidence before verdict:** findings include their evidence, source, confidence, and limitations. “Unknown” is a valid result.
3. **Safe by default:** read-only functionality first; destructive or privileged operations require clear user intent and safeguards.
4. **Least privilege:** run unelevated by default and elevate only a specific operation when necessary.
5. **Responsive and bounded:** scans are cancellable, produce incremental progress, and obey configurable resource limits.
6. **Recoverable actions:** quarantine and reversible workflows precede permanent deletion or modification.
7. **Observable and testable:** errors, scan outcomes, performance, and actions can be diagnosed without collecting personal data by default.
8. **Modular evolution:** stable contracts separate UI, orchestration, domain logic, Windows APIs, persistence, and reporting.

---

## 2. Product scope and release strategy

Deliver a trustworthy core before adding security or control features. Each release must be independently usable and meet its acceptance criteria before the next capability expands risk.

### Release 0 — Foundation and first milestone

- Tauri desktop application launches and reports a useful error if initialization fails.
- Read-only system overview: Windows version, CPU, memory, disks, and basic network adapters.
- Running-process list with safe, clearly labeled metadata.
- User-triggered refresh and bounded periodic refresh, with visible last-updated time.
- Diagnostics view for application logs and environment information that is safe to share.
- No malware verdicts, file deletion, service control, process termination, or administrator requirement.

**Exit criteria:** clean install and upgrade smoke tests on supported Windows versions; no UI freezes during refresh; all collection commands have tests and structured errors; no unexpected network access or elevation.

### Release 1 — Storage intelligence

- Drive discovery and cancellable directory traversal.
- File metadata, large-file summaries, extension/type summaries, and age filters.
- Duplicate candidate grouping using size and partial fingerprints before full cryptographic hashing.
- Optional bounded SHA-256 or BLAKE3 hashing with progress and pause/cancel.
- Storage views use aggregation and pagination/virtualization rather than loading all records into the UI.
- Recommendations only; no automatic cleanup.

### Release 2 — Process, startup, and service visibility

- Process hierarchy and additional process metadata where access is allowed.
- Read-only inventory of startup entries and Windows services.
- Explain source, privileges, and collection limitations for each field.
- Optional user-confirmed process termination or service control only after threat modeling, authorization, audit, and recovery/error handling are implemented.

### Release 3 — Security intelligence

- Evidence-based suspiciousness indicators, signature metadata, hashes, and reputation status where available.
- Clear distinction among “known,” “suspicious,” “unknown,” and “not scanned”; never present heuristic output as confirmed malware.
- Windows Defender status and supported scanning integration where Windows APIs permit it.
- Quarantine/restore workflow with integrity checks, access controls, audit history, and recovery guidance.
- No kernel driver or filesystem filter in the initial product.

### Release 4 — Diagnostics and network analysis

- Windows Event Log summaries, crash/application error correlations, and system diagnostics.
- Network adapters, DNS configuration, active connections/listening ports, and process ownership where supported.
- Historical resource trends with configurable retention and storage limits.

### Release 5 — Managed maintenance and automation

- User-defined scan schedules and policies, with clear battery/resource/network behavior.
- Exportable reports with privacy review and redaction controls.
- Reversible maintenance workflows and a system health summary whose components are explainable.
- Any background service, multi-user deployment, cloud synchronization, or fleet administration requires a separate architecture and privacy/security review.

---

## 3. Target architecture

Keep a modular monolith: one desktop application and one local Rust engine, organized around explicit domain boundaries. Do not split into microservices without a concrete deployment or scaling requirement.

```text
React + TypeScript UI (Tauri WebView)
  ├─ Feature pages, accessible components, virtualized tables
  ├─ UI state and typed request/event adapters
  └─ No direct OS access and no security decisions
          │ typed commands, events, DTOs
          ▼
Tauri application boundary
  ├─ Narrow command surface and input validation
  ├─ Capability/permission configuration
  ├─ Session and task lifecycle management
  └─ Event serialization and frontend authorization boundary
          │
          ▼
Rust application/core
  ├─ Orchestrator: jobs, cancellation, progress, resource budgets
  ├─ Domains: system, processes, storage, security, network, diagnostics
  ├─ Policies: privilege, protected paths, retention, action authorization
  ├─ Ports: OS collectors, persistence, report/export interfaces
  └─ Adapters: Windows APIs, SQLite, filesystem, Defender integrations
          │
          ├─ Windows user-mode APIs (Win32 / windows-rs as needed)
          ├─ Local SQLite database and managed quarantine storage
          └─ Optional external services only through explicit opt-in
```

### Suggested repository organization

Adapt this layout to the actual scaffold when it exists; do not create every directory before a feature needs it.

```text
frontend/
  src/
    app/                 # routing, shell, app-level providers
    features/            # dashboard, processes, storage, security, etc.
    components/          # shared presentation components
    services/            # typed Tauri command/event adapters
    stores/              # UI state only
src-tauri/
  src/
    commands/            # thin IPC boundary
    app/                 # application orchestration and job lifecycle
    domain/              # models, policies, use cases, error types
    adapters/
      windows/           # OS-specific collection/control adapters
      persistence/       # SQLite repositories and migrations
      filesystem/        # safe traversal and quarantine adapter
    reporting/            # export and report generation
  capabilities/          # minimal Tauri permissions
  migrations/
docs/
  architecture.md
  threat-model.md
  security-model.md
  data-retention.md
  release-process.md
```

### Dependency direction

- UI depends on typed IPC DTOs, not native implementation details.
- Tauri commands validate inputs and delegate; they do not contain scanning algorithms.
- Domain and policy logic should be testable without Windows or a WebView.
- Windows adapters implement narrow interfaces and isolate `unsafe` FFI.
- Persistence is behind repositories; domain workflows do not embed SQL.
- Cross-module calls use explicit request/result types. Avoid a global mutable “engine” object or unrestricted command surface.

---

## 4. Scalability and performance design

### Workload model

The largest expected workload is storage traversal: millions of directory entries, inaccessible paths, reparse points, long paths, removable/network drives, and changing files. Design the pipeline so work is streamed and bounded rather than accumulated in memory.

### Scan pipeline

1. **Discover:** enumerate selected roots and filesystem capabilities.
2. **Traverse:** bounded worker pool; honor cancellation; define reparse-point policy; track visited file identities where needed to avoid cycles.
3. **Collect:** emit compact metadata records; distinguish access denied, disappeared, and unsupported entries.
4. **Aggregate:** compute summaries incrementally and persist in batches.
5. **Enrich:** hash or inspect signatures only for eligible candidates under explicit limits.
6. **Classify:** attach evidence and policy version; never turn a missing signal into a positive verdict.
7. **Present:** query paginated/aggregated results; do not send millions of records over IPC.

### Resource controls

- Configurable worker concurrency and a global per-scan memory budget.
- Bounded channels/backpressure between traversal, enrichment, persistence, and event delivery.
- Chunked database writes and indexed, paginated reads.
- Hashing limits: maximum file size, maximum concurrent hashes, and pause/cancel support.
- UI progress events are throttled/coalesced; progress is informative but does not overwhelm IPC.
- Prioritize foreground responsiveness and allow scans to yield or reduce concurrency.
- Maintain database retention/compaction policies and show expected disk impact before large scans.
- Benchmark representative HDD, SATA SSD, NVMe, removable drive, permission-restricted, and large-tree workloads before setting defaults.

### Database and history

Use SQLite for the local single-user product. Use SQLx only if verified in the project dependency configuration; otherwise select a persistence approach deliberately and document the trade-offs.

Initial logical entities (introduce as features arrive):

- `scan_sessions`: scan ID, module, roots/options, start/end, status, cancellation, engine/policy version.
- `system_snapshots`: timestamped bounded system summaries.
- `process_snapshots`: timestamped process metadata with explicit retention.
- `file_records`: path/identity metadata scoped to a scan, with retention and privacy considerations.
- `file_hashes`: algorithm, digest, size, computation status, and provenance.
- `security_findings`: classification, evidence references, confidence, rule/policy version, and status.
- `quarantine_items`: original metadata, stored object identity, integrity data, and restore state.
- `action_history`: user-initiated operation, target, confirmation, result, and timestamp.

Database requirements:

- Versioned migrations; backup/restore strategy; transactional writes for multi-step state changes.
- Index for actual query patterns and validate query plans on large fixtures.
- Avoid storing full file contents or unnecessary personal data.
- Retention is configurable and visible; large historical tables must be bounded.
- Handle interrupted scans and application crashes with resumable or safely discardable session state.
- Define database corruption recovery and a non-destructive support export.

---

## 5. IPC and background job contract

Treat Tauri IPC as a security and performance boundary.

- Expose narrow, typed commands: start, inspect status, cancel, query page/summary, and invoke a specifically authorized action.
- Return stable DTOs, not internal Rust structures or raw platform handles.
- Every long-running operation returns a job/scan ID immediately; progress and completion are events keyed by that ID.
- Events include sequence/version fields where needed so the UI can detect stale or reordered updates.
- Cancellation is cooperative and checked at traversal, hashing, batching, and external-call boundaries.
- Validate all user-controlled paths and options at the Rust boundary; UI validation is only a usability aid.
- Bound result sizes, event rates, string/path lengths, and concurrent jobs.
- Define error categories that are actionable without leaking sensitive paths or internal details unnecessarily.
- Test command authorization and capability configuration as part of release checks.

---

## 6. Security, privacy, and safety model

Create a threat model before adding privileged or destructive operations. Assume the UI can be compromised independently of Rust and treat all IPC inputs as untrusted. Assume file paths, file metadata, and process names may be attacker-controlled.

### Baseline controls

- Minimal Tauri capabilities; deny unused filesystem, shell, HTTP, and plugin permissions.
- Content Security Policy and production asset integrity appropriate to the Tauri build.
- Keep OS collection and security decisions in Rust; avoid shelling out for core functionality.
- Use least privilege; no always-admin manifest or silent privilege escalation.
- Minimize `unsafe`; isolate and review it, document invariants, and test boundary behavior.
- Protect SQLite and quarantine data with appropriate Windows ACLs; do not claim encryption unless implemented and key-managed.
- Avoid logging secrets, file contents, full command lines, or personal paths by default.
- No telemetry by default. Any optional diagnostics upload must be opt-in, previewable, minimized, and documented.
- Pin/review dependencies, audit licenses and advisories, produce an SBOM, and track security updates.
- Sign release artifacts and protect signing credentials outside the repository/CI logs.

### Safe action rules

- Never automatically delete files because they are large, old, hidden, or suspicious.
- Never label a file malware based only on name, extension, path, or one heuristic.
- Protect Windows system locations and define explicit exclusion/allow policies.
- Prefer quarantine to deletion. Quarantine must verify the source identity/content before moving, store integrity metadata, and support safe restore.
- Show target, consequence, evidence, and reversibility before confirmation.
- Record user-initiated control operations and outcomes in an audit history.
- Design against TOCTOU path substitution and reparse-point tricks for any operation that changes files.
- Require a clear elevation flow for operations that genuinely need admin privileges; explain why and fail safely if elevation is denied.
- Keep read-only inspection separate from the privileged control pathway.

### Security finding schema

Every finding should make clear:

- What was observed and by which collector/rule.
- Evidence and timestamp, with provenance.
- Confidence/limitations and whether the item was actually scanned.
- Rule and engine version.
- Recommended next step and whether it changes the system.
- User disposition and history, if any.

---

## 7. Windows compatibility and native API strategy

- Define and publish supported Windows versions, architectures, edition assumptions, and minimum WebView2/runtime requirements.
- Prefer documented, supported Windows APIs. Use `windows`/`windows-sys` bindings only after confirming the selected crate/version and feature flags in Cargo configuration.
- Keep Windows-specific behavior behind adapters, with explicit unsupported/access-denied outcomes.
- Avoid relying on undocumented NT APIs for essential product behavior. If unavoidable, isolate, version-test, and provide a safe fallback.
- Test long paths, Unicode paths, ACL restrictions, reparse points, locked/changing files, disconnected volumes, and non-admin execution.
- Treat WMI/CIM and registry access as fallible, potentially slow external interfaces; use timeouts/cancellation where possible and do not block the UI thread.
- Avoid imposing administrator access on system-information or process-list screens.

---

## 8. Frontend product quality

- Use React + TypeScript + Vite; Next.js is unnecessary for the local Tauri UI.
- Establish accessible navigation, keyboard behavior, readable contrast, scalable typography, and clear loading/empty/error states.
- Use virtualized tables and server/query-side pagination for large datasets.
- Separate UI state from Rust scan state; Rust is authoritative for job status.
- Present scan scope, permissions, elapsed time, throughput, skipped items, cancellation, and partial results honestly.
- Build reusable feature patterns, not a monolithic dashboard component.
- Charts must have textual summaries and must not be the only way to access information.
- Keep dependencies limited; verify package availability and existing project conventions before adding libraries.

---

## 9. Quality strategy and acceptance gates

### Testing pyramid

1. **Unit tests:** domain rules, classification evidence, retention, path policy, job state transitions, and DTO serialization.
2. **Adapter tests:** Windows API behavior behind narrow interfaces; database migrations and repository queries.
3. **Integration tests:** scan pipeline with generated fixtures, permission failures, cancellation, interrupted sessions, and duplicate/hash behavior.
4. **UI tests:** key user workflows, accessibility, error and cancellation states, and IPC contract compatibility.
5. **End-to-end tests:** packaged app launch, read-only collection, refresh, install/upgrade/uninstall, and offline behavior on Windows VMs.
6. **Performance tests:** large directory trees, millions of records, memory ceilings, DB growth, cancellation latency, and UI responsiveness.
7. **Security tests:** Tauri capability review, path/reparse-point abuse, privilege boundary, malformed IPC inputs, dependency advisories, and quarantine/restore integrity.

### Release gates

A release is blocked unless:

- Formatting, lint, type checking, tests, and release build are green on the supported Windows CI matrix.
- No known critical/high exploitable dependency advisory is untriaged.
- Installer, upgrade, rollback/recovery, and uninstall paths are tested.
- Database migrations and retention are tested on realistic data sizes.
- Supported actions are auditable and safety-reviewed.
- Performance budgets and crash/error rates meet the product's documented thresholds.
- Security/privacy notes and release notes describe material behavior changes.

### Reliability targets to establish in early testing

Set numeric budgets only after measuring baseline hardware, but track from the first milestone:

- App launch and dashboard readiness time.
- Refresh completion and cancellation latency.
- Peak memory by scan size and concurrency.
- Storage scan throughput and responsiveness impact.
- UI frame responsiveness while work is active.
- Scan completion/failure/cancellation counts and crash-free sessions, locally measured unless telemetry is explicitly opted into.

---

## 10. Build, release, and operations

- Use reproducible, pinned Rust and Node toolchains; commit lockfiles.
- CI on Windows for formatting, linting, tests, frontend build, Tauri packaging, and installer smoke tests.
- Keep build signing keys and publish credentials in protected CI secret storage; never in source or logs.
- Generate an SBOM and retain release provenance, checksums, and signed artifacts.
- Use staged releases (internal, preview, stable) with rollback instructions and a support bundle that redacts sensitive data.
- Define a crash-reporting policy that is opt-in and privacy-reviewed before implementation.
- Document data locations, backup/restore, uninstall behavior, database migrations, and quarantine retention.
- Maintain an incident response and vulnerability reporting process before public distribution.

---

## 11. Development phases and decision gates

### Phase A — Product and threat-model baseline

- Confirm audience, supported Windows versions, privilege expectations, offline requirements, and distribution model.
- Create architecture, data classification, threat model, and privacy/data-retention notes.
- Set initial performance and accessibility measurement plan.

**Gate:** no ambiguous destructive or network behavior remains in the first milestone.

### Phase B — Vertical-slice foundation

- Scaffold Tauri 2 + React/TypeScript/Vite and Rust modules.
- Establish typed IPC, narrow capabilities, logging, error model, and CI.
- Implement system snapshot and process listing with read-only adapters.
- Add refresh lifecycle, cancellation where relevant, and frontend loading/error states.

**Gate:** first milestone acceptance criteria pass on clean Windows installations.

### Phase C — Storage scan engine

- Prototype traversal independently behind a testable Rust interface.
- Add bounded concurrency, cancellation, reparse-point policy, permission reporting, batching, and persistence.
- Validate performance and memory with generated large-tree fixtures before polishing treemap UI.

**Gate:** scan remains bounded, resumable or safely discardable, and cancellable under stress.

### Phase D — Trust and security features

- Add signatures, evidence-based indicators, Defender integration research, and finding provenance.
- Threat-model quarantine, restoration, and elevation before exposing controls.

**Gate:** security review and recovery tests before any destructive action ships.

### Phase E — Diagnostics and maintenance

- Add logs/network/history incrementally, each with permission, privacy, retention, and performance review.
- Add schedules and policy automation only after user-visible controls and failure recovery are mature.

**Gate:** all background work has user-visible status, cancellation/pause policy, and bounded resource use.

---

## 12. Open decisions to resolve before implementation

1. Is the first deliverable a learning prototype, a public consumer product, or an internal diagnostic tool? This affects support and release requirements.
2. Which Windows versions/architectures must be supported, and is Windows 10 support required?
3. Is the application strictly single-user/local, or is managed/fleet deployment a future requirement?
4. What exact data may be retained, for how long, and where should SQLite/quarantine data live?
5. Should external hash reputation lookups ever be offered? If yes, define explicit consent and data disclosure first.
6. What distribution channel is planned (MSIX, signed installer, portable build), and who owns code signing?
7. What performance test machine and large-scan dataset define “production-ready” for storage scanning?
8. Which features require administrator privileges, and can each be designed as a narrowly scoped optional operation?

Until these are answered, default to local-only, read-only, non-elevated behavior.

---

## 13. Immediate next steps

1. Confirm the open product decisions that affect architecture, especially audience, Windows support, distribution, and retention.
2. Create a minimal scaffold and CI only after the project owner confirms the intended starting workflow.
3. Implement the first milestone as one end-to-end vertical slice: system snapshot → typed Rust command → responsive UI → tests → packaged Windows smoke test.
4. Measure the baseline before optimizing or adding broad feature modules.
5. Review this plan at each release gate; defer features whose safety, privacy, or resource controls are not ready.

---

## 14. Master execution checklist

Use this checklist as the project’s execution tracker. Check an item only when the evidence exists (code, test output, reviewed decision, or release artifact); do not mark work complete because it is planned. Each release should add its own dated checklist or issue links rather than rewriting historical completion evidence. The checklist is deliberately sequenced: decisions and safety foundations precede features that can affect user data or system state.

### 14.1 Product definition and governance

- [ ] Identify primary users, core use cases, and non-goals for the first public release.
- [ ] Decide whether the product is a learning project, consumer product, or internal tool.
- [ ] Confirm supported Windows versions, editions, architectures, and minimum hardware/runtime requirements.
- [ ] Confirm whether Windows 10 support is required and how end-of-support changes will be handled.
- [ ] Define installation/distribution format, update channel, code-signing owner, and release cadence.
- [ ] Confirm local-only baseline and explicitly decide whether any optional online feature is in scope.
- [ ] Define data categories: collected, persisted, exported, never collected, and user-deletable.
- [ ] Define retention defaults, maximum database growth, quarantine retention, and uninstall behavior.
- [ ] Create a lightweight architecture decision record (ADR) for each consequential choice: context, options, decision, trade-offs, and revisit trigger.
- [ ] Assign ownership for vulnerability reports, release signing, dependency updates, and user support before public distribution.
- [ ] Define user-facing terminology for health, risk, evidence, confidence, unknown, skipped, and not scanned.
- [ ] Establish versioning, compatibility, deprecation, and migration policy for persisted data and exported reports.

### 14.2 Repository and engineering foundation

- [ ] Verify the working tree and project scaffold before applying generated templates; preserve existing user work.
- [ ] Pin Rust and Node toolchain versions and commit the appropriate lockfiles.
- [ ] Establish formatting, lint, type-check, unit-test, build, and packaging commands from the actual project configuration.
- [ ] Create CI on Windows that runs the same checks developers use locally.
- [ ] Add separate debug and release configurations; verify production builds do not expose development servers or secrets.
- [ ] Define module ownership and dependency direction; prevent UI, OS adapters, and persistence from becoming cyclic dependencies.
- [ ] Establish typed error categories and a policy for user-safe error messages versus internal diagnostics.
- [ ] Configure structured tracing with bounded log retention and sensitive-field redaction.
- [ ] Add a dependency review process for security advisories, licenses, update cadence, and transitive dependencies.
- [ ] Generate an SBOM in release CI and archive it with release provenance.
- [ ] Document local developer setup and clean-machine build prerequisites.
- [ ] Add a contribution/review checklist covering correctness, security, privacy, accessibility, and resource use.

### 14.3 Architecture and interface contracts

- [ ] Define domain boundaries for system, processes, storage, security, network, diagnostics, control, and reporting.
- [ ] Define command/event DTO schemas separately from internal Rust domain models.
- [ ] Version any persisted or externally exported schema that may outlive an application release.
- [ ] Keep Tauri commands thin: validate boundary input, authorize the operation, delegate, and map results.
- [ ] Define job lifecycle states (queued, running, pausing, paused if supported, cancelling, completed, failed, cancelled).
- [ ] Define behavior for duplicate start requests, app shutdown during a job, stale UI sessions, and event reconnection.
- [ ] Bound the number of concurrent jobs, queued jobs, outstanding results, and event frequency.
- [ ] Define cancellation guarantees and maximum expected cancellation latency for each long-running stage.
- [ ] Test malformed, oversized, stale, and unauthorized IPC requests.
- [ ] Ensure app updates and frontend/backend changes remain compatible within a packaged release.
- [ ] Add architecture tests or review checks that keep OS-specific code out of the domain layer.
- [ ] Document any exceptions to the dependency direction and why they are necessary.

### 14.4 Threat model, privacy, and safety baseline

- [ ] Write a threat model covering a compromised WebView, malicious filenames/metadata, hostile reparse points, untrusted removable media, and local privilege boundaries.
- [ ] Identify assets, trust boundaries, attacker capabilities, abuse cases, mitigations, and residual risks.
- [ ] Review the Tauri capability list and remove every permission not required by an implemented feature.
- [ ] Review production Content Security Policy, asset loading, plugin usage, and navigation/external-link behavior.
- [ ] Verify there is no silent network access, telemetry, cloud synchronization, or account requirement in the baseline product.
- [ ] Verify no secrets, full command lines, file contents, or unnecessary personal paths are logged.
- [ ] Define privacy-safe diagnostic export with preview, redaction, and explicit user action before implementation.
- [ ] Document privilege boundaries and prove read-only first-milestone workflows run unelevated.
- [ ] Inventory every `unsafe` block; document invariants and add focused tests or review evidence.
- [ ] Define protected-path and reparse-point policies before any filesystem action is implemented.
- [ ] Threat-model TOCTOU races, hard links, symlinks/reparse points, volume changes, and file replacement before quarantine or cleanup work.
- [ ] Define vulnerability intake and patch triage expectations before a public release.
- [ ] Record security review outcomes and unresolved risks; do not silently treat accepted risk as eliminated risk.

### 14.5 First milestone: system information and process visibility

- [ ] Establish a minimal Tauri 2 + React/TypeScript/Vite application that builds and launches on a clean supported Windows machine.
- [ ] Confirm the UI never performs native OS collection directly.
- [ ] Implement a read-only system snapshot through a narrow Rust command and documented Windows adapter.
- [ ] Report unavailable or permission-restricted fields explicitly instead of fabricating defaults.
- [ ] Implement a read-only process list with stable identity handling and clearly defined refresh semantics.
- [ ] Avoid treating process names or paths as security verdicts.
- [ ] Add user-triggered refresh, refresh-in-progress state, last-updated time, and recoverable error state.
- [ ] Add bounded periodic refresh only with visible interval and pause/disable control.
- [ ] Prevent overlapping refreshes or define and test their ordering/cancellation behavior.
- [ ] Verify refresh does not block rendering or allow stale responses to overwrite newer results.
- [ ] Test normal, partial, empty, access-denied, and transient-failure results.
- [ ] Verify first-milestone operation requires no administrator access and performs no unexpected network requests.
- [ ] Package and smoke-test the first vertical slice; retain logs and test evidence for the release gate.

### 14.6 Storage scanner readiness and implementation

**Before implementation**

- [ ] Define selected-root behavior, exclusions, hidden/system file handling, network/removable volume behavior, and default scan scope.
- [ ] Decide whether and how to traverse reparse points; specify cycle detection and target-volume boundaries.
- [ ] Define identity and deduplication semantics when file IDs are unavailable or change during scanning.
- [ ] Define treatment of files that disappear, change, become locked, or become inaccessible mid-scan.
- [ ] Define scan consistency expectations: best-effort point-in-time approximation versus snapshot-based consistency.
- [ ] Define resource budgets and user-visible controls for concurrency, hashing, disk use, pause, and cancellation.
- [ ] Design large-result UX around summaries, filters, paginated queries, and virtualization before sending records to the UI.

**Implementation and verification**

- [ ] Build a traversal prototype behind a testable interface before integrating the full UI.
- [ ] Stream directory entries through bounded queues; do not retain the full tree in memory.
- [ ] Implement cooperative cancellation at enumeration, enrichment, persistence, and reporting boundaries.
- [ ] Add explicit per-entry outcomes for success, skipped, inaccessible, disappeared, unsupported, and failed.
- [ ] Ensure one inaccessible file does not abort the entire scan unless the selected policy requires it.
- [ ] Batch persistence in transactions sized by measured memory and latency budgets.
- [ ] Add resumable or safely discardable scan-session recovery after process termination or power loss.
- [ ] Add index/query-plan tests for the actual filter/sort/grouping UI queries.
- [ ] Compare duplicate candidates by low-cost metadata before optional content hashing.
- [ ] Show hash algorithm, completion state, limits, and partial/failed hash status; never imply unhashed items were checked.
- [ ] Test empty drives, permission-restricted folders, junctions, loops, Unicode and long paths, locked files, changing files, and disconnected drives.
- [ ] Stress test generated trees at multiple scales and capture peak memory, throughput, database growth, and cancellation latency.
- [ ] Verify scan behavior on HDD, SATA SSD, NVMe, removable storage, and network paths where supported.
- [ ] Confirm the UI remains usable and scan impact can be reduced while a large scan runs.

### 14.7 Persistence, migrations, and data lifecycle

- [ ] Choose and document SQLite access/concurrency strategy after validating the actual crate and project needs.
- [ ] Define schema ownership, migration naming, forward compatibility, and failure recovery.
- [ ] Test migrations from every supported released schema, not only a fresh database.
- [ ] Make multi-step state changes transactional or define recovery for partial completion.
- [ ] Define database busy/locking behavior and avoid blocking the UI on long transactions.
- [ ] Add retention enforcement for snapshots, scan records, logs, and derived hashes.
- [ ] Show or document expected disk usage and what user data will be removed when retention applies.
- [ ] Ensure retention does not remove records needed to safely restore quarantined items or understand an action.
- [ ] Test low-disk-space, read-only profile, locked database, corrupted database, and interrupted migration scenarios.
- [ ] Provide a non-destructive export/backup path and verify restore on a clean profile.
- [ ] Define database and quarantine file ACLs; document whether data is protected by OS account permissions only.
- [ ] Test uninstall and reinstall behavior against the documented data-retention choice.

### 14.8 Security intelligence and control operations

- [ ] Define finding states and evidence provenance before introducing suspiciousness rules.
- [ ] Ensure every rule has tests for positive, negative, missing-evidence, and conflicting-evidence cases.
- [ ] Version rules and preserve the version used to produce historical findings.
- [ ] Label reputation as unavailable, not checked, unknown, or known as appropriate; avoid binary false certainty.
- [ ] Research and validate supported Windows Defender interfaces before promising integration behavior.
- [ ] Document precisely what information leaves the device for any optional reputation request and obtain explicit consent.
- [ ] Do not implement a kernel driver or filesystem filter as part of the initial releases.
- [ ] Threat-model quarantine storage, ACLs, integrity verification, name collisions, disk-full conditions, and restore conflicts.
- [ ] Prove restore returns the expected file and metadata or reports exactly what could not be restored.
- [ ] Require explicit confirmation that identifies the exact target and consequence before a system-changing action.
- [ ] Keep deletion separate from detection and quarantine; never automate permanent deletion by default.
- [ ] Require an audit record for each attempted action, including denied/failed outcomes, without logging sensitive content.
- [ ] Add authorization tests proving read-only commands cannot invoke privileged actions.
- [ ] Test elevation denial, cancellation, timeout, and partial failure without leaving inconsistent system state.
- [ ] Conduct a security review and recovery drill before enabling any destructive or privileged control.

### 14.9 Frontend, accessibility, and usability

- [ ] Establish route/navigation structure and feature ownership without building unused placeholder modules.
- [ ] Define shared visual states for loading, empty, partial, stale, failed, offline, and permission-limited data.
- [ ] Add keyboard navigation, visible focus, semantic labels, and screen-reader descriptions to core workflows.
- [ ] Verify color contrast and avoid color-only meaning for severity or status.
- [ ] Use textual equivalents for charts and accessible tables for essential information.
- [ ] Virtualize large tables and test sorting/filtering without loading the complete result set.
- [ ] Test window resizing, display scaling, high-DPI, and minimum supported window size.
- [ ] Ensure confirmation dialogs clearly identify the target and consequence; avoid dark patterns or preselected destructive actions.
- [ ] Make partial data and skipped items visible in summaries.
- [ ] Review user-facing copy for distinction between observation, recommendation, suspicion, and confirmed action.
- [ ] Run accessibility checks on core release workflows and resolve critical issues before release.

### 14.10 Reliability, performance, and observability

- [ ] Establish baseline hardware and representative scan fixtures before setting numeric performance targets.
- [ ] Set measurable budgets for launch, refresh, memory, scan impact, IPC event rate, database growth, and cancellation latency.
- [ ] Record benchmark methodology and compare regressions against a stable baseline.
- [ ] Test cancellation while waiting for I/O, hashing, database writes, and app shutdown.
- [ ] Test repeated start/cancel cycles for leaked handles, tasks, database connections, or growing memory.
- [ ] Ensure all worker tasks are supervised and failures reach the owning job state.
- [ ] Make logs useful for local diagnosis while applying redaction and retention bounds.
- [ ] Define whether crash reporting exists; keep it off unless its collection, consent, and privacy policy are explicitly approved.
- [ ] Exercise forced termination, restart, database recovery, and partial-scan presentation.
- [ ] Track local reliability metrics without silently uploading them.
- [ ] Define support bundle contents and confirm the user can review/redact them before sharing.

### 14.11 Release and maintenance readiness

- [ ] Build a clean release on the supported Windows CI matrix using pinned toolchains.
- [ ] Verify signatures, checksums, SBOM, provenance, and installer metadata.
- [ ] Test install, upgrade from prior supported versions, rollback/recovery, uninstall, and data handling.
- [ ] Test offline installation and operation wherever the product promises offline use.
- [ ] Verify no development-only endpoints, debug permissions, secrets, or verbose sensitive logging ship in release artifacts.
- [ ] Test Windows Defender/SmartScreen reputation and signing workflow early; document expected first-release warnings honestly.
- [ ] Review dependency advisories and licenses for every release; document accepted exceptions and owners.
- [ ] Prepare release notes, supported-platform matrix, known limitations, privacy/data-retention notes, and recovery instructions.
- [ ] Stage releases through internal/preview channels before stable publication.
- [ ] Define rollback criteria and a process to stop distribution of a harmful build.
- [ ] Confirm vulnerability reporting contact and incident-response ownership are visible before public release.
- [ ] Run the release gate checklist and retain evidence for every unchecked/waived item.

### 14.12 Release-specific definition of done

A feature is not done merely because its main screen works. For every feature, verify all applicable items below:

- [ ] Scope, user value, non-goals, and data handling are documented.
- [ ] Permission requirements and failure/partial-result semantics are explicit.
- [ ] Domain logic is separated from the UI and Windows-specific adapter.
- [ ] Inputs, concurrency, memory, disk, event, and result limits are defined.
- [ ] Errors are actionable, non-misleading, and do not expose unnecessary sensitive data.
- [ ] Unit, adapter, integration, UI, and abuse-case tests are added at the appropriate levels.
- [ ] Accessibility and large-result behavior are reviewed.
- [ ] Logs and diagnostics are bounded and privacy-reviewed.
- [ ] Performance has been measured against a relevant workload.
- [ ] Documentation, release notes, and support/recovery guidance are updated.
- [ ] Security and product risks have an owner and a documented disposition.
- [ ] Acceptance evidence is available for the release gate.
