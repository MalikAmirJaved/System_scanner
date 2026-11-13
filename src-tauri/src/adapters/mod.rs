//! Platform adapters. Only this layer may import `windows-sys`.

#[cfg(windows)]
pub mod windows;
