//! Permission modes, rules, and the filesystem/bash rule engine.
//!
//! Phase 2 / T2.2 defines the type contracts here:
//!
//! | Module | Contents |
//! |---|---|
//! | [`mode`] | [`PermissionMode`] (`default` / `acceptEdits` / `bypassPermissions` / `plan`) |
//! | [`rules`] | [`PermissionBehavior`], [`PermissionRule`], [`PermissionRuleSource`] |
//! | [`decision`] | [`PermissionResult`], [`PermissionDecisionReason`], [`PermissionUpdate`] |
//! | [`context`] | [`ToolPermissionContext`] passed to tools |
//!
//! The rule *engine* that evaluates these types against a concrete tool call is
//! Phase 6 (T6.2). No decision logic lives in this crate yet.

#![forbid(unsafe_code)]

pub mod context;
pub mod decision;
pub mod mode;
pub mod rules;

pub use context::{AdditionalWorkingDirectory, ToolPermissionContext};
pub use decision::{
    PermissionDecision, PermissionDecisionReason, PermissionResult, PermissionUpdate,
    PermissionUpdateDestination,
};
pub use mode::{ParsePermissionModeError, PermissionMode};
pub use rules::{
    PermissionBehavior, PermissionRule, PermissionRuleSource, PermissionRuleValue,
    ToolPermissionRulesBySource,
};
