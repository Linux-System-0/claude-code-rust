//! Lightweight tracing and diagnostics for the whole workspace.
//!
//! Phase 1 / T1.4. This crate is the bottom of the dependency graph: it knows
//! about `tracing` and its own error type, nothing else. Every other crate may
//! depend on it to emit diagnostics.

#![forbid(unsafe_code)]

mod error;
mod logging;

pub use error::{Result, TelemetryError};
pub use logging::{default_log_path, init, InitOptions, Verbosity};

// Re-export the macros so downstream crates can log without depending on
// `tracing` directly.
pub use tracing::{debug, error, info, trace, warn};
