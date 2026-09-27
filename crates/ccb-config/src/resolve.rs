//! Resolving layered settings + environment into a concrete endpoint (T1.3).
//!
//! Priority (highest first), per `Plan/03-configuration.md` §2:
//! `CLI flags > process env > project settings env > user settings env > none`.
//!
//! There is no compiled-in default endpoint (decision D3): when nothing is
//! configured the resolver returns [`ConfigError::NoEndpoint`] with an
//! actionable message.

use std::collections::BTreeMap;

use crate::error::{ConfigError, Result};
use crate::provider::Protocol;
use crate::secrets::{run_api_key_helper, SecretLookup};
use crate::settings::Settings;

/// Values supplied directly on the command line.
#[derive(Debug, Clone, Default)]
pub struct ConfigOverrides {
    /// `--provider` (not present in the TS CLI, but required once endpoints are
    /// user-supplied).
    pub protocol: Option<Protocol>,
    /// `--base-url`.
    pub base_url: Option<String>,
    /// `--api-key`.
    pub api_key: Option<String>,
    /// `--model`.
    pub model: Option<String>,
}

/// Where the resolved API key came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiKeySource {
    /// `--api-key`.
    Flag,
    /// `apiKeyHelper` in settings.
    ApiKeyHelper,
    /// A process environment variable.
    Env(String),
    /// The `env` block of a settings layer.
    Settings(String),
    /// The platform key store (`~/.claude.json` / Keychain).
    KeyStore,
}

/// A fully resolved endpoint, ready to hand to a protocol adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedConfig {
    /// Selected protocol.
    pub protocol: Protocol,
    /// Explicit base URL (no default).
    pub base_url: String,
    /// API key / bearer token.
    pub api_key: String,
    /// Model identifier, if configured.
    pub model: Option<String>,
    /// Provenance of [`Self::api_key`], for diagnostics.
    pub api_key_source: ApiKeySource,
}

/// Resolve configuration from settings, a snapshot of the process environment,
/// CLI overrides, and a secret store.
///
/// `env` is passed explicitly (rather than read from `std::env`) so tests are
/// deterministic.
pub fn resolve(
    settings: &Settings,
    env: &BTreeMap<String, String>,
    overrides: &ConfigOverrides,
    secrets: &dyn SecretLookup,
) -> Result<ResolvedConfig> {
    let protocol = resolve_protocol(settings, env, overrides)?;

    let base_url = first_non_empty([
        overrides.base_url.as_deref(),
        env.get(protocol.base_url_var()).map(String::as_str),
        settings
            .env
            .get(protocol.base_url_var())
            .map(String::as_str),
    ])
    .ok_or(ConfigError::MissingBaseUrl {
        protocol: protocol.as_str(),
        var: protocol.base_url_var(),
    })?;

    let (api_key, api_key_source) = resolve_api_key(settings, env, overrides, secrets, protocol)?;

    let model = overrides.model.clone().or_else(|| settings.model.clone());

    Ok(ResolvedConfig {
        protocol,
        base_url,
        api_key,
        model,
        api_key_source,
    })
}

fn resolve_protocol(
    settings: &Settings,
    env: &BTreeMap<String, String>,
    overrides: &ConfigOverrides,
) -> Result<Protocol> {
    if let Some(protocol) = overrides.protocol.or(settings.model_type) {
        return Ok(protocol);
    }

    let anthropic = looks_configured(Protocol::Anthropic, settings, env);
    let openai = looks_configured(Protocol::OpenAi, settings, env);

    match (anthropic, openai) {
        (true, false) => Ok(Protocol::Anthropic),
        (false, true) => Ok(Protocol::OpenAi),
        (true, true) => Err(ConfigError::AmbiguousProtocol),
        (false, false) => Err(ConfigError::NoEndpoint),
    }
}

fn resolve_api_key(
    settings: &Settings,
    env: &BTreeMap<String, String>,
    overrides: &ConfigOverrides,
    secrets: &dyn SecretLookup,
    protocol: Protocol,
) -> Result<(String, ApiKeySource)> {
    if let Some(key) = non_empty(overrides.api_key.as_deref()) {
        return Ok((key.to_owned(), ApiKeySource::Flag));
    }

    if let Some(helper) = non_empty(settings.api_key_helper.as_deref()) {
        let key = run_api_key_helper(helper)?;
        return Ok((key, ApiKeySource::ApiKeyHelper));
    }

    for var in protocol.api_key_vars() {
        if let Some(key) = non_empty(env.get(*var).map(String::as_str)) {
            return Ok((key.to_owned(), ApiKeySource::Env((*var).to_owned())));
        }
    }
    for var in protocol.api_key_vars() {
        if let Some(key) = non_empty(settings.env.get(*var).map(String::as_str)) {
            return Ok((key.to_owned(), ApiKeySource::Settings((*var).to_owned())));
        }
    }

    if let Some(key) = secrets
        .api_key()
        .and_then(|key| non_empty(Some(&key)).map(str::to_owned))
    {
        return Ok((key, ApiKeySource::KeyStore));
    }

    Err(ConfigError::MissingApiKey {
        protocol: protocol.as_str(),
        vars: protocol.api_key_vars_display(),
    })
}

/// Does any environment surface mention this protocol?
fn looks_configured(
    protocol: Protocol,
    settings: &Settings,
    env: &BTreeMap<String, String>,
) -> bool {
    let has = |key: &str| {
        non_empty(env.get(key).map(String::as_str)).is_some()
            || non_empty(settings.env.get(key).map(String::as_str)).is_some()
    };
    has(protocol.base_url_var()) || protocol.api_key_vars().iter().any(|var| has(var))
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn first_non_empty<const N: usize>(values: [Option<&str>; N]) -> Option<String> {
    values
        .into_iter()
        .find_map(non_empty)
        .map(|value| value.trim_end_matches('/').to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secrets::NoSecrets;

    fn settings_with(model_type: Option<Protocol>, env: &[(&str, &str)]) -> Settings {
        Settings {
            model_type,
            env: env
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                .collect(),
            ..Settings::default()
        }
    }

    fn map(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect()
    }

    #[test]
    fn missing_everything_is_actionable() {
        let err = resolve(
            &Settings::default(),
            &BTreeMap::new(),
            &ConfigOverrides::default(),
            &NoSecrets,
        )
        .unwrap_err();
        assert!(matches!(err, ConfigError::NoEndpoint));
        assert!(err.to_string().contains("No API endpoint configured."));
        assert!(err.to_string().contains("ANTHROPIC_BASE_URL"));
        assert!(err.to_string().contains("OPENAI_BASE_URL"));
    }

    #[test]
    fn infers_protocol_from_environment() {
        let env = map(&[
            ("OPENAI_BASE_URL", "https://api.example.com/v1"),
            ("OPENAI_API_KEY", "sk-x"),
        ]);
        let resolved = resolve(
            &Settings::default(),
            &env,
            &ConfigOverrides::default(),
            &NoSecrets,
        )
        .unwrap();
        assert_eq!(resolved.protocol, Protocol::OpenAi);
        assert_eq!(resolved.base_url, "https://api.example.com/v1");
        assert_eq!(resolved.api_key, "sk-x");
        assert_eq!(
            resolved.api_key_source,
            ApiKeySource::Env("OPENAI_API_KEY".into())
        );
    }

    #[test]
    fn both_protocols_is_ambiguous() {
        let env = map(&[
            ("ANTHROPIC_BASE_URL", "https://a"),
            ("OPENAI_BASE_URL", "https://b"),
        ]);
        let err = resolve(
            &Settings::default(),
            &env,
            &ConfigOverrides::default(),
            &NoSecrets,
        )
        .unwrap_err();
        assert!(matches!(err, ConfigError::AmbiguousProtocol));
    }

    #[test]
    fn explicit_model_type_disambiguates() {
        let settings = settings_with(Some(Protocol::Anthropic), &[]);
        let env = map(&[
            ("ANTHROPIC_BASE_URL", "https://a"),
            ("ANTHROPIC_API_KEY", "sk-a"),
            ("OPENAI_BASE_URL", "https://b"),
        ]);
        let resolved = resolve(&settings, &env, &ConfigOverrides::default(), &NoSecrets).unwrap();
        assert_eq!(resolved.protocol, Protocol::Anthropic);
    }

    #[test]
    fn cli_overrides_beat_environment() {
        let env = map(&[
            ("ANTHROPIC_BASE_URL", "https://env"),
            ("ANTHROPIC_API_KEY", "sk-env"),
        ]);
        let overrides = ConfigOverrides {
            protocol: Some(Protocol::Anthropic),
            base_url: Some("https://flag/".into()),
            api_key: Some("sk-flag".into()),
            model: Some("claude-x".into()),
        };
        let resolved = resolve(&Settings::default(), &env, &overrides, &NoSecrets).unwrap();
        assert_eq!(resolved.base_url, "https://flag");
        assert_eq!(resolved.api_key, "sk-flag");
        assert_eq!(resolved.api_key_source, ApiKeySource::Flag);
        assert_eq!(resolved.model.as_deref(), Some("claude-x"));
    }

    #[test]
    fn settings_env_is_used_when_process_env_is_absent() {
        let settings = settings_with(
            Some(Protocol::OpenAi),
            &[
                ("OPENAI_BASE_URL", "https://from-settings"),
                ("OPENAI_API_KEY", "sk-settings"),
            ],
        );
        let resolved = resolve(
            &settings,
            &BTreeMap::new(),
            &ConfigOverrides::default(),
            &NoSecrets,
        )
        .unwrap();
        assert_eq!(resolved.base_url, "https://from-settings");
        assert_eq!(
            resolved.api_key_source,
            ApiKeySource::Settings("OPENAI_API_KEY".into())
        );
    }

    #[test]
    fn missing_base_url_is_reported() {
        let settings = settings_with(Some(Protocol::Anthropic), &[]);
        let env = map(&[("ANTHROPIC_API_KEY", "sk-a")]);
        let err = resolve(&settings, &env, &ConfigOverrides::default(), &NoSecrets).unwrap_err();
        assert!(matches!(err, ConfigError::MissingBaseUrl { .. }));
        assert!(err.to_string().contains("ANTHROPIC_BASE_URL"));
    }

    #[test]
    fn anthropic_auth_token_is_accepted() {
        let env = map(&[
            ("ANTHROPIC_BASE_URL", "https://a"),
            ("ANTHROPIC_AUTH_TOKEN", "tok"),
        ]);
        let resolved = resolve(
            &Settings::default(),
            &env,
            &ConfigOverrides::default(),
            &NoSecrets,
        )
        .unwrap();
        assert_eq!(resolved.api_key, "tok");
        assert_eq!(
            resolved.api_key_source,
            ApiKeySource::Env("ANTHROPIC_AUTH_TOKEN".into())
        );
    }
}
