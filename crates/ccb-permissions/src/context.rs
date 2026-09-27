//! Tool permission context (Phase 2 / T2.2).
//!
//! The immutable-ish snapshot handed to a tool when it asks whether it may run.
//! The rule engine that consults it is Phase 6 (T6.2); this module defines the
//! shape so settings, tools, and the UI agree on it.

use serde::{Deserialize, Serialize};

use crate::mode::PermissionMode;
use crate::rules::{PermissionBehavior, PermissionRuleSource, ToolPermissionRulesBySource};

/// A directory added to the permission scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdditionalWorkingDirectory {
    /// Absolute path.
    pub path: String,
    /// Where the addition came from.
    pub source: PermissionRuleSource,
}

/// Everything a permission check needs to know about the session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolPermissionContext {
    /// Active permission mode.
    #[serde(default)]
    pub mode: PermissionMode,
    /// Directories included in addition to the project root.
    #[serde(default)]
    pub additional_working_directories:
        std::collections::BTreeMap<String, AdditionalWorkingDirectory>,
    /// Rules that auto-allow.
    #[serde(default)]
    pub always_allow_rules: ToolPermissionRulesBySource,
    /// Rules that auto-deny.
    #[serde(default)]
    pub always_deny_rules: ToolPermissionRulesBySource,
    /// Rules that force a prompt.
    #[serde(default)]
    pub always_ask_rules: ToolPermissionRulesBySource,
    /// Whether `bypassPermissions` may be selected in this session.
    #[serde(default)]
    pub is_bypass_permissions_mode_available: bool,
    /// Whether prompts should be auto-denied (e.g. non-interactive agents).
    #[serde(default)]
    pub should_avoid_permission_prompts: bool,
    /// The mode to restore when leaving plan mode.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pre_plan_mode: Option<PermissionMode>,
}

impl Default for ToolPermissionContext {
    fn default() -> Self {
        ToolPermissionContext {
            mode: PermissionMode::Default,
            additional_working_directories: std::collections::BTreeMap::new(),
            always_allow_rules: ToolPermissionRulesBySource::new(),
            always_deny_rules: ToolPermissionRulesBySource::new(),
            always_ask_rules: ToolPermissionRulesBySource::new(),
            // Matches the TS `getEmptyToolPermissionContext` default.
            is_bypass_permissions_mode_available: true,
            should_avoid_permission_prompts: false,
            pre_plan_mode: None,
        }
    }
}

impl ToolPermissionContext {
    /// A context with every collection empty and `default` mode.
    pub fn empty() -> Self {
        Self::default()
    }

    /// The rule list for a given behavior.
    pub fn rules_for(&self, behavior: PermissionBehavior) -> &ToolPermissionRulesBySource {
        match behavior {
            PermissionBehavior::Allow => &self.always_allow_rules,
            PermissionBehavior::Deny => &self.always_deny_rules,
            PermissionBehavior::Ask => &self.always_ask_rules,
        }
    }

    /// Whether prompts are suppressed for this session.
    pub fn suppresses_prompts(&self) -> bool {
        self.mode.is_bypass() || self.should_avoid_permission_prompts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn empty_context_matches_ts_default() {
        let ctx = ToolPermissionContext::empty();
        assert_eq!(ctx.mode, PermissionMode::Default);
        assert!(ctx.is_bypass_permissions_mode_available);
        assert!(!ctx.should_avoid_permission_prompts);
        assert!(ctx.rules_for(PermissionBehavior::Allow).is_empty());
        assert!(!ctx.suppresses_prompts());
    }

    #[test]
    fn context_roundtrips_and_omits_optional_fields() {
        let ctx = ToolPermissionContext::empty();
        let value = serde_json::to_value(&ctx).unwrap();
        assert_eq!(
            value,
            json!({
                "mode": "default",
                "additionalWorkingDirectories": {},
                "alwaysAllowRules": {},
                "alwaysDenyRules": {},
                "alwaysAskRules": {},
                "isBypassPermissionsModeAvailable": true,
                "shouldAvoidPermissionPrompts": false
            })
        );
        let decoded: ToolPermissionContext = serde_json::from_value(value).unwrap();
        assert_eq!(decoded, ctx);
    }

    #[test]
    fn bypass_suppresses_prompts() {
        let ctx = ToolPermissionContext {
            mode: PermissionMode::BypassPermissions,
            ..ToolPermissionContext::empty()
        };
        assert!(ctx.suppresses_prompts());
    }
}
