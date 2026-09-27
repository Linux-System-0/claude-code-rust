//! Message model, query loop, and QueryEngine orchestration.
//!
//! Scaffolded in Phase 0 (T0.1). Implementation lands in later phases.

#![forbid(unsafe_code)]

// The CLI must always be built with at least one protocol adapter. This turns
// a misconfigured `--no-default-features` build into an actionable error
// instead of a runtime "unknown provider" surprise.
#[cfg(not(any(feature = "anthropic", feature = "openai")))]
compile_error!(
    "ccb-core requires at least one protocol feature: enable `anthropic` and/or \
     `openai` (e.g. `--no-default-features --features anthropic`)"
);

/// Protocol adapters compiled into this build.
///
/// Exactly two protocols are supported (Plan/01-trim-scope.md §2): the
/// Anthropic Messages adapter and the OpenAI Chat Completions adapter.
pub mod protocols {
    #[cfg(feature = "anthropic")]
    pub use protocol_anthropic as anthropic;

    #[cfg(feature = "openai")]
    pub use protocol_openai as openai;
}
