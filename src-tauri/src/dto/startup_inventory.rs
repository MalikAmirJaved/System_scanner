use serde::Serialize;

use crate::domain::startup as domain;
use crate::dto::unavailable::{UnavailableFieldDto, take, take_group};

/// Bump when a breaking field change is made to this DTO.
pub const STARTUP_INVENTORY_SCHEMA_VERSION: u32 = 1;

/// Wire shape of `get_startup_inventory` data (see `API.md` §3.5).
///
/// A missing group or leaf field is always accompanied by an entry in the
/// top-level `unavailable` list; `null` alone never means "could not read".
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupInventoryDto {
    pub captured_at: u64,
    pub schema_version: u32,
    /// Services enumerated. Absent when the service database itself was refused.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub services: Option<Vec<ServiceDto>>,
    /// Every startup source, including the ones that could not be read.
    pub startup: Vec<StartupSourceDto>,
    /// The service cap stopped enumeration; the list is a prefix.
    pub services_truncated: bool,
    /// The startup-entry cap stopped enumeration; the lists are a prefix.
    pub startup_truncated: bool,
    pub unavailable: Vec<UnavailableFieldDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceDto {
    /// SCM key name; the identity of the row.
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// One of `running`, `stopped`, `start_pending`, `stop_pending`,
    /// `continue_pending`, `pause_pending`, `paused`, `unknown`.
    pub state: &'static str,
    /// One of `boot`, `system`, `automatic`, `manual`, `disabled`, `unknown`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_type: Option<&'static str>,
    /// Configured binary path, including arguments. Local to this machine.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary_path: Option<String>,
    pub unavailable: Vec<UnavailableFieldDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupSourceDto {
    /// `current_user` or `all_users`.
    pub scope: &'static str,
    /// `registry_run`, `registry_run_once`, or `startup_folder`.
    pub source: &'static str,
    /// Absent when this source was refused, and named in `unavailable`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entries: Option<Vec<StartupEntryDto>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupEntryDto {
    /// Registry value name, or the file name of a startup-folder item.
    pub name: String,
    /// Registry value data, or the item's own path. Local to this machine.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
}

impl StartupInventoryDto {
    pub fn from_inventory(inventory: &domain::StartupInventory, captured_at: u64) -> Self {
        let mut unavailable = Vec::new();

        let services =
            take_group("services", &inventory.services, &mut unavailable).map(|services| {
                let mut items = Vec::with_capacity(services.len());
                for (index, service) in services.iter().enumerate() {
                    let (item, mut gaps) = service_from(index, service);
                    items.push(item);
                    unavailable.append(&mut gaps);
                }
                items
            });

        let mut startup = Vec::with_capacity(inventory.startup.len());
        for (index, group) in inventory.startup.iter().enumerate() {
            let prefix = format!("startup[{index}]");
            let entries = take_group(
                &format!("{prefix}.entries"),
                &group.entries,
                &mut unavailable,
            )
            .map(|entries| {
                entries
                    .iter()
                    .enumerate()
                    .map(|(entry_index, entry)| StartupEntryDto {
                        name: entry.name.clone(),
                        command: take(
                            &format!("{prefix}.entries[{entry_index}].command"),
                            &entry.command,
                            &mut unavailable,
                        ),
                    })
                    .collect()
            });

            startup.push(StartupSourceDto {
                scope: group.scope.as_str(),
                source: group.source.as_str(),
                entries,
            });
        }

        Self {
            captured_at,
            schema_version: STARTUP_INVENTORY_SCHEMA_VERSION,
            services,
            startup,
            services_truncated: inventory.services_truncated,
            startup_truncated: inventory.startup_truncated,
            unavailable,
        }
    }
}

fn service_from(
    index: usize,
    service: &domain::ServiceInfo,
) -> (ServiceDto, Vec<UnavailableFieldDto>) {
    let prefix = format!("services[{index}]");
    let mut gaps = Vec::new();

    let item = ServiceDto {
        name: service.name.clone(),
        display_name: take(
            &format!("{prefix}.displayName"),
            &service.display_name,
            &mut gaps,
        ),
        state: service.state.as_str(),
        start_type: take(
            &format!("{prefix}.startType"),
            &service.start_type,
            &mut gaps,
        )
        .map(|start_type| start_type.as_str()),
        binary_path: take(
            &format!("{prefix}.binaryPath"),
            &service.binary_path,
            &mut gaps,
        ),
        unavailable: Vec::new(),
    };

    (item, gaps)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::startup::{
        ServiceInfo, ServiceStartType, ServiceState, StartupEntry, StartupInventory, StartupScope,
        StartupSource, StartupSourceGroup,
    };
    use crate::domain::system::{Availability, UnavailableReason};

    fn service(name: &str) -> ServiceInfo {
        ServiceInfo {
            name: name.to_owned(),
            display_name: Availability::Available(format!("Display {name}")),
            state: ServiceState::Running,
            start_type: Availability::Available(ServiceStartType::Automatic),
            binary_path: Availability::Available("C:\\Windows\\System32\\svc.exe".to_owned()),
        }
    }

    fn inventory() -> StartupInventory {
        StartupInventory {
            services: Availability::Available(vec![service("Dhcp")]),
            startup: vec![StartupSourceGroup {
                scope: StartupScope::CurrentUser,
                source: StartupSource::RegistryRun,
                entries: Availability::Available(vec![StartupEntry {
                    name: "Update".to_owned(),
                    command: Availability::Available("C:\\bin\\update.exe".to_owned()),
                }]),
            }],
            services_truncated: false,
            startup_truncated: false,
        }
    }

    #[test]
    fn a_full_inventory_serializes_camel_case_with_schema_version() {
        let value =
            serde_json::to_value(StartupInventoryDto::from_inventory(&inventory(), 7)).unwrap();

        assert_eq!(value["capturedAt"], 7u64);
        assert_eq!(value["schemaVersion"], STARTUP_INVENTORY_SCHEMA_VERSION);
        assert_eq!(value["services"][0]["name"], "Dhcp");
        assert_eq!(value["services"][0]["state"], "running");
        assert_eq!(value["services"][0]["startType"], "automatic");
        assert_eq!(
            value["services"][0]["binaryPath"],
            "C:\\Windows\\System32\\svc.exe"
        );
        assert_eq!(value["services"][0]["unavailable"], serde_json::json!([]));
        assert_eq!(value["startup"][0]["scope"], "current_user");
        assert_eq!(value["startup"][0]["source"], "registry_run");
        assert_eq!(value["startup"][0]["entries"][0]["name"], "Update");
        assert_eq!(
            value["startup"][0]["entries"][0]["command"],
            "C:\\bin\\update.exe"
        );
        assert_eq!(value["servicesTruncated"], false);
        assert_eq!(value["startupTruncated"], false);
        assert_eq!(value["unavailable"], serde_json::json!([]));
        assert!(value["services"][0].get("binary_path").is_none());
    }

    #[test]
    fn an_unreadable_service_database_is_omitted_and_explained() {
        let mut inventory = inventory();
        inventory.services = Availability::Unavailable(UnavailableReason::AccessDenied);

        let value =
            serde_json::to_value(StartupInventoryDto::from_inventory(&inventory, 0)).unwrap();

        assert!(value.get("services").is_none());
        assert_eq!(
            value["unavailable"],
            serde_json::json!([{ "field": "services", "reason": "access_denied" }])
        );
    }

    #[test]
    fn a_denied_service_configuration_keeps_the_row_and_names_its_fields() {
        let mut denied = service("Dhcp");
        denied.start_type = Availability::Unavailable(UnavailableReason::AccessDenied);
        denied.binary_path = Availability::Unavailable(UnavailableReason::AccessDenied);
        let mut inventory = inventory();
        inventory.services = Availability::Available(vec![denied]);

        let value =
            serde_json::to_value(StartupInventoryDto::from_inventory(&inventory, 0)).unwrap();

        assert_eq!(value["services"][0]["name"], "Dhcp");
        assert_eq!(value["services"][0]["state"], "running");
        assert!(value["services"][0].get("startType").is_none());
        assert!(value["services"][0].get("binaryPath").is_none());
        assert_eq!(
            value["unavailable"],
            serde_json::json!([
                { "field": "services[0].startType", "reason": "access_denied" },
                { "field": "services[0].binaryPath", "reason": "access_denied" }
            ])
        );
    }

    #[test]
    fn a_denied_source_keeps_its_group_so_the_reason_is_visible() {
        let mut inventory = inventory();
        inventory.startup[0].entries = Availability::Unavailable(UnavailableReason::Failed);

        let value =
            serde_json::to_value(StartupInventoryDto::from_inventory(&inventory, 0)).unwrap();

        assert_eq!(value["startup"][0]["source"], "registry_run");
        assert!(value["startup"][0].get("entries").is_none());
        assert_eq!(
            value["unavailable"],
            serde_json::json!([{ "field": "startup[0].entries", "reason": "failed" }])
        );
    }

    #[test]
    fn a_denied_command_names_the_entry_path() {
        let mut inventory = inventory();
        inventory.startup[0].entries = Availability::Available(vec![
            StartupEntry {
                name: "A".to_owned(),
                command: Availability::Available("a.exe".to_owned()),
            },
            StartupEntry {
                name: "B".to_owned(),
                command: Availability::Unavailable(UnavailableReason::Unsupported),
            },
        ]);

        let value =
            serde_json::to_value(StartupInventoryDto::from_inventory(&inventory, 0)).unwrap();

        assert_eq!(value["startup"][0]["entries"][1]["name"], "B");
        assert!(value["startup"][0]["entries"][1].get("command").is_none());
        assert_eq!(
            value["unavailable"],
            serde_json::json!([
                { "field": "startup[0].entries[1].command", "reason": "unsupported" }
            ])
        );
    }
}
