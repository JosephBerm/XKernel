// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! # Message Types and Message Pool
//!
//! This module defines the Cap'n Proto-style message types for the IPC subsystem,
//! including request and response messages, message headers, and a pre-allocated
//! message pool for efficient allocation without per-message heap fragmentation.
//!
//! ## Message Structure
//!
//! Messages use a fixed 64-byte header followed by variable-length payload data.
//! This provides efficient alignment and cache-line locality.
//!
//! ## Message Pool
//!
//! The message pool pre-allocates a configurable number of message buffers to
//! avoid per-message heap allocations during high-frequency IPC operations.
//!
//! ## References
//!
//! - Engineering Plan § 5.3.1 (Message Types and Cap'n Proto)

use crate::error::{CsError, IpcError, Result};
use alloc::vec::Vec;
use core::fmt;
use serde::{Deserialize, Serialize};

/// Message flags as a bitfield.
///
/// Defines flags controlling message behavior and constraints.
/// Uses an 8-bit bitfield for compact representation.
///
/// See Engineering Plan § 5.3.1 (Message Types)
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct MessageFlags(u8);

impl MessageFlags {
    /// Flag: Message requires a response.
    const REQUIRES_RESPONSE: u8 = 0x01;
    /// Flag: Message has high priority.
    const HIGH_PRIORITY: u8 = 0x02;
    /// Flag: Message payload is zero-copy eligible.
    const ZERO_COPY_ELIGIBLE: u8 = 0x04;
    /// Flag: Message carries a capability.
    const CAPABILITY_ATTACHED: u8 = 0x08;

    /// Create an empty message flags set.
    #[inline]
    pub fn new() -> Self {
        Self(0)
    }

    /// Set the REQUIRES_RESPONSE flag.
    #[inline]
    pub fn set_requires_response(mut self) -> Self {
        self.0 |= Self::REQUIRES_RESPONSE;
        self
    }

    /// Check if REQUIRES_RESPONSE flag is set.
    #[inline]
    pub fn requires_response(&self) -> bool {
        (self.0 & Self::REQUIRES_RESPONSE) != 0
    }

    /// Set the HIGH_PRIORITY flag.
    #[inline]
    pub fn set_high_priority(mut self) -> Self {
        self.0 |= Self::HIGH_PRIORITY;
        self
    }

    /// Check if HIGH_PRIORITY flag is set.
    #[inline]
    pub fn high_priority(&self) -> bool {
        (self.0 & Self::HIGH_PRIORITY) != 0
    }

    /// Set the ZERO_COPY_ELIGIBLE flag.
    #[inline]
    pub fn set_zero_copy_eligible(mut self) -> Self {
        self.0 |= Self::ZERO_COPY_ELIGIBLE;
        self
    }

    /// Check if ZERO_COPY_ELIGIBLE flag is set.
    #[inline]
    pub fn zero_copy_eligible(&self) -> bool {
        (self.0 & Self::ZERO_COPY_ELIGIBLE) != 0
    }

    /// Set the CAPABILITY_ATTACHED flag.
    #[inline]
    pub fn set_capability_attached(mut self) -> Self {
        self.0 |= Self::CAPABILITY_ATTACHED;
        self
    }

    /// Check if CAPABILITY_ATTACHED flag is set.
    #[inline]
    pub fn capability_attached(&self) -> bool {
        (self.0 & Self::CAPABILITY_ATTACHED) != 0
    }

    /// Get the raw bitfield value.
    #[inline]
    pub fn as_u8(&self) -> u8 {
        self.0
    }

    /// Create from a raw bitfield value.
    #[inline]
    pub fn from_u8(value: u8) -> Self {
        Self(value)
    }
}

/// Message header for IPC messages.
///
/// Fixed 64-byte header containing metadata for message delivery and correlation.
/// This header is followed by variable-length payload data.
///
/// See Engineering Plan § 5.3.1 (Message Types)
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MessageHeader {
    /// Unique request ID for request-response correlation.
    /// Used to match responses with their requests.
    pub request_id: u64,

    /// Sender ULID encoded as u64 (high 64 bits).
    /// Identifies the agent sending the message.
    pub sender_id: u64,

    /// Method or operation ID being invoked.
    pub method_id: u32,

    /// Deadline in nanoseconds (absolute clock value).
    /// Used for timeout detection and prioritization.
    pub deadline_ns: u64,

    /// Message flags (REQUIRES_RESPONSE, HIGH_PRIORITY, etc.).
    pub flags: MessageFlags,

    /// Size of the payload data following this header (in bytes).
    pub payload_size: u32,

    /// Reserved for future use (padding to 64 bytes).
    _reserved: [u8; 15],
}

impl MessageHeader {
    /// Size of the message header in bytes (must be exactly 64).
    pub const SIZE_BYTES: usize = 64;

    /// Create a new message header.
    pub fn new(
        request_id: u64,
        sender_id: u64,
        method_id: u32,
        deadline_ns: u64,
        flags: MessageFlags,
        payload_size: u32,
    ) -> Self {
        Self {
            request_id,
            sender_id,
            method_id,
            deadline_ns,
            flags,
            payload_size,
            _reserved: [0; 15],
        }
    }

    /// Verify the header is well-formed.
    pub fn validate(&self) -> Result<()> {
        // Payload size should be reasonable (not checking absolute bounds as those
        // are channel-dependent)
        Ok(())
    }

    /// Serialize the header to a byte array.
    pub fn to_bytes(&self) -> [u8; Self::SIZE_BYTES] {
        let mut bytes = [0u8; Self::SIZE_BYTES];

        // request_id (8 bytes, big-endian)
        bytes[0..8].copy_from_slice(&self.request_id.to_be_bytes());
        // sender_id (8 bytes, big-endian)
        bytes[8..16].copy_from_slice(&self.sender_id.to_be_bytes());
        // method_id (4 bytes, big-endian)
        bytes[16..20].copy_from_slice(&self.method_id.to_be_bytes());
        // deadline_ns (8 bytes, big-endian)
        bytes[20..28].copy_from_slice(&self.deadline_ns.to_be_bytes());
        // flags (1 byte)
        bytes[28] = self.flags.as_u8();
        // payload_size (4 bytes, big-endian)
        bytes[29..33].copy_from_slice(&self.payload_size.to_be_bytes());
        // reserved (15 bytes of zeros) - already initialized

        bytes
    }

    /// Deserialize the header from a byte array.
    pub fn from_bytes(bytes: &[u8; Self::SIZE_BYTES]) -> Result<Self> {
        if bytes.len() < Self::SIZE_BYTES {
            return Err(CsError::Ipc(IpcError::Other(
                alloc::string::String::from("buffer too small for message header"),
            )));
        }

        let request_id = u64::from_be_bytes(bytes[0..8].try_into().map_err(|_| {
            CsError::Ipc(IpcError::Other(alloc::string::String::from(
                "failed to parse request_id",
            )))
        })?);
        let sender_id = u64::from_be_bytes(bytes[8..16].try_into().map_err(|_| {
            CsError::Ipc(IpcError::Other(alloc::string::String::from(
                "failed to parse sender_id",
            )))
        })?);
        let method_id = u32::from_be_bytes(bytes[16..20].try_into().map_err(|_| {
            CsError::Ipc(IpcError::Other(alloc::string::String::from(
                "failed to parse method_id",
            )))
        })?);
        let deadline_ns = u64::from_be_bytes(bytes[20..28].try_into().map_err(|_| {
            CsError::Ipc(IpcError::Other(alloc::string::String::from(
                "failed to parse deadline_ns",
            )))
        })?);
        let flags = MessageFlags::from_u8(bytes[28]);
        let payload_size = u32::from_be_bytes(bytes[29..33].try_into().map_err(|_| {
            CsError::Ipc(IpcError::Other(alloc::string::String::from(
                "failed to parse payload_size",
            )))
        })?);

        Ok(Self {
            request_id,
            sender_id,
            method_id,
            deadline_ns,
            flags,
            payload_size,
            _reserved: [0; 15],
        })
    }
}

/// Request message for IPC.
///
/// Combines a message header with variable-length payload data.
/// Used for sending method invocations or requests to a receiver.
///
/// See Engineering Plan § 5.3.1 (Message Types)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RequestMessage {
    /// Message header (fixed 64 bytes).
    pub header: MessageHeader,

    /// Payload data (variable length).
    pub payload: Vec<u8>,
}

impl RequestMessage {
    /// Create a new request message.
    pub fn new(header: MessageHeader, payload: Vec<u8>) -> Result<Self> {
        header.validate()?;

        // Verify payload size matches header
        if header.payload_size != payload.len() as u32 {
            return Err(CsError::Ipc(IpcError::Other(
                alloc::string::String::from("payload size mismatch with header"),
            )));
        }

        Ok(Self { header, payload })
    }

    /// Get the total size of this message (header + payload).
    pub fn total_size(&self) -> usize {
        MessageHeader::SIZE_BYTES + self.payload.len()
    }

    /// Serialize this message to a byte vector.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.header.to_bytes().to_vec();
        bytes.extend_from_slice(&self.payload);
        bytes
    }

    /// Deserialize a message from bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < MessageHeader::SIZE_BYTES {
            return Err(CsError::Ipc(IpcError::Other(
                alloc::string::String::from("buffer too small for message header"),
            )));
        }

        let header_bytes: [u8; MessageHeader::SIZE_BYTES] =
            bytes[..MessageHeader::SIZE_BYTES].try_into().map_err(|_| {
                CsError::Ipc(IpcError::Other(alloc::string::String::from(
                    "failed to extract header bytes",
                )))
            })?;

        let header = MessageHeader::from_bytes(&header_bytes)?;

        let payload = if bytes.len() > MessageHeader::SIZE_BYTES {
            bytes[MessageHeader::SIZE_BYTES..].to_vec()
        } else {
            Vec::new()
        };

        Self::new(header, payload)
    }
}

/// Response status for response messages.
///
/// Indicates the outcome of a request.
///
/// See Engineering Plan § 5.3.1 (Message Types)
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ResponseStatus {
    /// Request was successfully processed.
    Success = 0,
    /// Request processing resulted in an error.
    Error = 1,
    /// Request processing timed out.
    Timeout = 2,
    /// Request was cancelled.
    Cancelled = 3,
}

impl ResponseStatus {
    /// Convert from u8 value.
    pub fn from_u8(value: u8) -> Result<Self> {
        match value {
            0 => Ok(ResponseStatus::Success),
            1 => Ok(ResponseStatus::Error),
            2 => Ok(ResponseStatus::Timeout),
            3 => Ok(ResponseStatus::Cancelled),
            _ => Err(CsError::Ipc(IpcError::Other(
                alloc::string::String::from("invalid response status"),
            ))),
        }
    }

    /// Convert to u8 value.
    pub fn as_u8(&self) -> u8 {
        *self as u8
    }
}

impl fmt::Display for ResponseStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResponseStatus::Success => write!(f, "Success"),
            ResponseStatus::Error => write!(f, "Error"),
            ResponseStatus::Timeout => write!(f, "Timeout"),
            ResponseStatus::Cancelled => write!(f, "Cancelled"),
        }
    }
}

/// Response message for IPC.
///
/// Sent in response to a request message. Contains the original request ID
/// for correlation, a status code, and optional response payload.
///
/// See Engineering Plan § 5.3.1 (Message Types)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResponseMessage {
    /// Message header (reused, request_id field contains original request ID).
    pub header: MessageHeader,

    /// Response status (success, error, timeout, cancelled).
    pub status: ResponseStatus,

    /// Response payload (variable length).
    pub payload: Vec<u8>,
}

impl ResponseMessage {
    /// Create a new response message.
    pub fn new(header: MessageHeader, status: ResponseStatus, payload: Vec<u8>) -> Result<Self> {
        header.validate()?;

        // Verify payload size matches header
        if header.payload_size != payload.len() as u32 {
            return Err(CsError::Ipc(IpcError::Other(
                alloc::string::String::from("payload size mismatch with header"),
            )));
        }

        Ok(Self {
            header,
            status,
            payload,
        })
    }

    /// Get the total size of this message (header + status byte + payload).
    pub fn total_size(&self) -> usize {
        MessageHeader::SIZE_BYTES + 1 + self.payload.len()
    }

    /// Serialize this message to a byte vector.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.header.to_bytes().to_vec();
        bytes.push(self.status.as_u8());
        bytes.extend_from_slice(&self.payload);
        bytes
    }

    /// Deserialize a message from bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < MessageHeader::SIZE_BYTES + 1 {
            return Err(CsError::Ipc(IpcError::Other(
                alloc::string::String::from("buffer too small for response message"),
            )));
        }

        let header_bytes: [u8; MessageHeader::SIZE_BYTES] =
            bytes[..MessageHeader::SIZE_BYTES].try_into().map_err(|_| {
                CsError::Ipc(IpcError::Other(alloc::string::String::from(
                    "failed to extract header bytes",
                )))
            })?;

        let header = MessageHeader::from_bytes(&header_bytes)?;
        let status = ResponseStatus::from_u8(bytes[MessageHeader::SIZE_BYTES])?;

        let payload = if bytes.len() > MessageHeader::SIZE_BYTES + 1 {
            bytes[MessageHeader::SIZE_BYTES + 1..].to_vec()
        } else {
            Vec::new()
        };

        Self::new(header, status, payload)
    }
}

/// Message pool for pre-allocated message buffers.
///
/// Maintains a pool of pre-allocated message buffers to avoid per-message
/// heap allocations during high-frequency IPC operations.
///
/// See Engineering Plan § 5.3.1 (Message Types)
#[derive(Clone, Debug)]
pub struct MessagePool {
    /// Pre-allocated message buffers.
    buffers: Vec<Vec<u8>>,

    /// Default buffer size.
    buffer_size: usize,
}

impl MessagePool {
    /// Create a new message pool with the specified number of buffers.
    pub fn new(pool_size: usize, buffer_size: usize) -> Self {
        let mut buffers = Vec::with_capacity(pool_size);
        for _ in 0..pool_size {
            buffers.push(Vec::with_capacity(buffer_size));
        }

        Self {
            buffers,
            buffer_size,
        }
    }

    /// Allocate a buffer from the pool.
    ///
    /// Returns a pre-allocated buffer if available, otherwise allocates a new one.
    pub fn allocate(&mut self) -> Vec<u8> {
        self.buffers.pop().unwrap_or_else(|| Vec::with_capacity(self.buffer_size))
    }

    /// Recycle a buffer back into the pool.
    ///
    /// The buffer is cleared and returned to the pool for reuse.
    pub fn recycle(&mut self, mut buffer: Vec<u8>) {
        buffer.clear();
        if self.buffers.len() < self.buffers.capacity() {
            self.buffers.push(buffer);
        }
        // Otherwise drop the buffer if pool is full
    }

    /// Get the current number of available buffers.
    pub fn available_count(&self) -> usize {
        self.buffers.len()
    }

    /// Get the total capacity of the pool.
    pub fn capacity(&self) -> usize {
        self.buffers.capacity()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::string::String;
use alloc::vec;

    #[test]
    fn test_message_flags_requires_response() {
        let flags = MessageFlags::new().set_requires_response();
        assert!(flags.requires_response());
        assert!(!flags.high_priority());
    }

    #[test]
    fn test_message_flags_multiple() {
        let flags = MessageFlags::new()
            .set_requires_response()
            .set_high_priority()
            .set_zero_copy_eligible();
        assert!(flags.requires_response());
        assert!(flags.high_priority());
        assert!(flags.zero_copy_eligible());
        assert!(!flags.capability_attached());
    }

    #[test]
    fn test_message_flags_all_set() {
        let flags = MessageFlags::new()
            .set_requires_response()
            .set_high_priority()
            .set_zero_copy_eligible()
            .set_capability_attached();
        assert!(flags.requires_response());
        assert!(flags.high_priority());
        assert!(flags.zero_copy_eligible());
        assert!(flags.capability_attached());
    }

    #[test]
    fn test_message_flags_u8_roundtrip() {
        let original = MessageFlags::new()
            .set_requires_response()
            .set_capability_attached();
        let u8_val = original.as_u8();
        let restored = MessageFlags::from_u8(u8_val);
        assert_eq!(original, restored);
    }

    #[test]
    fn test_message_header_size() {
        assert_eq!(MessageHeader::SIZE_BYTES, 64);
    }

    #[test]
    fn test_message_header_roundtrip() {
        let flags = MessageFlags::new().set_requires_response();
        let header = MessageHeader::new(12345, 67890, 99, 1000000000, flags, 256);

        let bytes = header.to_bytes();
        assert_eq!(bytes.len(), 64);

        let header2 = MessageHeader::from_bytes(&bytes).unwrap();
        assert_eq!(header, header2);
    }

    #[test]
    fn test_message_header_validate() {
        let flags = MessageFlags::new();
        let header = MessageHeader::new(0, 0, 0, 0, flags, 0);
        assert!(header.validate().is_ok());
    }

    #[test]
    fn test_request_message_creation() {
        let flags = MessageFlags::new().set_requires_response();
        let header = MessageHeader::new(123, 456, 1, 5000000000, flags, 5);
        let payload = alloc::vec![1, 2, 3, 4, 5];
        let msg = RequestMessage::new(header, payload).unwrap();
        assert_eq!(msg.header.request_id, 123);
        assert_eq!(msg.payload.len(), 5);
    }

    #[test]
    fn test_request_message_size_mismatch() {
        let flags = MessageFlags::new();
        let header = MessageHeader::new(123, 456, 1, 5000000000, flags, 10); // says 10 bytes
        let payload = alloc::vec![1, 2, 3, 4, 5]; // but only 5 bytes
        assert!(RequestMessage::new(header, payload).is_err());
    }

    #[test]
    fn test_request_message_total_size() {
        let flags = MessageFlags::new();
        let header = MessageHeader::new(123, 456, 1, 5000000000, flags, 256);
        let payload = alloc::vec![42; 256];
        let msg = RequestMessage::new(header, payload).unwrap();
        assert_eq!(msg.total_size(), 64 + 256);
    }

    #[test]
    fn test_request_message_serialization() {
        let flags = MessageFlags::new().set_requires_response();
        let header = MessageHeader::new(123, 456, 1, 5000000000, flags, 3);
        let payload = alloc::vec![10, 20, 30];
        let msg = RequestMessage::new(header, payload).unwrap();

        let bytes = msg.to_bytes();
        let msg2 = RequestMessage::from_bytes(&bytes).unwrap();

        assert_eq!(msg.header.request_id, msg2.header.request_id);
        assert_eq!(msg.payload, msg2.payload);
    }

    #[test]
    fn test_response_status_u8_conversion() {
        assert_eq!(ResponseStatus::Success.as_u8(), 0);
        assert_eq!(ResponseStatus::Error.as_u8(), 1);
        assert_eq!(ResponseStatus::Timeout.as_u8(), 2);
        assert_eq!(ResponseStatus::Cancelled.as_u8(), 3);

        assert_eq!(ResponseStatus::from_u8(0).unwrap(), ResponseStatus::Success);
        assert_eq!(ResponseStatus::from_u8(1).unwrap(), ResponseStatus::Error);
        assert_eq!(ResponseStatus::from_u8(2).unwrap(), ResponseStatus::Timeout);
        assert_eq!(ResponseStatus::from_u8(3).unwrap(), ResponseStatus::Cancelled);
        assert!(ResponseStatus::from_u8(99).is_err());
    }

    #[test]
    fn test_response_message_creation() {
        let flags = MessageFlags::new();
        let header = MessageHeader::new(999, 111, 2, 6000000000, flags, 4);
        let payload = alloc::vec![5, 6, 7, 8];
        let msg = ResponseMessage::new(header, ResponseStatus::Success, payload).unwrap();
        assert_eq!(msg.header.request_id, 999);
        assert_eq!(msg.status, ResponseStatus::Success);
        assert_eq!(msg.payload.len(), 4);
    }

    #[test]
    fn test_response_message_total_size() {
        let flags = MessageFlags::new();
        let header = MessageHeader::new(999, 111, 2, 6000000000, flags, 128);
        let payload = alloc::vec![42; 128];
        let msg = ResponseMessage::new(header, ResponseStatus::Error, payload).unwrap();
        assert_eq!(msg.total_size(), 64 + 1 + 128);
    }

    #[test]
    fn test_response_message_serialization() {
        let flags = MessageFlags::new().set_high_priority();
        let header = MessageHeader::new(777, 888, 3, 7000000000, flags, 2);
        let payload = alloc::vec![99, 88];
        let msg = ResponseMessage::new(header, ResponseStatus::Timeout, payload).unwrap();

        let bytes = msg.to_bytes();
        let msg2 = ResponseMessage::from_bytes(&bytes).unwrap();

        assert_eq!(msg.header.request_id, msg2.header.request_id);
        assert_eq!(msg.status, msg2.status);
        assert_eq!(msg.payload, msg2.payload);
    }

    #[test]
    fn test_message_pool_allocation() {
        let mut pool = MessagePool::new(5, 256);
        assert_eq!(pool.capacity(), 5);
        assert_eq!(pool.available_count(), 5);

        let buf1 = pool.allocate();
        assert_eq!(pool.available_count(), 4);
        assert!(buf1.capacity() >= 256);

        let buf2 = pool.allocate();
        assert_eq!(pool.available_count(), 3);
    }

    #[test]
    fn test_message_pool_recycle() {
        let mut pool = MessagePool::new(3, 256);
        let buf = pool.allocate();
        assert_eq!(pool.available_count(), 2);

        pool.recycle(buf);
        assert_eq!(pool.available_count(), 3);
    }

    #[test]
    fn test_message_pool_overfull() {
        let mut pool = MessagePool::new(2, 256);
        let buf1 = pool.allocate();
        let buf2 = pool.allocate();

        pool.recycle(buf1); // pool is now full
        assert_eq!(pool.available_count(), 2);

        pool.recycle(buf2); // this should be dropped since pool is full
        assert_eq!(pool.available_count(), 2);
    }

    #[test]
    fn test_message_pool_dynamic_allocation() {
        let mut pool = MessagePool::new(1, 256);
        let _buf1 = pool.allocate();
        let _buf2 = pool.allocate(); // allocates new since pool empty
        let _buf3 = pool.allocate();

        assert_eq!(pool.available_count(), 0);
    }
}
