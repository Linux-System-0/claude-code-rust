//! Permission decisions and updates (Phase 2 / T2.2).
//!
//! Mirrors the `PermissionDecision` / `PermissionResult` / `PermissionUpdate`
//! unions from `src/types/permissions.ts`, trimmed to the sources and reasons
//! that survive the scope cut (no async-agent, classifier, permission-prompt
//! tool, or sandbox-override reasons).

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::mode::PermissionMode;
use crate::rules::{PermissionBehavior, PermissionRule, PermissionRuleValue};

/// Where a permission update should be persisted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionUpdateDestination {
    /// `~/.claude/settings.json`.
    UserSettings,
    /// `<project>/.claude/settings.json`.
    ProjectSettings,
    /// `<project>/.claude/settings.local.json`.
    LocalSettings,
    /// In-memory only, for the current session.
    Session,
    /// The invoking CLI argument.
    CliArg,
}

/// An operation that mutates the active rule set or mode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum PermissionUpdate {
    /// Add rules for a behavior.
    AddRules {
        /// Where to persist the change.
        destination: PermissionUpdateDestination,
        /// Rules to add.
        rules: Vec<PermissionRuleValue>,
        /// Behavior the rules record.
        behavior: PermissionBehavior,
    },
    /// Replace all rules for a behavior.
    ReplaceRules {
        /// Where to persist the change.
        destination: PermissionUpdateDestination,
        /// Replacement rules.
        rules: Vec<PermissionRuleValue>,
        /// Behavior the rules record.
        behavior: PermissionBehavior,
    },
    /// Remove matching rules.
    RemoveRules {
        /// Where to persist the change.
        destination: PermissionUpdateDestination,
        /// Rules to remove.
        rules: Vec<PermissionRuleValue>,
        /// Behavior the rules belong to.
        behavior: PermissionBehavior,
    },
    /// Switch the active permission mode.
    SetMode {
        /// Where to persist the change.
        destination: PermissionUpdateDestination,
        /// The new mode.
        mode: PermissionMode,
    },
    /// Add working directories to the permission scope.
    AddDirectories {
        /// Where to persist the change.
        destination: PermissionUpdateDestination,
        /// Directories to add.
        directories: Vec<String>,
    },
    /// Remove working directories from the permission scope.
    RemoveDirectories {
        /// Where to persist the change.
        destination: PermissionUpdateDestination,
        /// Directories to remove.
        directories: Vec<String>,
    },
}

/// Why a decision was reached, for display and auditing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum PermissionDecisionReason {
    /// A configured rule matched.
    Rule {
        /// The matching rule.
        rule: PermissionRule,
    },
    /// The active mode decided without prompting.
    Mode {
        /// The active mode.
        mode: PermissionMode,
    },
    /// A hook decided.
    Hook {
        /// Hook name.
        hook_name: String,
        /// Optional hook source.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        hook_source: Option<String>,
        /// Optional explanation.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
    /// A hard-coded safety check decided.
    SafetyCheck {
        /// Explanation of the check.
        reason: String,
        /// Whether the decision is classifier-approvable in auto mode.
        classifier_approvable: bool,
    },
    /// The path is outside the working directory.
    WorkingDir {
        /// Explanation.
        reason: String,
    },
    /// Anything not covered above.
    Other {
        /// Explanation.
        reason: String,
    },
}

/// The outcome of a permission check.
///
/// This combines TS's `PermissionDecision` (allow/ask/deny) and the
/// `passthrough` arm of `PermissionResult` into a single tagged enum. Use
/// [`PermissionResult::as_decision`] when only the three canonical decisions
/// are meaningful; `Passthrough` means "this layer has no opinion".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "behavior",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum PermissionResult {
    /// The call may proceed.
    Allow {
        /// Optional rewritten tool input.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        updated_input: Option<Value>,
        /// Whether the user edited the input while approving.
        #[serde(default, skip_serializing_if = "is_false")]
        user_modified: bool,
        /// Why the call was allowed.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        decision_reason: Option<PermissionDecisionReason>,
        /// The tool-use id this decision concerns.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tool_use_id: Option<String>,
        /// Optional feedback supplied with the approval.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        accept_feedback: Option<String>,
    },
    /// The user must be prompted.
    Ask {
        /// Prompt message.
        message: String,
        /// Optional rewritten tool input.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        updated_input: Option<Value>,
        /// Why a prompt is required.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        decision_reason: Option<PermissionDecisionReason>,
        /// Suggested rule updates to offer the user.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        suggestions: Vec<PermissionUpdate>,
        /// A path that is the subject of the request, if any.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        blocked_path: Option<String>,
    },
    /// The call is refused.
    Deny {
        /// Explanation shown to the model.
        message: String,
        /// Why the call was denied.
        decision_reason: PermissionDecisionReason,
        /// The tool-use id this decision concerns.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tool_use_id: Option<String>,
    },
    /// This layer has no opinion; an outer layer should decide.
    Passthrough {
        /// Explanation.
        message: String,
        /// Why no decision was made.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        decision_reason: Option<PermissionDecisionReason>,
        /// Suggested rule updates.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        suggestions: Vec<PermissionUpdate>,
        /// A path that is the subject of the request, if any.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        blocked_path: Option<String>,
    },
}

impl PermissionResult {
    /// Whether the call may proceed.
    pub const fn is_allow(&self) -> bool {
        matches!(self, PermissionResult::Allow { .. })
    }

    /// Whether the user must be prompted.
    pub const fn is_ask(&self) -> bool {
        matches!(self, PermissionResult::Ask { .. })
    }

    /// Whether the call was refused.
    pub const fn is_deny(&self) -> bool {
        matches!(self, PermissionResult::Deny { .. })
    }

    /// Whether this layer deferred the decision.
    pub const fn is_passthrough(&self) -> bool {
        matches!(self, PermissionResult::Passthrough { .. })
    }

    /// Return `Some(self)` for the canonical allow/ask/deny outcomes and
    /// `None` for `Passthrough`.
    pub fn as_decision(&self) -> Option<&PermissionResult> {
        match self {
            PermissionResult::Passthrough { .. } => None,
            other => Some(other),
        }
    }
}

/// Alias for the three canonical outcomes, matching the TS name.
pub type PermissionDecision = PermissionResult;

fn is_false(value: &bool) -> bool {
    !*value
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mode::PermissionMode;
    use crate::rules::{PermissionRuleSource, PermissionRuleValue};
    use serde_json::json;

    #[test]
    fn allow_serializes_with_behavior_tag() {
        let decision = PermissionResult::Allow {
            updated_input: None,
            user_modified: false,
            decision_reason: None,
            tool_use_id: Some("toolu_1".into()),
            accept_feedback: None,
        };
        let value = serde_json::to_value(&decision).unwrap();
        assert_eq!(value, json!({"behavior": "allow", "toolUseId": "toolu_1"}));
        assert!(decision.is_allow());
        let decoded: PermissionResult = serde_json::from_value(value).unwrap();
        assert_eq!(decoded, decision);
    }

    #[test]
    fn ask_carries_suggestions_and_reason() {
        let decision = PermissionResult::Ask {
            message: "Allow Bash?".into(),
            updated_input: None,
            decision_reason: Some(PermissionDecisionReason::Mode {
                mode: PermissionMode::Default,
            }),
            suggestions: vec![PermissionUpdate::AddRules {
                destination: PermissionUpdateDestination::Session,
                rules: vec![PermissionRuleValue::with_content("Bash", "git status")],
                behavior: PermissionBehavior::Allow,
            }],
            blocked_path: None,
        };
        let value = serde_json::to_value(&decision).unwrap();
        assert_eq!(value["behavior"], json!("ask"));
        assert_eq!(
            value["decisionReason"],
            json!({"type": "mode", "mode": "default"})
        );
        assert_eq!(value["suggestions"][0]["type"], json!("addRules"));
        let decoded: PermissionResult = serde_json::from_value(value).unwrap();
        assert_eq!(decoded, decision);
    }

    #[test]
    fn deny_reason_serializes_rule() {
        let decision = PermissionResult::Deny {
            message: "blocked".into(),
            decision_reason: PermissionDecisionReason::Rule {
                rule: PermissionRule::new(
                    PermissionRuleSource::LocalSettings,
                    PermissionBehavior::Deny,
                    PermissionRuleValue::tool("Bash"),
                ),
            },
            tool_use_id: None,
        };
        let value = serde_json::to_value(&decision).unwrap();
        assert_eq!(value["decisionReason"]["type"], json!("rule"));
        assert_eq!(
            value["decisionReason"]["rule"]["ruleBehavior"],
            json!("deny")
        );
    }

    #[test]
    fn passthrough_has_no_decision() {
        let decision = PermissionResult::Passthrough {
            message: "defer".into(),
            decision_reason: None,
            suggestions: Vec::new(),
            blocked_path: None,
        };
        assert!(decision.is_passthrough());
        assert!(decision.as_decision().is_none());
        assert_eq!(
            serde_json::to_value(&decision).unwrap(),
            json!({"behavior": "passthrough", "message": "defer"})
        );
    }
}
