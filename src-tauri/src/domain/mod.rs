//! Domain rules, policies, and use cases. Platform- and storage-neutral.

pub mod diagnostics;
pub mod job;
// The protected-path policy has no production caller until the first
// state-changing command exists (B-041); its own tests exercise it meanwhile.
#[allow(dead_code)]
pub mod policy;
pub mod process;
pub mod startup;
pub mod system;
pub mod traversal;
