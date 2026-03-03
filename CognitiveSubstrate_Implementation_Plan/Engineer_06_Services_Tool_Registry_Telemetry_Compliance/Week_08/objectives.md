# Engineer 6 — Services: Tool Registry, Telemetry & Compliance — Week 8

## Phase: Phase 1 (Weeks 7-14)

## Weekly Objective
Complete MCP-native Tool Registry implementation with full tool binding, sandbox enforcement, and integration with telemetry. Transition from Phase 0 stub to production-ready registry.

## Document References
- **Primary:** Section 6.2 (Phase 1, Week 7-8: MCP-native Tool Registry completion), Section 3.3.3 (Tool Registry)
- **Supporting:** Week 7 (MCP integration started), Week 5-6 (telemetry engine)

## Deliverables
- [ ] Finalize MCP Tool Registry implementation from Week 7
  - Production-ready MCP client with connection pooling
  - Tool discovery caching and refresh mechanism
  - Handle tool updates and removals
- [ ] Complete sandbox enforcement
  - Runtime constraint validation on every tool invocation
  - Audit all constraint violations (block and log)
  - Resource limit enforcement (memory, CPU, disk, network)
- [ ] Tool binding lifecycle management
  - Tool registration (from MCP discovery)
  - Tool activation (ready for invocation)
  - Tool deactivation (sandbox failure, security incident)
  - Tool removal (MCP server-side deletion)
- [ ] Extended tool catalog and config
  - Register 5 production tools:
    1. Web Search (e.g., Tavily Search, Google Search API)
    2. Code Executor (e.g., Python REPL, Node.js runner)
    3. File System (e.g., local path access with scoping)
    4. Database (e.g., SQL executor with per-user isolation)
    5. Calculator (e.g., safe math operations)
  - Each with complete sandbox config and effect class declaration
- [ ] Telemetry event emission integration
  - Log all tool operations (discovery, binding, invocation, sandbox violations)
  - Attach sandbox config to ToolCallRequested events
  - Emit SandboxViolation events on constraint breaches
  - Cost attribution for sandbox validation overhead
- [ ] Recovery and error handling
  - Handle MCP server disconnection and reconnection
  - Graceful degradation when tools become unavailable
  - Sandbox engine failure recovery (fail-safe to DENIED)
- [ ] Performance and reliability
  - Tool lookup latency <5ms (cached)
  - Sandbox validation <1ms per constraint
  - Telemetry event emission <1ms
  - Connection pooling for MCP client
  - Circuit breaker for failed tools
- [ ] Documentation and runbooks
  - Complete MCP Tool Registry architecture
  - Tool onboarding guide (how to add new tools)
  - Sandbox configuration guide
  - Troubleshooting and incident response
- [ ] Comprehensive testing
  - All 5 tools registered and callable
  - Sandbox enforcement on each tool (positive and negative tests)
  - MCP reconnection and tool availability updates
  - Cost attribution validation

## Technical Specifications

### Production MCP Client with Connection Pooling
```rust
pub struct ProductionMCPClient {
    pools: Arc<RwLock<HashMap<String, ConnectionPool>>>,
    default_timeout: Duration,
    max_retries: u32,
}

pub struct ConnectionPool {
    addr: String,
    connections: Arc<Mutex<Vec<MCPConnection>>>,
    max_connections: usize,
    idle_timeout: Duration,
}

impl ProductionMCPClient {
    pub async fn new(config: MCPClientConfig) -> Result<Self, Error> {
        Ok(Self {
            pools: Arc::new(RwLock::new(HashMap::new())),
            default_timeout: config.timeout,
            max_retries: config.max_retries,
        })
    }

    pub async fn list_tools(&self, server_addr: &str) -> Result<Vec<MCPTool>, Error> {
        let mut retries = 0;
        loop {
            match self.list_tools_internal(server_addr).await {
                Ok(tools) => return Ok(tools),
                Err(e) if retries < self.max_retries => {
                    retries += 1;
                    tokio::time::sleep(Duration::from_millis(100 * retries as u64)).await;
                }
                Err(e) => return Err(e),
            }
        }
    }

    async fn list_tools_internal(&self, server_addr: &str) -> Result<Vec<MCPTool>, Error> {
        let mut pools = self.pools.write().await;
        let pool = pools.entry(server_addr.to_string())
            .or_insert_with(|| ConnectionPool::new(server_addr.to_string(), 10));

        let conn = pool.acquire().await?;
        let result = conn.rpc_call("tools/list", serde_json::json!({})).await;
        pool.release(conn).await;
        result
    }
}

impl ConnectionPool {
    fn new(addr: String, max_connections: usize) -> Self {
        Self {
            addr,
            connections: Arc::new(Mutex::new(Vec::new())),
            max_connections,
            idle_timeout: Duration::from_secs(300),
        }
    }

    async fn acquire(&mut self) -> Result<MCPConnection, Error> {
        let mut conns = self.connections.lock().await;
        if let Some(conn) = conns.pop() {
            return Ok(conn);
        }

        if conns.len() < self.max_connections {
            let conn = MCPConnection::connect(&self.addr).await?;
            return Ok(conn);
        }

        Err(Error::PoolExhausted)
    }

    async fn release(&self, conn: MCPConnection) {
        let mut conns = self.connections.lock().await;
        conns.push(conn);
    }
}
```

### Tool Binding Lifecycle State Machine
```rust
#[derive(Clone, Debug, PartialEq)]
pub enum ToolBindingState {
    Discovered,      // Found via MCP ListTools
    Activated,       // Ready for invocation
    Degraded,        // Partial functionality (sandbox soft failures)
    Deactivated,     // Disabled due to failures
    Removed,         // Removed by MCP server
}

pub struct ToolBindingLifecycle {
    binding: ToolBinding,
    state: ToolBindingState,
    failure_count: u32,
    last_failure_time: Option<Instant>,
    failure_threshold: u32,
}

impl ToolBindingLifecycle {
    pub async fn on_invocation_failure(&mut self, error: &Error,
                                       telemetry: &TelemetryEngine) -> Result<(), Error>
    {
        self.failure_count += 1;
        self.last_failure_time = Some(Instant::now());

        if self.failure_count >= self.failure_threshold {
            self.state = ToolBindingState::Deactivated;
            telemetry.emit_event(CEFEvent {
                event_type: EventType::ToolDeactivated,
                actor: "tool_lifecycle",
                resource: self.binding.tool.clone(),
                action: "DEACTIVATE",
                context: {
                    "failure_count": format!("{}", self.failure_count),
                    "reason": format!("{:?}", error),
                }.into(),
                ..Default::default()
            }).await.ok();
        }

        Ok(())
    }

    pub fn can_invoke(&self) -> bool {
        matches!(self.state, ToolBindingState::Activated | ToolBindingState::Degraded)
    }
}
```

### 5-Tool Production Catalog
```rust
pub struct ToolCatalog;

impl ToolCatalog {
    pub fn create_default_catalog() -> HashMap<String, ToolBinding> {
        let mut catalog = HashMap::new();

        // Tool 1: Web Search (Tavily API)
        catalog.insert("web_search".to_string(), ToolBinding {
            id: "tool-web-search".to_string(),
            tool: "web_search".to_string(),
            effect_class: EffectClass::READ_ONLY,
            sandbox_config: SandboxConfig {
                allowed_domains: vec!["api.tavily.com".to_string(), "google.com".to_string()],
                allowed_paths: vec![],
                allowed_syscalls: vec!["socket".to_string()],
                resource_limits: ResourceLimits {
                    max_memory_mb: 256,
                    max_cpu_percent: 50,
                    max_network_bandwidth_mbps: 5,
                    max_disk_bandwidth_mbps: 0,
                    max_execution_time_ms: 30_000,
                },
            },
            ..Default::default()
        });

        // Tool 2: Code Executor (Python REPL)
        catalog.insert("code_executor".to_string(), ToolBinding {
            id: "tool-code-executor".to_string(),
            tool: "code_executor".to_string(),
            effect_class: EffectClass::WRITE_COMPENSABLE,
            sandbox_config: SandboxConfig {
                allowed_domains: vec![],
                allowed_paths: vec![PathBuf::from("/tmp/sandbox/code")],
                allowed_syscalls: vec!["execve".to_string(), "read".to_string(), "write".to_string()],
                resource_limits: ResourceLimits {
                    max_memory_mb: 1024,
                    max_cpu_percent: 100,
                    max_network_bandwidth_mbps: 0,
                    max_disk_bandwidth_mbps: 100,
                    max_execution_time_ms: 60_000,
                },
            },
            commit_protocol: Some(CommitProtocol {
                protocol_type: "PREPARE_COMMIT".to_string(),
                prepare_timeout_ms: 5000,
                commit_timeout_ms: 10000,
                rollback_strategy: "delete_temp_files".to_string(),
            }),
            ..Default::default()
        });

        // Tool 3: File System
        catalog.insert("file_system".to_string(), ToolBinding {
            id: "tool-file-system".to_string(),
            tool: "file_system".to_string(),
            effect_class: EffectClass::WRITE_REVERSIBLE,
            sandbox_config: SandboxConfig {
                allowed_domains: vec![],
                allowed_paths: vec![PathBuf::from(".")],
                allowed_syscalls: vec!["read".to_string(), "write".to_string(), "open".to_string()],
                resource_limits: ResourceLimits {
                    max_memory_mb: 128,
                    max_cpu_percent: 50,
                    max_network_bandwidth_mbps: 0,
                    max_disk_bandwidth_mbps: 200,
                    max_execution_time_ms: 10_000,
                },
            },
            ..Default::default()
        });

        // Tool 4: Database (SQL Executor)
        catalog.insert("database".to_string(), ToolBinding {
            id: "tool-database".to_string(),
            tool: "database".to_string(),
            effect_class: EffectClass::WRITE_COMPENSABLE,
            sandbox_config: SandboxConfig {
                allowed_domains: vec!["db.internal.example.com".to_string()],
                allowed_paths: vec![],
                allowed_syscalls: vec!["socket".to_string()],
                resource_limits: ResourceLimits {
                    max_memory_mb: 512,
                    max_cpu_percent: 75,
                    max_network_bandwidth_mbps: 50,
                    max_disk_bandwidth_mbps: 0,
                    max_execution_time_ms: 30_000,
                },
            },
            commit_protocol: Some(CommitProtocol {
                protocol_type: "PREPARE_COMMIT".to_string(),
                prepare_timeout_ms: 3000,
                commit_timeout_ms: 10000,
                rollback_strategy: "database_rollback".to_string(),
            }),
            ..Default::default()
        });

        // Tool 5: Calculator (Safe Math)
        catalog.insert("calculator".to_string(), ToolBinding {
            id: "tool-calculator".to_string(),
            tool: "calculator".to_string(),
            effect_class: EffectClass::READ_ONLY,
            sandbox_config: SandboxConfig {
                allowed_domains: vec![],
                allowed_paths: vec![],
                allowed_syscalls: vec![],
                resource_limits: ResourceLimits {
                    max_memory_mb: 64,
                    max_cpu_percent: 25,
                    max_network_bandwidth_mbps: 0,
                    max_disk_bandwidth_mbps: 0,
                    max_execution_time_ms: 5_000,
                },
            },
            ..Default::default()
        });

        catalog
    }
}
```

### Sandbox Violation Event and Logging
```rust
pub struct SandboxViolationEvent {
    pub binding_id: String,
    pub constraint_type: String,
    pub constraint_value: String,
    pub violation_details: String,
    pub action_taken: String, // BLOCKED, DEGRADED, etc.
}

impl TelemetryEngine {
    pub async fn emit_sandbox_violation(&self, violation: &SandboxViolationEvent) {
        let event = CEFEvent {
            event_type: EventType::SandboxViolation,
            actor: "sandbox_engine",
            resource: violation.binding_id.clone(),
            action: "VALIDATE",
            result: EventResult::DENIED,
            context: {
                "constraint_type": violation.constraint_type.clone(),
                "constraint_value": violation.constraint_value.clone(),
                "violation_details": violation.violation_details.clone(),
                "action_taken": violation.action_taken.clone(),
            }.into(),
            ..Default::default()
        };

        self.emit_event(event).await.ok();
    }
}
```

## Dependencies
- **Blocked by:** Week 7 (MCP integration started), Phase 0 (complete)
- **Blocking:** Week 9-10 (response caching), Week 11-12 (telemetry full implementation)

## Acceptance Criteria
- [ ] Production MCP client with connection pooling implemented
- [ ] All 5 tools registered with sandbox configs and effect classes
- [ ] Sandbox enforcement prevents out-of-constraint invocations
- [ ] Tool binding lifecycle state machine operational (Discovered -> Activated -> Degraded/Deactivated)
- [ ] MCP server disconnection handled gracefully; reconnection automatic
- [ ] Sandbox violation events emitted and logged
- [ ] Tool lookup latency <5ms; sandbox validation <1ms
- [ ] All 5 tools callable and tested (positive and negative cases)
- [ ] Integration tests pass; MCP tool discovery verified
- [ ] Documentation complete; tool onboarding guide written

## Design Principles Alignment
- **Production-ready:** Connection pooling, error recovery, graceful degradation
- **Security enforcement:** Every tool invocation validated against sandbox constraints; fail-safe to DENIED
- **Observability:** All tool operations, sandbox violations, and state transitions logged
- **Resilience:** Circuit breaker for failed tools; automatic MCP reconnection
- **Extensibility:** Catalog pattern makes adding new tools straightforward
