//! Shared failure type for capability probes.

/// Group-level probe failure.
///
/// Field-level gaps inside a successful probe are represented by
/// [`crate::domain::system::Availability`]; this type is only for the case where
/// the whole probe could not be read at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeError {
    AccessDenied,
    Failed,
}
