# Engineer 6 — Services: Tool Registry, Telemetry & Compliance — Week 7

## Phase: Phase 1 (Weeks 7-14)

## Weekly Objective
Begin MCP-native Tool Registry implementation with real tool bindings and per-tool sandbox configuration. Replace Phase 0 mock registry with production-ready MCP integration.

## Document References
- **Primary:** Section 6.2 (Phase 1, Week 7-8: MCP-native Tool Registry with real tool binding, sandbox config), Section 3.3.3 (Tool Registry, MCP-native, sandbox, effect classes, response caching)
- **Supporting:** Section 2.11 (ToolBinding), Week 4 (Stub Tool Registry), Week 5-6 (telemetry baseline)

## Deliverables
- [ ] MCP protocol integration layer
  - MCP client initialization and connection management
  - Tool discovery via MCP ListTools RPC
  - Tool schema retrieval and caching
- [ ] Real tool binding implementation
  - Map MCP tools to ToolBinding entities
  - Extract effect_class from tool metadata (or assign default WRITE_IRREVERSIBLE)
  - Extract commit_protocol hints from tool capabilities
  - Validate sandbox compatibility
- [ ] Per-tool sandbox configuration system
  - Define SandboxConfig structure: allowed_domains, allowed_paths, allowed_syscalls, resource_limits
  - Implement configuration DSL for common tools (web search, code executor, file system, database, calculator)
  - Runtime enforcement: check tool invocation against declared sandbox constraints
- [ ] Tool capability matrix
  - Document each of 5 planned tools and their sandbox constraints:
    1. Web Search Tool: network access to search engines, no file system access
    2. Code Executor: local file system (temp dir only), no external network
    3. File System Tool: local filesystem, scoped to working directory, no code execution
    4. Database Tool: network access to designated database only, no external API calls
    5. Calculator Tool: no I/O, computation only
- [ ] Integration with Phase 0 telemetry
  - Emit ToolDiscovered event on MCP ListTools
  - Emit ToolBindingCreated event on successful binding
  - Track sandbox constraint violations as audit events
- [ ] Unit and integration tests
  - MCP connection and tool discovery
  - ToolBinding creation from MCP tool metadata
  - Sandbox enforcement (allow/deny invocation based on constraints)

## Technical Specifications

### MCP Integration Layer
```rust
pub struct MCPToolRegistry {
    mcp_client: Arc<MCPClient>,
    bindings: Arc<RwLock<HashMap<String, ToolBinding>>>,
    sandbox_engine: Arc<SandboxEngine>,
    telemetry: Arc<TelemetryEngine>,
}

pub struct MCPClient {
    // MCP protocol implementation
    // Connects to MCP server and manages tool list
}

impl MCPToolRegistry {
    pub async fn new(mcp_server_addr: &str, telemetry: Arc<TelemetryEngine>)
        -> Result<Self, RegistryError>
    {
        let mcp_client = MCPClient::connect(mcp_server_addr).await?;
        Ok(Self {
            mcp_client: Arc::new(mcp_client),
            bindings: Arc::new(RwLock::new(HashMap::new())),
            sandbox_engine: Arc::new(SandboxEngine::new()),
            telemetry,
        })
    }

    pub async fn discover_tools(&self) -> Result<Vec<String>, RegistryError> {
        // Call MCP ListTools RPC
        let mcp_tools = self.mcp_client.list_tools().await?;

        for mcp_tool in mcp_tools {
            // Create ToolBinding from MCP tool metadata
            let binding = self.create_binding_from_mcp(&mcp_tool).await?;
            self.bindings.write().await.insert(binding.id.clone(), binding.clone());

            // Emit ToolDiscovered event
            self.telemetry.emit_event(CEFEvent {
                event_type: EventType::ToolDiscovered,
                actor: "mcp_registry",
                resource: mcp_tool.name.clone(),
                action: "DISCOVER",
                result: EventResult::COMPLETED,
                context: {
                    "tool_name": mcp_tool.name.clone(),
                    "mcp_server": self.mcp_client.server_addr().to_string(),
                }.into(),
                ..Default::default()
            }).await.ok();
        }

        Ok(mcp_tools.iter().map(|t| t.name.clone()).collect())
    }

    async fn create_binding_from_mcp(&self, mcp_tool: &MCPTool)
        -> Result<ToolBinding, RegistryError>
    {
        // Extract effect_class from metadata, default to WRITE_IRREVERSIBLE
        let effect_class = mcp_tool.metadata.effect_class
            .unwrap_or(EffectClass::WRITE_IRREVERSIBLE);

        // Extract commit protocol if present
        let commit_protocol = mcp_tool.metadata.commit_protocol.clone();

        // Get or create sandbox config for this tool
        let sandbox_config = self.sandbox_engine.get_config_for_tool(&mcp_tool.name).await?;

        let binding = ToolBinding {
            id: format!("tool-mcp-{}", mcp_tool.name),
            tool: mcp_tool.name.clone(),
            agent: "mcp_system".to_string(),
            capability: format!("mcp.invoke.{}", mcp_tool.name),
            schema: mcp_tool.schema.clone(),
            sandbox_config,
            response_cache: CacheConfig {
                ttl_seconds: 3600,
                freshness_policy: "stale_while_revalidate".to_string(),
            },
            effect_class,
            commit_protocol,
        };

        Ok(binding)
    }

    pub async fn get_binding(&self, tool_id: &str) -> Result<ToolBinding, RegistryError> {
        self.bindings.read().await
            .get(tool_id)
            .cloned()
            .ok_or(RegistryError::NotFound)
    }
}
```

### Sandbox Configuration System
```rust
pub struct SandboxConfig {
    pub allowed_domains: Vec<String>,
    pub allowed_paths: Vec<PathBuf>,
    pub allowed_syscalls: Vec<String>,
    pub resource_limits: ResourceLimits,
}

pub struct ResourceLimits {
    pub max_memory_mb: u64,
    pub max_cpu_percent: u32,
    pub max_network_bandwidth_mbps: u32,
    pub max_disk_bandwidth_mbps: u32,
    pub max_execution_time_ms: u64,
}

pub struct SandboxEngine;

impl SandboxEngine {
    pub async fn get_config_for_tool(&self, tool_name: &str)
        -> Result<SandboxConfig, SandboxError>
    {
        match tool_name {
            "web_search" => Ok(SandboxConfig {
                allowed_domains: vec![
                    "google.com".to_string(),
                    "bing.com".to_string(),
                    "duckduckgo.com".to_string(),
                ],
                allowed_paths: vec![],
                allowed_syscalls: vec!["socket".to_string(), "sendto".to_string()],
                resource_limits: ResourceLimits {
                    max_memory_mb: 256,
                    max_cpu_percent: 50,
                    max_network_bandwidth_mbps: 10,
                    max_disk_bandwidth_mbps: 0,
                    max_execution_time_ms: 30_000,
                },
            }),
            "code_executor" => Ok(SandboxConfig {
                allowed_domains: vec![],
                allowed_paths: vec![PathBuf::from("/tmp/sandbox/code")],
                allowed_syscalls: vec![
                    "read".to_string(),
                    "write".to_string(),
                    "open".to_string(),
                    "close".to_string(),
                    "execve".to_string(),
                ],
                resource_limits: ResourceLimits {
                    max_memory_mb: 512,
                    max_cpu_percent: 100,
                    max_network_bandwidth_mbps: 0,
                    max_disk_bandwidth_mbps: 50,
                    max_execution_time_ms: 60_000,
                },
            }),
            "file_system" => Ok(SandboxConfig {
                allowed_domains: vec![],
                allowed_paths: vec![PathBuf::from(".")],
                allowed_syscalls: vec![
                    "read".to_string(),
                    "write".to_string(),
                    "open".to_string(),
                    "close".to_string(),
                    "stat".to_string(),
                    "lstat".to_string(),
                ],
                resource_limits: ResourceLimits {
                    max_memory_mb: 128,
                    max_cpu_percent: 50,
                    max_network_bandwidth_mbps: 0,
                    max_disk_bandwidth_mbps: 100,
                    max_execution_time_ms: 10_000,
                },
            }),
            "database" => Ok(SandboxConfig {
                allowed_domains: vec!["db.internal.example.com".to_string()],
                allowed_paths: vec![],
                allowed_syscalls: vec!["socket".to_string(), "sendto".to_string()],
                resource_limits: ResourceLimits {
                    max_memory_mb: 256,
                    max_cpu_percent: 75,
                    max_network_bandwidth_mbps: 50,
                    max_disk_bandwidth_mbps: 0,
                    max_execution_time_ms: 30_000,
                },
            }),
            "calculator" => Ok(SandboxConfig {
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
            }),
            _ => Err(SandboxError::UnknownTool(tool_name.to_string())),
        }
    }

    pub async fn validate_invocation(&self, binding: &ToolBinding,
                                     constraint: &SandboxConstraint)
        -> Result<(), SandboxError>
    {
        match constraint {
            SandboxConstraint::NetworkDomain(domain) => {
                if binding.sandbox_config.allowed_domains.contains(domain) {
                    Ok(())
                } else {
                    Err(SandboxError::ConstraintViolation(
                        format!("Network domain {} not allowed", domain)
                    ))
                }
            }
            SandboxConstraint::FilePath(path) => {
                if binding.sandbox_config.allowed_paths.iter()
                    .any(|p| path.starts_with(p))
                {
                    Ok(())
                } else {
                    Err(SandboxError::ConstraintViolation(
                        format!("File path {} not allowed", path.display())
                    ))
                }
            }
            SandboxConstraint::Memory(bytes) => {
                if bytes <= binding.sandbox_config.resource_limits.max_memory_mb * 1_000_000 {
                    Ok(())
                } else {
                    Err(SandboxError::ConstraintViolation(
                        format!("Memory limit {} MB exceeded", binding.sandbox_config.resource_limits.max_memory_mb)
                    ))
                }
            }
        }
    }
}
```

### Integration with Telemetry
```rust
pub enum EventType {
    ThoughtStep,
    ToolCallRequested,
    ToolCallCompleted,
    PolicyDecision,
    MemoryAccess,
    IPCMessage,
    PhaseTransition,
    CheckpointCreated,
    SignalDispatched,
    ExceptionRaised,
    ToolDiscovered,        // New in Phase 1
    ToolBindingCreated,    // New in Phase 1
    SandboxViolation,      // New in Phase 1
}
```

## Dependencies
- **Blocked by:** Phase 0 (Weeks 1-6 complete), MCP server availability
- **Blocking:** Week 8 (complete MCP-native Tool Registry), Week 9-10 (response caching), Week 11-12 (telemetry full implementation)

## Acceptance Criteria
- [ ] MCPClient connects to MCP server and fetches tool list
- [ ] ToolBinding created for each MCP tool with correct effect class and sandbox config
- [ ] All 5 planned tools have defined sandbox constraints (domain, path, resource limits)
- [ ] Sandbox validation prevents out-of-constraint invocations
- [ ] ToolDiscovered and ToolBindingCreated events emitted and logged
- [ ] SandboxViolation events emitted on constraint violations
- [ ] Unit tests cover MCP connection, binding creation, sandbox enforcement
- [ ] Integration tests with mock MCP server pass

## Design Principles Alignment
- **Security first:** Per-tool sandbox constraints limit blast radius of tool compromise
- **MCP-native:** Leverages MCP protocol for tool discovery and binding
- **Audit trail:** All tool discovery and binding events logged
- **Resource governance:** Resource limits prevent resource exhaustion attacks
- **Explicit safety:** Sandbox configuration mandatory for all tools; defaults conservative
