//! Platform adapters. Only this layer may import `windows-sys`.

pub mod persistence;

#[cfg(windows)]
pub mod windows;
