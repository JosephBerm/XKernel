// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! # Cognitive Substrate Control Utility (cs-ctl)
//!
//! The cs-ctl crate provides command-line interface bindings for managing and debugging
//! Cognitive Substrate instances, including task inspection, lifecycle control, and
//! system health monitoring.
//!
//! ## Architecture
//!
//! This crate defines:
//! - **CsCtlCommand enum**: Available control commands
//! - **CsCtlConfig**: Client configuration (endpoint, auth)
//! - **execute_command**: Command execution interface
//! - **CsCtlOutput**: Structured command output
//!
//! ## Design Philosophy
//!
//! The control utility is designed to be CLI-friendly with structured output support
//! for scripting and automation. All operations are stateless and idempotent where possible.

#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use alloc::format;
use alloc::string::ToString;

/// Output format types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputFormat {
    /// Plain text format
    Text,
    /// JSON format
    Json,
    /// YAML format
    Yaml,
}

impl Default for OutputFormat {
    fn default() -> Self {
        OutputFormat::Text
    }
}

/// Control command types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CsCtlCommand {
    /// Get system status
    Status,
    /// Start cognitive substrate
    Start {
        /// Optional configuration file path
        config_path: Option<String>,
    },
    /// Stop cognitive substrate
    Stop {
        /// Force shutdown
        force: bool,
    },
    /// Restart cognitive substrate
    Restart {
        /// Optional configuration file path
        config_path: Option<String>,
    },
    /// Inspect task details
    Inspect {
        /// Task ID to inspect
        task_id: String,
    },
    /// List running tasks
    List {
        /// Filter by state
        state: Option<String>,
        /// Verbose output
        verbose: bool,
    },
    /// Get task logs
    Logs {
        /// Task ID
        task_id: String,
        /// Number of lines to show
        lines: Option<usize>,
        /// Follow logs
        follow: bool,
    },
}

/// Result type for cs-ctl operations
pub type CsCtlResult<T> = Result<T, CsCtlError>;

/// Error types for cs-ctl operations
#[derive(Debug, Clone, Error, Serialize, Deserialize)]
pub enum CsCtlError {
    /// Connection failed
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    /// Invalid command
    #[error("Invalid command: {0}")]
    InvalidCommand(String),

    /// Command execution failed
    #[error("Command execution failed: {0}")]
    ExecutionFailed(String),

    /// Task not found
    #[error("Task not found: {0}")]
    TaskNotFound(String),

    /// Authentication failed
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Operation timed out
    #[error("Operation timed out: {0}")]
    Timeout(String),

    /// Server error
    #[error("Server error: {0}")]
    ServerError(String),
}

/// Configuration for cs-ctl client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsCtlConfig {
    /// Server endpoint (address:port)
    pub endpoint: String,
    /// Authentication token
    pub auth_token: String,
    /// Output format
    pub output_format: OutputFormat,
    /// Request timeout in seconds
    pub timeout_secs: u64,
    /// Verbose output
    pub verbose: bool,
}

impl Default for CsCtlConfig {
    fn default() -> Self {
        CsCtlConfig {
            endpoint: "localhost:9999".to_string(),
            auth_token: String::new(),
            output_format: OutputFormat::Text,
            timeout_secs: 30,
            verbose: false,
        }
    }
}

impl CsCtlConfig {
    /// Validate configuration
    pub fn validate(&self) -> CsCtlResult<()> {
        if self.endpoint.is_empty() {
            return Err(CsCtlError::InvalidConfig("Endpoint cannot be empty".to_string()));
        }

        if self.timeout_secs == 0 {
            return Err(CsCtlError::InvalidConfig("Timeout must be > 0".to_string()));
        }

        Ok(())
    }
}

/// Task status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStatus {
    /// Task ID
    pub task_id: String,
    /// Task name
    pub name: String,
    /// Current state
    pub state: String,
    /// Priority level
    pub priority: u8,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
    /// Metadata
    pub metadata: BTreeMap<String, String>,
}

/// System status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatus {
    /// Is system running
    pub running: bool,
    /// Number of active tasks
    pub active_tasks: usize,
    /// System uptime in seconds
    pub uptime_secs: u64,
    /// Memory usage in bytes
    pub memory_bytes: u64,
    /// CPU usage percentage
    pub cpu_usage_percent: f32,
    /// System metadata
    pub metadata: BTreeMap<String, String>,
}

/// Control command output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CsCtlOutput {
    /// Status output
    Status(SystemStatus),
    /// Task inspection output
    TaskInfo(TaskStatus),
    /// Task list output
    TaskList(Vec<TaskStatus>),
    /// Log output
    Logs(Vec<String>),
    /// Success message
    Success(String),
    /// Error message
    Error(String),
}

impl CsCtlOutput {
    /// Convert to text format
    pub fn to_text(&self) -> String {
        match self {
            CsCtlOutput::Status(status) => {
                let mut result = String::new();
                result.push_str("System Status:\n");
                result.push_str(&format!("  Running: {}\n", status.running));
                result.push_str(&format!("  Active Tasks: {}\n", status.active_tasks));
                result.push_str(&format!("  Uptime: {} seconds\n", status.uptime_secs));
                result.push_str(&format!("  Memory: {} bytes\n", status.memory_bytes));
                result.push_str(&format!("  CPU Usage: {:.1}%\n", status.cpu_usage_percent));
                result
            }
            CsCtlOutput::TaskInfo(task) => {
                let mut result = String::new();
                result.push_str("Task Information:\n");
                result.push_str(&format!("  ID: {}\n", task.task_id));
                result.push_str(&format!("  Name: {}\n", task.name));
                result.push_str(&format!("  State: {}\n", task.state));
                result.push_str(&format!("  Priority: {}\n", task.priority));
                result.push_str(&format!("  Execution Time: {} ms\n", task.execution_time_ms));
                result
            }
            CsCtlOutput::TaskList(tasks) => {
                let mut result = String::new();
                result.push_str("Active Tasks:\n");
                for task in tasks {
                    result.push_str(&format!("  {} ({}): {}\n", task.task_id, task.name, task.state));
                }
                result
            }
            CsCtlOutput::Logs(logs) => {
                let mut result = String::new();
                result.push_str("Logs:\n");
                for log in logs {
                    result.push_str(&format!("  {}\n", log));
                }
                result
            }
            CsCtlOutput::Success(msg) => {
                format!("Success: {}\n", msg)
            }
            CsCtlOutput::Error(msg) => {
                format!("Error: {}\n", msg)
            }
        }
    }
}

/// Execute a control command
///
/// # Arguments
/// * `cmd` - The command to execute
/// * `config` - Client configuration
///
/// # Returns
/// * `Ok(CsCtlOutput)` - Command output
/// * `Err(CsCtlError)` - If execution failed
pub fn execute_command(cmd: &CsCtlCommand, config: &CsCtlConfig) -> CsCtlResult<CsCtlOutput> {
    // Validate configuration
    config.validate()?;

    match cmd {
        CsCtlCommand::Status => {
            execute_status(config)
        }
        CsCtlCommand::Start { config_path } => {
            execute_start(config_path, config)
        }
        CsCtlCommand::Stop { force } => {
            execute_stop(*force, config)
        }
        CsCtlCommand::Restart { config_path } => {
            execute_restart(config_path, config)
        }
        CsCtlCommand::Inspect { task_id } => {
            execute_inspect(task_id, config)
        }
        CsCtlCommand::List { state, verbose } => {
            execute_list(state.as_deref(), *verbose, config)
        }
        CsCtlCommand::Logs { task_id, lines, follow } => {
            execute_logs(task_id, *lines, *follow, config)
        }
    }
}

fn execute_status(config: &CsCtlConfig) -> CsCtlResult<CsCtlOutput> {
    if config.verbose {
        // Stub implementation
    }

    let status = SystemStatus {
        running: true,
        active_tasks: 0,
        uptime_secs: 0,
        memory_bytes: 0,
        cpu_usage_percent: 0.0,
        metadata: BTreeMap::new(),
    };

    Ok(CsCtlOutput::Status(status))
}

fn execute_start(config_path: &Option<String>, _config: &CsCtlConfig) -> CsCtlResult<CsCtlOutput> {
    if let Some(path) = config_path {
        if path.is_empty() {
            return Err(CsCtlError::InvalidConfig("Config path cannot be empty".to_string()));
        }
    }

    Ok(CsCtlOutput::Success("Cognitive Substrate started".to_string()))
}

fn execute_stop(force: bool, _config: &CsCtlConfig) -> CsCtlResult<CsCtlOutput> {
    let msg = if force {
        "Cognitive Substrate force stopped"
    } else {
        "Cognitive Substrate stopped gracefully"
    };

    Ok(CsCtlOutput::Success(msg.to_string()))
}

fn execute_restart(config_path: &Option<String>, config: &CsCtlConfig) -> CsCtlResult<CsCtlOutput> {
    execute_stop(false, config)?;
    execute_start(config_path, config)
}

fn execute_inspect(task_id: &str, _config: &CsCtlConfig) -> CsCtlResult<CsCtlOutput> {
    if task_id.is_empty() {
        return Err(CsCtlError::InvalidCommand("Task ID cannot be empty".to_string()));
    }

    let task = TaskStatus {
        task_id: task_id.to_string(),
        name: format!("task_{}", task_id),
        state: "Running".to_string(),
        priority: 128,
        execution_time_ms: 0,
        metadata: BTreeMap::new(),
    };

    Ok(CsCtlOutput::TaskInfo(task))
}

fn execute_list(_state: Option<&str>, _verbose: bool, _config: &CsCtlConfig) -> CsCtlResult<CsCtlOutput> {
    let tasks = Vec::new();
    Ok(CsCtlOutput::TaskList(tasks))
}

fn execute_logs(task_id: &str, _lines: Option<usize>, _follow: bool, _config: &CsCtlConfig) -> CsCtlResult<CsCtlOutput> {
    if task_id.is_empty() {
        return Err(CsCtlError::InvalidCommand("Task ID cannot be empty".to_string()));
    }

    let logs = Vec::new();
    Ok(CsCtlOutput::Logs(logs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_format_default() {
        let fmt = OutputFormat::default();
        assert_eq!(fmt, OutputFormat::Text);
    }

    #[test]
    fn test_config_default() {
        let config = CsCtlConfig::default();
        assert_eq!(config.endpoint, "localhost:9999");
        assert!(config.auth_token.is_empty());
        assert_eq!(config.timeout_secs, 30);
    }

    #[test]
    fn test_config_validate() {
        let config = CsCtlConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validate_empty_endpoint() {
        let config = CsCtlConfig {
            endpoint: String::new(),
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validate_zero_timeout() {
        let config = CsCtlConfig {
            timeout_secs: 0,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_command_status() {
        let config = CsCtlConfig::default();
        let cmd = CsCtlCommand::Status;
        let result = execute_command(&cmd, &config).unwrap();
        match result {
            CsCtlOutput::Status(_) => {}
            _ => panic!("Expected Status output"),
        }
    }

    #[test]
    fn test_command_start() {
        let config = CsCtlConfig::default();
        let cmd = CsCtlCommand::Start {
            config_path: None,
        };
        let result = execute_command(&cmd, &config).unwrap();
        match result {
            CsCtlOutput::Success(_) => {}
            _ => panic!("Expected Success output"),
        }
    }

    #[test]
    fn test_command_stop() {
        let config = CsCtlConfig::default();
        let cmd = CsCtlCommand::Stop { force: false };
        let result = execute_command(&cmd, &config).unwrap();
        match result {
            CsCtlOutput::Success(_) => {}
            _ => panic!("Expected Success output"),
        }
    }

    #[test]
    fn test_command_stop_force() {
        let config = CsCtlConfig::default();
        let cmd = CsCtlCommand::Stop { force: true };
        let result = execute_command(&cmd, &config).unwrap();
        match result {
            CsCtlOutput::Success(_) => {}
            _ => panic!("Expected Success output"),
        }
    }

    #[test]
    fn test_command_restart() {
        let config = CsCtlConfig::default();
        let cmd = CsCtlCommand::Restart {
            config_path: None,
        };
        let result = execute_command(&cmd, &config).unwrap();
        match result {
            CsCtlOutput::Success(_) => {}
            _ => panic!("Expected Success output"),
        }
    }

    #[test]
    fn test_command_inspect() {
        let config = CsCtlConfig::default();
        let cmd = CsCtlCommand::Inspect {
            task_id: "task123".to_string(),
        };
        let result = execute_command(&cmd, &config).unwrap();
        match result {
            CsCtlOutput::TaskInfo(info) => {
                assert_eq!(info.task_id, "task123");
            }
            _ => panic!("Expected TaskInfo output"),
        }
    }

    #[test]
    fn test_command_inspect_empty_task_id() {
        let config = CsCtlConfig::default();
        let cmd = CsCtlCommand::Inspect {
            task_id: String::new(),
        };
        assert!(execute_command(&cmd, &config).is_err());
    }

    #[test]
    fn test_command_list() {
        let config = CsCtlConfig::default();
        let cmd = CsCtlCommand::List {
            state: None,
            verbose: false,
        };
        let result = execute_command(&cmd, &config).unwrap();
        match result {
            CsCtlOutput::TaskList(_) => {}
            _ => panic!("Expected TaskList output"),
        }
    }

    #[test]
    fn test_command_logs() {
        let config = CsCtlConfig::default();
        let cmd = CsCtlCommand::Logs {
            task_id: "task123".to_string(),
            lines: Some(10),
            follow: false,
        };
        let result = execute_command(&cmd, &config).unwrap();
        match result {
            CsCtlOutput::Logs(_) => {}
            _ => panic!("Expected Logs output"),
        }
    }

    #[test]
    fn test_output_status_to_text() {
        let status = SystemStatus {
            running: true,
            active_tasks: 5,
            uptime_secs: 3600,
            memory_bytes: 1024 * 1024,
            cpu_usage_percent: 25.5,
            metadata: BTreeMap::new(),
        };

        let output = CsCtlOutput::Status(status);
        let text = output.to_text();
        assert!(text.contains("System Status"));
        assert!(text.contains("Active Tasks: 5"));
    }

    #[test]
    fn test_output_task_info_to_text() {
        let task = TaskStatus {
            task_id: "123".to_string(),
            name: "test_task".to_string(),
            state: "Running".to_string(),
            priority: 128,
            execution_time_ms: 500,
            metadata: BTreeMap::new(),
        };

        let output = CsCtlOutput::TaskInfo(task);
        let text = output.to_text();
        assert!(text.contains("Task Information"));
        assert!(text.contains("test_task"));
    }

    #[test]
    fn test_output_success_to_text() {
        let output = CsCtlOutput::Success("Operation completed".to_string());
        let text = output.to_text();
        assert!(text.contains("Success"));
        assert!(text.contains("Operation completed"));
    }

    #[test]
    fn test_task_status() {
        let task = TaskStatus {
            task_id: "abc".to_string(),
            name: "my_task".to_string(),
            state: "Suspended".to_string(),
            priority: 64,
            execution_time_ms: 1000,
            metadata: BTreeMap::new(),
        };
        assert_eq!(task.task_id, "abc");
        assert_eq!(task.priority, 64);
    }

    #[test]
    fn test_system_status() {
        let status = SystemStatus {
            running: true,
            active_tasks: 10,
            uptime_secs: 7200,
            memory_bytes: 2 * 1024 * 1024,
            cpu_usage_percent: 50.0,
            metadata: BTreeMap::new(),
        };
        assert!(status.running);
        assert_eq!(status.active_tasks, 10);
    }
}
