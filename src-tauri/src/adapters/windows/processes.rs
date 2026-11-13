//! Win32 implementation of [`ProcessInfoPort`].
//!
//! Rules honoured here:
//! - Read-only: only query handles are opened, and none outlive the call.
//! - One unreadable process never fails the inventory. Windows denies access to
//!   system-owned processes, so those fields are marked `access_denied` and the
//!   row is still reported with its identifiers.
//! - Enumeration is capped at [`MAX_PROCESSES`] and the cap is reported.
//! - Every `unsafe` block carries a `SAFETY:` note.

use std::mem::size_of;

use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_ACCESS_DENIED, ERROR_NO_MORE_FILES, FILETIME, GetLastError, HANDLE,
    INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
use windows_sys::Win32::System::Threading::{
    GetProcessTimes, OpenProcess, PROCESS_ACCESS_RIGHTS, PROCESS_QUERY_LIMITED_INFORMATION,
    PROCESS_VM_READ,
};

use crate::domain::process::{ProcessInfo, ProcessInventory};
use crate::domain::system::{Availability, UnavailableReason};
use crate::ports::ProbeError;
use crate::ports::processes::ProcessInfoPort;

/// Upper bound on one enumeration. Far above a normal machine (a few hundred
/// processes), low enough that a hostile process table cannot exhaust memory.
const MAX_PROCESSES: usize = 8192;

/// Unix milliseconds between 1601-01-01 and 1970-01-01, in 100 ns ticks.
const FILETIME_EPOCH_OFFSET_MS: u64 = 11_644_473_600_000;

/// Read-only process probe. Stateless, so construction is free.
#[derive(Debug, Default, Clone, Copy)]
pub struct WindowsProcesses;

impl ProcessInfoPort for WindowsProcesses {
    fn inventory(&self) -> Result<ProcessInventory, ProbeError> {
        let snapshot = Snapshot::open()?;
        let mut entry = PROCESSENTRY32W {
            dwSize: size_of::<PROCESSENTRY32W>() as u32,
            ..PROCESSENTRY32W::default()
        };

        // SAFETY: `snapshot.0` is a live Toolhelp handle and `entry` is a live
        // PROCESSENTRY32W whose `dwSize` is set to its own size.
        if unsafe { Process32FirstW(snapshot.0, &mut entry) } == 0 {
            let code = last_error_code();
            return if code == ERROR_NO_MORE_FILES {
                Ok(ProcessInventory {
                    processes: Vec::new(),
                    truncated: false,
                })
            } else {
                Err(error_from(code))
            };
        }

        let mut processes = Vec::new();
        let mut truncated = false;

        loop {
            processes.push(describe(&entry));

            if processes.len() == MAX_PROCESSES {
                // One extra read tells us whether the cap actually hid a row.
                // SAFETY: same live handle and initialised entry as above.
                truncated = unsafe { Process32NextW(snapshot.0, &mut entry) } != 0;
                break;
            }

            // SAFETY: same live handle and initialised entry as above.
            if unsafe { Process32NextW(snapshot.0, &mut entry) } == 0 {
                break;
            }
        }

        Ok(ProcessInventory {
            processes,
            truncated,
        })
    }
}

fn describe(entry: &PROCESSENTRY32W) -> ProcessInfo {
    let pid = entry.th32ProcessID;
    let (started_at_ms, working_set_bytes) = process_metadata(pid);

    ProcessInfo {
        pid,
        parent_pid: entry.th32ParentProcessID,
        name: exe_name(&entry.szExeFile),
        thread_count: Availability::Available(entry.cntThreads),
        started_at_ms,
        working_set_bytes,
    }
}

fn exe_name(buffer: &[u16]) -> Availability<String> {
    let len = buffer
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(buffer.len());
    if len == 0 {
        return Availability::Unavailable(UnavailableReason::Failed);
    }

    match String::from_utf16(&buffer[..len]) {
        Ok(name) => Availability::Available(name),
        Err(_) => Availability::Unavailable(UnavailableReason::Failed),
    }
}

/// Reads start time and working set with as few handles as possible.
///
/// `K32GetProcessMemoryInfo` needs `PROCESS_VM_READ`, which is denied for
/// processes owned by another account. Opening with both rights first keeps the
/// common case to one handle; the fallback still reports the start time and marks
/// only the working set as denied.
fn process_metadata(pid: u32) -> (Availability<u64>, Availability<u64>) {
    if let Some(handle) =
        ProcessHandle::open(pid, PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_VM_READ)
    {
        return (start_time(handle.0), working_set(handle.0));
    }

    if let Some(handle) = ProcessHandle::open(pid, PROCESS_QUERY_LIMITED_INFORMATION) {
        return (
            start_time(handle.0),
            Availability::Unavailable(UnavailableReason::AccessDenied),
        );
    }

    let reason = last_error_reason();
    (
        Availability::Unavailable(reason),
        Availability::Unavailable(reason),
    )
}

fn start_time(handle: HANDLE) -> Availability<u64> {
    let mut creation = FILETIME::default();
    let mut exit = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();

    // SAFETY: four live FILETIME out-parameters and a valid process handle.
    if unsafe { GetProcessTimes(handle, &mut creation, &mut exit, &mut kernel, &mut user) } == 0 {
        return Availability::Unavailable(last_error_reason());
    }

    match filetime_to_unix_ms(&creation) {
        Some(value) => Availability::Available(value),
        None => Availability::Unavailable(UnavailableReason::Failed),
    }
}

fn working_set(handle: HANDLE) -> Availability<u64> {
    let mut counters = PROCESS_MEMORY_COUNTERS {
        cb: size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
        ..PROCESS_MEMORY_COUNTERS::default()
    };

    // SAFETY: `counters` is live and `cb` is its own size, as documented.
    if unsafe { K32GetProcessMemoryInfo(handle, &mut counters, counters.cb) } == 0 {
        return Availability::Unavailable(last_error_reason());
    }

    Availability::Available(counters.WorkingSetSize as u64)
}

fn filetime_to_unix_ms(time: &FILETIME) -> Option<u64> {
    let ticks = (u64::from(time.dwHighDateTime) << 32) | u64::from(time.dwLowDateTime);
    if ticks == 0 {
        return None;
    }

    ticks
        .checked_div(10_000)?
        .checked_sub(FILETIME_EPOCH_OFFSET_MS)
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

fn last_error_reason() -> UnavailableReason {
    match error_from(last_error_code()) {
        ProbeError::AccessDenied => UnavailableReason::AccessDenied,
        ProbeError::Failed => UnavailableReason::Failed,
    }
}

/// Owns the Toolhelp snapshot handle so it is closed on every return path.
struct Snapshot(HANDLE);

impl Snapshot {
    fn open() -> Result<Self, ProbeError> {
        // SAFETY: no pointer arguments; the call returns a handle or
        // INVALID_HANDLE_VALUE.
        let handle = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
        if handle == INVALID_HANDLE_VALUE {
            return Err(error_from(last_error_code()));
        }

        Ok(Self(handle))
    }
}

impl Drop for Snapshot {
    fn drop(&mut self) {
        // SAFETY: the handle came from CreateToolhelp32Snapshot and is closed once.
        unsafe { CloseHandle(self.0) };
    }
}

/// Owns a process handle so it is closed even when a field read fails.
struct ProcessHandle(HANDLE);

impl ProcessHandle {
    fn open(pid: u32, access: PROCESS_ACCESS_RIGHTS) -> Option<Self> {
        // SAFETY: OpenProcess takes no pointers; a null result means failure.
        let handle = unsafe { OpenProcess(access, 0, pid) };
        if handle.is_null() {
            None
        } else {
            Some(Self(handle))
        }
    }
}

impl Drop for ProcessHandle {
    fn drop(&mut self) {
        // SAFETY: the handle came from OpenProcess and is closed exactly once.
        unsafe { CloseHandle(self.0) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Machine smoke test: structural invariants only, no machine-specific names.
    #[test]
    fn native_inventory_has_structural_invariants() {
        let inventory = WindowsProcesses
            .inventory()
            .expect("process inventory must be collectable");

        assert!(!inventory.processes.is_empty());
        assert!(inventory.processes.len() <= MAX_PROCESSES);

        let own = std::process::id();
        assert!(
            inventory.processes.iter().any(|item| item.pid == own),
            "the current process must appear in its own inventory"
        );

        let mut pids: Vec<u32> = inventory.processes.iter().map(|item| item.pid).collect();
        pids.sort_unstable();
        pids.dedup();
        assert_eq!(
            pids.len(),
            inventory.processes.len(),
            "process IDs must be unique within one snapshot"
        );

        for process in &inventory.processes {
            if let Availability::Available(name) = &process.name {
                assert!(!name.is_empty());
            }
            if let Availability::Available(started) = process.started_at_ms {
                assert!(
                    started > 1_500_000_000_000,
                    "start times must look like unix milliseconds"
                );
            }
            if let Availability::Available(bytes) = process.working_set_bytes {
                assert!(bytes > 0, "a live process must have a working set");
            }
            assert!(!process.key().is_empty());
        }
    }
}
