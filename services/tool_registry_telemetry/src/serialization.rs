// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Event serialization and deserialization.
//!
//! Provides serialization support for CEF events in multiple formats
//! (JSON, Protobuf, Cap'n Proto, Parquet) aligned with OpenTelemetry standards.
//!
//! See Engineering Plan § 2.12: CEF Event Structure
//! and Addendum v2.5.1: Serialization Requirements.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use crate::cef::{CefEvent, CefEventType};
use crate::error::Result;

/// Serialization format enumeration.
///
/// See Engineering Plan § 2.12: Serialization.
/// Specifies the wire format for event serialization.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SerializationFormat {
    /// JSON format (human-readable).
    Json,

    /// Protocol Buffers format (compact binary).
    Protobuf,

    /// Cap'n Proto format (zero-copy binary).
    CapnProto,

    /// Apache Parquet columnar format (analytical).
    Parquet,
}

impl fmt::Display for SerializationFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SerializationFormat::Json => write!(f, "JSON"),
            SerializationFormat::Protobuf => write!(f, "Protobuf"),
            SerializationFormat::CapnProto => write!(f, "Cap'n Proto"),
            SerializationFormat::Parquet => write!(f, "Parquet"),
        }
    }
}

/// Trait for serializing CEF events.
///
/// See Engineering Plan § 2.12: Serialization.
pub trait EventSerializer {
    /// Serializes a CEF event to bytes in the implementation-specific format.
    fn serialize(&self, event: &CefEvent) -> Result<Vec<u8>>;

    /// Returns the format this serializer uses.
    fn format(&self) -> SerializationFormat;

    /// Returns an estimated size of a serialized event in bytes.
    fn estimated_size(&self, event_type: CefEventType) -> usize {
        // Default estimates based on event type
        match event_type {
            CefEventType::ThoughtStep => 512,
            CefEventType::ToolCallRequested => 256,
            CefEventType::ToolCallCompleted => 512,
            CefEventType::PolicyDecision => 256,
            CefEventType::MemoryAccess => 128,
            CefEventType::IpcMessage => 256,
            CefEventType::PhaseTransition => 128,
            CefEventType::CheckpointCreated => 256,
            CefEventType::SignalDispatched => 128,
            CefEventType::ExceptionRaised => 384,
        }
    }
}

/// Trait for deserializing CEF events.
///
/// See Engineering Plan § 2.12: Deserialization.
pub trait EventDeserializer {
    /// Deserializes bytes into a CEF event in the implementation-specific format.
    fn deserialize(&self, data: &[u8]) -> Result<CefEvent>;

    /// Returns the format this deserializer expects.
    fn format(&self) -> SerializationFormat;
}

/// JSON event serializer (stub).
///
/// See Engineering Plan § 2.12: JSON Serialization.
#[derive(Clone, Debug)]
pub struct JsonEventSerializer;

impl JsonEventSerializer {
    /// Creates a new JSON event serializer.
    pub fn new() -> Self {
        JsonEventSerializer
    }
}

impl Default for JsonEventSerializer {
    fn default() -> Self {
        Self::new()
    }
}

impl EventSerializer for JsonEventSerializer {
    fn serialize(&self, event: &CefEvent) -> Result<Vec<u8>> {
        // Stub: Would use serde_json in production
        // For no_std environment, would need custom JSON builder
        let json_str = alloc::format!(
            r#"{{"event_id":"{}","trace_id":"{}","span_id":"{}","ct_id":"{}","agent_id":"{}","timestamp_ns":{},"event_type":"{}","phase":"{}","cost":{{"tokens":{},"gpu_ms":{},"wall_clock_ms":{},"tpc_hours":{}}},"data_classification":"{}"}}"#,
            event.event_id,
            event.trace_id,
            event.span_id,
            event.ct_id,
            event.agent_id,
            event.timestamp_ns,
            event.event_type,
            event.phase,
            event.cost.tokens,
            event.cost.gpu_ms,
            event.cost.wall_clock_ms,
            event.cost.tpc_hours,
            event.data_classification
        );
        Ok(json_str.into_bytes())
    }

    fn format(&self) -> SerializationFormat {
        SerializationFormat::Json
    }

    fn estimated_size(&self, event_type: CefEventType) -> usize {
        // JSON is larger than binary formats
        let base = EventSerializer::estimated_size(self, event_type);
        (base as f64 * 1.5) as usize
    }
}

/// Protobuf event serializer (stub).
///
/// See Engineering Plan § 2.12: Protobuf Serialization.
#[derive(Clone, Debug)]
pub struct ProtobufEventSerializer;

impl ProtobufEventSerializer {
    /// Creates a new Protobuf event serializer.
    pub fn new() -> Self {
        ProtobufEventSerializer
    }
}

impl Default for ProtobufEventSerializer {
    fn default() -> Self {
        Self::new()
    }
}

impl EventSerializer for ProtobufEventSerializer {
    fn serialize(&self, _event: &CefEvent) -> Result<Vec<u8>> {
        // Stub: Would use prost or protobuf crate in production
        // For now, return minimal protobuf varint encoding
        Ok(Vec::new())
    }

    fn format(&self) -> SerializationFormat {
        SerializationFormat::Protobuf
    }
}

/// Cap'n Proto event serializer (stub).
///
/// See Engineering Plan § 2.12: Cap'n Proto Serialization.
#[derive(Clone, Debug)]
pub struct CapnProtoEventSerializer;

impl CapnProtoEventSerializer {
    /// Creates a new Cap'n Proto event serializer.
    pub fn new() -> Self {
        CapnProtoEventSerializer
    }
}

impl Default for CapnProtoEventSerializer {
    fn default() -> Self {
        Self::new()
    }
}

impl EventSerializer for CapnProtoEventSerializer {
    fn serialize(&self, _event: &CefEvent) -> Result<Vec<u8>> {
        // Stub: Would use capnp crate in production
        Ok(Vec::new())
    }

    fn format(&self) -> SerializationFormat {
        SerializationFormat::CapnProto
    }

    fn estimated_size(&self, event_type: CefEventType) -> usize {
        // Cap'n Proto is very compact
        let base = EventSerializer::estimated_size(self, event_type);
        (base as f64 * 0.7) as usize
    }
}

/// Parquet event serializer (stub).
///
/// See Engineering Plan § 2.12: Parquet Serialization.
/// For batch processing and analytical queries.
#[derive(Clone, Debug)]
pub struct ParquetEventSerializer;

impl ParquetEventSerializer {
    /// Creates a new Parquet event serializer.
    pub fn new() -> Self {
        ParquetEventSerializer
    }
}

impl Default for ParquetEventSerializer {
    fn default() -> Self {
        Self::new()
    }
}

impl EventSerializer for ParquetEventSerializer {
    fn serialize(&self, _event: &CefEvent) -> Result<Vec<u8>> {
        // Stub: Would use parquet crate in production
        Ok(Vec::new())
    }

    fn format(&self) -> SerializationFormat {
        SerializationFormat::Parquet
    }

    fn estimated_size(&self, event_type: CefEventType) -> usize {
        // Parquet has columnar overhead but excellent compression
        let base = EventSerializer::estimated_size(self, event_type);
        (base as f64 * 0.4) as usize
    }
}

/// Serialization format selector.
///
/// Provides factory for creating appropriate serializers.
pub struct SerializerFactory;

impl SerializerFactory {
    /// Creates a serializer for the specified format.
    pub fn create(format: SerializationFormat) -> Box<dyn EventSerializer> {
        match format {
            SerializationFormat::Json => Box::new(JsonEventSerializer::new()),
            SerializationFormat::Protobuf => Box::new(ProtobufEventSerializer::new()),
            SerializationFormat::CapnProto => Box::new(CapnProtoEventSerializer::new()),
            SerializationFormat::Parquet => Box::new(ParquetEventSerializer::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cef::{CostAttribution, DataClassification};
use alloc::boxed::Box;
use alloc::format;
use alloc::string::ToString;
use alloc::vec;

    #[test]
    fn test_serialization_format_display() {
        assert_eq!(SerializationFormat::Json.to_string(), "JSON");
        assert_eq!(SerializationFormat::Protobuf.to_string(), "Protobuf");
        assert_eq!(SerializationFormat::CapnProto.to_string(), "Cap'n Proto");
        assert_eq!(SerializationFormat::Parquet.to_string(), "Parquet");
    }

    #[test]
    fn test_serialization_format_equality() {
        assert_eq!(SerializationFormat::Json, SerializationFormat::Json);
        assert_ne!(SerializationFormat::Json, SerializationFormat::Protobuf);
    }

    #[test]
    fn test_json_serializer_creation() {
        let serializer = JsonEventSerializer::new();
        assert_eq!(serializer.format(), SerializationFormat::Json);
    }

    #[test]
    fn test_json_serializer_default() {
        let serializer = JsonEventSerializer::default();
        assert_eq!(serializer.format(), SerializationFormat::Json);
    }

    #[test]
    fn test_json_serializer_serialize() {
        let serializer = JsonEventSerializer::new();
        let event = CefEvent::new(
            "event-001",
            "trace-001",
            "span-001",
            "ct-001",
            "agent-001",
            1000,
            CefEventType::ThoughtStep,
            "thinking",
        );

        let result = serializer.serialize(&event);
        assert!(result.is_ok());

        let bytes = result.unwrap();
        assert!(!bytes.is_empty());

        // Verify it contains expected JSON fields
        let json_str = alloc::string::String::from_utf8(bytes).unwrap();
        assert!(json_str.contains("event_id"));
        assert!(json_str.contains("trace_id"));
        assert!(json_str.contains("ThoughtStep"));
    }

    #[test]
    fn test_json_serializer_estimated_size() {
        let serializer = JsonEventSerializer::new();
        let size_thought = serializer.estimated_size(CefEventType::ThoughtStep);
        let size_exception = serializer.estimated_size(CefEventType::ExceptionRaised);

        // JSON should estimate larger sizes
        assert!(size_thought > 512);
        assert!(size_exception > 384);
    }

    #[test]
    fn test_protobuf_serializer_creation() {
        let serializer = ProtobufEventSerializer::new();
        assert_eq!(serializer.format(), SerializationFormat::Protobuf);
    }

    #[test]
    fn test_protobuf_serializer_default() {
        let serializer = ProtobufEventSerializer::default();
        assert_eq!(serializer.format(), SerializationFormat::Protobuf);
    }

    #[test]
    fn test_protobuf_serializer_serialize() {
        let serializer = ProtobufEventSerializer::new();
        let event = CefEvent::new(
            "event-001",
            "trace-001",
            "span-001",
            "ct-001",
            "agent-001",
            1000,
            CefEventType::ToolCallRequested,
            "acting",
        );

        let result = serializer.serialize(&event);
        assert!(result.is_ok());
    }

    #[test]
    fn test_capnproto_serializer_creation() {
        let serializer = CapnProtoEventSerializer::new();
        assert_eq!(serializer.format(), SerializationFormat::CapnProto);
    }

    #[test]
    fn test_capnproto_serializer_default() {
        let serializer = CapnProtoEventSerializer::default();
        assert_eq!(serializer.format(), SerializationFormat::CapnProto);
    }

    #[test]
    fn test_capnproto_serializer_estimated_size() {
        let serializer = CapnProtoEventSerializer::new();
        let size_thought = serializer.estimated_size(CefEventType::ThoughtStep);

        // Cap'n Proto should estimate smaller sizes (70% of base)
        assert!(size_thought < 512);
    }

    #[test]
    fn test_parquet_serializer_creation() {
        let serializer = ParquetEventSerializer::new();
        assert_eq!(serializer.format(), SerializationFormat::Parquet);
    }

    #[test]
    fn test_parquet_serializer_default() {
        let serializer = ParquetEventSerializer::default();
        assert_eq!(serializer.format(), SerializationFormat::Parquet);
    }

    #[test]
    fn test_parquet_serializer_estimated_size() {
        let serializer = ParquetEventSerializer::new();
        let size_thought = serializer.estimated_size(CefEventType::ThoughtStep);

        // Parquet should estimate very small sizes (40% of base)
        assert!(size_thought < 256);
    }

    #[test]
    fn test_serializer_factory_json() {
        let serializer = SerializerFactory::create(SerializationFormat::Json);
        assert_eq!(serializer.format(), SerializationFormat::Json);
    }

    #[test]
    fn test_serializer_factory_protobuf() {
        let serializer = SerializerFactory::create(SerializationFormat::Protobuf);
        assert_eq!(serializer.format(), SerializationFormat::Protobuf);
    }

    #[test]
    fn test_serializer_factory_capnproto() {
        let serializer = SerializerFactory::create(SerializationFormat::CapnProto);
        assert_eq!(serializer.format(), SerializationFormat::CapnProto);
    }

    #[test]
    fn test_serializer_factory_parquet() {
        let serializer = SerializerFactory::create(SerializationFormat::Parquet);
        assert_eq!(serializer.format(), SerializationFormat::Parquet);
    }

    #[test]
    fn test_json_serialize_with_cost() {
        let serializer = JsonEventSerializer::new();
        let cost = CostAttribution::new(500, 100, 50, 2);
        let event = CefEvent::new(
            "event-001",
            "trace-001",
            "span-001",
            "ct-001",
            "agent-001",
            1000,
            CefEventType::ToolCallCompleted,
            "acting",
        )
        .with_cost(cost);

        let result = serializer.serialize(&event);
        assert!(result.is_ok());

        let json_str = alloc::string::String::from_utf8(result.unwrap()).unwrap();
        assert!(json_str.contains("500")); // tokens
        assert!(json_str.contains("100")); // gpu_ms
    }

    #[test]
    fn test_json_serialize_with_crew() {
        let serializer = JsonEventSerializer::new();
        let event = CefEvent::new(
            "event-001",
            "trace-001",
            "span-001",
            "ct-001",
            "agent-001",
            1000,
            CefEventType::PolicyDecision,
            "acting",
        )
        .with_crew("crew-001");

        let result = serializer.serialize(&event);
        assert!(result.is_ok());
    }

    #[test]
    fn test_estimated_sizes_hierarchy() {
        let json_ser = JsonEventSerializer::new();
        let protobuf_ser = ProtobufEventSerializer::new();
        let capnproto_ser = CapnProtoEventSerializer::new();
        let parquet_ser = ParquetEventSerializer::new();

        let event_type = CefEventType::ThoughtStep;

        let json_size = json_ser.estimated_size(event_type);
        let protobuf_size = protobuf_ser.estimated_size(event_type);
        let capnproto_size = capnproto_ser.estimated_size(event_type);
        let parquet_size = parquet_ser.estimated_size(event_type);

        // JSON > Protobuf >= Cap'n Proto > Parquet
        assert!(json_size > protobuf_size);
        assert!(protobuf_size >= capnproto_size);
        assert!(capnproto_size > parquet_size);
    }

    #[test]
    fn test_all_event_types_serializable() {
        let serializer = JsonEventSerializer::new();

        let event_types = vec![
            CefEventType::ThoughtStep,
            CefEventType::ToolCallRequested,
            CefEventType::ToolCallCompleted,
            CefEventType::PolicyDecision,
            CefEventType::MemoryAccess,
            CefEventType::IpcMessage,
            CefEventType::PhaseTransition,
            CefEventType::CheckpointCreated,
            CefEventType::SignalDispatched,
            CefEventType::ExceptionRaised,
        ];

        for event_type in event_types {
            let event = CefEvent::new(
                "event-001",
                "trace-001",
                "span-001",
                "ct-001",
                "agent-001",
                1000,
                event_type,
                "phase",
            );

            let result = serializer.serialize(&event);
            assert!(result.is_ok(), "Failed to serialize {:?}", event_type);
        }
    }

    #[test]
    fn test_serializer_clone() {
        let ser1 = JsonEventSerializer::new();
        let ser2 = ser1.clone();
        assert_eq!(ser1.format(), ser2.format());
    }
}
