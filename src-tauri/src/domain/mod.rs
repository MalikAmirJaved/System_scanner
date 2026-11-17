//! Domain rules, policies, and use cases. Platform- and storage-neutral.

pub mod diagnostics;
pub mod process;
pub mod system;
// The traversal vocabulary has no production caller until scan jobs exist
// (B-025). The allowance is scoped to this module and should be removed then.
#[allow(dead_code)]
pub mod traversal;
