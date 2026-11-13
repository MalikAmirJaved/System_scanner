//! Read-only system facts and the snapshot use case.
//!
//! This module is platform-neutral on purpose: it never imports `windows-sys`,
//! Tauri, or SQL. Windows probes arrive through
//! [`crate::ports::system_info::SystemInfoPort`].

use crate::ports::ProbeError;
use crate::ports::system_info::SystemInfoPort;

/// Why a field could not be collected. Serialized as a stable `snake_case` code.
///
/// Adding a variant is an additive IPC change; renaming one is breaking and must
/// be documented in `API.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnavailableReason {
    AccessDenied,
    Unsupported,
    Failed,
}

impl UnavailableReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AccessDenied => "access_denied",
            Self::Unsupported => "unsupported",
            Self::Failed => "failed",
        }
    }
}

impl From<ProbeError> for UnavailableReason {
    fn from(error: ProbeError) -> Self {
        match error {
            ProbeError::AccessDenied => Self::AccessDenied,
            ProbeError::Failed => Self::Failed,
        }
    }
}

/// A value that is either collected or explicitly unavailable with a reason.
///
/// `Unavailable` is never represented as `0`, `""`, or `null` without a reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Availability<T> {
    Available(T),
    Unavailable(UnavailableReason),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OsInfo {
    pub name: Availability<String>,
    pub edition: Availability<String>,
    pub version: Availability<String>,
    pub build: Availability<String>,
    /// `GetNativeSystemInfo` always succeeds; `String` is honest here.
    pub arch: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpuInfo {
    pub model: Availability<String>,
    pub logical_cores: Availability<u32>,
    pub physical_cores: Availability<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryInfo {
    pub total_bytes: u64,
    pub available_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VolumeKind {
    Fixed,
    Removable,
    Network,
    Optical,
    Ram,
    Unknown,
}

impl VolumeKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Fixed => "fixed",
            Self::Removable => "removable",
            Self::Network => "network",
            Self::Optical => "optical",
            Self::Ram => "ram",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VolumeInfo {
    /// Mount root exactly as Windows reports it, for example `C:\`.
    pub mount: String,
    pub kind: VolumeKind,
    pub label: Availability<String>,
    pub filesystem: Availability<String>,
    pub total_bytes: Availability<u64>,
    pub free_bytes: Availability<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkAdapterInfo {
    pub name: String,
    pub description: Availability<String>,
    pub is_up: bool,
    /// Formatted addresses only; no connectivity probe is performed.
    pub addresses: Vec<String>,
}

/// One point-in-time reading of the machine. Every group may be unavailable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemSnapshot {
    pub captured_at_ms: u64,
    pub os: Availability<OsInfo>,
    pub cpu: Availability<CpuInfo>,
    pub memory: Availability<MemoryInfo>,
    pub volumes: Availability<Vec<VolumeInfo>>,
    pub network_adapters: Availability<Vec<NetworkAdapterInfo>>,
}

/// Collects all groups through the port. Never panics and never fabricates
/// values: a failing group becomes `Unavailable` with the mapped reason.
pub fn collect_snapshot(port: &dyn SystemInfoPort, captured_at_ms: u64) -> SystemSnapshot {
    SystemSnapshot {
        captured_at_ms,
        os: group(port.os_info()),
        cpu: group(port.cpu_info()),
        memory: group(port.memory_info()),
        volumes: group(port.volumes()),
        network_adapters: group(port.network_adapters()),
    }
}

fn group<T>(result: Result<T, ProbeError>) -> Availability<T> {
    match result {
        Ok(value) => Availability::Available(value),
        Err(error) => Availability::Unavailable(UnavailableReason::from(error)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakePort {
        os: Result<OsInfo, ProbeError>,
        cpu: Result<CpuInfo, ProbeError>,
        memory: Result<MemoryInfo, ProbeError>,
        volumes: Result<Vec<VolumeInfo>, ProbeError>,
        adapters: Result<Vec<NetworkAdapterInfo>, ProbeError>,
    }

    impl Default for FakePort {
        fn default() -> Self {
            Self {
                os: Ok(OsInfo {
                    name: Availability::Available("Windows 11 Pro".to_owned()),
                    edition: Availability::Available("Professional".to_owned()),
                    version: Availability::Available("10.0.26100".to_owned()),
                    build: Availability::Available("26100".to_owned()),
                    arch: "x64".to_owned(),
                }),
                cpu: Ok(CpuInfo {
                    model: Availability::Available("Example CPU".to_owned()),
                    logical_cores: Availability::Available(16),
                    physical_cores: Availability::Available(8),
                }),
                memory: Ok(MemoryInfo {
                    total_bytes: 34_359_738_368,
                    available_bytes: 12_884_901_888,
                }),
                volumes: Ok(vec![VolumeInfo {
                    mount: "C:\\".to_owned(),
                    kind: VolumeKind::Fixed,
                    label: Availability::Available("System".to_owned()),
                    filesystem: Availability::Available("NTFS".to_owned()),
                    total_bytes: Availability::Available(1_000_000),
                    free_bytes: Availability::Available(500_000),
                }]),
                adapters: Ok(vec![NetworkAdapterInfo {
                    name: "Ethernet".to_owned(),
                    description: Availability::Available("Example adapter".to_owned()),
                    is_up: true,
                    addresses: vec!["192.0.2.10".to_owned()],
                }]),
            }
        }
    }

    impl SystemInfoPort for FakePort {
        fn os_info(&self) -> Result<OsInfo, ProbeError> {
            self.os.clone()
        }

        fn cpu_info(&self) -> Result<CpuInfo, ProbeError> {
            self.cpu.clone()
        }

        fn memory_info(&self) -> Result<MemoryInfo, ProbeError> {
            self.memory.clone()
        }

        fn volumes(&self) -> Result<Vec<VolumeInfo>, ProbeError> {
            self.volumes.clone()
        }

        fn network_adapters(&self) -> Result<Vec<NetworkAdapterInfo>, ProbeError> {
            self.adapters.clone()
        }
    }

    #[test]
    fn a_successful_probe_marks_every_group_available() {
        let snapshot = collect_snapshot(&FakePort::default(), 1_762_387_200_000);

        assert_eq!(snapshot.captured_at_ms, 1_762_387_200_000);
        assert_eq!(
            snapshot.os,
            Availability::Available(FakePort::default().os.unwrap())
        );
        assert!(matches!(snapshot.cpu, Availability::Available(_)));
        assert!(matches!(snapshot.memory, Availability::Available(_)));
        assert!(matches!(snapshot.volumes, Availability::Available(_)));
        assert!(matches!(
            snapshot.network_adapters,
            Availability::Available(_)
        ));
    }

    #[test]
    fn a_denied_group_becomes_unavailable_with_the_mapped_reason() {
        let port = FakePort {
            memory: Err(ProbeError::AccessDenied),
            ..FakePort::default()
        };

        let snapshot = collect_snapshot(&port, 0);

        assert_eq!(
            snapshot.memory,
            Availability::Unavailable(UnavailableReason::AccessDenied)
        );
        // Other groups are unaffected: partial success must stay partial.
        assert!(matches!(snapshot.os, Availability::Available(_)));
    }

    #[test]
    fn unsupported_and_failed_reasons_map_explicitly() {
        let port = FakePort {
            cpu: Err(ProbeError::AccessDenied),
            volumes: Err(ProbeError::Failed),
            ..FakePort::default()
        };

        let snapshot = collect_snapshot(&port, 0);

        assert_eq!(
            snapshot.cpu,
            Availability::Unavailable(UnavailableReason::AccessDenied)
        );
        assert_eq!(
            snapshot.volumes,
            Availability::Unavailable(UnavailableReason::Failed)
        );
        assert_eq!(UnavailableReason::Unsupported.as_str(), "unsupported");
        assert_eq!(UnavailableReason::Failed.as_str(), "failed");
    }

    #[test]
    fn field_level_availability_survives_a_group_probe() {
        let port = FakePort {
            cpu: Ok(CpuInfo {
                model: Availability::Unavailable(UnavailableReason::Unsupported),
                logical_cores: Availability::Available(4),
                physical_cores: Availability::Unavailable(UnavailableReason::Failed),
            }),
            ..FakePort::default()
        };

        let snapshot = collect_snapshot(&port, 0);

        match snapshot.cpu {
            Availability::Available(cpu) => {
                assert!(matches!(
                    cpu.model,
                    Availability::Unavailable(UnavailableReason::Unsupported)
                ));
                assert!(matches!(cpu.logical_cores, Availability::Available(4)));
                assert!(matches!(
                    cpu.physical_cores,
                    Availability::Unavailable(UnavailableReason::Failed)
                ));
            }
            Availability::Unavailable(reason) => {
                panic!("cpu should be partially available: {reason:?}")
            }
        }
    }
}
