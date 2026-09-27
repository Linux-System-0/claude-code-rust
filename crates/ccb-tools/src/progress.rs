//! Tool progress events (Phase 2 / T2.2).
//!
//! Tools report incremental progress while they run; the engine forwards each
//! event to the UI. The TS implementation used one payload type per tool
//! (`BashProgress`, `MCPProgress`, ...), all of which were reduced to `any` in
//! the frozen reference. The Rust model uses a tagged enum so the UI can match
//! on a stable set of kinds and fall back to [`ToolProgressData::Generic`] for
//! tool-specific payloads.

use ccb_api::ToolUseId;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Incremental progress emitted by a running tool.
///
/// The tag is `snake_case` (`bash`, `read`, ...) and fields are `camelCase`,
/// matching the rest of the streamed model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum ToolProgressData {
    /// `Bash` / `PowerShell` stdout while a command runs.
    Bash {
        /// Output received so far.
        output: String,
        /// Full buffered output, when different from `output`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        full_output: Option<String>,
        /// Milliseconds elapsed.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        elapsed_ms: Option<u64>,
    },
    /// File read progress.
    Read {
        /// Path being read.
        path: String,
        /// Bytes read so far.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bytes_read: Option<u64>,
    },
    /// Search progress.
    Search {
        /// The query or pattern.
        query: String,
        /// Number of matches found so far.
        matches: u64,
    },
    /// MCP call progress.
    Mcp {
        /// Server name.
        server: String,
        /// JSON-RPC method.
        method: String,
    },
    /// A human-readable status line.
    Status {
        /// Status text.
        message: String,
    },
    /// Tool-specific payload not covered by a dedicated variant.
    Generic {
        /// Tool-defined progress kind.
        kind: String,
        /// Opaque payload.
        data: Value,
    },
}

/// A progress event bound to the tool call that produced it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolProgress {
    /// The originating tool-use id.
    #[serde(rename = "toolUseID")]
    pub tool_use_id: ToolUseId,
    /// The progress payload.
    pub data: ToolProgressData,
}

impl ToolProgress {
    /// Build a progress event.
    pub fn new(tool_use_id: impl Into<ToolUseId>, data: ToolProgressData) -> Self {
        ToolProgress {
            tool_use_id: tool_use_id.into(),
            data,
        }
    }

    /// A short, one-line label for the given progress event, if available.
    pub fn summary(&self) -> Option<&str> {
        match &self.data {
            ToolProgressData::Status { message } => Some(message),
            ToolProgressData::Search { query, .. } => Some(query),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn bash_progress_roundtrips() {
        let progress = ToolProgress::new(
            "toolu_1",
            ToolProgressData::Bash {
                output: "hello\n".into(),
                full_output: None,
                elapsed_ms: Some(12),
            },
        );
        let value = serde_json::to_value(&progress).unwrap();
        assert_eq!(
            value,
            json!({
                "toolUseID": "toolu_1",
                "data": {"type": "bash", "output": "hello\n", "elapsedMs": 12}
            })
        );
        let decoded: ToolProgress = serde_json::from_value(value).unwrap();
        assert_eq!(decoded, progress);
    }

    #[test]
    fn generic_progress_summary_is_none() {
        let progress = ToolProgress::new(
            "toolu_2",
            ToolProgressData::Generic {
                kind: "custom".into(),
                data: json!({"pct": 50}),
            },
        );
        assert_eq!(progress.summary(), None);
    }

    #[test]
    fn status_summary_returns_message() {
        let progress = ToolProgress::new(
            "toolu_3",
            ToolProgressData::Status {
                message: "compiling".into(),
            },
        );
        assert_eq!(progress.summary(), Some("compiling"));
    }
}
