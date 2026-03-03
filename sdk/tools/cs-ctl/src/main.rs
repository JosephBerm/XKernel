// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! cs-ctl - Cognitive Substrate Unified Control CLI
//!
//! Provides unified control and dispatching to all CS SDK tools.

use clap::{Parser, Subcommand};
use std::process::Command;

#[derive(Parser)]
#[command(name = "cs-ctl")]
#[command(version = "1.0.0")]
#[command(about = "Cognitive Substrate Unified Control", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Package management (cs-pkg)
    Pkg {
        /// Arguments to pass through
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Tracing and debugging (cs-trace)
    Trace {
        /// Arguments to pass through
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Replay debugger (cs-replay)
    Replay {
        /// Arguments to pass through
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Profiler (cs-profile)
    Profile {
        /// Arguments to pass through
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Capability graph (cs-capgraph)
    Capgraph {
        /// Arguments to pass through
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// System monitor (cs-top)
    Top {
        /// Arguments to pass through
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Show version information
    Version,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Pkg { args } => {
            dispatch_tool("cs-pkg", &args);
        }
        Commands::Trace { args } => {
            dispatch_tool("cs-trace", &args);
        }
        Commands::Replay { args } => {
            dispatch_tool("cs-replay", &args);
        }
        Commands::Profile { args } => {
            dispatch_tool("cs-profile", &args);
        }
        Commands::Capgraph { args } => {
            dispatch_tool("cs-capgraph", &args);
        }
        Commands::Top { args } => {
            dispatch_tool("cs-top", &args);
        }
        Commands::Version => {
            println!("cs-ctl version 1.0.0");
            println!("Cognitive Substrate Unified Control");
        }
    }
}

fn dispatch_tool(tool_name: &str, args: &[String]) {
    println!("Dispatching to {}", tool_name);

    let _result = Command::new(tool_name)
        .args(args)
        .status();
}
