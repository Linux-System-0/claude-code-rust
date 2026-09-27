//! The protocol-neutral message model (Phase 2 / T2.1).
//!
//! This is the shared vocabulary between the conversation engine
//! (`ccb-core`) and the protocol adapters (`protocol-anthropic` /
//! `protocol-openai`). It lives in `ccb-api` rather than `ccb-core` because
//! the adapters depend on `ccb-api` directly and must be able to build
//! requests from these types without a dependency on the engine.
//!
//! The shapes deliberately mirror the Anthropic Messages wire format, which is
//! the richer of the two supported protocols: an OpenAI adapter can project
//! this model down to `{role, content}` + `tool_calls` (see T3.6). Content is
//! always normalized to a block array internally, even though both wires also
//! accept a bare string.

use serde::{Deserialize, Serialize};

use crate::ids::ToolUseId;

/// Conversation role. The assistant role is never valid for tool results;
/// those are carried on a [`Role::User`] message as [`ContentBlock::ToolResult`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// Human / tool-result turn.
    User,
    /// Model turn.
    Assistant,
}

impl Role {
    /// The wire name (`user` / `assistant`).
    pub const fn as_str(self) -> &'static str {
        match self {
            Role::User => "user",
            Role::Assistant => "assistant",
        }
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Source of an image content block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ImageSource {
    /// Inline base64-encoded image data.
    Base64 {
        /// MIME type, e.g. `image/png`.
        media_type: String,
        /// Base64 payload.
        data: String,
    },
    /// A URL the provider should fetch.
    Url {
        /// Image URL.
        url: String,
    },
}

/// Content of a `tool_result` block: either a plain string or a list of blocks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolResultContent {
    /// A single text payload (the common case).
    Text(String),
    /// Structured blocks, e.g. text plus images.
    Blocks(Vec<ContentBlock>),
}

impl Default for ToolResultContent {
    fn default() -> Self {
        ToolResultContent::Text(String::new())
    }
}

impl From<String> for ToolResultContent {
    fn from(value: String) -> Self {
        ToolResultContent::Text(value)
    }
}

impl From<&str> for ToolResultContent {
    fn from(value: &str) -> Self {
        ToolResultContent::Text(value.to_owned())
    }
}

/// A single element of a message's `content` array.
///
/// Serialized with an internal `type` tag so the shape matches the Anthropic
/// Messages API exactly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// Plain assistant/user text.
    Text {
        /// The text payload.
        text: String,
    },
    /// Extended-thinking text (Anthropic only).
    Thinking {
        /// The thinking text.
        thinking: String,
        /// Opaque signature used to replay thinking in later turns.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        signature: Option<String>,
    },
    /// Redacted thinking block (Anthropic only).
    RedactedThinking {
        /// Opaque provider payload.
        data: String,
    },
    /// A model request to invoke a tool.
    ToolUse {
        /// Unique id echoed back in the matching tool result.
        id: ToolUseId,
        /// Tool name.
        name: String,
        /// JSON arguments.
        #[serde(default = "empty_object")]
        input: serde_json::Value,
    },
    /// The result of a tool invocation (carried on a user message).
    ToolResult {
        /// Id of the [`ContentBlock::ToolUse`] this responds to.
        tool_use_id: ToolUseId,
        /// Result payload.
        #[serde(default)]
        content: ToolResultContent,
        /// Whether the payload represents an error.
        #[serde(default, skip_serializing_if = "is_false")]
        is_error: bool,
    },
    /// An inline image.
    Image {
        /// Where the image bytes come from.
        source: ImageSource,
    },
}

fn empty_object() -> serde_json::Value {
    serde_json::Value::Object(serde_json::Map::new())
}

fn is_false(value: &bool) -> bool {
    !*value
}

impl ContentBlock {
    /// Convenience constructor for a text block.
    pub fn text(text: impl Into<String>) -> Self {
        ContentBlock::Text { text: text.into() }
    }

    /// Convenience constructor for a tool-use block.
    pub fn tool_use(
        id: impl Into<ToolUseId>,
        name: impl Into<String>,
        input: serde_json::Value,
    ) -> Self {
        ContentBlock::ToolUse {
            id: id.into(),
            name: name.into(),
            input,
        }
    }

    /// Convenience constructor for a tool-result block.
    pub fn tool_result(
        tool_use_id: impl Into<ToolUseId>,
        content: impl Into<ToolResultContent>,
        is_error: bool,
    ) -> Self {
        ContentBlock::ToolResult {
            tool_use_id: tool_use_id.into(),
            content: content.into(),
            is_error,
        }
    }

    /// Borrow the block as a text payload, if it is one.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            ContentBlock::Text { text } => Some(text),
            _ => None,
        }
    }

    /// Borrow the block as a tool-use id, if it is one.
    pub fn tool_use_id(&self) -> Option<&ToolUseId> {
        match self {
            ContentBlock::ToolUse { id, .. } => Some(id),
            _ => None,
        }
    }
}

/// A conversation message.
///
/// Tagged by `role`, so it serializes to the Anthropic shape directly:
/// `{"role":"user","content":[{"type":"text","text":"hi"}]}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "role", rename_all = "snake_case")]
pub enum Message {
    /// A user turn (may carry tool results).
    User {
        /// Ordered content blocks.
        content: Vec<ContentBlock>,
    },
    /// An assistant turn.
    Assistant {
        /// Ordered content blocks.
        content: Vec<ContentBlock>,
    },
}

impl Message {
    /// Build a user message from arbitrary blocks.
    pub fn user(content: Vec<ContentBlock>) -> Self {
        Message::User { content }
    }

    /// Build an assistant message from arbitrary blocks.
    pub fn assistant(content: Vec<ContentBlock>) -> Self {
        Message::Assistant { content }
    }

    /// Build a user message with a single text block.
    pub fn user_text(text: impl Into<String>) -> Self {
        Message::User {
            content: vec![ContentBlock::text(text)],
        }
    }

    /// Build an assistant message with a single text block.
    pub fn assistant_text(text: impl Into<String>) -> Self {
        Message::Assistant {
            content: vec![ContentBlock::text(text)],
        }
    }

    /// Build a user message carrying one tool result.
    pub fn tool_result(
        tool_use_id: impl Into<ToolUseId>,
        content: impl Into<ToolResultContent>,
        is_error: bool,
    ) -> Self {
        Message::User {
            content: vec![ContentBlock::tool_result(tool_use_id, content, is_error)],
        }
    }

    /// This message's role.
    pub fn role(&self) -> Role {
        match self {
            Message::User { .. } => Role::User,
            Message::Assistant { .. } => Role::Assistant,
        }
    }

    /// The content blocks.
    pub fn content(&self) -> &[ContentBlock] {
        match self {
            Message::User { content } | Message::Assistant { content } => content,
        }
    }

    /// Mutable access to the content blocks.
    pub fn content_mut(&mut self) -> &mut Vec<ContentBlock> {
        match self {
            Message::User { content } | Message::Assistant { content } => content,
        }
    }

    /// Whether the message has no content blocks.
    pub fn is_empty(&self) -> bool {
        self.content().is_empty()
    }

    /// Concatenate every text block with newlines.
    pub fn text(&self) -> String {
        self.content()
            .iter()
            .filter_map(ContentBlock::as_text)
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Iterate over tool-use blocks as `(id, name, input)`.
    pub fn tool_uses(&self) -> impl Iterator<Item = (&ToolUseId, &str, &serde_json::Value)> {
        self.content().iter().filter_map(|block| match block {
            ContentBlock::ToolUse { id, name, input } => Some((id, name.as_str(), input)),
            _ => None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn user_text_serializes_to_wire_shape() {
        let msg = Message::user_text("hello");
        let value = serde_json::to_value(&msg).unwrap();
        assert_eq!(
            value,
            json!({"role": "user", "content": [{"type": "text", "text": "hello"}]})
        );
    }

    #[test]
    fn assistant_tool_use_roundtrips() {
        let msg = Message::assistant(vec![
            ContentBlock::text("thinking out loud"),
            ContentBlock::tool_use("toolu_1", "Read", json!({"file_path": "/tmp/a"})),
        ]);
        let encoded = serde_json::to_string(&msg).unwrap();
        let decoded: Message = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, msg);
        assert_eq!(decoded.role(), Role::Assistant);
        let uses: Vec<_> = decoded.tool_uses().collect();
        assert_eq!(uses.len(), 1);
        assert_eq!(uses[0].1, "Read");
    }

    #[test]
    fn tool_result_text_and_blocks_both_parse() {
        let as_text: Message = serde_json::from_value(json!({
            "role": "user",
            "content": [{
                "type": "tool_result",
                "tool_use_id": "toolu_1",
                "content": "ok"
            }]
        }))
        .unwrap();
        assert_eq!(
            as_text.content()[0],
            ContentBlock::tool_result("toolu_1", "ok", false)
        );

        let as_blocks: Message = serde_json::from_value(json!({
            "role": "user",
            "content": [{
                "type": "tool_result",
                "tool_use_id": "toolu_2",
                "content": [{"type": "text", "text": "ok"}],
                "is_error": true
            }]
        }))
        .unwrap();
        match &as_blocks.content()[0] {
            ContentBlock::ToolResult {
                content: ToolResultContent::Blocks(blocks),
                is_error: true,
                ..
            } => assert_eq!(blocks.len(), 1),
            other => panic!("unexpected block: {other:?}"),
        }
    }

    #[test]
    fn is_error_is_omitted_when_false() {
        let value = serde_json::to_value(ContentBlock::tool_result("t", "ok", false)).unwrap();
        assert!(value.get("is_error").is_none());
    }

    #[test]
    fn text_concatenates_only_text_blocks() {
        let msg = Message::assistant(vec![
            ContentBlock::text("a"),
            ContentBlock::tool_use("t", "X", json!({})),
            ContentBlock::text("b"),
        ]);
        assert_eq!(msg.text(), "a\nb");
    }

    #[test]
    fn empty_tool_use_input_defaults_to_object() {
        let block: ContentBlock =
            serde_json::from_value(json!({"type": "tool_use", "id": "t", "name": "X"})).unwrap();
        assert_eq!(block.tool_use_id().unwrap().as_str(), "t");
        if let ContentBlock::ToolUse { input, .. } = block {
            assert!(input.is_object());
        } else {
            unreachable!();
        }
    }
}
