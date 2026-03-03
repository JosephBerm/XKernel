// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! Health check and probe mechanisms.
//!
//! Defines types for agent health monitoring including readiness and liveness probes,
//! health check types, and probe results. Supports HTTP, TCP, EXEC, CSCI syscall,
//! and custom health check handlers.
//!
//! Reference: Engineering Plan § Agent Lifecycle Management § Health Checks

use std::collections::VecDeque;

/// Probe type for health endpoint checks.
///
/// Determines the protocol and method used to probe agent health.
/// Supports HTTP, TCP, command execution, CSCI syscalls, and custom gRPC handlers.
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Health Probes
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthProbeType {
    /// HTTP GET request to endpoint.
    ///
    /// Checks that endpoint returns 2xx status code.
    /// Example: `HttpGet("http://localhost:8080/health")`
    HttpGet(String),

    /// TCP connection to port.
    ///
    /// Checks that port accepts connections.
    /// Example: `Tcp(8080)`
    Tcp(u16),

    /// Execute shell command and check exit code.
    ///
    /// Exit code 0 indicates healthy.
    /// Example: `Exec("/opt/agent/health_check.sh")`
    Exec(String),

    /// CSCI kernel syscall for health check.
    ///
    /// Invokes named CSCI syscall; success indicates healthy.
    /// Example: `Csci("cs_agent_probe")`
    Csci(String),

    /// Custom gRPC handler for health check.
    ///
    /// Calls custom gRPC service; used for complex health logic.
    /// Example: `CustomGrpc("grpc://localhost:9090/Health")`
    CustomGrpc(String),
}

impl HealthProbeType {
    /// Returns true if this is an HTTP GET probe.
    pub fn is_http_get(&self) -> bool {
        matches!(self, Self::HttpGet(_))
    }

    /// Returns true if this is a TCP probe.
    pub fn is_tcp(&self) -> bool {
        matches!(self, Self::Tcp(_))
    }

    /// Returns true if this is an Exec probe.
    pub fn is_exec(&self) -> bool {
        matches!(self, Self::Exec(_))
    }

    /// Returns true if this is a CSCI syscall probe.
    pub fn is_csci(&self) -> bool {
        matches!(self, Self::Csci(_))
    }

    /// Returns true if this is a custom gRPC probe.
    pub fn is_custom_grpc(&self) -> bool {
        matches!(self, Self::CustomGrpc(_))
    }
}

/// Result of a single health probe execution.
///
/// Captures probe outcome including health status and latency measurement.
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Health Probes
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeResult {
    /// Agent is healthy.
    ///
    /// Contains round-trip latency in nanoseconds.
    Healthy(u64),

    /// Agent is unhealthy.
    ///
    /// Contains reason describing the failure.
    Unhealthy(String),

    /// Probe result is unknown (timeout, error, etc.).
    Unknown,

    /// Probe timed out.
    Timeout,
}

impl ProbeResult {
    /// Returns true if probe indicates healthy status.
    pub fn is_healthy(&self) -> bool {
        matches!(self, Self::Healthy(_))
    }

    /// Returns true if probe indicates unhealthy status.
    pub fn is_unhealthy(&self) -> bool {
        matches!(self, Self::Unhealthy(_))
    }

    /// Returns true if probe result is unknown.
    pub fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown)
    }

    /// Returns true if probe timed out.
    pub fn is_timeout(&self) -> bool {
        matches!(self, Self::Timeout)
    }

    /// Extracts latency in nanoseconds if healthy, returns None otherwise.
    pub fn latency_ns(&self) -> Option<u64> {
        match self {
            Self::Healthy(ns) => Some(*ns),
            _ => None,
        }
    }
}

/// Health endpoint specification for agent health assessment.
///
/// Defines a specific health endpoint including its type, location, and probe parameters.
/// Supports multiple endpoint types for flexibility in different agent environments.
///
/// # Fields
///
/// - `endpoint_type`: Protocol/mechanism for health check (HTTP, TCP, Exec, CSCI, CustomGrpc)
/// - `address`: Network address (for HTTP/TCP) or identifier (for CSCI/CustomGrpc)
/// - `path`: Path component (for HTTP endpoints)
/// - `timeout_ms`: Maximum time to wait for probe response
/// - `interval_ms`: Time between probe attempts
/// - `failure_threshold`: Consecutive failures before marking unhealthy
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Health Probes
#[derive(Debug, Clone)]
pub struct HealthEndpoint {
    /// Type of health probe to perform.
    pub endpoint_type: HealthProbeType,

    /// Network address or service identifier.
    ///
    /// For HTTP: "localhost" or "api.example.com"
    /// For TCP: "localhost"
    /// For CSCI: syscall service name
    pub address: String,

    /// Path component for HTTP endpoints.
    ///
    /// Example: "/health" or "/readiness"
    /// Ignored for non-HTTP endpoints.
    pub path: String,

    /// Timeout in milliseconds for probe execution.
    pub timeout_ms: u64,

    /// Interval in milliseconds between probe attempts.
    pub interval_ms: u64,

    /// Number of consecutive failures before marking unhealthy.
    pub failure_threshold: u32,
}

impl HealthEndpoint {
    /// Creates a new HTTP health endpoint.
    pub fn http(address: impl Into<String>, path: impl Into<String>) -> Self {
        let address_str: String = address.into();
        let path_str: String = path.into();
        Self {
            endpoint_type: HealthProbeType::HttpGet(format!(
                "http://{}{}",
                address_str,
                path_str
            )),
            address: address_str,
            path: path_str,
            timeout_ms: 5000,
            interval_ms: 10000,
            failure_threshold: 3,
        }
    }

    /// Creates a new TCP health endpoint.
    pub fn tcp(address: impl Into<String>, port: u16) -> Self {
        Self {
            endpoint_type: HealthProbeType::Tcp(port),
            address: address.into(),
            path: String::new(),
            timeout_ms: 5000,
            interval_ms: 10000,
            failure_threshold: 3,
        }
    }

    /// Creates a new EXEC health endpoint.
    pub fn exec(command: impl Into<String>) -> Self {
        Self {
            endpoint_type: HealthProbeType::Exec(command.into()),
            address: String::new(),
            path: String::new(),
            timeout_ms: 5000,
            interval_ms: 10000,
            failure_threshold: 3,
        }
    }

    /// Creates a new CSCI syscall health endpoint.
    pub fn csci(syscall_name: impl Into<String>) -> Self {
        Self {
            endpoint_type: HealthProbeType::Csci(syscall_name.into()),
            address: String::new(),
            path: String::new(),
            timeout_ms: 5000,
            interval_ms: 10000,
            failure_threshold: 3,
        }
    }

    /// Creates a new custom gRPC health endpoint.
    pub fn custom_grpc(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint_type: HealthProbeType::CustomGrpc(endpoint.into()),
            address: String::new(),
            path: String::new(),
            timeout_ms: 5000,
            interval_ms: 10000,
            failure_threshold: 3,
        }
    }

    /// Sets the timeout in milliseconds.
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Sets the interval in milliseconds.
    pub fn with_interval(mut self, interval_ms: u64) -> Self {
        self.interval_ms = interval_ms;
        self
    }

    /// Sets the failure threshold.
    pub fn with_failure_threshold(mut self, threshold: u32) -> Self {
        self.failure_threshold = threshold;
        self
    }
}

/// Probe schedule controlling timing of health checks.
///
/// Specifies when and how often to execute health probes with timing
/// parameters and success/failure thresholds.
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Health Probes
#[derive(Debug, Clone)]
pub struct ProbeSchedule {
    /// Interval in milliseconds between probe executions.
    pub interval_ms: u64,

    /// Initial delay in milliseconds before first probe.
    ///
    /// Allows agent time to initialize before health checking begins.
    pub initial_delay_ms: u64,

    /// Timeout in milliseconds for each probe execution.
    pub timeout_ms: u64,

    /// Consecutive successes required to mark agent healthy.
    pub success_threshold: u32,

    /// Consecutive failures required to mark agent unhealthy.
    pub failure_threshold: u32,
}

impl ProbeSchedule {
    /// Creates a new probe schedule with default parameters.
    pub fn new() -> Self {
        Self {
            interval_ms: 10000,
            initial_delay_ms: 0,
            timeout_ms: 5000,
            success_threshold: 1,
            failure_threshold: 3,
        }
    }

    /// Sets the interval in milliseconds.
    pub fn with_interval(mut self, interval_ms: u64) -> Self {
        self.interval_ms = interval_ms;
        self
    }

    /// Sets the initial delay in milliseconds.
    pub fn with_initial_delay(mut self, initial_delay_ms: u64) -> Self {
        self.initial_delay_ms = initial_delay_ms;
        self
    }

    /// Sets the timeout in milliseconds.
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Sets the success threshold.
    pub fn with_success_threshold(mut self, threshold: u32) -> Self {
        self.success_threshold = threshold;
        self
    }

    /// Sets the failure threshold.
    pub fn with_failure_threshold(mut self, threshold: u32) -> Self {
        self.failure_threshold = threshold;
        self
    }
}

impl Default for ProbeSchedule {
    fn default() -> Self {
        Self::new()
    }
}

/// Health history tracking recent probe results and failure counts.
///
/// Maintains a circular buffer of recent health check results and tracks
/// consecutive failure counts for threshold evaluation.
///
/// # Fields
///
/// - `recent_results`: Circular buffer of last N probe results
/// - `consecutive_failures`: Count of consecutive failures
/// - `last_healthy_timestamp`: Timestamp of last successful probe (milliseconds)
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Health Probes
#[derive(Debug, Clone)]
pub struct HealthHistory {
    /// Circular buffer of recent probe results (limited to 100 most recent).
    pub recent_results: VecDeque<ProbeResult>,

    /// Number of consecutive failures.
    pub consecutive_failures: u32,

    /// Timestamp in milliseconds of last successful probe.
    pub last_healthy_timestamp: u64,
}

impl HealthHistory {
    /// Creates a new empty health history.
    pub fn new() -> Self {
        Self {
            recent_results: VecDeque::with_capacity(100),
            consecutive_failures: 0,
            last_healthy_timestamp: 0,
        }
    }

    /// Records a probe result and updates internal state.
    ///
    /// Updates consecutive failure count based on result.
    /// Maintains circular buffer of results.
    pub fn record_result(&mut self, result: ProbeResult, current_timestamp_ms: u64) {
        if result.is_healthy() {
            self.consecutive_failures = 0;
            self.last_healthy_timestamp = current_timestamp_ms;
        } else {
            self.consecutive_failures += 1;
        }

        // Maintain circular buffer of max 100 results
        if self.recent_results.len() >= 100 {
            self.recent_results.pop_front();
        }
        self.recent_results.push_back(result);
    }

    /// Returns the total number of stored results.
    pub fn result_count(&self) -> usize {
        self.recent_results.len()
    }

    /// Returns the number of recent consecutive failures.
    pub fn failure_count(&self) -> u32 {
        self.consecutive_failures
    }

    /// Checks if threshold has been exceeded.
    pub fn threshold_exceeded(&self, threshold: u32) -> bool {
        self.consecutive_failures >= threshold
    }

    /// Clears the history.
    pub fn clear(&mut self) {
        self.recent_results.clear();
        self.consecutive_failures = 0;
        self.last_healthy_timestamp = 0;
    }
}

impl Default for HealthHistory {
    fn default() -> Self {
        Self::new()
    }
}

/// Overall health status of an agent.
///
/// Represents the aggregated health state combining information from
/// readiness, liveness, and startup probes.
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Health Probes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentHealthStatus {
    /// Agent is fully healthy and ready.
    ///
    /// All probes passing, agent can accept work.
    Healthy,

    /// Agent is degraded but still operational.
    ///
    /// Some health checks failing but below threshold;
    /// agent continues operating with reduced capacity.
    Degraded,

    /// Agent is unhealthy and cannot accept work.
    ///
    /// One or more probes exceeded failure threshold.
    /// Restart policy may be invoked.
    Unhealthy,

    /// Agent health status is unknown.
    ///
    /// Insufficient information to determine health;
    /// probes not yet executed or all probes timed out.
    Unknown,
}

impl AgentHealthStatus {
    /// Returns true if agent is healthy.
    pub fn is_healthy(&self) -> bool {
        matches!(self, Self::Healthy)
    }

    /// Returns true if agent is degraded.
    pub fn is_degraded(&self) -> bool {
        matches!(self, Self::Degraded)
    }

    /// Returns true if agent is unhealthy.
    pub fn is_unhealthy(&self) -> bool {
        matches!(self, Self::Unhealthy)
    }

    /// Returns true if agent health is unknown.
    pub fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown)
    }

    /// Returns true if agent can accept work (healthy or degraded).
    pub fn is_operational(&self) -> bool {
        matches!(self, Self::Healthy | Self::Degraded)
    }
}

/// Health check type determining how agent health is assessed.
///
/// Supports multiple health check mechanisms aligned with Kubernetes patterns:
/// - **HTTP**: Check HTTP endpoint for healthy response
/// - **TCP**: Verify TCP port is accepting connections
/// - **Exec**: Execute command and check exit code
/// - **CsciSyscall**: Use CSCI syscall for kernel-level health check
/// - **Custom**: Handler reference for custom health check logic
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Health Checks
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthCheckType {
    /// HTTP endpoint health check.
    ///
    /// Agent exposes an HTTP endpoint that returns 200 for healthy status.
    /// Example: `http://localhost:8080/health`
    Http(String),

    /// TCP port health check.
    ///
    /// Agent listens on a TCP port; successful connection indicates healthy.
    /// Example: `tcp:8080` means port 8080
    Tcp(u16),

    /// Execute command health check.
    ///
    /// Run a shell command; exit code 0 indicates healthy.
    /// Example: `/opt/agent/health_check.sh`
    Exec(String),

    /// CSCI syscall health check.
    ///
    /// Invoke a CSCI syscall by name; successful invocation indicates healthy.
    /// Example: `cs_agent_probe`
    CsciSyscall(String),

    /// Custom handler reference for health check.
    ///
    /// Reference to a handler function or service that implements custom
    /// health check logic specific to the agent.
    Custom(String),
}

impl HealthCheckType {
    /// Returns true if this is an HTTP health check.
    pub fn is_http(&self) -> bool {
        matches!(self, Self::Http(_))
    }

    /// Returns true if this is a TCP health check.
    pub fn is_tcp(&self) -> bool {
        matches!(self, Self::Tcp(_))
    }

    /// Returns true if this is an EXEC health check.
    pub fn is_exec(&self) -> bool {
        matches!(self, Self::Exec(_))
    }

    /// Returns true if this is a CSCI syscall health check.
    pub fn is_csci_syscall(&self) -> bool {
        matches!(self, Self::CsciSyscall(_))
    }

    /// Returns true if this is a custom health check.
    pub fn is_custom(&self) -> bool {
        matches!(self, Self::Custom(_))
    }
}

/// Health check configuration.
///
/// Specifies how and how often to assess agent health.
///
/// # Fields
///
/// - `check_type`: What type of health check to perform
/// - `interval_ms`: How often to run the check (milliseconds)
/// - `timeout_ms`: Maximum time to wait for check result (milliseconds)
/// - `failure_threshold`: Consecutive failures before marking unhealthy
/// - `success_threshold`: Consecutive successes before marking healthy
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Health Checks
#[derive(Debug, Clone)]
pub struct HealthCheckConfig {
    /// Type of health check to perform.
    pub check_type: HealthCheckType,

    /// Interval in milliseconds between health checks.
    pub interval_ms: u64,

    /// Timeout in milliseconds for each health check execution.
    pub timeout_ms: u64,

    /// Number of consecutive failures before marking agent unhealthy.
    pub failure_threshold: u32,

    /// Number of consecutive successes before marking agent healthy.
    pub success_threshold: u32,
}

impl HealthCheckConfig {
    /// Creates a new HTTP health check configuration with sensible defaults.
    ///
    /// Defaults:
    /// - interval_ms: 10000 (10 seconds)
    /// - timeout_ms: 5000 (5 seconds)
    /// - failure_threshold: 3
    /// - success_threshold: 1
    ///
    /// Reference: Engineering Plan § Agent Lifecycle Management § Health Checks
    pub fn new_http(endpoint: impl Into<String>) -> Self {
        Self {
            check_type: HealthCheckType::Http(endpoint.into()),
            interval_ms: 10000,
            timeout_ms: 5000,
            failure_threshold: 3,
            success_threshold: 1,
        }
    }

    /// Creates a new TCP health check configuration with sensible defaults.
    pub fn new_tcp(port: u16) -> Self {
        Self {
            check_type: HealthCheckType::Tcp(port),
            interval_ms: 10000,
            timeout_ms: 5000,
            failure_threshold: 3,
            success_threshold: 1,
        }
    }

    /// Creates a new EXEC health check configuration with sensible defaults.
    pub fn new_exec(command: impl Into<String>) -> Self {
        Self {
            check_type: HealthCheckType::Exec(command.into()),
            interval_ms: 10000,
            timeout_ms: 5000,
            failure_threshold: 3,
            success_threshold: 1,
        }
    }

    /// Sets the check interval in milliseconds.
    pub fn with_interval(mut self, interval_ms: u64) -> Self {
        self.interval_ms = interval_ms;
        self
    }

    /// Sets the check timeout in milliseconds.
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Sets the failure threshold.
    pub fn with_failure_threshold(mut self, threshold: u32) -> Self {
        self.failure_threshold = threshold;
        self
    }

    /// Sets the success threshold.
    pub fn with_success_threshold(mut self, threshold: u32) -> Self {
        self.success_threshold = threshold;
        self
    }
}

// ProbeResult already defined above

// ProbeResult methods already defined above

/// Health probe for agent readiness or liveness.
///
/// Represents a complete probe specification for determining agent readiness
/// (ready to accept work) or liveness (still alive and responsive).
///
/// # Use Cases
///
/// - **Readiness Probe**: Determines if agent is ready to handle requests.
///   After passing readiness probe, agent transitions from Starting to Running.
///   Used to delay traffic until initialization is complete.
///
/// - **Liveness Probe**: Determines if agent is still alive and responsive.
///   Failing liveness probe may trigger restart or transition to Degraded state.
///   Prevents stuck agents from blocking the system.
///
/// # Fields
///
/// - `config`: The health check configuration
/// - `initial_delay_ms`: Time to wait before first probe (milliseconds)
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Health Checks
#[derive(Debug, Clone)]
pub struct HealthProbe {
    /// Health check configuration for this probe.
    pub config: HealthCheckConfig,

    /// Initial delay in milliseconds before first probe execution.
    ///
    /// Allows agent time to fully initialize before health checks begin.
    /// Prevents spurious failures during startup.
    pub initial_delay_ms: u64,
}

impl HealthProbe {
    /// Creates a new readiness probe with HTTP health check.
    ///
    /// Suitable for agents that expose an HTTP health endpoint.
    ///
    /// Arguments:
    /// - `endpoint`: HTTP endpoint URL (e.g., "http://localhost:8080/health")
    /// - `initial_delay_ms`: Time to wait before first check
    ///
    /// Reference: Engineering Plan § Agent Lifecycle Management § Health Checks
    pub fn readiness_http(endpoint: impl Into<String>, initial_delay_ms: u64) -> Self {
        Self {
            config: HealthCheckConfig::new_http(endpoint),
            initial_delay_ms,
        }
    }

    /// Creates a new liveness probe with HTTP health check.
    pub fn liveness_http(endpoint: impl Into<String>, initial_delay_ms: u64) -> Self {
        Self {
            config: HealthCheckConfig::new_http(endpoint),
            initial_delay_ms,
        }
    }

    /// Creates a new readiness probe with TCP health check.
    ///
    /// Suitable for agents that listen on a TCP port.
    pub fn readiness_tcp(port: u16, initial_delay_ms: u64) -> Self {
        Self {
            config: HealthCheckConfig::new_tcp(port),
            initial_delay_ms,
        }
    }

    /// Creates a new liveness probe with TCP health check.
    pub fn liveness_tcp(port: u16, initial_delay_ms: u64) -> Self {
        Self {
            config: HealthCheckConfig::new_tcp(port),
            initial_delay_ms,
        }
    }

    /// Creates a new readiness probe with EXEC health check.
    ///
    /// Suitable for agents with custom health check scripts.
    pub fn readiness_exec(command: impl Into<String>, initial_delay_ms: u64) -> Self {
        Self {
            config: HealthCheckConfig::new_exec(command),
            initial_delay_ms,
        }
    }

    /// Creates a new liveness probe with EXEC health check.
    pub fn liveness_exec(command: impl Into<String>, initial_delay_ms: u64) -> Self {
        Self {
            config: HealthCheckConfig::new_exec(command),
            initial_delay_ms,
        }
    }

    /// Sets the check interval.
    pub fn with_interval(mut self, interval_ms: u64) -> Self {
        self.config = self.config.with_interval(interval_ms);
        self
    }

    /// Sets the check timeout.
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.config = self.config.with_timeout(timeout_ms);
        self
    }

    /// Sets the failure threshold.
    pub fn with_failure_threshold(mut self, threshold: u32) -> Self {
        self.config = self.config.with_failure_threshold(threshold);
        self
    }

    /// Sets the success threshold.
    pub fn with_success_threshold(mut self, threshold: u32) -> Self {
        self.config = self.config.with_success_threshold(threshold);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // HealthProbeType tests
    #[test]
    fn test_health_probe_type_http_get() {
        let probe_type = HealthProbeType::HttpGet("http://localhost:8080/health".to_string());
        assert!(probe_type.is_http_get());
        assert!(!probe_type.is_tcp());
        assert!(!probe_type.is_exec());
        assert!(!probe_type.is_csci());
        assert!(!probe_type.is_custom_grpc());
    }

    #[test]
    fn test_health_probe_type_tcp() {
        let probe_type = HealthProbeType::Tcp(8080);
        assert!(!probe_type.is_http_get());
        assert!(probe_type.is_tcp());
        assert!(!probe_type.is_exec());
        assert!(!probe_type.is_csci());
        assert!(!probe_type.is_custom_grpc());
    }

    #[test]
    fn test_health_probe_type_exec() {
        let probe_type = HealthProbeType::Exec("/opt/agent/health.sh".to_string());
        assert!(!probe_type.is_http_get());
        assert!(!probe_type.is_tcp());
        assert!(probe_type.is_exec());
        assert!(!probe_type.is_csci());
        assert!(!probe_type.is_custom_grpc());
    }

    #[test]
    fn test_health_probe_type_csci() {
        let probe_type = HealthProbeType::Csci("cs_agent_probe".to_string());
        assert!(!probe_type.is_http_get());
        assert!(!probe_type.is_tcp());
        assert!(!probe_type.is_exec());
        assert!(probe_type.is_csci());
        assert!(!probe_type.is_custom_grpc());
    }

    #[test]
    fn test_health_probe_type_custom_grpc() {
        let probe_type = HealthProbeType::CustomGrpc("grpc://localhost:9090/Health".to_string());
        assert!(!probe_type.is_http_get());
        assert!(!probe_type.is_tcp());
        assert!(!probe_type.is_exec());
        assert!(!probe_type.is_csci());
        assert!(probe_type.is_custom_grpc());
    }

    // ProbeResult tests
    #[test]
    fn test_probe_result_healthy() {
        let result = ProbeResult::Healthy(1500000);
        assert!(result.is_healthy());
        assert!(!result.is_unhealthy());
        assert!(!result.is_unknown());
        assert!(!result.is_timeout());
        assert_eq!(result.latency_ns(), Some(1500000));
    }

    #[test]
    fn test_probe_result_unhealthy() {
        let result = ProbeResult::Unhealthy("HTTP 500".to_string());
        assert!(!result.is_healthy());
        assert!(result.is_unhealthy());
        assert!(!result.is_unknown());
        assert!(!result.is_timeout());
        assert!(result.latency_ns().is_none());
    }

    #[test]
    fn test_probe_result_unknown() {
        let result = ProbeResult::Unknown;
        assert!(!result.is_healthy());
        assert!(!result.is_unhealthy());
        assert!(result.is_unknown());
        assert!(!result.is_timeout());
        assert!(result.latency_ns().is_none());
    }

    #[test]
    fn test_probe_result_timeout() {
        let result = ProbeResult::Timeout;
        assert!(!result.is_healthy());
        assert!(!result.is_unhealthy());
        assert!(!result.is_unknown());
        assert!(result.is_timeout());
        assert!(result.latency_ns().is_none());
    }

    // HealthEndpoint tests
    #[test]
    fn test_health_endpoint_http() {
        let endpoint = HealthEndpoint::http("localhost", "/health");
        assert!(endpoint.endpoint_type.is_http_get());
        assert_eq!(endpoint.address, "localhost");
        assert_eq!(endpoint.path, "/health");
        assert_eq!(endpoint.timeout_ms, 5000);
        assert_eq!(endpoint.interval_ms, 10000);
        assert_eq!(endpoint.failure_threshold, 3);
    }

    #[test]
    fn test_health_endpoint_tcp() {
        let endpoint = HealthEndpoint::tcp("localhost", 8080);
        assert!(endpoint.endpoint_type.is_tcp());
        assert_eq!(endpoint.address, "localhost");
        assert_eq!(endpoint.timeout_ms, 5000);
    }

    #[test]
    fn test_health_endpoint_exec() {
        let endpoint = HealthEndpoint::exec("/opt/check.sh");
        assert!(endpoint.endpoint_type.is_exec());
        assert_eq!(endpoint.timeout_ms, 5000);
    }

    #[test]
    fn test_health_endpoint_csci() {
        let endpoint = HealthEndpoint::csci("cs_probe");
        assert!(endpoint.endpoint_type.is_csci());
        assert_eq!(endpoint.timeout_ms, 5000);
    }

    #[test]
    fn test_health_endpoint_custom_grpc() {
        let endpoint = HealthEndpoint::custom_grpc("grpc://localhost:9090/Health");
        assert!(endpoint.endpoint_type.is_custom_grpc());
        assert_eq!(endpoint.timeout_ms, 5000);
    }

    #[test]
    fn test_health_endpoint_builder() {
        let endpoint = HealthEndpoint::http("api.local", "/ready")
            .with_timeout(3000)
            .with_interval(5000)
            .with_failure_threshold(5);

        assert_eq!(endpoint.timeout_ms, 3000);
        assert_eq!(endpoint.interval_ms, 5000);
        assert_eq!(endpoint.failure_threshold, 5);
    }

    // ProbeSchedule tests
    #[test]
    fn test_probe_schedule_new() {
        let schedule = ProbeSchedule::new();
        assert_eq!(schedule.interval_ms, 10000);
        assert_eq!(schedule.initial_delay_ms, 0);
        assert_eq!(schedule.timeout_ms, 5000);
        assert_eq!(schedule.success_threshold, 1);
        assert_eq!(schedule.failure_threshold, 3);
    }

    #[test]
    fn test_probe_schedule_builder() {
        let schedule = ProbeSchedule::new()
            .with_interval(5000)
            .with_initial_delay(2000)
            .with_timeout(3000)
            .with_success_threshold(2)
            .with_failure_threshold(5);

        assert_eq!(schedule.interval_ms, 5000);
        assert_eq!(schedule.initial_delay_ms, 2000);
        assert_eq!(schedule.timeout_ms, 3000);
        assert_eq!(schedule.success_threshold, 2);
        assert_eq!(schedule.failure_threshold, 5);
    }

    // HealthHistory tests
    #[test]
    fn test_health_history_new() {
        let history = HealthHistory::new();
        assert_eq!(history.result_count(), 0);
        assert_eq!(history.failure_count(), 0);
        assert_eq!(history.last_healthy_timestamp, 0);
    }

    #[test]
    fn test_health_history_record_healthy() {
        let mut history = HealthHistory::new();
        history.record_result(ProbeResult::Healthy(1500000), 1000);

        assert_eq!(history.result_count(), 1);
        assert_eq!(history.failure_count(), 0);
        assert_eq!(history.last_healthy_timestamp, 1000);
    }

    #[test]
    fn test_health_history_record_unhealthy() {
        let mut history = HealthHistory::new();
        history.record_result(ProbeResult::Unhealthy("error".to_string()), 1000);

        assert_eq!(history.result_count(), 1);
        assert_eq!(history.failure_count(), 1);
        assert_eq!(history.last_healthy_timestamp, 0);
    }

    #[test]
    fn test_health_history_consecutive_failures() {
        let mut history = HealthHistory::new();
        history.record_result(ProbeResult::Unhealthy("error".to_string()), 1000);
        history.record_result(ProbeResult::Unhealthy("error".to_string()), 2000);
        history.record_result(ProbeResult::Unhealthy("error".to_string()), 3000);

        assert_eq!(history.failure_count(), 3);
        assert!(history.threshold_exceeded(3));
        assert!(!history.threshold_exceeded(4));
    }

    #[test]
    fn test_health_history_failure_reset_on_healthy() {
        let mut history = HealthHistory::new();
        history.record_result(ProbeResult::Unhealthy("error".to_string()), 1000);
        history.record_result(ProbeResult::Unhealthy("error".to_string()), 2000);
        assert_eq!(history.failure_count(), 2);

        history.record_result(ProbeResult::Healthy(1500000), 3000);
        assert_eq!(history.failure_count(), 0);
        assert_eq!(history.last_healthy_timestamp, 3000);
    }

    #[test]
    fn test_health_history_circular_buffer() {
        let mut history = HealthHistory::new();
        // Add 105 results (exceeds 100 capacity)
        for i in 0..105 {
            let result = if i % 2 == 0 {
                ProbeResult::Healthy(1000000)
            } else {
                ProbeResult::Unhealthy("error".to_string())
            };
            history.record_result(result, i as u64);
        }

        // Should maintain max 100 results
        assert_eq!(history.result_count(), 100);
    }

    #[test]
    fn test_health_history_clear() {
        let mut history = HealthHistory::new();
        history.record_result(ProbeResult::Healthy(1500000), 1000);
        history.record_result(ProbeResult::Unhealthy("error".to_string()), 2000);

        history.clear();
        assert_eq!(history.result_count(), 0);
        assert_eq!(history.failure_count(), 0);
        assert_eq!(history.last_healthy_timestamp, 0);
    }

    // AgentHealthStatus tests
    #[test]
    fn test_agent_health_status_healthy() {
        let status = AgentHealthStatus::Healthy;
        assert!(status.is_healthy());
        assert!(!status.is_degraded());
        assert!(!status.is_unhealthy());
        assert!(!status.is_unknown());
        assert!(status.is_operational());
    }

    #[test]
    fn test_agent_health_status_degraded() {
        let status = AgentHealthStatus::Degraded;
        assert!(!status.is_healthy());
        assert!(status.is_degraded());
        assert!(!status.is_unhealthy());
        assert!(!status.is_unknown());
        assert!(status.is_operational());
    }

    #[test]
    fn test_agent_health_status_unhealthy() {
        let status = AgentHealthStatus::Unhealthy;
        assert!(!status.is_healthy());
        assert!(!status.is_degraded());
        assert!(status.is_unhealthy());
        assert!(!status.is_unknown());
        assert!(!status.is_operational());
    }

    #[test]
    fn test_agent_health_status_unknown() {
        let status = AgentHealthStatus::Unknown;
        assert!(!status.is_healthy());
        assert!(!status.is_degraded());
        assert!(!status.is_unhealthy());
        assert!(status.is_unknown());
        assert!(!status.is_operational());
    }

    #[test]
    fn test_health_check_type_http() {
        let check = HealthCheckType::Http("http://localhost:8080/health".to_string());
        assert!(check.is_http());
        assert!(!check.is_tcp());
        assert!(!check.is_exec());
        assert!(!check.is_csci_syscall());
        assert!(!check.is_custom());
    }

    #[test]
    fn test_health_check_type_tcp() {
        let check = HealthCheckType::Tcp(8080);
        assert!(!check.is_http());
        assert!(check.is_tcp());
        assert!(!check.is_exec());
        assert!(!check.is_csci_syscall());
        assert!(!check.is_custom());
    }

    #[test]
    fn test_health_check_type_exec() {
        let check = HealthCheckType::Exec("/opt/agent/health.sh".to_string());
        assert!(!check.is_http());
        assert!(!check.is_tcp());
        assert!(check.is_exec());
        assert!(!check.is_csci_syscall());
        assert!(!check.is_custom());
    }

    #[test]
    fn test_health_check_type_csci_syscall() {
        let check = HealthCheckType::CsciSyscall("cs_agent_probe".to_string());
        assert!(!check.is_http());
        assert!(!check.is_tcp());
        assert!(!check.is_exec());
        assert!(check.is_csci_syscall());
        assert!(!check.is_custom());
    }

    #[test]
    fn test_health_check_type_custom() {
        let check = HealthCheckType::Custom("my_handler".to_string());
        assert!(!check.is_http());
        assert!(!check.is_tcp());
        assert!(!check.is_exec());
        assert!(!check.is_csci_syscall());
        assert!(check.is_custom());
    }

    #[test]
    fn test_health_check_config_http_defaults() {
        let config = HealthCheckConfig::new_http("http://localhost:8080/health");
        assert_eq!(config.interval_ms, 10000);
        assert_eq!(config.timeout_ms, 5000);
        assert_eq!(config.failure_threshold, 3);
        assert_eq!(config.success_threshold, 1);
        assert!(config.check_type.is_http());
    }

    #[test]
    fn test_health_check_config_tcp_defaults() {
        let config = HealthCheckConfig::new_tcp(8080);
        assert_eq!(config.interval_ms, 10000);
        assert_eq!(config.timeout_ms, 5000);
        assert_eq!(config.failure_threshold, 3);
        assert_eq!(config.success_threshold, 1);
        assert!(config.check_type.is_tcp());
    }

    #[test]
    fn test_health_check_config_builder() {
        let config = HealthCheckConfig::new_http("http://localhost:8080/health")
            .with_interval(5000)
            .with_timeout(2000)
            .with_failure_threshold(5)
            .with_success_threshold(2);

        assert_eq!(config.interval_ms, 5000);
        assert_eq!(config.timeout_ms, 2000);
        assert_eq!(config.failure_threshold, 5);
        assert_eq!(config.success_threshold, 2);
    }

    #[test]
    fn test_probe_result_healthy_basic() {
        let result = ProbeResult::Healthy(1000);
        assert!(result.is_healthy());
        assert!(!result.is_unhealthy());
        assert!(!result.is_unknown());
    }

    #[test]
    fn test_probe_result_unhealthy_basic() {
        let result = ProbeResult::Unhealthy("HTTP 500".to_string());
        assert!(!result.is_healthy());
        assert!(result.is_unhealthy());
        assert!(!result.is_unknown());
    }

    #[test]
    fn test_probe_result_unknown_basic() {
        let result = ProbeResult::Unknown;
        assert!(!result.is_healthy());
        assert!(!result.is_unhealthy());
        assert!(result.is_unknown());
    }

    #[test]
    fn test_health_probe_readiness_http() {
        let probe = HealthProbe::readiness_http("http://localhost:8080/ready", 5000);
        assert_eq!(probe.initial_delay_ms, 5000);
        assert!(probe.config.check_type.is_http());
        assert_eq!(probe.config.interval_ms, 10000);
    }

    #[test]
    fn test_health_probe_liveness_tcp() {
        let probe = HealthProbe::liveness_tcp(9090, 10000);
        assert_eq!(probe.initial_delay_ms, 10000);
        assert!(probe.config.check_type.is_tcp());
        assert_eq!(probe.config.interval_ms, 10000);
    }

    #[test]
    fn test_health_probe_builder() {
        let probe = HealthProbe::readiness_http("http://localhost:8080/ready", 5000)
            .with_interval(3000)
            .with_timeout(1000)
            .with_failure_threshold(2)
            .with_success_threshold(1);

        assert_eq!(probe.config.interval_ms, 3000);
        assert_eq!(probe.config.timeout_ms, 1000);
        assert_eq!(probe.config.failure_threshold, 2);
        assert_eq!(probe.config.success_threshold, 1);
    }
}
