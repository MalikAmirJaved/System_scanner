//! Minimal modal error dialog for the last-resort startup path.
//!
//! This is the only UI the Rust core draws itself, and it is used only after
//! Tauri failed before a window could show the message. It blocks until the
//! user dismisses it, so it must never be called from an async worker or a
//! command.
//!
//! The dialog is deliberately plain: a title, a message, and an OK button. No
//! icon choice, no retry action, and no callback, because the process exits
//! immediately afterwards.

use std::ffi::OsStr;
use std::iter;
use std::os::windows::ffi::OsStrExt;
use std::ptr;

use windows_sys::Win32::UI::WindowsAndMessaging::{
    MB_ICONERROR, MB_OK, MB_SETFOREGROUND, MessageBoxW,
};

/// Shows a blocking error dialog owned by the desktop.
///
/// A null owner handle is documented for a dialog that has no parent window,
/// which is exactly the case when the app failed before creating one.
pub fn show_error(title: &str, message: &str) {
    let title = wide(title);
    let message = wide(message);

    // SAFETY: both buffers are NUL-terminated UTF-16 owned for the duration of
    // the call, and a null HWND is valid for a dialog with no parent window.
    unsafe {
        MessageBoxW(
            ptr::null_mut(),
            message.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONERROR | MB_SETFOREGROUND,
        );
    }
}

fn wide(text: &str) -> Vec<u16> {
    OsStr::new(text)
        .encode_wide()
        .chain(iter::once(0))
        .collect()
}
