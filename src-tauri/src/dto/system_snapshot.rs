use serde::Serialize;

use crate::domain::system as domain;
use crate::domain::system::Availability;

/// Bump when a breaking field change is made to this DTO.
pub const SNAPSHOT_SCHEMA_VERSION: u32 = 1;

/// Wire shape of `get_system_snapshot` data (see `API.md` §3.2).
///
/// A missing group or leaf field is always accompanied by an entry in
/// `unavailable`; `null` alone never means "could not read".
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemSnapshotDto {
    pub captured_at: u64,
    pub schema_version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os: Option<OsDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu: Option<CpuDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<MemoryDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disks: Option<Vec<DiskDto>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<Vec<NetworkAdapterDto>>,
    pub unavailable: Vec<UnavailableFieldDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OsDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build: Option<String>,
    pub arch: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CpuDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logical_cores: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub physical_cores: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryDto {
    pub total_bytes: u64,
    pub available_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskDto {
    pub mount: String,
    pub kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filesystem: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub free_bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkAdapterDto {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub is_up: bool,
    pub addresses: Vec<String>,
}

/// One field path that could not be collected, with a stable reason code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnavailableFieldDto {
    pub field: String,
    pub reason: &'static str,
}

impl From<&domain::SystemSnapshot> for SystemSnapshotDto {
    fn from(snapshot: &domain::SystemSnapshot) -> Self {
        let mut unavailable = Vec::new();

        let os = take_group("os", &snapshot.os, &mut unavailable).map(|info| OsDto {
            name: take("os.name", &info.name, &mut unavailable),
            edition: take("os.edition", &info.edition, &mut unavailable),
            version: take("os.version", &info.version, &mut unavailable),
            build: take("os.build", &info.build, &mut unavailable),
            arch: info.arch.clone(),
        });

        let cpu = take_group("cpu", &snapshot.cpu, &mut unavailable).map(|info| CpuDto {
            model: take("cpu.model", &info.model, &mut unavailable),
            logical_cores: take("cpu.logicalCores", &info.logical_cores, &mut unavailable),
            physical_cores: take("cpu.physicalCores", &info.physical_cores, &mut unavailable),
        });

        let memory =
            take_group("memory", &snapshot.memory, &mut unavailable).map(|info| MemoryDto {
                total_bytes: info.total_bytes,
                available_bytes: info.available_bytes,
            });

        let disks = take_group("disks", &snapshot.volumes, &mut unavailable).map(|volumes| {
            let mut items = Vec::with_capacity(volumes.len());
            for (index, volume) in volumes.iter().enumerate() {
                let (disk, mut gaps) = disk_from(index, volume);
                items.push(disk);
                unavailable.append(&mut gaps);
            }
            items
        });

        let network =
            take_group("network", &snapshot.network_adapters, &mut unavailable).map(|adapters| {
                let mut items = Vec::with_capacity(adapters.len());
                for (index, adapter) in adapters.iter().enumerate() {
                    let (item, mut gaps) = adapter_from(index, adapter);
                    items.push(item);
                    unavailable.append(&mut gaps);
                }
                items
            });

        Self {
            captured_at: snapshot.captured_at_ms,
            schema_version: SNAPSHOT_SCHEMA_VERSION,
            os,
            cpu,
            memory,
            disks,
            network,
            unavailable,
        }
    }
}

fn disk_from(index: usize, volume: &domain::VolumeInfo) -> (DiskDto, Vec<UnavailableFieldDto>) {
    let prefix = format!("disks[{index}]");
    let mut gaps = Vec::new();

    let disk = DiskDto {
        mount: volume.mount.clone(),
        kind: volume.kind.as_str(),
        label: take(&format!("{prefix}.label"), &volume.label, &mut gaps),
        filesystem: take(
            &format!("{prefix}.filesystem"),
            &volume.filesystem,
            &mut gaps,
        ),
        total_bytes: take(
            &format!("{prefix}.totalBytes"),
            &volume.total_bytes,
            &mut gaps,
        ),
        free_bytes: take(
            &format!("{prefix}.freeBytes"),
            &volume.free_bytes,
            &mut gaps,
        ),
    };

    (disk, gaps)
}

fn adapter_from(
    index: usize,
    adapter: &domain::NetworkAdapterInfo,
) -> (NetworkAdapterDto, Vec<UnavailableFieldDto>) {
    let mut gaps = Vec::new();
    let item = NetworkAdapterDto {
        name: adapter.name.clone(),
        description: take(
            &format!("network[{index}].description"),
            &adapter.description,
            &mut gaps,
        ),
        is_up: adapter.is_up,
        addresses: adapter.addresses.clone(),
    };

    (item, gaps)
}

fn take_group<'a, T>(
    field: &str,
    group: &'a Availability<T>,
    unavailable: &mut Vec<UnavailableFieldDto>,
) -> Option<&'a T> {
    match group {
        Availability::Available(value) => Some(value),
        Availability::Unavailable(reason) => {
            unavailable.push(UnavailableFieldDto {
                field: field.to_owned(),
                reason: reason.as_str(),
            });
            None
        }
    }
}

fn take<T: Clone>(
    field: &str,
    value: &Availability<T>,
    unavailable: &mut Vec<UnavailableFieldDto>,
) -> Option<T> {
    match value {
        Availability::Available(value) => Some(value.clone()),
        Availability::Unavailable(reason) => {
            unavailable.push(UnavailableFieldDto {
                field: field.to_owned(),
                reason: reason.as_str(),
            });
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::system::{
        Availability, CpuInfo, MemoryInfo, NetworkAdapterInfo, OsInfo, SystemSnapshot,
        UnavailableReason, VolumeInfo, VolumeKind,
    };

    fn full_snapshot() -> SystemSnapshot {
        SystemSnapshot {
            captured_at_ms: 1_762_387_200_000,
            os: Availability::Available(OsInfo {
                name: Availability::Available("Windows 11 Pro".to_owned()),
                edition: Availability::Available("Professional".to_owned()),
                version: Availability::Available("10.0.26100".to_owned()),
                build: Availability::Available("26100".to_owned()),
                arch: "x64".to_owned(),
            }),
            cpu: Availability::Available(CpuInfo {
                model: Availability::Available("Example CPU".to_owned()),
                logical_cores: Availability::Available(16),
                physical_cores: Availability::Available(8),
            }),
            memory: Availability::Available(MemoryInfo {
                total_bytes: 34_359_738_368,
                available_bytes: 12_884_901_888,
            }),
            volumes: Availability::Available(vec![VolumeInfo {
                mount: "C:\\".to_owned(),
                kind: VolumeKind::Fixed,
                label: Availability::Available("System".to_owned()),
                filesystem: Availability::Available("NTFS".to_owned()),
                total_bytes: Availability::Available(1_000_000),
                free_bytes: Availability::Available(500_000),
            }]),
            network_adapters: Availability::Available(vec![NetworkAdapterInfo {
                name: "Ethernet".to_owned(),
                description: Availability::Available("Example adapter".to_owned()),
                is_up: true,
                addresses: vec!["192.0.2.10".to_owned()],
            }]),
        }
    }

    #[test]
    fn full_snapshot_serializes_camel_case_with_schema_version() {
        let dto = SystemSnapshotDto::from(&full_snapshot());
        let value = serde_json::to_value(&dto).unwrap();

        assert_eq!(value["capturedAt"], 1_762_387_200_000u64);
        assert_eq!(value["schemaVersion"], SNAPSHOT_SCHEMA_VERSION);
        assert_eq!(value["os"]["name"], "Windows 11 Pro");
        assert_eq!(value["cpu"]["logicalCores"], 16);
        assert_eq!(value["cpu"]["physicalCores"], 8);
        assert_eq!(value["memory"]["totalBytes"], 34_359_738_368u64);
        assert_eq!(value["disks"][0]["mount"], "C:\\");
        assert_eq!(value["disks"][0]["kind"], "fixed");
        assert_eq!(value["network"][0]["isUp"], true);
        assert_eq!(value["unavailable"], serde_json::json!([]));
        assert!(value["cpu"].get("logical_cores").is_none());
    }

    #[test]
    fn an_unavailable_group_is_omitted_and_explained() {
        let mut snapshot = full_snapshot();
        snapshot.memory = Availability::Unavailable(UnavailableReason::AccessDenied);

        let value = serde_json::to_value(SystemSnapshotDto::from(&snapshot)).unwrap();

        assert!(value.get("memory").is_none());
        assert_eq!(
            value["unavailable"],
            serde_json::json!([{ "field": "memory", "reason": "access_denied" }])
        );
    }

    #[test]
    fn a_missing_leaf_field_keeps_its_group_and_names_the_field() {
        let mut snapshot = full_snapshot();
        snapshot.cpu = Availability::Available(CpuInfo {
            model: Availability::Unavailable(UnavailableReason::Unsupported),
            logical_cores: Availability::Available(4),
            physical_cores: Availability::Unavailable(UnavailableReason::Failed),
        });

        let value = serde_json::to_value(SystemSnapshotDto::from(&snapshot)).unwrap();

        assert_eq!(value["cpu"]["logicalCores"], 4);
        assert!(value["cpu"].get("model").is_none());
        assert_eq!(
            value["unavailable"],
            serde_json::json!([
                { "field": "cpu.model", "reason": "unsupported" },
                { "field": "cpu.physicalCores", "reason": "failed" }
            ])
        );
    }

    #[test]
    fn per_disk_gaps_are_reported_not_swallowed() {
        let mut snapshot = full_snapshot();
        snapshot.volumes = Availability::Available(vec![VolumeInfo {
            mount: "Z:\\".to_owned(),
            kind: VolumeKind::Network,
            label: Availability::Unavailable(UnavailableReason::Unsupported),
            filesystem: Availability::Unavailable(UnavailableReason::Unsupported),
            total_bytes: Availability::Unavailable(UnavailableReason::Unsupported),
            free_bytes: Availability::Unavailable(UnavailableReason::Unsupported),
        }]);

        let value = serde_json::to_value(SystemSnapshotDto::from(&snapshot)).unwrap();

        assert_eq!(value["disks"][0]["kind"], "network");
        assert_eq!(
            value["unavailable"],
            serde_json::json!([
                { "field": "disks[0].label", "reason": "unsupported" },
                { "field": "disks[0].filesystem", "reason": "unsupported" },
                { "field": "disks[0].totalBytes", "reason": "unsupported" },
                { "field": "disks[0].freeBytes", "reason": "unsupported" }
            ])
        );
    }
}
