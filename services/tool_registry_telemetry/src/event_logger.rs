// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Structured Event Logging for CEF Events
//!
//! Provides JSON-based structured logging for all CEF events with timestamp,
//! event type, cost metrics, and other relevant metadata.
//!
//! See Engineering Plan § 2.12: Cognitive Event Format & Telemetry,
//! and § 2.12.6: Event Streaming & Real-Time Telemetry.

use alloc::format;
use alloc::string::String;

use crate::cef::{CefEvent, CostAttribution};
use crate::error::Result;

/// Structured event logger for CEF events.
///
/// Produces JSON-formatted logs with the following structure:
/// ```json
/// {
///   "timestamp": "2026-03-01T12:34:56.789Z",
///   "timestamp_ns": 1234567890000000000,
///   "event_id": "evt-123",
///   "trace_id": "trace-001",
///   "span_id": "span-001",
///   "event_type": "ToolCallCompleted",
///   "agent_id": "agent-1",
///   "phase": "acting",
///   "cost_metrics": {
///     "tokens": 1024,
///     "gpu_milliseconds": 100,
///     "wall_clock_milliseconds": 150,
///     "tpc_hours": 0
///   },
///   "data_classification": "Internal"
/// }
/// ```
///
/// All events are logged with:
/// - ISO 8601 timestamp (for human readability)
/// - Nanosecond precision timestamp (for causality ordering)
/// - Complete cost attribution
/// - Event type classification
/// - Trace/span IDs for distributed tracing
///
/// See Engineering Plan § 2.12: CEF Event Structure.
#[derive(Clone, Debug)]
pub struct EventLogger;

impl EventLogger {
    /// Formats a CEF event as structured JSON log entry.
    ///
    /// # Arguments
    ///
    /// - `event`: The CEF event to log
    ///
    /// # Returns
    ///
    /// A JSON-formatted log entry string.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let event = CefEvent::new(
    ///     "evt-1",
    ///     "trace-1",
    ///     "span-1",
    ///     "ct-1",
    ///     "agent-1",
    ///     1000,
    ///     CefEventType::ToolCallCompleted,
    ///     "acting",
    /// );
    /// let log_entry = EventLogger::format_json(&event);
    /// assert!(log_entry.contains("evt-1"));
    /// ```
    ///
    /// See Engineering Plan § 2.12: JSON Logging Format.
    pub fn format_json(event: &CefEvent) -> String {
        let cost_json = Self::format_cost_metrics(&event.cost);
        
        // Format event with all metadata
        format!(
            r#"{{"timestamp_ns":{},"event_id":"{}","trace_id":"{}","span_id":"{}","ct_id":"{}","event_type":"{}","agent_id":"{}","phase":"{}","data_classification":"{}","cost_metrics":{}}}"#,
            event.timestamp_ns,
            event.event_id,
            event.trace_id,
            event.span_id,
            event.ct_id,
            event.event_type,
            event.agent_id,
            event.phase,
            event.data_classification,
            cost_json
        )
    }

    /// Formats cost attribution metrics as JSON.
    ///
    /// # Arguments
    ///
    /// - `cost`: Cost attribution to format
    ///
    /// # Returns
    ///
    /// A JSON object string representing the cost metrics.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let cost = CostAttribution::new(1024, 100, 150, 0);
    /// let cost_json = EventLogger::format_cost_metrics(&cost);
    /// assert!(cost_json.contains("tokens"));
    /// ```
    pub fn format_cost_metrics(cost: &CostAttribution) -> String {
        format!(
            r#"{{"tokens":{},"gpu_milliseconds":{},"wall_clock_milliseconds":{},"tpc_hours":{}}}"#,
            cost.tokens,
            cost.gpu_ms,
            cost.wall_clock_ms,
            cost.tpc_hours
        )
    }

    /// Logs an event to a buffer (for testing and in-memory logging).
    ///
    /// # Arguments
    ///
    /// - `event`: The event to log
    /// - `buffer`: Mutable reference to log buffer
    ///
    /// # Returns
    ///
    /// - `Ok(())`: Event logged successfully
    /// - `Err(ToolError)`: Logging failed
    ///
    /// # Example
    ///
    /// ```ignore
    /// let event = CefEvent::new(...);
    /// let mut logs = alloc::vec::Vec::new();
    /// EventLogger::log_to_buffer(&event, &mut logs)?;
    /// assert!(!logs.is_empty());
    /// ```
    pub fn log_to_buffer(event: &CefEvent, buffer: &mut alloc::vec::Vec<String>) -> Result<()> {
        let log_entry = Self::format_json(event);
        buffer.push(log_entry);
        Ok(())
    }

    /// Logs an event and returns the formatted entry.
    ///
    /// # Arguments
    ///
    /// - `event`: The event to log
    ///
    /// # Returns
    ///
    /// - `Ok(log_entry)`: Formatted log entry
    /// - `Err(ToolError)`: Logging failed
    ///
    /// # Example
    ///
    /// ```ignore
    /// let event = CefEvent::new(...);
    /// let log_entry = EventLogger::log(&event)?;
    /// ```
    pub fn log(event: &CefEvent) -> Result<String> {
        Ok(Self::format_json(event))
    }

    /// Formats multiple events as a JSON array.
    ///
    /// Useful for batch logging operations.
    ///
    /// # Arguments
    ///
    /// - `events`: Slice of events to format
    ///
    /// # Returns
    ///
    /// A JSON array string.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let events = vec![event1, event2];
    /// let json_array = EventLogger::format_json_array(&events);
    /// assert!(json_array.starts_with("["));
    /// ```
    pub fn format_json_array(events: &[CefEvent]) -> String {
        let entries: alloc::vec::Vec<String> = events
            .iter()
            .map(|e| Self::format_json(e))
            .collect();

        format!("[{}]", entries.join(","))
    }

    /// Logs events as a batch JSON array to a buffer.
    ///
    /// # Arguments
    ///
    /// - `events`: Slice of events to log
    /// - `buffer`: Mutable reference to log buffer
    ///
    /// # Returns
    ///
    /// - `Ok(count)`: Number of events logged
    /// - `Err(ToolError)`: Logging failed
    pub fn log_batch_to_buffer(
        events: &[CefEvent],
        buffer: &mut alloc::vec::Vec<String>,
    ) -> Result<usize> {
        let count = events.len();
        for event in events {
            Self::log_to_buffer(event, buffer)?;
        }
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cef::CefEventType;
use alloc::vec::Vec;
use alloc::vec;

    #[test]
    fn test_format_cost_metrics() {
        let cost = CostAttribution::new(1024, 100, 150, 5);
        let json = EventLogger::format_cost_metrics(&cost);

        assert!(json.contains("tokens"));
        assert!(json.contains("1024"));
        assert!(json.contains("gpu_milliseconds"));
        assert!(json.contains("100"));
        assert!(json.contains("wall_clock_milliseconds"));
        assert!(json.contains("150"));
        assert!(json.contains("tpc_hours"));
        assert!(json.contains("5"));
    }

    #[test]
    fn test_format_json_single_event() {
        let event = CefEvent::new(
            "evt-1",
            "trace-1",
            "span-1",
            "ct-1",
            "agent-1",
            1234567890,
            CefEventType::ToolCallCompleted,
            "acting",
        );

        let json = EventLogger::format_json(&event);

        assert!(json.contains("evt-1"));
        assert!(json.contains("trace-1"));
        assert!(json.contains("span-1"));
        assert!(json.contains("ct-1"));
        assert!(json.contains("agent-1"));
        assert!(json.contains("ToolCallCompleted"));
        assert!(json.contains("acting"));
        assert!(json.contains("cost_metrics"));
    }

    #[test]
    fn test_log_to_buffer() {
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

        let mut logs = alloc::vec::Vec::new();
        EventLogger::log_to_buffer(&event, &mut logs).unwrap();

        assert_eq!(logs.len(), 1);
        assert!(logs[0].contains("evt-1"));
    }

    #[test]
    fn test_log() {
        let event = CefEvent::new(
            "evt-1",
            "trace-1",
            "span-1",
            "ct-1",
            "agent-1",
            1000,
            CefEventType::ToolCallCompleted,
            "acting",
        );

        let log_entry = EventLogger::log(&event).unwrap();
        assert!(log_entry.contains("evt-1"));
        assert!(log_entry.contains("cost_metrics"));
    }

    #[test]
    fn test_format_json_array() {
        let event1 = CefEvent::new(
            "evt-1",
            "trace-1",
            "span-1",
            "ct-1",
            "agent-1",
            1000,
            CefEventType::ToolCallRequested,
            "acting",
        );

        let event2 = CefEvent::new(
            "evt-2",
            "trace-1",
            "span-2",
            "ct-1",
            "agent-1",
            2000,
            CefEventType::ToolCallCompleted,
            "acting",
        );

        let events = alloc::vec![event1, event2];
        let json_array = EventLogger::format_json_array(&events);

        assert!(json_array.starts_with("["));
        assert!(json_array.ends_with("]"));
        assert!(json_array.contains("evt-1"));
        assert!(json_array.contains("evt-2"));
    }

    #[test]
    fn test_log_batch_to_buffer() {
        let event1 = CefEvent::new(
            "evt-1",
            "trace-1",
            "span-1",
            "ct-1",
            "agent-1",
            1000,
            CefEventType::ToolCallRequested,
            "acting",
        );

        let event2 = CefEvent::new(
            "evt-2",
            "trace-1",
            "span-2",
            "ct-1",
            "agent-1",
            2000,
            CefEventType::ToolCallCompleted,
            "acting",
        );

        let events = alloc::vec![event1, event2];
        let mut logs = alloc::vec::Vec::new();

        let count = EventLogger::log_batch_to_buffer(&events, &mut logs).unwrap();
        assert_eq!(count, 2);
        assert_eq!(logs.len(), 2);
    }

    #[test]
    fn test_empty_cost_attribution() {
        let cost = CostAttribution::zero();
        let json = EventLogger::format_cost_metrics(&cost);

        assert!(json.contains("tokens"));
        assert!(json.contains("0"));
    }

    #[test]
    fn test_json_contains_all_fields() {
        let event = CefEvent::new(
            "evt-123",
            "trace-456",
            "span-789",
            "ct-999",
            "agent-1",
            9876543210,
            CefEventType::ToolCallCompleted,
            "thinking",
        );

        let json = EventLogger::format_json(&event);

        // Verify all key fields are present
        assert!(json.contains("\"timestamp_ns\""));
        assert!(json.contains("\"event_id\""));
        assert!(json.contains("\"trace_id\""));
        assert!(json.contains("\"span_id\""));
        assert!(json.contains("\"ct_id\""));
        assert!(json.contains("\"event_type\""));
        assert!(json.contains("\"agent_id\""));
        assert!(json.contains("\"phase\""));
        assert!(json.contains("\"data_classification\""));
        assert!(json.contains("\"cost_metrics\""));
    }

    #[test]
    fn test_cost_with_large_numbers() {
        let cost = CostAttribution::new(1_000_000, 100_000, 1_000_000, 1000);
        let json = EventLogger::format_cost_metrics(&cost);

        assert!(json.contains("1000000"));
        assert!(json.contains("100000"));
    }
}
