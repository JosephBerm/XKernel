//! CLI Stub Implementation for cs-agentctl Command
//!
//! Provides command-line interface for agent lifecycle management with subcommands:
//! status (show agent states and resource usage), logs (tail lifecycle events),
//! list (enumerate managed agents), and help/usage text.
//! See RFC: Week 6 cs-agentctl CLI subsystem design.

use std::collections::BTreeMap;
use std::io;
use crate::error::{LifecycleError, Result};
use crate::health_status::{HealthStatusAggregator, AgentHealthStatus};
use crate::lifecycle_logging::{LifecycleLogger, LogLevel, EventType};

/// CLI argument parser result.
///
/// Represents parsed command-line arguments and subcommand.
#[derive(Debug, Clone)]
pub struct CliArgs {
    /// Subcommand to execute
    pub subcommand: Subcommand,
    /// Global flags
    pub verbose: bool,
    /// Optional JSON output flag
    pub json: bool,
}

/// Subcommand enumeration.
///
/// Represents top-level CLI subcommands.
#[derive(Debug, Clone)]
pub enum Subcommand {
    /// Status subcommand: show agent states
    Status {
        /// Optional agent ID filter
        agent_id: Option<String>,
    },
    /// Logs subcommand: tail lifecycle events
    Logs {
        /// Number of entries to tail
        count: usize,
        /// Optional agent ID filter
        agent_id: Option<String>,
        /// Optional log level filter
        level: Option<LogLevel>,
    },
    /// List subcommand: enumerate agents
    List,
    /// Help subcommand
    Help,
    /// Version subcommand
    Version,
}

/// Agent status summary for CLI display.
///
/// Snapshot of agent status for reporting.
#[derive(Debug, Clone)]
pub struct AgentStatusSummary {
    /// Agent identifier
    pub agent_id: String,
    /// Health state as string
    pub state: String,
    /// Uptime in seconds
    pub uptime_secs: u64,
    /// Memory usage in MB
    pub memory_mb: u64,
    /// CPU usage percentage
    pub cpu_percent: f64,
    /// Optional failure reason
    pub reason: Option<String>,
}

impl AgentStatusSummary {
    /// Create a summary from agent health status.
    pub fn from_health_status(status: &AgentHealthStatus) -> Self {
        let metrics = status.metrics();
        Self {
            agent_id: status.agent_id().to_string(),
            state: status.state().as_str().to_string(),
            uptime_secs: metrics.uptime_secs,
            memory_mb: metrics.memory_bytes / (1024 * 1024),
            cpu_percent: metrics.cpu_percent,
            reason: status.reason().map(|s| s.to_string()),
        }
    }

    /// Format as human-readable string.
    pub fn format_human(&self) -> String {
        let reason_str = self.reason.as_ref()
            .map(|r| format!(" ({})", r))
            .unwrap_or_default();

        format!(
            "{:<30} | State: {:<10} | Memory: {:<6}MB | CPU: {:<6.1}% | Uptime: {:<6}s{}",
            self.agent_id,
            self.state,
            self.memory_mb,
            self.cpu_percent,
            self.uptime_secs,
            reason_str
        )
    }

    /// Format as JSON string.
    pub fn format_json(&self) -> String {
        format!(
            r#"{{"agent_id":"{}","state":"{}","uptime_secs":{},"memory_mb":{},"cpu_percent":{}{}}}"#,
            self.agent_id.replace('"', "\\\""),
            self.state,
            self.uptime_secs,
            self.memory_mb,
            self.cpu_percent,
            self.reason.as_ref()
                .map(|r| format!(r#","reason":"{}""#, r.replace('"', "\\\"")))
                .unwrap_or_default()
        )
    }
}

/// CLI command processor.
///
/// Handles parsing and execution of CLI subcommands.
pub struct CliCommandProcessor {
    /// Health status aggregator reference
    health_agg: HealthStatusAggregator,
    /// Lifecycle logger reference
    logger: LifecycleLogger,
}

impl CliCommandProcessor {
    /// Create a new CLI command processor.
    ///
    /// # Arguments
    /// * `health_agg` - Health status aggregator
    /// * `logger` - Lifecycle logger
    pub fn new(health_agg: HealthStatusAggregator, logger: LifecycleLogger) -> Self {
        Self {
            health_agg,
            logger,
        }
    }

    /// Parse command-line arguments.
    ///
    /// # Arguments
    /// * `args` - Raw command-line arguments (excluding program name)
    ///
    /// # Returns
    /// Result containing parsed CLI arguments.
    pub fn parse_args(args: &[&str]) -> Result<CliArgs> {
        if args.is_empty() {
            return Ok(CliArgs {
                subcommand: Subcommand::Help,
                verbose: false,
                json: false,
            });
        }

        let mut verbose = false;
        let mut json = false;
        let mut subcommand_args = Vec::new();

        // Parse global flags
        for (i, arg) in args.iter().enumerate() {
            match *arg {
                "-v" | "--verbose" => verbose = true,
                "--json" => json = true,
                _ => {
                    subcommand_args.extend_from_slice(&args[i..]);
                    break;
                }
            }
        }

        if subcommand_args.is_empty() {
            return Ok(CliArgs {
                subcommand: Subcommand::Help,
                verbose,
                json,
            });
        }

        let subcommand = match subcommand_args[0] {
            "status" => {
                let agent_id = subcommand_args.get(1).map(|s| s.to_string());
                Subcommand::Status { agent_id }
            }
            "logs" => {
                let mut count = 20;
                let mut agent_id = None;
                let mut level = None;

                let mut i = 1;
                while i < subcommand_args.len() {
                    match subcommand_args[i] {
                        "-n" | "--number" => {
                            if let Ok(n) = subcommand_args.get(i + 1)
                                .ok_or_else(|| LifecycleError::CliError("Missing argument for -n".to_string()))?
                                .parse::<usize>() {
                                count = n;
                                i += 1;
                            }
                        }
                        "-a" | "--agent" => {
                            agent_id = Some(subcommand_args.get(i + 1)
                                .ok_or_else(|| LifecycleError::CliError("Missing argument for -a".to_string()))?
                                .to_string());
                            i += 1;
                        }
                        "-l" | "--level" => {
                            let level_str = subcommand_args.get(i + 1)
                                .ok_or_else(|| LifecycleError::CliError("Missing argument for -l".to_string()))?;
                            level = LogLevel::from_str(level_str);
                            i += 1;
                        }
                        _ => {}
                    }
                    i += 1;
                }

                Subcommand::Logs {
                    count,
                    agent_id,
                    level,
                }
            }
            "list" => Subcommand::List,
            "help" | "-h" | "--help" => Subcommand::Help,
            "version" | "-v" | "--version" => Subcommand::Version,
            other => {
                return Err(LifecycleError::CliError(format!("Unknown subcommand: {}", other)));
            }
        };

        Ok(CliArgs {
            subcommand,
            verbose,
            json,
        })
    }

    /// Execute a parsed CLI command.
    ///
    /// # Arguments
    /// * `cli_args` - Parsed CLI arguments
    /// * `output` - Output writer for results
    ///
    /// # Returns
    /// Result indicating success or error.
    pub fn execute(&self, cli_args: CliArgs, output: &mut dyn io::Write) -> Result<()> {
        match cli_args.subcommand {
            Subcommand::Status { agent_id } => {
                self.handle_status(agent_id, cli_args.json, output)
            }
            Subcommand::Logs { count, agent_id, level } => {
                self.handle_logs(count, agent_id, level, cli_args.json, output)
            }
            Subcommand::List => {
                self.handle_list(cli_args.json, output)
            }
            Subcommand::Help => {
                self.handle_help(output)
            }
            Subcommand::Version => {
                self.handle_version(output)
            }
        }
    }

    /// Handle status subcommand.
    fn handle_status(&self, agent_id: Option<String>, json: bool, output: &mut dyn io::Write) -> Result<()> {
        let statuses = self.health_agg.get_all_statuses()?;

        let filtered: Vec<AgentStatusSummary> = statuses.into_iter()
            .filter(|s| agent_id.as_ref().map_or(true, |id| s.agent_id() == id))
            .map(|s| AgentStatusSummary::from_health_status(&s))
            .collect();

        if filtered.is_empty() {
            writeln!(output, "No agents found")?;
            return Ok(());
        }

        if json {
            writeln!(output, "[")?;
            for (i, summary) in filtered.iter().enumerate() {
                write!(output, "  {}", summary.format_json())?;
                if i < filtered.len() - 1 {
                    writeln!(output, ",")?;
                } else {
                    writeln!(output)?;
                }
            }
            writeln!(output, "]")?;
        } else {
            writeln!(output, "Agent Status Report")?;
            writeln!(output, "{}", "=".repeat(100))?;
            for summary in filtered {
                writeln!(output, "{}", summary.format_human())?;
            }
        }

        Ok(())
    }

    /// Handle logs subcommand.
    fn handle_logs(
        &self,
        count: usize,
        agent_id: Option<String>,
        level: Option<LogLevel>,
        json: bool,
        output: &mut dyn io::Write,
    ) -> Result<()> {
        let entries = if let Some(level_filter) = level {
            self.logger.query(level_filter, agent_id.as_deref(), None)?
        } else {
            self.logger.query(LogLevel::Debug, agent_id.as_deref(), None)?
        };

        let recent: Vec<_> = entries.into_iter().rev().take(count).rev().collect();

        if recent.is_empty() {
            writeln!(output, "No log entries found")?;
            return Ok(());
        }

        if json {
            writeln!(output, "[")?;
            for (i, entry) in recent.iter().enumerate() {
                write!(output, "  {}", entry.to_json())?;
                if i < recent.len() - 1 {
                    writeln!(output, ",")?;
                } else {
                    writeln!(output)?;
                }
            }
            writeln!(output, "]")?;
        } else {
            writeln!(output, "Lifecycle Event Log")?;
            writeln!(output, "{}", "=".repeat(100))?;
            for entry in recent {
                let ts = entry.timestamp_ms;
                let context_str = entry.context.as_ref()
                    .map(|c| format!(" | {}", c))
                    .unwrap_or_default();
                writeln!(output, "[{}] {} | {} | {} | {}{}",
                    ts,
                    entry.level.as_str(),
                    entry.event_type.as_str(),
                    entry.agent_id,
                    entry.message,
                    context_str
                )?;
            }
        }

        Ok(())
    }

    /// Handle list subcommand.
    fn handle_list(&self, json: bool, output: &mut dyn io::Write) -> Result<()> {
        let statuses = self.health_agg.get_all_statuses()?;

        if statuses.is_empty() {
            writeln!(output, "No agents found")?;
            return Ok(());
        }

        if json {
            writeln!(output, "[")?;
            for (i, status) in statuses.iter().enumerate() {
                write!(output, r#"  "{}""#, status.agent_id())?;
                if i < statuses.len() - 1 {
                    writeln!(output, ",")?;
                } else {
                    writeln!(output)?;
                }
            }
            writeln!(output, "]")?;
        } else {
            writeln!(output, "Managed Agents")?;
            writeln!(output, "{}", "=".repeat(50))?;
            for status in statuses {
                writeln!(output, "  {}", status.agent_id())?;
            }
        }

        Ok(())
    }

    /// Handle help subcommand.
    fn handle_help(&self, output: &mut dyn io::Write) -> Result<()> {
        let help_text = r#"
cs-agentctl - Agent Lifecycle Management CLI

USAGE:
    cs-agentctl [FLAGS] <SUBCOMMAND> [ARGS]

FLAGS:
    -v, --verbose    Enable verbose output
    --json           Output in JSON format
    -h, --help       Show this help message
    --version        Show version information

SUBCOMMANDS:
    status [AGENT_ID]
        Show agent health status and resource usage.
        Optional: filter by agent ID.
        
        Example: cs-agentctl status
        Example: cs-agentctl status agent-1
    
    logs [OPTIONS]
        Tail lifecycle event logs.
        
        OPTIONS:
            -n, --number <COUNT>     Number of entries to show (default: 20)
            -a, --agent <AGENT_ID>   Filter by agent ID
            -l, --level <LEVEL>      Filter by log level (DEBUG|INFO|WARN|ERROR)
        
        Example: cs-agentctl logs
        Example: cs-agentctl logs -n 50 -a agent-1
        Example: cs-agentctl logs -l ERROR
    
    list
        List all managed agents.
        
        Example: cs-agentctl list
    
    help
        Show this help message.

EXAMPLES:
    Show status of all agents:
        $ cs-agentctl status
    
    Show status of specific agent:
        $ cs-agentctl status my-agent-1
    
    Show last 50 log entries in JSON:
        $ cs-agentctl logs -n 50 --json
    
    Show only error logs:
        $ cs-agentctl logs -l ERROR
    
    List agents in JSON format:
        $ cs-agentctl list --json
"#;
        write!(output, "{}", help_text)?;
        Ok(())
    }

    /// Handle version subcommand.
    fn handle_version(&self, output: &mut dyn io::Write) -> Result<()> {
        writeln!(output, "cs-agentctl version 0.1.0")?;
        writeln!(output, "Cognitive Substrate Agent Lifecycle Manager")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_args_help() {
        let args = vec![];
        let cli_args = CliCommandProcessor::parse_args(&args).unwrap();
        matches!(cli_args.subcommand, Subcommand::Help);
    }

    #[test]
    fn test_parse_args_status() {
        let args = vec!["status"];
        let cli_args = CliCommandProcessor::parse_args(&args).unwrap();
        matches!(cli_args.subcommand, Subcommand::Status { agent_id: None });
    }

    #[test]
    fn test_parse_args_status_with_agent() {
        let args = vec!["status", "agent-1"];
        let cli_args = CliCommandProcessor::parse_args(&args).unwrap();
        match cli_args.subcommand {
            Subcommand::Status { agent_id } => {
                assert_eq!(agent_id, Some("agent-1".to_string()));
            }
            _ => panic!("Expected Status subcommand"),
        }
    }

    #[test]
    fn test_parse_args_logs_defaults() {
        let args = vec!["logs"];
        let cli_args = CliCommandProcessor::parse_args(&args).unwrap();
        match cli_args.subcommand {
            Subcommand::Logs { count, agent_id, level } => {
                assert_eq!(count, 20);
                assert_eq!(agent_id, None);
                assert_eq!(level, None);
            }
            _ => panic!("Expected Logs subcommand"),
        }
    }

    #[test]
    fn test_parse_args_logs_with_count() {
        let args = vec!["logs", "-n", "50"];
        let cli_args = CliCommandProcessor::parse_args(&args).unwrap();
        match cli_args.subcommand {
            Subcommand::Logs { count, .. } => {
                assert_eq!(count, 50);
            }
            _ => panic!("Expected Logs subcommand"),
        }
    }

    #[test]
    fn test_parse_args_logs_with_agent() {
        let args = vec!["logs", "-a", "agent-1"];
        let cli_args = CliCommandProcessor::parse_args(&args).unwrap();
        match cli_args.subcommand {
            Subcommand::Logs { agent_id, .. } => {
                assert_eq!(agent_id, Some("agent-1".to_string()));
            }
            _ => panic!("Expected Logs subcommand"),
        }
    }

    #[test]
    fn test_parse_args_logs_with_level() {
        let args = vec!["logs", "-l", "ERROR"];
        let cli_args = CliCommandProcessor::parse_args(&args).unwrap();
        match cli_args.subcommand {
            Subcommand::Logs { level, .. } => {
                assert_eq!(level, Some(LogLevel::Error));
            }
            _ => panic!("Expected Logs subcommand"),
        }
    }

    #[test]
    fn test_parse_args_list() {
        let args = vec!["list"];
        let cli_args = CliCommandProcessor::parse_args(&args).unwrap();
        matches!(cli_args.subcommand, Subcommand::List);
    }

    #[test]
    fn test_parse_args_version() {
        let args = vec!["version"];
        let cli_args = CliCommandProcessor::parse_args(&args).unwrap();
        matches!(cli_args.subcommand, Subcommand::Version);
    }

    #[test]
    fn test_parse_args_verbose_flag() {
        let args = vec!["-v", "status"];
        let cli_args = CliCommandProcessor::parse_args(&args).unwrap();
        assert!(cli_args.verbose);
    }

    #[test]
    fn test_parse_args_json_flag() {
        let args = vec!["--json", "status"];
        let cli_args = CliCommandProcessor::parse_args(&args).unwrap();
        assert!(cli_args.json);
    }

    #[test]
    fn test_agent_status_summary_format_human() {
        let status = AgentStatusSummary {
            agent_id: "agent-1".to_string(),
            state: "running".to_string(),
            uptime_secs: 3600,
            memory_mb: 256,
            cpu_percent: 45.5,
            reason: None,
        };

        let formatted = status.format_human();
        assert!(formatted.contains("agent-1"));
        assert!(formatted.contains("running"));
        assert!(formatted.contains("256"));
    }

    #[test]
    fn test_agent_status_summary_format_json() {
        let status = AgentStatusSummary {
            agent_id: "agent-1".to_string(),
            state: "running".to_string(),
            uptime_secs: 3600,
            memory_mb: 256,
            cpu_percent: 45.5,
            reason: None,
        };

        let formatted = status.format_json();
        assert!(formatted.contains(r#""agent_id":"agent-1""#));
        assert!(formatted.contains(r#""state":"running""#));
    }

    #[test]
    fn test_agent_status_summary_with_reason() {
        let status = AgentStatusSummary {
            agent_id: "agent-1".to_string(),
            state: "failed".to_string(),
            uptime_secs: 100,
            memory_mb: 512,
            cpu_percent: 0.0,
            reason: Some("Out of memory".to_string()),
        };

        let json = status.format_json();
        assert!(json.contains(r#""reason":"Out of memory""#));
    }

    #[test]
    fn test_cli_command_processor_creation() {
        let agg = HealthStatusAggregator::new();
        let logger = LifecycleLogger::new();
        let _processor = CliCommandProcessor::new(agg, logger);
    }
}
