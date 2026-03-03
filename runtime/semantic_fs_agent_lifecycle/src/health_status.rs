//! Health Status Tracking and Reporting for Agents
//!
//! This module provides health state tracking, polling, and reporting for agent instances.
//! Implements HealthState enum and health event emission for multi-agent scenarios.
//! See RFC: Week 6 Health Status subsystem design.

use alloc::collections::BTreeMap;
use alloc::sync::Arc; // Mutex not available in no_std
// use std::time removed - not available in no_std
use crate::error::{LifecycleError, Result};
use alloc::collections::BTreeMap as HashMap;

/// Health state of an agent or service.
///
/// Represents the current health condition of a managed agent.
/// Used in health check polling and status aggregation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HealthState {
    /// Agent is running normally
    Running,
    /// Agent has been stopped
    Stopped,
    /// Agent has failed with reason
    Failed,
}

impl HealthState {
    /// Returns true if the health state is Running.
    pub fn is_running(&self) -> bool {
        matches!(self, HealthState::Running)
    }

    /// Returns true if the health state is Stopped.
    pub fn is_stopped(&self) -> bool {
        matches!(self, HealthState::Stopped)
    }

    /// Returns true if the health state is Failed.
    pub fn is_failed(&self) -> bool {
        matches!(self, HealthState::Failed)
    }

    /// Returns string representation of health state.
    pub fn as_str(&self) -> &str {
        match self {
            HealthState::Running => "running",
            HealthState::Stopped => "stopped",
            HealthState::Failed => "failed",
        }
    }
}

/// Health event for an agent.
///
/// Represents a health state transition or status update event.
/// Emitted during health check polling and aggregation operations.
#[derive(Debug, Clone)]
pub struct HealthEvent {
    /// Unique identifier for the agent
    pub agent_id: String,
    /// Current health state
    pub state: HealthState,
    /// Optional failure reason
    pub reason: Option<String>,
    /// Timestamp of the event
    pub timestamp: SystemTime,
    /// Health check metrics
    pub metrics: HealthMetrics,
}

impl HealthEvent {
    /// Create a new health event.
    ///
    /// # Arguments
    /// * `agent_id` - Unique identifier for the agent
    /// * `state` - Current health state
    /// * `reason` - Optional failure reason
    ///
    /// Returns a new HealthEvent with current timestamp.
    pub fn new(agent_id: String, state: HealthState, reason: Option<String>) -> Self {
        Self {
            agent_id,
            state,
            reason,
            timestamp: SystemTime::now(),
            metrics: HealthMetrics::default(),
        }
    }

    /// Create a running health event.
    pub fn running(agent_id: String) -> Self {
        Self::new(agent_id, HealthState::Running, None)
    }

    /// Create a stopped health event.
    pub fn stopped(agent_id: String) -> Self {
        Self::new(agent_id, HealthState::Stopped, None)
    }

    /// Create a failed health event with reason.
    pub fn failed(agent_id: String, reason: String) -> Self {
        Self::new(agent_id, HealthState::Failed, Some(reason))
    }

    /// Add metrics to this event.
    pub fn with_metrics(mut self, metrics: HealthMetrics) -> Self {
        self.metrics = metrics;
        self
    }
}

/// Health check metrics for an agent.
///
/// Contains quantitative measurements of agent health.
#[derive(Debug, Clone, Default)]
pub struct HealthMetrics {
    /// CPU usage in percentage
    pub cpu_percent: f64,
    /// Memory usage in bytes
    pub memory_bytes: u64,
    /// Number of threads
    pub thread_count: usize,
    /// Uptime in seconds
    pub uptime_secs: u64,
    /// Number of health check failures
    pub failure_count: u32,
    /// Last successful check timestamp
    pub last_check: Option<SystemTime>,
}

impl HealthMetrics {
    /// Create new health metrics with defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set CPU usage percentage.
    pub fn with_cpu(mut self, cpu_percent: f64) -> Self {
        self.cpu_percent = cpu_percent;
        self
    }

    /// Set memory usage in bytes.
    pub fn with_memory(mut self, memory_bytes: u64) -> Self {
        self.memory_bytes = memory_bytes;
        self
    }

    /// Set thread count.
    pub fn with_threads(mut self, count: usize) -> Self {
        self.thread_count = count;
        self
    }

    /// Set uptime in seconds.
    pub fn with_uptime(mut self, secs: u64) -> Self {
        self.uptime_secs = secs;
        self
    }
}

/// Health status tracker for a single agent.
///
/// Tracks current health state and provides status queries.
#[derive(Debug, Clone)]
pub struct AgentHealthStatus {
    /// Agent identifier
    agent_id: String,
    /// Current health state
    state: HealthState,
    /// Optional failure reason
    reason: Option<String>,
    /// Last status update time
    last_update: SystemTime,
    /// Health metrics
    metrics: HealthMetrics,
}

impl AgentHealthStatus {
    /// Create a new agent health status tracker.
    ///
    /// # Arguments
    /// * `agent_id` - Unique agent identifier
    ///
    /// Initializes with Stopped state.
    pub fn new(agent_id: String) -> Self {
        Self {
            agent_id,
            state: HealthState::Stopped,
            reason: None,
            last_update: SystemTime::now(),
            metrics: HealthMetrics::default(),
        }
    }

    /// Get the agent ID.
    pub fn agent_id(&self) -> &str {
        &self.agent_id
    }

    /// Get current health state.
    pub fn state(&self) -> HealthState {
        self.state
    }

    /// Get optional failure reason.
    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }

    /// Get last update timestamp.
    pub fn last_update(&self) -> SystemTime {
        self.last_update
    }

    /// Get health metrics.
    pub fn metrics(&self) -> &HealthMetrics {
        &self.metrics
    }

    /// Update health state.
    ///
    /// # Arguments
    /// * `state` - New health state
    /// * `reason` - Optional failure reason
    pub fn update_state(&mut self, state: HealthState, reason: Option<String>) {
        self.state = state;
        self.reason = reason;
        self.last_update = SystemTime::now();
    }

    /// Update health metrics.
    pub fn update_metrics(&mut self, metrics: HealthMetrics) {
        self.metrics = metrics;
        self.last_update = SystemTime::now();
    }

    /// Convert to a health event.
    pub fn to_event(&self) -> HealthEvent {
        HealthEvent {
            agent_id: self.agent_id.clone(),
            state: self.state,
            reason: self.reason.clone(),
            timestamp: self.last_update,
            metrics: self.metrics.clone(),
        }
    }
}

/// Health status aggregator for multi-agent scenarios.
///
/// Tracks health status across multiple agents and provides aggregate reporting.
/// Thread-safe via Arc<Mutex<>>.
#[derive(Clone)]
pub struct HealthStatusAggregator {
    /// Map of agent ID to health status
    statuses: Arc<Mutex<HashMap<String, AgentHealthStatus>>>,
    /// Listeners for health events
    listeners: Arc<Mutex<Vec<Box<dyn Fn(HealthEvent) + Send>>>>,
}

impl HealthStatusAggregator {
    /// Create a new health status aggregator.
    pub fn new() -> Self {
        Self {
            statuses: Arc::new(Mutex::new(HashMap::new())),
            listeners: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Register a health event listener.
    ///
    /// # Arguments
    /// * `listener` - Callback function invoked on health events
    ///
    /// # Returns
    /// Result indicating success or lock contention error.
    pub fn register_listener<F>(&self, listener: F) -> Result<()>
    where
        F: Fn(HealthEvent) + Send + 'static,
    {
        let mut listeners = self.listeners.lock()
            .map_err(|_| LifecycleError::HealthStatusError("Failed to acquire listener lock".to_string()))?;
        listeners.push(Box::new(listener));
        Ok(())
    }

    /// Register an agent for health tracking.
    ///
    /// # Arguments
    /// * `agent_id` - Unique agent identifier
    ///
    /// # Returns
    /// Result indicating success or lock contention error.
    pub fn register_agent(&self, agent_id: String) -> Result<()> {
        let mut statuses = self.statuses.lock()
            .map_err(|_| LifecycleError::HealthStatusError("Failed to acquire status lock".to_string()))?;
        statuses.insert(agent_id.clone(), AgentHealthStatus::new(agent_id));
        Ok(())
    }

    /// Get health status for a specific agent.
    ///
    /// # Arguments
    /// * `agent_id` - Agent identifier to query
    ///
    /// # Returns
    /// Result containing optional health status snapshot.
    pub fn get_agent_status(&self, agent_id: &str) -> Result<Option<AgentHealthStatus>> {
        let statuses = self.statuses.lock()
            .map_err(|_| LifecycleError::HealthStatusError("Failed to acquire status lock".to_string()))?;
        Ok(statuses.get(agent_id).cloned())
    }

    /// Update agent health state.
    ///
    /// # Arguments
    /// * `agent_id` - Agent identifier
    /// * `state` - New health state
    /// * `reason` - Optional failure reason
    ///
    /// # Returns
    /// Result indicating success or error.
    pub fn update_agent_state(&self, agent_id: String, state: HealthState, reason: Option<String>) -> Result<()> {
        let mut statuses = self.statuses.lock()
            .map_err(|_| LifecycleError::HealthStatusError("Failed to acquire status lock".to_string()))?;

        let status = statuses.entry(agent_id.clone())
            .or_insert_with(|| AgentHealthStatus::new(agent_id.clone()));

        status.update_state(state, reason.clone());
        drop(statuses);

        self.emit_event(HealthEvent {
            agent_id,
            state,
            reason,
            timestamp: SystemTime::now(),
            metrics: HealthMetrics::default(),
        })?;

        Ok(())
    }

    /// Update agent health metrics.
    ///
    /// # Arguments
    /// * `agent_id` - Agent identifier
    /// * `metrics` - Health metrics snapshot
    ///
    /// # Returns
    /// Result indicating success or error.
    pub fn update_agent_metrics(&self, agent_id: String, metrics: HealthMetrics) -> Result<()> {
        let mut statuses = self.statuses.lock()
            .map_err(|_| LifecycleError::HealthStatusError("Failed to acquire status lock".to_string()))?;

        if let Some(status) = statuses.get_mut(&agent_id) {
            status.update_metrics(metrics);
        }

        Ok(())
    }

    /// Get all agent statuses.
    ///
    /// # Returns
    /// Result containing vector of all health statuses.
    pub fn get_all_statuses(&self) -> Result<Vec<AgentHealthStatus>> {
        let statuses = self.statuses.lock()
            .map_err(|_| LifecycleError::HealthStatusError("Failed to acquire status lock".to_string()))?;
        Ok(statuses.values().cloned().collect())
    }

    /// Get aggregate health state across all agents.
    ///
    /// Returns Failed if any agent failed, Running if all are running,
    /// Stopped otherwise.
    pub fn aggregate_state(&self) -> Result<HealthState> {
        let statuses = self.statuses.lock()
            .map_err(|_| LifecycleError::HealthStatusError("Failed to acquire status lock".to_string()))?;

        if statuses.is_empty() {
            return Ok(HealthState::Stopped);
        }

        let mut all_running = true;
        for status in statuses.values() {
            if status.state == HealthState::Failed {
                return Ok(HealthState::Failed);
            }
            if !status.state.is_running() {
                all_running = false;
            }
        }

        Ok(if all_running { HealthState::Running } else { HealthState::Stopped })
    }

    /// Emit a health event to all registered listeners.
    ///
    /// # Arguments
    /// * `event` - Health event to emit
    ///
    /// # Returns
    /// Result indicating success or listener invocation error.
    pub fn emit_event(&self, event: HealthEvent) -> Result<()> {
        let listeners = self.listeners.lock()
            .map_err(|_| LifecycleError::HealthStatusError("Failed to acquire listener lock".to_string()))?;

        for listener in listeners.iter() {
            listener(event.clone());
        }

        Ok(())
    }

    /// Get count of running agents.
    pub fn running_count(&self) -> Result<usize> {
        let statuses = self.statuses.lock()
            .map_err(|_| LifecycleError::HealthStatusError("Failed to acquire status lock".to_string()))?;
        Ok(statuses.values().filter(|s| s.state.is_running()).count())
    }

    /// Get count of failed agents.
    pub fn failed_count(&self) -> Result<usize> {
        let statuses = self.statuses.lock()
            .map_err(|_| LifecycleError::HealthStatusError("Failed to acquire status lock".to_string()))?;
        Ok(statuses.values().filter(|s| s.state.is_failed()).count())
    }

    /// Check if any agent has failed.
    pub fn has_failed_agents(&self) -> Result<bool> {
        Ok(self.failed_count()? > 0)
    }
}

impl Default for HealthStatusAggregator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::string::ToString;
use alloc::sync::Arc;
use alloc::vec::Vec;

    #[test]
    fn test_health_state_enum() {
        assert!(HealthState::Running.is_running());
        assert!(!HealthState::Running.is_stopped());
        assert!(!HealthState::Running.is_failed());

        assert!(HealthState::Stopped.is_stopped());
        assert!(!HealthState::Stopped.is_running());

        assert!(HealthState::Failed.is_failed());
        assert!(!HealthState::Failed.is_running());
    }

    #[test]
    fn test_health_state_as_str() {
        assert_eq!(HealthState::Running.as_str(), "running");
        assert_eq!(HealthState::Stopped.as_str(), "stopped");
        assert_eq!(HealthState::Failed.as_str(), "failed");
    }

    #[test]
    fn test_health_event_creation() {
        let event = HealthEvent::new("agent-1".to_string(), HealthState::Running, None);
        assert_eq!(event.agent_id, "agent-1");
        assert_eq!(event.state, HealthState::Running);
        assert_eq!(event.reason, None);
    }

    #[test]
    fn test_health_event_running() {
        let event = HealthEvent::running("agent-1".to_string());
        assert_eq!(event.state, HealthState::Running);
        assert_eq!(event.reason, None);
    }

    #[test]
    fn test_health_event_failed() {
        let event = HealthEvent::failed("agent-1".to_string(), "Memory exhausted".to_string());
        assert_eq!(event.state, HealthState::Failed);
        assert_eq!(event.reason, Some("Memory exhausted".to_string()));
    }

    #[test]
    fn test_health_event_with_metrics() {
        let metrics = HealthMetrics::new()
            .with_cpu(50.5)
            .with_memory(1024 * 1024)
            .with_threads(8)
            .with_uptime(3600);

        let event = HealthEvent::running("agent-1".to_string())
            .with_metrics(metrics.clone());

        assert_eq!(event.metrics.cpu_percent, 50.5);
        assert_eq!(event.metrics.memory_bytes, 1024 * 1024);
        assert_eq!(event.metrics.thread_count, 8);
        assert_eq!(event.metrics.uptime_secs, 3600);
    }

    #[test]
    fn test_agent_health_status_creation() {
        let status = AgentHealthStatus::new("agent-1".to_string());
        assert_eq!(status.agent_id(), "agent-1");
        assert_eq!(status.state(), HealthState::Stopped);
        assert_eq!(status.reason(), None);
    }

    #[test]
    fn test_agent_health_status_update_state() {
        let mut status = AgentHealthStatus::new("agent-1".to_string());
        status.update_state(HealthState::Running, None);
        assert_eq!(status.state(), HealthState::Running);

        status.update_state(HealthState::Failed, Some("Crash detected".to_string()));
        assert_eq!(status.state(), HealthState::Failed);
        assert_eq!(status.reason(), Some("Crash detected"));
    }

    #[test]
    fn test_agent_health_status_update_metrics() {
        let mut status = AgentHealthStatus::new("agent-1".to_string());
        let metrics = HealthMetrics::new().with_cpu(75.0).with_memory(2048);
        status.update_metrics(metrics);
        assert_eq!(status.metrics().cpu_percent, 75.0);
        assert_eq!(status.metrics().memory_bytes, 2048);
    }

    #[test]
    fn test_agent_health_status_to_event() {
        let mut status = AgentHealthStatus::new("agent-1".to_string());
        status.update_state(HealthState::Running, None);
        let event = status.to_event();
        assert_eq!(event.agent_id, "agent-1");
        assert_eq!(event.state, HealthState::Running);
    }

    #[test]
    fn test_health_status_aggregator_register_agent() {
        let agg = HealthStatusAggregator::new();
        assert!(agg.register_agent("agent-1".to_string()).is_ok());
        let status = agg.get_agent_status("agent-1").unwrap();
        assert!(status.is_some());
    }

    #[test]
    fn test_health_status_aggregator_update_state() {
        let agg = HealthStatusAggregator::new();
        agg.register_agent("agent-1".to_string()).unwrap();
        agg.update_agent_state("agent-1".to_string(), HealthState::Running, None).unwrap();
        let status = agg.get_agent_status("agent-1").unwrap().unwrap();
        assert_eq!(status.state(), HealthState::Running);
    }

    #[test]
    fn test_health_status_aggregator_aggregate_state_empty() {
        let agg = HealthStatusAggregator::new();
        let state = agg.aggregate_state().unwrap();
        assert_eq!(state, HealthState::Stopped);
    }

    #[test]
    fn test_health_status_aggregator_aggregate_state_all_running() {
        let agg = HealthStatusAggregator::new();
        agg.register_agent("agent-1".to_string()).unwrap();
        agg.register_agent("agent-2".to_string()).unwrap();
        agg.update_agent_state("agent-1".to_string(), HealthState::Running, None).unwrap();
        agg.update_agent_state("agent-2".to_string(), HealthState::Running, None).unwrap();

        let state = agg.aggregate_state().unwrap();
        assert_eq!(state, HealthState::Running);
    }

    #[test]
    fn test_health_status_aggregator_aggregate_state_with_failure() {
        let agg = HealthStatusAggregator::new();
        agg.register_agent("agent-1".to_string()).unwrap();
        agg.register_agent("agent-2".to_string()).unwrap();
        agg.update_agent_state("agent-1".to_string(), HealthState::Running, None).unwrap();
        agg.update_agent_state("agent-2".to_string(), HealthState::Failed, Some("Crash".to_string())).unwrap();

        let state = agg.aggregate_state().unwrap();
        assert_eq!(state, HealthState::Failed);
    }

    #[test]
    fn test_health_status_aggregator_get_all_statuses() {
        let agg = HealthStatusAggregator::new();
        agg.register_agent("agent-1".to_string()).unwrap();
        agg.register_agent("agent-2".to_string()).unwrap();

        let statuses = agg.get_all_statuses().unwrap();
        assert_eq!(statuses.len(), 2);
    }

    #[test]
    fn test_health_status_aggregator_running_count() {
        let agg = HealthStatusAggregator::new();
        agg.register_agent("agent-1".to_string()).unwrap();
        agg.register_agent("agent-2".to_string()).unwrap();
        agg.update_agent_state("agent-1".to_string(), HealthState::Running, None).unwrap();

        let count = agg.running_count().unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_health_status_aggregator_failed_count() {
        let agg = HealthStatusAggregator::new();
        agg.register_agent("agent-1".to_string()).unwrap();
        agg.register_agent("agent-2".to_string()).unwrap();
        agg.update_agent_state("agent-1".to_string(), HealthState::Failed, Some("Error".to_string())).unwrap();

        let count = agg.failed_count().unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_health_metrics_builder() {
        let metrics = HealthMetrics::new()
            .with_cpu(65.0)
            .with_memory(5_000_000)
            .with_threads(16)
            .with_uptime(7200);

        assert_eq!(metrics.cpu_percent, 65.0);
        assert_eq!(metrics.memory_bytes, 5_000_000);
        assert_eq!(metrics.thread_count, 16);
        assert_eq!(metrics.uptime_secs, 7200);
    }

    #[test]
    fn test_health_event_listener_emission() {
        let agg = HealthStatusAggregator::new();
        let counter = Arc::new(Mutex::new(0));
        let counter_clone = counter.clone();

        agg.register_listener(move |_event| {
            *counter_clone.lock().unwrap() += 1;
        }).unwrap();

        agg.register_agent("agent-1".to_string()).unwrap();
        agg.update_agent_state("agent-1".to_string(), HealthState::Running, None).unwrap();

        let count = *counter.lock().unwrap();
        assert_eq!(count, 1);
    }
}
