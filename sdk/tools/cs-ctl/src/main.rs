//! cs-ctl — Cognitive Substrate control CLI.
//!
//! Real HTTP client that talks to the cs-daemon API to manage agents,
//! channels, memory, tools, and system state.
//!
//! # Usage
//!
//! ```bash
//! cs-ctl status                              # Daemon health + metrics
//! cs-ctl agent create mybot --command "python bot.py"
//! cs-ctl agent list                          # List all agents
//! cs-ctl agent get <id>                      # Get agent details
//! cs-ctl agent stop <id>                     # Stop an agent
//! cs-ctl agent logs <id>                     # View agent logs
//! cs-ctl channel create --from <id> --to <id>
//! cs-ctl channel send <id> "hello"
//! cs-ctl tool register --name calculator
//! cs-ctl metrics                             # Full system metrics
//! cs-ctl events                              # Recent telemetry events
//! ```

use clap::{Parser, Subcommand};
use serde_json::Value;

#[derive(Parser)]
#[command(name = "cs-ctl")]
#[command(version = "1.0.0")]
#[command(about = "Cognitive Substrate control CLI — manage AI agents via cs-daemon")]
struct Cli {
    /// Daemon URL (default: http://127.0.0.1:7600)
    #[arg(long, default_value = "http://127.0.0.1:7600", global = true)]
    daemon: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show daemon health and status
    Status,
    /// Full system metrics
    Metrics,
    /// Recent telemetry events
    Events {
        /// Maximum events to show
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },
    /// Agent management
    Agent {
        #[command(subcommand)]
        action: AgentAction,
    },
    /// IPC channel management
    Channel {
        #[command(subcommand)]
        action: ChannelAction,
    },
    /// Tool registry management
    Tool {
        #[command(subcommand)]
        action: ToolAction,
    },
    /// Memory management
    Memory {
        #[command(subcommand)]
        action: MemoryAction,
    },
}

#[derive(Subcommand)]
enum AgentAction {
    /// Create a new agent
    Create {
        /// Agent name
        name: String,
        /// Command to execute
        #[arg(short, long)]
        command: Option<String>,
        /// Framework type
        #[arg(short, long, default_value = "custom")]
        framework: String,
        /// Priority (0-255)
        #[arg(short, long, default_value = "128")]
        priority: u8,
        /// Restart policy: never, on_failure, always
        #[arg(short, long, default_value = "never")]
        restart: String,
    },
    /// List all agents
    List,
    /// Get agent details
    Get {
        /// Agent ID
        id: String,
    },
    /// Stop an agent
    Stop {
        /// Agent ID
        id: String,
    },
    /// View agent logs
    Logs {
        /// Agent ID
        id: String,
    },
    /// Send a signal to an agent
    Signal {
        /// Agent ID
        id: String,
        /// Signal: stop, checkpoint, yield
        signal: String,
    },
}

#[derive(Subcommand)]
enum ChannelAction {
    /// Create a channel between two agents
    Create {
        /// Sender agent ID
        #[arg(long)]
        from: String,
        /// Receiver agent ID
        #[arg(long)]
        to: String,
        /// Capacity
        #[arg(long, default_value = "256")]
        capacity: usize,
    },
    /// List all channels
    List,
    /// Send a message through a channel
    Send {
        /// Channel ID
        id: u64,
        /// Message payload
        message: String,
    },
    /// Receive a message from a channel
    Receive {
        /// Channel ID
        id: u64,
    },
}

#[derive(Subcommand)]
enum ToolAction {
    /// Register a tool
    Register {
        /// Tool name
        #[arg(long)]
        name: String,
        /// Effect class: read_only, write_reversible, write_irreversible
        #[arg(long, default_value = "read_only")]
        effect: String,
    },
    /// List registered tools
    List,
    /// Unregister a tool
    Remove {
        /// Tool binding ID
        id: String,
    },
}

#[derive(Subcommand)]
enum MemoryAction {
    /// Show memory statistics
    Stats,
    /// Allocate memory pages
    Alloc {
        /// Number of pages (4KB each)
        #[arg(long)]
        pages: u64,
        /// Owner CT ID
        #[arg(long, default_value = "1")]
        owner: u32,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let base = cli.daemon.trim_end_matches('/');
    let client = reqwest::Client::new();

    let result: Result<(), String> = match cli.command {
        Commands::Status => {
            let health = get(&client, &format!("{}/healthz", base)).await;
            let metrics = get(&client, &format!("{}/api/v1/metrics", base)).await;
            println!("=== Cognitive Substrate Daemon ===\n");
            print_json(&health);
            println!("\n=== Metrics ===\n");
            print_json(&metrics);
            Ok(())
        }
        Commands::Metrics => {
            let resp = get(&client, &format!("{}/api/v1/metrics", base)).await;
            print_json(&resp);
            Ok(())
        }
        Commands::Events { limit } => {
            let resp = get(&client, &format!("{}/api/v1/events?limit={}", base, limit)).await;
            print_json(&resp);
            Ok(())
        }
        Commands::Agent { action } => match action {
            AgentAction::Create { name, command, framework, priority, restart } => {
                let body = serde_json::json!({
                    "name": name,
                    "framework": framework,
                    "entrypoint": command,
                    "priority": priority,
                    "restart_policy": restart,
                });
                let resp = post(&client, &format!("{}/api/v1/agents", base), &body).await;
                println!("Agent created:");
                print_json(&resp);
                Ok(())
            }
            AgentAction::List => {
                let resp = get(&client, &format!("{}/api/v1/agents", base)).await;
                print_json(&resp);
                Ok(())
            }
            AgentAction::Get { id } => {
                let resp = get(&client, &format!("{}/api/v1/agents/{}", base, id)).await;
                print_json(&resp);
                Ok(())
            }
            AgentAction::Stop { id } => {
                let resp = del(&client, &format!("{}/api/v1/agents/{}", base, id)).await;
                println!("Agent stopped:");
                print_json(&resp);
                Ok(())
            }
            AgentAction::Logs { id } => {
                let resp = get(&client, &format!("{}/api/v1/agents/{}/logs", base, id)).await;
                print_json(&resp);
                Ok(())
            }
            AgentAction::Signal { id, signal } => {
                let body = serde_json::json!({ "signal": signal });
                let resp = post(&client, &format!("{}/api/v1/agents/{}/signal", base, id), &body).await;
                print_json(&resp);
                Ok(())
            }
        },
        Commands::Channel { action } => match action {
            ChannelAction::Create { from, to, capacity } => {
                let body = serde_json::json!({
                    "sender": from,
                    "receiver": to,
                    "capacity": capacity,
                });
                let resp = post(&client, &format!("{}/api/v1/channels", base), &body).await;
                println!("Channel created:");
                print_json(&resp);
                Ok(())
            }
            ChannelAction::List => {
                let resp = get(&client, &format!("{}/api/v1/channels", base)).await;
                print_json(&resp);
                Ok(())
            }
            ChannelAction::Send { id, message } => {
                let body = serde_json::json!({ "payload": message });
                let resp = post(&client, &format!("{}/api/v1/channels/{}/send", base, id), &body).await;
                println!("Message sent:");
                print_json(&resp);
                Ok(())
            }
            ChannelAction::Receive { id } => {
                let resp = post_empty(&client, &format!("{}/api/v1/channels/{}/receive", base, id)).await;
                print_json(&resp);
                Ok(())
            }
        },
        Commands::Tool { action } => match action {
            ToolAction::Register { name, effect } => {
                let body = serde_json::json!({
                    "name": name,
                    "effect_class": effect,
                });
                let resp = post(&client, &format!("{}/api/v1/tools", base), &body).await;
                println!("Tool registered:");
                print_json(&resp);
                Ok(())
            }
            ToolAction::List => {
                let resp = get(&client, &format!("{}/api/v1/tools", base)).await;
                print_json(&resp);
                Ok(())
            }
            ToolAction::Remove { id } => {
                let resp = del(&client, &format!("{}/api/v1/tools/{}", base, id)).await;
                print_json(&resp);
                Ok(())
            }
        },
        Commands::Memory { action } => match action {
            MemoryAction::Stats => {
                let resp = get(&client, &format!("{}/api/v1/memory", base)).await;
                print_json(&resp);
                Ok(())
            }
            MemoryAction::Alloc { pages, owner } => {
                let body = serde_json::json!({
                    "pages": pages,
                    "owner_ct_id": owner,
                });
                let resp = post(&client, &format!("{}/api/v1/memory/allocate", base), &body).await;
                println!("Memory allocated:");
                print_json(&resp);
                Ok(())
            }
        },
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

async fn get(client: &reqwest::Client, url: &str) -> Value {
    match client.get(url).send().await {
        Ok(resp) => resp.json::<Value>().await.unwrap_or_else(|e| {
            serde_json::json!({"error": format!("failed to parse response: {}", e)})
        }),
        Err(e) => {
            serde_json::json!({"error": format!("connection failed: {} — is cs-daemon running?", e)})
        }
    }
}

async fn post(client: &reqwest::Client, url: &str, body: &Value) -> Value {
    match client.post(url).json(body).send().await {
        Ok(resp) => resp.json::<Value>().await.unwrap_or_else(|e| {
            serde_json::json!({"error": format!("failed to parse response: {}", e)})
        }),
        Err(e) => {
            serde_json::json!({"error": format!("connection failed: {} — is cs-daemon running?", e)})
        }
    }
}

async fn post_empty(client: &reqwest::Client, url: &str) -> Value {
    match client.post(url).send().await {
        Ok(resp) => resp.json::<Value>().await.unwrap_or_else(|e| {
            serde_json::json!({"error": format!("failed to parse response: {}", e)})
        }),
        Err(e) => {
            serde_json::json!({"error": format!("connection failed: {} — is cs-daemon running?", e)})
        }
    }
}

async fn del(client: &reqwest::Client, url: &str) -> Value {
    match client.delete(url).send().await {
        Ok(resp) => resp.json::<Value>().await.unwrap_or_else(|e| {
            serde_json::json!({"error": format!("failed to parse response: {}", e)})
        }),
        Err(e) => {
            serde_json::json!({"error": format!("connection failed: {} — is cs-daemon running?", e)})
        }
    }
}

fn print_json(value: &Value) {
    match serde_json::to_string_pretty(value) {
        Ok(s) => println!("{}", s),
        Err(_) => println!("{:?}", value),
    }
}
