//! Read-only startup entries and services.

use crate::domain::startup::StartupInventory;

/// Observes autostart entries and Windows services.
///
/// The implementation reports a limit per location: a source it could not read
/// carries a reason in its own group, and a source Windows does not have (a
/// missing folder) is an empty list. It never changes state and never needs
/// elevation: anything requiring more access is reported as unavailable.
///
/// The return type is not a `Result` on purpose — one unreadable registry key or
/// service configuration must not discard the rest of the inventory.
pub trait StartupInventoryPort {
    fn inventory(&self) -> StartupInventory;
}
