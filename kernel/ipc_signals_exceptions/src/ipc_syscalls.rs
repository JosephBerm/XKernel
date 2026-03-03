// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! # Kernel Syscall Handlers for IPC
//!
//! This module implements the kernel syscall handlers for inter-process communication.
//! These syscalls provide the primary interface for agents to interact with the IPC
//! subsystem, including channel creation, message sending/receiving, and cleanup.
//!
//! ## Syscall Operations
//!
//! - `sys_chan_open`: Create a new IPC channel
//! - `sys_chan_send`: Send a message through a channel
//! - `sys_chan_recv`: Receive a message from a channel
//! - `sys_chan_close`: Close and destroy a channel
//!
//! ## References
//!
//! - Engineering Plan § 5.3.5 (IPC Syscalls)

use crate::channel_registry::ChannelRegistry;
use crate::error::{CsError, IpcError, Result};
use crate::ids::ChannelID;
use crate::message::{RequestMessage, ResponseMessage};
use crate::request_response::{ChannelConfig, RequestId};
use alloc::string::String;
use serde::{Deserialize, Serialize};

/// Syscall flags for channel operations.
///
/// Flags controlling the behavior of channel syscalls.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyscallFlags(u32);

impl SyscallFlags {
    /// Flag: Use zero-copy for large messages.
    const ZERO_COPY: u32 = 0x01;
    /// Flag: Make the operation non-blocking.
    const NONBLOCKING: u32 = 0x02;

    /// Create empty flags.
    #[inline]
    pub fn new() -> Self {
        Self(0)
    }

    /// Set zero-copy flag.
    #[inline]
    pub fn set_zero_copy(mut self) -> Self {
        self.0 |= Self::ZERO_COPY;
        self
    }

    /// Check zero-copy flag.
    #[inline]
    pub fn zero_copy(&self) -> bool {
        (self.0 & Self::ZERO_COPY) != 0
    }

    /// Set non-blocking flag.
    #[inline]
    pub fn set_nonblocking(mut self) -> Self {
        self.0 |= Self::NONBLOCKING;
        self
    }

    /// Check non-blocking flag.
    #[inline]
    pub fn nonblocking(&self) -> bool {
        (self.0 & Self::NONBLOCKING) != 0
    }

    /// Get the raw flags value.
    #[inline]
    pub fn as_u32(&self) -> u32 {
        self.0
    }
}

impl Default for SyscallFlags {
    fn default() -> Self {
        Self::new()
    }
}

/// Message received from a syscall operation.
///
/// Represents either a request message or a response message.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ReceivedMessage {
    /// A request message with its ID.
    Request(RequestId, RequestMessage),

    /// A response message with its ID.
    Response(RequestId, ResponseMessage),
}

impl ReceivedMessage {
    /// Get the request ID.
    pub fn request_id(&self) -> RequestId {
        match self {
            ReceivedMessage::Request(id, _) => *id,
            ReceivedMessage::Response(id, _) => *id,
        }
    }
}

/// IPC syscall handler.
///
/// Manages syscall operations on the kernel's behalf. This would typically
/// be part of the kernel's syscall dispatcher.
///
/// See Engineering Plan § 5.3.5 (IPC Syscalls)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IpcSyscallHandler {
    /// The channel registry being managed.
    registry: ChannelRegistry,
}

impl IpcSyscallHandler {
    /// Create a new syscall handler.
    pub fn new() -> Self {
        Self {
            registry: ChannelRegistry::new(),
        }
    }

    /// sys_chan_open - Create and open a new IPC channel.
    ///
    /// Opens a new request-response channel with the given configuration.
    /// The channel is created with the calling context as the requestor
    /// and the target context as the responder.
    ///
    /// Arguments:
    /// - requestor_ct: Caller's context tag
    /// - responder_ct: Target responder context tag
    /// - flags: Syscall flags
    /// - config: Channel configuration
    ///
    /// Returns:
    /// - ChannelId on success
    /// - IpcError on failure
    pub fn sys_chan_open(
        &mut self,
        requestor_ct: u64,
        responder_ct: u64,
        _flags: SyscallFlags,
        config: ChannelConfig,
    ) -> Result<ChannelID> {
        // Capability check: caller must be able to create channels
        if requestor_ct == 0 || responder_ct == 0 {
            return Err(CsError::Ipc(IpcError::CapabilityVerificationFailed));
        }

        let channel_id = ChannelID::new();
        self.registry
            .create_channel_request_response(channel_id, requestor_ct, responder_ct, config)?;

        Ok(channel_id)
    }

    /// sys_chan_send - Send a message through a channel.
    ///
    /// Sends a request message to the responder endpoint.
    /// Returns a request ID for correlation with the response.
    ///
    /// Arguments:
    /// - chan_id: Channel to send through
    /// - message: Message to send
    /// - _timeout_ms: Timeout in milliseconds (for future use)
    /// - _flags: Syscall flags
    ///
    /// Returns:
    /// - RequestId on success
    /// - IpcError on failure
    pub fn sys_chan_send(
        &mut self,
        chan_id: ChannelID,
        message: RequestMessage,
        _timeout_ms: u64,
        _flags: SyscallFlags,
    ) -> Result<RequestId> {
        // Capability check: caller must have access to the channel
        if !self.registry.contains_channel(chan_id) {
            return Err(CsError::Ipc(IpcError::InvalidChannel));
        }

        let channel = self.registry.lookup_channel_mut(chan_id)?;

        if let crate::channel_registry::RegisteredChannel::RequestResponse(chan) = channel {
            chan.send_request(message)
        } else {
            Err(CsError::Ipc(IpcError::Other(
                String::from("channel is not a request-response channel"),
            )))
        }
    }

    /// sys_chan_recv - Receive a message from a channel.
    ///
    /// Receives a request message that was sent through the channel.
    ///
    /// Arguments:
    /// - chan_id: Channel to receive from
    /// - _timeout_ms: Timeout in milliseconds (for future use)
    /// - _flags: Syscall flags
    ///
    /// Returns:
    /// - ReceivedMessage on success
    /// - IpcError on failure
    pub fn sys_chan_recv(
        &mut self,
        chan_id: ChannelID,
        _timeout_ms: u64,
        _flags: SyscallFlags,
    ) -> Result<ReceivedMessage> {
        // Capability check: caller must have access to the channel
        if !self.registry.contains_channel(chan_id) {
            return Err(CsError::Ipc(IpcError::InvalidChannel));
        }

        let channel = self.registry.lookup_channel_mut(chan_id)?;

        if let crate::channel_registry::RegisteredChannel::RequestResponse(chan) = channel {
            let (request_id, message) = chan.recv_request()?;
            Ok(ReceivedMessage::Request(request_id, message))
        } else {
            Err(CsError::Ipc(IpcError::Other(
                String::from("channel is not a request-response channel"),
            )))
        }
    }

    /// sys_chan_response - Send a response to a request.
    ///
    /// Sends a response message back to the requester, matching the request ID.
    ///
    /// Arguments:
    /// - chan_id: Channel to send through
    /// - request_id: Request ID being responded to
    /// - response: Response message
    /// - _flags: Syscall flags
    ///
    /// Returns:
    /// - Ok(()) on success
    /// - IpcError on failure
    pub fn sys_chan_response(
        &mut self,
        chan_id: ChannelID,
        request_id: RequestId,
        response: ResponseMessage,
        _flags: SyscallFlags,
    ) -> Result<()> {
        // Capability check: caller must have access to the channel
        if !self.registry.contains_channel(chan_id) {
            return Err(CsError::Ipc(IpcError::InvalidChannel));
        }

        let channel = self.registry.lookup_channel_mut(chan_id)?;

        if let crate::channel_registry::RegisteredChannel::RequestResponse(chan) = channel {
            chan.send_response(request_id, response)
        } else {
            Err(CsError::Ipc(IpcError::Other(
                String::from("channel is not a request-response channel"),
            )))
        }
    }

    /// sys_chan_close - Close and destroy a channel.
    ///
    /// Closes the channel and removes it from the registry. All pending
    /// requests on this channel are lost.
    ///
    /// Arguments:
    /// - chan_id: Channel to close
    ///
    /// Returns:
    /// - Ok(()) on success
    /// - IpcError on failure
    pub fn sys_chan_close(&mut self, chan_id: ChannelID) -> Result<()> {
        // Capability check: caller must have access to the channel
        if !self.registry.contains_channel(chan_id) {
            return Err(CsError::Ipc(IpcError::InvalidChannel));
        }

        self.registry.destroy_channel(chan_id)
    }

    /// Get the channel registry.
    pub fn registry(&self) -> &ChannelRegistry {
        &self.registry
    }

    /// Get mutable access to the channel registry (for testing).
    #[cfg(test)]
    pub fn registry_mut(&mut self) -> &mut ChannelRegistry {
        &mut self.registry
    }
}

impl Default for IpcSyscallHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{MessageFlags, MessageHeader};

    fn create_test_request_message(request_id: u64, payload_size: usize) -> RequestMessage {
        let flags = MessageFlags::new().set_requires_response();
        let header = MessageHeader::new(request_id, 100, 1, 1_000_000_000, flags, payload_size as u32);
        let payload = alloc::vec![42; payload_size];
        RequestMessage::new(header, payload).unwrap()
    }

    fn create_test_response_message(request_id: u64, payload_size: usize) -> ResponseMessage {
        use crate::message::ResponseStatus;
use alloc::vec;
        let flags = MessageFlags::new();
        let header = MessageHeader::new(request_id, 200, 1, 2_000_000_000, flags, payload_size as u32);
        let payload = alloc::vec![99; payload_size];
        ResponseMessage::new(header, ResponseStatus::Success, payload).unwrap()
    }

    #[test]
    fn test_syscall_flags_zero_copy() {
        let flags = SyscallFlags::new().set_zero_copy();
        assert!(flags.zero_copy());
        assert!(!flags.nonblocking());
    }

    #[test]
    fn test_syscall_flags_nonblocking() {
        let flags = SyscallFlags::new().set_nonblocking();
        assert!(!flags.zero_copy());
        assert!(flags.nonblocking());
    }

    #[test]
    fn test_syscall_flags_both() {
        let flags = SyscallFlags::new().set_zero_copy().set_nonblocking();
        assert!(flags.zero_copy());
        assert!(flags.nonblocking());
    }

    #[test]
    fn test_received_message_request() {
        let request_id = RequestId::new(123);
        let request = create_test_request_message(123, 50);
        let received = ReceivedMessage::Request(request_id, request);
        assert_eq!(received.request_id(), request_id);
    }

    #[test]
    fn test_received_message_response() {
        let request_id = RequestId::new(456);
        let response = create_test_response_message(456, 75);
        let received = ReceivedMessage::Response(request_id, response);
        assert_eq!(received.request_id(), request_id);
    }

    #[test]
    fn test_ipc_syscall_handler_creation() {
        let handler = IpcSyscallHandler::new();
        assert_eq!(handler.registry().channel_count(), 0);
    }

    #[test]
    fn test_sys_chan_open() {
        let mut handler = IpcSyscallHandler::new();
        let flags = SyscallFlags::new();
        let config = ChannelConfig::default();

        let result = handler.sys_chan_open(100, 200, flags, config);
        assert!(result.is_ok());

        let chan_id = result.unwrap();
        assert!(handler.registry().contains_channel(chan_id));
    }

    #[test]
    fn test_sys_chan_open_invalid_context() {
        let mut handler = IpcSyscallHandler::new();
        let flags = SyscallFlags::new();
        let config = ChannelConfig::default();

        // Context 0 is invalid
        let result = handler.sys_chan_open(0, 200, flags, config);
        assert!(result.is_err());
    }

    #[test]
    fn test_sys_chan_send() {
        let mut handler = IpcSyscallHandler::new();
        let config = ChannelConfig::default();
        let chan_id = handler
            .sys_chan_open(100, 200, SyscallFlags::new(), config)
            .unwrap();

        let message = create_test_request_message(1, 100);
        let result = handler.sys_chan_send(chan_id, message, 5000, SyscallFlags::new());
        assert!(result.is_ok());
    }

    #[test]
    fn test_sys_chan_send_invalid_channel() {
        let mut handler = IpcSyscallHandler::new();
        let fake_chan_id = ChannelID::new();
        let message = create_test_request_message(1, 100);

        let result = handler.sys_chan_send(fake_chan_id, message, 5000, SyscallFlags::new());
        assert!(result.is_err());
    }

    #[test]
    fn test_sys_chan_recv() {
        let mut handler = IpcSyscallHandler::new();
        let config = ChannelConfig::default();
        let chan_id = handler
            .sys_chan_open(100, 200, SyscallFlags::new(), config)
            .unwrap();

        let message = create_test_request_message(1, 100);
        let req_id = handler
            .sys_chan_send(chan_id, message.clone(), 5000, SyscallFlags::new())
            .unwrap();

        let result = handler.sys_chan_recv(chan_id, 5000, SyscallFlags::new());
        assert!(result.is_ok());

        let received = result.unwrap();
        match received {
            ReceivedMessage::Request(id, _msg) => {
                assert_eq!(id, req_id);
            }
            _ => panic!("Expected request message"),
        }
    }

    #[test]
    fn test_sys_chan_recv_empty() {
        let mut handler = IpcSyscallHandler::new();
        let config = ChannelConfig::default();
        let chan_id = handler
            .sys_chan_open(100, 200, SyscallFlags::new(), config)
            .unwrap();

        let result = handler.sys_chan_recv(chan_id, 5000, SyscallFlags::new());
        assert!(result.is_err());
    }

    #[test]
    fn test_sys_chan_response() {
        let mut handler = IpcSyscallHandler::new();
        let config = ChannelConfig::default();
        let chan_id = handler
            .sys_chan_open(100, 200, SyscallFlags::new(), config)
            .unwrap();

        let message = create_test_request_message(1, 100);
        let req_id = handler
            .sys_chan_send(chan_id, message, 5000, SyscallFlags::new())
            .unwrap();

        // Receive the request
        let _ = handler.sys_chan_recv(chan_id, 5000, SyscallFlags::new()).unwrap();

        // Send a response
        let response = create_test_response_message(req_id.as_u64(), 50);
        let result = handler.sys_chan_response(chan_id, req_id, response, SyscallFlags::new());
        assert!(result.is_ok());
    }

    #[test]
    fn test_sys_chan_close() {
        let mut handler = IpcSyscallHandler::new();
        let config = ChannelConfig::default();
        let chan_id = handler
            .sys_chan_open(100, 200, SyscallFlags::new(), config)
            .unwrap();

        assert!(handler.registry().contains_channel(chan_id));

        let result = handler.sys_chan_close(chan_id);
        assert!(result.is_ok());
        assert!(!handler.registry().contains_channel(chan_id));
    }

    #[test]
    fn test_sys_chan_close_nonexistent() {
        let mut handler = IpcSyscallHandler::new();
        let fake_chan_id = ChannelID::new();

        let result = handler.sys_chan_close(fake_chan_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_full_syscall_roundtrip() {
        let mut handler = IpcSyscallHandler::new();
        let config = ChannelConfig::default();

        // Open channel
        let chan_id = handler
            .sys_chan_open(100, 200, SyscallFlags::new(), config)
            .unwrap();

        // Send request
        let request = create_test_request_message(1, 100);
        let req_id = handler
            .sys_chan_send(chan_id, request, 5000, SyscallFlags::new())
            .unwrap();

        // Receive request
        let received = handler.sys_chan_recv(chan_id, 5000, SyscallFlags::new()).unwrap();
        match received {
            ReceivedMessage::Request(id, _msg) => {
                assert_eq!(id, req_id);
            }
            _ => panic!("Expected request message"),
        }

        // Send response
        let response = create_test_response_message(req_id.as_u64(), 75);
        handler
            .sys_chan_response(chan_id, req_id, response, SyscallFlags::new())
            .unwrap();

        // Close channel
        handler.sys_chan_close(chan_id).unwrap();
        assert!(!handler.registry().contains_channel(chan_id));
    }

    #[test]
    fn test_multiple_channels() {
        let mut handler = IpcSyscallHandler::new();
        let config = ChannelConfig::default();

        let chan_id1 = handler
            .sys_chan_open(100, 200, SyscallFlags::new(), config)
            .unwrap();
        let chan_id2 = handler
            .sys_chan_open(300, 400, SyscallFlags::new(), config)
            .unwrap();

        assert_ne!(chan_id1, chan_id2);
        assert!(handler.registry().contains_channel(chan_id1));
        assert!(handler.registry().contains_channel(chan_id2));
        assert_eq!(handler.registry().channel_count(), 2);
    }

    #[test]
    fn test_syscall_with_flags() {
        let mut handler = IpcSyscallHandler::new();
        let config = ChannelConfig::default();
        let flags = SyscallFlags::new().set_zero_copy();

        let chan_id = handler
            .sys_chan_open(100, 200, flags, config)
            .unwrap();

        let message = create_test_request_message(1, 8192); // Large message
        let result = handler.sys_chan_send(chan_id, message, 5000, flags);
        assert!(result.is_ok());
    }
}
