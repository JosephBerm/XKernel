// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! # Synchronous Request-Response Channel
//!
//! This module implements synchronous request-response channels for the IPC subsystem.
//! A request-response channel pairs a requestor and responder endpoint, maintaining
//! correlation between requests and responses using request IDs.
//!
//! ## Channel Flow
//!
//! 1. Requestor sends request via send_request() - returns RequestId
//! 2. Responder receives request via recv_request()
//! 3. Responder sends response via send_response() with same RequestId
//! 4. Requestor receives response via recv_response() with timeout
//! 5. Timeout detection via check_timeouts()
//!
//! ## References
//!
//! - Engineering Plan § 5.3.2 (Request-Response Channel)

use crate::error::{CsError, IpcError, Result};
use crate::ids::{ChannelID, EndpointID};
use crate::message::{MessageHeader, RequestMessage, ResponseMessage, ResponseStatus};
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::fmt;
use serde::{Deserialize, Serialize};

/// Request ID for correlation.
///
/// Uniquely identifies a request within a channel to correlate with responses.
/// Uses u64 to provide 2^64 unique request IDs.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RequestId(u64);

impl RequestId {
    /// Create a new request ID.
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    /// Get the underlying u64 value.
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for RequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "req-{}", self.0)
    }
}

/// Configuration for a request-response channel.
///
/// See Engineering Plan § 5.3.2 (Request-Response Channel)
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChannelConfig {
    /// Maximum number of pending requests.
    pub max_pending: u32,

    /// Default timeout for responses in milliseconds.
    pub default_timeout_ms: u64,

    /// Maximum payload size in bytes.
    pub max_payload_size: u32,

    /// Threshold above which payloads use zero-copy (in bytes).
    pub zero_copy_threshold: u32,
}

impl ChannelConfig {
    /// Create a new channel configuration.
    pub fn new(
        max_pending: u32,
        default_timeout_ms: u64,
        max_payload_size: u32,
        zero_copy_threshold: u32,
    ) -> Self {
        Self {
            max_pending,
            default_timeout_ms,
            max_payload_size,
            zero_copy_threshold,
        }
    }

    /// Create a default configuration.
    pub fn default() -> Self {
        Self {
            max_pending: 1024,
            default_timeout_ms: 5000,
            max_payload_size: 1024 * 1024, // 1MB
            zero_copy_threshold: 4096,
        }
    }
}

impl Default for ChannelConfig {
    fn default() -> Self {
        Self::default()
    }
}

/// Pending request state.
///
/// Tracks the state of a pending request awaiting a response.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct PendingRequest {
    /// Request ID for correlation.
    request_id: RequestId,

    /// Sender's context tag (for capability tracking).
    sender_ct: u64,

    /// Timestamp when request was sent (ns from epoch).
    sent_at: u64,

    /// Absolute deadline for response (ns from epoch).
    deadline_ns: u64,

    /// Copy of the request message.
    request_buffer: RequestMessage,

    /// Optional received response (when response arrives).
    response_slot: Option<ResponseMessage>,
}

/// Timed-out request information.
///
/// Information about a request that exceeded its deadline.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimedOutRequest {
    /// The request ID that timed out.
    pub request_id: RequestId,

    /// The original request message.
    pub request: RequestMessage,

    /// How long past the deadline (ns).
    pub deadline_exceeded_by: u64,
}

/// Request-response channel.
///
/// Provides synchronous request-response communication with timeout handling
/// and request-response correlation.
///
/// See Engineering Plan § 5.3.2 (Request-Response Channel)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RequestResponseChannel {
    /// Unique channel identifier.
    pub id: ChannelID,

    /// Requestor endpoint ID.
    pub requestor_endpoint: EndpointID,

    /// Responder endpoint ID.
    pub responder_endpoint: EndpointID,

    /// Map of pending requests by request ID.
    pending_requests: BTreeMap<RequestId, PendingRequest>,

    /// Channel configuration.
    pub config: ChannelConfig,

    /// Counter for generating unique request IDs.
    request_id_counter: u64,
}

impl RequestResponseChannel {
    /// Create a new request-response channel.
    pub fn new(
        id: ChannelID,
        requestor_endpoint: EndpointID,
        responder_endpoint: EndpointID,
        config: ChannelConfig,
    ) -> Self {
        Self {
            id,
            requestor_endpoint,
            responder_endpoint,
            pending_requests: BTreeMap::new(),
            config,
            request_id_counter: 0,
        }
    }

    /// Generate a new unique request ID.
    fn generate_request_id(&mut self) -> RequestId {
        self.request_id_counter += 1;
        RequestId::new(self.request_id_counter)
    }

    /// Send a request message.
    ///
    /// Queues a request and returns the request ID for later correlation.
    /// Returns an error if the channel is at capacity or payload is too large.
    pub fn send_request(&mut self, request: RequestMessage) -> Result<RequestId> {
        // Check channel capacity
        if self.pending_requests.len() >= self.config.max_pending as usize {
            return Err(CsError::Ipc(IpcError::BackpressureTriggered));
        }

        // Check payload size
        if request.payload.len() > self.config.max_payload_size as usize {
            return Err(CsError::Ipc(IpcError::CapacityExceeded));
        }

        let request_id = self.generate_request_id();

        let pending = PendingRequest {
            request_id,
            sender_ct: request.header.sender_id,
            sent_at: 0, // Would use clock in real implementation
            deadline_ns: request.header.deadline_ns,
            request_buffer: request,
            response_slot: None,
        };

        self.pending_requests.insert(request_id, pending);
        Ok(request_id)
    }

    /// Receive the oldest pending request.
    ///
    /// Returns the request ID and the request message.
    pub fn recv_request(&mut self) -> Result<(RequestId, RequestMessage)> {
        if self.pending_requests.is_empty() {
            return Err(CsError::Ipc(IpcError::Other(
                alloc::string::String::from("no pending requests"),
            )));
        }

        // Get the first (oldest) request
        if let Some((request_id, pending)) = self.pending_requests.iter().next() {
            let request_id = *request_id;
            let request = pending.request_buffer.clone();
            Ok((request_id, request))
        } else {
            Err(CsError::Ipc(IpcError::Other(
                alloc::string::String::from("no pending requests"),
            )))
        }
    }

    /// Send a response to a pending request.
    ///
    /// Matches the response to the request using the request ID.
    pub fn send_response(&mut self, request_id: RequestId, response: ResponseMessage) -> Result<()> {
        if let Some(pending) = self.pending_requests.get_mut(&request_id) {
            pending.response_slot = Some(response);
            Ok(())
        } else {
            Err(CsError::Ipc(IpcError::Other(
                alloc::string::String::from("request not found"),
            )))
        }
    }

    /// Receive a response for a specific request.
    ///
    /// Waits for a response matching the given request ID, up to the
    /// specified timeout in milliseconds.
    pub fn recv_response(&mut self, request_id: RequestId, timeout_ms: u64) -> Result<ResponseMessage> {
        if let Some(pending) = self.pending_requests.get(&request_id) {
            if let Some(response) = &pending.response_slot {
                let response = response.clone();
                // Clean up the pending request
                self.pending_requests.remove(&request_id);
                Ok(response)
            } else {
                Err(CsError::Ipc(IpcError::Timeout))
            }
        } else {
            Err(CsError::Ipc(IpcError::Other(
                alloc::string::String::from("request not found"),
            )))
        }
    }

    /// Check for timed-out requests.
    ///
    /// Returns a list of requests that have exceeded their deadlines.
    /// Note: In a real implementation, this would use system clock.
    pub fn check_timeouts(&mut self, current_time_ns: u64) -> Vec<TimedOutRequest> {
        let mut timed_out = Vec::new();

        let expired: Vec<RequestId> = self
            .pending_requests
            .iter()
            .filter(|(_, pending)| pending.deadline_ns < current_time_ns && pending.response_slot.is_none())
            .map(|(id, _)| *id)
            .collect();

        for request_id in expired {
            if let Some(pending) = self.pending_requests.remove(&request_id) {
                let deadline_exceeded_by = current_time_ns.saturating_sub(pending.deadline_ns);
                timed_out.push(TimedOutRequest {
                    request_id,
                    request: pending.request_buffer,
                    deadline_exceeded_by,
                });
            }
        }

        timed_out
    }

    /// Get the number of pending requests.
    pub fn pending_count(&self) -> usize {
        self.pending_requests.len()
    }

    /// Get all pending request IDs.
    pub fn pending_request_ids(&self) -> Vec<RequestId> {
        self.pending_requests.keys().copied().collect()
    }

    /// Check if a specific request is pending.
    pub fn has_request(&self, request_id: RequestId) -> bool {
        self.pending_requests.contains_key(&request_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::MessageFlags;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec;

    fn create_test_request_message(request_id: u64, payload_size: usize) -> RequestMessage {
        let flags = MessageFlags::new().set_requires_response();
        let header = MessageHeader::new(request_id, 100, 1, 1_000_000_000, flags, payload_size as u32);
        let payload = alloc::vec![42; payload_size];
        RequestMessage::new(header, payload).unwrap()
    }

    fn create_test_response_message(request_id: u64, payload_size: usize) -> ResponseMessage {
        let flags = MessageFlags::new();
        let header = MessageHeader::new(request_id, 200, 1, 2_000_000_000, flags, payload_size as u32);
        let payload = alloc::vec![99; payload_size];
        ResponseMessage::new(header, ResponseStatus::Success, payload).unwrap()
    }

    #[test]
    fn test_request_id_creation() {
        let req_id1 = RequestId::new(42);
        let req_id2 = RequestId::new(43);
        assert_ne!(req_id1, req_id2);
        assert_eq!(req_id1.as_u64(), 42);
    }

    #[test]
    fn test_request_id_display() {
        let req_id = RequestId::new(123);
        let s = req_id.to_string();
        assert!(s.contains("123"));
    }

    #[test]
    fn test_channel_config_default() {
        let config = ChannelConfig::default();
        assert_eq!(config.max_pending, 1024);
        assert!(config.default_timeout_ms > 0);
    }

    #[test]
    fn test_request_response_channel_creation() {
        let chan_id = ChannelID::new();
        let req_ep = EndpointID::new();
        let resp_ep = EndpointID::new();
        let config = ChannelConfig::default();

        let channel = RequestResponseChannel::new(chan_id, req_ep, resp_ep, config);
        assert_eq!(channel.id, chan_id);
        assert_eq!(channel.requestor_endpoint, req_ep);
        assert_eq!(channel.responder_endpoint, resp_ep);
        assert_eq!(channel.pending_count(), 0);
    }

    #[test]
    fn test_send_request() {
        let chan_id = ChannelID::new();
        let req_ep = EndpointID::new();
        let resp_ep = EndpointID::new();
        let config = ChannelConfig::default();
        let mut channel = RequestResponseChannel::new(chan_id, req_ep, resp_ep, config);

        let request = create_test_request_message(1, 100);
        let req_id = channel.send_request(request).unwrap();
        assert_eq!(channel.pending_count(), 1);
        assert!(channel.has_request(req_id));
    }

    #[test]
    fn test_send_request_exceeds_capacity() {
        let chan_id = ChannelID::new();
        let req_ep = EndpointID::new();
        let resp_ep = EndpointID::new();
        let config = ChannelConfig::new(2, 5000, 1024, 4096);
        let mut channel = RequestResponseChannel::new(chan_id, req_ep, resp_ep, config);

        let req1 = create_test_request_message(1, 100);
        let req2 = create_test_request_message(2, 100);
        let req3 = create_test_request_message(3, 100);

        assert!(channel.send_request(req1).is_ok());
        assert!(channel.send_request(req2).is_ok());
        assert!(channel.send_request(req3).is_err()); // Should fail due to capacity
    }

    #[test]
    fn test_send_request_exceeds_payload_size() {
        let chan_id = ChannelID::new();
        let req_ep = EndpointID::new();
        let resp_ep = EndpointID::new();
        let config = ChannelConfig::new(1024, 5000, 256, 4096); // max_payload_size = 256
        let mut channel = RequestResponseChannel::new(chan_id, req_ep, resp_ep, config);

        let request = create_test_request_message(1, 512); // 512 bytes, exceeds max
        assert!(channel.send_request(request).is_err());
    }

    #[test]
    fn test_recv_request() {
        let chan_id = ChannelID::new();
        let req_ep = EndpointID::new();
        let resp_ep = EndpointID::new();
        let config = ChannelConfig::default();
        let mut channel = RequestResponseChannel::new(chan_id, req_ep, resp_ep, config);

        let request = create_test_request_message(1, 100);
        let req_id = channel.send_request(request.clone()).unwrap();

        let (received_id, received_msg) = channel.recv_request().unwrap();
        assert_eq!(received_id, req_id);
        assert_eq!(received_msg.payload.len(), 100);
    }

    #[test]
    fn test_recv_request_empty() {
        let chan_id = ChannelID::new();
        let req_ep = EndpointID::new();
        let resp_ep = EndpointID::new();
        let config = ChannelConfig::default();
        let mut channel = RequestResponseChannel::new(chan_id, req_ep, resp_ep, config);

        assert!(channel.recv_request().is_err());
    }

    #[test]
    fn test_send_response() {
        let chan_id = ChannelID::new();
        let req_ep = EndpointID::new();
        let resp_ep = EndpointID::new();
        let config = ChannelConfig::default();
        let mut channel = RequestResponseChannel::new(chan_id, req_ep, resp_ep, config);

        let request = create_test_request_message(1, 100);
        let req_id = channel.send_request(request).unwrap();

        let response = create_test_response_message(req_id.as_u64(), 50);
        assert!(channel.send_response(req_id, response).is_ok());
    }

    #[test]
    fn test_send_response_nonexistent_request() {
        let chan_id = ChannelID::new();
        let req_ep = EndpointID::new();
        let resp_ep = EndpointID::new();
        let config = ChannelConfig::default();
        let mut channel = RequestResponseChannel::new(chan_id, req_ep, resp_ep, config);

        let response = create_test_response_message(999, 50);
        assert!(channel.send_response(RequestId::new(999), response).is_err());
    }

    #[test]
    fn test_recv_response() {
        let chan_id = ChannelID::new();
        let req_ep = EndpointID::new();
        let resp_ep = EndpointID::new();
        let config = ChannelConfig::default();
        let mut channel = RequestResponseChannel::new(chan_id, req_ep, resp_ep, config);

        let request = create_test_request_message(1, 100);
        let req_id = channel.send_request(request).unwrap();

        let response = create_test_response_message(req_id.as_u64(), 50);
        channel.send_response(req_id, response.clone()).unwrap();

        let received_response = channel.recv_response(req_id, 5000).unwrap();
        assert_eq!(received_response.status, ResponseStatus::Success);
        assert_eq!(received_response.payload.len(), 50);
    }

    #[test]
    fn test_recv_response_not_arrived() {
        let chan_id = ChannelID::new();
        let req_ep = EndpointID::new();
        let resp_ep = EndpointID::new();
        let config = ChannelConfig::default();
        let mut channel = RequestResponseChannel::new(chan_id, req_ep, resp_ep, config);

        let request = create_test_request_message(1, 100);
        let req_id = channel.send_request(request).unwrap();

        // Try to receive without response being sent
        assert!(channel.recv_response(req_id, 5000).is_err());
    }

    #[test]
    fn test_check_timeouts() {
        let chan_id = ChannelID::new();
        let req_ep = EndpointID::new();
        let resp_ep = EndpointID::new();
        let config = ChannelConfig::default();
        let mut channel = RequestResponseChannel::new(chan_id, req_ep, resp_ep, config);

        let request1 = create_test_request_message(1, 100);
        let request2 = create_test_request_message(2, 100);

        let req_id1 = channel.send_request(request1).unwrap();
        let req_id2 = channel.send_request(request2).unwrap();

        // Check timeouts with current time far in the future
        let timed_out = channel.check_timeouts(5_000_000_000);
        assert_eq!(timed_out.len(), 2);
        assert!(channel.has_request(req_id1) == false);
        assert!(channel.has_request(req_id2) == false);
    }

    #[test]
    fn test_check_timeouts_no_expiry() {
        let chan_id = ChannelID::new();
        let req_ep = EndpointID::new();
        let resp_ep = EndpointID::new();
        let config = ChannelConfig::default();
        let mut channel = RequestResponseChannel::new(chan_id, req_ep, resp_ep, config);

        let request = create_test_request_message(1, 100);
        let req_id = channel.send_request(request).unwrap();

        // Check timeouts with current time before deadline
        let timed_out = channel.check_timeouts(500_000_000);
        assert_eq!(timed_out.len(), 0);
        assert!(channel.has_request(req_id));
    }

    #[test]
    fn test_multiple_pending_requests() {
        let chan_id = ChannelID::new();
        let req_ep = EndpointID::new();
        let resp_ep = EndpointID::new();
        let config = ChannelConfig::new(10, 5000, 1024, 4096);
        let mut channel = RequestResponseChannel::new(chan_id, req_ep, resp_ep, config);

        let mut ids = Vec::new();
        for i in 1..=5 {
            let request = create_test_request_message(i, 50);
            let req_id = channel.send_request(request).unwrap();
            ids.push(req_id);
        }

        assert_eq!(channel.pending_count(), 5);

        let pending_ids = channel.pending_request_ids();
        assert_eq!(pending_ids.len(), 5);
    }

    #[test]
    fn test_full_request_response_cycle() {
        let chan_id = ChannelID::new();
        let req_ep = EndpointID::new();
        let resp_ep = EndpointID::new();
        let config = ChannelConfig::default();
        let mut channel = RequestResponseChannel::new(chan_id, req_ep, resp_ep, config);

        // Requestor sends request
        let request = create_test_request_message(1, 100);
        let req_id = channel.send_request(request).unwrap();

        // Responder receives request
        let (received_id, _received_msg) = channel.recv_request().unwrap();
        assert_eq!(received_id, req_id);

        // Responder sends response
        let response = create_test_response_message(req_id.as_u64(), 75);
        channel.send_response(req_id, response).unwrap();

        // Requestor receives response
        let received_response = channel.recv_response(req_id, 5000).unwrap();
        assert_eq!(received_response.status, ResponseStatus::Success);
        assert_eq!(received_response.payload.len(), 75);
        assert_eq!(channel.pending_count(), 0);
    }

    #[test]
    fn test_response_after_timeout_check() {
        let chan_id = ChannelID::new();
        let req_ep = EndpointID::new();
        let resp_ep = EndpointID::new();
        let config = ChannelConfig::default();
        let mut channel = RequestResponseChannel::new(chan_id, req_ep, resp_ep, config);

        let request = create_test_request_message(1, 100);
        let req_id = channel.send_request(request).unwrap();

        // Send response before timeout check
        let response = create_test_response_message(req_id.as_u64(), 50);
        channel.send_response(req_id, response).unwrap();

        // Check timeouts - response is already there, so shouldn't be timed out
        let timed_out = channel.check_timeouts(5_000_000_000);
        assert_eq!(timed_out.len(), 0);

        // Response should still be retrievable
        let received_response = channel.recv_response(req_id, 5000).unwrap();
        assert_eq!(received_response.status, ResponseStatus::Success);
    }
}
