# Week 10 Deliverable: Translation Error Handling & Telemetry Design (Phase 1)

## Engineer 7: Runtime Framework Adapters

**Objective:** Design error handling and fallback mechanisms at translation boundary. Design telemetry and tracing for framework→kernel translation. Prepare for Week 11 implementation.

**Status:** Phase 1 Design Complete | Week 11 Implementation Ready

---

## 1. Error Handling Design

### Error Class Hierarchy

The translation layer distinguishes between five primary error categories:

- **TranslationError**: Malformed agent instructions, invalid framework directives, syntax violations
- **IncompatibilityError**: Framework/kernel API mismatch, unsupported feature combinations, version conflicts
- **ResourceExhaustedError**: Memory limits exceeded, queue saturation, kernel handle depletion
- **TimeoutError**: Translation timeout, kernel response timeout, deadlock detection
- **IpcError**: Message queue failure, serialization error, kernel communication breakdown

### Validation Strategy

**Fail Early Principle**: All inputs validated before translation begins.

```
Input Validation Gates (sequential):
  1. Schema validation (framework directive format)
  2. Semantic validation (instruction references, type correctness)
  3. Capability checking (runtime environment readiness)
  4. Resource pre-flight checks (memory, kernel capacity)
  5. Dependency resolution (tool bindings, handler registration)
```

**Clear Error Messages**: Each error includes:
- Error code (e.g., `ERR_TRANSLATION_001`)
- Context (failing instruction, line number in agent spec)
- Recovery suggestion (retry conditions, fallback options)
- Correlation ID (linked to telemetry)

---

## 2. Fallback Mechanisms

### Automatic Retry Strategy

**Retry Policy** (3 attempts maximum):
- **Attempt 1**: Immediate retry (transient I/O glitch)
- **Attempt 2**: 100ms backoff (kernel saturation)
- **Attempt 3**: 500ms backoff (resource unavailability)
- **Failure**: Escalate to circuit breaker

**Backoff Calculation**:
```
backoff_ms = base_delay_ms * (2 ^ attempt_number)
  + random_jitter(0, base_delay_ms)
```

### Backpressure Handling

When kernel queue is full:
1. **Queue Operation**: Enqueue translation job with priority
2. **Monitor Saturation**: Track queue depth every 50ms
3. **Threshold Trigger**: At 80% capacity, enable adaptive timeout extension
4. **Flush Coordination**: Prioritize critical tasks (TaskSpawn > ChainTranslate > MemoryMap)

### Optional Operation Skipping

Framework directives may mark operations as optional:
- **Example**: Non-critical telemetry write, secondary memory mapping
- **Behavior**: Skip operation, log deprecation warning, continue execution
- **Logging**: Record skipped ops for observability

### Fail-Fast on Critical

Critical operations (e.g., initial FrameworkLoad, TaskSpawn) fail immediately:
- No retry attempts
- Escalate error to calling runtime
- Abort entire agent execution context

### Circuit Breaker Pattern

**State Machine**:
```
CLOSED (normal)
  → consecutive_timeouts++
  → [5 timeouts reached]
  → OPEN (fast-fail)
    → request rejected immediately
    → HALF_OPEN [after recovery_delay = 5s]
      → single probe request
      → [success]
        → backoff_reset()
        → CLOSED
      → [timeout]
        → recovery_delay *= exponential_factor (2.0)
        → remain HALF_OPEN
```

**Configuration**:
- Failure threshold: 5 consecutive kernel timeouts
- Success threshold: 2 consecutive successful requests in HALF_OPEN
- Max recovery delay: 30 seconds
- Exponential factor: 2.0 (resets on successful close)

---

## 3. Telemetry Infrastructure

### CEF Event Schema

All translation events published in Common Event Format (CEF):

```
CEF:0|XKernal|FrameworkAdapter|7.0|[event_type]|[event_name]|[severity]|
  event_type=[type]
  framework=[framework_id]
  agent_id=[uuid]
  error_code=[code]
  timestamp=[iso8601]
  severity=[CRITICAL|ERROR|WARNING|INFO]
  correlation_id=[uuid]
  duration_ms=[latency]
  message=[clear_text_summary]
```

**Event Types**:
- `ERR_TRANSLATION`: Translation process failed
- `ERR_TIMEOUT`: Kernel response timeout
- `ERR_RESOURCE`: Resource exhaustion
- `CIRCUIT_BREAK_TRIP`: Circuit breaker transitioned OPEN
- `CIRCUIT_BREAK_RESET`: Circuit breaker recovered to CLOSED

---

## 4. Tracing Spans

Instrumentation spans track end-to-end latency and dependencies:

| Span Name | Description | Parent | Tags |
|-----------|-------------|--------|------|
| **FrameworkLoad** | Framework bootstrap and initialization | ROOT | framework_type, agent_count |
| **ChainTranslate** | Translate agent execution chain to kernel IR | FrameworkLoad | chain_depth, instr_count |
| **MemoryMap** | Reserve and configure memory regions | ChainTranslate | region_count, total_bytes |
| **ToolBind** | Bind framework tools to kernel handlers | ChainTranslate | tool_count, binding_latency_ms |
| **TaskSpawn** | Create kernel task context and apply execution policy | ChainTranslate | task_id, cpu_affinity |
| **ResultCollect** | Gather and deserialize kernel results | ROOT (sibling to FrameworkLoad) | result_count, deserialize_latency_ms |

**Span Tags Propagated**: correlation_id, agent_id, framework_id, error_code (if applicable)

---

## 5. Metrics Schema

Published to metrics backend (Prometheus-compatible):

### Histograms

- **translation_latency_ms**: Latency of complete translation (FrameworkLoad → TaskSpawn)
  - Buckets: [10, 50, 100, 500, 1000, 5000]
  - Labels: framework_type, success (bool)

### Gauges

- **success_rate**: Percentage of successful translations (5-min rolling)
  - Labels: framework_type, operation_type
- **memory_used**: Current kernel memory used by translation context
  - Labels: agent_id, region_type (code, data, heap)
- **queue_depth**: Current kernel IPC queue depth
  - Labels: queue_type (standard, priority)

### Counters

- **error_count**: Total translation errors (cumulative)
  - Labels: error_type, framework_type, error_code
- **retry_count**: Total retry attempts across all translations
  - Labels: error_type, attempt_number
- **circuit_breaker_trips**: Total circuit breaker transitions to OPEN
  - Labels: recovery_count

---

## 6. Correlation IDs

**UUID Generation**: One UUID per agent execution (created at framework load time)

**Propagation**:
1. Generated at framework entry point
2. Embedded in all CEF events
3. Included in all IPC messages to kernel
4. Returned in kernel responses
5. Linked in all telemetry (spans, metrics, logs)
6. Retained for post-mortem analysis (30-day retention)

**Usage**: Enables full request tracing from user input through kernel execution and back to result delivery.

---

## 7. Translation Failure Catalog

### Common Failure Patterns

| Failure | Cause | Recovery | Telemetry Code |
|---------|-------|----------|-----------------|
| **Stale Framework Cache** | Kernel API updated mid-session | Invalidate cache, reload metadata | ERR_INCOMP_001 |
| **Memory Exhaustion** | Agent context too large for kernel heap | Reduce context size, compress data | ERR_RESOURCE_001 |
| **Kernel Timeout** | Kernel unresponsive or overloaded | Exponential backoff, circuit break | ERR_TIMEOUT_001 |
| **Invalid IPC Message** | Serialization error in message payload | Validate schema, retry serialization | ERR_IPC_001 |
| **Tool Not Registered** | Framework tool binding missing | Register tool, retry translation | ERR_TRANS_001 |
| **Circular Dependency** | Agent chain circular reference | Detect cycle, fail-fast with context | ERR_TRANS_002 |

---

## 8. Implementation Readiness Checklist

- [x] Error class hierarchy finalized
- [x] Validation gates specified
- [x] Retry policy with exponential backoff designed
- [x] Circuit breaker state machine defined
- [x] CEF event schema standardized
- [x] Tracing span catalog complete
- [x] Metrics schema aligned with observability team
- [x] Correlation ID propagation strategy detailed
- [x] Failure recovery strategies documented
- [x] Rust implementation code complete (see Section 9)
- [ ] Week 11: Implement TranslationErrorHandler
- [ ] Week 11: Implement FallbackPolicy executor
- [ ] Week 11: Integrate with kernel IPC layer
- [ ] Week 12: End-to-end telemetry testing

---

## 9. Rust Implementation: Core Components

```rust
// ============================================================================
// TranslationErrorHandler: Centralized error classification and recovery
// ============================================================================

use std::fmt;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    TranslationMalformed,
    IncompatibilityDetected,
    ResourceExhausted,
    TimeoutExceeded,
    IpcMessageFailure,
    ToolNotRegistered,
    CircularDependency,
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let code = match self {
            Self::TranslationMalformed => "ERR_TRANS_001",
            Self::IncompatibilityDetected => "ERR_INCOMP_001",
            Self::ResourceExhausted => "ERR_RESOURCE_001",
            Self::TimeoutExceeded => "ERR_TIMEOUT_001",
            Self::IpcMessageFailure => "ERR_IPC_001",
            Self::ToolNotRegistered => "ERR_TOOL_001",
            Self::CircularDependency => "ERR_TRANS_002",
        };
        write!(f, "{}", code)
    }
}

pub struct TranslationError {
    pub code: ErrorCode,
    pub message: String,
    pub context: String,
    pub correlation_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub recovery_hint: String,
    pub severity: Severity,
}

#[derive(Debug, Clone, Copy)]
pub enum Severity {
    Critical,
    Error,
    Warning,
    Info,
}

impl TranslationError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            context: String::new(),
            correlation_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            recovery_hint: String::new(),
            severity: Severity::Error,
        }
    }

    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = context.into();
        self
    }

    pub fn with_recovery_hint(mut self, hint: impl Into<String>) -> Self {
        self.recovery_hint = hint.into();
        self
    }

    pub fn with_severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    pub fn with_correlation_id(mut self, id: Uuid) -> Self {
        self.correlation_id = id;
        self
    }

    pub fn to_cef_event(&self) -> String {
        format!(
            "CEF:0|XKernal|FrameworkAdapter|7.0|{}|{}|{:?}| \
             event_type=ERR_TRANSLATION \
             framework=xkernal \
             agent_id={} \
             error_code={} \
             timestamp={} \
             severity={:?} \
             correlation_id={} \
             message={} \
             recovery_hint={}",
            self.code,
            self.message,
            self.severity,
            self.correlation_id,
            self.code,
            self.timestamp,
            self.severity,
            self.correlation_id,
            self.message,
            self.recovery_hint
        )
    }
}

// ============================================================================
// FallbackPolicy: Retry, backoff, and fail-fast coordination
// ============================================================================

pub struct FallbackPolicy {
    max_retries: u32,
    base_delay_ms: u64,
    backoff_factor: f64,
    is_critical: bool,
}

impl FallbackPolicy {
    pub fn new(is_critical: bool) -> Self {
        Self {
            max_retries: if is_critical { 0 } else { 3 },
            base_delay_ms: 10,
            backoff_factor: 2.0,
            is_critical,
        }
    }

    pub fn calculate_backoff(&self, attempt: u32) -> u64 {
        if self.is_critical || attempt == 0 {
            return 0;
        }
        let delay = self.base_delay_ms * (self.backoff_factor.powi(attempt as i32) as u64);
        let jitter = (delay as f64 * 0.1) as u64;
        delay + (rand::random::<u64>() % (jitter + 1))
    }

    pub fn should_retry(&self, attempt: u32) -> bool {
        !self.is_critical && attempt < self.max_retries
    }
}

// ============================================================================
// CircuitBreaker: Prevent cascading kernel timeouts
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitBreakerState {
    Closed,
    Open,
    HalfOpen,
}

pub struct CircuitBreaker {
    state: CircuitBreakerState,
    consecutive_failures: u32,
    consecutive_successes: u32,
    failure_threshold: u32,
    success_threshold: u32,
    recovery_delay_ms: u64,
    max_recovery_delay_ms: u64,
    exponential_factor: f64,
    last_failure_time: Option<DateTime<Utc>>,
}

impl CircuitBreaker {
    pub fn new() -> Self {
        Self {
            state: CircuitBreakerState::Closed,
            consecutive_failures: 0,
            consecutive_successes: 0,
            failure_threshold: 5,
            success_threshold: 2,
            recovery_delay_ms: 5000,
            max_recovery_delay_ms: 30000,
            exponential_factor: 2.0,
            last_failure_time: None,
        }
    }

    pub fn record_failure(&mut self) -> CircuitBreakerState {
        self.consecutive_failures += 1;
        self.consecutive_successes = 0;
        self.last_failure_time = Some(Utc::now());

        if self.state == CircuitBreakerState::Closed
            && self.consecutive_failures >= self.failure_threshold
        {
            self.state = CircuitBreakerState::Open;
        }
        self.state
    }

    pub fn record_success(&mut self) -> CircuitBreakerState {
        self.consecutive_successes += 1;
        self.consecutive_failures = 0;

        match self.state {
            CircuitBreakerState::HalfOpen => {
                if self.consecutive_successes >= self.success_threshold {
                    self.state = CircuitBreakerState::Closed;
                    self.recovery_delay_ms = 5000; // Reset recovery delay
                }
            }
            _ => {}
        }
        self.state
    }

    pub fn allow_request(&mut self) -> bool {
        match self.state {
            CircuitBreakerState::Closed => true,
            CircuitBreakerState::Open => {
                let now = Utc::now();
                if let Some(last_failure) = self.last_failure_time {
                    let elapsed = (now - last_failure).num_milliseconds() as u64;
                    if elapsed >= self.recovery_delay_ms {
                        self.state = CircuitBreakerState::HalfOpen;
                        self.consecutive_successes = 0;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CircuitBreakerState::HalfOpen => true,
        }
    }

    pub fn apply_exponential_backoff(&mut self) {
        self.recovery_delay_ms = std::cmp::min(
            (self.recovery_delay_ms as f64 * self.exponential_factor) as u64,
            self.max_recovery_delay_ms,
        );
    }
}

// ============================================================================
// TracingInfrastructure: Span creation and context propagation
// ============================================================================

use std::collections::HashMap;

pub struct TracingSpan {
    pub name: String,
    pub parent_id: Option<Uuid>,
    pub span_id: Uuid,
    pub correlation_id: Uuid,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub tags: HashMap<String, String>,
    pub status: SpanStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanStatus {
    Active,
    Completed,
    Failed,
}

impl TracingSpan {
    pub fn new(name: impl Into<String>, correlation_id: Uuid) -> Self {
        Self {
            name: name.into(),
            parent_id: None,
            span_id: Uuid::new_v4(),
            correlation_id,
            start_time: Utc::now(),
            end_time: None,
            tags: HashMap::new(),
            status: SpanStatus::Active,
        }
    }

    pub fn with_parent(mut self, parent_id: Uuid) -> Self {
        self.parent_id = Some(parent_id);
        self
    }

    pub fn add_tag(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.tags.insert(key.into(), value.into());
    }

    pub fn finish(&mut self) {
        self.end_time = Some(Utc::now());
        self.status = SpanStatus::Completed;
    }

    pub fn finish_with_error(&mut self) {
        self.end_time = Some(Utc::now());
        self.status = SpanStatus::Failed;
    }

    pub fn duration_ms(&self) -> Option<u64> {
        self.end_time.map(|end| (end - self.start_time).num_milliseconds() as u64)
    }
}

pub struct TracingContext {
    pub correlation_id: Uuid,
    pub spans: Vec<TracingSpan>,
}

impl TracingContext {
    pub fn new() -> Self {
        Self {
            correlation_id: Uuid::new_v4(),
            spans: Vec::new(),
        }
    }

    pub fn create_span(&mut self, name: impl Into<String>) -> Uuid {
        let span = TracingSpan::new(name, self.correlation_id);
        let span_id = span.span_id;
        self.spans.push(span);
        span_id
    }

    pub fn create_child_span(&mut self, name: impl Into<String>, parent_id: Uuid) -> Uuid {
        let span = TracingSpan::new(name, self.correlation_id).with_parent(parent_id);
        let span_id = span.span_id;
        self.spans.push(span);
        span_id
    }
}

// ============================================================================
// Integration: TranslationErrorHandler main facade
// ============================================================================

pub struct TranslationErrorHandler {
    circuit_breaker: CircuitBreaker,
    fallback_policy: FallbackPolicy,
    tracing_context: TracingContext,
}

impl TranslationErrorHandler {
    pub fn new(is_critical_operation: bool) -> Self {
        Self {
            circuit_breaker: CircuitBreaker::new(),
            fallback_policy: FallbackPolicy::new(is_critical_operation),
            tracing_context: TracingContext::new(),
        }
    }

    pub fn can_proceed(&mut self) -> bool {
        self.circuit_breaker.allow_request()
    }

    pub fn handle_failure(&mut self) -> Option<u64> {
        self.circuit_breaker.record_failure();
        let backoff = self.fallback_policy.calculate_backoff(self.circuit_breaker.consecutive_failures);

        if self.circuit_breaker.state == CircuitBreakerState::Open {
            self.circuit_breaker.apply_exponential_backoff();
        }

        if self.fallback_policy.should_retry(self.circuit_breaker.consecutive_failures) {
            Some(backoff)
        } else {
            None
        }
    }

    pub fn handle_success(&mut self) {
        self.circuit_breaker.record_success();
    }

    pub fn correlation_id(&self) -> Uuid {
        self.tracing_context.correlation_id
    }

    pub fn create_span(&mut self, name: impl Into<String>) -> Uuid {
        self.tracing_context.create_span(name)
    }
}
```

---

## Week 11 Preparation

All design components are finalized and Rust reference implementations provided. Week 11 will focus on:

1. Integrating TranslationErrorHandler into the main translation pipeline
2. Implementing FallbackPolicy executor for retry coordination
3. Connecting CircuitBreaker to kernel IPC timeout handling
4. Setting up CEF event publishing to centralized telemetry platform
5. Implementing TracingContext propagation across all translation boundaries

**Estimated Implementation Time**: 3-4 days
**Testing Strategy**: Unit tests for each component, integration tests for error recovery scenarios, chaos engineering for circuit breaker validation
