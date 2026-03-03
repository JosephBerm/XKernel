// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Event Subscriber Interface for CEF Events
//!
//! Provides subscription-based event delivery with filtering, routing, and
//! match-based subscriber notification for real-time CEF event consumption.
//!
//! Complements event_logger.rs (structured logging) with proper subscriber
//! interface for distributed telemetry systems.
//!
//! See Engineering Plan § 2.12.6: Event Streaming & Real-Time Telemetry,
//! and Week 5 Objective: Event Subscriber Module.

use alloc::string::String;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use core::fmt;

use crate::cef::CefEvent;
use crate::error::{Result, ToolError};

/// Filter criteria for selecting events by type, actor, and resource.
///
/// Enables selective subscription to specific event streams based on:
/// - Event type classification
/// - Actor (agent/crew) identification
/// - Resource (tool/system) patterns
///
/// See Engineering Plan § 2.12.6: Event Filtering.
#[derive(Clone, Debug)]
pub struct EventFilter {
    /// Event types to match (empty = all types)
    pub event_types: alloc::vec::Vec<String>,

    /// Actor filter (agent/crew ID prefix) - None = all actors
    pub actor_filter: Option<String>,

    /// Resource filter (tool/system identifier) - None = all resources
    pub resource_filter: Option<String>,
}

impl EventFilter {
    /// Creates a filter that matches all events.
    ///
    /// # Returns
    ///
    /// An EventFilter with no restrictions.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let filter = EventFilter::accept_all();
    /// ```
    pub fn accept_all() -> Self {
        EventFilter {
            event_types: Vec::new(),
            actor_filter: None,
            resource_filter: None,
        }
    }

    /// Creates a filter for specific event types.
    ///
    /// # Arguments
    ///
    /// - `types`: Slice of event type names to match
    ///
    /// # Returns
    ///
    /// An EventFilter matching only these event types.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let filter = EventFilter::by_event_types(&["ToolCallRequested", "ToolCallCompleted"]);
    /// ```
    pub fn by_event_types(types: &[&str]) -> Self {
        EventFilter {
            event_types: types.iter().map(|t| t.to_string()).collect(),
            actor_filter: None,
            resource_filter: None,
        }
    }

    /// Adds event type filter.
    ///
    /// # Arguments
    ///
    /// - `event_type`: Event type name to add
    ///
    /// # Returns
    ///
    /// Self for method chaining.
    pub fn with_event_type(mut self, event_type: &str) -> Self {
        self.event_types.push(event_type.to_string());
        self
    }

    /// Sets actor filter (agent/crew ID prefix).
    ///
    /// # Arguments
    ///
    /// - `actor`: Actor ID or prefix to match
    ///
    /// # Returns
    ///
    /// Self for method chaining.
    pub fn with_actor(mut self, actor: &str) -> Self {
        self.actor_filter = Some(actor.to_string());
        self
    }

    /// Sets resource filter (tool/system identifier).
    ///
    /// # Arguments
    ///
    /// - `resource`: Resource identifier to match
    ///
    /// # Returns
    ///
    /// Self for method chaining.
    pub fn with_resource(mut self, resource: &str) -> Self {
        self.resource_filter = Some(resource.to_string());
        self
    }

    /// Checks if an event matches this filter.
    ///
    /// # Arguments
    ///
    /// - `event`: Event to test against filter
    ///
    /// # Returns
    ///
    /// True if the event matches all filter criteria.
    ///
    /// See Engineering Plan § 2.12.6: Event Filtering.
    pub fn matches(&self, event: &CefEvent) -> bool {
        // Check event type filter
        if !self.event_types.is_empty() {
            let event_type_str = event.event_type.to_string();
            if !self.event_types.iter().any(|t| t == &event_type_str) {
                return false;
            }
        }

        // Check actor filter
        if let Some(ref actor) = self.actor_filter {
            if !event.agent_id.contains(actor) {
                return false;
            }
        }

        // Check resource filter
        // Resource is derived from span_id for this version
        if let Some(ref resource) = self.resource_filter {
            if !event.span_id.contains(resource) {
                return false;
            }
        }

        true
    }
}

impl Default for EventFilter {
    fn default() -> Self {
        Self::accept_all()
    }
}

impl fmt::Display for EventFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "EventFilter {{ event_types: {:?}, actor: {:?}, resource: {:?} }}",
            self.event_types, self.actor_filter, self.resource_filter
        )
    }
}

/// Subscription handle for receiving filtered events.
///
/// Represents a subscription to events matching specific criteria.
/// Each subscription has a unique ID, filter, and delivery channel.
///
/// See Engineering Plan § 2.12.6: Event Subscriptions.
#[derive(Clone, Debug)]
pub struct EventSubscription {
    /// Unique subscription identifier
    pub subscription_id: u64,

    /// Event filter criteria
    pub filter: EventFilter,

    /// Subscriber endpoint or channel name
    pub delivery_channel: String,

    /// Subscription creation timestamp (nanoseconds)
    pub created_at_ns: u64,

    /// Number of events delivered on this subscription
    pub event_count: u64,

    /// Is this subscription active?
    pub is_active: bool,
}

impl EventSubscription {
    /// Creates a new event subscription.
    ///
    /// # Arguments
    ///
    /// - `subscription_id`: Unique identifier for this subscription
    /// - `filter`: Event filter criteria
    /// - `delivery_channel`: Endpoint/channel for event delivery
    /// - `timestamp_ns`: Creation timestamp in nanoseconds
    ///
    /// # Returns
    ///
    /// A new active EventSubscription.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let filter = EventFilter::accept_all();
    /// let sub = EventSubscription::new(1, filter, "subscriber-1".to_string(), 1234567890);
    /// assert!(sub.is_active);
    /// ```
    pub fn new(
        subscription_id: u64,
        filter: EventFilter,
        delivery_channel: String,
        timestamp_ns: u64,
    ) -> Self {
        EventSubscription {
            subscription_id,
            filter,
            delivery_channel,
            created_at_ns: timestamp_ns,
            event_count: 0,
            is_active: true,
        }
    }

    /// Records delivery of an event on this subscription.
    ///
    /// Increments the event count for this subscription.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut sub = EventSubscription::new(...);
    /// sub.record_delivery();
    /// assert_eq!(sub.event_count, 1);
    /// ```
    pub fn record_delivery(&mut self) {
        self.event_count = self.event_count.saturating_add(1);
    }

    /// Deactivates this subscription.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut sub = EventSubscription::new(...);
    /// sub.deactivate();
    /// assert!(!sub.is_active);
    /// ```
    pub fn deactivate(&mut self) {
        self.is_active = false;
    }

    /// Activates this subscription.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut sub = EventSubscription::new(...);
    /// sub.deactivate();
    /// sub.activate();
    /// assert!(sub.is_active);
    /// ```
    pub fn activate(&mut self) {
        self.is_active = true;
    }
}

impl fmt::Display for EventSubscription {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "EventSubscription {{ id: {}, channel: {}, active: {}, delivered: {} }}",
            self.subscription_id, self.delivery_channel, self.is_active, self.event_count
        )
    }
}

/// Manages multiple event subscriptions and routes events to matching subscribers.
///
/// Central hub for subscription management and event routing. Maintains a registry
/// of active subscriptions and delivers events to all matching subscribers.
///
/// See Engineering Plan § 2.12.6: Event Subscription Management.
#[derive(Clone, Debug)]
pub struct SubscriptionManager {
    /// Map of subscription ID to subscription
    subscriptions: BTreeMap<u64, EventSubscription>,

    /// Counter for generating unique subscription IDs
    next_subscription_id: u64,

    /// Total events routed by this manager
    total_events_routed: u64,

    /// Total subscriptions ever created
    total_subscriptions_created: u64,
}

impl SubscriptionManager {
    /// Creates a new subscription manager.
    ///
    /// # Returns
    ///
    /// A new SubscriptionManager ready to manage subscriptions.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let manager = SubscriptionManager::new();
    /// assert_eq!(manager.subscription_count(), 0);
    /// ```
    pub fn new() -> Self {
        SubscriptionManager {
            subscriptions: BTreeMap::new(),
            next_subscription_id: 1,
            total_events_routed: 0,
            total_subscriptions_created: 0,
        }
    }

    /// Creates and registers a new subscription.
    ///
    /// # Arguments
    ///
    /// - `filter`: Event filter criteria
    /// - `delivery_channel`: Subscriber endpoint/channel
    /// - `timestamp_ns`: Current timestamp in nanoseconds
    ///
    /// # Returns
    ///
    /// - `Ok(subscription_id)`: ID of the new subscription
    /// - `Err(ToolError)`: Subscription creation failed
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut manager = SubscriptionManager::new();
    /// let filter = EventFilter::accept_all();
    /// let sub_id = manager.subscribe(filter, "channel-1".to_string(), 1000)?;
    /// assert_eq!(manager.subscription_count(), 1);
    /// ```
    pub fn subscribe(
        &mut self,
        filter: EventFilter,
        delivery_channel: String,
        timestamp_ns: u64,
    ) -> Result<u64> {
        let subscription_id = self.next_subscription_id;
        self.next_subscription_id = self.next_subscription_id.saturating_add(1);

        let subscription = EventSubscription::new(
            subscription_id,
            filter,
            delivery_channel,
            timestamp_ns,
        );

        self.subscriptions.insert(subscription_id, subscription);
        self.total_subscriptions_created = self.total_subscriptions_created.saturating_add(1);

        Ok(subscription_id)
    }

    /// Unsubscribes and removes a subscription.
    ///
    /// # Arguments
    ///
    /// - `subscription_id`: ID of subscription to remove
    ///
    /// # Returns
    ///
    /// - `Ok(())`: Subscription removed
    /// - `Err(ToolError)`: Subscription not found
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut manager = SubscriptionManager::new();
    /// let filter = EventFilter::accept_all();
    /// let sub_id = manager.subscribe(filter, "channel".to_string(), 1000)?;
    /// manager.unsubscribe(sub_id)?;
    /// assert_eq!(manager.subscription_count(), 0);
    /// ```
    pub fn unsubscribe(&mut self, subscription_id: u64) -> Result<()> {
        if self.subscriptions.remove(&subscription_id).is_none() {
            return Err(ToolError::Other(format!(
                "subscription {} not found",
                subscription_id
            )));
        }
        Ok(())
    }

    /// Routes an event to all matching subscribers.
    ///
    /// Checks each active subscription's filter against the event.
    /// For matching subscriptions, records the delivery.
    ///
    /// # Arguments
    ///
    /// - `event`: Event to route
    ///
    /// # Returns
    ///
    /// - `Ok(count)`: Number of subscribers that received the event
    /// - `Err(ToolError)`: Event routing failed
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut manager = SubscriptionManager::new();
    /// let filter = EventFilter::accept_all();
    /// manager.subscribe(filter, "channel".to_string(), 1000)?;
    ///
    /// let event = CefEvent::new(...);
    /// let count = manager.route_event(&event)?;
    /// assert_eq!(count, 1);
    /// ```
    ///
    /// See Engineering Plan § 2.12.6: Event Routing.
    pub fn route_event(&mut self, event: &CefEvent) -> Result<usize> {
        let mut matched_count = 0;

        // Collect matching subscription IDs first to avoid borrow issues
        let mut matching_ids: Vec<u64> = Vec::new();
        for (id, subscription) in self.subscriptions.iter() {
            if subscription.is_active && subscription.filter.matches(event) {
                matching_ids.push(*id);
                matched_count += 1;
            }
        }

        // Record delivery for matching subscriptions
        for id in matching_ids {
            if let Some(subscription) = self.subscriptions.get_mut(&id) {
                subscription.record_delivery();
            }
        }

        self.total_events_routed = self.total_events_routed.saturating_add(1);

        Ok(matched_count)
    }

    /// Returns the number of active subscriptions.
    ///
    /// # Returns
    ///
    /// Count of active subscriptions.
    pub fn subscription_count(&self) -> usize {
        self.subscriptions.values().filter(|s| s.is_active).count()
    }

    /// Returns the total number of subscriptions (including inactive).
    ///
    /// # Returns
    ///
    /// Count of all subscriptions.
    pub fn total_subscription_count(&self) -> usize {
        self.subscriptions.len()
    }

    /// Returns a reference to a subscription by ID.
    ///
    /// # Arguments
    ///
    /// - `subscription_id`: ID of subscription to retrieve
    ///
    /// # Returns
    ///
    /// - `Some(&subscription)`: Subscription found
    /// - `None`: Subscription not found
    pub fn get_subscription(&self, subscription_id: u64) -> Option<&EventSubscription> {
        self.subscriptions.get(&subscription_id)
    }

    /// Returns a mutable reference to a subscription by ID.
    ///
    /// # Arguments
    ///
    /// - `subscription_id`: ID of subscription to retrieve
    ///
    /// # Returns
    ///
    /// - `Some(&mut subscription)`: Subscription found
    /// - `None`: Subscription not found
    pub fn get_subscription_mut(&mut self, subscription_id: u64) -> Option<&mut EventSubscription> {
        self.subscriptions.get_mut(&subscription_id)
    }

    /// Deactivates a subscription without removing it.
    ///
    /// # Arguments
    ///
    /// - `subscription_id`: ID of subscription to deactivate
    ///
    /// # Returns
    ///
    /// - `Ok(())`: Subscription deactivated
    /// - `Err(ToolError)`: Subscription not found
    pub fn pause_subscription(&mut self, subscription_id: u64) -> Result<()> {
        if let Some(subscription) = self.subscriptions.get_mut(&subscription_id) {
            subscription.deactivate();
            Ok(())
        } else {
            Err(ToolError::Other(format!(
                "subscription {} not found",
                subscription_id
            )))
        }
    }

    /// Reactivates a paused subscription.
    ///
    /// # Arguments
    ///
    /// - `subscription_id`: ID of subscription to reactivate
    ///
    /// # Returns
    ///
    /// - `Ok(())`: Subscription reactivated
    /// - `Err(ToolError)`: Subscription not found
    pub fn resume_subscription(&mut self, subscription_id: u64) -> Result<()> {
        if let Some(subscription) = self.subscriptions.get_mut(&subscription_id) {
            subscription.activate();
            Ok(())
        } else {
            Err(ToolError::Other(format!(
                "subscription {} not found",
                subscription_id
            )))
        }
    }

    /// Returns total events routed by this manager.
    ///
    /// # Returns
    ///
    /// Count of events routed.
    pub fn total_events_routed(&self) -> u64 {
        self.total_events_routed
    }

    /// Returns total subscriptions ever created.
    ///
    /// # Returns
    ///
    /// Count of subscriptions created (including deleted ones).
    pub fn total_subscriptions_created(&self) -> u64 {
        self.total_subscriptions_created
    }

    /// Resets all counters to zero.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut manager = SubscriptionManager::new();
    /// manager.total_events_routed = 100;
    /// manager.reset_stats();
    /// assert_eq!(manager.total_events_routed(), 0);
    /// ```
    pub fn reset_stats(&mut self) {
        self.total_events_routed = 0;
        self.total_subscriptions_created = 0;
    }
}

impl Default for SubscriptionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for SubscriptionManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SubscriptionManager {{ subscriptions: {}, routed: {}, total_created: {} }}",
            self.subscription_count(),
            self.total_events_routed,
            self.total_subscriptions_created
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cef::{CefEvent, CefEventType};
use alloc::format;
use alloc::string::ToString;

    #[test]
    fn test_event_filter_accept_all() {
        let filter = EventFilter::accept_all();
        assert!(filter.event_types.is_empty());
        assert!(filter.actor_filter.is_none());
        assert!(filter.resource_filter.is_none());
    }

    #[test]
    fn test_event_filter_by_event_types() {
        let filter = EventFilter::by_event_types(&["ToolCallRequested", "ToolCallCompleted"]);
        assert_eq!(filter.event_types.len(), 2);
    }

    #[test]
    fn test_event_filter_with_event_type() {
        let filter = EventFilter::accept_all()
            .with_event_type("ToolCallRequested")
            .with_event_type("PolicyDecision");
        assert_eq!(filter.event_types.len(), 2);
    }

    #[test]
    fn test_event_filter_with_actor() {
        let filter = EventFilter::accept_all().with_actor("agent-1");
        assert_eq!(filter.actor_filter, Some("agent-1".to_string()));
    }

    #[test]
    fn test_event_filter_with_resource() {
        let filter = EventFilter::accept_all().with_resource("tool-1");
        assert_eq!(filter.resource_filter, Some("tool-1".to_string()));
    }

    #[test]
    fn test_event_filter_matches_all() {
        let filter = EventFilter::accept_all();
        let event = CefEvent::new(
            "evt-1",
            "trace-1",
            "span-1",
            "ct-1",
            "agent-1",
            1000,
            CefEventType::ToolCallRequested,
            "acting",
        );
        assert!(filter.matches(&event));
    }

    #[test]
    fn test_event_filter_matches_event_type() {
        let filter = EventFilter::by_event_types(&["ToolCallRequested"]);
        let event = CefEvent::new(
            "evt-1",
            "trace-1",
            "span-1",
            "ct-1",
            "agent-1",
            1000,
            CefEventType::ToolCallRequested,
            "acting",
        );
        assert!(filter.matches(&event));
    }

    #[test]
    fn test_event_filter_no_match_event_type() {
        let filter = EventFilter::by_event_types(&["PolicyDecision"]);
        let event = CefEvent::new(
            "evt-1",
            "trace-1",
            "span-1",
            "ct-1",
            "agent-1",
            1000,
            CefEventType::ToolCallRequested,
            "acting",
        );
        assert!(!filter.matches(&event));
    }

    #[test]
    fn test_event_filter_matches_actor() {
        let filter = EventFilter::accept_all().with_actor("agent-1");
        let event = CefEvent::new(
            "evt-1",
            "trace-1",
            "span-1",
            "ct-1",
            "agent-1",
            1000,
            CefEventType::ToolCallRequested,
            "acting",
        );
        assert!(filter.matches(&event));
    }

    #[test]
    fn test_event_filter_no_match_actor() {
        let filter = EventFilter::accept_all().with_actor("agent-2");
        let event = CefEvent::new(
            "evt-1",
            "trace-1",
            "span-1",
            "ct-1",
            "agent-1",
            1000,
            CefEventType::ToolCallRequested,
            "acting",
        );
        assert!(!filter.matches(&event));
    }

    #[test]
    fn test_event_filter_matches_resource() {
        let filter = EventFilter::accept_all().with_resource("span-1");
        let event = CefEvent::new(
            "evt-1",
            "trace-1",
            "span-1",
            "ct-1",
            "agent-1",
            1000,
            CefEventType::ToolCallRequested,
            "acting",
        );
        assert!(filter.matches(&event));
    }

    #[test]
    fn test_event_filter_display() {
        let filter = EventFilter::accept_all()
            .with_actor("agent-1")
            .with_resource("tool-1");
        let display = filter.to_string();
        assert!(display.contains("EventFilter"));
    }

    #[test]
    fn test_event_subscription_creation() {
        let filter = EventFilter::accept_all();
        let sub = EventSubscription::new(1, filter, "channel-1".to_string(), 1000);
        assert_eq!(sub.subscription_id, 1);
        assert_eq!(sub.delivery_channel, "channel-1");
        assert!(sub.is_active);
        assert_eq!(sub.event_count, 0);
    }

    #[test]
    fn test_event_subscription_record_delivery() {
        let filter = EventFilter::accept_all();
        let mut sub = EventSubscription::new(1, filter, "channel-1".to_string(), 1000);
        sub.record_delivery();
        assert_eq!(sub.event_count, 1);
        sub.record_delivery();
        assert_eq!(sub.event_count, 2);
    }

    #[test]
    fn test_event_subscription_deactivate() {
        let filter = EventFilter::accept_all();
        let mut sub = EventSubscription::new(1, filter, "channel-1".to_string(), 1000);
        assert!(sub.is_active);
        sub.deactivate();
        assert!(!sub.is_active);
    }

    #[test]
    fn test_event_subscription_activate() {
        let filter = EventFilter::accept_all();
        let mut sub = EventSubscription::new(1, filter, "channel-1".to_string(), 1000);
        sub.deactivate();
        assert!(!sub.is_active);
        sub.activate();
        assert!(sub.is_active);
    }

    #[test]
    fn test_event_subscription_display() {
        let filter = EventFilter::accept_all();
        let sub = EventSubscription::new(1, filter, "channel-1".to_string(), 1000);
        let display = sub.to_string();
        assert!(display.contains("EventSubscription"));
        assert!(display.contains("channel-1"));
    }

    #[test]
    fn test_subscription_manager_creation() {
        let manager = SubscriptionManager::new();
        assert_eq!(manager.subscription_count(), 0);
        assert_eq!(manager.total_subscriptions_created(), 0);
        assert_eq!(manager.total_events_routed(), 0);
    }

    #[test]
    fn test_subscription_manager_subscribe() {
        let mut manager = SubscriptionManager::new();
        let filter = EventFilter::accept_all();
        let sub_id = manager.subscribe(filter, "channel-1".to_string(), 1000).unwrap();
        assert_eq!(sub_id, 1);
        assert_eq!(manager.subscription_count(), 1);
    }

    #[test]
    fn test_subscription_manager_unsubscribe() {
        let mut manager = SubscriptionManager::new();
        let filter = EventFilter::accept_all();
        let sub_id = manager.subscribe(filter, "channel-1".to_string(), 1000).unwrap();
        assert_eq!(manager.subscription_count(), 1);
        manager.unsubscribe(sub_id).unwrap();
        assert_eq!(manager.subscription_count(), 0);
    }

    #[test]
    fn test_subscription_manager_unsubscribe_not_found() {
        let mut manager = SubscriptionManager::new();
        let result = manager.unsubscribe(999);
        assert!(result.is_err());
    }

    #[test]
    fn test_subscription_manager_route_event() {
        let mut manager = SubscriptionManager::new();
        let filter = EventFilter::accept_all();
        manager.subscribe(filter, "channel-1".to_string(), 1000).unwrap();

        let event = CefEvent::new(
            "evt-1",
            "trace-1",
            "span-1",
            "ct-1",
            "agent-1",
            1000,
            CefEventType::ToolCallRequested,
            "acting",
        );

        let count = manager.route_event(&event).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_subscription_manager_route_event_filtered() {
        let mut manager = SubscriptionManager::new();
        let filter = EventFilter::by_event_types(&["PolicyDecision"]);
        manager.subscribe(filter, "channel-1".to_string(), 1000).unwrap();

        let event = CefEvent::new(
            "evt-1",
            "trace-1",
            "span-1",
            "ct-1",
            "agent-1",
            1000,
            CefEventType::ToolCallRequested,
            "acting",
        );

        let count = manager.route_event(&event).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_subscription_manager_multiple_subscriptions() {
        let mut manager = SubscriptionManager::new();
        let filter = EventFilter::accept_all();
        manager.subscribe(filter.clone(), "channel-1".to_string(), 1000).unwrap();
        manager.subscribe(filter.clone(), "channel-2".to_string(), 1000).unwrap();
        manager.subscribe(filter, "channel-3".to_string(), 1000).unwrap();
        assert_eq!(manager.subscription_count(), 3);
    }

    #[test]
    fn test_subscription_manager_get_subscription() {
        let mut manager = SubscriptionManager::new();
        let filter = EventFilter::accept_all();
        let sub_id = manager.subscribe(filter, "channel-1".to_string(), 1000).unwrap();

        let subscription = manager.get_subscription(sub_id);
        assert!(subscription.is_some());
        assert_eq!(subscription.unwrap().subscription_id, sub_id);
    }

    #[test]
    fn test_subscription_manager_pause_resume() {
        let mut manager = SubscriptionManager::new();
        let filter = EventFilter::accept_all();
        let sub_id = manager.subscribe(filter, "channel-1".to_string(), 1000).unwrap();

        manager.pause_subscription(sub_id).unwrap();
        assert!(!manager.get_subscription(sub_id).unwrap().is_active);

        manager.resume_subscription(sub_id).unwrap();
        assert!(manager.get_subscription(sub_id).unwrap().is_active);
    }

    #[test]
    fn test_subscription_manager_pause_inactive_not_routed() {
        let mut manager = SubscriptionManager::new();
        let filter = EventFilter::accept_all();
        let sub_id = manager.subscribe(filter, "channel-1".to_string(), 1000).unwrap();

        manager.pause_subscription(sub_id).unwrap();

        let event = CefEvent::new(
            "evt-1",
            "trace-1",
            "span-1",
            "ct-1",
            "agent-1",
            1000,
            CefEventType::ToolCallRequested,
            "acting",
        );

        let count = manager.route_event(&event).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_subscription_manager_display() {
        let manager = SubscriptionManager::new();
        let display = manager.to_string();
        assert!(display.contains("SubscriptionManager"));
    }

    #[test]
    fn test_subscription_manager_reset_stats() {
        let mut manager = SubscriptionManager::new();
        let filter = EventFilter::accept_all();
        manager.subscribe(filter, "channel-1".to_string(), 1000).unwrap();

        let event = CefEvent::new(
            "evt-1",
            "trace-1",
            "span-1",
            "ct-1",
            "agent-1",
            1000,
            CefEventType::ToolCallRequested,
            "acting",
        );
        manager.route_event(&event).unwrap();

        assert_eq!(manager.total_events_routed(), 1);
        manager.reset_stats();
        assert_eq!(manager.total_events_routed(), 0);
    }

    #[test]
    fn test_subscription_manager_event_count_tracking() {
        let mut manager = SubscriptionManager::new();
        let filter = EventFilter::accept_all();
        let sub_id = manager.subscribe(filter, "channel-1".to_string(), 1000).unwrap();

        let event = CefEvent::new(
            "evt-1",
            "trace-1",
            "span-1",
            "ct-1",
            "agent-1",
            1000,
            CefEventType::ToolCallRequested,
            "acting",
        );

        manager.route_event(&event).unwrap();
        assert_eq!(manager.get_subscription(sub_id).unwrap().event_count, 1);

        manager.route_event(&event).unwrap();
        assert_eq!(manager.get_subscription(sub_id).unwrap().event_count, 2);
    }
}
