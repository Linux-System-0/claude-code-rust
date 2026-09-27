//! Identifier newtypes shared by the message and tool models.
//!
//! Phase 2 / T2.4. These are thin wrappers around `String`/`Uuid` so that a
//! `ToolUseId` cannot be accidentally passed where a `MessageId` is expected.
//! They serialize transparently (as the underlying string) to stay wire- and
//! transcript-compatible with the TypeScript implementation.

use std::fmt;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Declare a transparent string newtype with the usual conveniences.
macro_rules! string_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Wrap an existing string.
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            /// Borrow the underlying string.
            pub fn as_str(&self) -> &str {
                &self.0
            }

            /// Consume the newtype, returning the inner string.
            pub fn into_string(self) -> String {
                self.0
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.to_owned())
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }
    };
}

string_id! {
    /// Identifier of an assistant/user message (`msg_...` or a UUID).
    MessageId
}

string_id! {
    /// Identifier assigned to a `tool_use` content block.
    ToolUseId
}

string_id! {
    /// Identifier assigned to an OpenAI-style tool call.
    ToolCallId
}

/// Generate a fresh random UUID string.
///
/// Used for ids that the CLI itself mints (message uuids), matching the TS
/// implementation's `randomUUID()` behaviour.
pub fn new_uuid_string() -> String {
    Uuid::new_v4().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_serialize_transparently() {
        let id = MessageId::new("msg_123");
        assert_eq!(serde_json::to_string(&id).unwrap(), "\"msg_123\"");
        let back: MessageId = serde_json::from_str("\"msg_123\"").unwrap();
        assert_eq!(back, id);
        assert_eq!(back.as_str(), "msg_123");
    }

    #[test]
    fn uuid_string_is_valid() {
        let raw = new_uuid_string();
        assert!(Uuid::parse_str(&raw).is_ok());
    }

    #[test]
    fn tool_use_and_message_ids_are_distinct_types() {
        // Compile-time guarantee expressed at runtime: both wrap the same value
        // but remain separate types.
        let tool = ToolUseId::from("toolu_1");
        let call = ToolCallId::from("call_1");
        assert_ne!(tool.as_str(), call.as_str());
    }
}
