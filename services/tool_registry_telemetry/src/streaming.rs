// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Real-time Event Streaming Infrastructure
//!
//! Provides subscription-based event streaming with ring buffers, filtering,
//! backpressure, and delivery guarantees for real-time telemetry consumption.
//!
//! See Engineering Plan § 2.12.6: Event Streaming & Real-Time Telemetry.

use alloc::string::String;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use core::fmt;

use crate::cef::CefEvent;
use crate::error::{Result, ToolError};

/// Subscription identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubscriptionId(u64);

impl SubscriptionId {
    /// Creates a new subscription ID.
    pub fn new(id: u64) -> Self {
        SubscriptionId(id)
    }

    /// Returns the inner ID value.
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for SubscriptionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "sub-{}", self.0)
    }
}

/// Overflow policy for event buffer when capacity exceeded.
///
/// See Engineering Plan § 2.12.6: Event Buffering & Backpressure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverflowPolicy {
    /// Drop oldest events when buffer full
    Drop,
    /// Apply backpressure to publisher
    BackpressureSender,
    /// Persist overflow to persistent storage
    PersistOverflow,
}

impl Default for OverflowPolicy {
    fn default() -> Self {
        OverflowPolicy::Drop
    }
}

/// Event filtering criteria for subscriptions.
///
/// See Engineering Plan § 2.12.6: Event Filtering.
#[derive(Clone, Debug)]
pub struct EventFilter {
    /// Event types to include (empty = all types)
    pub event_types: alloc::collections::BTreeSet<String>,
    /// Filter by actor (agent/crew) if specified
    pub actor_filter: Option<String>,
    /// Filter by resource pattern (glob-style) if specified
    pub resource_pattern: Option<String>,
    /// Filter by trace ID prefix if specified
    pub trace_id_prefix: Option<String>,
    /// Include cost attribution data
    pub include_cost: bool,
}

impl EventFilter {
    /// Creates a new event filter that accepts all events.
    pub fn accept_all() -> Self {
        EventFilter {
            event_types: alloc::collections::BTreeSet::new(),
            actor_filter: None,
            resource_pattern: None,
            trace_id_prefix: None,
            include_cost: false,
        }
    }

    /// Creates a filter for specific event types.
    pub fn for_event_types(types: Vec<&'static str>) -> Self {
        let mut event_types = alloc::collections::BTreeSet::new();
        for t in types {
            event_types.insert(t.to_string());
        }
        EventFilter {
            event_types,
            actor_filter: None,
            resource_pattern: None,
            trace_id_prefix: None,
            include_cost: false,
        }
    }

    /// Checks if an event matches this filter.
    pub fn matches(&self, event: &CefEvent) -> bool {
        // Check event type
        if !self.event_types.is_empty() {
            let event_type_str = match event.event_type {
                crate::cef::CefEventType::ThoughtStep => "ThoughtStep",
                crate::cef::CefEventType::ToolCallRequested => "ToolCallRequested",
                crate::cef::CefEventType::ToolCallCompleted => "ToolCallCompleted",
                crate::cef::CefEventType::PolicyDecision => "PolicyDecision",
                crate::cef::CefEventType::MemoryAccess => "MemoryAccess",
                crate::cef::CefEventType::IpcMessage => "IpcMessage",
                crate::cef::CefEventType::PhaseTransition => "PhaseTransition",
                crate::cef::CefEventType::CheckpointCreated => "CheckpointCreated",
                crate::cef::CefEventType::SignalDispatched => "SignalDispatched",
                crate::cef::CefEventType::ExceptionRaised => "ExceptionRaised",
            };
            if !self.event_types.contains(event_type_str) {
                return false;
            }
        }

        // Check actor filter
        if let Some(ref actor) = self.actor_filter {
            if !event.agent_id.contains(actor) {
                return false;
            }
        }

        // Check trace ID prefix
        if let Some(ref prefix) = self.trace_id_prefix {
            if !event.trace_id.starts_with(prefix) {
                return false;
            }
        }

        true
    }
}

impl Default for EventFilter {
    fn default() -> Self {
        Self::accept_all()
    }
}

/// Delivery guarantee level for subscriptions.
///
/// See Engineering Plan § 2.12.6: Delivery Guarantees.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeliveryMode {
    /// Best-effort delivery, events may be lost
    BestEffort,
    /// Guaranteed delivery, events not lost
    Guaranteed,
}

impl Default for DeliveryMode {
    fn default() -> Self {
        DeliveryMode::BestEffort
    }
}

/// Message ordering guarantee.
///
/// See Engineering Plan § 2.12.6: Ordering Guarantees.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageOrdering {
    /// Maintain strict timestamp ordering
    StrictTimestamp,
    /// Causal ordering within trace
    CausalOrdering,
    /// No ordering guarantee
    BestEffort,
}

impl Default for MessageOrdering {
    fn default() -> Self {
        MessageOrdering::BestEffort
    }
}

/// Subscription descriptor.
#[derive(Clone, Debug)]
pub struct Subscription {
    /// Subscription ID
    pub id: SubscriptionId,
    /// Event filter criteria
    pub filter: EventFilter,
    /// Subscriber endpoint (e.g., "http://telemetry.internal:8080")
    pub subscriber_endpoint: String,
    /// Delivery guarantee mode
    pub delivery_mode: DeliveryMode,
    /// Message ordering requirement
    pub ordering: MessageOrdering,
}

impl Subscription {
    /// Creates a new subscription.
    pub fn new(
        id: SubscriptionId,
        filter: EventFilter,
        subscriber_endpoint: String,
        delivery_mode: DeliveryMode,
    ) -> Self {
        Subscription {
            id,
            filter,
            subscriber_endpoint,
            delivery_mode,
            ordering: MessageOrdering::default(),
        }
    }
}

/// Event buffer with ring buffer semantics and overflow policy.
///
/// See Engineering Plan § 2.12.6: Event Buffering.
#[derive(Clone, Debug)]
pub struct EventBuffer {
    /// Ring buffer of events
    pub events: Vec<CefEvent>,
    /// Current write position
    pub write_pos: usize,
    /// Overflow policy
    pub overflow_policy: OverflowPolicy,
    /// Capacity
    pub capacity: usize,
}

impl EventBuffer {
    /// Creates a new event buffer with specified capacity.
    pub fn new(capacity: usize, overflow_policy: OverflowPolicy) -> Self {
        EventBuffer {
            events: Vec::with_capacity(capacity),
            write_pos: 0,
            overflow_policy,
            capacity,
        }
    }

    /// Adds an event to the buffer.
    pub fn push(&mut self, event: CefEvent) -> Result<()> {
        if self.events.len() < self.capacity {
            self.events.push(event);
        } else {
            match self.overflow_policy {
                OverflowPolicy::Drop => {
                    // Drop oldest event and insert new one
                    if self.write_pos >= self.events.len() {
                        self.write_pos = 0;
                    }
                    if !self.events.is_empty() {
                        self.events[self.write_pos] = event;
                        self.write_pos = (self.write_pos + 1) % self.capacity;
                    }
                }
                OverflowPolicy::BackpressureSender => {
                    return Err(ToolError::Other(
                        "event buffer full, backpressure applied".to_string(),
                    ));
                }
                OverflowPolicy::PersistOverflow => {
                    // In production, would persist to storage
                    return Err(ToolError::Other(
                        "event buffer overflow, persisting to storage".to_string(),
                    ));
                }
            }
        }
        Ok(())
    }

    /// Returns the number of events currently in buffer.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Returns true if buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Returns true if buffer is at capacity.
    pub fn is_full(&self) -> bool {
        self.events.len() >= self.capacity
    }

    /// Clears all events from buffer.
    pub fn clear(&mut self) {
        self.events.clear();
        self.write_pos = 0;
    }
}

impl Default for EventBuffer {
    fn default() -> Self {
        EventBuffer::new(1000, OverflowPolicy::Drop)
    }
}

/// Metrics for streaming engine performance.
///
/// See Engineering Plan § 2.12.6: Streaming Metrics.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StreamingMetrics {
    /// Total events published
    pub events_published: u64,
    /// Total events successfully delivered
    pub events_delivered: u64,
    /// Total events dropped
    pub events_dropped: u64,
    /// Current number of active subscribers
    pub subscriber_count: u64,
    /// Average delivery latency in nanoseconds
    pub avg_latency_ns: u64,
}

impl StreamingMetrics {
    /// Creates new metrics.
    pub fn new() -> Self {
        StreamingMetrics {
            events_published: 0,
            events_delivered: 0,
            events_dropped: 0,
            subscriber_count: 0,
            avg_latency_ns: 0,
        }
    }

    /// Returns delivery success rate as percentage.
    pub fn delivery_rate(&self) -> f64 {
        if self.events_published == 0 {
            100.0
        } else {
            (self.events_delivered as f64 / self.events_published as f64) * 100.0
        }
    }
}

impl Default for StreamingMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for streaming event distribution.
///
/// See Engineering Plan § 2.12.6: Streaming Engine Interface.
pub trait StreamingEngine {
    /// Publishes an event to all matching subscribers.
    fn publish(&mut self, event: CefEvent) -> Result<()>;

    /// Creates a new subscription with the given filter.
    fn subscribe(&mut self, filter: EventFilter) -> Result<SubscriptionId>;

    /// Removes a subscription.
    fn unsubscribe(&mut self, sub_id: SubscriptionId) -> Result<()>;

    /// Returns current streaming metrics.
    fn metrics(&self) -> StreamingMetrics;
}

/// In-memory streaming engine implementation.
///
/// Uses ring buffer and fan-out to subscribers.
/// See Engineering Plan § 2.12.6: In-Memory Streaming.
#[derive(Clone, Debug)]
pub struct InMemoryStreamingEngine {
    /// Event buffer
    buffer: EventBuffer,
    /// Active subscriptions
    subscriptions: BTreeMap<u64, Subscription>,
    /// Next subscription ID
    next_sub_id: u64,
    /// Metrics
    metrics: StreamingMetrics,
}

impl InMemoryStreamingEngine {
    /// Creates a new in-memory streaming engine.
    pub fn new(buffer_capacity: usize) -> Self {
        InMemoryStreamingEngine {
            buffer: EventBuffer::new(buffer_capacity, OverflowPolicy::Drop),
            subscriptions: BTreeMap::new(),
            next_sub_id: 1,
            metrics: StreamingMetrics::new(),
        }
    }

    /// Creates with default buffer capacity (10000 events).
    pub fn default_capacity() -> Self {
        Self::new(10000)
    }

    /// Returns reference to event buffer.
    pub fn buffer(&self) -> &EventBuffer {
        &self.buffer
    }

    /// Returns number of active subscriptions.
    pub fn subscription_count(&self) -> usize {
        self.subscriptions.len()
    }

    /// Returns all active subscriptions.
    pub fn subscriptions(&self) -> Vec<&Subscription> {
        self.subscriptions.values().collect()
    }
}

impl StreamingEngine for InMemoryStreamingEngine {
    fn publish(&mut self, event: CefEvent) -> Result<()> {
        self.buffer.push(event.clone())?;
        self.metrics.events_published += 1;

        // Count delivered events to matching subscribers
        let mut delivered = 0;
        for sub in self.subscriptions.values() {
            if sub.filter.matches(&event) {
                delivered += 1;
            }
        }
        self.metrics.events_delivered += delivered as u64;

        Ok(())
    }

    fn subscribe(&mut self, filter: EventFilter) -> Result<SubscriptionId> {
        let sub_id = SubscriptionId::new(self.next_sub_id);
        self.next_sub_id += 1;

        let subscription = Subscription::new(
            sub_id,
            filter,
            "local://memory".to_string(),
            DeliveryMode::BestEffort,
        );

        self.subscriptions.insert(sub_id.as_u64(), subscription);
        self.metrics.subscriber_count = self.subscriptions.len() as u64;

        Ok(sub_id)
    }

    fn unsubscribe(&mut self, sub_id: SubscriptionId) -> Result<()> {
        self.subscriptions
            .remove(&sub_id.as_u64())
            .ok_or_else(|| {
                ToolError::Other(alloc::format!("subscription not found: {}", sub_id))
            })?;

        self.metrics.subscriber_count = self.subscriptions.len() as u64;
        Ok(())
    }

    fn metrics(&self) -> StreamingMetrics {
        self.metrics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::collections::BTreeSet;
use alloc::format;
use alloc::string::ToString;
use alloc::vec;

    #[test]
    fn test_subscription_id_creation() {
        let id = SubscriptionId::new(42);
        assert_eq!(id.as_u64(), 42);
    }

    #[test]
    fn test_subscription_id_display() {
        let id = SubscriptionId::new(42);
        assert_eq!(id.to_string(), "sub-42");
    }

    #[test]
    fn test_event_filter_accept_all() {
        let filter = EventFilter::accept_all();
        assert!(filter.event_types.is_empty());
        assert!(filter.actor_filter.is_none());
    }

    #[test]
    fn test_event_filter_for_event_types() {
        let filter = EventFilter::for_event_types(vec!["ThoughtStep", "ToolCallRequested"]);
        assert_eq!(filter.event_types.len(), 2);
        assert!(filter.event_types.contains("ThoughtStep"));
    }

    #[test]
    fn test_event_filter_matches_all() {
        let filter = EventFilter::accept_all();
        let event = crate::cef::CefEvent::new(
            "e1",
            "trace1",
            "span1",
            "ct1",
            "agent1",
            1000,
            crate::cef::CefEventType::ThoughtStep,
            "phase",
        );
        assert!(filter.matches(&event));
    }

    #[test]
    fn test_event_filter_matches_event_type() {
        let filter = EventFilter::for_event_types(vec!["ToolCallRequested"]);
        let event_thought = crate::cef::CefEvent::new(
            "e1",
            "trace1",
            "span1",
            "ct1",
            "agent1",
            1000,
            crate::cef::CefEventType::ThoughtStep,
            "phase",
        );
        let event_tool = crate::cef::CefEvent::new(
            "e2",
            "trace1",
            "span1",
            "ct1",
            "agent1",
            1000,
            crate::cef::CefEventType::ToolCallRequested,
            "phase",
        );

        assert!(!filter.matches(&event_thought));
        assert!(filter.matches(&event_tool));
    }

    #[test]
    fn test_event_filter_matches_actor() {
        let mut filter = EventFilter::accept_all();
        filter.actor_filter = Some("agent1".to_string());

        let event_match = crate::cef::CefEvent::new(
            "e1",
            "trace1",
            "span1",
            "ct1",
            "agent1-sub",
            1000,
            crate::cef::CefEventType::ThoughtStep,
            "phase",
        );
        let event_no_match = crate::cef::CefEvent::new(
            "e2",
            "trace1",
            "span1",
            "ct1",
            "agent2",
            1000,
            crate::cef::CefEventType::ThoughtStep,
            "phase",
        );

        assert!(filter.matches(&event_match));
        assert!(!filter.matches(&event_no_match));
    }

    #[test]
    fn test_event_filter_matches_trace_id_prefix() {
        let mut filter = EventFilter::accept_all();
        filter.trace_id_prefix = Some("trace-prod".to_string());

        let event_match = crate::cef::CefEvent::new(
            "e1",
            "trace-prod-001",
            "span1",
            "ct1",
            "agent1",
            1000,
            crate::cef::CefEventType::ThoughtStep,
            "phase",
        );
        let event_no_match = crate::cef::CefEvent::new(
            "e2",
            "trace-dev-001",
            "span1",
            "ct1",
            "agent1",
            1000,
            crate::cef::CefEventType::ThoughtStep,
            "phase",
        );

        assert!(filter.matches(&event_match));
        assert!(!filter.matches(&event_no_match));
    }

    #[test]
    fn test_event_buffer_creation() {
        let buffer = EventBuffer::new(100, OverflowPolicy::Drop);
        assert_eq!(buffer.capacity, 100);
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_event_buffer_push() {
        let mut buffer = EventBuffer::new(10, OverflowPolicy::Drop);
        let event = crate::cef::CefEvent::new(
            "e1",
            "trace1",
            "span1",
            "ct1",
            "agent1",
            1000,
            crate::cef::CefEventType::ThoughtStep,
            "phase",
        );
        assert!(buffer.push(event).is_ok());
        assert_eq!(buffer.len(), 1);
    }

    #[test]
    fn test_event_buffer_full() {
        let mut buffer = EventBuffer::new(2, OverflowPolicy::Drop);
        let event1 = crate::cef::CefEvent::new(
            "e1",
            "trace1",
            "span1",
            "ct1",
            "agent1",
            1000,
            crate::cef::CefEventType::ThoughtStep,
            "phase",
        );
        let event2 = crate::cef::CefEvent::new(
            "e2",
            "trace1",
            "span1",
            "ct1",
            "agent1",
            1000,
            crate::cef::CefEventType::ToolCallRequested,
            "phase",
        );

        buffer.push(event1).ok();
        buffer.push(event2).ok();
        assert!(buffer.is_full());
    }

    #[test]
    fn test_event_buffer_overflow_drop() {
        let mut buffer = EventBuffer::new(2, OverflowPolicy::Drop);
        let events: Vec<_> = (0..5)
            .map(|i| {
                crate::cef::CefEvent::new(
                    &alloc::format!("e{}", i),
                    "trace1",
                    "span1",
                    "ct1",
                    "agent1",
                    1000 + i as u64,
                    crate::cef::CefEventType::ThoughtStep,
                    "phase",
                )
            })
            .collect();

        for event in events {
            buffer.push(event).ok();
        }

        // With drop policy, buffer should still be at capacity
        assert!(buffer.len() <= 2);
    }

    #[test]
    fn test_event_buffer_overflow_backpressure() {
        let mut buffer = EventBuffer::new(1, OverflowPolicy::BackpressureSender);
        let event1 = crate::cef::CefEvent::new(
            "e1",
            "trace1",
            "span1",
            "ct1",
            "agent1",
            1000,
            crate::cef::CefEventType::ThoughtStep,
            "phase",
        );
        let event2 = crate::cef::CefEvent::new(
            "e2",
            "trace1",
            "span1",
            "ct1",
            "agent1",
            1000,
            crate::cef::CefEventType::ToolCallRequested,
            "phase",
        );

        assert!(buffer.push(event1).is_ok());
        assert!(buffer.push(event2).is_err());
    }

    #[test]
    fn test_event_buffer_clear() {
        let mut buffer = EventBuffer::new(10, OverflowPolicy::Drop);
        let event = crate::cef::CefEvent::new(
            "e1",
            "trace1",
            "span1",
            "ct1",
            "agent1",
            1000,
            crate::cef::CefEventType::ThoughtStep,
            "phase",
        );
        buffer.push(event).ok();
        assert!(!buffer.is_empty());

        buffer.clear();
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_streaming_metrics_creation() {
        let metrics = StreamingMetrics::new();
        assert_eq!(metrics.events_published, 0);
        assert_eq!(metrics.events_delivered, 0);
    }

    #[test]
    fn test_streaming_metrics_delivery_rate() {
        let metrics = StreamingMetrics {
            events_published: 100,
            events_delivered: 95,
            events_dropped: 5,
            subscriber_count: 1,
            avg_latency_ns: 1000,
        };
        assert_eq!(metrics.delivery_rate(), 95.0);
    }

    #[test]
    fn test_streaming_metrics_delivery_rate_empty() {
        let metrics = StreamingMetrics::new();
        assert_eq!(metrics.delivery_rate(), 100.0);
    }

    #[test]
    fn test_in_memory_streaming_engine_creation() {
        let engine = InMemoryStreamingEngine::new(1000);
        assert_eq!(engine.subscription_count(), 0);
        assert!(engine.buffer().is_empty());
    }

    #[test]
    fn test_in_memory_streaming_engine_subscribe() {
        let mut engine = InMemoryStreamingEngine::new(1000);
        let filter = EventFilter::accept_all();
        let result = engine.subscribe(filter);

        assert!(result.is_ok());
        assert_eq!(engine.subscription_count(), 1);
    }

    #[test]
    fn test_in_memory_streaming_engine_unsubscribe() {
        let mut engine = InMemoryStreamingEngine::new(1000);
        let filter = EventFilter::accept_all();
        let sub_id = engine.subscribe(filter).expect("subscribe failed");

        let result = engine.unsubscribe(sub_id);
        assert!(result.is_ok());
        assert_eq!(engine.subscription_count(), 0);
    }

    #[test]
    fn test_in_memory_streaming_engine_unsubscribe_nonexistent() {
        let mut engine = InMemoryStreamingEngine::new(1000);
        let sub_id = SubscriptionId::new(999);

        let result = engine.unsubscribe(sub_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_in_memory_streaming_engine_publish() {
        let mut engine = InMemoryStreamingEngine::new(1000);
        let event = crate::cef::CefEvent::new(
            "e1",
            "trace1",
            "span1",
            "ct1",
            "agent1",
            1000,
            crate::cef::CefEventType::ThoughtStep,
            "phase",
        );

        let result = engine.publish(event);
        assert!(result.is_ok());
        assert_eq!(engine.metrics().events_published, 1);
    }

    #[test]
    fn test_in_memory_streaming_engine_publish_with_subscriber() {
        let mut engine = InMemoryStreamingEngine::new(1000);
        let filter = EventFilter::accept_all();
        engine.subscribe(filter).ok();

        let event = crate::cef::CefEvent::new(
            "e1",
            "trace1",
            "span1",
            "ct1",
            "agent1",
            1000,
            crate::cef::CefEventType::ThoughtStep,
            "phase",
        );

        engine.publish(event).ok();
        let metrics = engine.metrics();
        assert_eq!(metrics.events_published, 1);
        assert_eq!(metrics.events_delivered, 1);
    }

    #[test]
    fn test_in_memory_streaming_engine_metrics() {
        let engine = InMemoryStreamingEngine::default_capacity();
        let metrics = engine.metrics();
        assert_eq!(metrics.events_published, 0);
    }
}
