//! Read-only system information probes.
//!
//! The trait is deliberately one capability: a platform adapter implements it
//! with Win32 calls. Domain code depends on this trait, never on the adapter.

use crate::domain::system::{CpuInfo, MemoryInfo, NetworkAdapterInfo, OsInfo, VolumeInfo};

/// Group-level probe failure.
///
/// Field-level gaps inside a successful group are represented by
/// [`crate::domain::system::Availability`]; this type is only for the case where
/// the whole group could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeError {
    AccessDenied,
    Failed,
}

/// Reads static and semi-static facts about the local machine.
///
/// Implementations are read-only, must not block on network resources, and must
/// never panic on an OS failure.
pub trait SystemInfoPort {
    fn os_info(&self) -> Result<OsInfo, ProbeError>;

    fn cpu_info(&self) -> Result<CpuInfo, ProbeError>;

    fn memory_info(&self) -> Result<MemoryInfo, ProbeError>;

    fn volumes(&self) -> Result<Vec<VolumeInfo>, ProbeError>;

    fn network_adapters(&self) -> Result<Vec<NetworkAdapterInfo>, ProbeError>;
}
