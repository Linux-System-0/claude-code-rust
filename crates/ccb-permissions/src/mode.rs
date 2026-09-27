//! Permission mode contract (Phase 2 / T2.2).
//!
//! The user-addressable modes kept after trimming (Plan/04-task-list.md T6.1):
//! `default`, `acceptEdits`, `bypassPermissions`, `plan`. The TS
//! `dontAsk`/`auto`/`bubble` modes were removed with their features.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// How tool permissions are decided for a session.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionMode {
    /// Prompt the user for anything not covered by an allow rule.
    #[default]
    Default,
    /// Auto-approve file edits; still prompt for other tools.
    AcceptEdits,
    /// Skip all prompts (requires the session flag to be available).
    BypassPermissions,
    /// Read-only planning: mutating tools are denied.
    Plan,
}

impl PermissionMode {
    /// The wire/settings spelling of this mode.
    pub const fn as_str(self) -> &'static str {
        match self {
            PermissionMode::Default => "default",
            PermissionMode::AcceptEdits => "acceptEdits",
            PermissionMode::BypassPermissions => "bypassPermissions",
            PermissionMode::Plan => "plan",
        }
    }

    /// Every mode, in canonical order.
    pub const ALL: [PermissionMode; 4] = [
        PermissionMode::Default,
        PermissionMode::AcceptEdits,
        PermissionMode::BypassPermissions,
        PermissionMode::Plan,
    ];

    /// Whether the mode suppresses permission prompts.
    pub const fn is_bypass(self) -> bool {
        matches!(self, PermissionMode::BypassPermissions)
    }

    /// Whether the mode is read-only.
    pub const fn is_plan(self) -> bool {
        matches!(self, PermissionMode::Plan)
    }

    /// Whether file edits are auto-approved in this mode.
    pub const fn auto_approves_edits(self) -> bool {
        matches!(
            self,
            PermissionMode::AcceptEdits | PermissionMode::BypassPermissions
        )
    }
}

impl fmt::Display for PermissionMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Raised when a string is not a valid [`PermissionMode`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unknown permission mode `{0}`; expected default, acceptEdits, bypassPermissions, or plan")]
pub struct ParsePermissionModeError(pub String);

impl FromStr for PermissionMode {
    type Err = ParsePermissionModeError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim() {
            "default" => Ok(PermissionMode::Default),
            "acceptEdits" => Ok(PermissionMode::AcceptEdits),
            "bypassPermissions" => Ok(PermissionMode::BypassPermissions),
            "plan" => Ok(PermissionMode::Plan),
            other => Err(ParsePermissionModeError(other.to_owned())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_and_display_roundtrip() {
        for mode in PermissionMode::ALL {
            assert_eq!(mode.as_str().parse::<PermissionMode>().unwrap(), mode);
            assert_eq!(mode.to_string(), mode.as_str());
        }
        assert!("dontAsk".parse::<PermissionMode>().is_err());
    }

    #[test]
    fn serde_uses_camel_case() {
        assert_eq!(
            serde_json::to_value(PermissionMode::BypassPermissions).unwrap(),
            json!("bypassPermissions")
        );
        let mode: PermissionMode = serde_json::from_value(json!("acceptEdits")).unwrap();
        assert_eq!(mode, PermissionMode::AcceptEdits);
    }

    #[test]
    fn mode_capabilities() {
        assert!(PermissionMode::BypassPermissions.is_bypass());
        assert!(PermissionMode::Plan.is_plan());
        assert!(PermissionMode::AcceptEdits.auto_approves_edits());
        assert!(!PermissionMode::Default.auto_approves_edits());
        assert_eq!(PermissionMode::default(), PermissionMode::Default);
    }
}
