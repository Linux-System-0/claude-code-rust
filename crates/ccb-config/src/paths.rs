//! Filesystem locations used by the configuration layer (T1.5).
//!
//! Everything is derived from an explicit [`ConfigPaths`] value so tests can
//! point at a temporary directory instead of the real user profile.

use std::env;
use std::path::{Path, PathBuf};

use crate::error::{ConfigError, Result};

/// Resolved platform paths for a session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigPaths {
    /// The user's home directory.
    pub home: PathBuf,
    /// The Claude configuration directory (`$CLAUDE_CONFIG_DIR` or `~/.claude`).
    pub config_dir: PathBuf,
    /// The working directory the CLI was launched from.
    pub cwd: PathBuf,
    /// The legacy global config file (`~/.claude.json`).
    pub claude_json: PathBuf,
}

impl ConfigPaths {
    /// Discover paths from the environment and the given working directory.
    pub fn discover(cwd: impl Into<PathBuf>) -> Result<Self> {
        let home = home_dir()?;
        let config_dir = match env::var_os("CLAUDE_CONFIG_DIR") {
            Some(dir) if !dir.is_empty() => PathBuf::from(dir),
            _ => home.join(".claude"),
        };
        let claude_json = home.join(".claude.json");
        Ok(Self {
            home,
            config_dir,
            cwd: cwd.into(),
            claude_json,
        })
    }

    /// Explicit constructor, primarily for tests.
    pub fn new(
        home: impl Into<PathBuf>,
        config_dir: impl Into<PathBuf>,
        cwd: impl Into<PathBuf>,
    ) -> Self {
        let home = home.into();
        let claude_json = home.join(".claude.json");
        Self {
            home,
            config_dir: config_dir.into(),
            cwd: cwd.into(),
            claude_json,
        }
    }

    /// User-level settings: `$CLAUDE_CONFIG_DIR/settings.json`.
    pub fn user_settings(&self) -> PathBuf {
        self.config_dir.join("settings.json")
    }

    /// Project-level settings: `<cwd>/.claude/settings.json`.
    pub fn project_settings(&self) -> PathBuf {
        self.cwd.join(".claude").join("settings.json")
    }

    /// Local (git-ignored) settings: `<cwd>/.claude/settings.local.json`.
    pub fn local_settings(&self) -> PathBuf {
        self.cwd.join(".claude").join("settings.local.json")
    }

    /// Directory for logs: `$CLAUDE_CONFIG_DIR/logs`.
    pub fn log_dir(&self) -> PathBuf {
        self.config_dir.join("logs")
    }
}

/// Determine the user's home directory without pulling in a heftier crate.
fn home_dir() -> Result<PathBuf> {
    if let Some(dir) = dirs::home_dir() {
        return Ok(dir);
    }
    // `dirs` can fail in stripped-down containers; fall back to $HOME.
    match env::var_os("HOME") {
        Some(home) if !home.is_empty() => Ok(PathBuf::from(home)),
        _ => Err(ConfigError::NoHomeDir),
    }
}

/// Expand a leading `~/` using the resolved home directory.
///
/// Used for values such as `--settings ~/my.json` and `apiKeyHelper` paths.
pub fn expand_tilde(path: &str, home: &Path) -> PathBuf {
    if path == "~" {
        return home.to_path_buf();
    }
    if let Some(rest) = path.strip_prefix("~/").or_else(|| path.strip_prefix("~\\")) {
        return home.join(rest);
    }
    PathBuf::from(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_paths_are_scoped() {
        let paths = ConfigPaths::new("/home/u", "/home/u/.claude", "/work/repo");
        assert_eq!(
            paths.user_settings(),
            PathBuf::from("/home/u/.claude/settings.json")
        );
        assert_eq!(
            paths.project_settings(),
            PathBuf::from("/work/repo/.claude/settings.json")
        );
        assert_eq!(
            paths.local_settings(),
            PathBuf::from("/work/repo/.claude/settings.local.json")
        );
    }

    #[test]
    fn expands_tilde() {
        let home = Path::new("/home/u");
        assert_eq!(expand_tilde("~/a/b", home), PathBuf::from("/home/u/a/b"));
        assert_eq!(expand_tilde("~", home), PathBuf::from("/home/u"));
        assert_eq!(expand_tilde("/abs", home), PathBuf::from("/abs"));
    }
}
