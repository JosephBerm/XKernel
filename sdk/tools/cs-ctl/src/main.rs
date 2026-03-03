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
        #[command(subcommand)]
        subcommand: Option<String>,
    },
    /// Tracing and debugging (cs-trace)
    Trace {
        #[command(subcommand)]
        subcommand: Option<String>,
    },
    /// Replay debugger (cs-replay)
    Replay {
        #[command(subcommand)]
        subcommand: Option<String>,
    },
    /// Profiler (cs-profile)
    Profile {
        #[command(subcommand)]
        subcommand: Option<String>,
    },
    /// Capability graph (cs-capgraph)
    Capgraph {
        #[command(subcommand)]
        subcommand: Option<String>,
    },
    /// System monitor (cs-top)
    Top {
        #[command(subcommand)]
        subcommand: Option<String>,
    },
    /// Show version information
    Version,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Pkg { subcommand } => {
            dispatch_tool("cs-pkg", subcommand);
        }
        Commands::Trace { subcommand } => {
            dispatch_tool("cs-trace", subcommand);
        }
        Commands::Replay { subcommand } => {
            dispatch_tool("cs-replay", subcommand);
        }
        Commands::Profile { subcommand } => {
            dispatch_tool("cs-profile", subcommand);
        }
        Commands::Capgraph { subcommand } => {
            dispatch_tool("cs-capgraph", subcommand);
        }
        Commands::Top { subcommand } => {
            dispatch_tool("cs-top", subcommand);
        }
        Commands::Version => {
            println!("cs-ctl version 1.0.0");
            println!("Cognitive Substrate Unified Control");
        }
    }
}

fn dispatch_tool(tool_name: &str, _subcommand: Option<String>) {
    println!("Dispatching to {}", tool_name);

    let _result = Command::new(tool_name)
        .args(std::env::args().skip(2))
        .status();
}
