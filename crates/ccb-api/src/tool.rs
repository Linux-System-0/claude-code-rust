//! Tool definition contract shared with the protocol adapters (T2.2).
//!
//! The richer executable contract (`Tool` trait, registry, dispatch) is Phase 4
//! / T4.1. What lives here is only what the adapters need to serialize a tool
//! definition into a request, plus a validator for model-provided arguments
//! (T2.3).

use serde::{Deserialize, Serialize};

use crate::schema::{self, ValidationError};

/// A tool exposed to the model.
///
/// `input_schema` is a JSON Schema document. Both supported protocols accept
/// this shape: Anthropic emits it as `input_schema`, the OpenAI adapter wraps
/// it in `function.parameters`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolSpec {
    /// Unique tool name (`Read`, `Bash`, ...).
    pub name: String,
    /// Natural-language description shown to the model.
    pub description: String,
    /// JSON Schema for the tool's arguments.
    pub input_schema: serde_json::Value,
}

impl ToolSpec {
    /// Build a tool spec; `input_schema` should be an object schema.
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: serde_json::Value,
    ) -> Self {
        ToolSpec {
            name: name.into(),
            description: description.into(),
            input_schema,
        }
    }

    /// Validate model-provided arguments against this tool's schema.
    pub fn validate_input(&self, input: &serde_json::Value) -> Result<(), ValidationError> {
        schema::validate(&self.input_schema, input)
    }
}

/// How the model should choose a tool, when constrained.
///
/// A protocol-neutral subset: `Any` corresponds to Anthropic's `{"type":"any"}`
/// and OpenAI's `"required"`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolChoice {
    /// Let the model decide.
    Auto,
    /// Disable tool use entirely.
    None,
    /// Force the model to call some tool.
    Any,
    /// Force a specific tool.
    Tool {
        /// The tool name to force.
        name: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn tool_spec_roundtrips() {
        let spec = ToolSpec::new(
            "Read",
            "Read a file",
            json!({
                "type": "object",
                "properties": {"file_path": {"type": "string"}},
                "required": ["file_path"],
                "additionalProperties": false
            }),
        );
        let encoded = serde_json::to_string(&spec).unwrap();
        let decoded: ToolSpec = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, spec);
        assert!(spec.validate_input(&json!({"file_path": "/a"})).is_ok());
        assert!(spec.validate_input(&json!({})).is_err());
    }

    #[test]
    fn tool_choice_serializes_with_tag() {
        assert_eq!(
            serde_json::to_value(ToolChoice::Auto).unwrap(),
            json!({"type": "auto"})
        );
        assert_eq!(
            serde_json::to_value(ToolChoice::Tool {
                name: "Read".into()
            })
            .unwrap(),
            json!({"type": "tool", "name": "Read"})
        );
    }
}
