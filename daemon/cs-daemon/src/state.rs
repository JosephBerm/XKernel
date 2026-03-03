//! Shared application state that wraps all kernel subsystems.
//!
//! This is the heart of the daemon — it holds real instances of every
//! kernel type and exposes them through thread-safe shared state.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;

use ct_lifecycle::{TaskStateMachine, PriorityScheduler};
use capability_engine::{PolicyEngine, Capability};
use ipc_signals_exceptions::Channel;
use cs_tool_registry_telemetry::ToolRegistry;

use crate::models::SystemEvent;

/// Maximum log lines retained per agent.
const MAX_LOG_LINES: usize = 1000;

/// Thread-safe shared state, wrapped in Arc<RwLock<>> for concurrent access.
pub type SharedState = Arc<RwLock<AppState>>;

/// The complete daemon state, holding real kernel subsystem instances.
pub struct AppState {
    // ── Agent management ──
    pub agents: HashMap<String, ManagedAgent>,
    pub next_task_id: u64,

    // ── L0 Kernel: Scheduler ──
    pub scheduler: PriorityScheduler,

    // ── L0 Kernel: Capability engine ──
    pub policy_engine: PolicyEngine,
    pub capabilities: HashMap<u64, Capability>,
    pub next_cap_id: u64,

    // ── L0 Kernel: IPC ──
    pub channels: HashMap<u64, ManagedChannel>,
    pub next_channel_id: u64,

    // ── L1 Services: Tool registry ──
    pub tool_registry: ToolRegistry,

    // ── Telemetry & metrics ──
    pub events: Vec<SystemEvent>,
    pub start_time: std::time::Instant,

    // ── Counters ──
    pub total_agents_created: u64,
    pub total_messages_sent: u64,
    pub total_messages_received: u64,
    pub total_scheduled: u64,
    pub total_completed: u64,
    pub total_allocations: u64,
    pub total_bytes_allocated: u64,
    pub total_tool_invocations: u64,
}

/// A managed agent with its kernel state machine and optional OS process.
pub struct ManagedAgent {
    pub id: String,
    pub name: String,
    pub framework: String,
    pub entrypoint: Option<String>,
    pub working_dir: Option<String>,
    pub env: HashMap<String, String>,
    pub state: AgentState,
    pub task_state_machine: TaskStateMachine,
    pub task_id: u64,
    pub pid: Option<u32>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub restart_count: u32,
    pub max_retries: u32,
    pub restart_policy: RestartPolicy,
    pub capabilities: Vec<String>,
    pub logs: VecDeque<LogEntry>,
    pub process_handle: Option<tokio::process::Child>,
}

/// Agent lifecycle state (maps to kernel TaskState but adds process-level states).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentState {
    Created,
    Starting,
    Running,
    Stopping,
    Stopped,
    Failed,
    Restarting,
}

impl std::fmt::Display for AgentState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentState::Created => write!(f, "created"),
            AgentState::Starting => write!(f, "starting"),
            AgentState::Running => write!(f, "running"),
            AgentState::Stopping => write!(f, "stopping"),
            AgentState::Stopped => write!(f, "stopped"),
            AgentState::Failed => write!(f, "failed"),
            AgentState::Restarting => write!(f, "restarting"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestartPolicy {
    Never,
    OnFailure,
    Always,
}

impl RestartPolicy {
    pub fn from_str(s: &str) -> Self {
        match s {
            "on_failure" => RestartPolicy::OnFailure,
            "always" => RestartPolicy::Always,
            _ => RestartPolicy::Never,
        }
    }
}

use serde::{Serialize, Deserialize};

/// A log entry captured from an agent's stdout/stderr.
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub stream: String,
    pub message: String,
}

/// A managed IPC channel wrapping the kernel Channel type.
pub struct ManagedChannel {
    pub channel: Channel,
    pub sender_agent: String,
    pub receiver_agent: String,
}

impl AppState {
    /// Create a new AppState with all kernel subsystems initialized.
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
            next_task_id: 1,
            scheduler: PriorityScheduler::new(),
            policy_engine: PolicyEngine::new(),
            capabilities: HashMap::new(),
            next_cap_id: 1,
            channels: HashMap::new(),
            next_channel_id: 1,
            tool_registry: ToolRegistry::new(),
            events: Vec::new(),
            start_time: std::time::Instant::now(),
            total_agents_created: 0,
            total_messages_sent: 0,
            total_messages_received: 0,
            total_scheduled: 0,
            total_completed: 0,
            total_allocations: 0,
            total_bytes_allocated: 0,
            total_tool_invocations: 0,
        }
    }

    /// Record a system event for telemetry.
    pub fn record_event(&mut self, event_type: &str, agent_id: Option<&str>, details: &str) {
        let event = SystemEvent {
            timestamp: chrono::Utc::now().to_rfc3339(),
            event_type: event_type.to_string(),
            agent_id: agent_id.map(|s| s.to_string()),
            details: details.to_string(),
        };
        tracing::info!(
            event_type = %event.event_type,
            agent_id = ?event.agent_id,
            details = %event.details,
            "system_event"
        );
        self.events.push(event);
    }

    /// Get uptime in seconds.
    pub fn uptime_seconds(&self) -> f64 {
        self.start_time.elapsed().as_secs_f64()
    }

    /// Push a log line to an agent's log buffer.
    pub fn push_agent_log(&mut self, agent_id: &str, stream: &str, message: &str) {
        if let Some(agent) = self.agents.get_mut(agent_id) {
            if agent.logs.len() >= MAX_LOG_LINES {
                agent.logs.pop_front();
            }
            agent.logs.push_back(LogEntry {
                timestamp: chrono::Utc::now(),
                stream: stream.to_string(),
                message: message.to_string(),
            });
        }
    }
}
