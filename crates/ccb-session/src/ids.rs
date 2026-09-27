//! Session and agent identifiers (Phase 2 / T2.4).
//!
//! Mirrors `src/types/ids.ts` and `src/utils/uuid.ts`:
//!
//! - [`SessionId`] is a UUID, generated with `Uuid::new_v4()`.
//! - [`AgentId`] is `a{label-}{16 hex chars}`, e.g. `aa3f2c1b4d5e6f7a8` or
//!   `acompact-a3f2c1b4d5e6f7a8`.

use std::fmt;
use std::str::FromStr;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A session UUID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SessionId(Uuid);

impl SessionId {
    /// Generate a fresh random session id.
    pub fn new() -> Self {
        SessionId(Uuid::new_v4())
    }

    /// Parse a session id from its UUID string.
    pub fn parse(value: &str) -> Option<Self> {
        Uuid::parse_str(value).ok().map(SessionId)
    }

    /// The underlying UUID.
    pub const fn as_uuid(self) -> Uuid {
        self.0
    }
}

impl Default for SessionId {
    fn default() -> Self {
        SessionId::new()
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for SessionId {
    type Err = uuid::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(value).map(SessionId)
    }
}

/// The process-wide current session id.
///
/// Mirrors the TypeScript module-level `sessionId` singleton: every caller in
/// a process observes the same id unless it is explicitly overridden (for
/// example when resuming an existing session from disk).
static CURRENT_SESSION_ID: OnceLock<SessionId> = OnceLock::new();

/// Return the current session id, minting one on first use.
pub fn current_session_id() -> SessionId {
    *CURRENT_SESSION_ID.get_or_init(SessionId::new)
}

/// Override the current session id (e.g. when resuming from disk).
///
/// Returns `Err` with the rejected value if an id was already installed.
pub fn set_current_session_id(id: SessionId) -> Result<(), SessionId> {
    CURRENT_SESSION_ID.set(id)
}

/// A subagent id, `a{label-}{16 hex chars}`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AgentId(String);

impl AgentId {
    /// Generate a fresh agent id, optionally with a label prefix.
    ///
    /// The 16-hex suffix is the first 16 characters of a random UUID; this
    /// avoids pulling in a dedicated RNG dependency while matching the TS
    /// `randomBytes(8).toString('hex')` output shape.
    pub fn new(label: Option<&str>) -> Self {
        let uuid = Uuid::new_v4().simple().to_string();
        let suffix = &uuid[..16];
        match label {
            Some(label) => AgentId(format!("a{label}-{suffix}")),
            None => AgentId(format!("a{suffix}")),
        }
    }

    /// Parse an agent id, returning `None` if it does not match the format.
    ///
    /// Matches the TS `AGENT_ID_PATTERN = /^a(?:.+-)?[0-9a-f]{16}$/`.
    pub fn parse(value: &str) -> Option<Self> {
        let rest = value.strip_prefix('a')?;
        if rest.len() < 16 {
            return None;
        }
        let (label_part, hex_part) = rest.split_at(rest.len() - 16);
        // Lowercase hex only, matching the TS `/^a(?:.+-)?[0-9a-f]{16}$/`.
        if !hex_part
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
        {
            return None;
        }
        if label_part.is_empty() {
            return Some(AgentId(value.to_owned()));
        }
        // A non-empty label must be at least one character followed by `-`.
        if label_part.len() >= 2 && label_part.ends_with('-') {
            return Some(AgentId(value.to_owned()));
        }
        None
    }

    /// The raw id.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The label portion, if present.
    pub fn label(&self) -> Option<&str> {
        let rest = self.0.strip_prefix('a')?;
        if rest.len() <= 16 {
            return None;
        }
        rest.split_at(rest.len() - 16).0.strip_suffix('-')
    }

    /// Consume the id, returning the inner string.
    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Display for AgentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_session_id_is_a_stable_singleton() {
        let first = current_session_id();
        let second = current_session_id();
        assert_eq!(first, second);
        assert_eq!(SessionId::parse(&first.to_string()), Some(first));
    }

    #[test]
    fn session_id_is_a_uuid() {
        let id = SessionId::new();
        assert_eq!(SessionId::parse(&id.to_string()), Some(id));
        assert!(SessionId::parse("not-a-uuid").is_none());
        assert_eq!(id, id.as_uuid().to_string().parse().unwrap());
    }

    #[test]
    fn agent_id_generation_has_expected_shape() {
        let unlabeled = AgentId::new(None);
        assert!(unlabeled.as_str().starts_with('a'));
        assert_eq!(unlabeled.as_str().len(), 17);
        assert_eq!(unlabeled.label(), None);
        assert_eq!(AgentId::parse(unlabeled.as_str()), Some(unlabeled.clone()));

        let labeled = AgentId::new(Some("compact"));
        assert!(labeled.as_str().starts_with("acompact-"));
        assert_eq!(labeled.label(), Some("compact"));
        assert_eq!(AgentId::parse(labeled.as_str()), Some(labeled));
    }

    #[test]
    fn agent_id_parse_rejects_bad_values() {
        assert!(AgentId::parse("").is_none());
        assert!(AgentId::parse("a").is_none());
        assert!(AgentId::parse("a0123456789abcdef").is_some());
        // Uppercase hex is not accepted (matches the TS regex).
        assert!(AgentId::parse("a0123456789ABCDEF").is_none());
        // A label must be followed by `-`.
        assert!(AgentId::parse("ax0123456789abcdef").is_none());
        // A bare dash is not a valid label.
        assert!(AgentId::parse("a-0123456789abcdef").is_none());
    }
}
