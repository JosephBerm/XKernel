// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Event correlation and distributed tracing.
//!
//! Provides trace context management, span correlation, and causal chain tracking
//! for distributed tracing aligned with OpenTelemetry standards.
//!
//! See Engineering Plan § 2.12: Correlation and Tracing
//! and Addendum v2.5.1: OpenTelemetry W3C Trace Context Format.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use crate::ids::{SpanID, TraceID};

/// Trace flags for OpenTelemetry W3C Trace Context.
///
/// See Engineering Plan § 2.12: Trace Context.
/// Flags indicate tracing decisions and sampling state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TraceFlags {
    /// Trace is sampled (should be collected and exported).
    pub sampled: bool,
}

impl TraceFlags {
    /// Creates new trace flags.
    pub fn new(sampled: bool) -> Self {
        TraceFlags { sampled }
    }

    /// Returns the byte representation for W3C Trace Context.
    pub fn as_byte(&self) -> u8 {
        if self.sampled { 1 } else { 0 }
    }
}

impl fmt::Display for TraceFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02x}", self.as_byte())
    }
}

/// OpenTelemetry-compatible trace context.
///
/// See Engineering Plan § 2.12: Trace Context.
/// Encodes distributed trace identity and parent-child relationships.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceContext {
    /// 128-bit trace ID (W3C Trace Context format).
    pub trace_id: TraceID,

    /// 64-bit span ID for this operation.
    pub span_id: SpanID,

    /// Parent span ID (if any).
    pub parent_span_id: Option<SpanID>,

    /// Trace flags (sampled, etc).
    pub trace_flags: TraceFlags,
}

impl TraceContext {
    /// Creates a new trace context.
    pub fn new(trace_id: TraceID, span_id: SpanID, trace_flags: TraceFlags) -> Self {
        TraceContext {
            trace_id,
            span_id,
            parent_span_id: None,
            trace_flags,
        }
    }

    /// Creates a new trace context with a parent span ID.
    pub fn with_parent(
        trace_id: TraceID,
        span_id: SpanID,
        parent_span_id: SpanID,
        trace_flags: TraceFlags,
    ) -> Self {
        TraceContext {
            trace_id,
            span_id,
            parent_span_id: Some(parent_span_id),
            trace_flags,
        }
    }

    /// Sets the parent span ID.
    pub fn set_parent(mut self, parent_span_id: SpanID) -> Self {
        self.parent_span_id = Some(parent_span_id);
        self
    }

    /// W3C Trace Context header format.
    /// Format: traceparent: version-traceid-spanid-traceflags
    /// Example: traceparent: 00-0af7651916cd43dd8448eb211c80319c-b9c7c989f97918e1-01
    pub fn to_w3c_traceparent(&self) -> String {
        alloc::format!(
            "00-{}-{}-{}",
            self.trace_id.as_str(),
            self.span_id.as_str(),
            self.trace_flags
        )
    }

    /// Parses W3C traceparent header format.
    pub fn from_w3c_traceparent(header: &str) -> Option<Self> {
        let parts: Vec<&str> = header.split('-').collect();
        if parts.len() != 4 {
            return None;
        }

        let version = parts[0];
        if version != "00" {
            return None; // Unsupported version
        }

        let trace_id = TraceID::new(parts[1]);
        let span_id = SpanID::new(parts[2]);

        let sampled = match parts[3] {
            "01" => true,
            "00" => false,
            _ => return None,
        };

        let trace_flags = TraceFlags::new(sampled);

        Some(TraceContext::new(trace_id, span_id, trace_flags))
    }
}

impl fmt::Display for TraceContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Trace({})", self.to_w3c_traceparent())
    }
}

/// Causal chain of events for order-preserving audit trails.
///
/// See Engineering Plan § 2.12: Causality Chain.
/// Maintains a linked list of event IDs representing causal relationships.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CausalityChain {
    /// Ordered list of event IDs forming the causal chain.
    events: Vec<String>,
}

impl CausalityChain {
    /// Creates a new causality chain starting with an initial event.
    pub fn new(initial_event: impl Into<String>) -> Self {
        let mut events = Vec::new();
        events.push(initial_event.into());
        CausalityChain { events }
    }

    /// Creates an empty causality chain.
    pub fn empty() -> Self {
        CausalityChain {
            events: Vec::new(),
        }
    }

    /// Appends an event to the causal chain.
    pub fn append(&mut self, event_id: impl Into<String>) {
        self.events.push(event_id.into());
    }

    /// Returns the current length of the causal chain.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Returns true if the chain is empty.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Returns the root event ID (first in chain).
    pub fn root(&self) -> Option<&str> {
        self.events.first().map(|s| s.as_str())
    }

    /// Returns the latest event ID (last in chain).
    pub fn latest(&self) -> Option<&str> {
        self.events.last().map(|s| s.as_str())
    }

    /// Returns all events in the chain.
    pub fn events(&self) -> &[String] {
        &self.events
    }

    /// Returns a reference to events as a mutable vector (for batch operations).
    pub fn events_mut(&mut self) -> &mut Vec<String> {
        &mut self.events
    }

    /// Creates a copy of the chain with all events.
    pub fn clone_chain(&self) -> Vec<String> {
        self.events.clone()
    }
}

impl fmt::Display for CausalityChain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CausalityChain({})", self.events.len())
    }
}

/// Correlation engine for managing distributed traces and spans.
///
/// See Engineering Plan § 2.12: Correlation and Tracing.
pub trait CorrelationEngine {
    /// Creates a new trace with a fresh trace ID.
    fn new_trace(&mut self) -> TraceContext;

    /// Creates a new span within an existing trace.
    fn new_span(&mut self, trace: &TraceContext) -> TraceContext;

    /// Links two events in a causal relationship.
    fn link_events(&mut self, from: &str, to: &str);
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;
use alloc::string::ToString;

    #[test]
    fn test_trace_flags_creation() {
        let flags = TraceFlags::new(true);
        assert!(flags.sampled);
        assert_eq!(flags.as_byte(), 1);
    }

    #[test]
    fn test_trace_flags_not_sampled() {
        let flags = TraceFlags::new(false);
        assert!(!flags.sampled);
        assert_eq!(flags.as_byte(), 0);
    }

    #[test]
    fn test_trace_flags_display() {
        let flags_sampled = TraceFlags::new(true);
        assert_eq!(flags_sampled.to_string(), "01");

        let flags_not = TraceFlags::new(false);
        assert_eq!(flags_not.to_string(), "00");
    }

    #[test]
    fn test_trace_flags_equality() {
        let f1 = TraceFlags::new(true);
        let f2 = TraceFlags::new(true);
        assert_eq!(f1, f2);

        let f3 = TraceFlags::new(false);
        assert_ne!(f1, f3);
    }

    #[test]
    fn test_trace_context_creation() {
        let trace_id = TraceID::new("0af7651916cd43dd8448eb211c80319c");
        let span_id = SpanID::new("b9c7c989f97918e1");
        let flags = TraceFlags::new(true);

        let ctx = TraceContext::new(trace_id.clone(), span_id.clone(), flags);
        assert_eq!(ctx.trace_id, trace_id);
        assert_eq!(ctx.span_id, span_id);
        assert_eq!(ctx.parent_span_id, None);
        assert!(ctx.trace_flags.sampled);
    }

    #[test]
    fn test_trace_context_with_parent() {
        let trace_id = TraceID::new("0af7651916cd43dd8448eb211c80319c");
        let span_id = SpanID::new("b9c7c989f97918e1");
        let parent_span_id = SpanID::new("a1b2c3d4e5f6g7h8");
        let flags = TraceFlags::new(true);

        let ctx = TraceContext::with_parent(trace_id.clone(), span_id.clone(), parent_span_id.clone(), flags);
        assert_eq!(ctx.parent_span_id, Some(parent_span_id));
    }

    #[test]
    fn test_trace_context_set_parent() {
        let trace_id = TraceID::new("0af7651916cd43dd8448eb211c80319c");
        let span_id = SpanID::new("b9c7c989f97918e1");
        let parent_span_id = SpanID::new("a1b2c3d4e5f6g7h8");
        let flags = TraceFlags::new(false);

        let ctx = TraceContext::new(trace_id, span_id, flags).set_parent(parent_span_id.clone());
        assert_eq!(ctx.parent_span_id, Some(parent_span_id));
    }

    #[test]
    fn test_trace_context_w3c_traceparent() {
        let trace_id = TraceID::new("0af7651916cd43dd8448eb211c80319c");
        let span_id = SpanID::new("b9c7c989f97918e1");
        let flags = TraceFlags::new(true);

        let ctx = TraceContext::new(trace_id, span_id, flags);
        let header = ctx.to_w3c_traceparent();
        assert_eq!(header, "00-0af7651916cd43dd8448eb211c80319c-b9c7c989f97918e1-01");
    }

    #[test]
    fn test_trace_context_w3c_traceparent_not_sampled() {
        let trace_id = TraceID::new("0af7651916cd43dd8448eb211c80319c");
        let span_id = SpanID::new("b9c7c989f97918e1");
        let flags = TraceFlags::new(false);

        let ctx = TraceContext::new(trace_id, span_id, flags);
        let header = ctx.to_w3c_traceparent();
        assert_eq!(header, "00-0af7651916cd43dd8448eb211c80319c-b9c7c989f97918e1-00");
    }

    #[test]
    fn test_trace_context_from_w3c_traceparent() {
        let header = "00-0af7651916cd43dd8448eb211c80319c-b9c7c989f97918e1-01";
        let ctx = TraceContext::from_w3c_traceparent(header).unwrap();

        assert_eq!(ctx.trace_id.as_str(), "0af7651916cd43dd8448eb211c80319c");
        assert_eq!(ctx.span_id.as_str(), "b9c7c989f97918e1");
        assert!(ctx.trace_flags.sampled);
    }

    #[test]
    fn test_trace_context_from_w3c_traceparent_not_sampled() {
        let header = "00-0af7651916cd43dd8448eb211c80319c-b9c7c989f97918e1-00";
        let ctx = TraceContext::from_w3c_traceparent(header).unwrap();
        assert!(!ctx.trace_flags.sampled);
    }

    #[test]
    fn test_trace_context_from_w3c_traceparent_invalid_version() {
        let header = "01-0af7651916cd43dd8448eb211c80319c-b9c7c989f97918e1-01";
        let ctx = TraceContext::from_w3c_traceparent(header);
        assert_eq!(ctx, None);
    }

    #[test]
    fn test_trace_context_from_w3c_traceparent_invalid_format() {
        let header = "00-invalid-header-format";
        let ctx = TraceContext::from_w3c_traceparent(header);
        assert_eq!(ctx, None);
    }

    #[test]
    fn test_trace_context_w3c_roundtrip() {
        let trace_id = TraceID::new("0af7651916cd43dd8448eb211c80319c");
        let span_id = SpanID::new("b9c7c989f97918e1");
        let flags = TraceFlags::new(true);

        let ctx1 = TraceContext::new(trace_id, span_id, flags);
        let header = ctx1.to_w3c_traceparent();
        let ctx2 = TraceContext::from_w3c_traceparent(&header).unwrap();

        assert_eq!(ctx1, ctx2);
    }

    #[test]
    fn test_trace_context_display() {
        let trace_id = TraceID::new("0af7651916cd43dd8448eb211c80319c");
        let span_id = SpanID::new("b9c7c989f97918e1");
        let flags = TraceFlags::new(true);

        let ctx = TraceContext::new(trace_id, span_id, flags);
        let display = ctx.to_string();
        assert!(display.contains("Trace"));
    }

    #[test]
    fn test_trace_context_equality() {
        let trace_id = TraceID::new("0af7651916cd43dd8448eb211c80319c");
        let span_id = SpanID::new("b9c7c989f97918e1");
        let flags = TraceFlags::new(true);

        let ctx1 = TraceContext::new(trace_id.clone(), span_id.clone(), flags);
        let ctx2 = TraceContext::new(trace_id, span_id, flags);
        assert_eq!(ctx1, ctx2);
    }

    #[test]
    fn test_causality_chain_new() {
        let chain = CausalityChain::new("event-001");
        assert_eq!(chain.len(), 1);
        assert_eq!(chain.root(), Some("event-001"));
        assert_eq!(chain.latest(), Some("event-001"));
    }

    #[test]
    fn test_causality_chain_empty() {
        let chain: CausalityChain = CausalityChain::empty();
        assert_eq!(chain.len(), 0);
        assert!(chain.is_empty());
        assert_eq!(chain.root(), None);
        assert_eq!(chain.latest(), None);
    }

    #[test]
    fn test_causality_chain_append() {
        let mut chain = CausalityChain::new("event-001");
        chain.append("event-002");
        chain.append("event-003");

        assert_eq!(chain.len(), 3);
        assert_eq!(chain.root(), Some("event-001"));
        assert_eq!(chain.latest(), Some("event-003"));
    }

    #[test]
    fn test_causality_chain_events() {
        let mut chain = CausalityChain::new("event-001");
        chain.append("event-002");

        let events = chain.events();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0], "event-001");
        assert_eq!(events[1], "event-002");
    }

    #[test]
    fn test_causality_chain_clone_chain() {
        let mut chain = CausalityChain::new("event-001");
        chain.append("event-002");
        chain.append("event-003");

        let cloned = chain.clone_chain();
        assert_eq!(cloned.len(), 3);
        assert_eq!(cloned[0], "event-001");
        assert_eq!(cloned[2], "event-003");
    }

    #[test]
    fn test_causality_chain_display() {
        let mut chain = CausalityChain::new("event-001");
        chain.append("event-002");

        let display = chain.to_string();
        assert!(display.contains("CausalityChain"));
        assert!(display.contains("2"));
    }

    #[test]
    fn test_causality_chain_equality() {
        let mut chain1 = CausalityChain::new("event-001");
        chain1.append("event-002");

        let mut chain2 = CausalityChain::new("event-001");
        chain2.append("event-002");

        assert_eq!(chain1, chain2);
    }

    #[test]
    fn test_causality_chain_clone() {
        let mut chain = CausalityChain::new("event-001");
        chain.append("event-002");
        chain.append("event-003");

        let cloned = chain.clone();
        assert_eq!(chain, cloned);
    }

    #[test]
    fn test_causality_chain_long_chain() {
        let mut chain = CausalityChain::new("event-001");
        for i in 2..=100 {
            chain.append(alloc::format!("event-{:03}", i));
        }

        assert_eq!(chain.len(), 100);
        assert_eq!(chain.root(), Some("event-001"));
        assert_eq!(chain.latest(), Some("event-100"));
    }

    #[test]
    fn test_causality_chain_events_mut() {
        let mut chain = CausalityChain::new("event-001");
        chain.append("event-002");

        {
            let events = chain.events_mut();
            assert_eq!(events.len(), 2);
            events.push("event-003".to_string());
        }

        assert_eq!(chain.len(), 3);
        assert_eq!(chain.latest(), Some("event-003"));
    }

    #[test]
    fn test_trace_context_clone() {
        let trace_id = TraceID::new("0af7651916cd43dd8448eb211c80319c");
        let span_id = SpanID::new("b9c7c989f97918e1");
        let flags = TraceFlags::new(true);

        let ctx = TraceContext::new(trace_id, span_id, flags);
        let cloned = ctx.clone();
        assert_eq!(ctx, cloned);
    }
}
