//! Win32 implementation of [`StartupInventoryPort`].
//!
//! Rules honoured here:
//! - Read-only: services are opened with `SERVICE_QUERY_CONFIG` only, and every
//!   handle is closed on the way out.
//! - No elevation is requested. Whatever the current user cannot read is reported
//!   as unavailable with a reason, per source group and per service.
//! - Enumeration is capped ([`MAX_SERVICES`], [`MAX_STARTUP_ENTRIES`]) and the
//!   cap is reported.
//! - Every `unsafe` block carries a `SAFETY:` note.

use std::mem::size_of;
use std::path::PathBuf;
use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::{
    ERROR_ACCESS_DENIED, ERROR_INSUFFICIENT_BUFFER, ERROR_MORE_DATA, ERROR_NO_MORE_ITEMS,
    ERROR_SUCCESS, GetLastError,
};
use windows_sys::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, REG_EXPAND_SZ, REG_MULTI_SZ, REG_SZ,
    RegCloseKey, RegEnumValueW, RegOpenKeyExW,
};
use windows_sys::Win32::System::Services::{
    CloseServiceHandle, ENUM_SERVICE_STATUS_PROCESSW, EnumServicesStatusExW, OpenSCManagerW,
    OpenServiceW, QUERY_SERVICE_CONFIGW, QueryServiceConfigW, SC_ENUM_PROCESS_INFO, SC_HANDLE,
    SC_MANAGER_CONNECT, SC_MANAGER_ENUMERATE_SERVICE, SERVICE_AUTO_START, SERVICE_BOOT_START,
    SERVICE_CONTINUE_PENDING, SERVICE_DEMAND_START, SERVICE_DISABLED, SERVICE_PAUSE_PENDING,
    SERVICE_PAUSED, SERVICE_QUERY_CONFIG, SERVICE_RUNNING, SERVICE_START_PENDING,
    SERVICE_STATE_ALL, SERVICE_STOP_PENDING, SERVICE_STOPPED, SERVICE_SYSTEM_START, SERVICE_WIN32,
};

use crate::domain::startup::{
    MAX_SERVICES, MAX_STARTUP_ENTRIES, ServiceInfo, ServiceStartType, ServiceState, StartupEntry,
    StartupInventory, StartupScope, StartupSource, StartupSourceGroup,
};
use crate::domain::system::{Availability, UnavailableReason};
use crate::ports::ProbeError;
use crate::ports::startup::StartupInventoryPort;

/// Initial buffer for `EnumServicesStatusExW`; growth follows `pcbBytesNeeded`.
const INITIAL_SERVICES_BYTES: usize = 64 * 1024;
/// Refuse to allocate more than this for one service enumeration.
const MAX_SERVICES_BYTES: usize = 64 * 1024 * 1024;
/// Initial buffer for one `QueryServiceConfigW` call.
const INITIAL_CONFIG_BYTES: usize = 4 * 1024;
/// Cap on any single string Win32 hands back through a pointer.
const MAX_TEXT_UNITS: usize = 32_768;
/// Longest registry value name Windows allows, plus room for the terminator.
const MAX_VALUE_NAME_UNITS: usize = 16_384;
/// Room for one registry value's data. A Run value is a command line; 64 KiB is
/// far above any real one.
const MAX_VALUE_BYTES: usize = 64 * 1024;

const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const RUN_ONCE_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\RunOnce";
const STARTUP_FOLDER: &str = "Microsoft\\Windows\\Start Menu\\Programs\\Startup";

/// Read-only startup and service probe. Stateless, so construction is free.
#[derive(Debug, Default, Clone, Copy)]
pub struct WindowsStartup;

impl StartupInventoryPort for WindowsStartup {
    fn inventory(&self) -> StartupInventory {
        let (services, services_truncated) = match enumerate_services() {
            Ok((list, truncated)) => (Availability::Available(list), truncated),
            Err(error) => (Availability::Unavailable(error.into()), false),
        };

        let mut budget = MAX_STARTUP_ENTRIES;
        let mut startup_truncated = false;
        let mut startup = Vec::with_capacity(6);

        let registry_sources = [
            (
                StartupScope::CurrentUser,
                HKEY_CURRENT_USER,
                RUN_KEY,
                StartupSource::RegistryRun,
            ),
            (
                StartupScope::CurrentUser,
                HKEY_CURRENT_USER,
                RUN_ONCE_KEY,
                StartupSource::RegistryRunOnce,
            ),
            (
                StartupScope::AllUsers,
                HKEY_LOCAL_MACHINE,
                RUN_KEY,
                StartupSource::RegistryRun,
            ),
            (
                StartupScope::AllUsers,
                HKEY_LOCAL_MACHINE,
                RUN_ONCE_KEY,
                StartupSource::RegistryRunOnce,
            ),
        ];

        for (scope, root, subkey, source) in registry_sources {
            let entries = registry_values(root, subkey, &mut budget, &mut startup_truncated);
            startup.push(StartupSourceGroup {
                scope,
                source,
                entries,
            });
        }

        for (scope, variable) in [
            (StartupScope::CurrentUser, "APPDATA"),
            (StartupScope::AllUsers, "ProgramData"),
        ] {
            let entries = folder_entries(variable, &mut budget, &mut startup_truncated);
            startup.push(StartupSourceGroup {
                scope,
                source: StartupSource::StartupFolder,
                entries,
            });
        }

        StartupInventory {
            services,
            startup,
            services_truncated,
            startup_truncated,
        }
    }
}

// --- Services ---------------------------------------------------------------

fn enumerate_services() -> Result<(Vec<ServiceInfo>, bool), ProbeError> {
    // SAFETY: both string arguments are null, meaning "local machine" and "active
    // database", as documented; the result is a live handle or null.
    let manager = ScHandle::from(unsafe {
        OpenSCManagerW(
            null(),
            null(),
            SC_MANAGER_CONNECT | SC_MANAGER_ENUMERATE_SERVICE,
        )
    })?;

    let mut words: Vec<u64> = vec![0; INITIAL_SERVICES_BYTES / size_of::<u64>()];
    let mut services = Vec::new();
    let mut truncated = false;
    let mut resume = 0u32;

    loop {
        let mut needed = 0u32;
        let mut returned = 0u32;

        // SAFETY: `words` is live for its full byte length and 8-byte aligned,
        // which satisfies ENUM_SERVICE_STATUS_PROCESSW in this array; the three
        // out-parameters and the resume handle are live; the group name is null.
        let ok = unsafe {
            EnumServicesStatusExW(
                manager.0,
                SC_ENUM_PROCESS_INFO,
                SERVICE_WIN32,
                SERVICE_STATE_ALL,
                words.as_mut_ptr().cast::<u8>(),
                (words.len() * size_of::<u64>()) as u32,
                &mut needed,
                &mut returned,
                &mut resume,
                null(),
            )
        };

        if ok == 0 {
            let code = last_error_code();
            if code == ERROR_MORE_DATA {
                grow(&mut words, needed, MAX_SERVICES_BYTES)?;
                continue;
            }
            return Err(error_from(code));
        }

        // SAFETY: on success the SCM filled the buffer with `returned` contiguous
        // ENUM_SERVICE_STATUS_PROCESSW records; the slice does not outlive it.
        let batch = unsafe {
            std::slice::from_raw_parts(
                words.as_ptr().cast::<ENUM_SERVICE_STATUS_PROCESSW>(),
                returned as usize,
            )
        };

        for entry in batch {
            if services.len() >= MAX_SERVICES {
                truncated = true;
                break;
            }
            services.push(describe_service(manager.0, entry));
        }

        if truncated || resume == 0 {
            break;
        }
    }

    Ok((services, truncated))
}

fn describe_service(manager: SC_HANDLE, entry: &ENUM_SERVICE_STATUS_PROCESSW) -> ServiceInfo {
    let name = wide_string(entry.lpServiceName);
    let display_name = wide_string(entry.lpDisplayName);
    let (start_type, binary_path) = match &name {
        Availability::Available(name) => query_config(manager, name),
        // Without a name the service cannot be opened at all, and a fabricated
        // identity would be worse than an explicit gap.
        Availability::Unavailable(reason) => (
            Availability::Unavailable(*reason),
            Availability::Unavailable(*reason),
        ),
    };

    ServiceInfo {
        name: match name {
            Availability::Available(name) => name,
            Availability::Unavailable(_) => "<unnamed>".to_owned(),
        },
        display_name,
        state: state_from(entry.ServiceStatusProcess.dwCurrentState),
        start_type,
        binary_path,
    }
}

/// Reads start type and binary path from one service handle.
///
/// `OpenServiceW` with `SERVICE_QUERY_CONFIG` is the narrowest right that answers
/// both questions, and Windows denies it for some services to a standard user.
fn query_config(
    manager: SC_HANDLE,
    name: &str,
) -> (Availability<ServiceStartType>, Availability<String>) {
    let path = wide(name);
    // SAFETY: `path` is null-terminated and lives for the call; the result is a
    // live handle or null.
    let service =
        ScHandle::from(unsafe { OpenServiceW(manager, path.as_ptr(), SERVICE_QUERY_CONFIG) });
    let service = match service {
        Ok(service) => service,
        Err(error) => {
            let reason = UnavailableReason::from(error);
            return (
                Availability::Unavailable(reason),
                Availability::Unavailable(reason),
            );
        }
    };

    let mut words: Vec<u64> = vec![0; INITIAL_CONFIG_BYTES / size_of::<u64>()];
    let mut needed = 0u32;

    loop {
        // SAFETY: the buffer is live for its full byte length and 8-byte aligned,
        // which covers QUERY_SERVICE_CONFIGW; `needed` is a live out-parameter.
        let ok = unsafe {
            QueryServiceConfigW(
                service.0,
                words.as_mut_ptr().cast::<QUERY_SERVICE_CONFIGW>(),
                (words.len() * size_of::<u64>()) as u32,
                &mut needed,
            )
        };

        if ok != 0 {
            break;
        }

        let code = last_error_code();
        if code == ERROR_INSUFFICIENT_BUFFER {
            if grow(&mut words, needed, MAX_SERVICES_BYTES).is_err() {
                let reason = UnavailableReason::Failed;
                return (
                    Availability::Unavailable(reason),
                    Availability::Unavailable(reason),
                );
            }
            continue;
        }

        let reason = reason_from(code);
        return (
            Availability::Unavailable(reason),
            Availability::Unavailable(reason),
        );
    }

    // SAFETY: a successful call wrote one QUERY_SERVICE_CONFIGW (and its strings)
    // into the buffer, which is still alive here.
    let config = unsafe { &*words.as_ptr().cast::<QUERY_SERVICE_CONFIGW>() };

    (
        Availability::Available(start_type_from(config.dwStartType)),
        binary_path(config.lpBinaryPathName),
    )
}

fn state_from(state: u32) -> ServiceState {
    match state {
        SERVICE_RUNNING => ServiceState::Running,
        SERVICE_STOPPED => ServiceState::Stopped,
        SERVICE_START_PENDING => ServiceState::StartPending,
        SERVICE_STOP_PENDING => ServiceState::StopPending,
        SERVICE_CONTINUE_PENDING => ServiceState::ContinuePending,
        SERVICE_PAUSE_PENDING => ServiceState::PausePending,
        SERVICE_PAUSED => ServiceState::Paused,
        _ => ServiceState::Unknown,
    }
}

fn start_type_from(start_type: u32) -> ServiceStartType {
    match start_type {
        SERVICE_BOOT_START => ServiceStartType::Boot,
        SERVICE_SYSTEM_START => ServiceStartType::System,
        SERVICE_AUTO_START => ServiceStartType::Automatic,
        SERVICE_DEMAND_START => ServiceStartType::Manual,
        SERVICE_DISABLED => ServiceStartType::Disabled,
        _ => ServiceStartType::Unknown,
    }
}

fn binary_path(pointer: *const u16) -> Availability<String> {
    match wide_string(pointer) {
        // An empty configured path is not a path; say so instead of rendering a
        // blank cell that looks like a successful read.
        Availability::Available(value) if value.is_empty() => {
            Availability::Unavailable(UnavailableReason::Failed)
        }
        other => other,
    }
}

// --- Startup entries --------------------------------------------------------

fn registry_values(
    root: HKEY,
    subkey: &str,
    budget: &mut usize,
    truncated: &mut bool,
) -> Availability<Vec<StartupEntry>> {
    let path = wide(subkey);
    let mut key: HKEY = null_mut();

    // SAFETY: `path` is null-terminated and lives for the call; `key` is a live
    // out-parameter that receives a handle or is left untouched.
    let status = unsafe { RegOpenKeyExW(root, path.as_ptr(), 0, KEY_READ, &mut key) };
    if status != ERROR_SUCCESS {
        return Availability::Unavailable(reason_from(status));
    }
    let key = RegistryKey(key);

    let mut entries = Vec::new();
    let mut index = 0u32;
    let mut name = vec![0u16; MAX_VALUE_NAME_UNITS];
    let mut data = vec![0u8; MAX_VALUE_BYTES];

    loop {
        if *budget == 0 {
            *truncated = true;
            break;
        }

        let mut name_len = name.len() as u32;
        let mut value_type = 0u32;
        let mut data_len = data.len() as u32;

        // SAFETY: both buffers are live for their stated lengths, the lengths are
        // their own sizes in units/bytes, and the reserved pointer is null.
        let status = unsafe {
            RegEnumValueW(
                key.0,
                index,
                name.as_mut_ptr(),
                &mut name_len,
                null_mut(),
                &mut value_type,
                data.as_mut_ptr(),
                &mut data_len,
            )
        };

        match status {
            ERROR_SUCCESS => {}
            ERROR_NO_MORE_ITEMS => break,
            ERROR_MORE_DATA => {
                // The name (or data) did not fit. The value exists, so report it
                // with an explicit gap rather than skipping it silently.
                let name = decode_name(&name[..name_len as usize]);
                if !name.is_empty() {
                    entries.push(StartupEntry {
                        name,
                        command: Availability::Unavailable(UnavailableReason::Failed),
                    });
                    *budget = budget.saturating_sub(1);
                }
                index += 1;
                continue;
            }
            other => return Availability::Unavailable(reason_from(other)),
        }

        let name = decode_name(&name[..name_len as usize]);
        if !name.is_empty() {
            entries.push(StartupEntry {
                name,
                command: command_from(value_type, &data[..data_len as usize]),
            });
            *budget = budget.saturating_sub(1);
        }

        index += 1;
    }

    Availability::Available(entries)
}

/// Little-endian UTF-16 bytes to units; a trailing lone byte is ignored.
fn utf16_units(data: &[u8]) -> Vec<u16> {
    data.chunks(2)
        .filter(|pair| pair.len() == 2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect()
}

fn decode_name(units: &[u16]) -> String {
    let end = units
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(units.len());
    String::from_utf16_lossy(&units[..end])
}

/// Interprets one registry value as an autostart command.
///
/// `REG_MULTI_SZ` values are joined with `; ` because the wire shape is one
/// string; a non-string type is reported `unsupported` rather than stringified.
fn command_from(value_type: u32, data: &[u8]) -> Availability<String> {
    match value_type {
        REG_SZ | REG_EXPAND_SZ => {
            let units = utf16_units(data);
            Availability::Available(decode_name(&units))
        }
        REG_MULTI_SZ => {
            let units = utf16_units(data);
            let strings: Vec<String> = units
                .split(|unit| *unit == 0)
                .filter(|part| !part.is_empty())
                .map(String::from_utf16_lossy)
                .collect();
            Availability::Available(strings.join("; "))
        }
        _ => Availability::Unavailable(UnavailableReason::Unsupported),
    }
}

fn folder_entries(
    variable: &str,
    budget: &mut usize,
    truncated: &mut bool,
) -> Availability<Vec<StartupEntry>> {
    let Some(base) = std::env::var_os(variable) else {
        return Availability::Unavailable(UnavailableReason::Unsupported);
    };
    let path = PathBuf::from(base).join(STARTUP_FOLDER);

    let directory = match std::fs::read_dir(&path) {
        Ok(directory) => directory,
        // A user with no Startup folder is not a limitation.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Availability::Available(Vec::new());
        }
        Err(error) => return Availability::Unavailable(reason_from_io(&error)),
    };

    let mut entries = Vec::new();
    let mut failure: Option<UnavailableReason> = None;

    for item in directory {
        if *budget == 0 {
            *truncated = true;
            break;
        }

        let item = match item {
            Ok(item) => item,
            Err(error) => {
                failure.get_or_insert_with(|| reason_from_io(&error));
                continue;
            }
        };

        // Folder metadata is not an autostart entry.
        if item.file_name().eq_ignore_ascii_case("desktop.ini") {
            continue;
        }

        match item.file_type() {
            Ok(kind) if kind.is_file() => {}
            Ok(_) => continue,
            Err(error) => {
                failure.get_or_insert_with(|| reason_from_io(&error));
                continue;
            }
        }

        entries.push(StartupEntry {
            name: item.file_name().to_string_lossy().into_owned(),
            command: Availability::Available(item.path().to_string_lossy().into_owned()),
        });
        *budget = budget.saturating_sub(1);
    }

    if entries.is_empty()
        && let Some(reason) = failure
    {
        return Availability::Unavailable(reason);
    }

    Availability::Available(entries)
}

// --- Shared helpers ---------------------------------------------------------

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Copies a null-terminated UTF-16 string from Win32 into a Rust `String`.
fn wide_string(pointer: *const u16) -> Availability<String> {
    if pointer.is_null() {
        return Availability::Unavailable(UnavailableReason::Failed);
    }

    let mut len = 0usize;
    // SAFETY: a Win32 string is null-terminated. The walk stops at the terminator,
    // and at MAX_TEXT_UNITS so a missing terminator cannot read forever.
    while len < MAX_TEXT_UNITS {
        if unsafe { *pointer.add(len) } == 0 {
            // SAFETY: the same pointer, with `len` initialised units before it.
            let units = unsafe { std::slice::from_raw_parts(pointer, len) };
            return Availability::Available(String::from_utf16_lossy(units));
        }
        len += 1;
    }

    Availability::Unavailable(UnavailableReason::Failed)
}

/// Grows a byte buffer to the size the API asked for.
fn grow(words: &mut Vec<u64>, needed: u32, max_bytes: usize) -> Result<(), ProbeError> {
    let bytes = usize::try_from(needed).map_err(|_| ProbeError::Failed)?;
    if bytes == 0 || bytes > max_bytes {
        return Err(ProbeError::Failed);
    }

    let units = bytes.div_ceil(size_of::<u64>()).max(words.len() + 1);
    words.resize(units, 0);
    Ok(())
}

fn last_error_code() -> u32 {
    // SAFETY: GetLastError has no preconditions; the value is read on the same
    // thread that made the failing call.
    unsafe { GetLastError() }
}

fn error_from(code: u32) -> ProbeError {
    if code == ERROR_ACCESS_DENIED {
        ProbeError::AccessDenied
    } else {
        ProbeError::Failed
    }
}

fn reason_from(code: u32) -> UnavailableReason {
    UnavailableReason::from(error_from(code))
}

fn reason_from_io(error: &std::io::Error) -> UnavailableReason {
    match error.kind() {
        std::io::ErrorKind::PermissionDenied => UnavailableReason::AccessDenied,
        _ => UnavailableReason::Failed,
    }
}

/// Owns an SCM handle so it is closed on every return path.
struct ScHandle(SC_HANDLE);

impl ScHandle {
    fn from(handle: SC_HANDLE) -> Result<Self, ProbeError> {
        if handle.is_null() {
            Err(error_from(last_error_code()))
        } else {
            Ok(Self(handle))
        }
    }
}

impl Drop for ScHandle {
    fn drop(&mut self) {
        // SAFETY: the handle came from the SCM and is closed exactly once.
        unsafe { CloseServiceHandle(self.0) };
    }
}

/// Owns a registry key so it is closed on every return path.
struct RegistryKey(HKEY);

impl Drop for RegistryKey {
    fn drop(&mut self) {
        // SAFETY: the key came from RegOpenKeyExW and is closed exactly once.
        unsafe { RegCloseKey(self.0) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_single_string_value_is_decoded() {
        let data: Vec<u8> = "C:\\bin\\app.exe -start"
            .encode_utf16()
            .chain([0u16])
            .flat_map(u16::to_le_bytes)
            .collect();

        assert_eq!(
            command_from(REG_SZ, &data),
            Availability::Available("C:\\bin\\app.exe -start".to_owned())
        );
    }

    #[test]
    fn a_multi_string_value_is_joined_and_a_non_string_is_unsupported() {
        let data: Vec<u8> = "one.exe\0two.exe\0\0"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();

        assert_eq!(
            command_from(REG_MULTI_SZ, &data),
            Availability::Available("one.exe; two.exe".to_owned())
        );
        assert_eq!(
            command_from(4, &data),
            Availability::Unavailable(UnavailableReason::Unsupported)
        );
    }

    #[test]
    fn an_unterminated_string_stops_at_the_cap() {
        let units = vec![b'x' as u16; MAX_TEXT_UNITS];

        assert_eq!(
            wide_string(units.as_ptr()),
            Availability::Unavailable(UnavailableReason::Failed)
        );
    }

    #[test]
    fn every_state_and_start_type_maps_explicitly() {
        assert_eq!(state_from(SERVICE_RUNNING), ServiceState::Running);
        assert_eq!(state_from(SERVICE_STOPPED), ServiceState::Stopped);
        assert_eq!(state_from(SERVICE_PAUSED), ServiceState::Paused);
        assert_eq!(state_from(0xdead), ServiceState::Unknown);
        assert_eq!(
            start_type_from(SERVICE_AUTO_START),
            ServiceStartType::Automatic
        );
        assert_eq!(
            start_type_from(SERVICE_DISABLED),
            ServiceStartType::Disabled
        );
        assert_eq!(start_type_from(0xdead), ServiceStartType::Unknown);
    }

    // Machine smoke test: structural invariants only, no machine-specific names.
    #[test]
    fn native_inventory_has_structural_invariants() {
        let inventory = WindowsStartup.inventory();

        // Six sources are always reported, in a fixed set of keys.
        assert_eq!(inventory.startup.len(), 6);
        let keys: Vec<String> = inventory
            .startup
            .iter()
            .map(|group| format!("{}.{}", group.scope.as_str(), group.source.as_str()))
            .collect();
        let unique: std::collections::HashSet<&String> = keys.iter().collect();
        assert_eq!(unique.len(), keys.len(), "source keys must be unique");

        for group in &inventory.startup {
            if let Availability::Available(entries) = &group.entries {
                assert!(entries.len() <= MAX_STARTUP_ENTRIES);
                for entry in entries {
                    assert!(!entry.name.is_empty());
                    if let Availability::Available(command) = &entry.command {
                        assert!(!command.contains('\u{0}'));
                    }
                }
            }
        }

        match &inventory.services {
            Availability::Available(services) => {
                assert!(!services.is_empty(), "a live Windows machine has services");
                assert!(services.len() <= MAX_SERVICES);
                let mut names: Vec<&str> = services
                    .iter()
                    .map(|service| service.name.as_str())
                    .collect();
                names.sort_unstable();
                names.dedup();
                assert_eq!(names.len(), services.len(), "service names must be unique");
            }
            // Denial is a legitimate result and must carry a reason; it just means
            // this machine's SCM refused enumeration to this account.
            Availability::Unavailable(_) => {}
        }
    }
}
