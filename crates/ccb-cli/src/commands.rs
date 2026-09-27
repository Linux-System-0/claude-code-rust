//! Top-level command dispatch and the configuration bootstrap shared by every
//! path (T1.1 / T1.2 / T1.3).

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::ExitCode;

use ccb_config::{
    load_layers_from, resolve, ApiKeyStore, ConfigError, ConfigOverrides, ConfigPaths, NoSecrets,
    ResolvedConfig, SecretLookup, SettingSource, Settings, SettingsArg, ALL_SETTING_SOURCES,
};
use ccb_telemetry::{init, InitOptions, Verbosity};
use tracing::{debug, info};

use crate::cli::{Cli, Command, DoctorArgs, Options, VERSION};

/// Entry point after argument parsing.
pub async fn run(cli: Cli) -> ExitCode {
    let Cli {
        prompt,
        options,
        command,
    } = cli;

    init_telemetry(&options);

    match command {
        Some(Command::Doctor(args)) => doctor(&options, &args),
        None => run_session(options, prompt).await,
    }
}

/// Install the tracing subscriber according to the CLI flags.
fn init_telemetry(options: &Options) {
    let verbosity = if options.debug.is_some() || options.debug_file.is_some() {
        Verbosity::Trace
    } else if options.verbose {
        Verbosity::Debug
    } else {
        Verbosity::Warn
    };
    // `--debug` without a value becomes the literal "true"; that is a request
    // for the default filter, not a category named "true".
    let filter = options
        .debug
        .as_deref()
        .filter(|value| *value != "true")
        .map(str::to_owned);

    let init_options = InitOptions {
        verbosity,
        filter,
        log_file: options.debug_file.clone(),
        ansi: None,
    };
    if let Err(err) = init(&init_options) {
        eprintln!("ccb: failed to initialize logging: {err}");
    }
}

/// Everything needed to start a session.
#[derive(Debug)]
struct SessionContext {
    paths: ConfigPaths,
    #[allow(dead_code)]
    settings: Settings,
    resolved: ResolvedConfig,
}

/// Load settings, merge the environment, and resolve a concrete endpoint.
fn load_context(options: &Options) -> Result<SessionContext, ConfigError> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let paths = ConfigPaths::discover(cwd)?;

    let sources = match &options.setting_sources {
        Some(raw) => SettingSource::parse_list(raw)?,
        None => ALL_SETTING_SOURCES.to_vec(),
    };
    let explicit: Vec<SettingsArg> = options
        .settings
        .iter()
        .map(|value| SettingsArg::classify(value))
        .collect();
    let settings = load_layers_from(&paths, &sources, &explicit)?;

    let env: BTreeMap<String, String> = std::env::vars().collect();
    let overrides = ConfigOverrides {
        protocol: options.provider.map(Into::into),
        base_url: options.base_url.clone(),
        api_key: options.api_key.clone(),
        model: options.model.clone(),
    };

    let store = ApiKeyStore::new(paths.claude_json.clone());
    // `--bare` avoids reading the key store entirely (matches the TS gate).
    let secrets: &dyn SecretLookup = if options.bare { &NoSecrets } else { &store };
    let resolved = resolve(&settings, &env, &overrides, secrets)?;

    Ok(SessionContext {
        paths,
        settings,
        resolved,
    })
}

/// `ccb doctor` — validate configuration and print a health report.
fn doctor(options: &Options, args: &DoctorArgs) -> ExitCode {
    match load_context(options) {
        Ok(ctx) => {
            if args.json {
                println!("{}", doctor_json(&ctx));
            } else {
                print_doctor_text(&ctx);
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            if args.json {
                let report = serde_json::json!({
                    "ok": false,
                    "error": err.to_string(),
                });
                println!("{report}");
            } else {
                eprintln!("Error: {err}");
            }
            ExitCode::FAILURE
        }
    }
}

fn doctor_json(ctx: &SessionContext) -> String {
    let report = serde_json::json!({
        "ok": true,
        "version": VERSION,
        "protocol": ctx.resolved.protocol.as_str(),
        "baseUrl": ctx.resolved.base_url,
        "model": ctx.resolved.model,
        "apiKeySource": format!("{:?}", ctx.resolved.api_key_source),
        "configDir": ctx.paths.config_dir,
        "settings": {
            "user": ctx.paths.user_settings().exists(),
            "project": ctx.paths.project_settings().exists(),
            "local": ctx.paths.local_settings().exists(),
        },
    });
    report.to_string()
}

fn print_doctor_text(ctx: &SessionContext) {
    println!("ccb {VERSION}");
    println!("  protocol:     {}", ctx.resolved.protocol);
    println!("  base URL:     {}", ctx.resolved.base_url);
    println!(
        "  model:        {}",
        ctx.resolved.model.as_deref().unwrap_or("(unset)")
    );
    println!("  API key:      ok ({:?})", ctx.resolved.api_key_source);
    println!("  config dir:   {}", ctx.paths.config_dir.display());
    println!(
        "  settings:     user={} project={} local={}",
        ctx.paths.user_settings().exists(),
        ctx.paths.project_settings().exists(),
        ctx.paths.local_settings().exists(),
    );
}

/// Default (non-subcommand) invocation.
///
/// Phase 1 resolves and validates configuration; the query loop and TUI land in
/// later phases, so this reports the resolved endpoint and exits successfully.
async fn run_session(options: Options, prompt: Option<String>) -> ExitCode {
    match load_context(&options) {
        Ok(ctx) => {
            info!(
                protocol = %ctx.resolved.protocol,
                base_url = %ctx.resolved.base_url,
                "configuration resolved"
            );
            debug!(?ctx.resolved.api_key_source, "api key source");
            eprintln!(
                "ccb {VERSION} — {} endpoint {}",
                ctx.resolved.protocol, ctx.resolved.base_url
            );
            if let Some(model) = &ctx.resolved.model {
                eprintln!("model: {model}");
            }
            if let Some(prompt) = prompt {
                eprintln!("prompt: {prompt}");
            }
            eprintln!(
                "note: the interactive/print loop is implemented in later phases (see Plan/04-task-list.md)."
            );
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("Error: {err}");
            ExitCode::FAILURE
        }
    }
}
