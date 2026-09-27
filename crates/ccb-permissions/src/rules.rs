//! Permission rules and behaviors (Phase 2 / T2.2).
//!
//! Mirrors `src/types/permissions.ts` after trimming OAuth/classifier-only
//! sources. Rule *matching* is implemented in Phase 6 (T6.2); this module is
//! the data contract shared by settings, the engine, and the UI.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// What a rule (or decision) does when it applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PermissionBehavior {
    /// Auto-approve the matching call.
    Allow,
    /// Auto-deny the matching call.
    Deny,
    /// Always prompt, even if an allow rule would otherwise match.
    Ask,
}

/// Where a rule came from.
///
/// The `policySettings` / `flagSettings` sources are retained as inert
/// variants so existing settings files still parse; the plan removes managed
/// policy loading (Plan/03-configuration.md §5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionRuleSource {
    /// `~/.claude/settings.json`.
    UserSettings,
    /// `<project>/.claude/settings.json`.
    ProjectSettings,
    /// `<project>/.claude/settings.local.json`.
    LocalSettings,
    /// Explicit `--settings` file.
    FlagSettings,
    /// Policy/managed settings.
    PolicySettings,
    /// A `--allowedTools` style CLI argument.
    CliArg,
    /// A runtime command (e.g. `/permissions`).
    Command,
    /// Granted for the current session only.
    Session,
}

impl PermissionRuleSource {
    /// The wire/settings spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            PermissionRuleSource::UserSettings => "userSettings",
            PermissionRuleSource::ProjectSettings => "projectSettings",
            PermissionRuleSource::LocalSettings => "localSettings",
            PermissionRuleSource::FlagSettings => "flagSettings",
            PermissionRuleSource::PolicySettings => "policySettings",
            PermissionRuleSource::CliArg => "cliArg",
            PermissionRuleSource::Command => "command",
            PermissionRuleSource::Session => "session",
        }
    }
}

/// A single rule target: a tool name plus optional rule content
/// (e.g. a bash command prefix or a path glob).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionRuleValue {
    /// Tool the rule applies to.
    pub tool_name: String,
    /// Optional tool-specific selector.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rule_content: Option<String>,
}

impl PermissionRuleValue {
    /// Build a rule value with no content selector.
    pub fn tool(tool_name: impl Into<String>) -> Self {
        PermissionRuleValue {
            tool_name: tool_name.into(),
            rule_content: None,
        }
    }

    /// Build a rule value with a content selector.
    pub fn with_content(tool_name: impl Into<String>, rule_content: impl Into<String>) -> Self {
        PermissionRuleValue {
            tool_name: tool_name.into(),
            rule_content: Some(rule_content.into()),
        }
    }
}

/// A rule together with its origin and behavior.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionRule {
    /// Where the rule was defined.
    pub source: PermissionRuleSource,
    /// What applying the rule does.
    pub rule_behavior: PermissionBehavior,
    /// Which tool/content the rule matches.
    pub rule_value: PermissionRuleValue,
}

impl PermissionRule {
    /// Build a rule.
    pub fn new(
        source: PermissionRuleSource,
        rule_behavior: PermissionBehavior,
        rule_value: PermissionRuleValue,
    ) -> Self {
        PermissionRule {
            source,
            rule_behavior,
            rule_value,
        }
    }
}

/// Rules grouped by their source, as stored in settings.
///
/// A `BTreeMap` keeps serialization deterministic (important for settings
/// round-trips and golden transcripts).
pub type ToolPermissionRulesBySource = BTreeMap<PermissionRuleSource, Vec<String>>;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn rule_uses_camel_case_fields() {
        let rule = PermissionRule::new(
            PermissionRuleSource::UserSettings,
            PermissionBehavior::Allow,
            PermissionRuleValue::with_content("Bash", "git status"),
        );
        let value = serde_json::to_value(&rule).unwrap();
        assert_eq!(
            value,
            json!({
                "source": "userSettings",
                "ruleBehavior": "allow",
                "ruleValue": {"toolName": "Bash", "ruleContent": "git status"}
            })
        );
        let decoded: PermissionRule = serde_json::from_value(value).unwrap();
        assert_eq!(decoded, rule);
    }

    #[test]
    fn rule_value_content_is_optional() {
        let value = serde_json::to_value(PermissionRuleValue::tool("Read")).unwrap();
        assert_eq!(value, json!({"toolName": "Read"}));
        let decoded: PermissionRuleValue = serde_json::from_value(value).unwrap();
        assert_eq!(decoded.rule_content, None);
    }

    #[test]
    fn rules_group_by_source_serialize_deterministically() {
        let mut rules = ToolPermissionRulesBySource::new();
        rules.insert(PermissionRuleSource::Session, vec!["Read".into()]);
        rules.insert(
            PermissionRuleSource::UserSettings,
            vec!["Bash(git:*)".into()],
        );
        let value = serde_json::to_value(&rules).unwrap();
        assert_eq!(
            value,
            json!({"session": ["Read"], "userSettings": ["Bash(git:*)"]})
        );
    }
}
