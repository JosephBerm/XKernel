// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! IPC message handler loop for Memory Manager syscalls.
//!
//! This module implements the request/response message handling pipeline that
//! receives serialized syscall requests from Cognitive Threads, deserializes them,
//! dispatches to appropriate handlers, and returns serialized responses.
//!
//! See Engineering Plan § 4.1.0: IPC Handler Implementation (Week 5).

use alloc::string::String;
use alloc::vec::Vec;
use crate::error::{MemoryError, Result};
use crate::mem_syscall_interface::{
use core::sync::atomic::{AtomicU64, Ordering};

    MemHandle, MountHandle, AllocFlags, MountFlags, MountSource,
};
use crate::mem_serialization::{
    SerializedMemoryRequest, SerializedMemoryResponse, MemoryRequestType,
    MemoryResponseType, RequestDecoder, ResponseEncoder,
};

/// Result of IPC request handling.
#[derive(Clone, Debug)]
pub enum IpcHandlerResult {
    /// Request successfully processed, response generated.
    Success(SerializedMemoryResponse),
    /// Request failed with an error message.
    Error(String),
    /// Request was invalid or malformed.
    InvalidRequest,
}

/// Errors from IPC operations (POSIX-compatible error codes).
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IpcErrorCode {
    /// EIO (5): I/O error during memory operation.
    EIO = 5,
    /// EACCES (13): Permission denied (capability check failed).
    EACCES = 13,
    /// ENOMEM (12): Out of memory.
    ENOMEM = 12,
    /// EINVAL (22): Invalid argument.
    EINVAL = 22,
}

/// Handler context for IPC message processing.
///
/// Maintains state for the message handling loop including capability
/// validation and error tracking.
/// See Engineering Plan § 4.1.0: IPC Handler.
pub struct IpcHandler {
    /// Handler identifier for logging/debugging.
    handler_id: String,
    /// Maximum requests to process in a batch.
    max_batch_size: u32,
    /// Track consecutive errors.
    error_count: u32,
}

impl IpcHandler {
    /// Creates a new IPC handler instance.
    ///
    /// # Arguments
    ///
    /// * `handler_id` - Unique identifier for this handler
    /// * `max_batch_size` - Maximum requests per batch (default: 100)
    pub fn new(handler_id: impl Into<String>, max_batch_size: u32) -> Self {
        IpcHandler {
            handler_id: handler_id.into(),
            max_batch_size,
            error_count: 0,
        }
    }

    /// Processes a single IPC request.
    ///
    /// This is the main entry point for handling incoming syscall messages.
    /// Deserializes the request, validates it, dispatches to the appropriate
    /// handler, and returns a serialized response.
    ///
    /// # Arguments
    ///
    /// * `request` - Serialized memory request
    /// * `capability_token` - Capability token for access control
    ///
    /// # Returns
    ///
    /// Serialized response or error
    ///
    /// # Error Handling
    ///
    /// Per engineering plan, all errors are returned as Result rather than panics.
    /// Invalid requests return Error variant; internal failures return Err.
    pub fn handle_request(
        &mut self,
        request: &SerializedMemoryRequest,
        capability_token: &str,
    ) -> Result<SerializedMemoryResponse> {
        // Parse message type
        let msg_type = match request.message_type() {
            Ok(t) => t,
            Err(_) => {
                self.error_count += 1;
                return self.encode_error_response("invalid request type");
            }
        };

        // Validate capability (stub implementation)
        if !self.validate_capability(capability_token, msg_type) {
            self.error_count += 1;
            return self.encode_error_response("capability denied");
        }

        // Dispatch based on message type
        let response = match msg_type {
            MemoryRequestType::Allocate => self.handle_allocate(request),
            MemoryRequestType::Read => self.handle_read(request),
            MemoryRequestType::Write => self.handle_write(request),
            MemoryRequestType::Mount => self.handle_mount(request),
        };

        match response {
            Ok(resp) => {
                self.error_count = 0; // Reset error count on success
                Ok(resp)
            }
            Err(e) => {
                self.error_count += 1;
                self.encode_error_response(&format!("{}", e))
            }
        }
    }

    /// Handles mem_alloc syscall.
    fn handle_allocate(&self, request: &SerializedMemoryRequest) -> Result<SerializedMemoryResponse> {
        let mut decoder = RequestDecoder::new(&request.bytes);
        let (size, alignment, flags) = decoder.decode_allocate()
            .map_err(|_| MemoryError::Other("failed to decode allocate".into()))?;

        // Validate parameters
        if size == 0 {
            return self.encode_error_response("allocation size must be > 0");
        }

        if alignment == 0 || (alignment & (alignment - 1)) != 0 {
            return self.encode_error_response("alignment must be power of 2");
        }

        // Stub: Actual allocation happens in memory manager
        // Generate a handle (in real system, this would be from allocator)
        let handle = MemHandle::new(self.generate_handle());

        // Encode success response
        let mut encoder = ResponseEncoder::new();
        encoder.encode_allocated(handle)
            .map_err(|e| MemoryError::Other(format!("encoding failed: {}", e)))
    }

    /// Handles mem_read syscall.
    fn handle_read(&self, request: &SerializedMemoryRequest) -> Result<SerializedMemoryResponse> {
        let mut decoder = RequestDecoder::new(&request.bytes);
        let (handle, offset, size) = decoder.decode_read()
            .map_err(|_| MemoryError::Other("failed to decode read".into()))?;

        // Validate parameters
        if size == 0 {
            // Return empty data for zero-size reads
            let mut encoder = ResponseEncoder::new();
            return encoder.encode_read_data(&[])
                .map_err(|e| MemoryError::Other(format!("encoding failed: {}", e)));
        }

        // Validate handle
        if handle.as_u64() == 0 {
            return self.encode_error_response("invalid memory handle");
        }

        // Stub: Actual read happens in memory manager
        // For now, return dummy data
        let data = alloc::vec![0u8; size as usize];

        let mut encoder = ResponseEncoder::new();
        encoder.encode_read_data(&data)
            .map_err(|e| MemoryError::Other(format!("encoding failed: {}", e)))
    }

    /// Handles mem_write syscall.
    fn handle_write(&self, request: &SerializedMemoryRequest) -> Result<SerializedMemoryResponse> {
        let mut decoder = RequestDecoder::new(&request.bytes);
        let (handle, offset, size, data) = decoder.decode_write()
            .map_err(|_| MemoryError::Other("failed to decode write".into()))?;

        // Validate parameters
        if size == 0 {
            // Zero-size write is valid
            let mut encoder = ResponseEncoder::new();
            return encoder.encode_write_ack()
                .map_err(|e| MemoryError::Other(format!("encoding failed: {}", e)));
        }

        if size as usize != data.len() {
            return self.encode_error_response("write size mismatch");
        }

        // Validate handle
        if handle.as_u64() == 0 {
            return self.encode_error_response("invalid memory handle");
        }

        // Stub: Actual write happens in memory manager
        // Validate write would succeed (not read-only, bounds check, etc.)

        let mut encoder = ResponseEncoder::new();
        encoder.encode_write_ack()
            .map_err(|e| MemoryError::Other(format!("encoding failed: {}", e)))
    }

    /// Handles mem_mount syscall.
    fn handle_mount(&self, request: &SerializedMemoryRequest) -> Result<SerializedMemoryResponse> {
        let mut decoder = RequestDecoder::new(&request.bytes);
        let (source, mount_point, flags) = decoder.decode_mount()
            .map_err(|_| MemoryError::Other("failed to decode mount".into()))?;

        // Validate parameters
        if mount_point.is_empty() {
            return self.encode_error_response("mount point must not be empty");
        }

        // Validate mount source exists (stub)
        match &source {
            MountSource::LocalPath(path) => {
                if path.is_empty() {
                    return self.encode_error_response("local path must not be empty");
                }
            }
            MountSource::RemoteUrl(url) => {
                if url.is_empty() {
                    return self.encode_error_response("remote URL must not be empty");
                }
            }
            MountSource::SharedRegion(id) => {
                if id.is_empty() {
                    return self.encode_error_response("shared region ID must not be empty");
                }
            }
            MountSource::CrewReplica(endpoint) => {
                if endpoint.is_empty() {
                    return self.encode_error_response("crew replica endpoint must not be empty");
                }
            }
        }

        // Stub: Actual mount happens in memory manager
        let mount_handle = MountHandle::new(self.generate_handle());

        let mut encoder = ResponseEncoder::new();
        encoder.encode_mounted(mount_handle)
            .map_err(|e| MemoryError::Other(format!("encoding failed: {}", e)))
    }

    /// Validates a capability token against the requested operation.
    ///
    /// This is a stub implementation; real capability checking would involve:
    /// - Looking up the capability in a capability table
    /// - Checking if it grants access to the requested operation
    /// - Checking if it's been revoked
    /// - Validating scope restrictions (region, size limit, etc.)
    fn validate_capability(&self, _capability_token: &str, _msg_type: MemoryRequestType) -> bool {
        // Stub: In real system, validate against capability database
        // For now, accept all capabilities as valid
        true
    }

    /// Encodes an error response.
    fn encode_error_response(&self, error_msg: &str) -> Result<SerializedMemoryResponse> {
        let mut encoder = ResponseEncoder::new();
        encoder.encode_error(error_msg)
            .map_err(|e| MemoryError::Other(format!("encoding failed: {}", e)))
    }

    /// Generates a unique handle (stub).
    ///
    /// In a real system, this would allocate from a handle registry and
    /// ensure uniqueness across all outstanding handles.
    fn generate_handle(&self) -> u64 {
        // Stub: Generate monotonically increasing handles
        // In real system, would be from a proper allocator
        static HANDLE_COUNTER: AtomicU64 = AtomicU64::new(1);
        HANDLE_COUNTER.fetch_add(1, Ordering::SeqCst)
    }

    /// Returns the number of consecutive errors since last success.
    pub fn error_count(&self) -> u32 {
        self.error_count
    }

    /// Resets the error count (e.g., after handling an error gracefully).
    pub fn reset_error_count(&mut self) {
        self.error_count = 0;
    }

    /// Returns whether error count indicates a failure state.
    pub fn is_in_error_state(&self) -> bool {
        self.error_count >= 10
    }
}

/// Request batch processor for handling multiple IPC messages.
pub struct IpcBatchProcessor {
    handler: IpcHandler,
    /// Maximum time to process batch in microseconds (0 = unlimited).
    max_batch_time_us: u64,
}

impl IpcBatchProcessor {
    /// Creates a new batch processor.
    pub fn new(handler: IpcHandler, max_batch_time_us: u64) -> Self {
        IpcBatchProcessor {
            handler,
            max_batch_time_us,
        }
    }

    /// Processes a batch of requests.
    ///
    /// # Arguments
    ///
    /// * `requests` - Vector of serialized requests with capability tokens
    /// * `responses` - Output vector for responses (same length as requests)
    ///
    /// # Returns
    ///
    /// Number of successfully processed requests
    pub fn process_batch(
        &mut self,
        requests: &[(SerializedMemoryRequest, String)],
        responses: &mut Vec<Result<SerializedMemoryResponse>>,
    ) -> usize {
        responses.clear();
        responses.reserve(requests.len());

        for (request, capability_token) in requests {
            let result = self.handler.handle_request(request, capability_token);
            responses.push(result);

            // Check error state
            if self.handler.is_in_error_state() {
                break;
            }
        }

        responses.len()
    }

    /// Returns a mutable reference to the underlying handler.
    pub fn handler_mut(&mut self) -> &mut IpcHandler {
        &mut self.handler
    }

    /// Returns a reference to the underlying handler.
    pub fn handler(&self) -> &IpcHandler {
        &self.handler
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mem_serialization::RequestEncoder;
use alloc::format;
use alloc::string::ToString;
use alloc::vec;

    #[test]
    fn test_ipc_handler_creation() {
        let handler = IpcHandler::new("test-handler", 100);
        assert_eq!(handler.error_count(), 0);
        assert!(!handler.is_in_error_state());
    }

    #[test]
    fn test_handle_allocate_request() {
        let mut handler = IpcHandler::new("test", 100);
        let mut encoder = RequestEncoder::new();
        let request = encoder
            .encode_allocate(1024, 8, AllocFlags::ZERO_INIT)
            .unwrap();

        let response = handler
            .handle_request(&request, "dummy_capability")
            .unwrap();

        assert_eq!(
            response.message_type().unwrap(),
            MemoryResponseType::Allocated
        );
    }

    #[test]
    fn test_handle_read_request() {
        let mut handler = IpcHandler::new("test", 100);
        let mut encoder = RequestEncoder::new();
        let request = encoder
            .encode_read(MemHandle::new(1), 0, 256)
            .unwrap();

        let response = handler
            .handle_request(&request, "dummy_capability")
            .unwrap();

        assert_eq!(
            response.message_type().unwrap(),
            MemoryResponseType::ReadData
        );
    }

    #[test]
    fn test_handle_write_request() {
        let mut handler = IpcHandler::new("test", 100);
        let mut encoder = RequestEncoder::new();
        let data = b"test data";
        let request = encoder
            .encode_write(MemHandle::new(1), 0, data.len() as u64, data)
            .unwrap();

        let response = handler
            .handle_request(&request, "dummy_capability")
            .unwrap();

        assert_eq!(
            response.message_type().unwrap(),
            MemoryResponseType::WriteAck
        );
    }

    #[test]
    fn test_handle_mount_request() {
        let mut handler = IpcHandler::new("test", 100);
        let mut encoder = RequestEncoder::new();
        let source = MountSource::LocalPath("/data".into());
        let request = encoder
            .encode_mount(&source, "/mnt", MountFlags::READ_ONLY)
            .unwrap();

        let response = handler
            .handle_request(&request, "dummy_capability")
            .unwrap();

        assert_eq!(
            response.message_type().unwrap(),
            MemoryResponseType::Mounted
        );
    }

    #[test]
    fn test_error_count_tracking() {
        let mut handler = IpcHandler::new("test", 100);
        assert_eq!(handler.error_count(), 0);

        // Create an invalid request
        let invalid_request = SerializedMemoryRequest {
            bytes: alloc::vec![0xFF], // Invalid type
        };

        let _ = handler.handle_request(&invalid_request, "dummy");
        assert!(handler.error_count() > 0);
    }

    #[test]
    fn test_batch_processor() {
        let handler = IpcHandler::new("test", 100);
        let mut processor = IpcBatchProcessor::new(handler, 0);

        let mut encoder = RequestEncoder::new();
        let req1 = encoder.encode_allocate(1024, 8, AllocFlags::NONE).unwrap();
        let req2 = encoder.encode_read(MemHandle::new(1), 0, 256).unwrap();

        let requests = alloc::vec![
            (req1, "cap1".to_string()),
            (req2, "cap2".to_string()),
        ];

        let mut responses = Vec::new();
        let count = processor.process_batch(&requests, &mut responses);

        assert_eq!(count, 2);
        assert_eq!(responses.len(), 2);
    }
}
