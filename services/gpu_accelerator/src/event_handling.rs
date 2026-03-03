// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! GPU event handling and notification system.
//!
//! Implements event dispatch for GPU completion notifications, synchronization,
//! and error handling. Events flow from GPU hardware → EventDispatcher → registered callbacks.
//!
//! Reference: Engineering Plan § Event Handling, Error Propagation

use crate::error::GpuError;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::fmt;

/// Unique identifier for a GPU event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EventId(u64);

impl fmt::Display for EventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EventId({})", self.0)
    }
}

/// Stream handle reference for GPU streams.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StreamHandle(u64);

/// GPU event type classification.
///
/// Categorizes different GPU events for routing to appropriate handlers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GpuEventType {
    /// Kernel execution completed successfully.
    KernelComplete,

    /// Memory transfer (host<->device) completed.
    MemoryTransferComplete,

    /// GPU error occurred (device error, ECC error, etc.).
    ErrorOccurred,

    /// Stream synchronization point reached.
    DeviceSynchronized,
}

impl fmt::Display for GpuEventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GpuEventType::KernelComplete => write!(f, "KernelComplete"),
            GpuEventType::MemoryTransferComplete => write!(f, "MemoryTransferComplete"),
            GpuEventType::ErrorOccurred => write!(f, "ErrorOccurred"),
            GpuEventType::DeviceSynchronized => write!(f, "DeviceSynchronized"),
        }
    }
}

/// GPU event notification.
///
/// Represents a GPU event (completion, error, sync) with associated metadata.
///
/// Reference: Engineering Plan § Event Handling
#[derive(Clone, Copy, Debug)]
pub struct GpuEvent {
    /// Unique event identifier.
    pub event_id: EventId,

    /// Event type classification.
    pub event_type: GpuEventType,

    /// Associated stream (if applicable).
    pub stream: StreamHandle,

    /// Event timestamp (nanoseconds since epoch).
    pub timestamp: u64,

    /// Error code (0 = no error, non-zero = error code).
    pub error_code: u32,
}

impl fmt::Display for GpuEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GpuEvent({}, type={}, stream=0x{:x}, ts={})",
            self.event_id, self.event_type, self.stream.0, self.timestamp
        )
    }
}

/// Unique identifier for a registered callback.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CallbackId(u64);

impl fmt::Display for CallbackId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CallbackId({})", self.0)
    }
}

/// GPU event callback handler.
///
/// Handlers are invoked when matching events are dispatched.
/// This is a trait object in real implementations; for testing,
/// we use a simple function pointer + context pattern.
pub type GpuEventCallback = fn(&GpuEvent) -> Result<(), GpuError>;

/// Event handler registration.
#[derive(Clone, Copy, Debug)]
struct EventHandler {
    callback_id: CallbackId,
    event_type: GpuEventType,
    callback: GpuEventCallback,
}

/// GPU event dispatcher.
///
/// Routes GPU events to registered handlers based on event type.
/// Maintains a registry of callbacks and dispatches incoming events.
///
/// Reference: Engineering Plan § Event Handling
#[derive(Debug)]
pub struct EventDispatcher {
    /// Registered event handlers (event_type -> callbacks).
    handlers: BTreeMap<u64, Vec<EventHandler>>,

    /// Event buffer (for polling).
    event_buffer: Vec<GpuEvent>,

    /// Next callback ID counter.
    next_callback_id: u64,

    /// Next event ID counter.
    next_event_id: u64,

    /// Maximum event buffer size.
    max_buffer_size: u32,
}

impl EventDispatcher {
    /// Create a new event dispatcher.
    ///
    /// # Arguments
    ///
    /// * `max_buffer_size` - Maximum events to buffer before dropping oldest
    pub fn new(max_buffer_size: u32) -> Self {
        EventDispatcher {
            handlers: BTreeMap::new(),
            event_buffer: Vec::new(),
            next_callback_id: 1,
            next_event_id: 1,
            max_buffer_size,
        }
    }

    /// Register a callback for an event type.
    ///
    /// # Arguments
    ///
    /// * `event_type` - Type of events to handle
    /// * `callback` - Callback function
    ///
    /// # Returns
    ///
    /// CallbackId for later unregistration.
    pub fn register_callback(
        &mut self,
        event_type: GpuEventType,
        callback: GpuEventCallback,
    ) -> Result<CallbackId, GpuError> {
        let callback_id = CallbackId(self.next_callback_id);
        self.next_callback_id += 1;

        let handler = EventHandler {
            callback_id,
            event_type,
            callback,
        };

        let event_key = event_type as u64;
        self.handlers.entry(event_key).or_insert_with(Vec::new).push(handler);

        Ok(callback_id)
    }

    /// Unregister a callback.
    ///
    /// # Arguments
    ///
    /// * `event_type` - Event type to unregister from
    /// * `callback_id` - Callback ID to remove
    pub fn unregister_callback(&mut self, event_type: GpuEventType, callback_id: CallbackId) {
        let event_key = event_type as u64;

        if let Some(handlers) = self.handlers.get_mut(&event_key) {
            handlers.retain(|h| h.callback_id != callback_id);
        }
    }

    /// Dispatch a GPU event to registered handlers.
    ///
    /// # Arguments
    ///
    /// * `event` - Event to dispatch
    ///
    /// # Returns
    ///
    /// Ok if all handlers succeed, Err if any handler fails.
    pub fn dispatch(&mut self, event: GpuEvent) -> Result<(), GpuError> {
        // Store event in buffer
        if self.event_buffer.len() >= self.max_buffer_size as usize {
            self.event_buffer.remove(0); // Drop oldest
        }
        self.event_buffer.push(event);

        // Find and invoke handlers
        let event_key = event.event_type as u64;
        if let Some(handlers) = self.handlers.get(&event_key) {
            for handler in handlers.iter() {
                (handler.callback)(&event)?;
            }
        }

        Ok(())
    }

    /// Poll for buffered events without removing them.
    ///
    /// # Returns
    ///
    /// Vector of all buffered events.
    pub fn poll_events(&self) -> Vec<GpuEvent> {
        self.event_buffer.clone()
    }

    /// Clear the event buffer.
    pub fn clear_buffer(&mut self) {
        self.event_buffer.clear();
    }

    /// Get the number of buffered events.
    pub fn buffer_size(&self) -> usize {
        self.event_buffer.len()
    }

    /// Get the number of registered handlers for an event type.
    pub fn handler_count(&self, event_type: GpuEventType) -> usize {
        let event_key = event_type as u64;
        self.handlers.get(&event_key).map(|h| h.len()).unwrap_or(0)
    }
}

/// Create a GPU event.
///
/// Helper function to construct a GpuEvent.
pub fn create_event(
    event_type: GpuEventType,
    stream: StreamHandle,
    timestamp: u64,
    error_code: u32,
    event_id_counter: &mut u64,
) -> GpuEvent {
    let event_id = EventId(*event_id_counter);
    *event_id_counter += 1;

    GpuEvent {
        event_id,
        event_type,
        stream,
        timestamp,
        error_code,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;

    #[test]
    fn test_event_dispatcher_creation() {
        let dispatcher = EventDispatcher::new(100);

        assert_eq!(dispatcher.buffer_size(), 0);
        assert_eq!(dispatcher.handler_count(GpuEventType::KernelComplete), 0);
    }

    #[test]
    fn test_event_dispatcher_register_callback() {
        let mut dispatcher = EventDispatcher::new(100);

        let callback: GpuEventCallback = |_event| Ok(());

        let result = dispatcher.register_callback(GpuEventType::KernelComplete, callback);

        assert!(result.is_ok());
        let callback_id = result.unwrap();
        assert_eq!(dispatcher.handler_count(GpuEventType::KernelComplete), 1);
    }

    #[test]
    fn test_event_dispatcher_unregister_callback() {
        let mut dispatcher = EventDispatcher::new(100);

        let callback: GpuEventCallback = |_event| Ok(());

        let callback_id = dispatcher
            .register_callback(GpuEventType::KernelComplete, callback)
            .unwrap();

        assert_eq!(dispatcher.handler_count(GpuEventType::KernelComplete), 1);

        dispatcher.unregister_callback(GpuEventType::KernelComplete, callback_id);

        assert_eq!(dispatcher.handler_count(GpuEventType::KernelComplete), 0);
    }

    #[test]
    fn test_event_dispatcher_dispatch() {
        let mut dispatcher = EventDispatcher::new(100);

        let callback: GpuEventCallback = |_event| Ok(());

        dispatcher
            .register_callback(GpuEventType::KernelComplete, callback)
            .unwrap();

        let event = GpuEvent {
            event_id: EventId(1),
            event_type: GpuEventType::KernelComplete,
            stream: StreamHandle(0),
            timestamp: 1000,
            error_code: 0,
        };

        let result = dispatcher.dispatch(event);

        assert!(result.is_ok());
        assert_eq!(dispatcher.buffer_size(), 1);
    }

    #[test]
    fn test_event_dispatcher_dispatch_error() {
        let mut dispatcher = EventDispatcher::new(100);

        let callback: GpuEventCallback = |_event| Err(GpuError::DriverError);

        dispatcher
            .register_callback(GpuEventType::ErrorOccurred, callback)
            .unwrap();

        let event = GpuEvent {
            event_id: EventId(1),
            event_type: GpuEventType::ErrorOccurred,
            stream: StreamHandle(0),
            timestamp: 1000,
            error_code: 1,
        };

        let result = dispatcher.dispatch(event);

        assert!(result.is_err());
    }

    #[test]
    fn test_event_dispatcher_poll_events() {
        let mut dispatcher = EventDispatcher::new(100);

        let event1 = GpuEvent {
            event_id: EventId(1),
            event_type: GpuEventType::KernelComplete,
            stream: StreamHandle(0),
            timestamp: 1000,
            error_code: 0,
        };

        let event2 = GpuEvent {
            event_id: EventId(2),
            event_type: GpuEventType::MemoryTransferComplete,
            stream: StreamHandle(1),
            timestamp: 2000,
            error_code: 0,
        };

        dispatcher.dispatch(event1).unwrap();
        dispatcher.dispatch(event2).unwrap();

        let events = dispatcher.poll_events();

        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_id, EventId(1));
        assert_eq!(events[1].event_id, EventId(2));
    }

    #[test]
    fn test_event_dispatcher_buffer_overflow() {
        let mut dispatcher = EventDispatcher::new(2); // Small buffer

        let event1 = GpuEvent {
            event_id: EventId(1),
            event_type: GpuEventType::KernelComplete,
            stream: StreamHandle(0),
            timestamp: 1000,
            error_code: 0,
        };

        let event2 = GpuEvent {
            event_id: EventId(2),
            event_type: GpuEventType::KernelComplete,
            stream: StreamHandle(0),
            timestamp: 2000,
            error_code: 0,
        };

        let event3 = GpuEvent {
            event_id: EventId(3),
            event_type: GpuEventType::KernelComplete,
            stream: StreamHandle(0),
            timestamp: 3000,
            error_code: 0,
        };

        dispatcher.dispatch(event1).unwrap();
        dispatcher.dispatch(event2).unwrap();
        dispatcher.dispatch(event3).unwrap();

        assert_eq!(dispatcher.buffer_size(), 2); // Only keeps last 2
        let events = dispatcher.poll_events();
        assert_eq!(events[0].event_id, EventId(2));
        assert_eq!(events[1].event_id, EventId(3));
    }

    #[test]
    fn test_event_dispatcher_clear_buffer() {
        let mut dispatcher = EventDispatcher::new(100);

        let event = GpuEvent {
            event_id: EventId(1),
            event_type: GpuEventType::KernelComplete,
            stream: StreamHandle(0),
            timestamp: 1000,
            error_code: 0,
        };

        dispatcher.dispatch(event).unwrap();
        assert_eq!(dispatcher.buffer_size(), 1);

        dispatcher.clear_buffer();
        assert_eq!(dispatcher.buffer_size(), 0);
    }

    #[test]
    fn test_gpu_event_type_display() {
        assert_eq!(format!("{}", GpuEventType::KernelComplete), "KernelComplete");
        assert_eq!(
            format!("{}", GpuEventType::MemoryTransferComplete),
            "MemoryTransferComplete"
        );
        assert_eq!(format!("{}", GpuEventType::ErrorOccurred), "ErrorOccurred");
        assert_eq!(format!("{}", GpuEventType::DeviceSynchronized), "DeviceSynchronized");
    }

    #[test]
    fn test_gpu_event_display() {
        let event = GpuEvent {
            event_id: EventId(42),
            event_type: GpuEventType::KernelComplete,
            stream: StreamHandle(7),
            timestamp: 9999,
            error_code: 0,
        };

        let display_str = format!("{}", event);
        assert!(display_str.contains("EventId(42)"));
        assert!(display_str.contains("KernelComplete"));
        assert!(display_str.contains("0x7"));
    }

    #[test]
    fn test_multiple_handlers_same_event() {
        let mut dispatcher = EventDispatcher::new(100);

        let callback1: GpuEventCallback = |_event| Ok(());
        let callback2: GpuEventCallback = |_event| Ok(());

        dispatcher
            .register_callback(GpuEventType::KernelComplete, callback1)
            .unwrap();
        dispatcher
            .register_callback(GpuEventType::KernelComplete, callback2)
            .unwrap();

        assert_eq!(dispatcher.handler_count(GpuEventType::KernelComplete), 2);

        let event = GpuEvent {
            event_id: EventId(1),
            event_type: GpuEventType::KernelComplete,
            stream: StreamHandle(0),
            timestamp: 1000,
            error_code: 0,
        };

        assert!(dispatcher.dispatch(event).is_ok());
    }

    #[test]
    fn test_create_event_helper() {
        let mut event_id_counter = 1u64;

        let event1 = create_event(
            GpuEventType::KernelComplete,
            StreamHandle(0),
            1000,
            0,
            &mut event_id_counter,
        );

        let event2 = create_event(
            GpuEventType::ErrorOccurred,
            StreamHandle(1),
            2000,
            1,
            &mut event_id_counter,
        );

        assert_eq!(event1.event_id, EventId(1));
        assert_eq!(event2.event_id, EventId(2));
        assert_eq!(event_id_counter, 3);
    }
}
