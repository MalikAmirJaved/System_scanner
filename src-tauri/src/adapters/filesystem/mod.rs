//! Read-only filesystem adapters.
//!
//! Enumeration lives here, not in `domain/`: it calls `std::fs` and reads
//! platform file attributes. The domain module only interprets the results.

pub mod directory;
pub mod traversal;

// The B-032 baseline harness: test-only, every test `#[ignore]`d. It writes a
// fixture tree, so it must never be part of a normal test run.
#[cfg(test)]
mod baseline;
