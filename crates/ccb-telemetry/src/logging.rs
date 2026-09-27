//! Tracing subscriber initialization.
//!
//! The whole workspace logs through the `tracing` facade; this module is the
//! single place that installs a subscriber. Logs go to stderr by default and to
//! a file when `--debug-file <path>` is given (matching the TS CLI).

use std::fs::OpenOptions;
use std::path::{Path, PathBuf};

use tracing_subscriber::fmt::writer::MakeWriterExt;
use tracing_subscriber::EnvFilter;

use crate::error::{Result, TelemetryError};

/// How chatty the process should be.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Verbosity {
    /// Suppress everything except errors.
    Error,
    /// Warnings and errors (the default).
    #[default]
    Warn,
    /// Normal operational logging.
    Info,
    /// Verbose logging, equivalent to the TS `--verbose` flag.
    Debug,
    /// Everything, equivalent to `--debug`.
    Trace,
}

impl Verbosity {
    /// The `tracing` level directive for this verbosity.
    pub const fn as_str(self) -> &'static str {
        match self {
            Verbosity::Error => "error",
            Verbosity::Warn => "warn",
            Verbosity::Info => "info",
            Verbosity::Debug => "debug",
            Verbosity::Trace => "trace",
        }
    }
}

/// Options controlling subscriber installation.
#[derive(Debug, Clone, Default)]
pub struct InitOptions {
    /// Base verbosity, ignored when `filter` or `RUST_LOG` is set.
    pub verbosity: Verbosity,
    /// Explicit `tracing` filter expression (from `--debug <filter>`).
    pub filter: Option<String>,
    /// Write logs to this file instead of stderr.
    pub log_file: Option<PathBuf>,
    /// Force ANSI colors on or off. `None` lets `tracing` decide from the TTY.
    pub ansi: Option<bool>,
}

impl InitOptions {
    /// Resolve the effective filter, preferring `RUST_LOG` over everything else.
    fn effective_filter(&self) -> Result<EnvFilter> {
        if let Ok(raw) = std::env::var("RUST_LOG") {
            if !raw.trim().is_empty() {
                return self.parse(&raw);
            }
        }
        if let Some(filter) = &self.filter {
            // `--debug` accepts a bare category filter such as `api,hooks`.
            // If it has no level, scope everything to `debug` first.
            if looks_like_levels(filter) {
                return self.parse(filter);
            }
            return self.parse(&format!("debug,{filter}"));
        }
        self.parse(self.verbosity.as_str())
    }

    fn parse(&self, filter: &str) -> Result<EnvFilter> {
        EnvFilter::try_new(filter).map_err(|source| TelemetryError::Filter {
            filter: filter.to_owned(),
            source,
        })
    }
}

/// Heuristic: does the filter already contain a `=level` or a known level?
fn looks_like_levels(filter: &str) -> bool {
    filter.split(',').any(|part| {
        let part = part.trim();
        part.contains('=')
            || matches!(
                part.to_ascii_lowercase().as_str(),
                "trace" | "debug" | "info" | "warn" | "error" | "off"
            )
    })
}

/// Install the global tracing subscriber.
///
/// Calling this more than once is a no-op; the first call wins. That mirrors
/// the TS behaviour where debug setup is idempotent and keeps tests from
/// fighting over the global subscriber.
pub fn init(options: &InitOptions) -> Result<()> {
    let filter = options.effective_filter()?;

    // The subscriber is `'static`, so the closure writer must own its handle.
    // `File` is `Clone`; `MakeWriter` is implemented for closures returning
    // any `Write`.
    let result = match &options.log_file {
        Some(path) => {
            if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
                std::fs::create_dir_all(parent).map_err(|source| TelemetryError::LogDir {
                    path: parent.to_path_buf(),
                    source,
                })?;
            }
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .map_err(|source| TelemetryError::LogFile {
                    path: path.clone(),
                    source,
                })?;
            let writer = std::sync::Mutex::new(file);
            let subscriber = tracing_subscriber::fmt()
                .with_env_filter(filter)
                .with_ansi(false)
                .with_writer(writer)
                .finish();
            tracing::subscriber::set_global_default(subscriber)
        }
        None => {
            let subscriber = tracing_subscriber::fmt()
                .with_env_filter(filter)
                .with_ansi(options.ansi.unwrap_or(true))
                .with_writer(
                    std::io::stderr
                        .with_max_level(tracing::Level::TRACE)
                        .or_else(std::io::sink),
                )
                .finish();
            tracing::subscriber::set_global_default(subscriber)
        }
    };

    // An already-installed subscriber is not an error.
    match result {
        Ok(()) => Ok(()),
        Err(_) => Ok(()),
    }
}

/// Best-effort default log path: `<config_dir>/logs/ccb.log`.
///
/// Kept here (rather than in `ccb-config`) so callers that only need logging
/// do not have to depend on the settings layer.
pub fn default_log_path(config_dir: &Path) -> PathBuf {
    config_dir.join("logs").join("ccb.log")
}
