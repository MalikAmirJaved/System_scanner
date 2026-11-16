-- 0001_initial.sql
--
-- First application schema. Additive only; never edit after release.
-- The migration runner records the version and checksum in `schema_migrations`
-- before this change is considered applied.
--
-- Scope: session lifecycle and counters only. File records, snapshots, hashes,
-- and action history arrive with the features that write them, once the
-- retention decision in `docs/plan.md` §12 is resolved.

CREATE TABLE scan_sessions (
  id                TEXT PRIMARY KEY,              -- UUID, generated in Rust
  module            TEXT NOT NULL,                 -- system|processes|storage|security|network|diagnostics
  roots_json        TEXT NOT NULL DEFAULT '[]',    -- requested scope (validated in Rust)
  options_json      TEXT NOT NULL DEFAULT '{}',    -- concurrency, limits, policies
  status            TEXT NOT NULL,                 -- queued|running|cancelling|completed|failed|cancelled|interrupted
  started_at        INTEGER NOT NULL,              -- unix ms UTC
  finished_at       INTEGER,
  engine_version    TEXT NOT NULL,
  policy_version    TEXT NOT NULL,
  files_seen        INTEGER NOT NULL DEFAULT 0,
  files_skipped     INTEGER NOT NULL DEFAULT 0,
  files_denied      INTEGER NOT NULL DEFAULT 0,
  files_failed      INTEGER NOT NULL DEFAULT 0,
  bytes_seen        INTEGER NOT NULL DEFAULT 0,
  cancel_reason     TEXT,
  error_code        TEXT,                          -- joins the app error taxonomy
  parent_session_id TEXT REFERENCES scan_sessions(id)
);

CREATE INDEX scan_sessions_started_at ON scan_sessions(started_at DESC);
CREATE INDEX scan_sessions_module_status ON scan_sessions(module, status);
