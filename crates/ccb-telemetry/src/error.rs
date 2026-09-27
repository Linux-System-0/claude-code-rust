//! Errors raised while setting up the tracing subscriber.

use std::io;
use std::path::PathBuf;

/// Failure modes for [`crate::init`].
#[derive(Debug, thiserror::Error)]
pub enum TelemetryError {
    /// The directory that should hold the debug log could not be created.
    #[error("failed to create log directory {path}: {source}")]
    LogDir {
        /// Directory that could not be created.
        path: PathBuf,
        /// Underlying I/O error.
        source: io::Error,
    },

    /// The debug log file could not be opened for appending.
    #[error("failed to open debug log file {path}: {source}")]
    LogFile {
        /// File that could not be opened.
        path: PathBuf,
        /// Underlying I/O error.
        source: io::Error,
    },

    /// The `RUST_LOG` / `--debug` filter string was not valid.
    #[error("invalid log filter `{filter}`: {source}")]
    Filter {
        /// The offending filter expression.
        filter: String,
        /// Parse error from `tracing-subscriber`.
        source: tracing_subscriber::filter::ParseError,
    },
}

/// Convenience alias for telemetry results.
pub type Result<T> = std::result::Result<T, TelemetryError>;
