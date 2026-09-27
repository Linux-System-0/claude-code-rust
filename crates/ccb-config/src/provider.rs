//! The two supported wire protocols.
//!
//! Decision D2 (`Plan/README.md`): only Anthropic Messages and OpenAI Chat
//! Completions compatibility are kept. Every other provider is gone, and there
//! is deliberately no default protocol.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::ConfigError;

/// A supported API protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    /// Anthropic Messages compatible (`/v1/messages`, SSE).
    Anthropic,
    /// OpenAI Chat Completions compatible (`/chat/completions`, SSE).
    OpenAi,
}

impl Protocol {
    /// The canonical lower-case name, used in `modelType` and diagnostics.
    pub const fn as_str(self) -> &'static str {
        match self {
            Protocol::Anthropic => "anthropic",
            Protocol::OpenAi => "openai",
        }
    }

    /// Environment variable that carries the base URL.
    ///
    /// There is no compiled-in default (decision D3); when this is unset the
    /// resolver fails with [`ConfigError::MissingBaseUrl`].
    pub const fn base_url_var(self) -> &'static str {
        match self {
            Protocol::Anthropic => "ANTHROPIC_BASE_URL",
            Protocol::OpenAi => "OPENAI_BASE_URL",
        }
    }

    /// Environment variables that may carry the API key, in priority order.
    pub const fn api_key_vars(self) -> &'static [&'static str] {
        match self {
            // `ANTHROPIC_AUTH_TOKEN` is the gateway-style bearer token.
            Protocol::Anthropic => &["ANTHROPIC_API_KEY", "ANTHROPIC_AUTH_TOKEN"],
            Protocol::OpenAi => &["OPENAI_API_KEY"],
        }
    }

    /// Comma-separated list of [`Self::api_key_vars`] for diagnostics.
    pub fn api_key_vars_display(self) -> String {
        self.api_key_vars().join(", ")
    }
}

impl fmt::Display for Protocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Protocol {
    type Err = ConfigError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "anthropic" => Ok(Protocol::Anthropic),
            "openai" => Ok(Protocol::OpenAi),
            other => Err(ConfigError::UnknownProtocol {
                value: other.to_owned(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_known_protocols_case_insensitively() {
        assert_eq!(
            "anthropic".parse::<Protocol>().unwrap(),
            Protocol::Anthropic
        );
        assert_eq!("OpenAI".parse::<Protocol>().unwrap(), Protocol::OpenAi);
    }

    #[test]
    fn rejects_removed_providers() {
        // D2: bedrock/vertex/gemini/grok must not resolve.
        for value in ["bedrock", "vertex", "gemini", "grok", "foundry"] {
            assert!(
                value.parse::<Protocol>().is_err(),
                "{value} must be rejected"
            );
        }
    }
}
