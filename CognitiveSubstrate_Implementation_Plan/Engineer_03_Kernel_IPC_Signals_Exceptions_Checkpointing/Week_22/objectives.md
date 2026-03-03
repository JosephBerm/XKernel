# Engineer 3 — Kernel: IPC, Signals, Exceptions & Checkpointing — Week 22

## Phase: PHASE 2 — Optimization & Integration

## Weekly Objective

Final SDK integration hardening: comprehensive API coverage, error handling, documentation, and validation that all subsystems work together seamlessly through SDK layer.

## Document References
- **Primary:** Section 3.2.4-3.2.8 (All Subsystems)
- **Supporting:** Section 6.2 (Exit Criteria)

## Deliverables
- [ ] Comprehensive error type unification: consistent error handling across SDK
- [ ] Advanced channel features: timeout handling, backpressure, priority
- [ ] Debugging support: SDK tracing and logging infrastructure
- [ ] Performance profiling hooks: SDK enables performance analysis
- [ ] Best practices guide: recommendations for using SDK
- [ ] API documentation: complete with examples for all APIs
- [ ] Test coverage: 95%+ coverage of SDK layer
- [ ] Migration guide: how to port existing code to SDK
- [ ] Compatibility matrix: document supported Rust versions, platforms
- [ ] Final validation: all Weeks 1-21 features work through SDK

## Technical Specifications

### Unified Error Type
```
pub enum SDKError {
    // IPC errors
    IPC(IpcError),
    ChannelNotFound,
    ChannelClosed,
    SendFailed(String),
    RecvTimeout,

    // Signal/Exception errors
    SignalRegisterFailed,
    ExceptionRegisterFailed,
    HandlerExecutionFailed(String),

    // Checkpoint errors
    CheckpointFailed(String),
    RestoreFailed(String),
    CheckpointNotFound,

    // System errors
    SystemError(String),
    PermissionDenied,
    OutOfMemory,

    // Kernel errors
    KernelNotResponding,
    KernelVersionMismatch,

    // Application errors
    SerializationError(serde_json::Error),
    DeserializationError(serde_json::Error),

    // User-defined errors
    Custom(Box<dyn std::error::Error>),
}

impl From<serde_json::Error> for SDKError {
    fn from(e: serde_json::Error) -> Self {
        if e.is_io() {
            SDKError::SerializationError(e)
        } else {
            SDKError::DeserializationError(e)
        }
    }
}

impl std::fmt::Display for SDKError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SDKError::IPC(e) => write!(f, "IPC error: {:?}", e),
            SDKError::SendFailed(msg) => write!(f, "Send failed: {}", msg),
            _ => write!(f, "{:?}", self),
        }
    }
}
```

### Advanced Channel Features
```
pub struct ChannelOptions {
    pub timeout_ms: u64,
    pub backpressure_policy: BackpressurePolicy,
    pub priority: u32,  // 0-255; higher = more important
    pub retry_policy: Option<RetryPolicy>,
}

impl Default for ChannelOptions {
    fn default() -> Self {
        Self {
            timeout_ms: 5000,
            backpressure_policy: BackpressurePolicy::SignalWarn,
            priority: 128,
            retry_policy: Some(RetryPolicy::default()),
        }
    }
}

pub struct ChannelBuilder {
    options: ChannelOptions,
}

impl ChannelBuilder {
    pub fn new() -> Self {
        Self { options: ChannelOptions::default() }
    }

    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.options.timeout_ms = timeout_ms;
        self
    }

    pub fn with_backpressure(mut self, policy: BackpressurePolicy) -> Self {
        self.options.backpressure_policy = policy;
        self
    }

    pub fn with_priority(mut self, priority: u32) -> Self {
        self.options.priority = priority;
        self
    }

    pub fn with_retry(mut self, policy: RetryPolicy) -> Self {
        self.options.retry_policy = Some(policy);
        self
    }

    pub fn build(self, sdk: &mut CognitiveSubstrateSDK, endpoint: ContextThreadRef) -> Result<Channel, SDKError> {
        // Create channel with options
        // Use custom timeout, backpressure policy, etc.
        unsafe {
            syscall::chan_open_with_options(&self.options, endpoint)
        }.map_err(SDKError::IPC)?;

        // ... rest of implementation
        todo!()
    }
}

// Usage
let channel = ChannelBuilder::new()
    .with_timeout(10000)
    .with_priority(200)
    .build(&mut sdk, endpoint)?;
```

### Debugging Support
```
pub struct SDKDebugger {
    tracing_enabled: bool,
    logging_level: log::Level,
    trace_buffer: VecDeque<TraceEvent>,
}

pub struct TraceEvent {
    pub timestamp: Timestamp,
    pub event_type: String,
    pub channel_id: Option<ChannelId>,
    pub message: String,
}

impl SDKDebugger {
    pub fn enable_tracing(&mut self) {
        self.tracing_enabled = true;
    }

    pub fn set_log_level(&mut self, level: log::Level) {
        self.logging_level = level;
    }

    pub fn record_event(&mut self, event: TraceEvent) {
        if self.tracing_enabled {
            self.trace_buffer.push_back(event);
            if self.trace_buffer.len() > 10000 {
                self.trace_buffer.pop_front();  // Keep last 10000 events
            }
        }
    }

    pub fn dump_trace(&self) -> String {
        let mut output = String::new();
        for event in &self.trace_buffer {
            output.push_str(&format!("{:?}: {} - {}\n",
                event.timestamp,
                event.event_type,
                event.message
            ));
        }
        output
    }
}

// Usage
let mut sdk = CognitiveSubstrateSDK::new()?;
sdk.debugger_mut().enable_tracing();
// ... run application ...
println!("{}", sdk.debugger().dump_trace());
```

### Performance Profiling Hooks
```
pub struct SDKProfiler {
    measurements: HashMap<String, Vec<u64>>,  // Event -> latencies in microseconds
}

impl SDKProfiler {
    pub fn measure<F, R>(&mut self, label: &str, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let start = Instant::now();
        let result = f();
        let elapsed = start.elapsed().as_micros() as u64;

        self.measurements
            .entry(label.to_string())
            .or_insert_with(Vec::new)
            .push(elapsed);

        result
    }

    pub fn report(&self) -> String {
        let mut output = String::new();
        for (label, measurements) in &self.measurements {
            let avg = measurements.iter().sum::<u64>() / measurements.len() as u64;
            let max = measurements.iter().max().unwrap_or(&0);
            let min = measurements.iter().min().unwrap_or(&0);

            output.push_str(&format!(
                "{}: avg={}us, min={}us, max={}us, count={}\n",
                label, avg, min, max, measurements.len()
            ));
        }
        output
    }
}

// Usage
let mut profiler = sdk.profiler_mut();
profiler.measure("send_message", || {
    channel.send(&request)?;
})?;

println!("{}", profiler.report());
```

### Best Practices Guide
```
/// # Best Practices for CognitiveSubstrateSDK
///
/// ## 1. Channel Management
/// - Reuse channels when possible; creating channels is expensive
/// - Use ChannelBuilder for custom configurations
/// - Always set appropriate timeouts
///
/// ## 2. Error Handling
/// - Use ? operator for propagating errors
/// - Match on SDKError for application-specific handling
/// - Log errors at appropriate levels
///
/// ## 3. Signal Handlers
/// - Keep handlers short and non-blocking
/// - Avoid I/O in signal handlers
/// - Return Continue for non-fatal signals
///
/// ## 4. Exception Handlers
/// - Implement different strategies for different exceptions
/// - Use Rollback for recoverable state errors
/// - Use Escalate for resource exhaustion
///
/// ## 5. Checkpoint Management
/// - Checkpoint before risky operations
/// - Use delta checkpoints for frequent saves
/// - Verify checkpoint restoration works
///
/// ## 6. Performance Optimization
/// - Use batching for multiple messages
/// - Profile before optimizing
/// - Consider distributed channels for multi-machine scenarios
///
/// ## 7. Debugging
/// - Enable tracing during development
/// - Use profiler to identify bottlenecks
/// - Check error messages for actionable feedback
```

### Comprehensive Test Coverage
```
#[cfg(test)]
mod sdk_tests {
    use super::*;

    #[test]
    fn test_sdk_initialization() { /* ... */ }

    #[test]
    fn test_channel_creation_and_messaging() { /* ... */ }

    #[test]
    fn test_signal_handler_registration() { /* ... */ }

    #[test]
    fn test_exception_handler_invocation() { /* ... */ }

    #[test]
    fn test_checkpoint_create_and_restore() { /* ... */ }

    #[test]
    fn test_pubsub_topic_operations() { /* ... */ }

    #[test]
    fn test_shared_context_updates() { /* ... */ }

    #[test]
    fn test_error_handling_and_recovery() { /* ... */ }

    #[test]
    fn test_timeout_behavior() { /* ... */ }

    #[test]
    fn test_backpressure_policies() { /* ... */ }

    #[test]
    fn test_multi_agent_coordination() { /* ... */ }

    #[test]
    fn test_distributed_channels() { /* ... */ }

    #[test]
    fn test_concurrent_operations() { /* ... */ }

    #[test]
    fn test_resource_cleanup() { /* ... */ }

    #[test]
    fn test_performance_basic() { /* ... */ }

    // Coverage target: 95%+
}
```

### API Documentation Examples
```rust
/// Send a message through a channel with timeout.
///
/// # Arguments
///
/// * `message` - Serializable message to send
/// * `timeout_ms` - Timeout in milliseconds (0 = blocking)
///
/// # Returns
///
/// Returns `Ok(())` on success, or `Err(SDKError)` on failure.
///
/// # Errors
///
/// - `SDKError::SendFailed` - Failed to send message
/// - `SDKError::RecvTimeout` - Timeout waiting for response
/// - `SDKError::ChannelClosed` - Channel was closed
///
/// # Example
///
/// ```ignore
/// let request = json!({ "action": "compute", "value": 42 });
/// channel.send(&request)?;
/// ```
pub fn send<T: Serialize>(&self, message: &T) -> Result<(), SDKError> {
    // implementation
}
```

## Dependencies
- **Blocked by:** Week 21 (SDK Layer)
- **Blocking:** Week 23-24 (Benchmarking & Launch)

## Acceptance Criteria
1. All SDKError variants handle various failure modes
2. ChannelBuilder enables easy configuration
3. Debugger provides useful tracing and logging
4. Profiler accurately measures operation latencies
5. Best practices guide covers 7+ key areas
6. API documentation complete with examples
7. Test coverage >= 95% of SDK layer
8. All major features testable via SDK
9. No breaking API changes from Weeks 1-21
10. SDK ready for production use

## Design Principles Alignment
- **Ergonomics:** Unified error type and builder pattern simplify API
- **Observability:** Tracing and profiling enable debugging
- **Reliability:** Comprehensive error handling prevents silent failures
- **Documentation:** Examples and guides help developers succeed
