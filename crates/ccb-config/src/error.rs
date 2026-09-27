//! Errors raised while locating, loading, merging, or resolving configuration.

use std::io;
use std::path::PathBuf;

/// The single actionable message shown when no protocol has been configured.
///
/// Mirrors `Plan/03-configuration.md` §3 verbatim; the caller prefixes `Error: `.
pub const NO_ENDPOINT_MESSAGE: &str = "No API endpoint configured.\n\n\
Configure one of the following:\n\n\
\x20 Anthropic-compatible:\n\
\x20   export ANTHROPIC_BASE_URL=<url>\n\
\x20   export ANTHROPIC_API_KEY=<key>\n\n\
\x20 OpenAI-compatible:\n\
\x20   export OPENAI_BASE_URL=<url>\n\
\x20   export OPENAI_API_KEY=<key>\n\n\
\x20 Or edit ~/.claude/settings.json (\"modelType\" + \"env\").";

/// Everything that can go wrong while producing a [`crate::ResolvedConfig`].
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// Neither an environment variable nor a settings layer provided an
    /// endpoint, and no `modelType` picked a protocol.
    #[error("{NO_ENDPOINT_MESSAGE}")]
    NoEndpoint,

    /// Both protocols look configured and nothing disambiguated them.
    #[error(
        "Ambiguous API configuration: both Anthropic and OpenAI endpoints are set.\n\
         Set \"modelType\" in settings.json (or pass --provider) to choose one."
    )]
    AmbiguousProtocol,

    /// `modelType` held a value other than `anthropic` / `openai`.
    #[error("unknown modelType `{value}`; expected `anthropic` or `openai`")]
    UnknownProtocol {
        /// The offending value.
        value: String,
    },

    /// `--setting-sources` held an unknown layer name.
    #[error("unknown setting source `{value}`; expected `user`, `project`, or `local`")]
    UnknownSettingSource {
        /// The offending value.
        value: String,
    },

    /// A protocol was selected but its base URL is unset.
    #[error(
        "Protocol `{protocol}` is selected but `{var}` is not set.\n\
         Set it in the environment or in settings.json (\"env\"), e.g.\n\n  \
         export {var}=<url>"
    )]
    MissingBaseUrl {
        /// Protocol name (`anthropic` / `openai`).
        protocol: &'static str,
        /// Environment variable that carries the base URL.
        var: &'static str,
    },

    /// A protocol was selected but no API key could be found.
    #[error(
        "Protocol `{protocol}` is selected but no API key was found.\n\
         Set one of {vars}, or configure `apiKeyHelper` in settings.json."
    )]
    MissingApiKey {
        /// Protocol name (`anthropic` / `openai`).
        protocol: &'static str,
        /// Comma-separated environment variable names that would satisfy it.
        vars: String,
    },

    /// A settings file existed but was not valid JSON.
    #[error("failed to parse settings file {path}: {source}")]
    ParseSettings {
        /// Path of the offending file.
        path: PathBuf,
        /// Underlying JSON error.
        source: serde_json::Error,
    },

    /// A settings file could not be read.
    #[error("failed to read settings file {path}: {source}")]
    ReadSettings {
        /// Path of the offending file.
        path: PathBuf,
        /// Underlying I/O error.
        source: io::Error,
    },

    /// The inline `--settings <json>` string was not valid JSON.
    #[error("failed to parse --settings JSON: {source}")]
    ParseInlineSettings {
        /// Underlying JSON error.
        source: serde_json::Error,
    },

    /// The configured `apiKeyHelper` command failed or produced no output.
    #[error("apiKeyHelper `{command}` failed: {message}")]
    ApiKeyHelper {
        /// The helper command as configured.
        command: String,
        /// Human-readable failure description.
        message: String,
    },

    /// A secret could not be persisted.
    #[error("failed to store API key: {message}")]
    SecretStore {
        /// Human-readable failure description.
        message: String,
    },

    /// The user's home directory could not be determined.
    #[error("could not determine the home directory; set $HOME or $CLAUDE_CONFIG_DIR")]
    NoHomeDir,
}

/// Convenience alias for configuration results.
pub type Result<T> = std::result::Result<T, ConfigError>;
