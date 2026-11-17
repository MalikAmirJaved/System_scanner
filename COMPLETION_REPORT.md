# Completion Report — B-004 recording + B-040 threat-model review

**Generated:** 2026-11-17  
**Base commit:** `eded2b4` (B-033 already committed; tree clean)  
**Work tree:** clean after verification gates

## Changed

These files were modified in this session:

| File | Change |
|---|---|
| `docs/BACKLOG.md` | ✅ Marked B-004 as complete; added Done row citing `src/App.tsx`, `src/main.tsx`, five panels with loading/empty/error states, no router; updated F-006 evidence |
| `docs/FEATURES.md` | Updated F-006 to cite B-004 evidence (`src/App.tsx`, responsive grid, `sr-only` h1, no router; every panel has loading/empty/error states) |
| `docs/THREAT_MODEL.md` | Updated status header ("Reviewed for the implemented read-only file surface (B-040, 2026-11-17)"); added §11 "Review 2026-11-17 — file operations (B-040)" with 7 findings (F-01–F-07), new threat T-14, residual-risk table, and open decisions (11.7); annotated T-02/T-03/T-09 |
| `SECURITY.md` | Updated status header ("Pre-alpha. The read-only file surface had a paper threat-model review (B-040, 2026-11-17)"); updated gap row #5 to reference B-040 review and open decision |
| `docs/ACCESS_CONTROL.md` | Updated §4 gap row to cite B-040 review; added §9 question 5 (network/removable) and question 6 (folder-picker constraint); added KI-052 (check-then-open race) |
| `docs/KNOWN_ISSUES.md` | Updated KI-049 to cite B-040 review and open decision; added KI-052 (check-then-open race); updated KI-002 reference (B-050 instead of B-004); updated KI-024 status |
| `docs/RISKS.md` | Updated R-12 status from "Not started" to "Partially mitigated" with B-040 review context |
| `docs/CHANGELOG.md` | Added Security section entry for B-040 review (threats T-14, F-01–F-07, open decisions); updated Documentation section to record B-004 as complete |
| `AI_CHANGELOG.md` | Added `## 2026-11-17 — File-operation threat-model review (B-040)` entry; original B-033 entry preserved (lines 55–81) |

## Verified

All gates pass after the changes:

| Gate | Command | Outcome |
|---|---|---|
| Rust fmt | `cargo fmt --all -- --check` (src-tauri) | Clean |
| Rust clippy | `cargo clippy --all-targets -- -D warnings` (src-tauri) | Clean |
| Rust tests | `cargo test` (src-tauri) | **125 passed, 0 failed** |
| Frontend tests | `npm test` (vitest) | **76 passed, 0 failed** |
| Frontend build | `npm run build` | Clean |
| Format check | `npm run format:check` (prettier) | All matched |

No new dependencies were added. No source code was changed — this is a documentation-and-review deliverable only.

## Not verified

The following were noted as out of scope for this session and could not be verified:

- Live ACL fixture for UNC/network root classification (open product decision, not implemented)
- Handle-based open + identity check to close F-01 (B-041, future work)
- Folder-picker capability and allow-list implementation (open product decision)
- Any network egress or signed-update-channel work

These are explicitly recorded as **open decisions** (`docs/plan.md` §12, `THREAT_MODEL.md` §11.7) and will be addressed in follow-up items B-041+, not in this session.

## Known gaps / next step

The main remaining gate is the **retention/location decision** (`docs/plan.md` §12 #4) that blocks P1 items B-024 remainder, B-026–B-029, B-031, B-032. Until that decision is resolved, no further persistence or UI-scale work can proceed.

The immediate next backlog item after B-040 is **B-041** (protected-path policy + handle-based identity check to close F-01), which is the concrete follow-on for the check-then-open window documented here. B-041 will also decide whether UNC/network roots are blocked, warned about, or allowed with a folder-picker allow-list — the policy question left open in §11.7 of the review.

All other P2 items (B-042 through B-050) are blocked until B-041 closes the check-then-open window and the retention decision clears the P1 track.