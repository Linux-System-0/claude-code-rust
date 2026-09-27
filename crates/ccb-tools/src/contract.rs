//! Tool execution contracts (Phase 2 / T2.2).
//!
//! The executable `Tool` trait, registry, and dispatch live in Phase 4 (T4.1).
//! This module defines the result/validation shapes that tools will return and
//! that the engine (`ccb-core`) will consume.

use ccb_api::Message;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The result of validating a tool's input before execution.
///
/// Mirrors the TS `ValidationResult` shape exactly: `{"result":true}` on
/// success, or `{"result":false,"message":...,"errorCode":...}` on failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationResult {
    /// Whether the input is acceptable.
    pub result: bool,
    /// Why the input is invalid (present only when `result` is false).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Stable error code carried back to the model.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_code: Option<i64>,
}

impl ValidationResult {
    /// The success value.
    pub const fn ok() -> Self {
        ValidationResult {
            result: true,
            message: None,
            error_code: None,
        }
    }

    /// A rejection with a message and error code.
    pub fn invalid(message: impl Into<String>, error_code: i64) -> Self {
        ValidationResult {
            result: false,
            message: Some(message.into()),
            error_code: Some(error_code),
        }
    }

    /// Whether the input is acceptable.
    pub const fn is_valid(&self) -> bool {
        self.result
    }
}

/// MCP protocol metadata returned alongside a tool result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpMeta {
    /// The MCP `_meta` object.
    #[serde(rename = "_meta", default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<Value>,
    /// MCP `structuredContent`, passed through to SDK consumers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub structured_content: Option<Value>,
}

/// The outcome of running a tool.
///
/// `data` is the tool-specific payload; `new_messages` lets a tool inject
/// follow-up messages into the transcript; `mcp_meta` carries protocol
/// metadata. The TS `contextModifier` closure is deliberately omitted here —
/// it is a runtime, non-serializable concern owned by Phase 4/5.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolResult {
    /// Tool-specific result payload.
    pub data: Value,
    /// Whether `data` represents an error.
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_error: bool,
    /// Additional messages to append to the conversation.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub new_messages: Vec<Message>,
    /// MCP protocol metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp_meta: Option<McpMeta>,
}

fn is_false(value: &bool) -> bool {
    !*value
}

impl ToolResult {
    /// A successful result wrapping `data`.
    pub fn new(data: Value) -> Self {
        ToolResult {
            data,
            is_error: false,
            new_messages: Vec::new(),
            mcp_meta: None,
        }
    }

    /// An error result wrapping `data`.
    pub fn error(data: Value) -> Self {
        ToolResult {
            data,
            is_error: true,
            new_messages: Vec::new(),
            mcp_meta: None,
        }
    }

    /// Attach follow-up messages.
    pub fn with_messages(mut self, messages: Vec<Message>) -> Self {
        self.new_messages = messages;
        self
    }

    /// Attach MCP metadata.
    pub fn with_mcp_meta(mut self, meta: McpMeta) -> Self {
        self.mcp_meta = Some(meta);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn validation_result_variants() {
        assert!(ValidationResult::ok().is_valid());
        let invalid = ValidationResult::invalid("bad path", 7);
        assert!(!invalid.is_valid());
        assert_eq!(
            serde_json::to_value(&invalid).unwrap(),
            json!({"result": false, "message": "bad path", "errorCode": 7})
        );
    }

    #[test]
    fn tool_result_omits_empty_optionals() {
        let result = ToolResult::new(json!({"ok": true}));
        assert_eq!(
            serde_json::to_value(&result).unwrap(),
            json!({"data": {"ok": true}})
        );
        let decoded: ToolResult = serde_json::from_value(json!({"data": {"ok": true}})).unwrap();
        assert_eq!(decoded, result);
    }

    #[test]
    fn tool_result_carries_messages_and_mcp_meta() {
        let result = ToolResult::error(json!("failed"))
            .with_messages(vec![Message::user_text("retry")])
            .with_mcp_meta(McpMeta {
                meta: Some(json!({"request": 1})),
                structured_content: None,
            });
        let value = serde_json::to_value(&result).unwrap();
        assert_eq!(value["isError"], json!(true));
        assert_eq!(value["newMessages"][0]["role"], json!("user"));
        assert_eq!(value["mcpMeta"]["_meta"]["request"], json!(1));
        let decoded: ToolResult = serde_json::from_value(value).unwrap();
        assert_eq!(decoded, result);
    }
}
