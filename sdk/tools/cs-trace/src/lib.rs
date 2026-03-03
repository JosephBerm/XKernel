// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! # Cognitive Substrate Tracer (cs-trace)
//!
//! The cs-trace crate provides syscall and event tracing for Cognitive Substrate,
//! enabling performance analysis and debugging of cognitive task execution.
//!
//! ## Architecture
//!
//! This crate defines:
//! - **TraceSession**: Tracing session lifecycle management
//! - **TraceFilter**: Event filtering criteria
//! - **TraceOutput**: Structured trace output


#![forbid(unsafe_code)]
#![warn(missing_docs)]





use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};




/// Trace event type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TraceEventType {
    /// Syscall entry
    SyscallEntry,
    /// Syscall exit
    SyscallExit,
    /// Task spawn
    TaskSpawn,
    /// Task yield
    TaskYield,
    /// Task resume
    TaskResume,
    /// Task completion
    TaskCompletion,
    /// Checkpoint created
    CheckpointCreate,
    /// Memory allocation
    MemoryAlloc,
    /// Memory deallocation
    MemoryFree,
}

/// Trace entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEntry {
    /// Event timestamp (nanoseconds since epoch)
    pub timestamp_ns: u64,
    /// Event type
    pub event_type: TraceEventType,
    /// Task/process ID
    pub task_id: String,
    /// Event details
    pub details: String,
    /// Associated metadata
    pub metadata: BTreeMap<String, String>,
}

/// Trace filter for event selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceFilter {
    /// Filter by event type
    pub event_types: Vec<TraceEventType>,
    /// Filter by task ID pattern
    pub task_id_pattern: Option<String>,
    /// Minimum timestamp (nanoseconds)
    pub min_timestamp_ns: Option<u64>,
    /// Maximum timestamp (nanoseconds)
    pub max_timestamp_ns: Option<u64>,
}

impl TraceFilter {
    /// Create a new filter matching all events
    pub fn all() -> Self {
        TraceFilter {
            event_types: Vec::new(),
            task_id_pattern: None,
            min_timestamp_ns: None,
            max_timestamp_ns: None,
        }
    }

    /// Check if entry matches filter
    pub fn matches(&self, entry: &TraceEntry) -> bool {
        if !self.event_types.is_empty() && !self.event_types.contains(&entry.event_type) {
            return false;
        }

        if let Some(ref pattern) = self.task_id_pattern {
            if !entry.task_id.contains(pattern.as_str()) {
                return false;
            }
        }

        if let Some(min) = self.min_timestamp_ns {
            if entry.timestamp_ns < min {
                return false;
            }
        }

        if let Some(max) = self.max_timestamp_ns {
            if entry.timestamp_ns > max {
                return false;
            }
        }

        true
    }
}

/// Trace output format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TraceOutput {
    /// Human-readable format
    Text(Vec<String>),
    /// Structured JSON records
    Json(Vec<TraceEntry>),
    /// Binary format
    Binary(Vec<u8>),
}

/// Tracing session
#[derive(Debug)]
pub struct TraceSession {
    /// Session ID
    pub session_id: String,
    /// Is recording
    pub recording: bool,
    /// Recorded entries
    pub entries: Vec<TraceEntry>,
    /// Active filters
    pub filters: Vec<TraceFilter>,
}

impl TraceSession {
    /// Create new trace session
    pub fn new(session_id: String) -> Self {
        TraceSession {
            session_id,
            recording: false,
            entries: Vec::new(),
            filters: Vec::new(),
        }
    }

    /// Start recording
    pub fn start_recording(&mut self) {
        self.recording = true;
    }

    /// Stop recording
    pub fn stop_recording(&mut self) {
        self.recording = false;
    }

    /// Add trace entry
    pub fn add_entry(&mut self, entry: TraceEntry) {
        if self.recording {
            self.entries.push(entry);
        }
    }

    /// Add filter
    pub fn add_filter(&mut self, filter: TraceFilter) {
        self.filters.push(filter);
    }

    /// Clear filters
    pub fn clear_filters(&mut self) {
        self.filters.clear();
    }

    /// Get filtered entries
    pub fn get_filtered_entries(&self) -> Vec<TraceEntry> {
        if self.filters.is_empty() {
            return self.entries.clone();
        }

        self.entries.iter()
            .filter(|entry| {
                self.filters.iter().any(|filter| filter.matches(entry))
            })
            .cloned()
            .collect()
    }

    /// Format as text output
    pub fn format_text(&self) -> Vec<String> {
        let entries = self.get_filtered_entries();
        entries.iter()
            .map(|e| {
                let mut line = String::new();
                line.push_str(&e.timestamp_ns.to_string());
                line.push(' ');
                line.push_str(&format!("{:?}", e.event_type));
                line.push(' ');
                line.push_str(&e.task_id);
                line.push_str(" : ");
                line.push_str(&e.details);
                line
            })
            .collect()
    }

    /// Get statistics
    pub fn get_stats(&self) -> BTreeMap<String, String> {
        let mut stats = BTreeMap::new();
        stats.insert("total_entries".to_string(), self.entries.len().to_string());
        stats.insert("filtered_entries".to_string(), self.get_filtered_entries().len().to_string());
        stats.insert("recording".to_string(), self.recording.to_string());

        let mut type_counts: BTreeMap<String, usize> = BTreeMap::new();
        for entry in &self.entries {
            let type_str = format!("{:?}", entry.event_type);
            *type_counts.entry(type_str).or_insert(0) += 1;
        }

        for (type_str, count) in type_counts {
            stats.insert(format!("count_{}", type_str), count.to_string());
        }

        stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trace_session_creation() {
        let session = TraceSession::new("test_session".to_string());
        assert_eq!(session.session_id, "test_session");
        assert!(!session.recording);
    }

    #[test]
    fn test_trace_session_start_stop() {
        let mut session = TraceSession::new("test".to_string());
        session.start_recording();
        assert!(session.recording);

        session.stop_recording();
        assert!(!session.recording);
    }

    #[test]
    fn test_trace_entry_addition() {
        let mut session = TraceSession::new("test".to_string());
        session.start_recording();

        let entry = TraceEntry {
            timestamp_ns: 1000,
            event_type: TraceEventType::TaskSpawn,
            task_id: "task1".to_string(),
            details: "Task spawned".to_string(),
            metadata: BTreeMap::new(),
        };

        session.add_entry(entry);
        assert_eq!(session.entries.len(), 1);
    }

    #[test]
    fn test_trace_no_record_when_stopped() {
        let mut session = TraceSession::new("test".to_string());

        let entry = TraceEntry {
            timestamp_ns: 1000,
            event_type: TraceEventType::TaskSpawn,
            task_id: "task1".to_string(),
            details: "Task spawned".to_string(),
            metadata: BTreeMap::new(),
        };

        session.add_entry(entry);
        assert_eq!(session.entries.len(), 0);
    }

    #[test]
    fn test_trace_filter_all() {
        let filter = TraceFilter::all();
        let entry = TraceEntry {
            timestamp_ns: 1000,
            event_type: TraceEventType::TaskSpawn,
            task_id: "task1".to_string(),
            details: "Test".to_string(),
            metadata: BTreeMap::new(),
        };
        assert!(filter.matches(&entry));
    }

    #[test]
    fn test_trace_filter_by_type() {
        let mut filter = TraceFilter::all();
        filter.event_types = vec![TraceEventType::TaskSpawn];

        let entry1 = TraceEntry {
            timestamp_ns: 1000,
            event_type: TraceEventType::TaskSpawn,
            task_id: "task1".to_string(),
            details: "Test".to_string(),
            metadata: BTreeMap::new(),
        };

        let entry2 = TraceEntry {
            timestamp_ns: 1000,
            event_type: TraceEventType::TaskYield,
            task_id: "task1".to_string(),
            details: "Test".to_string(),
            metadata: BTreeMap::new(),
        };

        assert!(filter.matches(&entry1));
        assert!(!filter.matches(&entry2));
    }

    #[test]
    fn test_trace_filter_by_task_id() {
        let mut filter = TraceFilter::all();
        filter.task_id_pattern = Some("task1".to_string());

        let entry1 = TraceEntry {
            timestamp_ns: 1000,
            event_type: TraceEventType::TaskSpawn,
            task_id: "task1".to_string(),
            details: "Test".to_string(),
            metadata: BTreeMap::new(),
        };

        let entry2 = TraceEntry {
            timestamp_ns: 1000,
            event_type: TraceEventType::TaskSpawn,
            task_id: "task2".to_string(),
            details: "Test".to_string(),
            metadata: BTreeMap::new(),
        };

        assert!(filter.matches(&entry1));
        assert!(!filter.matches(&entry2));
    }

    #[test]
    fn test_trace_filter_by_timestamp() {
        let mut filter = TraceFilter::all();
        filter.min_timestamp_ns = Some(1000);
        filter.max_timestamp_ns = Some(2000);

        let entry1 = TraceEntry {
            timestamp_ns: 1500,
            event_type: TraceEventType::TaskSpawn,
            task_id: "task1".to_string(),
            details: "Test".to_string(),
            metadata: BTreeMap::new(),
        };

        let entry2 = TraceEntry {
            timestamp_ns: 3000,
            event_type: TraceEventType::TaskSpawn,
            task_id: "task1".to_string(),
            details: "Test".to_string(),
            metadata: BTreeMap::new(),
        };

        assert!(filter.matches(&entry1));
        assert!(!filter.matches(&entry2));
    }

    #[test]
    fn test_trace_format_text() {
        let mut session = TraceSession::new("test".to_string());
        session.start_recording();

        let entry = TraceEntry {
            timestamp_ns: 1000,
            event_type: TraceEventType::TaskSpawn,
            task_id: "task1".to_string(),
            details: "Task spawned".to_string(),
            metadata: BTreeMap::new(),
        };

        session.add_entry(entry);
        let text = session.format_text();
        assert_eq!(text.len(), 1);
        assert!(text[0].contains("task1"));
    }

    #[test]
    fn test_trace_get_stats() {
        let mut session = TraceSession::new("test".to_string());
        session.start_recording();

        let entry = TraceEntry {
            timestamp_ns: 1000,
            event_type: TraceEventType::TaskSpawn,
            task_id: "task1".to_string(),
            details: "Test".to_string(),
            metadata: BTreeMap::new(),
        };

        session.add_entry(entry);
        let stats = session.get_stats();
        assert_eq!(stats.get("total_entries"), Some(&"1".to_string()));
    }
}
