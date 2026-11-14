//! Win32 implementation of [`DiagnosticsPort`].
//!
//! Both calls are read-only and need no elevation. Failures return `None` rather
//! than an invented value, because a wrong identity would weaken redaction and a
//! wrong elevation flag would misstate the environment.

use std::ptr;

use windows_sys::Win32::Foundation::CloseHandle;
use windows_sys::Win32::Security::{
    GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation,
};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
use windows_sys::Win32::System::WindowsProgramming::{GetComputerNameW, GetUserNameW};

use crate::ports::diagnostics::{DiagnosticsPort, LocalIdentity};

/// Account names max out at 256 UTF-16 units including the terminator.
const MAX_NAME_UNITS: usize = 256;

/// Read-only identity and token probe.
#[derive(Debug, Default, Clone, Copy)]
pub struct WindowsDiagnostics;

impl DiagnosticsPort for WindowsDiagnostics {
    fn identity(&self) -> LocalIdentity {
        LocalIdentity {
            user_name: user_name(),
            computer_name: computer_name(),
        }
    }

    fn is_elevated(&self) -> Option<bool> {
        let mut token = ptr::null_mut();
        // SAFETY: the current-process pseudo handle is always valid, and `token`
        // is written only on success.
        let opened = unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) };
        if opened == 0 {
            return None;
        }

        let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut returned = 0;
        // SAFETY: `elevation` is a correctly sized TOKEN_ELEVATION buffer and
        // `token` was opened for TOKEN_QUERY above.
        let read = unsafe {
            GetTokenInformation(
                token,
                TokenElevation,
                (&mut elevation as *mut TOKEN_ELEVATION).cast(),
                std::mem::size_of::<TOKEN_ELEVATION>() as u32,
                &mut returned,
            )
        };
        // SAFETY: `token` is an owned handle from OpenProcessToken and is closed
        // exactly once here.
        unsafe { CloseHandle(token) };

        if read == 0 {
            return None;
        }

        Some(elevation.TokenIsElevated != 0)
    }
}

fn user_name() -> Option<String> {
    let mut buffer = [0u16; MAX_NAME_UNITS];
    let mut size = buffer.len() as u32;

    // SAFETY: the buffer is valid for `size` UTF-16 units and `size` is updated
    // by the call on success.
    let ok = unsafe { GetUserNameW(buffer.as_mut_ptr(), &mut size) };
    if ok == 0 {
        return None;
    }

    Some(wide_name(&buffer))
}

fn computer_name() -> Option<String> {
    let mut buffer = [0u16; MAX_NAME_UNITS];
    let mut size = buffer.len() as u32;

    // SAFETY: the buffer is valid for `size` UTF-16 units and `size` is updated
    // by the call on success.
    let ok = unsafe { GetComputerNameW(buffer.as_mut_ptr(), &mut size) };
    if ok == 0 {
        return None;
    }

    Some(wide_name(&buffer))
}

fn wide_name(buffer: &[u16]) -> String {
    let end = buffer
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..end]).trim().to_owned()
}
