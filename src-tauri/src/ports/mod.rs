//! Narrow capability traits. Implementations live in `adapters/`.

pub mod diagnostics;
mod probe;
pub mod processes;
pub mod system_info;

pub use probe::ProbeError;
