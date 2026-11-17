//! Application orchestration: job lifecycle, cancellation, and event pacing.
//!
//! Sits between `commands/` (thin IPC handlers) and the adapters. It owns no
//! Windows or Tauri types, so it is testable with a plain closure as the event
//! sink.

pub mod jobs;
