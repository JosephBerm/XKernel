//! CEFEventIntegration: CEF event generation at adapter boundary with comprehensive event support.
//!
//! This module provides Common Event Format (CEF) event generation for the runtime adapter
//! boundary, emitting events on:
//! - Adapter lifecycle events (load, shutdown)
//! - Agent lifecycle events (load, configuration)
//! - State transitions
//! - Syscall invocation
//! - Error occurrence
//! - Configuration changes
//!
//! Per Week 6, Section 5: "CEF event generation at adapter boundary"

use crate::error::AdapterError;
use crate::AdapterResult;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap as HashMap;

/// CEF event header fields
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CefHeader {
    pub version: u32,
    pub device_vendor: String,
    pub device_product: String,
    pub device_version: String,
    pub signature_id: String,
    pub name: String,
    pub severity: String,
}

impl CefHeader {
    /// Create a new CEF header
    pub fn new(signature_id: String, name: String, severity: String) -> Self {
        CefHeader {
            version: 0,
            device_vendor: "CognitiveSubstrate".to_string(),
            device_product: "RuntimeAdapter".to_string(),
            device_version: "1.0".to_string(),
            signature_id,
            name,
            severity,
        }
    }
}

/// CEF event with extensions
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CefEvent {
    pub header: CefHeader,
    pub extensions: HashMap<String, String>,
    pub timestamp: u64,
}

impl CefEvent {
    /// Create a new CEF event
    pub fn new(header: CefHeader) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        CefEvent {
            header,
            extensions: HashMap::new(),
            timestamp,
        }
    }

    /// Add an extension field
    pub fn with_extension(mut self, key: String, value: String) -> Self {
        self.extensions.insert(key, value);
        self
    }

    /// Format as CEF string
    pub fn to_cef_string(&self) -> String {
        let header_str = format!(
            "CEF:{}|{}|{}|{}|{}|{}|{}",
            self.header.version,
            self.header.device_vendor,
            self.header.device_product,
            self.header.device_version,
            self.header.signature_id,
            self.header.name,
            self.header.severity,
        );

        let ext_str = self.extensions
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join(" ");

        if ext_str.is_empty() {
            header_str
        } else {
            format!("{} {}", header_str, ext_str)
        }
    }
}

/// Event type enumeration for adapter events
#[derive(Clone, Debug, PartialEq)]
pub enum EventType {
    AdapterLoaded,
    AdapterShutdown,
    AgentLoaded,
    AgentConfigured,
    StateTransition,
    SyscallInvoked,
    SyscallCompleted,
    ErrorOccurred,
    ConfigurationChanged,
    MemoryOperation,
    TaskSpawned,
    ChannelCreated,
    CapabilityGranted,
}

impl EventType {
    /// Get signature ID for event type
    pub fn signature_id(&self) -> String {
        match self {
            EventType::AdapterLoaded => "ADAPTER_LOADED".to_string(),
            EventType::AdapterShutdown => "ADAPTER_SHUTDOWN".to_string(),
            EventType::AgentLoaded => "AGENT_LOADED".to_string(),
            EventType::AgentConfigured => "AGENT_CONFIGURED".to_string(),
            EventType::StateTransition => "STATE_TRANSITION".to_string(),
            EventType::SyscallInvoked => "SYSCALL_INVOKED".to_string(),
            EventType::SyscallCompleted => "SYSCALL_COMPLETED".to_string(),
            EventType::ErrorOccurred => "ERROR_OCCURRED".to_string(),
            EventType::ConfigurationChanged => "CONFIG_CHANGED".to_string(),
            EventType::MemoryOperation => "MEMORY_OP".to_string(),
            EventType::TaskSpawned => "TASK_SPAWNED".to_string(),
            EventType::ChannelCreated => "CHANNEL_CREATED".to_string(),
            EventType::CapabilityGranted => "CAPABILITY_GRANTED".to_string(),
        }
    }

    /// Get severity level for event type
    pub fn severity(&self) -> String {
        match self {
            EventType::AdapterLoaded | EventType::AgentLoaded | EventType::SyscallCompleted => {
                "5".to_string() // Medium
            }
            EventType::ErrorOccurred => "8".to_string(), // High
            EventType::StateTransition => "4".to_string(), // Low
            _ => "5".to_string(), // Medium
        }
    }

    /// Get event name
    pub fn name(&self) -> String {
        format!("{:?}", self)
    }
}

/// Event emitter for CEF events
pub struct CefEventEmitter {
    subscribers: Arc<Mutex<Vec<Box<dyn CefEventSubscriber>>>>,
    event_log: Arc<Mutex<Vec<CefEvent>>>,
}

pub trait CefEventSubscriber: Send {
    /// Handle a CEF event
    fn on_event(&self, event: &CefEvent) -> AdapterResult<()>;
}

impl CefEventEmitter {
    /// Create a new CEF event emitter
    pub fn new() -> Self {
        CefEventEmitter {
            subscribers: Arc::new(Mutex::new(Vec::new())),
            event_log: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Emit a CEF event
    pub fn emit(&self, event: CefEvent) -> AdapterResult<()> {
        // Log the event
        {
            let mut log = self.event_log.lock()
                .map_err(|_| AdapterError::LockError("Failed to acquire event log lock".to_string()))?;
            log.push(event.clone());
        }

        // Notify subscribers
        {
            let subscribers = self.subscribers.lock()
                .map_err(|_| AdapterError::LockError("Failed to acquire subscribers lock".to_string()))?;
            for subscriber in subscribers.iter() {
                let _ = subscriber.on_event(&event);
            }
        }

        Ok(())
    }

    /// Subscribe to events
    pub fn subscribe(&self, subscriber: Box<dyn CefEventSubscriber>) -> AdapterResult<()> {
        let mut subscribers = self.subscribers.lock()
            .map_err(|_| AdapterError::LockError("Failed to acquire subscribers lock".to_string()))?;
        subscribers.push(subscriber);
        Ok(())
    }

    /// Get event log
    pub fn get_event_log(&self) -> AdapterResult<Vec<CefEvent>> {
        let log = self.event_log.lock()
            .map_err(|_| AdapterError::LockError("Failed to acquire event log lock".to_string()))?;
        Ok(log.clone())
    }

    /// Get events by type
    pub fn get_events_by_type(&self, event_type: &str) -> AdapterResult<Vec<CefEvent>> {
        let log = self.event_log.lock()
            .map_err(|_| AdapterError::LockError("Failed to acquire event log lock".to_string()))?;

        let filtered = log.iter()
            .filter(|e| e.header.signature_id == event_type)
            .cloned()
            .collect();

        Ok(filtered)
    }

    /// Clear event log
    pub fn clear_event_log(&self) -> AdapterResult<()> {
        let mut log = self.event_log.lock()
            .map_err(|_| AdapterError::LockError("Failed to acquire event log lock".to_string()))?;
        log.clear();
        Ok(())
    }

    /// Get subscriber count
    pub fn subscriber_count(&self) -> AdapterResult<usize> {
        let subscribers = self.subscribers.lock()
            .map_err(|_| AdapterError::LockError("Failed to acquire subscribers lock".to_string()))?;
        Ok(subscribers.len())
    }
}

/// CEF event factory for creating adapter events
pub struct CefEventFactory;

impl CefEventFactory {
    /// Create adapter loaded event
    /// Per Week 6, Section 5: "Emit events on: adapter load"
    pub fn adapter_loaded(adapter_name: &str, framework: &str) -> CefEvent {
        let event_type = EventType::AdapterLoaded;
        let header = CefHeader::new(
            event_type.signature_id(),
            event_type.name(),
            event_type.severity(),
        );

        CefEvent::new(header)
            .with_extension("adapter".to_string(), adapter_name.to_string())
            .with_extension("framework".to_string(), framework.to_string())
            .with_extension("status".to_string(), "loaded".to_string())
    }

    /// Create adapter shutdown event
    pub fn adapter_shutdown(adapter_name: &str) -> CefEvent {
        let event_type = EventType::AdapterShutdown;
        let header = CefHeader::new(
            event_type.signature_id(),
            event_type.name(),
            event_type.severity(),
        );

        CefEvent::new(header)
            .with_extension("adapter".to_string(), adapter_name.to_string())
            .with_extension("status".to_string(), "shutdown".to_string())
    }

    /// Create agent loaded event
    /// Per Week 6, Section 5: "Emit events on: agent load"
    pub fn agent_loaded(agent_id: &str, agent_type: &str) -> CefEvent {
        let event_type = EventType::AgentLoaded;
        let header = CefHeader::new(
            event_type.signature_id(),
            event_type.name(),
            event_type.severity(),
        );

        CefEvent::new(header)
            .with_extension("agent_id".to_string(), agent_id.to_string())
            .with_extension("agent_type".to_string(), agent_type.to_string())
            .with_extension("status".to_string(), "loaded".to_string())
    }

    /// Create configuration changed event
    /// Per Week 6, Section 5: "Emit events on: configuration change"
    pub fn configuration_changed(adapter_name: &str, config_key: &str, new_value: &str) -> CefEvent {
        let event_type = EventType::ConfigurationChanged;
        let header = CefHeader::new(
            event_type.signature_id(),
            event_type.name(),
            event_type.severity(),
        );

        CefEvent::new(header)
            .with_extension("adapter".to_string(), adapter_name.to_string())
            .with_extension("config_key".to_string(), config_key.to_string())
            .with_extension("new_value".to_string(), new_value.to_string())
    }

    /// Create state transition event
    /// Per Week 6, Section 5: "Emit events on: state transition"
    pub fn state_transition(adapter_name: &str, from_state: &str, to_state: &str) -> CefEvent {
        let event_type = EventType::StateTransition;
        let header = CefHeader::new(
            event_type.signature_id(),
            event_type.name(),
            event_type.severity(),
        );

        CefEvent::new(header)
            .with_extension("adapter".to_string(), adapter_name.to_string())
            .with_extension("from_state".to_string(), from_state.to_string())
            .with_extension("to_state".to_string(), to_state.to_string())
    }

    /// Create syscall invoked event
    /// Per Week 6, Section 5: "Emit events on: syscall invocation"
    pub fn syscall_invoked(syscall_id: &str, syscall_group: &str, agent_id: &str) -> CefEvent {
        let event_type = EventType::SyscallInvoked;
        let header = CefHeader::new(
            event_type.signature_id(),
            event_type.name(),
            event_type.severity(),
        );

        CefEvent::new(header)
            .with_extension("syscall_id".to_string(), syscall_id.to_string())
            .with_extension("syscall_group".to_string(), syscall_group.to_string())
            .with_extension("agent_id".to_string(), agent_id.to_string())
    }

    /// Create syscall completed event
    pub fn syscall_completed(syscall_id: &str, status: &str, duration_ms: u64) -> CefEvent {
        let event_type = EventType::SyscallCompleted;
        let header = CefHeader::new(
            event_type.signature_id(),
            event_type.name(),
            event_type.severity(),
        );

        CefEvent::new(header)
            .with_extension("syscall_id".to_string(), syscall_id.to_string())
            .with_extension("status".to_string(), status.to_string())
            .with_extension("duration_ms".to_string(), duration_ms.to_string())
    }

    /// Create error occurred event
    /// Per Week 6, Section 5: "Emit events on: error occurrence"
    pub fn error_occurred(error_type: &str, error_message: &str, severity_level: &str) -> CefEvent {
        let event_type = EventType::ErrorOccurred;
        let header = CefHeader::new(
            event_type.signature_id(),
            event_type.name(),
            event_type.severity(),
        );

        CefEvent::new(header)
            .with_extension("error_type".to_string(), error_type.to_string())
            .with_extension("error_message".to_string(), error_message.to_string())
            .with_extension("severity_level".to_string(), severity_level.to_string())
    }

    /// Create memory operation event
    pub fn memory_operation(operation: &str, size: u64, address: u64) -> CefEvent {
        let event_type = EventType::MemoryOperation;
        let header = CefHeader::new(
            event_type.signature_id(),
            event_type.name(),
            event_type.severity(),
        );

        CefEvent::new(header)
            .with_extension("operation".to_string(), operation.to_string())
            .with_extension("size".to_string(), size.to_string())
            .with_extension("address".to_string(), address.to_string())
    }

    /// Create task spawned event
    pub fn task_spawned(task_id: u64, entry_point: &str) -> CefEvent {
        let event_type = EventType::TaskSpawned;
        let header = CefHeader::new(
            event_type.signature_id(),
            event_type.name(),
            event_type.severity(),
        );

        CefEvent::new(header)
            .with_extension("task_id".to_string(), task_id.to_string())
            .with_extension("entry_point".to_string(), entry_point.to_string())
    }

    /// Create channel created event
    pub fn channel_created(channel_id: u64, channel_type: &str) -> CefEvent {
        let event_type = EventType::ChannelCreated;
        let header = CefHeader::new(
            event_type.signature_id(),
            event_type.name(),
            event_type.severity(),
        );

        CefEvent::new(header)
            .with_extension("channel_id".to_string(), channel_id.to_string())
            .with_extension("channel_type".to_string(), channel_type.to_string())
    }

    /// Create capability granted event
    pub fn capability_granted(agent_id: &str, capability: &str) -> CefEvent {
        let event_type = EventType::CapabilityGranted;
        let header = CefHeader::new(
            event_type.signature_id(),
            event_type.name(),
            event_type.severity(),
        );

        CefEvent::new(header)
            .with_extension("agent_id".to_string(), agent_id.to_string())
            .with_extension("capability".to_string(), capability.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use std::sync::Arc;

    #[test]
    fn test_cef_event_creation() {
        let header = CefHeader::new(
            "TEST_EVENT".to_string(),
            "Test Event".to_string(),
            "5".to_string(),
        );

        let event = CefEvent::new(header);
        assert_eq!(event.header.signature_id, "TEST_EVENT");
    }

    #[test]
    fn test_cef_event_extensions() {
        let header = CefHeader::new(
            "TEST".to_string(),
            "Test".to_string(),
            "5".to_string(),
        );

        let event = CefEvent::new(header)
            .with_extension("key1".to_string(), "value1".to_string())
            .with_extension("key2".to_string(), "value2".to_string());

        assert_eq!(event.extensions.len(), 2);
        assert_eq!(event.extensions.get("key1"), Some(&"value1".to_string()));
    }

    #[test]
    fn test_cef_to_string() {
        let header = CefHeader::new(
            "TEST".to_string(),
            "TestEvent".to_string(),
            "5".to_string(),
        );

        let event = CefEvent::new(header)
            .with_extension("test".to_string(), "value".to_string());

        let cef_str = event.to_cef_string();
        assert!(cef_str.starts_with("CEF:"));
        assert!(cef_str.contains("test=value"));
    }

    #[test]
    fn test_event_emitter() -> AdapterResult<()> {
        let emitter = CefEventEmitter::new();

        let event = CefEventFactory::adapter_loaded("test_adapter", "langchain");
        emitter.emit(event)?;

        let log = emitter.get_event_log()?;
        assert_eq!(log.len(), 1);

        Ok(())
    }

    #[test]
    fn test_event_factory_adapter_loaded() {
        let event = CefEventFactory::adapter_loaded("my_adapter", "semantic_kernel");
        assert_eq!(event.header.signature_id, "ADAPTER_LOADED");
        assert_eq!(event.extensions.get("adapter"), Some(&"my_adapter".to_string()));
    }

    #[test]
    fn test_event_factory_error_occurred() {
        let event = CefEventFactory::error_occurred(
            "ValidationError",
            "Invalid configuration",
            "high",
        );
        assert_eq!(event.header.signature_id, "ERROR_OCCURRED");
    }

    #[test]
    fn test_event_factory_syscall_invoked() {
        let event = CefEventFactory::syscall_invoked("mem_alloc", "memory", "agent1");
        assert_eq!(event.header.signature_id, "SYSCALL_INVOKED");
        assert_eq!(
            event.extensions.get("syscall_id"),
            Some(&"mem_alloc".to_string())
        );
    }

    #[test]
    fn test_get_events_by_type() -> AdapterResult<()> {
        let emitter = CefEventEmitter::new();

        let event1 = CefEventFactory::adapter_loaded("adapter1", "fw1");
        let event2 = CefEventFactory::error_occurred("Error", "msg", "high");
        let event3 = CefEventFactory::adapter_loaded("adapter2", "fw2");

        emitter.emit(event1)?;
        emitter.emit(event2)?;
        emitter.emit(event3)?;

        let adapter_events = emitter.get_events_by_type("ADAPTER_LOADED")?;
        assert_eq!(adapter_events.len(), 2);

        let error_events = emitter.get_events_by_type("ERROR_OCCURRED")?;
        assert_eq!(error_events.len(), 1);

        Ok(())
    }

    #[test]
    fn test_event_subscriber_count() -> AdapterResult<()> {
        let emitter = CefEventEmitter::new();
        assert_eq!(emitter.subscriber_count()?, 0);

        Ok(())
    }

    #[test]
    fn test_event_log_clearing() -> AdapterResult<()> {
        let emitter = CefEventEmitter::new();

        let event = CefEventFactory::adapter_loaded("test", "fw");
        emitter.emit(event)?;

        let log = emitter.get_event_log()?;
        assert_eq!(log.len(), 1);

        emitter.clear_event_log()?;
        let cleared = emitter.get_event_log()?;
        assert_eq!(cleared.len(), 0);

        Ok(())
    }

    #[test]
    fn test_event_types_severity() {
        assert_eq!(EventType::AdapterLoaded.severity(), "5"); // Medium
        assert_eq!(EventType::ErrorOccurred.severity(), "8"); // High
        assert_eq!(EventType::StateTransition.severity(), "4"); // Low
    }
}
