// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! # Kernel-wide Channel Registry
//!
//! This module manages all active channels in the Cognitive Substrate kernel.
//! The registry provides central lifecycle management for IPC channels, including
//! creation, destruction, lookup, and enumeration of channels by context.
//!
//! ## Channel Types
//!
//! The registry can store multiple channel types:
//! - RequestResponse channels (synchronous request-response)
//! - PubSub channels (publish-subscribe)
//! - SharedContext channels (context sharing)
//!
//! ## References
//!
//! - Engineering Plan § 5.3.4 (Channel Registry)

use crate::error::{CsError, IpcError, Result};
use crate::ids::ChannelID;
use crate::request_response::{ChannelConfig, RequestResponseChannel};
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

/// Registered channel in the kernel registry.
///
/// A channel variant that can hold different channel types.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RegisteredChannel {
    /// Request-response channel for synchronous communication.
    RequestResponse(RequestResponseChannel),

    /// Placeholder for PubSub channel (stub).
    PubSub,

    /// Placeholder for shared context channel (stub).
    SharedContext,
}

impl RegisteredChannel {
    /// Get the channel type name.
    pub fn channel_type(&self) -> &'static str {
        match self {
            RegisteredChannel::RequestResponse(_) => "RequestResponse",
            RegisteredChannel::PubSub => "PubSub",
            RegisteredChannel::SharedContext => "SharedContext",
        }
    }
}

/// Channel registry statistics.
///
/// Metrics about the channel registry state.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegistryStats {
    /// Total number of channels ever created.
    pub total_channels: u64,

    /// Current number of active channels.
    pub active_channels: u64,

    /// Peak number of channels at any point.
    pub peak_channels: u64,
}

impl RegistryStats {
    /// Create empty statistics.
    pub fn new() -> Self {
        Self {
            total_channels: 0,
            active_channels: 0,
            peak_channels: 0,
        }
    }

    /// Update peak if current is higher.
    fn update_peak(&mut self, current: u64) {
        if current > self.peak_channels {
            self.peak_channels = current;
        }
    }
}

impl Default for RegistryStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Kernel-wide channel registry.
///
/// Central location for managing all IPC channels in the system.
/// Provides creation, destruction, lookup, and enumeration operations.
///
/// See Engineering Plan § 5.3.4 (Channel Registry)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChannelRegistry {
    /// Map of all registered channels by ID.
    channels: BTreeMap<ChannelID, RegisteredChannel>,

    /// Map of channels for each context tag (CT).
    channels_by_ct: BTreeMap<u64, Vec<ChannelID>>,

    /// Statistics.
    pub stats: RegistryStats,
}

impl ChannelRegistry {
    /// Create a new channel registry.
    pub fn new() -> Self {
        Self {
            channels: BTreeMap::new(),
            channels_by_ct: BTreeMap::new(),
            stats: RegistryStats::new(),
        }
    }

    /// Create a new request-response channel.
    ///
    /// Creates a new request-response channel with the given configuration
    /// and associates it with the specified endpoints (which may be represented
    /// as context tags in practice).
    pub fn create_channel_request_response(
        &mut self,
        channel_id: ChannelID,
        requestor_ct: u64,
        responder_ct: u64,
        config: ChannelConfig,
    ) -> Result<ChannelID> {
        if self.channels.contains_key(&channel_id) {
            return Err(CsError::Ipc(IpcError::Other(
                alloc::string::String::from("channel already exists"),
            )));
        }

        // Create endpoint IDs from context tags (simplified for now)
        use crate::ids::EndpointID;
        let requestor_endpoint = EndpointID::new();
        let responder_endpoint = EndpointID::new();

        let channel = RequestResponseChannel::new(channel_id, requestor_endpoint, responder_endpoint, config);
        let registered = RegisteredChannel::RequestResponse(channel);

        self.channels.insert(channel_id, registered);

        // Track channels by context
        self.channels_by_ct.entry(requestor_ct).or_insert_with(Vec::new).push(channel_id);
        self.channels_by_ct.entry(responder_ct).or_insert_with(Vec::new).push(channel_id);

        // Update statistics
        self.stats.total_channels += 1;
        self.stats.active_channels += 1;
        self.stats.update_peak(self.stats.active_channels);

        Ok(channel_id)
    }

    /// Destroy a channel.
    ///
    /// Removes a channel from the registry. The channel must exist.
    pub fn destroy_channel(&mut self, channel_id: ChannelID) -> Result<()> {
        if !self.channels.contains_key(&channel_id) {
            return Err(CsError::Ipc(IpcError::InvalidChannel));
        }

        self.channels.remove(&channel_id);

        // Clean up from channels_by_ct
        for cts in self.channels_by_ct.values_mut() {
            cts.retain(|id| id != &channel_id);
        }

        self.stats.active_channels = self.stats.active_channels.saturating_sub(1);

        Ok(())
    }

    /// Look up a channel by ID.
    ///
    /// Returns a reference to the registered channel if it exists.
    pub fn lookup_channel(&self, channel_id: ChannelID) -> Result<&RegisteredChannel> {
        self.channels
            .get(&channel_id)
            .ok_or_else(|| CsError::Ipc(IpcError::InvalidChannel))
    }

    /// Look up a request-response channel by ID (mutable).
    ///
    /// Returns a mutable reference to a request-response channel.
    pub fn lookup_channel_mut(&mut self, channel_id: ChannelID) -> Result<&mut RequestResponseChannel> {
        if let Some(RegisteredChannel::RequestResponse(chan)) = self.channels.get_mut(&channel_id) {
            Ok(chan)
        } else {
            Err(CsError::Ipc(IpcError::InvalidChannel))
        }
    }

    /// Get all channels for a specific context.
    ///
    /// Returns a vector of channel IDs associated with the given context tag.
    pub fn channels_for_ct(&self, ct_id: u64) -> Vec<ChannelID> {
        self.channels_by_ct
            .get(&ct_id)
            .map(|v| v.clone())
            .unwrap_or_default()
    }

    /// Get all channels.
    ///
    /// Returns a vector of all registered channel IDs.
    pub fn all_channels(&self) -> Vec<ChannelID> {
        self.channels.keys().copied().collect()
    }

    /// Get the number of active channels.
    pub fn channel_count(&self) -> usize {
        self.channels.len()
    }

    /// Check if a channel exists.
    pub fn contains_channel(&self, channel_id: ChannelID) -> bool {
        self.channels.contains_key(&channel_id)
    }

    /// Get statistics about the registry.
    pub fn get_stats(&self) -> RegistryStats {
        self.stats
    }
}

impl Default for ChannelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::string::String;

    #[test]
    fn test_channel_registry_creation() {
        let registry = ChannelRegistry::new();
        assert_eq!(registry.channel_count(), 0);
        assert_eq!(registry.stats.active_channels, 0);
        assert_eq!(registry.stats.total_channels, 0);
    }

    #[test]
    fn test_create_request_response_channel() {
        let mut registry = ChannelRegistry::new();
        let channel_id = ChannelID::new();
        let config = ChannelConfig::default();

        let result = registry.create_channel_request_response(channel_id, 100, 200, config);
        assert!(result.is_ok());
        assert_eq!(registry.channel_count(), 1);
        assert_eq!(registry.stats.active_channels, 1);
        assert_eq!(registry.stats.total_channels, 1);
    }

    #[test]
    fn test_create_channel_duplicate() {
        let mut registry = ChannelRegistry::new();
        let channel_id = ChannelID::new();
        let config = ChannelConfig::default();

        let _ = registry.create_channel_request_response(channel_id, 100, 200, config);
        let result = registry.create_channel_request_response(channel_id, 100, 200, config);
        assert!(result.is_err());
    }

    #[test]
    fn test_destroy_channel() {
        let mut registry = ChannelRegistry::new();
        let channel_id = ChannelID::new();
        let config = ChannelConfig::default();

        registry.create_channel_request_response(channel_id, 100, 200, config).unwrap();
        assert_eq!(registry.channel_count(), 1);

        let result = registry.destroy_channel(channel_id);
        assert!(result.is_ok());
        assert_eq!(registry.channel_count(), 0);
        assert_eq!(registry.stats.active_channels, 0);
    }

    #[test]
    fn test_destroy_nonexistent_channel() {
        let mut registry = ChannelRegistry::new();
        let channel_id = ChannelID::new();
        let result = registry.destroy_channel(channel_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_lookup_channel() {
        let mut registry = ChannelRegistry::new();
        let channel_id = ChannelID::new();
        let config = ChannelConfig::default();

        registry.create_channel_request_response(channel_id, 100, 200, config).unwrap();

        let result = registry.lookup_channel(channel_id);
        assert!(result.is_ok());
        let channel = result.unwrap();
        assert_eq!(channel.channel_type(), "RequestResponse");
    }

    #[test]
    fn test_lookup_nonexistent_channel() {
        let registry = ChannelRegistry::new();
        let channel_id = ChannelID::new();
        let result = registry.lookup_channel(channel_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_lookup_channel_mut() {
        let mut registry = ChannelRegistry::new();
        let channel_id = ChannelID::new();
        let config = ChannelConfig::default();

        registry.create_channel_request_response(channel_id, 100, 200, config).unwrap();

        let result = registry.lookup_channel_mut(channel_id);
        assert!(result.is_ok());
        let channel = result.unwrap();
        assert_eq!(channel.pending_count(), 0);
    }

    #[test]
    fn test_channels_for_ct() {
        let mut registry = ChannelRegistry::new();
        let channel_id1 = ChannelID::new();
        let channel_id2 = ChannelID::new();
        let config = ChannelConfig::default();

        registry.create_channel_request_response(channel_id1, 100, 200, config).unwrap();
        registry.create_channel_request_response(channel_id2, 100, 300, config).unwrap();

        let channels_for_100 = registry.channels_for_ct(100);
        assert_eq!(channels_for_100.len(), 2);
        assert!(channels_for_100.contains(&channel_id1));
        assert!(channels_for_100.contains(&channel_id2));

        let channels_for_200 = registry.channels_for_ct(200);
        assert_eq!(channels_for_200.len(), 1);
        assert!(channels_for_200.contains(&channel_id1));
    }

    #[test]
    fn test_channels_for_ct_nonexistent() {
        let registry = ChannelRegistry::new();
        let channels = registry.channels_for_ct(999);
        assert_eq!(channels.len(), 0);
    }

    #[test]
    fn test_all_channels() {
        let mut registry = ChannelRegistry::new();
        let channel_id1 = ChannelID::new();
        let channel_id2 = ChannelID::new();
        let config = ChannelConfig::default();

        registry.create_channel_request_response(channel_id1, 100, 200, config).unwrap();
        registry.create_channel_request_response(channel_id2, 300, 400, config).unwrap();

        let all_channels = registry.all_channels();
        assert_eq!(all_channels.len(), 2);
        assert!(all_channels.contains(&channel_id1));
        assert!(all_channels.contains(&channel_id2));
    }

    #[test]
    fn test_contains_channel() {
        let mut registry = ChannelRegistry::new();
        let channel_id = ChannelID::new();
        let config = ChannelConfig::default();

        assert!(!registry.contains_channel(channel_id));

        registry.create_channel_request_response(channel_id, 100, 200, config).unwrap();
        assert!(registry.contains_channel(channel_id));

        registry.destroy_channel(channel_id).unwrap();
        assert!(!registry.contains_channel(channel_id));
    }

    #[test]
    fn test_statistics_peak() {
        let mut registry = ChannelRegistry::new();
        let config = ChannelConfig::default();

        for _ in 0..5 {
            let channel_id = ChannelID::new();
            registry.create_channel_request_response(channel_id, 100, 200, config).unwrap();
        }

        assert_eq!(registry.stats.active_channels, 5);
        assert_eq!(registry.stats.peak_channels, 5);
        assert_eq!(registry.stats.total_channels, 5);

        let channels = registry.all_channels();
        for channel_id in channels.iter().take(2) {
            registry.destroy_channel(*channel_id).unwrap();
        }

        assert_eq!(registry.stats.active_channels, 3);
        assert_eq!(registry.stats.peak_channels, 5); // Peak should not decrease
    }

    #[test]
    fn test_channels_by_ct_cleanup() {
        let mut registry = ChannelRegistry::new();
        let channel_id1 = ChannelID::new();
        let channel_id2 = ChannelID::new();
        let config = ChannelConfig::default();

        registry.create_channel_request_response(channel_id1, 100, 200, config).unwrap();
        registry.create_channel_request_response(channel_id2, 100, 300, config).unwrap();

        assert_eq!(registry.channels_for_ct(100).len(), 2);

        registry.destroy_channel(channel_id1).unwrap();
        assert_eq!(registry.channels_for_ct(100).len(), 1);
    }

    #[test]
    fn test_multiple_channels_same_cts() {
        let mut registry = ChannelRegistry::new();
        let config = ChannelConfig::default();

        let channel_id1 = ChannelID::new();
        let channel_id2 = ChannelID::new();
        let channel_id3 = ChannelID::new();

        registry.create_channel_request_response(channel_id1, 100, 200, config).unwrap();
        registry.create_channel_request_response(channel_id2, 100, 200, config).unwrap();
        registry.create_channel_request_response(channel_id3, 100, 200, config).unwrap();

        let channels_for_100 = registry.channels_for_ct(100);
        assert_eq!(channels_for_100.len(), 3);

        let channels_for_200 = registry.channels_for_ct(200);
        assert_eq!(channels_for_200.len(), 3);
    }

    #[test]
    fn test_registry_stats() {
        let mut registry = ChannelRegistry::new();
        let config = ChannelConfig::default();

        for i in 0..3 {
            let channel_id = ChannelID::new();
            registry.create_channel_request_response(channel_id, 100 + i, 200 + i, config).unwrap();
        }

        let stats = registry.get_stats();
        assert_eq!(stats.total_channels, 3);
        assert_eq!(stats.active_channels, 3);
        assert_eq!(stats.peak_channels, 3);
    }

    #[test]
    fn test_registered_channel_type_names() {
        let req_resp_channel = RegisteredChannel::RequestResponse(RequestResponseChannel::new(
            ChannelID::new(),
            crate::ids::EndpointID::new(),
            crate::ids::EndpointID::new(),
            ChannelConfig::default(),
        ));
        assert_eq!(req_resp_channel.channel_type(), "RequestResponse");

        let pubsub_channel = RegisteredChannel::PubSub;
        assert_eq!(pubsub_channel.channel_type(), "PubSub");

        let shared_context_channel = RegisteredChannel::SharedContext;
        assert_eq!(shared_context_channel.channel_type(), "SharedContext");
    }
}
