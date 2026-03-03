// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Telemetry Engine - CEF Event Emission and Buffering
//!
//! Implements the core telemetry engine for the Cognitive Substrate OS,
//! providing event buffering with ring buffer semantics, event serialization,
//! and subscriber notification.
//!
//! See Engineering Plan § 2.12: Cognitive Event Format & Telemetry,
//! and Week 5 Objective: Telemetry Engine Implementation.

use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use crate::cef::CefEvent;
use crate::error::{Result, ToolError};
use crate::serialization::{EventSerializer, JsonEventSerializer};
use crate::streaming::{EventFilter, Subscription, SubscriptionId};

/// Maximum number of events that can be buffered in-memory.
///
/// Per Engineering Plan § 2.12.6: Event Buffering,
/// the ring buffer capacity is 10,000 events. Oldest events are evicted when exceeded.
const TELEMETRY_ENGINE_CAPACITY: usize = 10_000;

/// Telemetry Engine for CEF event emission, buffering, and distribution.
///
/// Manages event buffering with a VecDeque (ring buffer semantics), JSON serialization,
/// stdout logging, and subscriber notification for real-time telemetry consumption.
///
/// See Engineering Plan § 2.12.6: Event Streaming & Real-Time Telemetry.
#[derive(Clone, Debug)]
pub struct TelemetryEngine {
    /// Ring buffer of events (VecDeque with max 10k events)
    event_buffer: VecDeque<CefEvent>,
    /// List of active subscribers
    subscribers: Vec<Subscription>,
    /// Subscriber ID counter for generating unique IDs
    next_subscriber_id: u64,
    /// Event serializer (JSON format)
    serializer: JsonEventSerializer,
    /// Event counter (total events emitted)
    event_count: u64,
    /// Log buffer for structured logging
    log_buffer: Vec<String>,
}

impl TelemetryEngine {
    /// Creates a new telemetry engine.
    ///
    /// Initializes an empty event buffer with capacity for 10,000 events,
    /// an empty subscriber list, and a JSON serializer.
    ///
    /// # Returns
    ///
    /// A new TelemetryEngine ready to emit and buffer events.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let engine = TelemetryEngine::new();
    /// assert_eq!(engine.event_count(), 0);
    /// assert_eq!(engine.buffer_size(), 0);
    /// ```
    ///
    /// See Engineering Plan § 2.12.6: Telemetry Engine Initialization.
    pub fn new() -> Self {
        TelemetryEngine {
            event_buffer: VecDeque::with_capacity(TELEMETRY_ENGINE_CAPACITY),
            subscribers: Vec::new(),
            next_subscriber_id: 1,
            serializer: JsonEventSerializer,
            event_count: 0,
            log_buffer: Vec::new(),
        }
    }

    /// Emits a CEF event to the telemetry engine.
    ///
    /// Performs the following actions in order:
    /// 1. Serializes the event to JSON format
    /// 2. Logs the JSON to the internal log buffer for structured logging
    /// 3. Adds the event to the in-memory ring buffer (evicts oldest if full)
    /// 4. Notifies all matching subscribers
    ///
    /// # Arguments
    ///
    /// - `event`: The CEF event to emit
    ///
    /// # Returns
    ///
    /// - `Ok(event_id)`: Event successfully emitted, returns the event ID
    /// - `Err(ToolError)`: Serialization or buffering failure
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut engine = TelemetryEngine::new();
    /// let event = CefEvent::new(
    ///     "event-1",
    ///     "trace-1",
    ///     "span-1",
    ///     "ct-1",
    ///     "agent-1",
    ///     1000,
    ///     CefEventType::ToolCallRequested,
    ///     "acting",
    /// );
    /// let event_id = engine.emit_event(event)?;
    /// assert_eq!(engine.buffer_size(), 1);
    /// ```
    ///
    /// See Engineering Plan § 2.12.6: Event Emission & Buffering.
    pub fn emit_event(&mut self, event: CefEvent) -> Result<String> {
        let event_id = event.event_id.clone();

        // 1. Serialize to JSON
        let json_bytes = self.serializer.serialize(&event)
            .map_err(|e| ToolError::Other(format!("event serialization failed: {}", e)))?;

        // Convert bytes to string for logging
        let json_str = alloc::string::String::from_utf8(json_bytes)
            .map_err(|_| ToolError::Other("invalid UTF-8 in serialized event".to_string()))?;

        // 2. Log to internal buffer (structured logging)
        self.log_buffer.push(json_str);

        // 3. Add to buffer with eviction on overflow
        if self.event_buffer.len() >= TELEMETRY_ENGINE_CAPACITY {
            // Evict oldest event (front of VecDeque)
            self.event_buffer.pop_front();
        }
        self.event_buffer.push_back(event.clone());

        // 4. Notify subscribers
        self.notify_subscribers(&event)?;

        // Update counter
        self.event_count = self.event_count.saturating_add(1);

        Ok(event_id)
    }

    /// Notifies all matching subscribers of an event.
    ///
    /// Iterates through all active subscribers, checks if the event matches
    /// their filter criteria, and sends the event to matching subscribers.
    ///
    /// # Arguments
    ///
    /// - `event`: The event to notify subscribers about
    ///
    /// # Returns
    ///
    /// - `Ok(())`: Notification completed (errors in individual subscribers are logged but don't fail)
    /// - `Err(ToolError)`: Critical failure in notification process
    ///
    /// # Implementation Notes
    ///
    /// Currently this is a stub that records subscriber notifications to the log buffer.
    /// In Phase 1, this will integrate with actual in-memory channels.
    ///
    /// See Engineering Plan § 2.12.6: Event Subscriber Interface.
    fn notify_subscribers(&mut self, event: &CefEvent) -> Result<()> {
        for subscriber in &self.subscribers {
            if subscriber.filter.matches(event) {
                // In Phase 1: send to actual channel
                // For now: log notification
                let msg = alloc::format!(
                    "TELEMETRY: Subscriber {} matched event {}",
                    subscriber.id, event.event_id
                );
                self.log_buffer.push(msg);
            }
        }
        Ok(())
    }

    /// Subscribes to events with a filter.
    ///
    /// Creates a new subscription that will receive events matching the filter criteria.
    /// Returns a subscription ID that can be used to unsubscribe later.
    ///
    /// # Arguments
    ///
    /// - `filter`: Event filter criteria (event types, actor, resource pattern, etc)
    /// - `subscriber_endpoint`: Subscriber endpoint (e.g., "http://telemetry.internal:8080")
    ///
    /// # Returns
    ///
    /// - `Ok(subscription_id)`: Subscription created successfully
    /// - `Err(ToolError)`: Subscription creation failed
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut engine = TelemetryEngine::new();
    /// let filter = EventFilter::for_event_types(vec!["ToolCallCompleted"]);
    /// let sub_id = engine.subscribe(filter, "http://localhost:8080".to_string())?;
    /// ```
    ///
    /// See Engineering Plan § 2.12.6: Event Subscriber Interface.
    pub fn subscribe(&mut self, filter: EventFilter, subscriber_endpoint: String) -> Result<SubscriptionId> {
        let sub_id = SubscriptionId::new(self.next_subscriber_id);
        self.next_subscriber_id = self.next_subscriber_id.saturating_add(1);

        let subscription = Subscription::new(
            sub_id,
            filter,
            subscriber_endpoint,
            crate::streaming::DeliveryMode::BestEffort,
        );

        self.subscribers.push(subscription);

        Ok(sub_id)
    }

    /// Unsubscribes a previously created subscription.
    ///
    /// # Arguments
    ///
    /// - `subscription_id`: The subscription ID to remove
    ///
    /// # Returns
    ///
    /// - `Ok(())`: Subscription removed successfully
    /// - `Err(ToolError)`: Subscription not found
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut engine = TelemetryEngine::new();
    /// let filter = EventFilter::accept_all();
    /// let sub_id = engine.subscribe(filter, "http://localhost:8080".to_string())?;
    /// engine.unsubscribe(sub_id)?;
    /// ```
    pub fn unsubscribe(&mut self, subscription_id: SubscriptionId) -> Result<()> {
        if let Some(pos) = self.subscribers.iter().position(|s| s.id == subscription_id) {
            self.subscribers.remove(pos);
            Ok(())
        } else {
            Err(ToolError::Other(format!(
                "subscription not found: {}",
                subscription_id
            )))
        }
    }

    /// Flushes the event buffer.
    ///
    /// In this basic implementation, "flush" means to clear the buffer
    /// (in production, would flush to persistent storage or external system).
    ///
    /// # Returns
    ///
    /// - `Ok(count)`: Number of events flushed
    /// - `Err(ToolError)`: Flush operation failed
    ///
    /// # Implementation Notes
    ///
    /// Currently this clears the in-memory buffer. In Phase 1, this will be enhanced to:
    /// - Write to persistent storage (Parquet, CapnProto, etc)
    /// - Send to external telemetry backend
    /// - Support batching and compression
    ///
    /// See Engineering Plan § 2.12.6: Event Flushing & Persistence.
    pub fn flush(&mut self) -> Result<usize> {
        let count = self.event_buffer.len();
        self.event_buffer.clear();
        Ok(count)
    }

    /// Returns the current number of events in the buffer.
    ///
    /// # Returns
    ///
    /// Current buffer size (0 to 10,000).
    ///
    /// # Example
    ///
    /// ```ignore
    /// let engine = TelemetryEngine::new();
    /// assert_eq!(engine.buffer_size(), 0);
    /// ```
    pub fn buffer_size(&self) -> usize {
        self.event_buffer.len()
    }

    /// Returns the total number of events emitted since engine creation.
    ///
    /// This counter never resets and is useful for telemetry metrics.
    ///
    /// # Returns
    ///
    /// Total event count.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut engine = TelemetryEngine::new();
    /// assert_eq!(engine.event_count(), 0);
    /// ```
    pub fn event_count(&self) -> u64 {
        self.event_count
    }

    /// Returns the number of active subscribers.
    ///
    /// # Returns
    ///
    /// Current subscriber count.
    pub fn subscriber_count(&self) -> usize {
        self.subscribers.len()
    }

    /// Retrieves events from the buffer as a vector.
    ///
    /// Returns a snapshot of the current buffer contents. Note that the buffer
    /// is a ring buffer that automatically evicts oldest events when full.
    ///
    /// # Returns
    ///
    /// Vector of buffered events (in insertion order).
    ///
    /// # Example
    ///
    /// ```ignore
    /// let engine = TelemetryEngine::new();
    /// let events = engine.get_events();
    /// assert_eq!(events.len(), 0);
    /// ```
    pub fn get_events(&self) -> Vec<CefEvent> {
        self.event_buffer.iter().cloned().collect()
    }

    /// Retrieves a single event from the buffer by event ID.
    ///
    /// # Arguments
    ///
    /// - `event_id`: The event ID to search for
    ///
    /// # Returns
    ///
    /// - `Some(event)`: Event found in buffer
    /// - `None`: Event not found
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut engine = TelemetryEngine::new();
    /// let event = CefEvent::new(...);
    /// let event_id = event.event_id.clone();
    /// engine.emit_event(event)?;
    /// let found = engine.get_event(&event_id);
    /// assert!(found.is_some());
    /// ```
    pub fn get_event(&self, event_id: &str) -> Option<CefEvent> {
        self.event_buffer
            .iter()
            .find(|e| e.event_id == event_id)
            .cloned()
    }

    /// Retrieves the log buffer (for testing and debugging).
    ///
    /// # Returns
    ///
    /// Vector of log entries (JSON strings and notifications).
    ///
    /// # Example
    ///
    /// ```ignore
    /// let engine = TelemetryEngine::new();
    /// let logs = engine.get_logs();
    /// assert_eq!(logs.len(), 0);
    /// ```
    pub fn get_logs(&self) -> Vec<String> {
        self.log_buffer.clone()
    }

    /// Clears the log buffer.
    ///
    /// # Returns
    ///
    /// Number of log entries cleared.
    pub fn clear_logs(&mut self) -> usize {
        let count = self.log_buffer.len();
        self.log_buffer.clear();
        count
    }
}

impl Default for TelemetryEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cef::CefEventType;
use alloc::format;
use alloc::string::ToString;
use alloc::vec;

    #[test]
    fn test_telemetry_engine_creation() {
        let engine = TelemetryEngine::new();
        assert_eq!(engine.buffer_size(), 0);
        assert_eq!(engine.event_count(), 0);
        assert_eq!(engine.subscriber_count(), 0);
    }

    #[test]
    fn test_emit_single_event() {
        let mut engine = TelemetryEngine::new();
        let event = CefEvent::new(
            "evt-1",
            "trace-1",
            "span-1",
            "ct-1",
            "agent-1",
            1000,
            CefEventType::ToolCallRequested,
            "acting",
        );
        let event_id = engine.emit_event(event).unwrap();
        assert_eq!(event_id, "evt-1");
        assert_eq!(engine.buffer_size(), 1);
        assert_eq!(engine.event_count(), 1);
    }

    #[test]
    fn test_emit_multiple_events() {
        let mut engine = TelemetryEngine::new();
        for i in 0..5 {
            let event = CefEvent::new(
                &format!("evt-{}", i),
                "trace-1",
                "span-1",
                "ct-1",
                "agent-1",
                1000,
                CefEventType::ToolCallRequested,
                "acting",
            );
            engine.emit_event(event).unwrap();
        }
        assert_eq!(engine.buffer_size(), 5);
        assert_eq!(engine.event_count(), 5);
    }

    #[test]
    fn test_ring_buffer_eviction() {
        let mut engine = TelemetryEngine::new();
        
        // Fill buffer beyond capacity to test eviction
        for i in 0..(TELEMETRY_ENGINE_CAPACITY + 100) {
            let event = CefEvent::new(
                &format!("evt-{}", i),
                "trace-1",
                "span-1",
                "ct-1",
                "agent-1",
                1000,
                CefEventType::ToolCallRequested,
                "acting",
            );
            engine.emit_event(event).unwrap();
        }
        
        // Buffer should not exceed capacity
        assert_eq!(engine.buffer_size(), TELEMETRY_ENGINE_CAPACITY);
        // But event count should include all emissions
        assert_eq!(engine.event_count(), (TELEMETRY_ENGINE_CAPACITY + 100) as u64);
        
        // Check that we have the most recent events (last 100)
        let events = engine.get_events();
        let last_event_idx = (TELEMETRY_ENGINE_CAPACITY + 99) as i32;
        assert_eq!(events[0].event_id, format!("evt-{}", last_event_idx - (TELEMETRY_ENGINE_CAPACITY as i32 - 1)));
    }

    #[test]
    fn test_get_event() {
        let mut engine = TelemetryEngine::new();
        let event = CefEvent::new(
            "evt-1",
            "trace-1",
            "span-1",
            "ct-1",
            "agent-1",
            1000,
            CefEventType::ToolCallRequested,
            "acting",
        );
        engine.emit_event(event).unwrap();
        
        let found = engine.get_event("evt-1");
        assert!(found.is_some());
        assert_eq!(found.unwrap().event_id, "evt-1");
        
        let not_found = engine.get_event("evt-999");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_subscribe() {
        let mut engine = TelemetryEngine::new();
        let filter = EventFilter::accept_all();
        let sub_id = engine.subscribe(filter, "http://localhost:8080".to_string()).unwrap();
        assert_eq!(engine.subscriber_count(), 1);
        assert_eq!(sub_id.as_u64(), 1);
    }

    #[test]
    fn test_multiple_subscriptions() {
        let mut engine = TelemetryEngine::new();
        let filter = EventFilter::accept_all();
        
        let sub1 = engine.subscribe(filter.clone(), "http://localhost:8080".to_string()).unwrap();
        let sub2 = engine.subscribe(filter.clone(), "http://localhost:8081".to_string()).unwrap();
        let sub3 = engine.subscribe(filter, "http://localhost:8082".to_string()).unwrap();
        
        assert_eq!(engine.subscriber_count(), 3);
        assert_eq!(sub1.as_u64(), 1);
        assert_eq!(sub2.as_u64(), 2);
        assert_eq!(sub3.as_u64(), 3);
    }

    #[test]
    fn test_unsubscribe() {
        let mut engine = TelemetryEngine::new();
        let filter = EventFilter::accept_all();
        let sub_id = engine.subscribe(filter, "http://localhost:8080".to_string()).unwrap();
        
        assert_eq!(engine.subscriber_count(), 1);
        engine.unsubscribe(sub_id).unwrap();
        assert_eq!(engine.subscriber_count(), 0);
    }

    #[test]
    fn test_unsubscribe_not_found() {
        let mut engine = TelemetryEngine::new();
        let result = engine.unsubscribe(SubscriptionId::new(999));
        assert!(result.is_err());
    }

    #[test]
    fn test_flush() {
        let mut engine = TelemetryEngine::new();
        for i in 0..3 {
            let event = CefEvent::new(
                &format!("evt-{}", i),
                "trace-1",
                "span-1",
                "ct-1",
                "agent-1",
                1000,
                CefEventType::ToolCallRequested,
                "acting",
            );
            engine.emit_event(event).unwrap();
        }
        
        assert_eq!(engine.buffer_size(), 3);
        let flushed = engine.flush().unwrap();
        assert_eq!(flushed, 3);
        assert_eq!(engine.buffer_size(), 0);
        assert_eq!(engine.event_count(), 3); // count not reset
    }

    #[test]
    fn test_default_creation() {
        let engine = TelemetryEngine::default();
        assert_eq!(engine.buffer_size(), 0);
        assert_eq!(engine.event_count(), 0);
    }

    #[test]
    fn test_log_buffer() {
        let mut engine = TelemetryEngine::new();
        let event = CefEvent::new(
            "evt-1",
            "trace-1",
            "span-1",
            "ct-1",
            "agent-1",
            1000,
            CefEventType::ToolCallRequested,
            "acting",
        );
        engine.emit_event(event).unwrap();
        
        let logs = engine.get_logs();
        assert!(!logs.is_empty());
        // Log should contain JSON representation
        assert!(logs[0].contains("evt-1"));
    }

    #[test]
    fn test_clear_logs() {
        let mut engine = TelemetryEngine::new();
        let event = CefEvent::new(
            "evt-1",
            "trace-1",
            "span-1",
            "ct-1",
            "agent-1",
            1000,
            CefEventType::ToolCallRequested,
            "acting",
        );
        engine.emit_event(event).unwrap();
        
        assert!(!engine.get_logs().is_empty());
        let cleared = engine.clear_logs();
        assert_eq!(cleared, 1);
        assert!(engine.get_logs().is_empty());
    }
}
