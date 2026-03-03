# Week 12 — LangChain Adapter 75%: Callback System, Capability Gating & Lifecycle Integration

## Executive Summary

This technical design document specifies the LangChain adapter's advancement to 75% completion, focusing on callback system integration, capability gating mechanisms, and end-to-end lifecycle hooks. The adapter bridges LangChain's asynchronous execution model with XKernal's cognitive substrate through callback translation, context propagation, and fail-fast capability validation. This phase enables production-ready agent orchestration with full observability, error recovery, and semantic memory persistence.

## Problem Statement

LangChain agents operate in a callback-driven execution paradigm that must synchronize with XKernal's synchronous task model while maintaining complete context propagation. Current gaps include:

1. **Callback Translation Asymmetry**: LangChain OnChainStart/End events lack direct CEF event equivalents, requiring bidirectional translation and timestamp correlation.
2. **Capability Validation Latency**: Tool binding occurs before capability checks, risking runtime failures instead of fail-fast rejection.
3. **Context Leakage**: agent_id, session_id, user_id context is lost across memory operations and task spawning boundaries.
4. **Memory Mapping Incompleteness**: VectorStoreMemory and ConversationKGMemory lack L3 semantic layer integration, forcing manual translation.
5. **Lifecycle Fragmentation**: Agent initialization, session binding, and chain execution lack unified hooks for observability and recovery.

This design introduces systematic callback handling, capability gating, context propagation, and lifecycle integration to achieve 75% completion milestone.

## Architecture

### 1. Callback System & CEF Translation

**Callback Handler Chain**:
```
LangChain Callbacks (OnChainStart/OnChainEnd/OnToolStart/OnToolEnd)
  ↓ [CallbackHandler::translate()]
XKernal CEF Events (task_spawn, mem_write, semantic_publish)
  ↓ [ContextPropagator::inject()]
Kernel Event Stream with agent_id, session_id, user_id, trace_id
  ↓ [ErrorRecoveryStrategy::process()]
Decision: Continue | Fail | Escalate
```

**Callback Types & CEF Mapping**:
- `OnChainStart` → `task_spawn(kind=chain_task, agent_id, session_id)`
- `OnChainEnd` → `mem_write(chain_result, L3_semantic)` + `semantic_publish(chain_summary)`
- `OnToolStart` → `task_spawn(kind=tool_task, capability_check)` + `cap_check(agent_id, tool_name)`
- `OnToolEnd` → `mem_write(tool_result, trace_id)` + event logging
- `OnAgentAction` → `semantic_publish(action_intent, reasoning)` for KG enrichment
- `OnAgentFinish` → `mem_write(final_result, agent_id)` + `on_session_close()` hook

### 2. Enhanced Memory Mapping

**VectorStoreMemory → L3 Semantic Layer**:
```rust
pub struct SemanticVectorStore {
    embeddings: Arc<EmbeddingModel>,
    vector_index: VectorIndex,
    context: ContextMetadata,
}

impl VectorStoreMemory {
    pub fn persist_semantic(&self, memory_key: &str, agent_id: &str, session_id: &str)
        -> Result<SemanticWriteHandle> {
        let embedding = self.embeddings.embed(memory_key)?;
        let semantic_write = SemanticWrite {
            vector: embedding,
            metadata: ContextMetadata { agent_id, session_id, timestamp: now() },
            layer: L3_SEMANTIC,
            operation: WriteOp::Append,
        };
        KERNEL.mem_write(semantic_write)
    }
}
```

**ConversationKGMemory → KG Triple Layer**:
```rust
pub struct ConversationKGMemory {
    kg_triples: Vec<KGTriple>,
    entity_map: HashMap<String, EntityId>,
}

impl ConversationKGMemory {
    pub fn persist_kg_triples(&self, session_id: &str) -> Result<Vec<TripleWriteHandle>> {
        self.kg_triples.iter().map(|triple| {
            let write = TripleWrite {
                subject: triple.subject.clone(),
                predicate: triple.predicate.clone(),
                object: triple.object.clone(),
                context: ContextMetadata { session_id, timestamp: now() },
            };
            KERNEL.mem_write(write)
        }).collect()
    }
}
```

### 3. Capability Gating System

**Pre-Binding Validation**:
```rust
pub struct CapabilityGate {
    agent_id: String,
    required_caps: HashSet<Capability>,
    cache: Arc<RwLock<CapabilityCache>>,
}

impl CapabilityGate {
    pub fn validate_before_binding(&self, tool: &Tool) -> Result<()> {
        let required = tool.required_capabilities();

        for cap in required {
            let has_cap = self.cap_check_syscall(&self.agent_id, &cap)?;
            if !has_cap {
                return Err(CapabilityError::Missing {
                    agent_id: self.agent_id.clone(),
                    capability: cap,
                    tool: tool.name.clone(),
                });
            }
        }
        Ok(())
    }

    fn cap_check_syscall(&self, agent_id: &str, cap: &Capability) -> Result<bool> {
        // Syscall to kernel: cap_check(agent_id, capability_name, scope)
        KERNEL.syscall(SyscallRequest {
            syscall: "cap_check",
            args: json!({
                "agent_id": agent_id,
                "capability": cap.name(),
                "scope": cap.scope(),
            }),
        }).map(|resp| resp.get_bool("allowed"))
    }
}
```

### 4. Context Propagation

**Context Injector**:
```rust
pub struct ContextPropagator {
    agent_id: String,
    session_id: String,
    user_id: String,
    trace_id: String,
}

impl ContextPropagator {
    pub fn inject_into_event(&self, event: &mut CEFEvent) {
        event.set_field("agent_id", &self.agent_id);
        event.set_field("session_id", &self.session_id);
        event.set_field("user_id", &self.user_id);
        event.set_field("trace_id", &self.trace_id);
    }

    pub fn inject_into_mem_write(&self, write: &mut MemWrite) {
        write.context = ContextMetadata {
            agent_id: self.agent_id.clone(),
            session_id: self.session_id.clone(),
            user_id: self.user_id.clone(),
            trace_id: self.trace_id.clone(),
            timestamp: SystemTime::now(),
        };
    }

    pub fn spawn_task_with_context(&self, task: Task) -> Result<TaskHandle> {
        let mut task = task;
        self.inject_into_event(&mut task.event);
        KERNEL.task_spawn(task)
    }
}
```

### 5. Lifecycle Hooks

**Agent Lifecycle**:
```rust
pub trait LifecycleHook: Send + Sync {
    fn on_agent_loaded(&self, agent: &Agent) -> Result<()>;
    fn on_session_init(&self, session: &Session) -> Result<()>;
    fn on_chain_start(&self, chain: &Chain, ctx: &ContextMetadata) -> Result<()>;
    fn on_chain_end(&self, result: &ChainResult, ctx: &ContextMetadata) -> Result<()>;
    fn on_tool_start(&self, tool: &Tool, input: &str) -> Result<()>;
    fn on_tool_end(&self, result: &ToolResult) -> Result<()>;
    fn on_error(&self, error: &ChainError, recovery: &RecoveryAction) -> Result<()>;
    fn on_session_close(&self, session: &Session) -> Result<()>;
}

pub struct LifecycleManager {
    hooks: Vec<Arc<dyn LifecycleHook>>,
}

impl LifecycleManager {
    pub fn fire_chain_start(&self, chain: &Chain, ctx: &ContextMetadata) -> Result<()> {
        for hook in &self.hooks {
            hook.on_chain_start(chain, ctx)?;
        }
        Ok(())
    }
}
```

### 6. CT Graph & Cycle Detection

**Enhanced CT Graph Builder**:
```rust
pub struct CTGraphBuilder {
    nodes: Vec<CTNode>,
    edges: Vec<(usize, usize)>,
    visited: HashSet<usize>,
    rec_stack: HashSet<usize>,
}

impl CTGraphBuilder {
    pub fn detect_cycles(&self) -> Result<Vec<Cycle>> {
        let mut cycles = Vec::new();
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for start in 0..self.nodes.len() {
            if !visited.contains(&start) {
                self.dfs_cycle_detect(start, &mut visited, &mut rec_stack, &mut cycles)?;
            }
        }
        Ok(cycles)
    }

    fn dfs_cycle_detect(&self, node: usize, visited: &mut HashSet<usize>,
                       rec_stack: &mut HashSet<usize>, cycles: &mut Vec<Cycle>) -> Result<()> {
        visited.insert(node);
        rec_stack.insert(node);

        for edge in self.edges.iter() {
            if edge.0 == node {
                let neighbor = edge.1;
                if !visited.contains(&neighbor) {
                    self.dfs_cycle_detect(neighbor, visited, rec_stack, cycles)?;
                } else if rec_stack.contains(&neighbor) {
                    cycles.push(Cycle::from_path(node, neighbor));
                }
            }
        }
        rec_stack.remove(&node);
        Ok(())
    }

    pub fn validate_dag(&self) -> Result<DAGValidation> {
        let cycles = self.detect_cycles()?;
        if cycles.is_empty() {
            Ok(DAGValidation {
                is_valid: true,
                cycles: vec![],
            })
        } else {
            Err(GraphError::CycleDetected(cycles))
        }
    }
}
```

### 7. Error Recovery Strategy

**Error Recovery Configuration**:
```rust
pub enum AgentErrorMode {
    FailFast,      // Stop on first error
    SkipStep,      // Log error, skip step, continue
    Escalate,      // Log and escalate to human
}

pub struct ErrorRecoveryStrategy {
    mode: AgentErrorMode,
    max_retries: usize,
    backoff_ms: u64,
}

impl ErrorRecoveryStrategy {
    pub fn process_step_failure(&self, error: &ChainError,
                                ctx: &ContextMetadata) -> Result<RecoveryAction> {
        match self.mode {
            AgentErrorMode::FailFast => {
                KERNEL.log_error_event(error, ctx)?;
                Err(error.clone())
            }
            AgentErrorMode::SkipStep => {
                KERNEL.log_error_event(error, ctx)?;
                Ok(RecoveryAction::ContinueNextStep)
            }
            AgentErrorMode::Escalate => {
                KERNEL.log_error_event(error, ctx)?;
                KERNEL.escalate_to_human(error, ctx)?;
                Ok(RecoveryAction::PendingHumanReview)
            }
        }
    }
}
```

## Implementation

### Callback Handler Implementation

```rust
pub struct LangChainCallbackHandler {
    propagator: Arc<ContextPropagator>,
    lifecycle: Arc<LifecycleManager>,
    recovery: Arc<ErrorRecoveryStrategy>,
}

impl LangChainCallbackHandler {
    pub fn new(agent_id: String, session_id: String, user_id: String) -> Self {
        let propagator = Arc::new(ContextPropagator {
            agent_id,
            session_id,
            user_id,
            trace_id: uuid::Uuid::new_v4().to_string(),
        });

        Self {
            propagator,
            lifecycle: Arc::new(LifecycleManager { hooks: vec![] }),
            recovery: Arc::new(ErrorRecoveryStrategy {
                mode: AgentErrorMode::SkipStep,
                max_retries: 3,
                backoff_ms: 100,
            }),
        }
    }

    pub fn on_chain_start(&self, chain: &Chain) -> Result<()> {
        let mut event = CEFEvent::new("chain_start");
        self.propagator.inject_into_event(&mut event);
        event.set_field("chain_name", &chain.name);

        let ctx = ContextMetadata::from_propagator(&self.propagator);
        self.lifecycle.fire_chain_start(chain, &ctx)?;

        KERNEL.publish_event(event)?;
        Ok(())
    }

    pub fn on_chain_end(&self, result: &ChainResult) -> Result<()> {
        let mut write = MemWrite::semantic_layer();
        self.propagator.inject_into_mem_write(&mut write);
        write.data = serde_json::to_value(result)?;

        KERNEL.mem_write(write)?;

        let ctx = ContextMetadata::from_propagator(&self.propagator);
        self.lifecycle.fire_chain_end(result, &ctx)?;

        Ok(())
    }

    pub fn on_tool_start(&self, tool: &Tool, input: &str) -> Result<()> {
        let gate = CapabilityGate {
            agent_id: self.propagator.agent_id.clone(),
            required_caps: tool.required_capabilities(),
            cache: Arc::new(RwLock::new(CapabilityCache::new())),
        };

        gate.validate_before_binding(tool)?;

        let mut event = CEFEvent::new("tool_start");
        self.propagator.inject_into_event(&mut event);
        event.set_field("tool", &tool.name);
        event.set_field("input", input);

        KERNEL.publish_event(event)?;
        self.lifecycle.fire_tool_start(tool, input)?;

        Ok(())
    }

    pub fn on_tool_end(&self, result: &ToolResult) -> Result<()> {
        let mut write = MemWrite::semantic_layer();
        self.propagator.inject_into_mem_write(&mut write);
        write.data = serde_json::to_value(result)?;

        KERNEL.mem_write(write)?;
        self.lifecycle.fire_tool_end(result)?;

        Ok(())
    }
}
```

## Testing

### End-to-End Test Scenario

```rust
#[tokio::test]
async fn test_langchain_agent_with_full_trace() {
    // Setup: Create kernel instance
    let kernel = XKernel::new();

    // Create agent with 4 tools
    let agent = Agent::new("research_agent")
        .with_tool(SearchTool::new())
        .with_tool(WebScrapeTool::new())
        .with_tool(KGQueryTool::new())
        .with_tool(SummarizeTool::new());

    // Initialize session
    let session = Session::new("session_123", "user_456");
    let session_id = session.id.clone();

    // Create callback handler
    let handler = LangChainCallbackHandler::new(
        agent.id.clone(),
        session_id.clone(),
        "user_456".to_string(),
    );

    // Execute chain with callback tracking
    let chain = agent.create_chain("research_task");
    handler.on_chain_start(&chain).expect("chain start");

    // Simulate tool execution
    let search_tool = agent.get_tool("search").unwrap();
    handler.on_tool_start(search_tool, "quantum computing").expect("tool start");

    // Verify context propagation
    let events = KERNEL.get_events_for_session(&session_id).await;
    assert!(events.iter().any(|e| {
        e.get_field("agent_id") == Some(agent.id.clone()) &&
        e.get_field("session_id") == Some(session_id.clone())
    }));

    // Verify memory writes
    let mem_reads = KERNEL.mem_read(
        MemQuery::session_id(&session_id),
        L3_SEMANTIC,
    ).await.expect("mem read");
    assert!(!mem_reads.is_empty());

    // Verify lifecycle hooks fired
    assert!(handler.lifecycle.hooks_executed.contains(&"on_chain_start"));
}
```

## Acceptance Criteria

1. **Callback Translation**: All LangChain callbacks (OnChainStart/End, OnToolStart/End, OnAgentAction, OnAgentFinish) translate to CEF events with <100ms overhead.
2. **Context Propagation**: agent_id, session_id, user_id, trace_id present in 100% of mem_write and task_spawn calls.
3. **Capability Gating**: Tool binding fails fast with explicit error if agent lacks required capabilities before execution.
4. **Memory Persistence**: VectorStoreMemory and ConversationKGMemory persist to L3 semantic layer with full context metadata.
5. **Lifecycle Hooks**: All 8 lifecycle hooks (agent_loaded, session_init, chain_start, chain_end, tool_start, tool_end, error, session_close) fire at appropriate execution points.
6. **Cycle Detection**: CT graph builder detects all cycles via DFS with O(V+E) complexity, validates DAG structure.
7. **Error Recovery**: Step failures logged, recovery action (FailFast/SkipStep/Escalate) executed per config, traces preserved.
8. **End-to-End Test**: Agent with 4 tools executes on kernel with full event trace, context propagation, memory persistence, and zero context loss.

## Design Principles

1. **Callback Transparency**: Callback translation maintains semantic equivalence between LangChain and CEF domains, preserving timing and causality.
2. **Fail-Fast Validation**: Capability checks occur before tool binding to prevent runtime surprises and enable deterministic error handling.
3. **Context Continuity**: Context metadata flows through all kernel operations (mem_write, task_spawn, event_publish) to enable end-to-end tracing.
4. **Semantic Memory Layer**: Abstract memory operations map to L3 semantic layer, enabling knowledge graph integration and reasoning.
5. **Lifecycle Observability**: Hooks at each lifecycle stage enable monitoring, recovery, and post-execution analysis.
6. **Graph Integrity**: Cycle detection and DAG validation prevent infinite loops and undefined execution paths in CT graphs.
7. **Recovery Determinism**: Error recovery modes (FailFast, SkipStep, Escalate) are configurable per agent, enabling tailored error handling.

## References

- LangChain Callbacks: https://python.langchain.com/docs/modules/callbacks/
- XKernal Memory Layers: /mnt/XKernal/runtime/memory_layers/
- CT Graph Specification: /mnt/XKernal/runtime/graph_theory/ct_graph.md
- CEF Event Schema: /mnt/XKernal/runtime/events/cef_schema.md
