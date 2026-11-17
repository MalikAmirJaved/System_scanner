-- 0002_scan_session_results.sql
--
-- Adds the result columns a finished scan session needs beyond schema v1:
-- the coverage verdict and the counters v1 did not have a column for.
-- Additive only; existing rows keep their values and get the defaults below.
--
-- Scope is still counters and lifecycle. No path, file name, or per-entry data
-- is stored; `roots_json` stays '[]' until the retention decision in
-- `docs/plan.md` §12 is resolved.

ALTER TABLE scan_sessions ADD COLUMN completeness TEXT;                              -- complete|partial|truncated|cancelled; NULL until finished
ALTER TABLE scan_sessions ADD COLUMN entries_seen INTEGER NOT NULL DEFAULT 0;        -- every observed entry
ALTER TABLE scan_sessions ADD COLUMN directories_seen INTEGER NOT NULL DEFAULT 0;
ALTER TABLE scan_sessions ADD COLUMN reparse_points_seen INTEGER NOT NULL DEFAULT 0;
ALTER TABLE scan_sessions ADD COLUMN files_disappeared INTEGER NOT NULL DEFAULT 0;
ALTER TABLE scan_sessions ADD COLUMN files_unsupported INTEGER NOT NULL DEFAULT 0;
