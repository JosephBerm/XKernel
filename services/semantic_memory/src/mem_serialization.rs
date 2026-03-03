// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Binary serialization format for Memory Manager IPC messages.
//!
//! This module defines compact binary encoding/decoding for MemoryRequest and
//! MemoryResponse enums used in IPC communication. The format is designed for
//! low latency and minimal bandwidth consumption.
//!
//! See Engineering Plan § 4.1.0: IPC Serialization (Week 5).

use alloc::string::String;
use alloc::vec::Vec;
use crate::error::{MemoryError, Result};
use crate::mem_syscall_interface::{
    AllocFlags, MountFlags, MountSource, MemHandle, MountHandle,
};

/// Maximum size of a single serialized request (32 KiB).
pub const MAX_REQUEST_SIZE: usize = 32 * 1024;

/// Maximum size of a single serialized response (256 KiB).
pub const MAX_RESPONSE_SIZE: usize = 256 * 1024;

/// Maximum length of string fields (mount points, paths, etc.).
pub const MAX_STRING_LEN: usize = 4096;

/// Maximum length of data buffer in mem_write requests (1 MiB).
pub const MAX_DATA_BUFFER_LEN: usize = 1024 * 1024;

/// Memory request types (message discriminators).
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryRequestType {
    /// mem_alloc syscall (0x01)
    Allocate = 0x01,
    /// mem_read syscall (0x02)
    Read = 0x02,
    /// mem_write syscall (0x03)
    Write = 0x03,
    /// mem_mount syscall (0x04)
    Mount = 0x04,
}

/// Memory response types (message discriminators).
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryResponseType {
    /// Allocation succeeded (0x80)
    Allocated = 0x80,
    /// Read succeeded (0x81)
    ReadData = 0x81,
    /// Write succeeded (0x82)
    WriteAck = 0x82,
    /// Mount succeeded (0x83)
    Mounted = 0x83,
    /// Error occurred (0xFF)
    Error = 0xFF,
}

/// Serialized memory request for IPC transmission.
///
/// Compact binary format: [type][sequence][data...]
/// See Engineering Plan § 4.1.0: IPC Serialization.
#[derive(Clone, Debug)]
pub struct SerializedMemoryRequest {
    /// Raw binary data
    pub bytes: Vec<u8>,
}

impl SerializedMemoryRequest {
    /// Creates an empty serialized request.
    fn new() -> Self {
        SerializedMemoryRequest {
            bytes: Vec::new(),
        }
    }

    /// Returns the message type discriminator.
    pub fn message_type(&self) -> Result<MemoryRequestType> {
        if self.bytes.is_empty() {
            return Err(MemoryError::Other("empty request".into()));
        }

        match self.bytes[0] {
            0x01 => Ok(MemoryRequestType::Allocate),
            0x02 => Ok(MemoryRequestType::Read),
            0x03 => Ok(MemoryRequestType::Write),
            0x04 => Ok(MemoryRequestType::Mount),
            _ => Err(MemoryError::Other("invalid request type".into())),
        }
    }

    /// Returns the size of serialized data.
    pub fn size(&self) -> usize {
        self.bytes.len()
    }
}

/// Serialized memory response for IPC transmission.
///
/// Compact binary format: [type][sequence][data...]
/// See Engineering Plan § 4.1.0: IPC Serialization.
#[derive(Clone, Debug)]
pub struct SerializedMemoryResponse {
    /// Raw binary data
    pub bytes: Vec<u8>,
}

impl SerializedMemoryResponse {
    /// Creates an empty serialized response.
    fn new() -> Self {
        SerializedMemoryResponse {
            bytes: Vec::new(),
        }
    }

    /// Returns the message type discriminator.
    pub fn message_type(&self) -> Result<MemoryResponseType> {
        if self.bytes.is_empty() {
            return Err(MemoryError::Other("empty response".into()));
        }

        match self.bytes[0] {
            0x80 => Ok(MemoryResponseType::Allocated),
            0x81 => Ok(MemoryResponseType::ReadData),
            0x82 => Ok(MemoryResponseType::WriteAck),
            0x83 => Ok(MemoryResponseType::Mounted),
            0xFF => Ok(MemoryResponseType::Error),
            _ => Err(MemoryError::Other("invalid response type".into())),
        }
    }

    /// Returns the size of serialized data.
    pub fn size(&self) -> usize {
        self.bytes.len()
    }
}

/// Encoder for memory requests (write-only binary builder).
pub struct RequestEncoder {
    buffer: Vec<u8>,
}

impl RequestEncoder {
    /// Creates a new request encoder.
    pub fn new() -> Self {
        RequestEncoder {
            buffer: Vec::new(),
        }
    }

    /// Encodes a mem_alloc request.
    pub fn encode_allocate(
        &mut self,
        size: u64,
        alignment: u64,
        flags: AllocFlags,
    ) -> Result<SerializedMemoryRequest> {
        self.buffer.clear();

        // Message type: Allocate (0x01)
        self.buffer.push(MemoryRequestType::Allocate as u8);

        // Sequence number (placeholder, handled by IPC layer)
        self.buffer.push(0);

        // Size (8 bytes, little-endian)
        self.buffer.extend_from_slice(&size.to_le_bytes());

        // Alignment (8 bytes, little-endian)
        self.buffer.extend_from_slice(&alignment.to_le_bytes());

        // Flags (4 bytes, little-endian)
        self.buffer.extend_from_slice(&flags.bits().to_le_bytes());

        if self.buffer.len() > MAX_REQUEST_SIZE {
            return Err(MemoryError::Other("request too large".into()));
        }

        Ok(SerializedMemoryRequest {
            bytes: self.buffer.clone(),
        })
    }

    /// Encodes a mem_read request.
    pub fn encode_read(
        &mut self,
        handle: MemHandle,
        offset: u64,
        size: u64,
    ) -> Result<SerializedMemoryRequest> {
        self.buffer.clear();

        // Message type: Read (0x02)
        self.buffer.push(MemoryRequestType::Read as u8);

        // Sequence number (placeholder)
        self.buffer.push(0);

        // Handle (8 bytes)
        self.buffer.extend_from_slice(&handle.as_u64().to_le_bytes());

        // Offset (8 bytes)
        self.buffer.extend_from_slice(&offset.to_le_bytes());

        // Size (8 bytes)
        self.buffer.extend_from_slice(&size.to_le_bytes());

        if self.buffer.len() > MAX_REQUEST_SIZE {
            return Err(MemoryError::Other("request too large".into()));
        }

        Ok(SerializedMemoryRequest {
            bytes: self.buffer.clone(),
        })
    }

    /// Encodes a mem_write request.
    pub fn encode_write(
        &mut self,
        handle: MemHandle,
        offset: u64,
        size: u64,
        data: &[u8],
    ) -> Result<SerializedMemoryRequest> {
        if size as usize > data.len() {
            return Err(MemoryError::Other("data buffer too small".into()));
        }

        if size as usize > MAX_DATA_BUFFER_LEN {
            return Err(MemoryError::Other("data too large".into()));
        }

        self.buffer.clear();

        // Message type: Write (0x03)
        self.buffer.push(MemoryRequestType::Write as u8);

        // Sequence number (placeholder)
        self.buffer.push(0);

        // Handle (8 bytes)
        self.buffer.extend_from_slice(&handle.as_u64().to_le_bytes());

        // Offset (8 bytes)
        self.buffer.extend_from_slice(&offset.to_le_bytes());

        // Size (8 bytes)
        self.buffer.extend_from_slice(&size.to_le_bytes());

        // Data (variable length)
        self.buffer.extend_from_slice(&data[..size as usize]);

        if self.buffer.len() > MAX_REQUEST_SIZE {
            return Err(MemoryError::Other("request too large".into()));
        }

        Ok(SerializedMemoryRequest {
            bytes: self.buffer.clone(),
        })
    }

    /// Encodes a mem_mount request.
    pub fn encode_mount(
        &mut self,
        source: &MountSource,
        mount_point: &str,
        flags: MountFlags,
    ) -> Result<SerializedMemoryRequest> {
        if mount_point.len() > MAX_STRING_LEN {
            return Err(MemoryError::Other("mount point too long".into()));
        }

        let source_str = source.as_str();
        if source_str.len() > MAX_STRING_LEN {
            return Err(MemoryError::Other("source path too long".into()));
        }

        self.buffer.clear();

        // Message type: Mount (0x04)
        self.buffer.push(MemoryRequestType::Mount as u8);

        // Sequence number (placeholder)
        self.buffer.push(0);

        // Source type (1 byte)
        let source_type = match source {
            MountSource::LocalPath(_) => 0u8,
            MountSource::RemoteUrl(_) => 1u8,
            MountSource::SharedRegion(_) => 2u8,
            MountSource::CrewReplica(_) => 3u8,
        };
        self.buffer.push(source_type);

        // Source string length (2 bytes) + data
        self.buffer.extend_from_slice(&(source_str.len() as u16).to_le_bytes());
        self.buffer.extend_from_slice(source_str.as_bytes());

        // Mount point length (2 bytes) + data
        self.buffer
            .extend_from_slice(&(mount_point.len() as u16).to_le_bytes());
        self.buffer.extend_from_slice(mount_point.as_bytes());

        // Flags (4 bytes)
        self.buffer.extend_from_slice(&flags.bits().to_le_bytes());

        if self.buffer.len() > MAX_REQUEST_SIZE {
            return Err(MemoryError::Other("request too large".into()));
        }

        Ok(SerializedMemoryRequest {
            bytes: self.buffer.clone(),
        })
    }
}

/// Decoder for memory requests (read-only binary parser).
pub struct RequestDecoder<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> RequestDecoder<'a> {
    /// Creates a new request decoder from a buffer.
    pub fn new(data: &'a [u8]) -> Self {
        RequestDecoder { data, pos: 0 }
    }

    /// Reads a single byte.
    fn read_byte(&mut self) -> Result<u8> {
        if self.pos >= self.data.len() {
            return Err(MemoryError::Other("unexpected end of data".into()));
        }
        let b = self.data[self.pos];
        self.pos += 1;
        Ok(b)
    }

    /// Reads a u64 (8 bytes, little-endian).
    fn read_u64(&mut self) -> Result<u64> {
        if self.pos + 8 > self.data.len() {
            return Err(MemoryError::Other("unexpected end of data".into()));
        }
        let bytes = &self.data[self.pos..self.pos + 8];
        self.pos += 8;
        let mut arr = [0u8; 8];
        arr.copy_from_slice(bytes);
        Ok(u64::from_le_bytes(arr))
    }

    /// Reads a u32 (4 bytes, little-endian).
    fn read_u32(&mut self) -> Result<u32> {
        if self.pos + 4 > self.data.len() {
            return Err(MemoryError::Other("unexpected end of data".into()));
        }
        let bytes = &self.data[self.pos..self.pos + 4];
        self.pos += 4;
        let mut arr = [0u8; 4];
        arr.copy_from_slice(bytes);
        Ok(u32::from_le_bytes(arr))
    }

    /// Reads a u16 (2 bytes, little-endian).
    fn read_u16(&mut self) -> Result<u16> {
        if self.pos + 2 > self.data.len() {
            return Err(MemoryError::Other("unexpected end of data".into()));
        }
        let bytes = &self.data[self.pos..self.pos + 2];
        self.pos += 2;
        let mut arr = [0u8; 2];
        arr.copy_from_slice(bytes);
        Ok(u16::from_le_bytes(arr))
    }

    /// Reads a variable-length byte buffer.
    fn read_bytes(&mut self, len: usize) -> Result<&'a [u8]> {
        if self.pos + len > self.data.len() {
            return Err(MemoryError::Other("unexpected end of data".into()));
        }
        let bytes = &self.data[self.pos..self.pos + len];
        self.pos += len;
        Ok(bytes)
    }

    /// Returns the message type.
    pub fn message_type(&self) -> Result<MemoryRequestType> {
        if self.data.is_empty() {
            return Err(MemoryError::Other("empty request".into()));
        }

        match self.data[0] {
            0x01 => Ok(MemoryRequestType::Allocate),
            0x02 => Ok(MemoryRequestType::Read),
            0x03 => Ok(MemoryRequestType::Write),
            0x04 => Ok(MemoryRequestType::Mount),
            _ => Err(MemoryError::Other("invalid request type".into())),
        }
    }

    /// Decodes a mem_alloc request.
    pub fn decode_allocate(&mut self) -> Result<(u64, u64, AllocFlags)> {
        self.pos = 0;
        let _msg_type = self.read_byte()?;
        let _seq = self.read_byte()?;

        let size = self.read_u64()?;
        let alignment = self.read_u64()?;
        let flags = AllocFlags::from_bits(self.read_u32()?);

        Ok((size, alignment, flags))
    }

    /// Decodes a mem_read request.
    pub fn decode_read(&mut self) -> Result<(MemHandle, u64, u64)> {
        self.pos = 0;
        let _msg_type = self.read_byte()?;
        let _seq = self.read_byte()?;

        let handle = MemHandle::new(self.read_u64()?);
        let offset = self.read_u64()?;
        let size = self.read_u64()?;

        Ok((handle, offset, size))
    }

    /// Decodes a mem_write request.
    pub fn decode_write(&mut self) -> Result<(MemHandle, u64, u64, Vec<u8>)> {
        self.pos = 0;
        let _msg_type = self.read_byte()?;
        let _seq = self.read_byte()?;

        let handle = MemHandle::new(self.read_u64()?);
        let offset = self.read_u64()?;
        let size = self.read_u64()?;

        if size as usize > MAX_DATA_BUFFER_LEN {
            return Err(MemoryError::Other("data too large".into()));
        }

        let data = self.read_bytes(size as usize)?;
        Ok((handle, offset, size, data.to_vec()))
    }

    /// Decodes a mem_mount request.
    pub fn decode_mount(&mut self) -> Result<(MountSource, String, MountFlags)> {
        self.pos = 0;
        let _msg_type = self.read_byte()?;
        let _seq = self.read_byte()?;

        let source_type = self.read_byte()?;

        let source_len = self.read_u16()? as usize;
        if source_len > MAX_STRING_LEN {
            return Err(MemoryError::Other("source too long".into()));
        }
        let source_bytes = self.read_bytes(source_len)?;
        let source_str =
            core::str::from_utf8(source_bytes).map_err(|_| {
                MemoryError::Other("invalid source encoding".into())
            })?;

        let mount_point_len = self.read_u16()? as usize;
        if mount_point_len > MAX_STRING_LEN {
            return Err(MemoryError::Other("mount point too long".into()));
        }
        let mount_point_bytes = self.read_bytes(mount_point_len)?;
        let mount_point = core::str::from_utf8(mount_point_bytes)
            .map_err(|_| MemoryError::Other("invalid mount point encoding".into()))?
            .to_string();

        let flags = MountFlags::from_bits(self.read_u32()?);

        let source = match source_type {
            0 => MountSource::LocalPath(source_str.to_string()),
            1 => MountSource::RemoteUrl(source_str.to_string()),
            2 => MountSource::SharedRegion(source_str.to_string()),
            3 => MountSource::CrewReplica(source_str.to_string()),
            _ => return Err(MemoryError::Other("invalid source type".into())),
        };

        Ok((source, mount_point, flags))
    }
}

/// Encoder for memory responses (write-only binary builder).
pub struct ResponseEncoder {
    buffer: Vec<u8>,
}

impl ResponseEncoder {
    /// Creates a new response encoder.
    pub fn new() -> Self {
        ResponseEncoder {
            buffer: Vec::new(),
        }
    }

    /// Encodes a successful allocation response.
    pub fn encode_allocated(&mut self, handle: MemHandle) -> Result<SerializedMemoryResponse> {
        self.buffer.clear();

        self.buffer.push(MemoryResponseType::Allocated as u8);
        self.buffer.push(0); // sequence

        // Handle (8 bytes)
        self.buffer.extend_from_slice(&handle.as_u64().to_le_bytes());

        if self.buffer.len() > MAX_RESPONSE_SIZE {
            return Err(MemoryError::Other("response too large".into()));
        }

        Ok(SerializedMemoryResponse {
            bytes: self.buffer.clone(),
        })
    }

    /// Encodes a successful read response.
    pub fn encode_read_data(&mut self, data: &[u8]) -> Result<SerializedMemoryResponse> {
        if data.len() > MAX_DATA_BUFFER_LEN {
            return Err(MemoryError::Other("data too large".into()));
        }

        self.buffer.clear();

        self.buffer.push(MemoryResponseType::ReadData as u8);
        self.buffer.push(0); // sequence

        // Data length (4 bytes)
        self.buffer
            .extend_from_slice(&(data.len() as u32).to_le_bytes());

        // Data
        self.buffer.extend_from_slice(data);

        if self.buffer.len() > MAX_RESPONSE_SIZE {
            return Err(MemoryError::Other("response too large".into()));
        }

        Ok(SerializedMemoryResponse {
            bytes: self.buffer.clone(),
        })
    }

    /// Encodes a successful write response.
    pub fn encode_write_ack(&mut self) -> Result<SerializedMemoryResponse> {
        self.buffer.clear();

        self.buffer.push(MemoryResponseType::WriteAck as u8);
        self.buffer.push(0); // sequence

        Ok(SerializedMemoryResponse {
            bytes: self.buffer.clone(),
        })
    }

    /// Encodes a successful mount response.
    pub fn encode_mounted(&mut self, mount_handle: MountHandle) -> Result<SerializedMemoryResponse> {
        self.buffer.clear();

        self.buffer.push(MemoryResponseType::Mounted as u8);
        self.buffer.push(0); // sequence

        // Mount handle (8 bytes)
        self.buffer
            .extend_from_slice(&mount_handle.as_u64().to_le_bytes());

        Ok(SerializedMemoryResponse {
            bytes: self.buffer.clone(),
        })
    }

    /// Encodes an error response.
    pub fn encode_error(&mut self, error_msg: &str) -> Result<SerializedMemoryResponse> {
        if error_msg.len() > MAX_STRING_LEN {
            return Err(MemoryError::Other("error message too long".into()));
        }

        self.buffer.clear();

        self.buffer.push(MemoryResponseType::Error as u8);
        self.buffer.push(0); // sequence

        // Error message length (2 bytes) + data
        self.buffer
            .extend_from_slice(&(error_msg.len() as u16).to_le_bytes());
        self.buffer.extend_from_slice(error_msg.as_bytes());

        Ok(SerializedMemoryResponse {
            bytes: self.buffer.clone(),
        })
    }
}

/// Decoder for memory responses.
pub struct ResponseDecoder<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> ResponseDecoder<'a> {
    /// Creates a new response decoder.
    pub fn new(data: &'a [u8]) -> Self {
        ResponseDecoder { data, pos: 0 }
    }

    /// Reads a single byte.
    fn read_byte(&mut self) -> Result<u8> {
        if self.pos >= self.data.len() {
            return Err(MemoryError::Other("unexpected end of data".into()));
        }
        let b = self.data[self.pos];
        self.pos += 1;
        Ok(b)
    }

    /// Reads a u64.
    fn read_u64(&mut self) -> Result<u64> {
        if self.pos + 8 > self.data.len() {
            return Err(MemoryError::Other("unexpected end of data".into()));
        }
        let bytes = &self.data[self.pos..self.pos + 8];
        self.pos += 8;
        let mut arr = [0u8; 8];
        arr.copy_from_slice(bytes);
        Ok(u64::from_le_bytes(arr))
    }

    /// Reads a u32.
    fn read_u32(&mut self) -> Result<u32> {
        if self.pos + 4 > self.data.len() {
            return Err(MemoryError::Other("unexpected end of data".into()));
        }
        let bytes = &self.data[self.pos..self.pos + 4];
        self.pos += 4;
        let mut arr = [0u8; 4];
        arr.copy_from_slice(bytes);
        Ok(u32::from_le_bytes(arr))
    }

    /// Reads a u16.
    fn read_u16(&mut self) -> Result<u16> {
        if self.pos + 2 > self.data.len() {
            return Err(MemoryError::Other("unexpected end of data".into()));
        }
        let bytes = &self.data[self.pos..self.pos + 2];
        self.pos += 2;
        let mut arr = [0u8; 2];
        arr.copy_from_slice(bytes);
        Ok(u16::from_le_bytes(arr))
    }

    /// Reads variable-length bytes.
    fn read_bytes(&mut self, len: usize) -> Result<&'a [u8]> {
        if self.pos + len > self.data.len() {
            return Err(MemoryError::Other("unexpected end of data".into()));
        }
        let bytes = &self.data[self.pos..self.pos + len];
        self.pos += len;
        Ok(bytes)
    }

    /// Returns the message type.
    pub fn message_type(&self) -> Result<MemoryResponseType> {
        if self.data.is_empty() {
            return Err(MemoryError::Other("empty response".into()));
        }

        match self.data[0] {
            0x80 => Ok(MemoryResponseType::Allocated),
            0x81 => Ok(MemoryResponseType::ReadData),
            0x82 => Ok(MemoryResponseType::WriteAck),
            0x83 => Ok(MemoryResponseType::Mounted),
            0xFF => Ok(MemoryResponseType::Error),
            _ => Err(MemoryError::Other("invalid response type".into())),
        }
    }

    /// Decodes a successful allocation response.
    pub fn decode_allocated(&mut self) -> Result<MemHandle> {
        self.pos = 0;
        let _msg_type = self.read_byte()?;
        let _seq = self.read_byte()?;

        let handle = MemHandle::new(self.read_u64()?);
        Ok(handle)
    }

    /// Decodes a successful read response.
    pub fn decode_read_data(&mut self) -> Result<Vec<u8>> {
        self.pos = 0;
        let _msg_type = self.read_byte()?;
        let _seq = self.read_byte()?;

        let len = self.read_u32()? as usize;
        if len > MAX_DATA_BUFFER_LEN {
            return Err(MemoryError::Other("data too large".into()));
        }

        let data = self.read_bytes(len)?;
        Ok(data.to_vec())
    }

    /// Decodes a mount response.
    pub fn decode_mounted(&mut self) -> Result<MountHandle> {
        self.pos = 0;
        let _msg_type = self.read_byte()?;
        let _seq = self.read_byte()?;

        let handle = MountHandle::new(self.read_u64()?);
        Ok(handle)
    }

    /// Decodes an error response.
    pub fn decode_error(&mut self) -> Result<String> {
        self.pos = 0;
        let _msg_type = self.read_byte()?;
        let _seq = self.read_byte()?;

        let len = self.read_u16()? as usize;
        if len > MAX_STRING_LEN {
            return Err(MemoryError::Other("error message too long".into()));
        }

        let msg_bytes = self.read_bytes(len)?;
        let msg = core::str::from_utf8(msg_bytes)
            .map_err(|_| MemoryError::Other("invalid error message encoding".into()))?
            .to_string();

        Ok(msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::string::ToString;

    #[test]
    fn test_encode_decode_allocate() {
        let mut encoder = RequestEncoder::new();
        let serialized = encoder
            .encode_allocate(1024, 8, AllocFlags::ZERO_INIT)
            .unwrap();

        assert_eq!(serialized.message_type().unwrap(), MemoryRequestType::Allocate);

        let mut decoder = RequestDecoder::new(&serialized.bytes);
        let (size, alignment, flags) = decoder.decode_allocate().unwrap();

        assert_eq!(size, 1024);
        assert_eq!(alignment, 8);
        assert!(flags.contains(AllocFlags::ZERO_INIT));
    }

    #[test]
    fn test_encode_decode_read() {
        let mut encoder = RequestEncoder::new();
        let handle = MemHandle::new(42);
        let serialized = encoder.encode_read(handle, 100, 256).unwrap();

        assert_eq!(serialized.message_type().unwrap(), MemoryRequestType::Read);

        let mut decoder = RequestDecoder::new(&serialized.bytes);
        let (h, offset, size) = decoder.decode_read().unwrap();

        assert_eq!(h.as_u64(), 42);
        assert_eq!(offset, 100);
        assert_eq!(size, 256);
    }

    #[test]
    fn test_encode_decode_write() {
        let mut encoder = RequestEncoder::new();
        let handle = MemHandle::new(42);
        let data = b"hello world";
        let serialized = encoder
            .encode_write(handle, 50, data.len() as u64, data)
            .unwrap();

        assert_eq!(serialized.message_type().unwrap(), MemoryRequestType::Write);

        let mut decoder = RequestDecoder::new(&serialized.bytes);
        let (h, offset, size, decoded_data) = decoder.decode_write().unwrap();

        assert_eq!(h.as_u64(), 42);
        assert_eq!(offset, 50);
        assert_eq!(size, data.len() as u64);
        assert_eq!(&decoded_data[..], data);
    }

    #[test]
    fn test_encode_decode_mount() {
        let mut encoder = RequestEncoder::new();
        let source = MountSource::LocalPath("/data/corpus".into());
        let serialized = encoder
            .encode_mount(&source, "/mnt/knowledge", MountFlags::INDEXED)
            .unwrap();

        assert_eq!(serialized.message_type().unwrap(), MemoryRequestType::Mount);

        let mut decoder = RequestDecoder::new(&serialized.bytes);
        let (s, mp, flags) = decoder.decode_mount().unwrap();

        assert_eq!(s, source);
        assert_eq!(mp, "/mnt/knowledge");
        assert!(flags.contains(MountFlags::INDEXED));
    }

    #[test]
    fn test_response_encode_decode_allocated() {
        let mut encoder = ResponseEncoder::new();
        let handle = MemHandle::new(100);
        let serialized = encoder.encode_allocated(handle).unwrap();

        assert_eq!(serialized.message_type().unwrap(), MemoryResponseType::Allocated);

        let mut decoder = ResponseDecoder::new(&serialized.bytes);
        let h = decoder.decode_allocated().unwrap();

        assert_eq!(h.as_u64(), 100);
    }

    #[test]
    fn test_response_encode_decode_read_data() {
        let mut encoder = ResponseEncoder::new();
        let data = b"test data";
        let serialized = encoder.encode_read_data(data).unwrap();

        assert_eq!(serialized.message_type().unwrap(), MemoryResponseType::ReadData);

        let mut decoder = ResponseDecoder::new(&serialized.bytes);
        let decoded = decoder.decode_read_data().unwrap();

        assert_eq!(&decoded[..], data);
    }

    #[test]
    fn test_response_encode_decode_error() {
        let mut encoder = ResponseEncoder::new();
        let error_msg = "allocation failed";
        let serialized = encoder.encode_error(error_msg).unwrap();

        assert_eq!(serialized.message_type().unwrap(), MemoryResponseType::Error);

        let mut decoder = ResponseDecoder::new(&serialized.bytes);
        let msg = decoder.decode_error().unwrap();

        assert_eq!(msg, error_msg);
    }
}
