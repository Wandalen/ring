//! Shared by the bench targets, `examples/report.rs` and `tests/harness_test.rs`: the candidates,
//! the drivers and their validation, and where results are written.

// Every target compiles its own copy of these modules and uses part of them.
#![allow(dead_code)]

pub mod bench;
pub mod candidates;
pub mod driver;
pub mod latency;
pub mod output;
pub mod topology;
