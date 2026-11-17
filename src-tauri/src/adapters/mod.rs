//! Platform adapters. Only this layer may import `windows-sys`.

pub mod filesystem;
pub mod persistence;

#[cfg(windows)]
pub mod windows;
