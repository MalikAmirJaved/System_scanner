//! Read-only process inventory.

use crate::domain::process::ProcessInventory;
use crate::ports::ProbeError;

/// Enumerates the processes visible to the current user.
///
/// The implementation must bound its own work: it returns at most a fixed number
/// of processes and sets [`ProcessInventory::truncated`] when the cap stopped it.
/// A single unreadable process must never fail the whole inventory; only a
/// total enumeration failure returns `Err`.
pub trait ProcessInfoPort {
    fn inventory(&self) -> Result<ProcessInventory, ProbeError>;
}
