// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! # CSCI Telemetry Family Syscalls
//!
//! Telemetry family syscalls manage tracing and observability:
//! - **trace_emit**: Emit CEF (Cognitive Event Format) trace events
//! - **trace_query**: Query trace events with filtering
//!
//! # Engineering Plan Reference
//! Section 7.6: Telemetry Family Specification.

use crate::error_codes::CsciErrorCode;
use crate::syscall::{ParamType, ReturnType, SyscallDefinition, SyscallFamily, SyscallParam};
use crate::types::{AgentID, CapabilitySet, CTID};
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

/// Telemetry family syscall numbers.
pub mod number {
    /// trace_emit syscall number within Telemetry family.
    pub const TRACE_EMIT: u8 = 0;
    /// trace_query syscall number within Telemetry family.
    pub const TRACE_QUERY: u8 = 1;
}

/// Identifier for a trace event.
///
/// A globally unique identifier assigned at trace event emission.
/// Used to reference specific trace events in queries.
///
/// # Engineering Plan Reference
/// Section 7.6.1: Trace event identification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EventID(pub u64);

impl fmt::Display for EventID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EVT-{:x}", self.0)
    }
}

/// Type of trace event.
///
/// Categorizes the nature of trace events for filtering and analysis.
///
/// # Engineering Plan Reference
/// Section 7.6.2: Trace event types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum EventType {
    /// Task lifecycle event (spawn, yield, checkpoint, resume).
    TaskLifecycle = 0,
    /// Memory operation event (alloc, dealloc, read, write).
    MemoryOp = 1,
    /// Tool invocation event.
    ToolInvoke = 2,
    /// Channel communication event (open, send, receive, close).
    ChannelComm = 3,
    /// Context switch event.
    ContextSwitch = 4,
    /// Capability grant/revoke event.
    CapabilityChange = 5,
    /// Agent crew event (create, join, leave).
    CrewManagement = 6,
    /// Error or exception event.
    ErrorEvent = 7,
    /// Performance metric event.
    PerformanceMetric = 8,
    /// Custom application event.
    Custom = 9,
}

impl fmt::Display for EventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TaskLifecycle => write!(f, "TaskLifecycle"),
            Self::MemoryOp => write!(f, "MemoryOp"),
            Self::ToolInvoke => write!(f, "ToolInvoke"),
            Self::ChannelComm => write!(f, "ChannelComm"),
            Self::ContextSwitch => write!(f, "ContextSwitch"),
            Self::CapabilityChange => write!(f, "CapabilityChange"),
            Self::CrewManagement => write!(f, "CrewManagement"),
            Self::ErrorEvent => write!(f, "ErrorEvent"),
            Self::PerformanceMetric => write!(f, "PerformanceMetric"),
            Self::Custom => write!(f, "Custom"),
        }
    }
}

impl EventType {
    /// Convert a u8 to an EventType.
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::TaskLifecycle),
            1 => Some(Self::MemoryOp),
            2 => Some(Self::ToolInvoke),
            3 => Some(Self::ChannelComm),
            4 => Some(Self::ContextSwitch),
            5 => Some(Self::CapabilityChange),
            6 => Some(Self::CrewManagement),
            7 => Some(Self::ErrorEvent),
            8 => Some(Self::PerformanceMetric),
            9 => Some(Self::Custom),
            _ => None,
        }
    }
}

/// CEF (Cognitive Event Format) trace event.
///
/// Captures a single trace event with type, associated cognitive task/agent,
/// execution phase, and event-specific data.
///
/// # Engineering Plan Reference
/// Section 7.6.3: Cognitive Event Format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceEvent {
    /// Event type classification.
    pub event_type: EventType,
    /// Associated cognitive task ID (if applicable).
    pub ct_id: Option<CTID>,
    /// Associated agent ID (if applicable).
    pub agent_id: Option<AgentID>,
    /// Execution phase name (e.g., "Spawn", "Reason", "Act", "Reflect").
    pub phase: [u8; 32],
    /// Phase name length.
    pub phase_len: usize,
    /// Event-specific data (JSON or structured format).
    pub data: [u8; 512],
    /// Data length.
    pub data_len: usize,
    /// Cost attribution hint (0-100, cost distribution).
    pub cost_attribution: u8,
}

impl TraceEvent {
    /// Create a new trace event.
    pub fn new(event_type: EventType) -> Self {
        Self {
            event_type,
            ct_id: None,
            agent_id: None,
            phase: [0; 32],
            phase_len: 0,
            data: [0; 512],
            data_len: 0,
            cost_attribution: 50,
        }
    }

    /// Set the cognitive task ID.
    pub fn with_ct_id(mut self, ct_id: CTID) -> Self {
        self.ct_id = Some(ct_id);
        self
    }

    /// Set the agent ID.
    pub fn with_agent_id(mut self, agent_id: AgentID) -> Self {
        self.agent_id = Some(agent_id);
        self
    }

    /// Set the phase name.
    pub fn with_phase(mut self, phase: &[u8]) -> Self {
        let len = core::cmp::min(phase.len(), 32);
        self.phase[..len].copy_from_slice(&phase[..len]);
        self.phase_len = len;
        self
    }

    /// Set the event data.
    pub fn with_data(mut self, data: &[u8]) -> Self {
        let len = core::cmp::min(data.len(), 512);
        self.data[..len].copy_from_slice(&data[..len]);
        self.data_len = len;
        self
    }

    /// Set the cost attribution.
    pub fn with_cost_attribution(mut self, cost_attribution: u8) -> Self {
        self.cost_attribution = cost_attribution;
        self
    }

    /// Get the phase as a string slice (if valid UTF-8).
    pub fn phase_str(&self) -> Option<&str> {
        if self.phase_len == 0 {
            return Some("");
        }
        core::str::from_utf8(&self.phase[..self.phase_len]).ok()
    }

    /// Get the data as a string slice (if valid UTF-8).
    pub fn data_str(&self) -> Option<&str> {
        if self.data_len == 0 {
            return Some("");
        }
        core::str::from_utf8(&self.data[..self.data_len]).ok()
    }
}

impl Default for TraceEvent {
    fn default() -> Self {
        Self::new(EventType::Custom)
    }
}

/// Filter for querying trace events.
///
/// Specifies criteria for filtering trace events by time, type, and entity.
///
/// # Engineering Plan Reference
/// Section 7.6.4: Trace event filtering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceFilter {
    /// Time range: (start_ms, end_ms). Use 0 for open-ended.
    pub time_range: (u64, u64),
    /// Event types to include (bitmask).
    pub event_types: u16,
    /// Filter by cognitive task ID (0 = any).
    pub ct_filter: u64,
    /// Filter by agent ID (0 = any).
    pub agent_filter: u64,
}

impl TraceFilter {
    /// Create a new trace filter matching all events.
    pub fn new() -> Self {
        Self {
            time_range: (0, u64::MAX),
            event_types: 0xFFFF,
            ct_filter: 0,
            agent_filter: 0,
        }
    }

    /// Filter by event type.
    pub fn with_event_type(mut self, event_type: EventType) -> Self {
        self.event_types = 1 << (event_type as u16);
        self
    }

    /// Filter by cognitive task.
    pub fn with_ct_filter(mut self, ct_id: CTID) -> Self {
        self.ct_filter = ct_id.0;
        self
    }

    /// Filter by agent.
    pub fn with_agent_filter(mut self, agent_id: AgentID) -> Self {
        self.agent_filter = agent_id.0;
        self
    }

    /// Filter by time range.
    pub fn with_time_range(mut self, start_ms: u64, end_ms: u64) -> Self {
        self.time_range = (start_ms, end_ms);
        self
    }
}

impl Default for TraceFilter {
    fn default() -> Self {
        Self::new()
    }
}

/// Summary of a trace event (returned by trace_query).
///
/// Compressed representation of a trace event for bulk queries.
///
/// # Engineering Plan Reference
/// Section 7.6.5: Trace event summaries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceEventSummary {
    /// Event ID.
    pub event_id: EventID,
    /// Event type.
    pub event_type: EventType,
    /// Cognitive task ID.
    pub ct_id: Option<CTID>,
    /// Agent ID.
    pub agent_id: Option<AgentID>,
    /// Timestamp (milliseconds since epoch).
    pub timestamp_ms: u64,
    /// Cost attribution.
    pub cost_attribution: u8,
}

impl fmt::Display for TraceEventSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TraceEventSummary {{ id: {}, type: {}, ct: {:?}, agent: {:?}, ts: {}, cost: {}% }}",
            self.event_id, self.event_type, self.ct_id, self.agent_id, self.timestamp_ms, self.cost_attribution
        )
    }
}

/// Get the definition of the trace_emit syscall.
///
/// **trace_emit**: Emit a CEF trace event.
///
/// Records a trace event in the kernel's telemetry system. Events are
/// persisted for subsequent analysis and querying.
///
/// # Parameters
/// - `event`: (TraceEvent) The trace event to emit
///
/// # Returns
/// - Success: EventID of the emitted event
/// - Error: CS_EINVAL (invalid event), CS_EBUFFER (trace buffer full)
///
/// # Preconditions
/// - Caller must have Telemetry family capability (CAP_TELEMETRY_FAMILY)
/// - Event must have valid type and reasonable size
/// - Trace system must have buffer space available
///
/// # Postconditions
/// - Event is recorded in telemetry system
/// - Event assigned immutable EventID
/// - Event queryable via trace_query
///
/// # Engineering Plan Reference
/// Section 7.6.1: trace_emit specification.
pub fn trace_emit_definition() -> SyscallDefinition {
    SyscallDefinition::new(
        "trace_emit",
        SyscallFamily::Telemetry,
        number::TRACE_EMIT,
        ReturnType::Identifier,
        CapabilitySet::CAP_TELEMETRY_FAMILY,
        "Emit a CEF trace event to the telemetry system",
    )
    .with_param(SyscallParam::new(
        "event",
        ParamType::Config,
        "CEF trace event (type, ct_id, agent_id, phase, data, cost_attribution)",
        false,
    ))
    .with_error(CsciErrorCode::CsSuccess)
    .with_error(CsciErrorCode::CsEinval)
    .with_error(CsciErrorCode::CsEbuffer)
    .with_preconditions("Caller has Telemetry capability; event is valid; buffer space available")
    .with_postconditions("Event recorded in telemetry; EventID assigned; queryable")
}

/// Get the definition of the trace_query syscall.
///
/// **trace_query**: Query trace events by filter criteria.
///
/// Retrieves trace events matching specified filters (time, type, task, agent).
/// Returns summary information suitable for bulk processing.
///
/// # Parameters
/// - `filter`: (TraceFilter) Filter criteria for event selection
///
/// # Returns
/// - Success: Vec<TraceEventSummary> containing matching events
/// - Error: CS_EINVAL (invalid filter)
///
/// # Preconditions
/// - Caller must have Telemetry family capability (CAP_TELEMETRY_FAMILY)
/// - Filter must have valid parameters
///
/// # Postconditions
/// - Matching events returned in chronological order
/// - No events are modified
/// - Results may be paginated for large result sets
///
/// # Engineering Plan Reference
/// Section 7.6.2: trace_query specification.
pub fn trace_query_definition() -> SyscallDefinition {
    SyscallDefinition::new(
        "trace_query",
        SyscallFamily::Telemetry,
        number::TRACE_QUERY,
        ReturnType::Memory,
        CapabilitySet::CAP_TELEMETRY_FAMILY,
        "Query trace events by filter criteria",
    )
    .with_param(SyscallParam::new(
        "filter",
        ParamType::Config,
        "Filter criteria (time_range, event_types, ct_filter, agent_filter)",
        false,
    ))
    .with_error(CsciErrorCode::CsSuccess)
    .with_error(CsciErrorCode::CsEinval)
    .with_preconditions("Caller has Telemetry capability; filter is valid")
    .with_postconditions("Matching events returned in chronological order; no modifications")
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::string::ToString;

    #[test]
    fn test_event_id_display() {
        let event_id = EventID(0xdeadbeef);
        assert_eq!(event_id.to_string(), "EVT-deadbeef");
    }

    #[test]
    fn test_event_type_display() {
        assert_eq!(EventType::TaskLifecycle.to_string(), "TaskLifecycle");
        assert_eq!(EventType::MemoryOp.to_string(), "MemoryOp");
        assert_eq!(EventType::Custom.to_string(), "Custom");
    }

    #[test]
    fn test_event_type_from_u8() {
        assert_eq!(EventType::from_u8(0), Some(EventType::TaskLifecycle));
        assert_eq!(EventType::from_u8(1), Some(EventType::MemoryOp));
        assert_eq!(EventType::from_u8(9), Some(EventType::Custom));
        assert_eq!(EventType::from_u8(99), None);
    }

    #[test]
    fn test_trace_event_creation() {
        let event = TraceEvent::new(EventType::TaskLifecycle);
        assert_eq!(event.event_type, EventType::TaskLifecycle);
        assert_eq!(event.ct_id, None);
        assert_eq!(event.agent_id, None);
        assert_eq!(event.cost_attribution, 50);
    }

    #[test]
    fn test_trace_event_with_ct_id() {
        let ct_id = CTID(42);
        let event = TraceEvent::new(EventType::MemoryOp).with_ct_id(ct_id);
        assert_eq!(event.ct_id, Some(ct_id));
    }

    #[test]
    fn test_trace_event_with_agent_id() {
        let agent_id = AgentID(100);
        let event = TraceEvent::new(EventType::Custom).with_agent_id(agent_id);
        assert_eq!(event.agent_id, Some(agent_id));
    }

    #[test]
    fn test_trace_event_with_phase() {
        let phase = b"Reason";
        let event = TraceEvent::new(EventType::TaskLifecycle).with_phase(phase);
        assert_eq!(event.phase_len, 6);
        assert_eq!(event.phase_str(), Some("Reason"));
    }

    #[test]
    fn test_trace_event_with_data() {
        let data = b"test data";
        let event = TraceEvent::new(EventType::Custom).with_data(data);
        assert_eq!(event.data_len, 9);
        assert_eq!(event.data_str(), Some("test data"));
    }

    #[test]
    fn test_trace_event_builder_chain() {
        let ct_id = CTID(42);
        let agent_id = AgentID(100);
        let event = TraceEvent::new(EventType::MemoryOp)
            .with_ct_id(ct_id)
            .with_agent_id(agent_id)
            .with_phase(b"Act")
            .with_data(b"alloc")
            .with_cost_attribution(75);

        assert_eq!(event.ct_id, Some(ct_id));
        assert_eq!(event.agent_id, Some(agent_id));
        assert_eq!(event.phase_str(), Some("Act"));
        assert_eq!(event.data_str(), Some("alloc"));
        assert_eq!(event.cost_attribution, 75);
    }

    #[test]
    fn test_trace_filter_creation() {
        let filter = TraceFilter::new();
        assert_eq!(filter.time_range, (0, u64::MAX));
        assert_eq!(filter.event_types, 0xFFFF);
    }

    #[test]
    fn test_trace_filter_with_event_type() {
        let filter = TraceFilter::new().with_event_type(EventType::MemoryOp);
        assert_eq!(filter.event_types, 1 << 1);
    }

    #[test]
    fn test_trace_filter_with_ct_filter() {
        let ct_id = CTID(42);
        let filter = TraceFilter::new().with_ct_filter(ct_id);
        assert_eq!(filter.ct_filter, 42);
    }

    #[test]
    fn test_trace_filter_with_agent_filter() {
        let agent_id = AgentID(100);
        let filter = TraceFilter::new().with_agent_filter(agent_id);
        assert_eq!(filter.agent_filter, 100);
    }

    #[test]
    fn test_trace_filter_with_time_range() {
        let filter = TraceFilter::new().with_time_range(1000, 2000);
        assert_eq!(filter.time_range, (1000, 2000));
    }

    #[test]
    fn test_trace_event_summary_display() {
        let summary = TraceEventSummary {
            event_id: EventID(1),
            event_type: EventType::TaskLifecycle,
            ct_id: Some(CTID(42)),
            agent_id: Some(AgentID(100)),
            timestamp_ms: 12345,
            cost_attribution: 75,
        };
        let display_str = summary.to_string();
        assert!(display_str.contains("EVT-1"));
        assert!(display_str.contains("TaskLifecycle"));
        assert!(display_str.contains("75%"));
    }

    #[test]
    fn test_trace_emit_definition() {
        let def = trace_emit_definition();
        assert_eq!(def.name, "trace_emit");
        assert_eq!(def.family, SyscallFamily::Telemetry);
        assert_eq!(def.number, number::TRACE_EMIT);
        assert!(!def.description.is_empty());
    }

    #[test]
    fn test_trace_query_definition() {
        let def = trace_query_definition();
        assert_eq!(def.name, "trace_query");
        assert_eq!(def.family, SyscallFamily::Telemetry);
        assert_eq!(def.number, number::TRACE_QUERY);
        assert_eq!(def.return_type, ReturnType::Memory);
    }
}
