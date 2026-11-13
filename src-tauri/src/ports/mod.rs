//! Narrow capability traits. Implementations live in `adapters/`.

mod probe;
pub mod processes;
pub mod system_info;

pub use probe::ProbeError;
