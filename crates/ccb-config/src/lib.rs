//! Layered `settings.json`, environment, and secret resolution.
//!
//! Phase 1 / T1.2, T1.3, T1.5. The crate intentionally reads no global state in
//! its pure APIs: callers pass a [`ConfigPaths`], an environment snapshot, and
//! an explicit list of `--settings` arguments. That keeps tests hermetic and
//! makes precedence rules easy to reason about.
//!
//! ```no_run
//! use ccb_config::{ConfigPaths, ConfigOverrides, Settings, resolve, ApiKeyStore, load_layers};
//!
//! # fn main() -> ccb_config::Result<()> {
//! let paths = ConfigPaths::discover(std::env::current_dir().unwrap())?;
//! let settings = load_layers(&paths, &[])?;
//! let env: std::collections::BTreeMap<_, _> = std::env::vars().collect();
//! let store = ApiKeyStore::new(paths.claude_json.clone());
//! let resolved = resolve(&settings, &env, &ConfigOverrides::default(), &store)?;
//! println!("{} -> {}", resolved.protocol, resolved.base_url);
//! # Ok(())
//! # }
//! ```

#![forbid(unsafe_code)]

mod error;
mod paths;
mod platform;
mod provider;
mod resolve;
mod secrets;
mod settings;

pub use error::{ConfigError, Result, NO_ENDPOINT_MESSAGE};
pub use paths::{expand_tilde, ConfigPaths};
pub use platform::{detect_shell, is_macos, is_windows, which, Shell};
pub use provider::Protocol;
pub use resolve::{resolve, ApiKeySource, ConfigOverrides, ResolvedConfig};
pub use secrets::{run_api_key_helper, ApiKeyStore, NoSecrets, SecretLookup};
pub use settings::{
    load_layers, load_layers_from, SettingSource, Settings, SettingsArg, ALL_SETTING_SOURCES,
};
