//! Platform adapters. Only this layer may import `windows-sys`.

// The traversal prototype has no production caller until scan jobs exist
// (B-025). The allowance is scoped to this module and should be removed then.
#[allow(dead_code)]
pub mod filesystem;
pub mod persistence;

#[cfg(windows)]
pub mod windows;
