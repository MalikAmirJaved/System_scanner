//! Read-only filesystem adapters.
//!
//! Enumeration lives here, not in `domain/`: it calls `std::fs` and reads
//! platform file attributes. The domain module only interprets the results.

pub mod directory;
pub mod traversal;
