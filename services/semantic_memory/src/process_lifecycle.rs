// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Memory Manager process lifecycle management.
//!
//! This module implements the lifecycle of the Memory Manager as a service process,
//! including state transitions, request serving, graceful shutdown, and health monitoring.
//!
//! # Process States
//!
//! The Memory Manager transitions through the following states:
//! - Initializing: Setting up physical memory, building free lists, tier structures
//! - Ready: Fully initialized, accepting requests
//! - Serving: Actively handling requests from CTs
//! - Draining: Accepting no new requests, completing pending operations
//! - ShuttingDown: Releasing resources, unmapping pages
//! - Terminated: Process has exited
//!
//! See Engineering Plan § 4.1.0: Process Lifecycle Management.

use alloc::string::String;
use alloc::vec::Vec;
use crate::error::{MemoryError, Result};
use crate::ipc_interface::{MemoryRequest, MemoryResponse};

/// Current state of the Memory Manager process.
///
/// Represents the state machine for the Memory Manager service.
/// See Engineering Plan § 4.1.0: State Machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MemoryManagerState {
    /// Initializing: scanning physical memory, building structures
    Initializing,
    /// Ready: fully initialized, accepting requests
    Ready,
    /// Serving: actively handling requests
    Serving,
    /// Draining: no new requests, finishing pending operations
    Draining,
    /// ShuttingDown: releasing resources
    ShuttingDown,
    /// Terminated: process has exited
    Terminated,
}

impl MemoryManagerState {
    /// Returns a human-readable name for this state.
    pub fn name(&self) -> &'static str {
        match self {
            MemoryManagerState::Initializing => "Initializing",
            MemoryManagerState::Ready => "Ready",
            MemoryManagerState::Serving => "Serving",
            MemoryManagerState::Draining => "Draining",
            MemoryManagerState::ShuttingDown => "ShuttingDown",
            MemoryManagerState::Terminated => "Terminated",
        }
    }

    /// Checks if this state can accept new requests.
    pub fn accepts_requests(&self) -> bool {
        matches!(
            self,
            MemoryManagerState::Ready | MemoryManagerState::Serving
        )
    }

    /// Checks if transitions to another state are allowed.
    pub fn can_transition_to(&self, next: &MemoryManagerState) -> bool {
        match (self, next) {
            // Initializing can transition to Ready
            (MemoryManagerState::Initializing, MemoryManagerState::Ready) => true,
            // Ready can transition to Serving
            (MemoryManagerState::Ready, MemoryManagerState::Serving) => true,
            // Serving can transition to Ready or Draining
            (MemoryManagerState::Serving, MemoryManagerState::Ready) => true,
            (MemoryManagerState::Serving, MemoryManagerState::Draining) => true,
            // Draining can transition to ShuttingDown
            (MemoryManagerState::Draining, MemoryManagerState::ShuttingDown) => true,
            // ShuttingDown can transition to Terminated
            (MemoryManagerState::ShuttingDown, MemoryManagerState::Terminated) => true,
            // No other transitions allowed
            _ => false,
        }
    }
}

/// Health status snapshot of the Memory Manager.
///
/// Provides observability into the process's health and performance.
/// See Engineering Plan § 4.1.0: Health Monitoring.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HealthStatus {
    /// Current process state
    pub state: MemoryManagerState,
    /// Uptime in milliseconds
    pub uptime_ms: u64,
    /// Total requests served since startup
    pub requests_served: u64,
    /// Total errors encountered
    pub error_count: u64,
    /// Current memory pressure level (0-100)
    pub memory_pressure_level: u8,
}

impl HealthStatus {
    /// Creates new health status.
    pub fn new(
        state: MemoryManagerState,
        uptime_ms: u64,
        requests: u64,
        errors: u64,
        pressure: u8,
    ) -> Self {
        HealthStatus {
            state,
            uptime_ms,
            requests_served: requests,
            error_count: errors,
            memory_pressure_level: pressure,
        }
    }

    /// Returns the error rate (errors per 1000 requests).
    pub fn error_rate_per_1000(&self) -> u64 {
        if self.requests_served == 0 {
            0
        } else {
            (self.error_count * 1000) / self.requests_served
        }
    }

    /// Returns whether the process is healthy (no critical issues).
    pub fn is_healthy(&self) -> bool {
        // Healthy if: error rate < 5%, pressure < 80%
        self.error_rate_per_1000() < 50 && self.memory_pressure_level < 80
    }
}

/// The Memory Manager process - handles the complete lifecycle.
///
/// Manages initialization, request serving, draining, and shutdown.
/// See Engineering Plan § 4.1.0: Memory Manager Process.
pub struct MemoryManagerProcess {
    /// Current process state
    state: MemoryManagerState,
    /// Uptime since process started (milliseconds)
    start_time_ms: u64,
    /// Total requests served
    total_requests: u64,
    /// Total errors
    total_errors: u64,
    /// Current memory pressure (0-100)
    memory_pressure: u8,
    /// Pending requests to drain before shutdown
    pending_requests: Vec<String>,
}

impl MemoryManagerProcess {
    /// Creates a new Memory Manager process in Initializing state.
    pub fn new() -> Self {
        MemoryManagerProcess {
            state: MemoryManagerState::Initializing,
            start_time_ms: 0,
            total_requests: 0,
            total_errors: 0,
            memory_pressure: 0,
            pending_requests: Vec::new(),
        }
    }

    /// Returns the current state.
    pub fn state(&self) -> &MemoryManagerState {
        &self.state
    }

    /// Transitions to a new state.
    ///
    /// # Arguments
    ///
    /// * `next_state` - The target state
    ///
    /// # Returns
    ///
    /// `Result<()>` if the transition is valid
    fn transition_to(&mut self, next_state: MemoryManagerState) -> Result<()> {
        if !self.state.can_transition_to(&next_state) {
            return Err(MemoryError::Other(format!(
                "invalid state transition: {} -> {}",
                self.state.name(),
                next_state.name()
            )));
        }

        self.state = next_state;
        Ok(())
    }

    /// Initializes the Memory Manager.
    ///
    /// Scans physical memory, builds free lists, and sets up tier structures.
    /// Must be called before the process can serve requests.
    ///
    /// # Returns
    ///
    /// `Result<()>` if initialization succeeded
    ///
    /// See Engineering Plan § 4.1.0: Initialization.
    pub fn initialize(&mut self) -> Result<()> {
        if self.state != MemoryManagerState::Initializing {
            return Err(MemoryError::Other(
                "process not in Initializing state".to_string(),
            ));
        }

        // Simulate initialization tasks:
        // 1. Scan physical memory layout
        // 2. Build L1, L2, L3 tier structures
        // 3. Initialize free lists
        // 4. Set up isolation boundaries
        // 5. Configure protection domains

        self.start_time_ms = 0; // In real impl, would be current time
        self.transition_to(MemoryManagerState::Ready)
    }

    /// Transitions to Serving state and serves a request.
    ///
    /// Processes a single request from a CT. Returns the response
    /// and updates statistics.
    ///
    /// # Arguments
    ///
    /// * `request` - The memory request to serve
    ///
    /// # Returns
    ///
    /// `Result<MemoryResponse>` with the response
    ///
    /// See Engineering Plan § 4.1.1: Request Processing.
    pub fn serve_request(&mut self, request: MemoryRequest) -> Result<MemoryResponse> {
        // Only serve if in Ready or Serving state
        if !self.state.accepts_requests() {
            return Err(MemoryError::Other(format!(
                "cannot serve request in {} state",
                self.state.name()
            )));
        }

        // Transition to Serving if in Ready
        if self.state == MemoryManagerState::Ready {
            self.transition_to(MemoryManagerState::Serving)?;
        }

        // Simulate request handling
        let response = match request {
            MemoryRequest::Allocate { .. } => {
                MemoryResponse::Allocated {
                    region_id: "alloc-001".to_string(),
                    mapped_addr: 0x1000,
                }
            }
            MemoryRequest::Read { .. } => {
                MemoryResponse::ReadData { data: Vec::new() }
            }
            MemoryRequest::Write { .. } => {
                MemoryResponse::WriteAck
            }
            MemoryRequest::Mount { .. } => {
                MemoryResponse::Mounted {
                    mount_id: "mount-001".to_string(),
                }
            }
            MemoryRequest::Query { .. } => {
                MemoryResponse::QueryResult {
                    stats: crate::ipc_interface::RegionStats::new(4096, 4096, 10, 50),
                }
            }
            MemoryRequest::Evict { .. } => {
                MemoryResponse::Evicted
            }
        };

        self.total_requests = self.total_requests.saturating_add(1);
        Ok(response)
    }

    /// Initiates graceful drain - stops accepting new requests.
    ///
    /// Transitions to Draining state and waits for pending requests to complete.
    /// After draining, the process can be shut down.
    ///
    /// # Returns
    ///
    /// `Result<()>` if drain initiated successfully
    ///
    /// See Engineering Plan § 4.1.0: Graceful Shutdown.
    pub fn drain(&mut self) -> Result<()> {
        if self.state == MemoryManagerState::Serving {
            self.transition_to(MemoryManagerState::Draining)?;
        } else if self.state != MemoryManagerState::Draining {
            return Err(MemoryError::Other(
                "can only drain from Serving or Draining state".to_string(),
            ));
        }

        Ok(())
    }

    /// Checks if draining is complete (all pending requests done).
    pub fn drain_complete(&self) -> bool {
        self.pending_requests.is_empty()
    }

    /// Shuts down the Memory Manager.
    ///
    /// Releases all resources, unmaps pages, and transitions to Terminated.
    /// Must be called after draining is complete.
    ///
    /// # Returns
    ///
    /// `Result<()>` if shutdown succeeded
    ///
    /// See Engineering Plan § 4.1.0: Shutdown.
    pub fn shutdown(&mut self) -> Result<()> {
        // Check if draining is complete
        if self.state == MemoryManagerState::Draining && !self.drain_complete() {
            return Err(MemoryError::Other(
                "drain not complete, pending requests remain".to_string(),
            ));
        }

        // Allow shutdown from Draining or Ready (fast path)
        if !matches!(
            self.state,
            MemoryManagerState::Draining | MemoryManagerState::Ready
        ) {
            return Err(MemoryError::Other(format!(
                "cannot shutdown from {} state",
                self.state.name()
            )));
        }

        // Perform shutdown tasks:
        // 1. Unmap all CT memory
        // 2. Release tier structures
        // 3. Flush caches
        // 4. Update persistent state

        self.transition_to(MemoryManagerState::ShuttingDown)?;
        self.transition_to(MemoryManagerState::Terminated)?;

        Ok(())
    }

    /// Returns the current health status.
    pub fn health_status(&self) -> HealthStatus {
        HealthStatus::new(
            self.state.clone(),
            self.start_time_ms,
            self.total_requests,
            self.total_errors,
            self.memory_pressure,
        )
    }

    /// Updates memory pressure level (0-100).
    pub fn set_memory_pressure(&mut self, pressure: u8) {
        self.memory_pressure = core::cmp::min(pressure, 100);
    }

    /// Records an error occurrence.
    pub fn record_error(&mut self) {
        self.total_errors = self.total_errors.saturating_add(1);
    }

    /// Returns current uptime in milliseconds.
    pub fn uptime_ms(&self) -> u64 {
        self.start_time_ms
    }

    /// Returns total requests served.
    pub fn total_requests_served(&self) -> u64 {
        self.total_requests
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;
use alloc::string::ToString;

    #[test]
    fn test_memory_manager_state_names() {
        assert_eq!(MemoryManagerState::Initializing.name(), "Initializing");
        assert_eq!(MemoryManagerState::Ready.name(), "Ready");
        assert_eq!(MemoryManagerState::Serving.name(), "Serving");
        assert_eq!(MemoryManagerState::Draining.name(), "Draining");
        assert_eq!(MemoryManagerState::ShuttingDown.name(), "ShuttingDown");
        assert_eq!(MemoryManagerState::Terminated.name(), "Terminated");
    }

    #[test]
    fn test_memory_manager_state_accepts_requests() {
        assert!(!MemoryManagerState::Initializing.accepts_requests());
        assert!(MemoryManagerState::Ready.accepts_requests());
        assert!(MemoryManagerState::Serving.accepts_requests());
        assert!(!MemoryManagerState::Draining.accepts_requests());
        assert!(!MemoryManagerState::ShuttingDown.accepts_requests());
        assert!(!MemoryManagerState::Terminated.accepts_requests());
    }

    #[test]
    fn test_memory_manager_state_transitions() {
        // Valid: Initializing -> Ready
        assert!(MemoryManagerState::Initializing.can_transition_to(&MemoryManagerState::Ready));

        // Valid: Ready -> Serving
        assert!(MemoryManagerState::Ready.can_transition_to(&MemoryManagerState::Serving));

        // Valid: Serving -> Draining
        assert!(MemoryManagerState::Serving.can_transition_to(&MemoryManagerState::Draining));

        // Invalid: Initializing -> Serving
        assert!(!MemoryManagerState::Initializing.can_transition_to(&MemoryManagerState::Serving));

        // Invalid: Ready -> ShuttingDown
        assert!(!MemoryManagerState::Ready.can_transition_to(&MemoryManagerState::ShuttingDown));
    }

    #[test]
    fn test_health_status_creation() {
        let status = HealthStatus::new(MemoryManagerState::Serving, 1000, 100, 5, 50);
        assert_eq!(status.state, MemoryManagerState::Serving);
        assert_eq!(status.uptime_ms, 1000);
        assert_eq!(status.requests_served, 100);
        assert_eq!(status.error_count, 5);
        assert_eq!(status.memory_pressure_level, 50);
    }

    #[test]
    fn test_health_status_error_rate() {
        let status = HealthStatus::new(MemoryManagerState::Serving, 1000, 1000, 10, 50);
        assert_eq!(status.error_rate_per_1000(), 10);

        let status_zero = HealthStatus::new(MemoryManagerState::Serving, 1000, 0, 0, 50);
        assert_eq!(status_zero.error_rate_per_1000(), 0);
    }

    #[test]
    fn test_health_status_is_healthy() {
        // Healthy: low error rate and low pressure
        let status = HealthStatus::new(MemoryManagerState::Serving, 1000, 1000, 10, 50);
        assert!(status.is_healthy());

        // Unhealthy: high error rate
        let status_bad_errors = HealthStatus::new(MemoryManagerState::Serving, 1000, 100, 10, 50);
        assert!(!status_bad_errors.is_healthy());

        // Unhealthy: high pressure
        let status_bad_pressure = HealthStatus::new(MemoryManagerState::Serving, 1000, 1000, 10, 85);
        assert!(!status_bad_pressure.is_healthy());
    }

    #[test]
    fn test_memory_manager_process_creation() {
        let process = MemoryManagerProcess::new();
        assert_eq!(process.state(), &MemoryManagerState::Initializing);
        assert_eq!(process.total_requests, 0);
        assert_eq!(process.total_errors, 0);
    }

    #[test]
    fn test_memory_manager_initialize_success() {
        let mut process = MemoryManagerProcess::new();
        assert!(process.initialize().is_ok());
        assert_eq!(process.state(), &MemoryManagerState::Ready);
    }

    #[test]
    fn test_memory_manager_initialize_wrong_state() {
        let mut process = MemoryManagerProcess::new();
        process.initialize().ok();
        // Try to initialize again
        let result = process.initialize();
        assert!(result.is_err());
    }

    #[test]
    fn test_memory_manager_serve_request_allocate() {
        let mut process = MemoryManagerProcess::new();
        process.initialize().ok();

        let request = MemoryRequest::Allocate {
            tier: crate::ipc_interface::MemoryTierSpec::L1,
            size: 1024,
            capability: "cap-001".to_string(),
        };

        let result = process.serve_request(request);
        assert!(result.is_ok());
        assert_eq!(process.total_requests, 1);
        assert_eq!(process.state(), &MemoryManagerState::Serving);
    }

    #[test]
    fn test_memory_manager_serve_request_read() {
        let mut process = MemoryManagerProcess::new();
        process.initialize().ok();

        let request = MemoryRequest::Read {
            region_id: "region-001".to_string(),
            offset: 0,
            len: 256,
            capability: "cap-001".to_string(),
        };

        let result = process.serve_request(request);
        assert!(result.is_ok());
        assert_eq!(process.total_requests, 1);
    }

    #[test]
    fn test_memory_manager_serve_request_not_ready() {
        let process = MemoryManagerProcess::new();
        // Still in Initializing state

        let request = MemoryRequest::Allocate {
            tier: crate::ipc_interface::MemoryTierSpec::L1,
            size: 1024,
            capability: "cap-001".to_string(),
        };

        let result = process.serve_request(request);
        assert!(result.is_err());
    }

    #[test]
    fn test_memory_manager_drain() {
        let mut process = MemoryManagerProcess::new();
        process.initialize().ok();
        process.serve_request(MemoryRequest::Allocate {
            tier: crate::ipc_interface::MemoryTierSpec::L1,
            size: 1024,
            capability: "cap-001".to_string(),
        }).ok();

        assert!(process.drain().is_ok());
        assert_eq!(process.state(), &MemoryManagerState::Draining);

        // Should not accept new requests
        let request = MemoryRequest::Allocate {
            tier: crate::ipc_interface::MemoryTierSpec::L1,
            size: 512,
            capability: "cap-001".to_string(),
        };
        assert!(process.serve_request(request).is_err());
    }

    #[test]
    fn test_memory_manager_shutdown_success() {
        let mut process = MemoryManagerProcess::new();
        process.initialize().ok();

        // Can shutdown from Ready state
        let result = process.shutdown();
        assert!(result.is_ok());
        assert_eq!(process.state(), &MemoryManagerState::Terminated);
    }

    #[test]
    fn test_memory_manager_shutdown_after_drain() {
        let mut process = MemoryManagerProcess::new();
        process.initialize().ok();
        process.serve_request(MemoryRequest::Allocate {
            tier: crate::ipc_interface::MemoryTierSpec::L1,
            size: 1024,
            capability: "cap-001".to_string(),
        }).ok();

        process.drain().ok();
        let result = process.shutdown();
        assert!(result.is_ok());
        assert_eq!(process.state(), &MemoryManagerState::Terminated);
    }

    #[test]
    fn test_memory_manager_health_status() {
        let mut process = MemoryManagerProcess::new();
        process.initialize().ok();

        let status = process.health_status();
        assert_eq!(status.state, MemoryManagerState::Ready);
        assert_eq!(status.requests_served, 0);
        assert_eq!(status.error_count, 0);
    }

    #[test]
    fn test_memory_manager_set_memory_pressure() {
        let mut process = MemoryManagerProcess::new();
        process.set_memory_pressure(75);
        assert_eq!(process.memory_pressure, 75);

        // Caps at 100
        process.set_memory_pressure(150);
        assert_eq!(process.memory_pressure, 100);
    }

    #[test]
    fn test_memory_manager_record_error() {
        let mut process = MemoryManagerProcess::new();
        assert_eq!(process.total_errors, 0);

        process.record_error();
        assert_eq!(process.total_errors, 1);

        process.record_error();
        assert_eq!(process.total_errors, 2);
    }
}
