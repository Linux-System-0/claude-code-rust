//! Error and stop-reason contracts for the API layer (Phase 2 / T2.1).
//!
//! Only the classifications relevant to the two supported protocols are kept;
//! the TS `classifyAPIError` union also carried OAuth/Bedrock/Gemini buckets
//! that were removed with their providers (see Plan/01-trim-scope.md §1–2).

use serde::{Deserialize, Serialize};

/// Why an assistant turn ended.
///
/// The canonical spellings are the Anthropic ones. Unknown values are
/// preserved verbatim in [`StopReason::Other`] so a proxy speaking a slightly
/// different dialect does not break the parser. The OpenAI adapter maps its
/// `finish_reason` values onto this type in T3.6.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum StopReason {
    /// Natural end of turn.
    EndTurn,
    /// Output token limit reached.
    MaxTokens,
    /// A configured stop sequence was emitted.
    StopSequence,
    /// The model is waiting for tool results.
    ToolUse,
    /// A long-running turn was paused and can be resumed (Anthropic).
    PauseTurn,
    /// The model declined to continue.
    Refusal,
    /// Any other provider value, preserved verbatim.
    Other(String),
}

impl StopReason {
    /// The wire string for this stop reason.
    pub fn as_str(&self) -> &str {
        match self {
            StopReason::EndTurn => "end_turn",
            StopReason::MaxTokens => "max_tokens",
            StopReason::StopSequence => "stop_sequence",
            StopReason::ToolUse => "tool_use",
            StopReason::PauseTurn => "pause_turn",
            StopReason::Refusal => "refusal",
            StopReason::Other(value) => value,
        }
    }

    /// Whether the turn is waiting for tool results.
    pub fn is_tool_use(&self) -> bool {
        matches!(self, StopReason::ToolUse)
    }
}

impl From<String> for StopReason {
    fn from(value: String) -> Self {
        match value.as_str() {
            "end_turn" => StopReason::EndTurn,
            "max_tokens" => StopReason::MaxTokens,
            "stop_sequence" => StopReason::StopSequence,
            "tool_use" => StopReason::ToolUse,
            "pause_turn" => StopReason::PauseTurn,
            "refusal" => StopReason::Refusal,
            _ => StopReason::Other(value),
        }
    }
}

impl From<StopReason> for String {
    fn from(value: StopReason) -> String {
        value.as_str().to_owned()
    }
}

impl std::fmt::Display for StopReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A coarse classification of an API failure.
///
/// Used by the retry layer (T3.5) to decide whether a request may be retried
/// and by the UI to choose an actionable message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiErrorKind {
    /// User aborted the request.
    Aborted,
    /// Request exceeded the client timeout.
    Timeout,
    /// Provider returned 401/403 or the key is missing.
    Auth,
    /// HTTP 429.
    RateLimit,
    /// Provider overloaded (HTTP 529 / repeated 5xx).
    Overloaded,
    /// The prompt exceeds the model's context window.
    PromptTooLong,
    /// The request was malformed (HTTP 4xx other than 401/403/429).
    InvalidRequest,
    /// HTTP 5xx.
    Server,
    /// Transport failure (DNS, TLS, connection reset).
    Network,
    /// The response body could not be decoded.
    Decode,
    /// Anything not covered above.
    Unknown,
}

impl ApiErrorKind {
    /// Whether a request with this failure may be retried.
    pub const fn is_retryable(self) -> bool {
        matches!(
            self,
            ApiErrorKind::RateLimit
                | ApiErrorKind::Overloaded
                | ApiErrorKind::Timeout
                | ApiErrorKind::Server
                | ApiErrorKind::Network
        )
    }
}

/// A structured API failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[error("{message}")]
pub struct ApiError {
    /// Coarse classification.
    pub kind: ApiErrorKind,
    /// Human-readable message (already actionable where possible).
    pub message: String,
    /// HTTP status, when the failure came from a response.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    /// Provider request id, when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

impl ApiError {
    /// Build an error with no HTTP metadata.
    pub fn new(kind: ApiErrorKind, message: impl Into<String>) -> Self {
        ApiError {
            kind,
            message: message.into(),
            status: None,
            request_id: None,
        }
    }

    /// Build an error from an HTTP status.
    pub fn from_status(status: u16, message: impl Into<String>) -> Self {
        let kind = match status {
            401 | 403 => ApiErrorKind::Auth,
            429 => ApiErrorKind::RateLimit,
            400 | 404 | 405 | 409 | 413 | 422 => ApiErrorKind::InvalidRequest,
            529 => ApiErrorKind::Overloaded,
            s if (500..600).contains(&s) => ApiErrorKind::Server,
            _ => ApiErrorKind::Unknown,
        };
        ApiError {
            kind,
            message: message.into(),
            status: Some(status),
            request_id: None,
        }
    }

    /// Whether this failure may be retried.
    pub const fn is_retryable(&self) -> bool {
        self.kind.is_retryable()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn stop_reason_roundtrips_known_values() {
        for (wire, expected) in [
            ("end_turn", StopReason::EndTurn),
            ("max_tokens", StopReason::MaxTokens),
            ("stop_sequence", StopReason::StopSequence),
            ("tool_use", StopReason::ToolUse),
            ("pause_turn", StopReason::PauseTurn),
            ("refusal", StopReason::Refusal),
        ] {
            let decoded: StopReason =
                serde_json::from_value(json!(wire)).expect("known stop reason");
            assert_eq!(decoded, expected);
            assert_eq!(serde_json::to_value(&decoded).unwrap(), json!(wire));
        }
    }

    #[test]
    fn stop_reason_preserves_unknown_values() {
        let decoded: StopReason = serde_json::from_value(json!("some_proxy_value")).unwrap();
        assert_eq!(decoded, StopReason::Other("some_proxy_value".into()));
        assert_eq!(
            serde_json::to_value(&decoded).unwrap(),
            json!("some_proxy_value")
        );
        assert!(!decoded.is_tool_use());
    }

    #[test]
    fn status_classification_and_retryability() {
        assert_eq!(ApiError::from_status(401, "no").kind, ApiErrorKind::Auth);
        assert_eq!(
            ApiError::from_status(429, "slow").kind,
            ApiErrorKind::RateLimit
        );
        assert_eq!(
            ApiError::from_status(529, "busy").kind,
            ApiErrorKind::Overloaded
        );
        assert_eq!(
            ApiError::from_status(503, "down").kind,
            ApiErrorKind::Server
        );
        assert_eq!(
            ApiError::from_status(400, "bad").kind,
            ApiErrorKind::InvalidRequest
        );
        assert!(ApiError::from_status(503, "down").is_retryable());
        assert!(!ApiError::from_status(400, "bad").is_retryable());
    }
}
