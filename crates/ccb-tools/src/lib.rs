//! Tool trait, registry, dispatch, and the built-in tool whitelist.
//!
//! Phase 2 / T2.2 establishes the tool-side contracts:
//!
//! | Module | Contents |
//! |---|---|
//! | [`contract`] | [`ValidationResult`], [`ToolResult`], [`McpMeta`] |
//! | [`progress`] | [`ToolProgress`], [`ToolProgressData`] |
//!
//! The executable [`ccb_api::ToolSpec`] contract is defined in `ccb-api` (it is
//! shared with the protocol adapters) and re-exported here. The `Tool` trait,
//! registry, and dispatch are Phase 4 (T4.1); no tools are implemented yet.

#![forbid(unsafe_code)]

pub mod contract;
pub mod progress;

pub use contract::{McpMeta, ToolResult, ValidationResult};
pub use progress::{ToolProgress, ToolProgressData};

// Re-export the shared tool-definition contract for ergonomic use.
pub use ccb_api::{ToolChoice, ToolSpec};
