//! Session persistence, compaction, and file history.
//!
//! Phase 2 / T2.4 establishes the identifier types and the process-wide
//! session-state singleton that later phases build on:
//!
//! | Module | Contents |
//! |---|---|
//! | [`ids`] | [`SessionId`], [`AgentId`], [`current_session_id`] |
//!
//! Persistence (`~/.claude/projects`), compaction, and file history land in
//! Phases 5/9; no on-disk layout is fixed yet.

#![forbid(unsafe_code)]

pub mod ids;

pub use ids::{current_session_id, set_current_session_id, AgentId, SessionId};
