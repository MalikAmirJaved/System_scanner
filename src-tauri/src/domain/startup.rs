//! Read-only startup entries and Windows services.
//!
//! Platform-neutral: Win32 enumeration arrives through
//! [`crate::ports::startup::StartupInventoryPort`]. Nothing here changes state;
//! both inventories are observations. A source Windows refused to read is
//! reported as unavailable with a reason, never as an empty list.

use crate::domain::system::Availability;
use crate::ports::startup::StartupInventoryPort;

/// Hard cap on services in one inventory. A normal machine has a few hundred;
/// the cap stops a hostile or broken service database from exhausting memory.
pub const MAX_SERVICES: usize = 4_096;

/// Hard cap on startup entries across every source.
pub const MAX_STARTUP_ENTRIES: usize = 512;

/// Current state of a service at capture time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceState {
    Running,
    Stopped,
    StartPending,
    StopPending,
    ContinuePending,
    PausePending,
    Paused,
    Unknown,
}

impl ServiceState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Stopped => "stopped",
            Self::StartPending => "start_pending",
            Self::StopPending => "stop_pending",
            Self::ContinuePending => "continue_pending",
            Self::PausePending => "pause_pending",
            Self::Paused => "paused",
            Self::Unknown => "unknown",
        }
    }
}

/// How a service is configured to start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceStartType {
    Boot,
    System,
    Automatic,
    Manual,
    Disabled,
    Unknown,
}

impl ServiceStartType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Boot => "boot",
            Self::System => "system",
            Self::Automatic => "automatic",
            Self::Manual => "manual",
            Self::Disabled => "disabled",
            Self::Unknown => "unknown",
        }
    }
}

/// One Windows service, as observed during a single enumeration.
///
/// `name` is the SCM key name and is the identity. Nothing else is required:
/// reading the configuration needs a second, per-service handle that can be
/// denied even when enumeration succeeded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceInfo {
    pub name: String,
    pub display_name: Availability<String>,
    pub state: ServiceState,
    pub start_type: Availability<ServiceStartType>,
    /// Binary path exactly as configured, including arguments. Local to this
    /// machine: never logged and never placed in a support bundle.
    pub binary_path: Availability<String>,
}

/// Which account a startup location belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupScope {
    CurrentUser,
    AllUsers,
}

impl StartupScope {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CurrentUser => "current_user",
            Self::AllUsers => "all_users",
        }
    }

    const fn rank(self) -> u8 {
        match self {
            Self::CurrentUser => 0,
            Self::AllUsers => 1,
        }
    }
}

/// Where Windows reads this class of autostart entry from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupSource {
    RegistryRun,
    RegistryRunOnce,
    StartupFolder,
}

impl StartupSource {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RegistryRun => "registry_run",
            Self::RegistryRunOnce => "registry_run_once",
            Self::StartupFolder => "startup_folder",
        }
    }

    const fn rank(self) -> u8 {
        match self {
            Self::RegistryRun => 0,
            Self::RegistryRunOnce => 1,
            Self::StartupFolder => 2,
        }
    }
}

/// One autostart entry inside a source group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartupEntry {
    /// Registry value name, or the file name for a startup-folder item.
    pub name: String,
    /// Registry value data for a registry source; the item's own path for a
    /// startup-folder source, whose shortcut target is not resolved. Local to
    /// this machine: never logged and never placed in a support bundle.
    pub command: Availability<String>,
}

/// One registry key or folder, so a denied source keeps its own explanation
/// instead of disappearing into an empty list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartupSourceGroup {
    pub scope: StartupScope,
    pub source: StartupSource,
    pub entries: Availability<Vec<StartupEntry>>,
}

/// Everything the adapter could observe, plus whether a cap stopped it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartupInventory {
    pub services: Availability<Vec<ServiceInfo>>,
    pub startup: Vec<StartupSourceGroup>,
    pub services_truncated: bool,
    pub startup_truncated: bool,
}

/// Reads through the port and puts every list in a deterministic order.
///
/// Ordering is a domain rule, not a property of the adapter: the UI groups by
/// source and compares refreshes, so the same machine state must always produce
/// the same sequence.
pub fn collect_inventory(port: &dyn StartupInventoryPort) -> StartupInventory {
    let mut inventory = port.inventory();

    if let Availability::Available(services) = &mut inventory.services {
        services.sort_by_key(|service| service.name.to_lowercase());
    }

    inventory
        .startup
        .sort_by_key(|group| (group.scope.rank(), group.source.rank()));

    for group in &mut inventory.startup {
        if let Availability::Available(entries) = &mut group.entries {
            entries.sort_by_key(|entry| entry.name.to_lowercase());
        }
    }

    inventory
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::system::UnavailableReason;

    fn service(name: &str) -> ServiceInfo {
        ServiceInfo {
            name: name.to_owned(),
            display_name: Availability::Available(format!("Display {name}")),
            state: ServiceState::Running,
            start_type: Availability::Available(ServiceStartType::Automatic),
            binary_path: Availability::Available("C:\\bin\\service.exe -k run".to_owned()),
        }
    }

    fn entry(name: &str) -> StartupEntry {
        StartupEntry {
            name: name.to_owned(),
            command: Availability::Available(format!("C:\\bin\\{name}.exe")),
        }
    }

    fn group(
        scope: StartupScope,
        source: StartupSource,
        entries: Availability<Vec<StartupEntry>>,
    ) -> StartupSourceGroup {
        StartupSourceGroup {
            scope,
            source,
            entries,
        }
    }

    struct FakePort {
        inventory: StartupInventory,
    }

    impl StartupInventoryPort for FakePort {
        fn inventory(&self) -> StartupInventory {
            self.inventory.clone()
        }
    }

    #[test]
    fn every_list_is_ordered_by_the_domain_not_the_adapter() {
        let port = FakePort {
            inventory: StartupInventory {
                services: Availability::Available(vec![
                    service("Windows Update"),
                    service("Appinfo"),
                    service("Dhcp"),
                ]),
                startup: vec![
                    group(
                        StartupScope::AllUsers,
                        StartupSource::StartupFolder,
                        Availability::Available(vec![entry("z-tool.exe"), entry("a-tool.lnk")]),
                    ),
                    group(
                        StartupScope::CurrentUser,
                        StartupSource::RegistryRun,
                        Availability::Available(vec![entry("Update")]),
                    ),
                ],
                services_truncated: false,
                startup_truncated: false,
            },
        };

        let inventory = collect_inventory(&port);

        let Availability::Available(services) = &inventory.services else {
            panic!("services should stay available");
        };
        assert_eq!(
            services
                .iter()
                .map(|service| service.name.as_str())
                .collect::<Vec<_>>(),
            vec!["Appinfo", "Dhcp", "Windows Update"]
        );

        // Current user before all users; registry before folder; names sorted.
        assert_eq!(
            inventory
                .startup
                .iter()
                .map(|group| (group.scope.as_str(), group.source.as_str()))
                .collect::<Vec<_>>(),
            vec![
                ("current_user", "registry_run"),
                ("all_users", "startup_folder")
            ]
        );

        let Availability::Available(entries) = &inventory.startup[1].entries else {
            panic!("folder entries should stay available");
        };
        assert_eq!(
            entries
                .iter()
                .map(|entry| entry.name.as_str())
                .collect::<Vec<_>>(),
            vec!["a-tool.lnk", "z-tool.exe"]
        );
    }

    #[test]
    fn an_unreadable_group_keeps_its_reason() {
        let port = FakePort {
            inventory: StartupInventory {
                services: Availability::Unavailable(UnavailableReason::AccessDenied),
                startup: vec![group(
                    StartupScope::CurrentUser,
                    StartupSource::RegistryRun,
                    Availability::Unavailable(UnavailableReason::Failed),
                )],
                services_truncated: false,
                startup_truncated: false,
            },
        };

        let inventory = collect_inventory(&port);

        assert_eq!(
            inventory.services,
            Availability::Unavailable(UnavailableReason::AccessDenied)
        );
        assert_eq!(
            inventory.startup[0].entries,
            Availability::Unavailable(UnavailableReason::Failed)
        );
    }

    #[test]
    fn a_denied_command_stays_with_its_entry() {
        let mut denied = entry("thing.exe");
        denied.command = Availability::Unavailable(UnavailableReason::AccessDenied);

        let port = FakePort {
            inventory: StartupInventory {
                services: Availability::Available(Vec::new()),
                startup: vec![group(
                    StartupScope::AllUsers,
                    StartupSource::RegistryRunOnce,
                    Availability::Available(vec![denied]),
                )],
                services_truncated: false,
                startup_truncated: false,
            },
        };

        let inventory = collect_inventory(&port);

        let Availability::Available(entries) = &inventory.startup[0].entries else {
            panic!("entries should stay available");
        };
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "thing.exe");
        assert_eq!(
            entries[0].command,
            Availability::Unavailable(UnavailableReason::AccessDenied)
        );
    }

    #[test]
    fn a_truncated_inventory_keeps_its_flag() {
        let port = FakePort {
            inventory: StartupInventory {
                services: Availability::Available(vec![service("Dhcp")]),
                startup: Vec::new(),
                services_truncated: true,
                startup_truncated: true,
            },
        };

        let inventory = collect_inventory(&port);

        assert!(inventory.services_truncated);
        assert!(inventory.startup_truncated);
    }

    #[test]
    fn codes_and_keys_are_stable() {
        assert_eq!(ServiceState::Running.as_str(), "running");
        assert_eq!(ServiceState::StartPending.as_str(), "start_pending");
        assert_eq!(ServiceState::Unknown.as_str(), "unknown");
        assert_eq!(ServiceStartType::Automatic.as_str(), "automatic");
        assert_eq!(ServiceStartType::Disabled.as_str(), "disabled");
        assert_eq!(StartupScope::CurrentUser.as_str(), "current_user");
        assert_eq!(StartupScope::AllUsers.as_str(), "all_users");
        assert_eq!(StartupSource::RegistryRunOnce.as_str(), "registry_run_once");
        assert_eq!(StartupSource::StartupFolder.as_str(), "startup_folder");
    }
}
