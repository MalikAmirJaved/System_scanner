//! Read-only identity and privilege facts used for diagnostics.
//!
//! The values are used as redaction secrets and as a single environment fact;
//! they are never returned to the renderer in clear text.

/// Account and machine names that must not appear in a support bundle.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct LocalIdentity {
    pub user_name: Option<String>,
    pub computer_name: Option<String>,
}

/// Narrow capability implemented by a platform adapter.
pub trait DiagnosticsPort {
    /// Names to remove from diagnostic text. Missing values are acceptable: the
    /// shape-based redaction rules still run.
    fn identity(&self) -> LocalIdentity;

    /// Whether the current process token is elevated. `None` when unknown.
    fn is_elevated(&self) -> Option<bool>;
}
