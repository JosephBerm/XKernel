// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Strongly typed identifiers for tool registry and telemetry.
//!
//! All identifiers in the Tool Registry system are typed to prevent accidental confusion
//! and enable compile-time verification.
//! See Engineering Plan § 2.11: ToolBinding Entity & Tool Registry.

use alloc::fmt;
use alloc::string::{String, ToString};
use core::fmt::{Debug, Display};
use core::hash::{Hash, Hasher};

/// A strongly-typed tool binding identifier.
///
/// See Engineering Plan § 2.11: ToolBinding Entity.
/// ToolBindingIDs uniquely identify a binding between a tool definition and an agent context.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ToolBindingID(String);

impl ToolBindingID {
    /// Creates a new tool binding ID from a string.
    pub fn new(id: impl Into<String>) -> Self {
        ToolBindingID(id.into())
    }

    /// Returns a reference to the underlying string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for ToolBindingID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ToolBinding({})", self.0)
    }
}

/// A strongly-typed tool identifier.
///
/// See Engineering Plan § 2.11: Tool Definition & Registry.
/// ToolIDs identify tool definitions (like a function or API endpoint schema).
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ToolID(String);

impl ToolID {
    /// Creates a new tool ID from a string.
    pub fn new(id: impl Into<String>) -> Self {
        ToolID(id.into())
    }

    /// Returns a reference to the underlying string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for ToolID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Tool({})", self.0)
    }
}

/// A strongly-typed event identifier for telemetry events.
///
/// See Engineering Plan § 2.12: Cognitive Event Format (CEF) & Telemetry.
/// EventIDs are globally unique ULID-based identifiers for telemetry events.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct EventID(String);

impl EventID {
    /// Creates a new event ID from a string (typically a ULID).
    pub fn new(id: impl Into<String>) -> Self {
        EventID(id.into())
    }

    /// Returns a reference to the underlying string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for EventID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Event({})", self.0)
    }
}

/// A strongly-typed policy identifier for tool policies.
///
/// See Engineering Plan § 2.11: ToolBinding Entity & Sandbox Configuration.
/// PolicyIDs identify access control, sandbox, or effect policies.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct PolicyID(String);

impl PolicyID {
    /// Creates a new policy ID from a string.
    pub fn new(id: impl Into<String>) -> Self {
        PolicyID(id.into())
    }

    /// Returns a reference to the underlying string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for PolicyID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Policy({})", self.0)
    }
}

/// A strongly-typed agent identifier.
///
/// See Engineering Plan § 2.12: CEF Event Structure.
/// AgentIDs identify the principal (agent) that initiated or triggered an event.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct AgentID(String);

impl AgentID {
    /// Creates a new agent ID from a string.
    pub fn new(id: impl Into<String>) -> Self {
        AgentID(id.into())
    }

    /// Returns a reference to the underlying string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for AgentID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Agent({})", self.0)
    }
}

/// A strongly-typed crew identifier.
///
/// See Engineering Plan § 2.12: CEF Event Structure.
/// CrewIDs identify collaborative crew contexts for multi-agent operations.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct CrewID(String);

impl CrewID {
    /// Creates a new crew ID from a string.
    pub fn new(id: impl Into<String>) -> Self {
        CrewID(id.into())
    }

    /// Returns a reference to the underlying string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for CrewID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Crew({})", self.0)
    }
}

/// A strongly-typed ULID identifier.
///
/// See Engineering Plan § 2.12: Event Identification.
/// ULIDs (Universally Unique Lexicographically Sortable Identifiers) are used
/// for event IDs, checkpoint IDs, and other temporal identifiers.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Ulid(String);

impl Ulid {
    /// Creates a new ULID from a string.
    pub fn new(id: impl Into<String>) -> Self {
        Ulid(id.into())
    }

    /// Returns a reference to the underlying string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for Ulid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// OpenTelemetry-compatible trace ID (128-bit, W3C Trace Context format).
///
/// See Engineering Plan § 2.12: Correlation and Tracing.
/// Trace IDs link events across distributed systems.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct TraceID(String);

impl TraceID {
    /// Creates a new trace ID from a hex string (16 bytes = 32 hex chars).
    pub fn new(id: impl Into<String>) -> Self {
        TraceID(id.into())
    }

    /// Returns a reference to the underlying string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for TraceID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// OpenTelemetry-compatible span ID (64-bit, hex-encoded).
///
/// See Engineering Plan § 2.12: Correlation and Tracing.
/// Span IDs identify specific operations within a trace.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct SpanID(String);

impl SpanID {
    /// Creates a new span ID from a hex string (8 bytes = 16 hex chars).
    pub fn new(id: impl Into<String>) -> Self {
        SpanID(id.into())
    }

    /// Returns a reference to the underlying string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for SpanID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Cognitive Thread ID (CT ID).
///
/// See Engineering Plan § 2.12: CEF Event Structure.
/// CT IDs identify the cognitive thread that generated an event.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct CognitiveThreadID(String);

impl CognitiveThreadID {
    /// Creates a new cognitive thread ID from a string.
    pub fn new(id: impl Into<String>) -> Self {
        CognitiveThreadID(id.into())
    }

    /// Returns a reference to the underlying string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for CognitiveThreadID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CT({})", self.0)
    }
}

/// A strongly-typed capability identifier.
///
/// See Engineering Plan § 3.1: Capability-Based Security.
/// CapIDs identify capability tokens that authorize operations.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct CapID([u8; 32]);

impl CapID {
    /// Creates a new capability ID from raw bytes.
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        CapID(bytes)
    }

    /// Returns the raw bytes of this capability ID.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl Display for CapID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Cap({:02x}{:02x}...)", self.0[0], self.0[1])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_binding_id_creation() {
        let id = ToolBindingID::new("binding-001");
        assert_eq!(id.as_str(), "binding-001");
    }

    #[test]
    fn test_tool_binding_id_display() {
        let id = ToolBindingID::new("binding-001");
        assert_eq!(id.to_string(), "ToolBinding(binding-001)");
    }

    #[test]
    fn test_tool_id_creation() {
        let id = ToolID::new("github-api");
        assert_eq!(id.as_str(), "github-api");
    }

    #[test]
    fn test_tool_id_display() {
        let id = ToolID::new("github-api");
        assert_eq!(id.to_string(), "Tool(github-api)");
    }

    #[test]
    fn test_event_id_creation() {
        let id = EventID::new("01ARZ3NDEKTSV4RRFFQ69G5FAV");
        assert_eq!(id.as_str(), "01ARZ3NDEKTSV4RRFFQ69G5FAV");
    }

    #[test]
    fn test_event_id_display() {
        let id = EventID::new("01ARZ3NDEKTSV4RRFFQ69G5FAV");
        assert_eq!(id.to_string(), "Event(01ARZ3NDEKTSV4RRFFQ69G5FAV)");
    }

    #[test]
    fn test_policy_id_creation() {
        let id = PolicyID::new("sandbox-strict");
        assert_eq!(id.as_str(), "sandbox-strict");
    }

    #[test]
    fn test_policy_id_display() {
        let id = PolicyID::new("sandbox-strict");
        assert_eq!(id.to_string(), "Policy(sandbox-strict)");
    }

    #[test]
    fn test_ids_are_distinct_types() {
        let tool_id = ToolID::new("tool-a");
        let binding_id = ToolBindingID::new("tool-a");
        // These have different types, so they can't be confused at compile time
        assert_eq!(tool_id.as_str(), binding_id.as_str());
    }

    #[test]
    fn test_id_hash() {
        use std::collections::hash_map::DefaultHasher;

        let id1 = ToolID::new("same");
        let id2 = ToolID::new("same");
        let id3 = ToolID::new("different");

        let mut h1 = DefaultHasher::new();
        id1.hash(&mut h1);
        let hash1 = h1.finish();

        let mut h2 = DefaultHasher::new();
        id2.hash(&mut h2);
        let hash2 = h2.finish();

        let mut h3 = DefaultHasher::new();
        id3.hash(&mut h3);
        let hash3 = h3.finish();

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_agent_id_creation() {
        let id = AgentID::new("agent-001");
        assert_eq!(id.as_str(), "agent-001");
    }

    #[test]
    fn test_agent_id_display() {
        let id = AgentID::new("agent-001");
        assert_eq!(id.to_string(), "Agent(agent-001)");
    }

    #[test]
    fn test_crew_id_creation() {
        let id = CrewID::new("crew-001");
        assert_eq!(id.as_str(), "crew-001");
    }

    #[test]
    fn test_crew_id_display() {
        let id = CrewID::new("crew-001");
        assert_eq!(id.to_string(), "Crew(crew-001)");
    }

    #[test]
    fn test_ulid_creation() {
        let id = Ulid::new("01ARZ3NDEKTSV4RRFFQ69G5FAV");
        assert_eq!(id.as_str(), "01ARZ3NDEKTSV4RRFFQ69G5FAV");
    }

    #[test]
    fn test_ulid_display() {
        let id = Ulid::new("01ARZ3NDEKTSV4RRFFQ69G5FAV");
        assert_eq!(id.to_string(), "01ARZ3NDEKTSV4RRFFQ69G5FAV");
    }

    #[test]
    fn test_trace_id_creation() {
        let id = TraceID::new("0af7651916cd43dd8448eb211c80319c");
        assert_eq!(id.as_str(), "0af7651916cd43dd8448eb211c80319c");
    }

    #[test]
    fn test_trace_id_display() {
        let id = TraceID::new("0af7651916cd43dd8448eb211c80319c");
        assert_eq!(id.to_string(), "0af7651916cd43dd8448eb211c80319c");
    }

    #[test]
    fn test_span_id_creation() {
        let id = SpanID::new("b9c7c989f97918e1");
        assert_eq!(id.as_str(), "b9c7c989f97918e1");
    }

    #[test]
    fn test_span_id_display() {
        let id = SpanID::new("b9c7c989f97918e1");
        assert_eq!(id.to_string(), "b9c7c989f97918e1");
    }

    #[test]
    fn test_cognitive_thread_id_creation() {
        let id = CognitiveThreadID::new("ct-001");
        assert_eq!(id.as_str(), "ct-001");
    }

    #[test]
    fn test_cognitive_thread_id_display() {
        let id = CognitiveThreadID::new("ct-001");
        assert_eq!(id.to_string(), "CT(ct-001)");
    }

    #[test]
    fn test_agent_id_equality() {
        let id1 = AgentID::new("agent-001");
        let id2 = AgentID::new("agent-001");
        let id3 = AgentID::new("agent-002");
        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_trace_id_equality() {
        let id1 = TraceID::new("0af7651916cd43dd8448eb211c80319c");
        let id2 = TraceID::new("0af7651916cd43dd8448eb211c80319c");
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_span_id_hash() {
        use std::collections::hash_map::DefaultHasher;
use ulid::Ulid;
use alloc::string::String;
use alloc::string::ToString;

        let id1 = SpanID::new("b9c7c989f97918e1");
        let id2 = SpanID::new("b9c7c989f97918e1");

        let mut h1 = DefaultHasher::new();
        id1.hash(&mut h1);
        let hash1 = h1.finish();

        let mut h2 = DefaultHasher::new();
        id2.hash(&mut h2);
        let hash2 = h2.finish();

        assert_eq!(hash1, hash2);
    }
}
