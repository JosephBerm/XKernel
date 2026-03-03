// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Telemetry engine with CEF events, cost attribution, and streaming.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

/// CEF (Common Event Format) event with 20+ fields for compliance logging
#[derive(Debug, Clone)]
pub struct CefEvent {
    pub version: u8,
    pub device: String,
    pub event_id: u64,
    pub severity: u8, // 0-10
    pub extension: EventExtension,
}

#[derive(Debug, Clone)]
pub struct EventExtension {
    pub source_ip: String,
    pub destination_ip: String,
    pub source_port: u16,
    pub destination_port: u16,
    pub protocol: String,
    pub action: String,
    pub outcome: String,
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub start_time: u64,
    pub end_time: u64,
    pub request: String,
    pub response: String,
    pub file_path: String,
    pub file_size: u64,
    pub user_id: String,
    pub source_user_id: String,
    pub privilege_level: u8,
    pub event_type: String,
    pub category: String,
}

impl Default for EventExtension {
    fn default() -> Self {
        Self {
            source_ip: String::new(),
            destination_ip: String::new(),
            source_port: 0,
            destination_port: 0,
            protocol: String::new(),
            action: String::new(),
            outcome: String::new(),
            bytes_in: 0,
            bytes_out: 0,
            start_time: 0,
            end_time: 0,
            request: String::new(),
            response: String::new(),
            file_path: String::new(),
            file_size: 0,
            user_id: String::new(),
            source_user_id: String::new(),
            privilege_level: 0,
            event_type: String::new(),
            category: String::new(),
        }
    }
}

impl CefEvent {
    pub fn new(device: String, event_id: u64, severity: u8) -> Self {
        Self {
            version: 0,
            device,
            event_id,
            severity: severity.min(10),
            extension: EventExtension::default(),
        }
    }
}

impl fmt::Display for CefEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CEF:{}|{}|{}|{}",
            self.version, self.device, self.event_id, self.severity
        )
    }
}

/// Cost attribution for tool executions
#[derive(Debug, Clone, Copy, Default)]
pub struct CostAttribution {
    pub tool_id: u64,
    pub compute_cost: f64,
    pub memory_cost: f64,
    pub io_cost: f64,
    pub network_cost: f64,
    pub total_cost: f64,
}

impl CostAttribution {
    pub fn new(tool_id: u64) -> Self {
        Self {
            tool_id,
            ..Default::default()
        }
    }

    pub fn add_compute(&mut self, cost: f64) {
        self.compute_cost += cost;
        self.total_cost += cost;
    }

    pub fn add_memory(&mut self, cost: f64) {
        self.memory_cost += cost;
        self.total_cost += cost;
    }

    pub fn add_io(&mut self, cost: f64) {
        self.io_cost += cost;
        self.total_cost += cost;
    }

    pub fn add_network(&mut self, cost: f64) {
        self.network_cost += cost;
        self.total_cost += cost;
    }

    /// Get cost breakdown as percentages
    pub fn cost_breakdown(&self) -> (f64, f64, f64, f64) {
        if self.total_cost == 0.0 {
            return (0.0, 0.0, 0.0, 0.0);
        }

        (
            (self.compute_cost / self.total_cost) * 100.0,
            (self.memory_cost / self.total_cost) * 100.0,
            (self.io_cost / self.total_cost) * 100.0,
            (self.network_cost / self.total_cost) * 100.0,
        )
    }
}

/// Streaming event processor
#[derive(Debug, Clone)]
pub struct StreamingProcessor {
    buffer: Vec<CefEvent>,
    max_buffer_size: usize,
    flush_interval_ms: u32,
}

impl StreamingProcessor {
    pub fn new(max_buffer_size: usize, flush_interval_ms: u32) -> Self {
        Self {
            buffer: Vec::new(),
            max_buffer_size,
            flush_interval_ms,
        }
    }

    /// Add event to streaming buffer
    pub fn add_event(&mut self, event: CefEvent) -> Result<(), TelemetryError> {
        if self.buffer.len() >= self.max_buffer_size {
            self.flush()?;
        }

        self.buffer.push(event);
        Ok(())
    }

    /// Flush buffered events
    pub fn flush(&mut self) -> Result<(), TelemetryError> {
        if self.buffer.is_empty() {
            return Ok(());
        }

        // Simulate flushing (in real implementation, would stream to endpoint)
        self.buffer.clear();
        Ok(())
    }

    pub fn buffer_size(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_full(&self) -> bool {
        self.buffer.len() >= self.max_buffer_size
    }
}

impl Default for StreamingProcessor {
    fn default() -> Self {
        Self::new(1000, 5000)
    }
}

/// Telemetry errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TelemetryError {
    BufferFull,
    FlushFailed,
    SerializationFailed,
}

impl fmt::Display for TelemetryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BufferFull => write!(f, "telemetry buffer full"),
            Self::FlushFailed => write!(f, "telemetry flush failed"),
            Self::SerializationFailed => write!(f, "telemetry serialization failed"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cef_event() {
        let event = CefEvent::new(String::from("device1"), 1, 5);
        assert_eq!(event.event_id, 1);
        assert_eq!(event.severity, 5);
    }

    #[test]
    fn test_cost_attribution() {
        let mut cost = CostAttribution::new(1);
        cost.add_compute(50.0);
        cost.add_memory(30.0);
        cost.add_io(15.0);
        cost.add_network(5.0);

        assert_eq!(cost.total_cost, 100.0);

        let (compute_pct, memory_pct, io_pct, network_pct) = cost.cost_breakdown();
        assert!(compute_pct > 49.0 && compute_pct < 51.0);
        assert!(memory_pct > 29.0 && memory_pct < 31.0);
    }

    #[test]
    fn test_streaming_processor() {
        let mut processor = StreamingProcessor::new(10, 1000);

        for i in 0..5 {
            let event = CefEvent::new(String::from("device1"), i, 5);
            processor.add_event(event).unwrap();
        }

        assert_eq!(processor.buffer_size(), 5);

        processor.flush().unwrap();
        assert_eq!(processor.buffer_size(), 0);
    }

    #[test]
    fn test_streaming_buffer_full() {
        let mut processor = StreamingProcessor::new(2, 1000);

        processor.add_event(CefEvent::new(String::from("d1"), 1, 5)).unwrap();
        processor.add_event(CefEvent::new(String::from("d1"), 2, 5)).unwrap();

        // Adding 3rd should flush automatically
        processor.add_event(CefEvent::new(String::from("d1"), 3, 5)).unwrap();
    }
}
