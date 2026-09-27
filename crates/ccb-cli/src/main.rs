//! `ccb` — command-line entry point.
//!
//! Phase 1 / T1.1. The `--version` fast-path runs before clap and before the
//! async runtime is created, mirroring the zero-import fast-path in the TS
//! `entrypoints/cli.tsx`.

mod cli;
mod commands;

use std::ffi::OsString;
use std::process::ExitCode;

use clap::Parser;
use cli::{Cli, VERSION};

fn main() -> ExitCode {
    fast_path();

    let cli = Cli::parse();
    match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime.block_on(commands::run(cli)),
        Err(err) => {
            eprintln!("ccb: failed to start the async runtime: {err}");
            ExitCode::FAILURE
        }
    }
}

/// Handle flags that must work before the full CLI (and runtime) load.
fn fast_path() {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();

    if args.len() == 1 {
        let arg = args[0].to_string_lossy();
        if matches!(arg.as_ref(), "--version" | "-v" | "-V") {
            println!("{VERSION}");
            std::process::exit(0);
        }
    }

    // `--bare` sets SIMPLE before any gated work runs (matches the TS CLI).
    if args.iter().any(|arg| arg == "--bare") {
        std::env::set_var("CLAUDE_CODE_SIMPLE", "1");
    }
}
