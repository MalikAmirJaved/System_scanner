# AI_ARCHITECTURE_RULES

> Enforceable architectural boundaries. Violations are review failures.
> **Last updated:** 2026-11-06 · **Context:** [`ARCHITECTURE.md`](ARCHITECTURE.md)

---

## 1. Layer map

```text
L5  UI           src/features, src/components        presentation only
L4  UI services  src/services                        typed IPC adapters
--- IPC boundary (untrusted input) ---
L3  Commands     src-tauri/src/commands              validate, authorize, delegate
L2  Application  src-tauri/src/app                   jobs, cancellation, budgets
L1  Domain       src-tauri/src/domain                rules, policies, use cases
L0  Ports        src-tauri/src/ports                 traits only
A   Adapters     src-tauri/src/adapters/*            OS, SQLite, filesystem
R   Reporting    src-tauri/src/reporting             export, redaction
```

**Allowed dependency direction**

```text
L5 → L4 → IPC → L3 → L2 → L1 → L0 ← A
                               └← R
```

- Higher layers may depend on lower layers.
- Lower layers never import higher layers.
- Adapters depend on ports/domain; domain never depends on adapters.
- `reporting` may depend on domain DTOs and ports, not on concrete adapters.

## 2. Import rules

| Module | May import | Must not import |
|---|---|---|
| `features/` | `components/`, `services/`, React | `@tauri-apps/api` directly, Node builtins |
| `services/` | `@tauri-apps/api`, shared DTO types | React components, feature state |
| `commands/` | `app/`, `domain/`, `ports/` | `adapters/*` concrete types, SQL |
| `app/` | `domain/`, `ports/`, `tokio` | Win32 crates, SQL, Tauri window APIs |
| `domain/` | `std`, `serde`, `thiserror` | `windows`, `tauri`, `rusqlite`, `tokio` |
| `ports/` | `domain/` | concrete adapters |
| `adapters/windows/` | `windows`/`windows-sys`, `ports/`, `domain/` | Tauri, SQL, UI types |
| `adapters/persistence/` | `rusqlite`/`sqlx`, `ports/`, `domain/` | Win32, Tauri |
| `reporting/` | `domain/`, `ports/` | concrete adapters |

`std::fs` in `domain` counts as an OS dependency — use a port.

## 3. Contract rules

1. **IPC DTOs are not domain entities.** Define them in `commands/dto` (serde) and
   map explicitly. This protects the UI from internal refactors and prevents
   leaking platform handles.
2. **Ports are narrow.** One capability per trait; no `trait System` with 20 methods.
3. **Adapters return explicit outcomes**, not `Option` that conflates "missing"
   with "denied".
4. **Cross-module calls use request/result types.** No positional booleans.
5. **Errors are typed** and mapped to the stable codes in [`API.md`](API.md) §2.
6. **IDs are opaque strings.** The UI never parses or constructs them.

## 4. State ownership

| State | Owner | Never |
|---|---|---|
| Job/scan status | Rust `app` (authoritative) | Mirrored as the source of truth in React |
| UI filters, sort, selection, open panel | React | Persisted as scan state |
| Persisted history | SQLite via repositories | Held in memory as the only copy |
| Settings | Tauri config store (planned) | Hard-coded in components |
| Cancellation | Job token in Rust | Boolean flag in the UI |

The UI may cache a projection of job state, but must reconcile from events and
`get_scan_status` on reconnect. **Rust wins conflicts.**

## 5. Async and concurrency rules

1. Blocking Win32/FS calls go through `spawn_blocking`; never on the async reactor.
2. Every spawned task is supervised and its failure reaches the owning job.
3. Every queue is bounded; producers await capacity (backpressure).
4. Cancellation is cooperative: check at enumeration, enrichment, persistence, and
   export boundaries; then flush and mark the state.
5. No lock is held across `.await`.
6. Progress events are coalesced; no per-file event emission.
7. Concurrent job limits are enforced in `app`, not the UI.

## 6. Persistence rules

1. Domain never issues SQL.
2. Repositories own queries and return domain types.
3. Multi-step state changes are transactional.
4. Migrations are versioned, forward-only, and tested from every released schema.
5. Large writes are batched; large reads are paged.
6. Schema/version changes are documented in [`DATABASE.md`](DATABASE.md) and
   [`docs/MIGRATIONS.md`](docs/MIGRATIONS.md).

## 7. Frontend rules

1. `features/` contains screens and feature state; `components/` is presentational.
2. `services/` is the only place that imports `@tauri-apps/api`.
3. No business rule in the UI (e.g., "is this file protected?"). Ask Rust.
4. Large data is fetched as pages; tables are virtualized.
5. Event subscriptions are established and torn down in one place (a hook/store).
6. No global store until two features demonstrably need to share state.

## 8. Extensibility rules

- Adding a domain (e.g., network) means: DTOs → domain → port → adapter → command
  → service → feature. Do not special-case it in an existing module.
- Adding a Windows version/edition difference belongs in the adapter, expressed as
  an `Unsupported` outcome — never a scattered `if windows_version` in the UI.
- A new persisted entity needs: migration, repository, retention rule, backup
  impact note, and privacy classification.

## 9. Forbidden patterns

| Forbidden | Why | Instead |
|---|---|---|
| `invoke()` inside a component | Bypasses typed adapters, untestable | Add a `services/` function |
| Win32 call in an async handler | Blocks the reactor | `spawn_blocking` + adapter |
| `Arc<Mutex<HashMap<...>>>` engine singleton | Hidden coupling, deadlocks | App-owned job registry with explicit API |
| `String` for paths crossing layers | Encoding/permission confusion | `PathBuf` internally, validated DTO at boundary |
| `unwrap()` on OS results | Crash on hostile input | Typed outcome |
| `any` in TS DTOs | Hides contract drift | Explicit types + runtime guard in dev |
| Catch-all `Err(_) => default` | Hides partial results | Propagate + report `partial` |
| SQL string built from IPC input | Injection/DoS | Parameterized queries + validated values |

## 10. Exceptions

An exception requires:

1. an ADR in [`docs/DECISIONS.md`](docs/DECISIONS.md) with context, options,
   decision, trade-offs, and revisit trigger;
2. a note in [`AI_CHANGELOG.md`](AI_CHANGELOG.md);
3. a test or review evidence proving the boundary risk is contained.
