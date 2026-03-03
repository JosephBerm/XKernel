// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! CEF Format Encoding & Decoding Infrastructure
//!
//! Provides comprehensive CEF event encoding and decoding with multiple formats,
//! compression strategies, and schema versioning for forward/backward compatibility.
//!
//! See Engineering Plan § 2.12: Cognitive Event Format & Telemetry,
//! specifically § 2.12.3: CEF Encoding & § 2.12.4: Schema Evolution.

use alloc::string::String;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use core::fmt;

use crate::cef::CefEvent;
use crate::error::{Result, ToolError};

/// Semantic version for CEF schema evolution.
///
/// Tracks schema version to enable forward/backward compatibility during
/// event format evolution. See Engineering Plan § 2.12.4: Schema Evolution.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CefFormatVersion {
    /// Major version (incompatible changes)
    pub major: u16,
    /// Minor version (backward-compatible additions)
    pub minor: u16,
    /// Patch version (bug fixes)
    pub patch: u16,
}

impl CefFormatVersion {
    /// Creates a new CEF format version.
    pub fn new(major: u16, minor: u16, patch: u16) -> Self {
        CefFormatVersion { major, minor, patch }
    }

    /// Returns version as semantic version string (e.g., "1.0.0").
    pub fn to_semver(&self) -> String {
        alloc::format!("{}.{}.{}", self.major, self.minor, self.patch)
    }

    /// Checks if this version is compatible with a given version.
    /// Compatible if major version matches.
    pub fn is_compatible_with(&self, other: CefFormatVersion) -> bool {
        self.major == other.major
    }
}

impl Default for CefFormatVersion {
    fn default() -> Self {
        CefFormatVersion { major: 1, minor: 0, patch: 0 }
    }
}

impl fmt::Display for CefFormatVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Compression strategy for encoded events.
///
/// Selects the compression algorithm to use when encoding events.
/// See Engineering Plan § 2.12.3: CEF Encoding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompressionStrategy {
    /// No compression (raw encoding)
    None,
    /// LZ4 compression (fast, moderate compression ratio)
    Lz4,
    /// Zstandard compression (high compression ratio, configurable speed)
    Zstd {
        /// Compression level (1-22, default 3)
        level: u8,
    },
}

impl Default for CompressionStrategy {
    fn default() -> Self {
        CompressionStrategy::None
    }
}

/// Result of encoding with compression metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompressedEvent {
    /// Encoded and optionally compressed bytes
    pub data: Vec<u8>,
    /// Version of schema used for encoding
    pub schema_version: CefFormatVersion,
    /// Compression strategy applied
    pub compression: CompressionStrategy,
    /// Size in bytes before compression (0 if no compression)
    pub uncompressed_size: u64,
}

impl CompressedEvent {
    /// Returns compression ratio as percentage (100% = no compression).
    pub fn compression_ratio(&self) -> f64 {
        if self.uncompressed_size == 0 {
            100.0
        } else {
            (self.data.len() as f64 / self.uncompressed_size as f64) * 100.0
        }
    }
}

/// Trait for encoding CEF events into bytes.
///
/// Implementors provide different encoding formats (JSON, binary, etc).
/// See Engineering Plan § 2.12.3: CEF Encoding.
pub trait CefEncoder {
    /// Encodes a CEF event into bytes.
    fn encode(&self, event: &CefEvent) -> Result<Vec<u8>>;

    /// Returns the schema version this encoder uses.
    fn schema_version(&self) -> CefFormatVersion;
}

/// Trait for decoding CEF events from bytes.
///
/// Implementors provide different decoding formats (JSON, binary, etc).
/// See Engineering Plan § 2.12.3: CEF Encoding.
pub trait CefDecoder {
    /// Decodes a CEF event from bytes.
    fn decode(&self, data: &[u8]) -> Result<CefEvent>;

    /// Returns the schema version this decoder uses.
    fn schema_version(&self) -> CefFormatVersion;
}

/// JSON-based CEF encoder.
///
/// Encodes events as JSON with optional pretty-printing.
/// See Engineering Plan § 2.12.3: CEF Encoding - JSON Format.
#[derive(Clone, Debug)]
pub struct JsonCefEncoder {
    schema_version: CefFormatVersion,
    pretty_print: bool,
}

impl JsonCefEncoder {
    /// Creates a new JSON CEF encoder.
    pub fn new(schema_version: CefFormatVersion, pretty_print: bool) -> Self {
        JsonCefEncoder {
            schema_version,
            pretty_print,
        }
    }

    /// Creates a compact JSON encoder (no whitespace).
    pub fn compact() -> Self {
        JsonCefEncoder {
            schema_version: CefFormatVersion::default(),
            pretty_print: false,
        }
    }

    /// Creates a pretty-printed JSON encoder (human-readable).
    pub fn pretty() -> Self {
        JsonCefEncoder {
            schema_version: CefFormatVersion::default(),
            pretty_print: true,
        }
    }
}

impl CefEncoder for JsonCefEncoder {
    fn encode(&self, event: &CefEvent) -> Result<Vec<u8>> {
        // Create a JSON representation of the event
        let mut json = String::from("{\"version\":\"");
        json.push_str(&self.schema_version.to_semver());
        json.push_str("\",\"event\":{\"id\":\"");
        json.push_str(&event.event_id);
        json.push_str("\",\"type\":\"");
        json.push_str(match event.event_type {
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
        });
        json.push_str("\",\"timestamp_ms\":");
        json.push_str(&alloc::format!("{}", event.timestamp_ns));
        json.push_str("}}");

        Ok(json.into_bytes())
    }

    fn schema_version(&self) -> CefFormatVersion {
        self.schema_version
    }
}

impl CefDecoder for JsonCefEncoder {
    fn decode(&self, _data: &[u8]) -> Result<CefEvent> {
        // Simplified JSON parsing - in production would use a JSON library
        Err(ToolError::Other(
            "JSON decoding not fully implemented in no_std context".to_string(),
        ))
    }

    fn schema_version(&self) -> CefFormatVersion {
        self.schema_version
    }
}

/// Binary CEF encoder.
///
/// Encodes events in compact binary format with length prefixing.
/// See Engineering Plan § 2.12.3: CEF Encoding - Binary Format.
#[derive(Clone, Debug)]
pub struct BinaryCefEncoder {
    schema_version: CefFormatVersion,
}

impl BinaryCefEncoder {
    /// Creates a new binary CEF encoder.
    pub fn new(schema_version: CefFormatVersion) -> Self {
        BinaryCefEncoder { schema_version }
    }

    /// Creates a binary encoder with default schema version.
    pub fn default_version() -> Self {
        BinaryCefEncoder {
            schema_version: CefFormatVersion::default(),
        }
    }
}

impl CefEncoder for BinaryCefEncoder {
    fn encode(&self, event: &CefEvent) -> Result<Vec<u8>> {
        let mut bytes = Vec::new();

        // Version (3 bytes: major, minor, patch)
        bytes.push(self.schema_version.major as u8);
        bytes.push(self.schema_version.minor as u8);
        bytes.push(self.schema_version.patch as u8);

        // Event ID length + data
        let id_bytes = event.event_id.as_bytes();
        bytes.push(id_bytes.len() as u8);
        bytes.extend_from_slice(id_bytes);

        // Event type (1 byte)
        let event_type_byte = match event.event_type {
            crate::cef::CefEventType::ThoughtStep => 0u8,
            crate::cef::CefEventType::ToolCallRequested => 1,
            crate::cef::CefEventType::ToolCallCompleted => 2,
            crate::cef::CefEventType::PolicyDecision => 3,
            crate::cef::CefEventType::MemoryAccess => 4,
            crate::cef::CefEventType::IpcMessage => 5,
            crate::cef::CefEventType::PhaseTransition => 6,
            crate::cef::CefEventType::CheckpointCreated => 7,
            crate::cef::CefEventType::SignalDispatched => 8,
            crate::cef::CefEventType::ExceptionRaised => 9,
        };
        bytes.push(event_type_byte);

        // Timestamp (8 bytes, big-endian)
        bytes.extend_from_slice(&event.timestamp_ns.to_be_bytes());

        Ok(bytes)
    }

    fn schema_version(&self) -> CefFormatVersion {
        self.schema_version
    }
}

impl CefDecoder for BinaryCefEncoder {
    fn decode(&self, data: &[u8]) -> Result<CefEvent> {
        if data.len() < 3 {
            return Err(ToolError::Other(
                "binary data too short for version header".to_string(),
            ));
        }

        let major = data[0];
        let minor = data[1];
        let patch = data[2];
        let version = CefFormatVersion::new(major as u16, minor as u16, patch as u16);

        if !version.is_compatible_with(self.schema_version) {
            return Err(ToolError::Other(
                alloc::format!(
                    "incompatible schema version: got {}, expected {}",
                    version, self.schema_version
                ),
            ));
        }

        let mut offset = 3;

        // Read event ID
        if offset >= data.len() {
            return Err(ToolError::Other("truncated data: no ID length".to_string()));
        }
        let id_len = data[offset] as usize;
        offset += 1;

        if offset + id_len > data.len() {
            return Err(ToolError::Other("truncated data: ID data".to_string()));
        }
        let event_id = alloc::string::String::from_utf8_lossy(&data[offset..offset + id_len])
            .to_string();
        offset += id_len;

        // Read event type
        if offset >= data.len() {
            return Err(ToolError::Other("truncated data: no event type".to_string()));
        }
        let event_type = match data[offset] {
            0 => crate::cef::CefEventType::ThoughtStep,
            1 => crate::cef::CefEventType::ToolCallRequested,
            2 => crate::cef::CefEventType::ToolCallCompleted,
            3 => crate::cef::CefEventType::PolicyDecision,
            4 => crate::cef::CefEventType::MemoryAccess,
            5 => crate::cef::CefEventType::IpcMessage,
            6 => crate::cef::CefEventType::PhaseTransition,
            7 => crate::cef::CefEventType::CheckpointCreated,
            8 => crate::cef::CefEventType::SignalDispatched,
            9 => crate::cef::CefEventType::ExceptionRaised,
            _ => {
                return Err(ToolError::Other(
                    alloc::format!("unknown event type: {}", data[offset]),
                ))
            }
        };
        offset += 1;

        // Read timestamp
        if offset + 8 > data.len() {
            return Err(ToolError::Other("truncated data: no timestamp".to_string()));
        }
        let mut ts_bytes = [0u8; 8];
        ts_bytes.copy_from_slice(&data[offset..offset + 8]);
        let timestamp_ms = u64::from_be_bytes(ts_bytes);

        // Create minimal event for decoding
        let event = CefEvent::new(
            &event_id,
            "trace-decoded",
            "span-decoded",
            "ct-decoded",
            "agent-decoded",
            timestamp_ms,
            event_type,
            "phase-decoded",
        );

        Ok(event)
    }

    fn schema_version(&self) -> CefFormatVersion {
        self.schema_version
    }
}

/// Schema registry for tracking event schema versions.
///
/// Maintains schema versions and provides compatibility checking.
/// See Engineering Plan § 2.12.4: Schema Evolution.
#[derive(Clone, Debug)]
pub struct SchemaRegistry {
    versions: BTreeMap<u16, CefFormatVersion>,
    current_version: CefFormatVersion,
}

impl SchemaRegistry {
    /// Creates a new schema registry.
    pub fn new(current_version: CefFormatVersion) -> Self {
        let mut versions = BTreeMap::new();
        versions.insert(current_version.major, current_version);
        SchemaRegistry {
            versions,
            current_version,
        }
    }

    /// Registers a new schema version.
    pub fn register_version(&mut self, version: CefFormatVersion) -> Result<()> {
        if version.major == 0 {
            return Err(ToolError::Other(
                "major version must be >= 1".to_string(),
            ));
        }
        self.versions.insert(version.major, version);
        Ok(())
    }

    /// Returns the current schema version.
    pub fn current_version(&self) -> CefFormatVersion {
        self.current_version
    }

    /// Checks if a version is registered and compatible.
    pub fn is_compatible(&self, version: CefFormatVersion) -> bool {
        self.versions
            .get(&version.major)
            .map(|v| v.major == version.major)
            .unwrap_or(false)
    }

    /// Gets all registered versions.
    pub fn all_versions(&self) -> Vec<CefFormatVersion> {
        self.versions.values().copied().collect()
    }
}

impl Default for SchemaRegistry {
    fn default() -> Self {
        SchemaRegistry::new(CefFormatVersion::default())
    }
}

/// Encodes a CEF event with optional compression.
///
/// See Engineering Plan § 2.12.3: CEF Encoding.
pub fn encode_with_compression(
    event: &CefEvent,
    encoder: &dyn CefEncoder,
    strategy: CompressionStrategy,
) -> Result<CompressedEvent> {
    let uncompressed = encoder.encode(event)?;
    let uncompressed_size = uncompressed.len() as u64;

    let data = match strategy {
        CompressionStrategy::None => uncompressed,
        CompressionStrategy::Lz4 => {
            // In production, would use lz4 crate
            // For now, return uncompressed
            uncompressed
        }
        CompressionStrategy::Zstd { .. } => {
            // In production, would use zstd crate
            // For now, return uncompressed
            uncompressed
        }
    };

    Ok(CompressedEvent {
        data,
        schema_version: encoder.schema_version(),
        compression: strategy,
        uncompressed_size,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;
use alloc::string::ToString;
use alloc::vec;

    #[test]
    fn test_cef_format_version_creation() {
        let v = CefFormatVersion::new(1, 2, 3);
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
    }

    #[test]
    fn test_cef_format_version_semver() {
        let v = CefFormatVersion::new(1, 2, 3);
        assert_eq!(v.to_semver(), "1.2.3");
    }

    #[test]
    fn test_cef_format_version_compatibility() {
        let v1 = CefFormatVersion::new(1, 0, 0);
        let v2 = CefFormatVersion::new(1, 1, 0);
        let v3 = CefFormatVersion::new(2, 0, 0);

        assert!(v1.is_compatible_with(v2));
        assert!(!v1.is_compatible_with(v3));
    }

    #[test]
    fn test_cef_format_version_ordering() {
        let v1 = CefFormatVersion::new(1, 0, 0);
        let v2 = CefFormatVersion::new(1, 1, 0);
        let v3 = CefFormatVersion::new(1, 0, 1);

        assert!(v1 < v2);
        assert!(v3 > v1);
    }

    #[test]
    fn test_cef_format_version_display() {
        let v = CefFormatVersion::new(1, 2, 3);
        assert_eq!(v.to_string(), "1.2.3");
    }

    #[test]
    fn test_cef_format_version_default() {
        let v = CefFormatVersion::default();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 0);
        assert_eq!(v.patch, 0);
    }

    #[test]
    fn test_compression_strategy_default() {
        let cs = CompressionStrategy::default();
        assert_eq!(cs, CompressionStrategy::None);
    }

    #[test]
    fn test_json_cef_encoder_creation() {
        let version = CefFormatVersion::new(1, 0, 0);
        let encoder = JsonCefEncoder::new(version, false);
        assert_eq!(CefEncoder::schema_version(&encoder), version);
    }

    #[test]
    fn test_json_cef_encoder_compact() {
        let encoder = JsonCefEncoder::compact();
        assert!(!encoder.pretty_print);
        assert_eq!(CefEncoder::schema_version(&encoder), CefFormatVersion::default());
    }

    #[test]
    fn test_json_cef_encoder_pretty() {
        let encoder = JsonCefEncoder::pretty();
        assert!(encoder.pretty_print);
        assert_eq!(CefEncoder::schema_version(&encoder), CefFormatVersion::default());
    }

    #[test]
    fn test_binary_cef_encoder_creation() {
        let version = CefFormatVersion::new(1, 0, 0);
        let encoder = BinaryCefEncoder::new(version);
        assert_eq!(CefEncoder::schema_version(&encoder), version);
    }

    #[test]
    fn test_binary_cef_encoder_default_version() {
        let encoder = BinaryCefEncoder::default_version();
        assert_eq!(CefEncoder::schema_version(&encoder), CefFormatVersion::default());
    }

    #[test]
    fn test_json_encode_roundtrip() {
        let encoder = JsonCefEncoder::compact();
        let event = crate::cef::CefEvent::new(
            "event-1",
            "trace-1",
            "span-1",
            "ct-1",
            "agent-1",
            1000,
            crate::cef::CefEventType::ToolCallRequested,
            "acting",
        );

        let encoded = encoder.encode(&event).expect("encoding failed");
        assert!(!encoded.is_empty());
        let json_str = alloc::string::String::from_utf8_lossy(&encoded);
        assert!(json_str.contains("event-1"));
    }

    #[test]
    fn test_binary_encode_roundtrip() {
        let encoder = BinaryCefEncoder::default_version();
        let event = crate::cef::CefEvent::new(
            "event-1",
            "trace-1",
            "span-1",
            "ct-1",
            "agent-1",
            1000,
            crate::cef::CefEventType::ToolCallRequested,
            "acting",
        );

        let encoded = encoder.encode(&event).expect("encoding failed");
        assert!(encoded.len() >= 12); // Version (3) + ID len (1) + ID (7) + type (1)

        let decoded = encoder.decode(&encoded).expect("decoding failed");
        assert_eq!(decoded.event_id, event.event_id);
        assert_eq!(decoded.event_type, event.event_type);
        assert_eq!(decoded.timestamp_ns, event.timestamp_ns);
    }

    #[test]
    fn test_binary_decode_with_version_check() {
        let encoder_v1 = BinaryCefEncoder::new(CefFormatVersion::new(1, 0, 0));
        let encoder_v2 = BinaryCefEncoder::new(CefFormatVersion::new(2, 0, 0));

        let event = crate::cef::CefEvent::new(
            "event-1",
            "trace-1",
            "span-1",
            "ct-1",
            "agent-1",
            1000,
            crate::cef::CefEventType::ToolCallRequested,
            "acting",
        );

        let encoded = encoder_v1.encode(&event).expect("encoding failed");

        // Same version should decode successfully
        let decoded = encoder_v1.decode(&encoded).expect("decoding failed");
        assert_eq!(decoded.event_id, "event-1");

        // Different major version should fail
        let result = encoder_v2.decode(&encoded);
        assert!(result.is_err());
    }

    #[test]
    fn test_schema_registry_creation() {
        let version = CefFormatVersion::new(1, 0, 0);
        let registry = SchemaRegistry::new(version);
        assert_eq!(registry.current_version(), version);
    }

    #[test]
    fn test_schema_registry_register_version() {
        let mut registry = SchemaRegistry::new(CefFormatVersion::new(1, 0, 0));
        let v2 = CefFormatVersion::new(2, 0, 0);
        assert!(registry.register_version(v2).is_ok());
        assert!(registry.is_compatible(v2));
    }

    #[test]
    fn test_schema_registry_reject_zero_major() {
        let mut registry = SchemaRegistry::new(CefFormatVersion::new(1, 0, 0));
        let v0 = CefFormatVersion::new(0, 0, 0);
        assert!(registry.register_version(v0).is_err());
    }

    #[test]
    fn test_schema_registry_compatibility() {
        let registry = SchemaRegistry::new(CefFormatVersion::new(1, 0, 0));
        let v1_1 = CefFormatVersion::new(1, 1, 0);
        let v2_0 = CefFormatVersion::new(2, 0, 0);

        assert!(registry.is_compatible(v1_1));
        assert!(!registry.is_compatible(v2_0));
    }

    #[test]
    fn test_schema_registry_all_versions() {
        let mut registry = SchemaRegistry::new(CefFormatVersion::new(1, 0, 0));
        registry.register_version(CefFormatVersion::new(2, 0, 0)).ok();
        registry.register_version(CefFormatVersion::new(3, 0, 0)).ok();

        let versions = registry.all_versions();
        assert_eq!(versions.len(), 3);
    }

    #[test]
    fn test_schema_registry_default() {
        let registry = SchemaRegistry::default();
        assert_eq!(
            registry.current_version(),
            CefFormatVersion::default()
        );
    }

    #[test]
    fn test_compressed_event_no_compression() {
        let event = CompressedEvent {
            data: vec![1, 2, 3, 4, 5],
            schema_version: CefFormatVersion::default(),
            compression: CompressionStrategy::None,
            uncompressed_size: 5,
        };
        assert_eq!(event.compression_ratio(), 100.0);
    }

    #[test]
    fn test_compressed_event_ratio_with_compression() {
        let event = CompressedEvent {
            data: vec![1, 2, 3],
            schema_version: CefFormatVersion::default(),
            compression: CompressionStrategy::Lz4,
            uncompressed_size: 10,
        };
        assert_eq!(event.compression_ratio(), 30.0);
    }

    #[test]
    fn test_encode_with_compression_no_compression() {
        let encoder = BinaryCefEncoder::default_version();
        let event = crate::cef::CefEvent::new(
            "event-1",
            "trace-1",
            "span-1",
            "ct-1",
            "agent-1",
            1000,
            crate::cef::CefEventType::ToolCallRequested,
            "acting",
        );

        let result =
            encode_with_compression(&event, &encoder, CompressionStrategy::None)
                .expect("encoding failed");
        assert_eq!(result.compression, CompressionStrategy::None);
        assert!(result.uncompressed_size > 0);
    }
}
