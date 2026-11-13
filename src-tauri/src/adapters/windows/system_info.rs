//! Win32 implementation of [`SystemInfoPort`].
//!
//! Rules honoured here:
//! - Every call is read-only and needs no elevation.
//! - Network drives are not queried for size/label: those calls can block for
//!   tens of seconds on a dead mapping, so they are reported as `unsupported`.
//! - Failures become explicit [`ProbeError`] or [`Availability::Unavailable`];
//!   nothing is fabricated and nothing panics.
//! - Every `unsafe` block carries a `SAFETY:` note.

use std::ffi::c_void;
use std::net::Ipv6Addr;
use std::ptr;

use windows_sys::Win32::Foundation::{
    ERROR_ACCESS_DENIED, ERROR_BUFFER_OVERFLOW, ERROR_INSUFFICIENT_BUFFER, ERROR_NO_DATA,
    ERROR_SUCCESS, GetLastError,
};
use windows_sys::Win32::NetworkManagement::IpHelper::{
    GAA_FLAG_SKIP_ANYCAST, GAA_FLAG_SKIP_DNS_SERVER, GAA_FLAG_SKIP_MULTICAST, GetAdaptersAddresses,
    IP_ADAPTER_ADDRESSES_LH,
};
use windows_sys::Win32::NetworkManagement::Ndis::IfOperStatusUp;
use windows_sys::Win32::Networking::WinSock::{
    AF_INET, AF_INET6, AF_UNSPEC, SOCKADDR, SOCKADDR_IN, SOCKADDR_IN6,
};
use windows_sys::Win32::Storage::FileSystem::{
    GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDriveStringsW, GetVolumeInformationW,
};
use windows_sys::Win32::System::Registry::{
    HKEY_LOCAL_MACHINE, RRF_RT_REG_DWORD, RRF_RT_REG_SZ, RegGetValueW,
};
use windows_sys::Win32::System::SystemInformation::{
    GetLogicalProcessorInformationEx, GetNativeSystemInfo, GlobalMemoryStatusEx, MEMORYSTATUSEX,
    PROCESSOR_ARCHITECTURE_AMD64, PROCESSOR_ARCHITECTURE_ARM, PROCESSOR_ARCHITECTURE_ARM64,
    PROCESSOR_ARCHITECTURE_IA64, PROCESSOR_ARCHITECTURE_INTEL, RelationProcessorCore, SYSTEM_INFO,
    SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX,
};
use windows_sys::Win32::System::Threading::{ALL_PROCESSOR_GROUPS, GetActiveProcessorCount};
use windows_sys::Win32::System::WindowsProgramming::{
    DRIVE_CDROM, DRIVE_FIXED, DRIVE_RAMDISK, DRIVE_REMOTE, DRIVE_REMOVABLE,
};

use crate::domain::system::{
    Availability, CpuInfo, MemoryInfo, NetworkAdapterInfo, OsInfo, UnavailableReason, VolumeInfo,
    VolumeKind,
};
use crate::ports::ProbeError;
use crate::ports::system_info::SystemInfoPort;

const CURRENT_VERSION_KEY: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion";
const CPU_KEY: &str = r"HARDWARE\DESCRIPTION\System\CentralProcessor\0";
const MAX_STRING_CHARS: usize = 64 * 1024;
const MAX_ADAPTERS: usize = 256;
const MAX_ADDRESSES_PER_ADAPTER: usize = 64;

/// Read-only Win32 system probe. Stateless, so construction is free.
#[derive(Debug, Default, Clone, Copy)]
pub struct WindowsSystemInfo;

impl SystemInfoPort for WindowsSystemInfo {
    fn os_info(&self) -> Result<OsInfo, ProbeError> {
        let name = registry_string(CURRENT_VERSION_KEY, "ProductName");
        let edition = registry_string(CURRENT_VERSION_KEY, "EditionID");
        let build = registry_string(CURRENT_VERSION_KEY, "CurrentBuildNumber");
        let version = os_version();

        if name.is_err() && edition.is_err() && build.is_err() && version.is_err() {
            return Err(ProbeError::Failed);
        }

        Ok(OsInfo {
            name: availability(name),
            edition: availability(edition),
            version: availability(version),
            build: availability(build),
            arch: native_arch(),
        })
    }

    fn cpu_info(&self) -> Result<CpuInfo, ProbeError> {
        let model = registry_string(CPU_KEY, "ProcessorNameString");
        let logical_cores = match logical_core_count() {
            0 => Availability::Unavailable(UnavailableReason::Failed),
            count => Availability::Available(count),
        };
        let physical_cores = match physical_core_count() {
            Some(count) => Availability::Available(count),
            None => Availability::Unavailable(UnavailableReason::Failed),
        };

        Ok(CpuInfo {
            model: availability(model),
            logical_cores,
            physical_cores,
        })
    }

    fn memory_info(&self) -> Result<MemoryInfo, ProbeError> {
        let mut status = MEMORYSTATUSEX {
            dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
            ..unsafe_zeroed_memory_status()
        };

        // SAFETY: `status` is a fully initialised MEMORYSTATUSEX with `dwLength`
        // set to its own size, which is the documented contract.
        let ok = unsafe { GlobalMemoryStatusEx(&mut status) };
        if ok == 0 {
            return Err(last_error());
        }

        Ok(MemoryInfo {
            total_bytes: status.ullTotalPhys,
            available_bytes: status.ullAvailPhys,
        })
    }

    fn volumes(&self) -> Result<Vec<VolumeInfo>, ProbeError> {
        let needed = unsafe { GetLogicalDriveStringsW(0, ptr::null_mut()) };
        if needed == 0 || needed as usize > MAX_STRING_CHARS {
            return Err(ProbeError::Failed);
        }

        let mut buffer = vec![0u16; needed as usize + 1];
        // SAFETY: the buffer holds `needed + 1` UTF-16 units, which is the size
        // the previous call reported, plus the terminator.
        let written = unsafe { GetLogicalDriveStringsW(buffer.len() as u32, buffer.as_mut_ptr()) };
        if written == 0 {
            return Err(last_error());
        }

        let mut volumes = Vec::new();
        for mount in buffer
            .split(|unit| *unit == 0)
            .filter(|part| !part.is_empty())
        {
            volumes.push(describe_volume(&String::from_utf16_lossy(mount)));
        }
        Ok(volumes)
    }

    fn network_adapters(&self) -> Result<Vec<NetworkAdapterInfo>, ProbeError> {
        let flags = GAA_FLAG_SKIP_ANYCAST | GAA_FLAG_SKIP_MULTICAST | GAA_FLAG_SKIP_DNS_SERVER;
        let mut size: u32 = 0;

        // SAFETY: a null buffer with a zero size is the documented sizing call.
        let mut code = unsafe {
            GetAdaptersAddresses(
                u32::from(AF_UNSPEC),
                flags,
                ptr::null(),
                ptr::null_mut(),
                &mut size,
            )
        };

        if code == ERROR_NO_DATA {
            return Ok(Vec::new());
        }
        if code != ERROR_BUFFER_OVERFLOW || size == 0 {
            return Err(map_win32_error(code));
        }

        // `IP_ADAPTER_ADDRESSES_LH` requires pointer alignment; `Vec<u64>` gives
        // it while keeping the raw byte length in `size`.
        let mut buffer: Vec<u64> = vec![0; (size as usize).div_ceil(8) + 1];
        // SAFETY: the allocation is at least `size` bytes and pointer-aligned.
        code = unsafe {
            GetAdaptersAddresses(
                u32::from(AF_UNSPEC),
                flags,
                ptr::null(),
                buffer.as_mut_ptr().cast::<IP_ADAPTER_ADDRESSES_LH>(),
                &mut size,
            )
        };

        if code == ERROR_NO_DATA {
            return Ok(Vec::new());
        }
        if code != ERROR_SUCCESS {
            return Err(map_win32_error(code));
        }

        let mut adapters = Vec::new();
        let mut current = buffer.as_ptr().cast::<IP_ADAPTER_ADDRESSES_LH>();

        // The OS owns the linked list inside `buffer`; both the pointer walk and
        // the per-adapter caps are bounded against a malformed or looping list.
        while !current.is_null() && adapters.len() < MAX_ADAPTERS {
            // SAFETY: `current` comes from the buffer filled by the OS and is
            // only dereferenced while the buffer is alive.
            let adapter = unsafe { &*current };

            // SAFETY: the adapter record and its string pointers come from the
            // buffer filled by GetAdaptersAddresses and stay alive here.
            if let Some(name) = unsafe { wide_ptr_to_string(adapter.FriendlyName) } {
                // SAFETY: same record and lifetime as above.
                let description = match unsafe { wide_ptr_to_string(adapter.Description) } {
                    Some(text) => Availability::Available(text),
                    None => Availability::Unavailable(UnavailableReason::Failed),
                };

                adapters.push(NetworkAdapterInfo {
                    name,
                    description,
                    is_up: adapter.OperStatus == IfOperStatusUp,
                    addresses: unicast_addresses(adapter.FirstUnicastAddress),
                });
            }

            current = adapter.Next;
        }

        Ok(adapters)
    }
}

/// `MEMORYSTATUSEX` has no `Default`; zeroing is the documented initialisation.
fn unsafe_zeroed_memory_status() -> MEMORYSTATUSEX {
    // SAFETY: MEMORYSTATUSEX is a plain-old-data struct of integers.
    unsafe { std::mem::zeroed() }
}

fn availability<T>(result: Result<T, ProbeError>) -> Availability<T> {
    match result {
        Ok(value) => Availability::Available(value),
        Err(_) => Availability::Unavailable(UnavailableReason::Failed),
    }
}

fn last_error() -> ProbeError {
    // SAFETY: GetLastError has no preconditions.
    map_win32_error(unsafe { GetLastError() })
}

fn map_win32_error(code: u32) -> ProbeError {
    if code == ERROR_ACCESS_DENIED {
        ProbeError::AccessDenied
    } else {
        ProbeError::Failed
    }
}

fn os_version() -> Result<String, ProbeError> {
    let major = registry_dword(CURRENT_VERSION_KEY, "CurrentMajorVersionNumber")?;
    let minor = registry_dword(CURRENT_VERSION_KEY, "CurrentMinorVersionNumber")?;
    let build = registry_string(CURRENT_VERSION_KEY, "CurrentBuildNumber")?;
    Ok(format!("{major}.{minor}.{build}"))
}

fn native_arch() -> String {
    let mut info = SYSTEM_INFO {
        ..unsafe_zeroed_system_info()
    };
    // SAFETY: `info` is a zeroed SYSTEM_INFO written by the OS.
    unsafe { GetNativeSystemInfo(&mut info) };

    // SAFETY: the union is valid after GetNativeSystemInfo; this arm reads the
    // documented architecture word.
    let arch = unsafe { info.Anonymous.Anonymous.wProcessorArchitecture };
    match arch {
        PROCESSOR_ARCHITECTURE_AMD64 => "x64",
        PROCESSOR_ARCHITECTURE_ARM64 => "arm64",
        PROCESSOR_ARCHITECTURE_INTEL => "x86",
        PROCESSOR_ARCHITECTURE_ARM => "arm",
        PROCESSOR_ARCHITECTURE_IA64 => "ia64",
        _ => "unknown",
    }
    .to_owned()
}

fn unsafe_zeroed_system_info() -> SYSTEM_INFO {
    // SAFETY: SYSTEM_INFO is a plain-old-data struct.
    unsafe { std::mem::zeroed() }
}

fn logical_core_count() -> u32 {
    // SAFETY: ALL_PROCESSOR_GROUPS is a documented sentinel value.
    let count = unsafe { GetActiveProcessorCount(ALL_PROCESSOR_GROUPS) };
    if count > 0 {
        return count;
    }

    let mut info = SYSTEM_INFO {
        ..unsafe_zeroed_system_info()
    };
    // SAFETY: `info` is zeroed and written by the OS.
    unsafe { GetNativeSystemInfo(&mut info) };
    info.dwNumberOfProcessors
}

fn physical_core_count() -> Option<u32> {
    let mut size: u32 = 0;
    // SAFETY: a null buffer with a zero size is the documented sizing call.
    let sized = unsafe {
        GetLogicalProcessorInformationEx(
            RelationProcessorCore,
            ptr::null_mut::<SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX>(),
            &mut size,
        )
    };

    if sized == 0 {
        // SAFETY: GetLastError has no preconditions.
        let error = unsafe { GetLastError() };
        if error != ERROR_INSUFFICIENT_BUFFER {
            return None;
        }
    }
    if size == 0 {
        return None;
    }

    let mut buffer: Vec<u64> = vec![0; (size as usize).div_ceil(8) + 1];
    // SAFETY: the allocation is at least `size` bytes and pointer-aligned.
    let ok = unsafe {
        GetLogicalProcessorInformationEx(
            RelationProcessorCore,
            buffer
                .as_mut_ptr()
                .cast::<SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX>(),
            &mut size,
        )
    };
    if ok == 0 {
        return None;
    }

    // Each record starts with `Relationship` (i32) and `Size` (u32). Only those
    // two words are read, so a short record can never be over-read.
    let base = buffer.as_ptr().cast::<u8>();
    let mut offset = 0usize;
    let mut count = 0u32;
    while offset + 8 <= size as usize {
        // SAFETY: `offset + 8 <= size` and the buffer is at least `size` bytes.
        let relationship = unsafe { ptr::read_unaligned(base.add(offset).cast::<i32>()) };
        // SAFETY: same bounds argument as above, second 4-byte word.
        let record_size = unsafe { ptr::read_unaligned(base.add(offset + 4).cast::<u32>()) };

        if record_size < 8 {
            break;
        }
        if relationship == RelationProcessorCore {
            count += 1;
        }

        let next = offset + record_size as usize;
        if next <= offset || next > size as usize {
            break;
        }
        offset = next;
    }

    Some(count)
}

fn registry_string(subkey: &str, value: &str) -> Result<String, ProbeError> {
    let subkey = to_wide(subkey);
    let value = to_wide(value);
    let mut size: u32 = 0;

    // SAFETY: the first call only requests the required byte length.
    let code = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            subkey.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_SZ,
            ptr::null_mut(),
            ptr::null_mut(),
            &mut size,
        )
    };
    if code != ERROR_SUCCESS || size == 0 || size as usize > MAX_STRING_CHARS * 2 {
        return Err(map_win32_error(code));
    }

    let mut buffer = vec![0u16; (size as usize).div_ceil(2) + 1];
    let mut written = (buffer.len() * 2) as u32;
    // SAFETY: the buffer is sized from the reported byte length and aligned.
    let code = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            subkey.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_SZ,
            ptr::null_mut(),
            buffer.as_mut_ptr().cast::<c_void>(),
            &mut written,
        )
    };
    if code != ERROR_SUCCESS {
        return Err(map_win32_error(code));
    }

    let len = buffer.iter().position(|unit| *unit == 0).unwrap_or(0);
    if len == 0 {
        return Err(ProbeError::Failed);
    }
    String::from_utf16(&buffer[..len]).map_err(|_| ProbeError::Failed)
}

fn registry_dword(subkey: &str, value: &str) -> Result<u32, ProbeError> {
    let subkey = to_wide(subkey);
    let value = to_wide(value);
    let mut data: u32 = 0;
    let mut size: u32 = std::mem::size_of::<u32>() as u32;

    // SAFETY: `data` is a correctly sized u32 and `size` describes it.
    let code = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            subkey.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_DWORD,
            ptr::null_mut(),
            (&mut data as *mut u32).cast::<c_void>(),
            &mut size,
        )
    };
    if code != ERROR_SUCCESS || size != std::mem::size_of::<u32>() as u32 {
        return Err(map_win32_error(code));
    }
    Ok(data)
}

fn describe_volume(mount: &str) -> VolumeInfo {
    let wide = to_wide(mount);
    // SAFETY: `wide` is a NUL-terminated UTF-16 path owned by this function.
    let drive_type = unsafe { GetDriveTypeW(wide.as_ptr()) };
    let kind = match drive_type {
        DRIVE_FIXED => VolumeKind::Fixed,
        DRIVE_REMOVABLE => VolumeKind::Removable,
        DRIVE_REMOTE => VolumeKind::Network,
        DRIVE_CDROM => VolumeKind::Optical,
        DRIVE_RAMDISK => VolumeKind::Ram,
        _ => VolumeKind::Unknown,
    };

    // Querying a dead network mapping can block for seconds; do not do it in a
    // snapshot. Reporting `unsupported` is honest and keeps the call bounded.
    if kind == VolumeKind::Network {
        return VolumeInfo {
            mount: mount.to_owned(),
            kind,
            label: Availability::Unavailable(UnavailableReason::Unsupported),
            filesystem: Availability::Unavailable(UnavailableReason::Unsupported),
            total_bytes: Availability::Unavailable(UnavailableReason::Unsupported),
            free_bytes: Availability::Unavailable(UnavailableReason::Unsupported),
        };
    }

    let (label, filesystem) = volume_identity(&wide);
    let (free_bytes, total_bytes) = volume_space(&wide);

    VolumeInfo {
        mount: mount.to_owned(),
        kind,
        label,
        filesystem,
        total_bytes,
        free_bytes,
    }
}

/// Reads label and filesystem in one call so the two values describe the same
/// volume state.
fn volume_identity(wide: &[u16]) -> (Availability<String>, Availability<String>) {
    let mut label = [0u16; 261];
    let mut filesystem = [0u16; 261];
    // SAFETY: both buffers are 261 UTF-16 units, the documented maximum, and
    // the optional out-pointers are null.
    let ok = unsafe {
        GetVolumeInformationW(
            wide.as_ptr(),
            label.as_mut_ptr(),
            label.len() as u32,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            filesystem.as_mut_ptr(),
            filesystem.len() as u32,
        )
    };
    if ok == 0 {
        return (
            Availability::Unavailable(UnavailableReason::Failed),
            Availability::Unavailable(UnavailableReason::Failed),
        );
    }

    let label_len = label.iter().position(|unit| *unit == 0).unwrap_or(0);
    let filesystem_len = filesystem.iter().position(|unit| *unit == 0).unwrap_or(0);
    (
        // An empty label is a real value (an unlabelled volume), not missing data.
        Availability::Available(String::from_utf16_lossy(&label[..label_len])),
        Availability::Available(String::from_utf16_lossy(&filesystem[..filesystem_len])),
    )
}

/// Reads free and total space in one call. A volume with no media (empty optical
/// drive, removed USB stick) reports `failed` rather than zero.
fn volume_space(wide: &[u16]) -> (Availability<u64>, Availability<u64>) {
    let Some((free_to_caller, total, _)) = disk_space(wide) else {
        return (
            Availability::Unavailable(UnavailableReason::Failed),
            Availability::Unavailable(UnavailableReason::Failed),
        );
    };

    (
        Availability::Available(free_to_caller),
        Availability::Available(total),
    )
}

fn disk_space(wide: &[u16]) -> Option<(u64, u64, u64)> {
    let mut free_to_caller: u64 = 0;
    let mut total: u64 = 0;
    let mut total_free: u64 = 0;

    // SAFETY: all three out-pointers reference live, correctly typed locals.
    let ok = unsafe {
        GetDiskFreeSpaceExW(
            wide.as_ptr(),
            &mut free_to_caller,
            &mut total,
            &mut total_free,
        )
    };
    if ok == 0 {
        None
    } else {
        Some((free_to_caller, total, total_free))
    }
}

fn unicast_addresses(
    mut current: *mut windows_sys::Win32::NetworkManagement::IpHelper::IP_ADAPTER_UNICAST_ADDRESS_LH,
) -> Vec<String> {
    let mut addresses = Vec::new();
    while !current.is_null() && addresses.len() < MAX_ADDRESSES_PER_ADAPTER {
        // SAFETY: `current` belongs to the adapter list filled by the OS.
        let entry = unsafe { &*current };
        // SAFETY: the sockaddr pointer and length come from the same OS record.
        if let Some(address) =
            unsafe { format_sockaddr(entry.Address.lpSockaddr, entry.Address.iSockaddrLength) }
        {
            addresses.push(address);
        }
        current = entry.Next;
    }
    addresses
}

/// Formats an IPv4/IPv6 sockaddr without any connectivity check.
///
/// # Safety
///
/// `address` must be null or point to a readable sockaddr of at least
/// `length` bytes, as guaranteed by `GetAdaptersAddresses`.
unsafe fn format_sockaddr(address: *const SOCKADDR, length: i32) -> Option<String> {
    if address.is_null() || length < 2 {
        return None;
    }

    // SAFETY: the first two bytes of a sockaddr are the address family.
    let family = unsafe { (*address).sa_family };
    if family == AF_INET && (length as usize) >= std::mem::size_of::<SOCKADDR_IN>() {
        // SAFETY: family and length confirm the layout.
        let ipv4 = unsafe { &*(address.cast::<SOCKADDR_IN>()) };
        // SAFETY: `S_addr` is stored in network byte order; the bytes are read in
        // memory order, which is exactly the dotted-quad order.
        let octets = unsafe { ipv4.sin_addr.S_un.S_addr }.to_ne_bytes();
        return Some(format!(
            "{}.{}.{}.{}",
            octets[0], octets[1], octets[2], octets[3]
        ));
    }

    if family == AF_INET6 && (length as usize) >= std::mem::size_of::<SOCKADDR_IN6>() {
        // SAFETY: family and length confirm the layout.
        let ipv6 = unsafe { &*(address.cast::<SOCKADDR_IN6>()) };
        // SAFETY: `Byte` is the raw 16-byte address.
        let bytes = unsafe { ipv6.sin6_addr.u.Byte };
        return Some(Ipv6Addr::from(bytes).to_string());
    }

    None
}

unsafe fn wide_ptr_to_string(pointer: *const u16) -> Option<String> {
    if pointer.is_null() {
        return None;
    }

    let mut len = 0usize;
    // SAFETY: the pointer is a NUL-terminated string owned by the OS; the scan is
    // capped at `MAX_STRING_CHARS` so a missing terminator cannot run away.
    while len < MAX_STRING_CHARS && unsafe { *pointer.add(len) } != 0 {
        len += 1;
    }
    if len == 0 {
        return None;
    }

    // SAFETY: the first `len` units were just validated as readable.
    let units = unsafe { std::slice::from_raw_parts(pointer, len) };
    String::from_utf16(units).ok()
}

fn to_wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Machine smoke test: structural invariants only, no machine-specific paths
    // or version numbers. Runs on the Windows CI job.
    #[test]
    fn native_probe_returns_structural_invariants() {
        let port = WindowsSystemInfo;

        let os = port.os_info().expect("OS info must be collectable");
        assert!(!os.arch.is_empty());
        assert!(matches!(
            os.arch.as_str(),
            "x64" | "arm64" | "x86" | "arm" | "ia64" | "unknown"
        ));

        let cpu = port.cpu_info().expect("CPU info must be collectable");
        assert!(matches!(
            cpu.logical_cores,
            Availability::Available(count) if count > 0
        ));
        assert!(matches!(
            cpu.physical_cores,
            Availability::Available(count) if count > 0
        ));

        let memory = port.memory_info().expect("memory must be collectable");
        assert!(memory.total_bytes > 0);
        assert!(memory.available_bytes <= memory.total_bytes);

        let volumes = port.volumes().expect("volumes must be collectable");
        assert!(!volumes.is_empty());
        for volume in &volumes {
            assert!(volume.mount.ends_with('\\'));
        }

        let adapters = port
            .network_adapters()
            .expect("adapters must be collectable");
        for adapter in &adapters {
            assert!(!adapter.name.is_empty());
            assert!(adapter.addresses.len() <= MAX_ADDRESSES_PER_ADAPTER);
        }
    }
}
