// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! # CSCI Common Types
//!
//! Type definitions used across multiple syscall families.
//!
//! # Engineering Plan Reference
//! Section 5: CSCI Common Types.

use core::fmt;

/// Identifier for a Cognitive Task (CT).
///
/// Globally unique identifier assigned at task creation and immutable
/// throughout the task's lifetime.
///
/// # Engineering Plan Reference
/// Section 5.1: Task identification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CTID(pub u64);

impl fmt::Display for CTID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CT-{:x}", self.0)
    }
}

/// Identifier for an Agent.
///
/// An agent is a persistent entity that creates and manages cognitive tasks.
/// Agent IDs are assigned by the system and remain constant across sessions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AgentID(pub u64);

impl fmt::Display for AgentID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AG-{:x}", self.0)
    }
}

/// Identifier for a memory region.
///
/// References a contiguous semantic memory region allocated via mem_alloc
/// or mem_mount syscalls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MemoryRegionID(pub u64);

impl fmt::Display for MemoryRegionID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MEM-{:x}", self.0)
    }
}

/// Identifier for a checkpoint.
///
/// References a saved state checkpoint of a cognitive task created via
/// ct_checkpoint syscall.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CheckpointID(pub u64);

impl fmt::Display for CheckpointID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CP-{:x}", self.0)
    }
}

/// Configuration for creating a new cognitive task.
///
/// Specifies parameters for task creation including timeouts, priority,
/// and behavior hints.
///
/// # Engineering Plan Reference
/// Section 5.2: Task configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CTConfig {
    /// Task name for debugging and identification (max 64 bytes).
    pub name: [u8; 64],
    /// Task name length.
    pub name_len: usize,
    /// Maximum wall-clock time in milliseconds, 0 = unlimited.
    pub timeout_ms: u64,
    /// Task priority level (0-255, higher = more important).
    pub priority: u8,
}

impl CTConfig {
    /// Create a new task configuration with defaults.
    pub fn new() -> Self {
        Self {
            name: [0; 64],
            name_len: 0,
            timeout_ms: 0,
            priority: 128,
        }
    }

    /// Set the task name.
    pub fn with_name(mut self, name: &[u8]) -> Self {
        let len = core::cmp::min(name.len(), 64);
        self.name[..len].copy_from_slice(&name[..len]);
        self.name_len = len;
        self
    }

    /// Set the timeout in milliseconds.
    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Set the priority level.
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    /// Get the task name as a string slice (if valid UTF-8).
    pub fn name_str(&self) -> Option<&str> {
        core::str::from_utf8(&self.name[..self.name_len]).ok()
    }
}

impl Default for CTConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Hint for yield behavior during task execution.
///
/// Provides information to the scheduler about why the task is yielding
/// and what it's waiting for.
///
/// # Engineering Plan Reference
/// Section 5.3: Yield hints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum YieldHint {
    /// Task needs more reasoning time; re-schedule immediately.
    MoreThinking,
    /// Waiting for external event (I/O, message, etc.).
    WaitingForEvent,
    /// Waiting for child tasks to complete.
    WaitingForChildren,
    /// Voluntary preemption; schedule next task.
    VoluntaryPreemption,
    /// Waiting with deadline (store deadline separately).
    WaitingWithDeadline { deadline_ms: u64 },
}

impl fmt::Display for YieldHint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MoreThinking => write!(f, "MoreThinking"),
            Self::WaitingForEvent => write!(f, "WaitingForEvent"),
            Self::WaitingForChildren => write!(f, "WaitingForChildren"),
            Self::VoluntaryPreemption => write!(f, "VoluntaryPreemption"),
            Self::WaitingWithDeadline { deadline_ms } => {
                write!(f, "WaitingWithDeadline({}ms)", deadline_ms)
            }
        }
    }
}

/// Checkpoint type indicating what is being preserved.
///
/// Different checkpoint types capture different levels of task state,
/// affecting recovery behavior and storage overhead.
///
/// # Engineering Plan Reference
/// Section 5.4: Checkpoint types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckpointType {
    /// Full task state: reasoning, actions, memory references, all output.
    Full,
    /// Reasoning checkpoint: only reasoning state and outputs.
    ReasoningOnly,
    /// Memory checkpoint: memory state and references only.
    MemoryOnly,
}

impl fmt::Display for CheckpointType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Full => write!(f, "Full"),
            Self::ReasoningOnly => write!(f, "ReasoningOnly"),
            Self::MemoryOnly => write!(f, "MemoryOnly"),
        }
    }
}

/// Memory tier indicating storage location and access characteristics.
///
/// The cognitive substrate manages semantic memory across multiple tiers
/// with different access times and capacity characteristics.
///
/// # Engineering Plan Reference
/// Section 5.5: Memory tiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemoryTier {
    /// Level 1: Fast, limited capacity (e.g., working memory). ~1-10 MB typical.
    L1,
    /// Level 2: Medium capacity, medium latency (e.g., task-local memory). ~100-1000 MB typical.
    L2,
    /// Level 3: Large capacity, slower latency (e.g., persistent semantic memory). ~1 GB+.
    L3,
}

impl fmt::Display for MemoryTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::L1 => write!(f, "L1"),
            Self::L2 => write!(f, "L2"),
            Self::L3 => write!(f, "L3"),
        }
    }
}

/// Memory slice: a view into a memory region.
///
/// Represents a contiguous sequence of bytes from a memory region,
/// used for mem_read and mem_write operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemorySlice {
    /// The bytes in this slice.
    pub data: alloc::vec::Vec<u8>,
}

impl MemorySlice {
    /// Create a new memory slice from a byte vector.
    pub fn new(data: alloc::vec::Vec<u8>) -> Self {
        Self { data }
    }

    /// Create a memory slice from a byte slice by copying.
    pub fn from_slice(data: &[u8]) -> Self {
        Self {
            data: alloc::vec::Vec::from(data),
        }
    }

    /// Get the length of this slice.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if the slice is empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Get as byte slice.
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Get as mutable byte slice.
    pub fn as_mut_bytes(&mut self) -> &mut [u8] {
        &mut self.data
    }
}

/// Reference to an external knowledge source.
///
/// Identifies a knowledge base, embedding, or semantic resource that can
/// be mounted into the memory system via mem_mount.
///
/// # Engineering Plan Reference
/// Section 5.6: Knowledge source references.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeSourceRef {
    /// URI or identifier for the knowledge source.
    pub source_id: alloc::string::String,
}

impl KnowledgeSourceRef {
    /// Create a new knowledge source reference.
    pub fn new(source_id: alloc::string::String) -> Self {
        Self { source_id }
    }

    /// Create from a string slice.
    pub fn from_str(source_id: &str) -> Self {
        Self {
            source_id: alloc::string::String::from(source_id),
        }
    }
}

/// Mount point for a knowledge source in the memory hierarchy.
///
/// Specifies where in the memory namespace a knowledge source is attached,
/// enabling address-like access patterns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MountPoint {
    /// Path-like mount point (e.g., "/knowledge/embeddings/v1").
    pub path: alloc::string::String,
}

impl MountPoint {
    /// Create a new mount point.
    pub fn new(path: alloc::string::String) -> Self {
        Self { path }
    }

    /// Create from a string slice.
    pub fn from_str(path: &str) -> Self {
        Self {
            path: alloc::string::String::from(path),
        }
    }
}

/// Access mode for memory operations.
///
/// Controls whether a memory region or mounted knowledge source supports
/// read, write, or both operations.
///
/// # Engineering Plan Reference
/// Section 5.7: Access modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessMode {
    /// Read-only access.
    ReadOnly,
    /// Write-only access (rare, used for output buffers).
    WriteOnly,
    /// Read and write access.
    ReadWrite,
}

impl fmt::Display for AccessMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadOnly => write!(f, "ReadOnly"),
            Self::WriteOnly => write!(f, "WriteOnly"),
            Self::ReadWrite => write!(f, "ReadWrite"),
        }
    }
}

/// Resource quota for a cognitive task.
///
/// Specifies limits on resources (memory, compute time, etc.) that a task
/// can consume.
///
/// # Engineering Plan Reference
/// Section 5.8: Resource quotas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceQuota {
    /// Maximum memory allocation in bytes.
    pub max_memory_bytes: u64,
    /// Maximum compute budget in milliseconds.
    pub max_compute_ms: u64,
    /// Maximum number of child tasks.
    pub max_child_tasks: u64,
}

impl ResourceQuota {
    /// Create a new resource quota with specified limits.
    pub fn new(max_memory_bytes: u64, max_compute_ms: u64, max_child_tasks: u64) -> Self {
        Self {
            max_memory_bytes,
            max_compute_ms,
            max_child_tasks,
        }
    }

    /// Create an unlimited resource quota.
    pub fn unlimited() -> Self {
        Self {
            max_memory_bytes: u64::MAX,
            max_compute_ms: u64::MAX,
            max_child_tasks: u64::MAX,
        }
    }
}

/// Capability set representing granted permissions.
///
/// A bitfield indicating which syscalls an agent can invoke.
///
/// # Engineering Plan Reference
/// Section 5.9: Capability sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CapabilitySet(pub u64);

impl CapabilitySet {
    /// Create an empty capability set.
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Create a full capability set.
    pub const fn full() -> Self {
        Self(u64::MAX)
    }

    /// Create a capability set with specific flags.
    pub const fn new(flags: u64) -> Self {
        Self(flags)
    }

    /// Check if a specific capability is granted.
    pub const fn has(&self, bit: u32) -> bool {
        if bit >= 64 {
            return false;
        }
        (self.0 & (1 << bit)) != 0
    }

    /// Add a capability.
    pub fn grant(&mut self, bit: u32) {
        if bit < 64 {
            self.0 |= 1 << bit;
        }
    }

    /// Remove a capability.
    pub fn revoke(&mut self, bit: u32) {
        if bit < 64 {
            self.0 &= !(1 << bit);
        }
    }

    /// Capability bit for task family syscalls.
    pub const CAP_TASK_FAMILY: u32 = 0;
    /// Capability bit for memory family syscalls.
    pub const CAP_MEMORY_FAMILY: u32 = 1;
    /// Capability bit for tool family syscalls.
    pub const CAP_TOOL_FAMILY: u32 = 2;
    /// Capability bit for channel family syscalls.
    pub const CAP_CHANNEL_FAMILY: u32 = 3;
    /// Capability bit for context family syscalls.
    pub const CAP_CONTEXT_FAMILY: u32 = 4;
    /// Capability bit for capability family syscalls.
    pub const CAP_CAPABILITY_FAMILY: u32 = 5;
    /// Capability bit for signals family syscalls.
    pub const CAP_SIGNALS_FAMILY: u32 = 6;
    /// Capability bit for crew family syscalls.
    pub const CAP_CREW_FAMILY: u32 = 7;
    /// Capability bit for telemetry family syscalls.
    pub const CAP_TELEMETRY_FAMILY: u32 = 8;
}

/// Identifier for an IPC channel.
///
/// References an inter-process communication channel created via chan_open.
///
/// # Engineering Plan Reference
/// Section 5.10: Channel identification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChannelID(pub u64);

impl fmt::Display for ChannelID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CH-{:x}", self.0)
    }
}

/// Protocol type for IPC channels.
///
/// Specifies the communication protocol and semantics for a channel.
///
/// # Engineering Plan Reference
/// Section 5.11: Channel protocol types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolType {
    /// Simple byte stream with no structure.
    ByteStream,
    /// Message-oriented with explicit boundaries.
    MessageBased,
    /// Request-response pattern (RPC-like).
    RequestResponse,
    /// Publish-subscribe pattern.
    PubSub,
}

impl fmt::Display for ProtocolType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ByteStream => write!(f, "ByteStream"),
            Self::MessageBased => write!(f, "MessageBased"),
            Self::RequestResponse => write!(f, "RequestResponse"),
            Self::PubSub => write!(f, "PubSub"),
        }
    }
}

/// Delivery guarantee for IPC messages.
///
/// Specifies the reliability and ordering guarantees for messages.
///
/// # Engineering Plan Reference
/// Section 5.12: Message delivery guarantees.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryGuarantee {
    /// Best-effort delivery, no ordering guarantee.
    BestEffort,
    /// At-least-once delivery with ordering.
    AtLeastOnce,
    /// Exactly-once delivery with ordering.
    ExactlyOnce,
}

impl fmt::Display for DeliveryGuarantee {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BestEffort => write!(f, "BestEffort"),
            Self::AtLeastOnce => write!(f, "AtLeastOnce"),
            Self::ExactlyOnce => write!(f, "ExactlyOnce"),
        }
    }
}

/// Typed message payload for IPC.
///
/// Represents a message with type information and binary payload.
///
/// # Engineering Plan Reference
/// Section 5.13: Message payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessagePayload {
    /// Message type identifier (application-defined).
    pub msg_type: u32,
    /// Serialized message data.
    pub data: alloc::vec::Vec<u8>,
}

impl MessagePayload {
    /// Create a new message payload.
    pub fn new(msg_type: u32, data: alloc::vec::Vec<u8>) -> Self {
        Self { msg_type, data }
    }

    /// Create from a byte slice.
    pub fn from_slice(msg_type: u32, data: &[u8]) -> Self {
        Self {
            msg_type,
            data: alloc::vec::Vec::from(data),
        }
    }

    /// Get the payload size in bytes.
    pub fn size(&self) -> usize {
        self.data.len()
    }
}

/// Identifier for a tool binding.
///
/// References a bound external tool created via tool_bind.
///
/// # Engineering Plan Reference
/// Section 5.14: Tool binding identification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ToolBindingID(pub u64);

impl fmt::Display for ToolBindingID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TOOL-{:x}", self.0)
    }
}

/// Specification for an external tool.
///
/// Describes a tool that can be bound and invoked, including its interface
/// and resource requirements.
///
/// # Engineering Plan Reference
/// Section 5.15: Tool specification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolSpec {
    /// Tool name/identifier.
    pub name: alloc::string::String,
    /// Tool version.
    pub version: alloc::string::String,
    /// Brief description of the tool.
    pub description: alloc::string::String,
    /// Tool type (e.g., "mcp_tool", "web_search", "code_executor").
    pub tool_type: alloc::string::String,
}

impl ToolSpec {
    /// Create a new tool specification.
    pub fn new(
        name: &str,
        version: &str,
        description: &str,
        tool_type: &str,
    ) -> Self {
        Self {
            name: alloc::string::String::from(name),
            version: alloc::string::String::from(version),
            description: alloc::string::String::from(description),
            tool_type: alloc::string::String::from(tool_type),
        }
    }
}

/// Sandbox configuration for tool execution.
///
/// Specifies resource limits, isolation level, and policy constraints
/// for a tool's execution environment.
///
/// # Engineering Plan Reference
/// Section 5.16: Sandbox configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxConfig {
    /// Maximum memory in bytes for the tool.
    pub max_memory_bytes: u64,
    /// Maximum execution time in milliseconds.
    pub timeout_ms: u64,
    /// Network access allowed.
    pub allow_network: bool,
    /// File system access level (0=none, 1=read-only, 2=read-write).
    pub fs_access_level: u8,
}

impl SandboxConfig {
    /// Create a new sandbox configuration with defaults (restrictive).
    pub fn new() -> Self {
        Self {
            max_memory_bytes: 100 * 1024 * 1024, // 100 MB default
            timeout_ms: 60000,                     // 60s default
            allow_network: false,
            fs_access_level: 0,
        }
    }

    /// Set maximum memory.
    pub fn with_max_memory(mut self, max_memory_bytes: u64) -> Self {
        self.max_memory_bytes = max_memory_bytes;
        self
    }

    /// Set timeout.
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Enable network access.
    pub fn with_network(mut self, allow: bool) -> Self {
        self.allow_network = allow;
        self
    }

    /// Set filesystem access level.
    pub fn with_fs_access(mut self, level: u8) -> Self {
        self.fs_access_level = core::cmp::min(level, 2);
        self
    }
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Arguments for tool invocation.
///
/// Encapsulates the arguments passed to a tool execution.
///
/// # Engineering Plan Reference
/// Section 5.17: Tool arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolArguments {
    /// Named arguments as key-value pairs.
    pub args: alloc::vec::Vec<(alloc::string::String, alloc::string::String)>,
}

impl ToolArguments {
    /// Create a new tool arguments container.
    pub fn new() -> Self {
        Self {
            args: alloc::vec::Vec::new(),
        }
    }

    /// Add an argument.
    pub fn add_arg(mut self, key: &str, value: &str) -> Self {
        self.args.push((
            alloc::string::String::from(key),
            alloc::string::String::from(value),
        ));
        self
    }

    /// Get an argument by key.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.args
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
}

impl Default for ToolArguments {
    fn default() -> Self {
        Self::new()
    }
}

/// Result from tool invocation.
///
/// Represents the output of a tool execution including status and payload.
///
/// # Engineering Plan Reference
/// Section 5.18: Tool result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolResult {
    /// Exit status code (0 = success).
    pub status: u32,
    /// Output data from the tool.
    pub output: alloc::vec::Vec<u8>,
    /// Optional error message.
    pub error_message: alloc::string::String,
}

impl ToolResult {
    /// Create a successful tool result.
    pub fn success(output: alloc::vec::Vec<u8>) -> Self {
        Self {
            status: 0,
            output,
            error_message: alloc::string::String::new(),
        }
    }

    /// Create a failed tool result.
    pub fn failure(status: u32, error_message: &str) -> Self {
        Self {
            status,
            output: alloc::vec::Vec::new(),
            error_message: alloc::string::String::from(error_message),
        }
    }

    /// Check if the result indicates success.
    pub fn is_success(&self) -> bool {
        self.status == 0
    }
}

/// Identifier for a capability.
///
/// References a capability granted via cap_grant or cap_delegate.
///
/// # Engineering Plan Reference
/// Section 5.19: Capability identification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CapabilityID(pub u64);

impl fmt::Display for CapabilityID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CAP-{:x}", self.0)
    }
}

/// Specification for a capability grant.
///
/// Describes what capability is being granted, including resource limits
/// and operational constraints.
///
/// # Engineering Plan Reference
/// Section 5.20: Capability specification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilitySpec {
    /// Capability name (e.g., "read_memory", "invoke_tool").
    pub name: alloc::string::String,
    /// Capability resource (e.g., memory region ID, tool ID).
    pub resource: alloc::string::String,
    /// Access level (0=none, 1=limited, 2=full).
    pub access_level: u8,
}

impl CapabilitySpec {
    /// Create a new capability specification.
    pub fn new(name: &str, resource: &str, access_level: u8) -> Self {
        Self {
            name: alloc::string::String::from(name),
            resource: alloc::string::String::from(resource),
            access_level: core::cmp::min(access_level, 2),
        }
    }
}

/// Constraints on a capability grant.
///
/// Specifies temporal, quantitative, and usage constraints on a capability.
///
/// # Engineering Plan Reference
/// Section 5.21: Capability constraints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapConstraints {
    /// Expiration time in milliseconds from grant (0 = no expiration).
    pub expiration_ms: u64,
    /// Maximum usage count (0 = unlimited).
    pub max_uses: u64,
    /// Delegatable to other agents.
    pub delegatable: bool,
    /// Can be revoked by granter.
    pub revocable: bool,
}

impl CapConstraints {
    /// Create new capability constraints.
    pub fn new(expiration_ms: u64, max_uses: u64, delegatable: bool, revocable: bool) -> Self {
        Self {
            expiration_ms,
            max_uses,
            delegatable,
            revocable,
        }
    }

    /// Create unlimited constraints.
    pub fn unlimited() -> Self {
        Self {
            expiration_ms: 0,
            max_uses: 0,
            delegatable: true,
            revocable: true,
        }
    }
}

impl Default for CapConstraints {
    fn default() -> Self {
        Self::unlimited()
    }
}

/// Attenuation specification for capability delegation.
///
/// Specifies how to reduce the scope or power of a capability when delegating.
///
/// # Engineering Plan Reference
/// Section 5.22: Attenuation specification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttenuationSpec {
    /// Reduce access level (e.g., from 2 to 1).
    pub new_access_level: u8,
    /// Reduce expiration time (in milliseconds from now).
    pub reduced_expiration_ms: u64,
    /// Reduce maximum uses.
    pub reduced_max_uses: u64,
    /// Delegate to further children.
    pub further_delegatable: bool,
}

impl AttenuationSpec {
    /// Create a new attenuation specification.
    pub fn new(
        new_access_level: u8,
        reduced_expiration_ms: u64,
        reduced_max_uses: u64,
        further_delegatable: bool,
    ) -> Self {
        Self {
            new_access_level: core::cmp::min(new_access_level, 2),
            reduced_expiration_ms,
            reduced_max_uses,
            further_delegatable,
        }
    }
}

/// Identifier for an agent crew.
///
/// References a crew of collaborating agents created via crew_create.
///
/// # Engineering Plan Reference
/// Section 5.23: Crew identification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CrewID(pub u64);

impl fmt::Display for CrewID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CREW-{:x}", self.0)
    }
}

/// Signal handler type.
///
/// Specifies the type of signal being handled.
///
/// # Engineering Plan Reference
/// Section 5.24: Signal types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalType {
    /// Task completion signal.
    TaskComplete,
    /// Channel message signal.
    ChannelMessage,
    /// Timeout signal.
    Timeout,
    /// Resource warning signal.
    ResourceWarning,
}

impl fmt::Display for SignalType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TaskComplete => write!(f, "TaskComplete"),
            Self::ChannelMessage => write!(f, "ChannelMessage"),
            Self::Timeout => write!(f, "Timeout"),
            Self::ResourceWarning => write!(f, "ResourceWarning"),
        }
    }
}

/// Exception type.
///
/// Specifies the type of exception being handled.
///
/// # Engineering Plan Reference
/// Section 5.25: Exception types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExceptionType {
    /// Memory access violation.
    MemoryViolation,
    /// Capability violation.
    CapabilityViolation,
    /// Task timeout.
    TaskTimeout,
    /// Assertion failure.
    AssertionFailure,
    /// Stack overflow.
    StackOverflow,
}

impl fmt::Display for ExceptionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MemoryViolation => write!(f, "MemoryViolation"),
            Self::CapabilityViolation => write!(f, "CapabilityViolation"),
            Self::TaskTimeout => write!(f, "TaskTimeout"),
            Self::AssertionFailure => write!(f, "AssertionFailure"),
            Self::StackOverflow => write!(f, "StackOverflow"),
        }
    }
}

/// Telemetry event for Cognitive Event Format (CEF).
///
/// Represents a system event for tracing and monitoring.
///
/// # Engineering Plan Reference
/// Section 5.26: Telemetry events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelemetryEvent {
    /// Event type identifier.
    pub event_type: u32,
    /// Event timestamp in milliseconds.
    pub timestamp_ms: u64,
    /// Event severity level (0=info, 1=warn, 2=error).
    pub severity: u8,
    /// Event message.
    pub message: alloc::string::String,
    /// Additional context key-value pairs.
    pub context: alloc::vec::Vec<(alloc::string::String, alloc::string::String)>,
}

impl TelemetryEvent {
    /// Create a new telemetry event.
    pub fn new(event_type: u32, timestamp_ms: u64, severity: u8, message: &str) -> Self {
        Self {
            event_type,
            timestamp_ms,
            severity: core::cmp::min(severity, 2),
            message: alloc::string::String::from(message),
            context: alloc::vec::Vec::new(),
        }
    }

    /// Add context information.
    pub fn with_context(mut self, key: &str, value: &str) -> Self {
        self.context.push((
            alloc::string::String::from(key),
            alloc::string::String::from(value),
        ));
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use alloc::vec;

    #[test]
    fn test_ctid_display() {
        let id = CTID(42);
        let s = id.to_string();
        assert!(s.contains("CT-"));
    }

    #[test]
    fn test_agent_id_display() {
        let id = AgentID(42);
        let s = id.to_string();
        assert!(s.contains("AG-"));
    }

    #[test]
    fn test_memory_region_id_display() {
        let id = MemoryRegionID(42);
        let s = id.to_string();
        assert!(s.contains("MEM-"));
    }

    #[test]
    fn test_checkpoint_id_display() {
        let id = CheckpointID(42);
        let s = id.to_string();
        assert!(s.contains("CP-"));
    }

    #[test]
    fn test_ct_config_new() {
        let cfg = CTConfig::new();
        assert_eq!(cfg.name_len, 0);
        assert_eq!(cfg.timeout_ms, 0);
        assert_eq!(cfg.priority, 128);
    }

    #[test]
    fn test_ct_config_builder() {
        let cfg = CTConfig::new()
            .with_name(b"test")
            .with_timeout_ms(5000)
            .with_priority(200);

        assert_eq!(cfg.name_len, 4);
        assert_eq!(cfg.timeout_ms, 5000);
        assert_eq!(cfg.priority, 200);
        assert_eq!(cfg.name_str(), Some("test"));
    }

    #[test]
    fn test_memory_slice_creation() {
        let slice = MemorySlice::from_slice(b"hello");
        assert_eq!(slice.len(), 5);
        assert_eq!(slice.as_bytes(), b"hello");
    }

    #[test]
    fn test_memory_slice_empty() {
        let slice = MemorySlice::new(alloc::vec::Vec::new());
        assert!(slice.is_empty());
    }

    #[test]
    fn test_knowledge_source_ref() {
        let ksr = KnowledgeSourceRef::from_str("embeddings://v1/default");
        assert_eq!(ksr.source_id, "embeddings://v1/default");
    }

    #[test]
    fn test_mount_point() {
        let mp = MountPoint::from_str("/knowledge/embeddings");
        assert_eq!(mp.path, "/knowledge/embeddings");
    }

    #[test]
    fn test_access_mode_display() {
        assert_eq!(AccessMode::ReadOnly.to_string(), "ReadOnly");
        assert_eq!(AccessMode::WriteOnly.to_string(), "WriteOnly");
        assert_eq!(AccessMode::ReadWrite.to_string(), "ReadWrite");
    }

    #[test]
    fn test_memory_tier_ordering() {
        assert!(MemoryTier::L1 < MemoryTier::L2);
        assert!(MemoryTier::L2 < MemoryTier::L3);
    }

    #[test]
    fn test_resource_quota_unlimited() {
        let quota = ResourceQuota::unlimited();
        assert_eq!(quota.max_memory_bytes, u64::MAX);
        assert_eq!(quota.max_compute_ms, u64::MAX);
    }

    #[test]
    fn test_capability_set_empty() {
        let cap = CapabilitySet::empty();
        assert!(!cap.has(0));
        assert!(!cap.has(1));
    }

    #[test]
    fn test_capability_set_full() {
        let cap = CapabilitySet::full();
        assert!(cap.has(0));
        assert!(cap.has(63));
        assert!(!cap.has(64));
    }

    #[test]
    fn test_capability_set_operations() {
        let mut cap = CapabilitySet::empty();
        assert!(!cap.has(CapabilitySet::CAP_TASK_FAMILY));

        cap.grant(CapabilitySet::CAP_TASK_FAMILY);
        assert!(cap.has(CapabilitySet::CAP_TASK_FAMILY));

        cap.revoke(CapabilitySet::CAP_TASK_FAMILY);
        assert!(!cap.has(CapabilitySet::CAP_TASK_FAMILY));
    }

    #[test]
    fn test_yield_hint_display() {
        assert!(YieldHint::MoreThinking.to_string().contains("MoreThinking"));
        assert!(YieldHint::WaitingForEvent
            .to_string()
            .contains("WaitingForEvent"));
    }

    #[test]
    fn test_checkpoint_type_display() {
        assert_eq!(CheckpointType::Full.to_string(), "Full");
        assert_eq!(CheckpointType::ReasoningOnly.to_string(), "ReasoningOnly");
        assert_eq!(CheckpointType::MemoryOnly.to_string(), "MemoryOnly");
    }

    #[test]
    fn test_channel_id_display() {
        let id = ChannelID(42);
        let s = id.to_string();
        assert!(s.contains("CH-"));
    }

    #[test]
    fn test_protocol_type_display() {
        assert_eq!(ProtocolType::ByteStream.to_string(), "ByteStream");
        assert_eq!(ProtocolType::MessageBased.to_string(), "MessageBased");
        assert_eq!(ProtocolType::RequestResponse.to_string(), "RequestResponse");
        assert_eq!(ProtocolType::PubSub.to_string(), "PubSub");
    }

    #[test]
    fn test_delivery_guarantee_display() {
        assert_eq!(DeliveryGuarantee::BestEffort.to_string(), "BestEffort");
        assert_eq!(
            DeliveryGuarantee::AtLeastOnce.to_string(),
            "AtLeastOnce"
        );
        assert_eq!(
            DeliveryGuarantee::ExactlyOnce.to_string(),
            "ExactlyOnce"
        );
    }

    #[test]
    fn test_message_payload_creation() {
        let payload = MessagePayload::new(42, alloc::vec![1, 2, 3]);
        assert_eq!(payload.msg_type, 42);
        assert_eq!(payload.size(), 3);
    }

    #[test]
    fn test_tool_binding_id_display() {
        let id = ToolBindingID(42);
        let s = id.to_string();
        assert!(s.contains("TOOL-"));
    }

    #[test]
    fn test_tool_spec_creation() {
        let spec = ToolSpec::new("calculator", "1.0", "A simple calculator", "mcp_tool");
        assert_eq!(spec.name, "calculator");
        assert_eq!(spec.version, "1.0");
    }

    #[test]
    fn test_sandbox_config_defaults() {
        let config = SandboxConfig::new();
        assert_eq!(config.timeout_ms, 60000);
        assert!(!config.allow_network);
    }

    #[test]
    fn test_sandbox_config_builder() {
        let config = SandboxConfig::new()
            .with_timeout(120000)
            .with_network(true)
            .with_fs_access(1);

        assert_eq!(config.timeout_ms, 120000);
        assert!(config.allow_network);
        assert_eq!(config.fs_access_level, 1);
    }

    #[test]
    fn test_tool_arguments_builder() {
        let args = ToolArguments::new()
            .add_arg("input", "hello")
            .add_arg("format", "json");

        assert_eq!(args.get("input"), Some("hello"));
        assert_eq!(args.get("format"), Some("json"));
        assert_eq!(args.get("nonexistent"), None);
    }

    #[test]
    fn test_tool_result_success() {
        let result = ToolResult::success(alloc::vec![1, 2, 3]);
        assert!(result.is_success());
        assert_eq!(result.status, 0);
    }

    #[test]
    fn test_tool_result_failure() {
        let result = ToolResult::failure(1, "Error message");
        assert!(!result.is_success());
        assert_eq!(result.status, 1);
    }

    #[test]
    fn test_capability_id_display() {
        let id = CapabilityID(42);
        let s = id.to_string();
        assert!(s.contains("CAP-"));
    }

    #[test]
    fn test_capability_spec_creation() {
        let spec = CapabilitySpec::new("read_memory", "mem-123", 2);
        assert_eq!(spec.name, "read_memory");
        assert_eq!(spec.access_level, 2);
    }

    #[test]
    fn test_cap_constraints_unlimited() {
        let constraints = CapConstraints::unlimited();
        assert_eq!(constraints.expiration_ms, 0);
        assert_eq!(constraints.max_uses, 0);
        assert!(constraints.delegatable);
        assert!(constraints.revocable);
    }

    #[test]
    fn test_attenuation_spec_creation() {
        let spec = AttenuationSpec::new(1, 60000, 10, false);
        assert_eq!(spec.new_access_level, 1);
        assert_eq!(spec.reduced_max_uses, 10);
    }

    #[test]
    fn test_crew_id_display() {
        let id = CrewID(42);
        let s = id.to_string();
        assert!(s.contains("CREW-"));
    }

    #[test]
    fn test_signal_type_display() {
        assert_eq!(SignalType::TaskComplete.to_string(), "TaskComplete");
        assert_eq!(SignalType::ChannelMessage.to_string(), "ChannelMessage");
        assert_eq!(SignalType::Timeout.to_string(), "Timeout");
    }

    #[test]
    fn test_exception_type_display() {
        assert_eq!(ExceptionType::MemoryViolation.to_string(), "MemoryViolation");
        assert_eq!(
            ExceptionType::CapabilityViolation.to_string(),
            "CapabilityViolation"
        );
    }

    #[test]
    fn test_telemetry_event_creation() {
        let event = TelemetryEvent::new(1, 1000, 0, "Test event");
        assert_eq!(event.event_type, 1);
        assert_eq!(event.severity, 0);
    }

    #[test]
    fn test_telemetry_event_with_context() {
        let event = TelemetryEvent::new(1, 1000, 0, "Test event")
            .with_context("key1", "value1")
            .with_context("key2", "value2");

        assert_eq!(event.context.len(), 2);
    }
}
