//! Command-line surface (T1.1).
//!
//! This mirrors the *retained* shape of the TypeScript `main.tsx` Commander
//! program: the flags that survive the trim decisions in
//! `Plan/01-trim-scope.md`, plus the Rust-only `--provider` / `--base-url` /
//! `--api-key` overrides required because there is no default endpoint (D3).
//!
//! Removed-provider flags, OAuth/daemon/bridge/ACP/weixin fast-paths and the
//! surrounding subcommands are intentionally absent.

use std::path::PathBuf;

use ccb_config::Protocol;
use clap::{Parser, Subcommand, ValueEnum};

/// Version string printed by the `--version` fast-path.
pub const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), " (Claude Code)");

/// `ccb` — Claude Code (Rust rewrite).
#[derive(Debug, Parser)]
#[command(
    name = "ccb",
    about = "Claude Code (Rust) — starts an interactive session by default, use -p/--print for non-interactive output",
    disable_version_flag = true,
    subcommand_precedence_over_arg = true,
    args_conflicts_with_subcommands = true,
    propagate_version = false
)]
pub struct Cli {
    /// Your prompt.
    #[arg(value_name = "PROMPT")]
    pub prompt: Option<String>,

    #[command(flatten)]
    pub options: Options,

    #[command(subcommand)]
    pub command: Option<Command>,
}

/// Top-level subcommands that survive the trim.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Check configuration and environment health.
    Doctor(DoctorArgs),
}

/// Arguments for `ccb doctor`.
#[derive(Debug, Default, clap::Args)]
pub struct DoctorArgs {
    /// Emit the report as JSON.
    #[arg(long)]
    pub json: bool,
}

/// All top-level options.
#[derive(Debug, Default, clap::Args)]
pub struct Options {
    /// Enable debug mode with optional category filtering (e.g. "api,hooks").
    #[arg(
        short = 'd',
        long,
        value_name = "FILTER",
        num_args = 0..=1,
        default_missing_value = "true"
    )]
    pub debug: Option<String>,

    /// Enable debug mode (to stderr).
    #[arg(long = "debug-to-stderr", hide = true)]
    pub debug_to_stderr: bool,

    /// Write debug logs to a specific file path (implicitly enables debug mode).
    #[arg(long = "debug-file", value_name = "PATH")]
    pub debug_file: Option<PathBuf>,

    /// Override verbose mode setting from config.
    #[arg(long)]
    pub verbose: bool,

    /// Print response and exit (useful for pipes).
    #[arg(short = 'p', long = "print")]
    pub print: bool,

    /// Minimal mode: skip hooks, LSP, plugin sync, attribution, auto-memory,
    /// background prefetches, keychain reads, and CLAUDE.md auto-discovery.
    #[arg(long)]
    pub bare: bool,

    /// Output format (only works with --print): text, json, or stream-json.
    #[arg(long = "output-format", value_enum, value_name = "FORMAT")]
    pub output_format: Option<OutputFormat>,

    /// Input format (only works with --print): text or stream-json.
    #[arg(long = "input-format", value_enum, value_name = "FORMAT")]
    pub input_format: Option<InputFormat>,

    /// Bypass all permission checks. Recommended only for sandboxes.
    #[arg(long = "dangerously-skip-permissions")]
    pub dangerously_skip_permissions: bool,

    /// Make bypassing permissions available without enabling it by default.
    #[arg(long = "allow-dangerously-skip-permissions")]
    pub allow_dangerously_skip_permissions: bool,

    /// Maximum number of agentic turns in non-interactive mode.
    #[arg(long = "max-turns", value_name = "TURNS")]
    pub max_turns: Option<u32>,

    /// Maximum dollar amount to spend on API calls (only works with --print).
    #[arg(long = "max-budget-usd", value_name = "AMOUNT")]
    pub max_budget_usd: Option<f64>,

    /// Comma or space-separated list of tool names to allow.
    #[arg(
        long = "allowed-tools",
        visible_alias = "allowedTools",
        value_delimiter = ',',
        num_args = 1..,
        value_name = "TOOLS"
    )]
    pub allowed_tools: Vec<String>,

    /// Built-in tools to expose (`""` disables all, `default` uses all).
    #[arg(long, value_delimiter = ',', num_args = 1.., value_name = "TOOLS")]
    pub tools: Vec<String>,

    /// Comma or space-separated list of tool names to deny.
    #[arg(
        long = "disallowed-tools",
        visible_alias = "disallowedTools",
        value_delimiter = ',',
        num_args = 1..,
        value_name = "TOOLS"
    )]
    pub disallowed_tools: Vec<String>,

    /// Load MCP servers from JSON files or strings.
    #[arg(long = "mcp-config", num_args = 1.., value_name = "CONFIGS")]
    pub mcp_config: Vec<String>,

    /// System prompt to use for the session.
    #[arg(long = "system-prompt", value_name = "PROMPT")]
    pub system_prompt: Option<String>,

    /// Append to the default system prompt.
    #[arg(long = "append-system-prompt", value_name = "PROMPT")]
    pub append_system_prompt: Option<String>,

    /// Permission mode to use for the session.
    #[arg(long = "permission-mode", value_enum, value_name = "MODE")]
    pub permission_mode: Option<PermissionMode>,

    /// Continue the most recent conversation in the current directory.
    #[arg(short = 'c', long = "continue")]
    pub continue_: bool,

    /// Resume a conversation by session ID (or open the picker).
    #[arg(
        short = 'r',
        long,
        value_name = "VALUE",
        num_args = 0..=1,
        default_missing_value = "true"
    )]
    pub resume: Option<String>,

    /// Create a new session ID when resuming.
    #[arg(long = "fork-session")]
    pub fork_session: bool,

    /// Model for the current session.
    #[arg(long, value_name = "MODEL")]
    pub model: Option<String>,

    /// Effort level for the current session.
    #[arg(long, value_enum, value_name = "LEVEL")]
    pub effort: Option<Effort>,

    /// Automatic fallback model when the default is overloaded.
    #[arg(long = "fallback-model", value_name = "MODEL")]
    pub fallback_model: Option<String>,

    /// Settings JSON file or inline JSON string (repeatable).
    #[arg(long, value_name = "FILE_OR_JSON")]
    pub settings: Vec<String>,

    /// Additional directories to allow tool access to.
    #[arg(long = "add-dir", num_args = 1.., value_name = "DIRECTORIES")]
    pub add_dir: Vec<PathBuf>,

    /// Only use MCP servers from --mcp-config.
    #[arg(long = "strict-mcp-config")]
    pub strict_mcp_config: bool,

    /// Use a specific session ID for the conversation.
    #[arg(long = "session-id", value_name = "UUID")]
    pub session_id: Option<String>,

    /// Display name for this session.
    #[arg(short = 'n', long = "name", value_name = "NAME")]
    pub name: Option<String>,

    /// Comma-separated setting sources to load (user, project, local).
    #[arg(long = "setting-sources", value_name = "SOURCES")]
    pub setting_sources: Option<String>,

    /// Load plugins from a directory for this session only (repeatable).
    #[arg(long = "plugin-dir", value_name = "PATH")]
    pub plugin_dir: Vec<PathBuf>,

    /// Disable all slash commands / skills.
    #[arg(long = "disable-slash-commands")]
    pub disable_slash_commands: bool,

    // --- Rust-only, required by the "no default endpoint" decision (D3) -----
    /// Protocol to use: `anthropic` or `openai`. Overrides `modelType`.
    #[arg(long, value_enum, value_name = "PROTOCOL")]
    pub provider: Option<Provider>,

    /// Base URL for the selected protocol. Must be explicit (no default).
    #[arg(long = "base-url", value_name = "URL")]
    pub base_url: Option<String>,

    /// API key / bearer token for the selected protocol.
    #[arg(long = "api-key", value_name = "KEY")]
    pub api_key: Option<String>,
}

/// Protocol selector for `--provider`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Provider {
    /// Anthropic Messages compatible.
    #[value(name = "anthropic")]
    Anthropic,
    /// OpenAI Chat Completions compatible.
    #[value(name = "openai")]
    OpenAi,
}

impl From<Provider> for Protocol {
    fn from(value: Provider) -> Self {
        match value {
            Provider::Anthropic => Protocol::Anthropic,
            Provider::OpenAi => Protocol::OpenAi,
        }
    }
}

/// `--output-format`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    /// Plain text (default).
    Text,
    /// A single JSON result.
    Json,
    /// Real-time JSON streaming.
    StreamJson,
}

/// `--input-format`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum InputFormat {
    /// Plain text (default).
    Text,
    /// Real-time JSON streaming.
    StreamJson,
}

/// `--effort`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Effort {
    /// Low effort.
    Low,
    /// Medium effort.
    Medium,
    /// High effort.
    High,
    /// Maximum effort.
    Max,
}

/// `--permission-mode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum PermissionMode {
    /// Accept file edits without prompting.
    #[value(name = "acceptEdits")]
    AcceptEdits,
    /// Bypass every permission check.
    #[value(name = "bypassPermissions")]
    BypassPermissions,
    /// Prompt on first use of each tool (default).
    #[value(name = "default")]
    Default,
    /// Do not prompt; deny what is not explicitly allowed.
    #[value(name = "dontAsk")]
    DontAsk,
    /// Plan mode: read-only.
    #[value(name = "plan")]
    Plan,
    /// Automatic classifier-based mode.
    #[value(name = "auto")]
    Auto,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Cli {
        Cli::parse_from(std::iter::once("ccb").chain(args.iter().copied()))
    }

    #[test]
    fn parses_prompt_and_print() {
        let cli = parse(&["-p", "hello"]);
        assert!(cli.options.print);
        assert_eq!(cli.prompt.as_deref(), Some("hello"));
        assert!(cli.command.is_none());
    }

    #[test]
    fn parses_provider_and_endpoint_overrides() {
        let cli = parse(&[
            "--provider",
            "openai",
            "--base-url",
            "https://api.example.com/v1",
            "--api-key",
            "sk-x",
            "--model",
            "deepseek-chat",
        ]);
        assert_eq!(cli.options.provider, Some(Provider::OpenAi));
        assert_eq!(
            cli.options.base_url.as_deref(),
            Some("https://api.example.com/v1")
        );
        assert_eq!(cli.options.model.as_deref(), Some("deepseek-chat"));
    }

    #[test]
    fn rejects_removed_provider() {
        let err = Cli::try_parse_from(["ccb", "--provider", "bedrock"]).unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::InvalidValue);
    }

    #[test]
    fn debug_flag_accepts_optional_filter() {
        assert_eq!(parse(&["--debug"]).options.debug.as_deref(), Some("true"));
        assert_eq!(
            parse(&["--debug", "api,hooks"]).options.debug.as_deref(),
            Some("api,hooks")
        );
    }

    #[test]
    fn subcommand_wins_over_prompt_positional() {
        let cli = parse(&["doctor", "--json"]);
        assert!(matches!(cli.command, Some(Command::Doctor(_))));
        assert!(cli.prompt.is_none());
        // A positional alongside a subcommand is rejected rather than silently
        // treated as a prompt.
        assert!(Cli::try_parse_from(["ccb", "doctor", "thing"]).is_err());
    }

    #[test]
    fn free_text_prompt_is_a_positional() {
        let cli = parse(&["explain this codebase"]);
        assert!(cli.command.is_none());
        assert_eq!(cli.prompt.as_deref(), Some("explain this codebase"));
    }

    #[test]
    fn tool_lists_split_on_commas() {
        let cli = parse(&["--allowed-tools", "Bash(git:*),Edit,Read"]);
        assert_eq!(
            cli.options.allowed_tools,
            vec!["Bash(git:*)", "Edit", "Read"]
        );
    }

    #[test]
    fn settings_flag_is_repeatable() {
        let cli = parse(&["--settings", "a.json", "--settings", r#"{"model":"x"}"#]);
        assert_eq!(cli.options.settings.len(), 2);
    }
}
