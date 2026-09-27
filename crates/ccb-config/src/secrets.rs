//! API key helpers and key storage (T1.2).
//!
//! Two mechanisms are kept from the TS implementation:
//! * `apiKeyHelper` — a shell command whose stdout is the key.
//! * macOS Keychain, with `~/.claude.json` (`primaryApiKey`) as the portable
//!   fallback. OAuth-backed storage is intentionally gone (decision D1).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{Map, Value};

use crate::error::{ConfigError, Result};

/// Run a configured `apiKeyHelper` command and return the trimmed stdout.
pub fn run_api_key_helper(command: &str) -> Result<String> {
    let output = if cfg!(windows) {
        Command::new("cmd").args(["/C", command]).output()
    } else {
        Command::new("sh").args(["-c", command]).output()
    }
    .map_err(|err| ConfigError::ApiKeyHelper {
        command: command.to_owned(),
        message: err.to_string(),
    })?;

    if !output.status.success() {
        return Err(ConfigError::ApiKeyHelper {
            command: command.to_owned(),
            message: format!(
                "exited with status {}{}",
                output.status,
                stderr_suffix(&output.stderr)
            ),
        });
    }

    let key = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if key.is_empty() {
        return Err(ConfigError::ApiKeyHelper {
            command: command.to_owned(),
            message: "produced no output".to_owned(),
        });
    }
    Ok(key)
}

fn stderr_suffix(stderr: &[u8]) -> String {
    let text = String::from_utf8_lossy(stderr);
    let text = text.trim();
    if text.is_empty() {
        String::new()
    } else {
        format!(": {text}")
    }
}

/// A place an API key can be read from, abstracted so tests stay hermetic.
pub trait SecretLookup: Send + Sync {
    /// Return a stored key, if any.
    fn api_key(&self) -> Option<String>;
}

/// A store that never finds anything (used for `--bare` and in tests).
#[derive(Debug, Default, Clone, Copy)]
pub struct NoSecrets;

impl SecretLookup for NoSecrets {
    fn api_key(&self) -> Option<String> {
        None
    }
}

/// The real platform key store.
#[derive(Debug, Clone)]
pub struct ApiKeyStore {
    claude_json: PathBuf,
}

impl ApiKeyStore {
    /// Create a store backed by the given `~/.claude.json`.
    pub fn new(claude_json: impl Into<PathBuf>) -> Self {
        Self {
            claude_json: claude_json.into(),
        }
    }

    /// Path of the portable fallback file.
    pub fn claude_json(&self) -> &Path {
        &self.claude_json
    }

    /// Read `primaryApiKey` from `~/.claude.json`.
    fn read_file_key(&self) -> Option<String> {
        let text = fs::read_to_string(&self.claude_json).ok()?;
        let value: Value = serde_json::from_str(&text).ok()?;
        value
            .get("primaryApiKey")
            .and_then(Value::as_str)
            .map(str::to_owned)
    }

    /// Persist the key, updating `~/.claude.json` and (on macOS) the Keychain.
    pub fn save(&self, key: &str) -> Result<()> {
        #[cfg(target_os = "macos")]
        {
            save_macos_keychain(key)?;
        }

        let mut root = self.read_root();
        root.insert("primaryApiKey".to_owned(), Value::String(key.to_owned()));
        let text = serde_json::to_string_pretty(&Value::Object(root)).map_err(|err| {
            ConfigError::SecretStore {
                message: err.to_string(),
            }
        })?;

        if let Some(parent) = self.claude_json.parent() {
            fs::create_dir_all(parent).map_err(|err| ConfigError::SecretStore {
                message: format!("creating {}: {err}", parent.display()),
            })?;
        }
        fs::write(&self.claude_json, text).map_err(|err| ConfigError::SecretStore {
            message: format!("writing {}: {err}", self.claude_json.display()),
        })?;
        restrict_permissions(&self.claude_json);
        Ok(())
    }

    /// Remove the stored key from both locations.
    pub fn delete(&self) -> Result<()> {
        #[cfg(target_os = "macos")]
        {
            delete_macos_keychain();
        }

        if self.claude_json.exists() {
            let mut root = self.read_root();
            if root.remove("primaryApiKey").is_some() {
                let text = serde_json::to_string_pretty(&Value::Object(root)).map_err(|err| {
                    ConfigError::SecretStore {
                        message: err.to_string(),
                    }
                })?;
                fs::write(&self.claude_json, text).map_err(|err| ConfigError::SecretStore {
                    message: format!("writing {}: {err}", self.claude_json.display()),
                })?;
            }
        }
        Ok(())
    }

    fn read_root(&self) -> Map<String, Value> {
        match fs::read_to_string(&self.claude_json)
            .ok()
            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        {
            Some(Value::Object(map)) => map,
            _ => Map::new(),
        }
    }
}

impl SecretLookup for ApiKeyStore {
    fn api_key(&self) -> Option<String> {
        #[cfg(target_os = "macos")]
        {
            if let Some(key) = read_macos_keychain() {
                return Some(key);
            }
        }
        self.read_file_key()
    }
}

#[cfg(unix)]
fn restrict_permissions(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(meta) = fs::metadata(path) {
        let mut perms = meta.permissions();
        perms.set_mode(0o600);
        let _ = fs::set_permissions(path, perms);
    }
}

#[cfg(not(unix))]
fn restrict_permissions(_path: &Path) {}

#[cfg(target_os = "macos")]
const KEYCHAIN_SERVICE: &str = "Claude Code";

#[cfg(target_os = "macos")]
fn read_macos_keychain() -> Option<String> {
    let user = std::env::var("USER").unwrap_or_else(|_| "claude-code-user".to_owned());
    let output = Command::new("security")
        .args([
            "find-generic-password",
            "-a",
            &user,
            "-w",
            "-s",
            KEYCHAIN_SERVICE,
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let key = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    (!key.is_empty()).then_some(key)
}

#[cfg(target_os = "macos")]
fn save_macos_keychain(key: &str) -> Result<()> {
    let user = std::env::var("USER").unwrap_or_else(|_| "claude-code-user".to_owned());
    let status = Command::new("security")
        .args([
            "add-generic-password",
            "-U",
            "-a",
            &user,
            "-w",
            key,
            "-s",
            KEYCHAIN_SERVICE,
        ])
        .status();
    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(ConfigError::SecretStore {
            message: format!("macOS `security add-generic-password` exited with {s}"),
        }),
        Err(err) => Err(ConfigError::SecretStore {
            message: err.to_string(),
        }),
    }
}

#[cfg(target_os = "macos")]
fn delete_macos_keychain() {
    let user = std::env::var("USER").unwrap_or_else(|_| "claude-code-user".to_owned());
    let _ = Command::new("security")
        .args([
            "delete-generic-password",
            "-a",
            &user,
            "-s",
            KEYCHAIN_SERVICE,
        ])
        .output();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helper_reports_nonzero_exit() {
        let err = run_api_key_helper("exit 3").unwrap_err();
        assert!(matches!(err, ConfigError::ApiKeyHelper { .. }));
    }

    #[test]
    fn helper_returns_trimmed_stdout() {
        let key = run_api_key_helper("printf '  sk-abc\\n'").unwrap();
        assert_eq!(key, "sk-abc");
    }

    #[test]
    fn file_store_round_trips() {
        let dir = std::env::temp_dir().join(format!("ccb-store-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let store = ApiKeyStore::new(dir.join(".claude.json"));
        assert!(store.api_key().is_none());
        store.save("sk-test").unwrap();
        #[cfg(not(target_os = "macos"))]
        assert_eq!(store.api_key().as_deref(), Some("sk-test"));
        store.delete().unwrap();
        #[cfg(not(target_os = "macos"))]
        assert!(store.api_key().is_none());
        let _ = fs::remove_dir_all(&dir);
    }
}
