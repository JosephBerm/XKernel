//! API request/response models for the cs-daemon REST API.
//!
//! These are the external-facing types that clients interact with.
//! They bridge between the HTTP JSON world and the internal kernel types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── Agent Models ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateAgentRequest {
    /// Human-readable agent name
    pub name: String,
    /// Framework type: "langchain", "crewai", "autogen", "semantic_kernel", "custom"
    #[serde(default = "default_framework")]
    pub framework: String,
    /// Command to execute as the agent process (e.g., "python agent.py")
    pub entrypoint: Option<String>,
    /// Working directory for the agent process
    pub working_dir: Option<String>,
    /// Environment variables for the agent process
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// Priority: 0-255 (higher = more urgent)
    #[serde(default = "default_priority")]
    pub priority: u8,
    /// Restart policy: "never", "on_failure", "always"
    #[serde(default = "default_restart_policy")]
    pub restart_policy: String,
    /// Maximum restart attempts (if restart_policy != "never")
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    /// Capabilities to grant: ["task", "memory", "tool", "channel", "telemetry"]
    #[serde(default)]
    pub capabilities: Vec<String>,
}

fn default_framework() -> String {
    "custom".to_string()
}
fn default_priority() -> u8 {
    128
}
fn default_restart_policy() -> String {
    "never".to_string()
}
fn default_max_retries() -> u32 {
    3
}

#[derive(Debug, Serialize)]
pub struct AgentResponse {
    pub id: String,
    pub name: String,
    pub framework: String,
    pub state: String,
    pub pid: Option<u32>,
    pub created_at: String,
    pub started_at: Option<String>,
    pub uptime_seconds: Option<f64>,
    pub restart_count: u32,
    pub capabilities: Vec<String>,
    pub task_phase: String,
    pub scheduler_position: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct AgentListResponse {
    pub agents: Vec<AgentResponse>,
    pub total: usize,
}

#[derive(Debug, Deserialize)]
pub struct SignalAgentRequest {
    /// Signal type: "stop", "restart", "checkpoint", "yield"
    pub signal: String,
    /// Optional reason
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AgentLogsResponse {
    pub agent_id: String,
    pub lines: Vec<LogLine>,
}

#[derive(Debug, Serialize)]
pub struct LogLine {
    pub timestamp: String,
    pub stream: String, // "stdout" or "stderr"
    pub message: String,
}

// ─── Channel Models ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateChannelRequest {
    /// Sender agent ID
    pub sender: String,
    /// Receiver agent ID
    pub receiver: String,
    /// Maximum message queue capacity
    #[serde(default = "default_capacity")]
    pub capacity: usize,
}

fn default_capacity() -> usize {
    256
}

#[derive(Debug, Serialize)]
pub struct ChannelResponse {
    pub id: u64,
    pub sender: String,
    pub receiver: String,
    pub capacity: usize,
    pub pending_messages: usize,
    pub is_closed: bool,
}

#[derive(Debug, Serialize)]
pub struct ChannelListResponse {
    pub channels: Vec<ChannelResponse>,
    pub total: usize,
}

#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    /// Message payload (arbitrary JSON or string)
    pub payload: String,
}

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub sender: String,
    pub receiver: String,
    pub payload: String,
    pub sequence: u64,
    pub timestamp: u64,
}

// ─── Memory Models ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct AllocateMemoryRequest {
    /// Number of pages to allocate (each page = 4096 bytes)
    pub pages: u64,
    /// Owner CT ID
    pub owner_ct_id: u32,
}

#[derive(Debug, Serialize)]
pub struct AllocationResponse {
    pub allocation_id: u64,
    pub pages: u64,
    pub size_bytes: u64,
    pub owner_ct_id: u32,
}

#[derive(Debug, Serialize)]
pub struct MemoryStatsResponse {
    pub total_pages: usize,
    pub allocated_pages: u64,
    pub free_pages: u64,
    pub active_allocations: usize,
    pub page_size_bytes: u64,
}

#[derive(Debug, Deserialize)]
pub struct FreeMemoryRequest {
    pub allocation_id: u64,
}

// ─── Tool Registry Models ────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct RegisterToolRequest {
    /// Tool name
    pub name: String,
    /// Tool description
    #[serde(default)]
    pub description: String,
    /// Input JSON schema (as string)
    #[serde(default = "default_schema")]
    pub input_schema: String,
    /// Output JSON schema (as string)
    #[serde(default = "default_schema")]
    pub output_schema: String,
    /// Effect class: "read_only", "write_reversible", "write_irreversible"
    #[serde(default = "default_effect_class")]
    pub effect_class: String,
    /// Agent ID that owns this tool binding
    pub agent_id: Option<String>,
}

fn default_schema() -> String {
    "{}".to_string()
}
fn default_effect_class() -> String {
    "read_only".to_string()
}

#[derive(Debug, Serialize)]
pub struct ToolResponse {
    pub binding_id: String,
    pub name: String,
    pub effect_class: String,
    pub agent_id: String,
}

#[derive(Debug, Serialize)]
pub struct ToolListResponse {
    pub tools: Vec<ToolResponse>,
    pub total: usize,
}

// ─── System Models ───────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_seconds: f64,
}

#[derive(Debug, Serialize)]
pub struct MetricsResponse {
    pub uptime_seconds: f64,
    pub agents: AgentMetrics,
    pub scheduler: SchedulerMetrics,
    pub channels: ChannelMetrics,
    pub memory: MemoryMetrics,
    pub tools: ToolMetrics,
    pub telemetry: TelemetryMetrics,
}

#[derive(Debug, Serialize)]
pub struct AgentMetrics {
    pub total_created: u64,
    pub active: usize,
    pub running: usize,
    pub stopped: usize,
    pub failed: usize,
}

#[derive(Debug, Serialize)]
pub struct SchedulerMetrics {
    pub queue_depth: usize,
    pub total_scheduled: u64,
    pub total_completed: u64,
}

#[derive(Debug, Serialize)]
pub struct ChannelMetrics {
    pub active_channels: usize,
    pub total_messages_sent: u64,
    pub total_messages_received: u64,
}

#[derive(Debug, Serialize)]
pub struct MemoryMetrics {
    pub total_allocations: u64,
    pub active_allocations: usize,
    pub total_bytes_allocated: u64,
}

#[derive(Debug, Serialize)]
pub struct ToolMetrics {
    pub registered_tools: usize,
    pub total_invocations: u64,
}

#[derive(Debug, Serialize)]
pub struct TelemetryMetrics {
    pub total_events: u64,
    pub events_per_type: HashMap<String, u64>,
}

#[derive(Debug, Serialize)]
pub struct EventResponse {
    pub events: Vec<SystemEvent>,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemEvent {
    pub timestamp: String,
    pub event_type: String,
    pub agent_id: Option<String>,
    pub details: String,
}
