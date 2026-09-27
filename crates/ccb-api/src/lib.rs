//! Unified API abstraction and the protocol adapter contract.
//!
//! Phase 2 (T2.1–T2.4) establishes the protocol-neutral data model and the
//! runtime schema-validation layer that every later phase builds on:
//!
//! | Module | Contents |
//! |---|---|
//! | [`message`] | [`Message`], [`ContentBlock`], [`Role`], image/tool-result payloads |
//! | [`usage`] | [`Usage`] token accounting |
//! | [`error`] | [`ApiError`], [`ApiErrorKind`], [`StopReason`] |
//! | [`tool`] | [`ToolSpec`], [`ToolChoice`] |
//! | [`schema`] | JSON-Schema validation for tool inputs |
//! | [`ids`] | [`MessageId`], [`ToolUseId`], [`ToolCallId`] newtypes |
//!
//! # Why the model lives here
//!
//! The shared message model is consumed both by the protocol adapters
//! (`protocol-anthropic` / `protocol-openai`, which depend on this crate) and
//! by the conversation engine (`ccb-core`, which depends on `ccb-api`). Placing
//! it in `ccb-core` would force the adapters to depend on the engine and create
//! a cycle, so the neutral vocabulary lives at the bottom of the graph.
//! `ccb-core` re-exports it for ergonomic use.
//!
//! The unified `StreamEvent` and the `ProtocolAdapter` trait are introduced in
//! Phase 3 / T3.1; they build directly on these types.

#![forbid(unsafe_code)]

pub mod error;
pub mod ids;
pub mod message;
pub mod schema;
pub mod tool;
pub mod usage;

pub use error::{ApiError, ApiErrorKind, StopReason};
pub use ids::{new_uuid_string, MessageId, ToolCallId, ToolUseId};
pub use message::{ContentBlock, ImageSource, Message, Role, ToolResultContent};
pub use schema::{validate, validate_tool_input, ValidationError};
pub use tool::{ToolChoice, ToolSpec};
pub use usage::Usage;
