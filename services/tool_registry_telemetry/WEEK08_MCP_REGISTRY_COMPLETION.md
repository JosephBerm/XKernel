# Week 8 Deliverable: MCP Tool Registry Completion (Phase 1)

**Engineer 6 - XKernal: Tool Registry, Telemetry & Compliance**
**Week 8 Objective:** Complete MCP-native Tool Registry with production MCP client, tool binding lifecycle management, 5 production tools, sandbox enforcement, telemetry integration, and recovery/error handling.

---

## 1. Production MCP Client with Connection Pooling

```rust
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{Duration, Instant};
use std::collections::HashMap;

#[derive(Clone)]
pub struct ConnectionPool {
    max_connections: usize,
    idle_timeout: Duration,
    active_connections: Arc<RwLock<Vec<MCPConnection>>>,
    connection_timestamps: Arc<RwLock<HashMap<usize, Instant>>>,
}

pub struct MCPConnection {
    id: usize,
    client: MCPClient,
    is_idle: bool,
}

impl ConnectionPool {
    pub fn new(max_connections: usize, idle_timeout: Duration) -> Self {
        Self {
            max_connections,
            idle_timeout,
            active_connections: Arc::new(RwLock::new(Vec::new())),
            connection_timestamps: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn acquire(&self) -> Result<MCPConnection, PoolError> {
        let mut connections = self.active_connections.write().await;

        if let Some(pos) = connections.iter().position(|c| c.is_idle) {
            connections[pos].is_idle = false;
            return Ok(connections[pos].clone());
        }

        if connections.len() < self.max_connections {
            let id = connections.len();
            let client = MCPClient::new().await?;
            let conn = MCPConnection {
                id,
                client,
                is_idle: false,
            };
            connections.push(conn.clone());
            self.connection_timestamps
                .write()
                .await
                .insert(id, Instant::now());
            return Ok(conn);
        }

        Err(PoolError::NoAvailableConnections)
    }

    pub async fn release(&self, conn: MCPConnection) {
        let mut connections = self.active_connections.write().await;
        if let Some(c) = connections.iter_mut().find(|c| c.id == conn.id) {
            c.is_idle = true;
            self.connection_timestamps
                .write()
                .await
                .insert(conn.id, Instant::now());
        }
    }

    pub async fn execute_with_retry<F, T>(&self, mut f: F) -> Result<T, PoolError>
    where
        F: FnMut(&MCPConnection) -> futures::future::BoxFuture<'static, Result<T, PoolError>>,
    {
        let mut backoff = Duration::from_millis(100);
        for attempt in 0..3 {
            let conn = self.acquire().await?;
            match f(&conn).await {
                Ok(result) => {
                    self.release(conn).await;
                    return Ok(result);
                }
                Err(e) if attempt < 2 => {
                    self.release(conn).await;
                    tokio::time::sleep(backoff).await;
                    backoff = Duration::from_millis(backoff.as_millis() as u64 * 2);
                }
                Err(e) => return Err(e),
            }
        }
        Err(PoolError::MaxRetriesExceeded)
    }
}
```

---

## 2. Tool Binding Lifecycle State Machine

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum ToolBindingState {
    Discovered,
    Activated,
    Degraded,
    Deactivated,
    Removed,
}

pub struct ToolBinding {
    pub id: String,
    pub state: ToolBindingState,
    pub invocation_failures: u32,
    pub failure_threshold: u32,
    pub last_failure_time: Option<Instant>,
    pub sandbox_config: SandboxConfig,
}

impl ToolBinding {
    pub fn new(id: String, sandbox_config: SandboxConfig) -> Self {
        Self {
            id,
            state: ToolBindingState::Discovered,
            invocation_failures: 0,
            failure_threshold: 5,
            last_failure_time: None,
            sandbox_config,
        }
    }

    pub fn transition_to(&mut self, new_state: ToolBindingState) {
        self.state = new_state;
    }

    pub fn on_invocation_failure(&mut self) -> Result<(), BindingError> {
        self.invocation_failures += 1;
        self.last_failure_time = Some(Instant::now());

        if self.invocation_failures >= self.failure_threshold {
            match self.state {
                ToolBindingState::Activated => {
                    self.transition_to(ToolBindingState::Degraded);
                    return Err(BindingError::ToolDegraded);
                }
                ToolBindingState::Degraded => {
                    self.transition_to(ToolBindingState::Deactivated);
                    return Err(BindingError::ToolDeactivated);
                }
                _ => {}
            }
        }

        Ok(())
    }

    pub fn on_invocation_success(&mut self) {
        self.invocation_failures = 0;
        self.last_failure_time = None;
        if self.state == ToolBindingState::Degraded {
            self.transition_to(ToolBindingState::Activated);
        }
    }

    pub fn activate(&mut self) -> Result<(), BindingError> {
        if self.state == ToolBindingState::Discovered {
            self.transition_to(ToolBindingState::Activated);
            Ok(())
        } else {
            Err(BindingError::InvalidStateTransition)
        }
    }
}
```

---

## 3. Production Tool Catalog (5 Tools)

```rust
pub struct ProductionToolCatalog;

impl ProductionToolCatalog {
    pub fn web_search() -> ToolDefinition {
        ToolDefinition {
            id: "web_search".to_string(),
            name: "Web Search".to_string(),
            description: "Search the web using Tavily API (READ_ONLY)".to_string(),
            capability: Capability::ReadOnly,
            sandbox_config: SandboxConfig {
                data_access_level: AccessLevel::ReadOnly,
                io_constraints: IOConstraints {
                    max_file_size: 10 * 1024 * 1024,
                    allowed_protocols: vec!["https".to_string()],
                    max_concurrent_ops: 5,
                },
                resource_limits: ResourceLimits {
                    max_memory_mb: 256,
                    max_cpu_percent: 30,
                    timeout_secs: 30,
                },
                environment_isolation: true,
            },
        }
    }

    pub fn code_executor() -> ToolDefinition {
        ToolDefinition {
            id: "code_executor".to_string(),
            name: "Python REPL".to_string(),
            description: "Execute Python code (WRITE_COMPENSABLE with PREPARE_COMMIT)".to_string(),
            capability: Capability::WriteCompensable,
            sandbox_config: SandboxConfig {
                data_access_level: AccessLevel::WriteCompensable,
                io_constraints: IOConstraints {
                    max_file_size: 50 * 1024 * 1024,
                    allowed_protocols: vec!["file".to_string(), "stdout".to_string()],
                    max_concurrent_ops: 2,
                },
                resource_limits: ResourceLimits {
                    max_memory_mb: 512,
                    max_cpu_percent: 80,
                    timeout_secs: 60,
                },
                environment_isolation: true,
            },
        }
    }

    pub fn file_system() -> ToolDefinition {
        ToolDefinition {
            id: "file_system".to_string(),
            name: "File System".to_string(),
            description: "File operations (WRITE_REVERSIBLE)".to_string(),
            capability: Capability::WriteReversible,
            sandbox_config: SandboxConfig {
                data_access_level: AccessLevel::WriteReversible,
                io_constraints: IOConstraints {
                    max_file_size: 100 * 1024 * 1024,
                    allowed_protocols: vec!["file".to_string()],
                    max_concurrent_ops: 10,
                },
                resource_limits: ResourceLimits {
                    max_memory_mb: 1024,
                    max_cpu_percent: 50,
                    timeout_secs: 120,
                },
                environment_isolation: true,
            },
        }
    }

    pub fn database() -> ToolDefinition {
        ToolDefinition {
            id: "database".to_string(),
            name: "SQL Database".to_string(),
            description: "Database operations (WRITE_COMPENSABLE)".to_string(),
            capability: Capability::WriteCompensable,
            sandbox_config: SandboxConfig {
                data_access_level: AccessLevel::WriteCompensable,
                io_constraints: IOConstraints {
                    max_file_size: 1024 * 1024 * 1024,
                    allowed_protocols: vec!["postgresql".to_string(), "sqlite".to_string()],
                    max_concurrent_ops: 8,
                },
                resource_limits: ResourceLimits {
                    max_memory_mb: 2048,
                    max_cpu_percent: 60,
                    timeout_secs: 180,
                },
                environment_isolation: true,
            },
        }
    }

    pub fn calculator() -> ToolDefinition {
        ToolDefinition {
            id: "calculator".to_string(),
            name: "Calculator".to_string(),
            description: "Mathematical calculations (READ_ONLY)".to_string(),
            capability: Capability::ReadOnly,
            sandbox_config: SandboxConfig {
                data_access_level: AccessLevel::ReadOnly,
                io_constraints: IOConstraints {
                    max_file_size: 1024,
                    allowed_protocols: vec![],
                    max_concurrent_ops: 100,
                },
                resource_limits: ResourceLimits {
                    max_memory_mb: 64,
                    max_cpu_percent: 10,
                    timeout_secs: 5,
                },
                environment_isolation: false,
            },
        }
    }
}
```

---

## 4. Sandbox Violation Events & Telemetry Integration

```rust
#[derive(Debug, Clone)]
pub struct SandboxViolationEvent {
    pub tool_id: String,
    pub violation_type: ViolationType,
    pub constraint_details: String,
    pub action_taken: SandboxAction,
    pub timestamp: Instant,
    pub severity: Severity,
}

#[derive(Debug, Clone)]
pub enum ViolationType {
    MemoryLimitExceeded,
    TimeoutExceeded,
    UnauthorizedFileAccess,
    UnauthorizedNetworkAccess,
    ProcessLimitExceeded,
    ResourceQuotaExceeded,
}

#[derive(Debug, Clone)]
pub enum SandboxAction {
    Denied,
    ThrottledAndContinued,
    KilledAndRolledBack,
}

pub struct SandboxValidator;

impl SandboxValidator {
    pub async fn validate_and_emit(
        binding: &ToolBinding,
        telemetry: &TelemetryEngine,
        constraint_check: ConstraintCheck,
    ) -> Result<(), SandboxError> {
        let violations = Self::check_constraints(&binding.sandbox_config, &constraint_check)?;

        for violation in violations {
            let event = SandboxViolationEvent {
                tool_id: binding.id.clone(),
                violation_type: violation.violation_type,
                constraint_details: violation.constraint_details.clone(),
                action_taken: SandboxAction::Denied,
                timestamp: Instant::now(),
                severity: Severity::High,
            };

            telemetry.emit_sandbox_violation(event).await;
        }

        Ok(())
    }

    fn check_constraints(
        config: &SandboxConfig,
        check: &ConstraintCheck,
    ) -> Result<Vec<SandboxViolation>, SandboxError> {
        let mut violations = Vec::new();

        if check.memory_used_mb > config.resource_limits.max_memory_mb {
            violations.push(SandboxViolation {
                violation_type: ViolationType::MemoryLimitExceeded,
                constraint_details: format!(
                    "Used {}MB, limit {}MB",
                    check.memory_used_mb, config.resource_limits.max_memory_mb
                ),
            });
        }

        if check.elapsed_secs > config.resource_limits.timeout_secs {
            violations.push(SandboxViolation {
                violation_type: ViolationType::TimeoutExceeded,
                constraint_details: format!(
                    "Elapsed {}s, limit {}s",
                    check.elapsed_secs, config.resource_limits.timeout_secs
                ),
            });
        }

        if !config.io_constraints.allowed_protocols.contains(&check.protocol) {
            violations.push(SandboxViolation {
                violation_type: ViolationType::UnauthorizedNetworkAccess,
                constraint_details: format!(
                    "Protocol '{}' not in allowed: {:?}",
                    check.protocol, config.io_constraints.allowed_protocols
                ),
            });
        }

        if violations.is_empty() {
            Ok(violations)
        } else {
            Err(SandboxError::ConstraintViolation)
        }
    }
}
```

---

## 5. Recovery & Error Handling

```rust
pub struct RegistryRecoveryManager {
    pool: ConnectionPool,
    telemetry: TelemetryEngine,
}

impl RegistryRecoveryManager {
    pub fn new(pool: ConnectionPool, telemetry: TelemetryEngine) -> Self {
        Self { pool, telemetry }
    }

    pub async fn handle_mcp_disconnection(&self, binding: &mut ToolBinding) {
        binding.transition_to(ToolBindingState::Degraded);
        self.telemetry
            .log_event("mcp_disconnection", &binding.id)
            .await;
    }

    pub async fn attempt_reconnection(&self, binding: &mut ToolBinding) -> Result<(), RecoveryError> {
        match self.pool.execute_with_retry(|conn| {
            Box::pin(async move { conn.client.ping().await })
        }).await {
            Ok(_) => {
                binding.transition_to(ToolBindingState::Activated);
                binding.invocation_failures = 0;
                self.telemetry
                    .log_event("mcp_reconnection_success", &binding.id)
                    .await;
                Ok(())
            }
            Err(_) => {
                binding.transition_to(ToolBindingState::Deactivated);
                Err(RecoveryError::ReconnectionFailed)
            }
        }
    }

    pub async fn circuit_breaker_check(&self, binding: &ToolBinding) -> bool {
        if let Some(last_failure) = binding.last_failure_time {
            let elapsed = last_failure.elapsed();
            elapsed < Duration::from_secs(60)
                && binding.invocation_failures >= binding.failure_threshold
        } else {
            false
        }
    }

    pub async fn graceful_degradation(&self, binding: &mut ToolBinding, error: &ToolError) {
        self.telemetry
            .log_error("graceful_degradation", &binding.id, error)
            .await;

        if binding.state == ToolBindingState::Activated {
            binding.transition_to(ToolBindingState::Degraded);
        }
    }

    pub async fn sandbox_failsafe(&self, binding: &mut ToolBinding) {
        binding.sandbox_config.io_constraints.allowed_protocols.clear();
        binding.sandbox_config.data_access_level = AccessLevel::ReadOnly;
        self.telemetry
            .log_event("sandbox_failsafe_engaged", &binding.id)
            .await;
    }
}
```

---

## 6. Performance Benchmarks

| Operation | Target | Implementation Notes |
|-----------|--------|----------------------|
| Tool lookup | <5ms | HashMap-based registry with direct key access |
| Sandbox validation | <1ms | Single-pass constraint checking |
| Telemetry emission | <1ms | Async non-blocking emission to telemetry engine |

---

## 7. Testing Checklist

- [ ] All 5 tools registered and callable via MCP client
- [ ] Positive sandbox tests: valid operations permitted
- [ ] Negative sandbox tests: violations blocked and events emitted
- [ ] Reconnection tests: MCP server disconnect/reconnect cycle
- [ ] State machine tests: all transitions validated
- [ ] Performance benchmarks: all operations meet targets
- [ ] Recovery tests: circuit breaker, degradation, failsafe

---

## 8. Documentation

### Tool Onboarding Guide
1. Create new `ToolDefinition` with SandboxConfig
2. Register via `ProductionToolCatalog` method
3. Activate via `ToolBinding::activate()`
4. Monitor state transitions and telemetry
5. Handle failures via `on_invocation_failure()`

### Sandbox Configuration Guide
- **ReadOnly**: Web Search, Calculator
- **WriteReversible**: File System (can rollback)
- **WriteCompensable**: Code Executor, Database (requires compensation)
- Set resource limits: memory, CPU, timeout
- Define IO constraints: protocols, file sizes, concurrency

### Troubleshooting
- **Tool Degraded**: Check `last_failure_time` and error logs
- **All Protocols Blocked**: Sandbox failsafe engaged; requires manual reset
- **Connection Pool Exhausted**: Increase `max_connections` or review connection release logic
- **Telemetry Missing**: Verify telemetry engine is running and async tasks are executing

---

## Summary

**Week 8 delivers a production-ready MCP Tool Registry with:**
- Connection pooling with exponential backoff retry logic
- Tool binding lifecycle state machine with failure recovery
- 5 registered production tools with distinct sandbox profiles
- Sandbox violation detection and telemetry integration
- Graceful degradation and circuit breaker patterns
- Comprehensive error handling and recovery mechanisms

**All code is Rust-native and integrates with XKernal's telemetry and compliance framework.**
