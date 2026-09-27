//! `settings.json` model, layered loading, and merging (T1.2).
//!
//! Layers are applied in increasing priority:
//! `user < project < local < explicit --settings`.
//! `env` maps merge key-by-key; scalars are overridden by later layers.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::Path;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::{ConfigError, Result};
use crate::paths::{expand_tilde, ConfigPaths};
use crate::provider::Protocol;

/// Which settings layers to read (mirrors `--setting-sources`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SettingSource {
    /// `$CLAUDE_CONFIG_DIR/settings.json`.
    User,
    /// `<cwd>/.claude/settings.json`.
    Project,
    /// `<cwd>/.claude/settings.local.json`.
    Local,
}

impl SettingSource {
    /// Canonical name used by `--setting-sources`.
    pub const fn as_str(self) -> &'static str {
        match self {
            SettingSource::User => "user",
            SettingSource::Project => "project",
            SettingSource::Local => "local",
        }
    }

    /// Parse a comma-separated list such as `user,project`.
    pub fn parse_list(raw: &str) -> Result<Vec<Self>> {
        let mut sources = Vec::new();
        for part in raw.split(',').map(str::trim).filter(|p| !p.is_empty()) {
            let source = part.parse::<SettingSource>()?;
            if !sources.contains(&source) {
                sources.push(source);
            }
        }
        Ok(sources)
    }
}

impl FromStr for SettingSource {
    type Err = ConfigError;

    fn from_str(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "user" => Ok(SettingSource::User),
            "project" => Ok(SettingSource::Project),
            "local" => Ok(SettingSource::Local),
            other => Err(ConfigError::UnknownSettingSource {
                value: other.to_owned(),
            }),
        }
    }
}

impl fmt::Display for SettingSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Every layer, in increasing priority.
pub const ALL_SETTING_SOURCES: [SettingSource; 3] = [
    SettingSource::User,
    SettingSource::Project,
    SettingSource::Local,
];

/// A single settings document.
///
/// Only the fields Phase 1 understands are typed; everything else is preserved
/// in `extra` so later phases can read it without a schema migration (and
/// round-tripping a file does not drop unknown keys).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// `modelType`: `anthropic` or `openai`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_type: Option<Protocol>,
    /// Default model identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Environment variables to apply at runtime.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
    /// Shell command whose stdout is the API key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key_helper: Option<String>,
    /// Optional allow-list of model identifiers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub available_models: Option<Vec<String>>,
    /// Any other settings this phase does not model yet.
    #[serde(flatten, default)]
    pub extra: Map<String, Value>,
}

impl Settings {
    /// Parse a settings document from JSON text.
    pub fn parse(text: &str, path: &Path) -> Result<Self> {
        serde_json::from_str(text).map_err(|source| ConfigError::ParseSettings {
            path: path.to_path_buf(),
            source,
        })
    }

    /// Parse an inline `--settings` JSON string.
    pub fn parse_inline(text: &str) -> Result<Self> {
        serde_json::from_str(text).map_err(|source| ConfigError::ParseInlineSettings { source })
    }

    /// Load a settings file, returning `None` when it does not exist.
    pub fn load(path: &Path) -> Result<Option<Self>> {
        match fs::read_to_string(path) {
            Ok(text) => Self::parse(&text, path).map(Some),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(source) => Err(ConfigError::ReadSettings {
                path: path.to_path_buf(),
                source,
            }),
        }
    }

    /// Merge `other` on top of `self` (other wins).
    pub fn merge(&mut self, other: Settings) {
        if other.model_type.is_some() {
            self.model_type = other.model_type;
        }
        if other.model.is_some() {
            self.model = other.model;
        }
        if other.api_key_helper.is_some() {
            self.api_key_helper = other.api_key_helper;
        }
        if other.available_models.is_some() {
            self.available_models = other.available_models;
        }
        self.env.extend(other.env);
        self.extra.extend(other.extra);
    }
}

/// Where an explicit `--settings` value came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsArg {
    /// A path on disk (possibly `~/...`).
    Path(String),
    /// An inline JSON object.
    Inline(String),
}

impl SettingsArg {
    /// Classify a `--settings` value: a leading `{` means inline JSON.
    pub fn classify(value: &str) -> Self {
        if value.trim_start().starts_with('{') {
            SettingsArg::Inline(value.to_owned())
        } else {
            SettingsArg::Path(value.to_owned())
        }
    }

    /// Load this argument into a [`Settings`] document.
    pub fn load(&self, home: &Path) -> Result<Settings> {
        match self {
            SettingsArg::Inline(json) => Settings::parse_inline(json),
            SettingsArg::Path(raw) => {
                let path = expand_tilde(raw, home);
                Settings::load(&path).map(|opt| opt.unwrap_or_default())
            }
        }
    }
}

/// Load and merge every settings layer for a session.
///
/// Missing files are skipped silently. Only the layers listed here are read;
/// policy/managed settings are intentionally out of scope (`Plan/03-...` §5).
pub fn load_layers(paths: &ConfigPaths, explicit: &[SettingsArg]) -> Result<Settings> {
    load_layers_from(paths, &ALL_SETTING_SOURCES, explicit)
}

/// Like [`load_layers`], but restricted to the given sources (in priority
/// order, lowest first).
pub fn load_layers_from(
    paths: &ConfigPaths,
    sources: &[SettingSource],
    explicit: &[SettingsArg],
) -> Result<Settings> {
    let mut merged = Settings::default();

    for source in sources {
        let path = match source {
            SettingSource::User => paths.user_settings(),
            SettingSource::Project => paths.project_settings(),
            SettingSource::Local => paths.local_settings(),
        };
        if let Some(layer) = Settings::load(&path)? {
            merged.merge(layer);
        }
    }

    for arg in explicit {
        merged.merge(arg.load(&paths.home)?);
    }

    Ok(merged)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn later_layers_override_scalars_and_merge_env() {
        let mut base = Settings {
            model: Some("a".into()),
            env: BTreeMap::from([("A".into(), "1".into()), ("B".into(), "1".into())]),
            ..Settings::default()
        };
        let override_ = Settings {
            model: Some("b".into()),
            env: BTreeMap::from([("B".into(), "2".into())]),
            ..Settings::default()
        };
        base.merge(override_);
        assert_eq!(base.model.as_deref(), Some("b"));
        assert_eq!(base.env.get("A").map(String::as_str), Some("1"));
        assert_eq!(base.env.get("B").map(String::as_str), Some("2"));
    }

    #[test]
    fn parses_model_type() {
        let settings = Settings::parse(
            r#"{"modelType":"openai","model":"deepseek-chat","env":{"OPENAI_BASE_URL":"https://x"}}"#,
            Path::new("test.json"),
        )
        .unwrap();
        assert_eq!(settings.model_type, Some(Protocol::OpenAi));
        assert_eq!(settings.model.as_deref(), Some("deepseek-chat"));
    }

    #[test]
    fn preserves_unknown_keys() {
        let settings =
            Settings::parse(r#"{"permissions":{"allow":["Bash"]}}"#, Path::new("t.json")).unwrap();
        assert!(settings.extra.contains_key("permissions"));
    }

    #[test]
    fn classifies_settings_argument() {
        assert_eq!(
            SettingsArg::classify(r#"{"model":"x"}"#),
            SettingsArg::Inline(r#"{"model":"x"}"#.into())
        );
        assert_eq!(
            SettingsArg::classify("~/s.json"),
            SettingsArg::Path("~/s.json".into())
        );
    }

    #[test]
    fn parses_setting_source_lists() {
        assert_eq!(
            SettingSource::parse_list("user, project").unwrap(),
            vec![SettingSource::User, SettingSource::Project]
        );
        assert!(SettingSource::parse_list("managed").is_err());
    }

    #[test]
    fn loads_only_selected_layers_from_disk() {
        let root = std::env::temp_dir().join(format!(
            "ccb-layers-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&root);
        let project = root.join("repo");
        fs::create_dir_all(project.join(".claude")).unwrap();
        fs::create_dir_all(root.join("home/.claude")).unwrap();
        fs::write(
            root.join("home/.claude/settings.json"),
            r#"{"model":"user-model"}"#,
        )
        .unwrap();
        fs::write(
            project.join(".claude/settings.json"),
            r#"{"model":"project-model"}"#,
        )
        .unwrap();

        let paths = ConfigPaths::new(root.join("home"), root.join("home/.claude"), &project);
        let project_only = load_layers_from(&paths, &[SettingSource::Project], &[]).unwrap();
        assert_eq!(project_only.model.as_deref(), Some("project-model"));

        let all = load_layers(&paths, &[]).unwrap();
        assert_eq!(all.model.as_deref(), Some("project-model"));
        let _ = fs::remove_dir_all(&root);
    }
}
