# Tool Registry & Telemetry

> **Crate:** [`tool_registry_telemetry`](Cargo.toml)
> **Stream:** 2 — Kernel Services
> **Layer:** L1 (Kernel Services)
> **Owner:** Engineer 02
> **Status:** Active

---

## 1. Purpose & Scope

Manages dynamic tool binding and telemetry collection for agents. Tool registry allows agents to discover and invoke external tools (APIs, databases, search engines). Telemetry collects execution traces, metrics, and logs for observability and debugging.

**Key Responsibilities:**
- Tool registry and dynamic binding
- Tool capability validation and access control
- Telemetry event collection and buffering
- Metrics export (Prometheus format)
- Distributed tracing integration
- Event log persistence and replay

**In Scope:**
- Tool metadata and interface descriptions
- Tool invocation sandboxing
- Event serialization and buffering
- Metrics aggregation

**Out of Scope:**
- Tool implementation (provided by tool authors)
- Long-term metrics storage (handled by external systems)
- Real-time alerting (handled by monitoring stack)

---

## 2. Engineering Plan Reference

**Relevant Sections:**
- Section 2.6: ToolBinding domain entity
- Section 4.7: Observability & Telemetry

**Domain Model Entities Involved:**
- **ToolBinding** — Bridge between agents and external tools
- **CognitiveTask** — Produce telemetry events during execution
- **Agent** — Own tool bindings and capabilities

---

## 3. Architecture & Design

### 3.1 High-Level Architecture

```
┌──────────────────────────────┐
│  Tool Registry API           │
│  register(), invoke()        │
└──────────────────────────────┘
             ↓
┌──────────────────────────────┐
│  Tool Registry Database      │
│  (Tool metadata, versions)   │
└──────────────────────────────┘
             ↓
┌──────────────────────────────┐
│  Tool Invocation Handler     │
│  (Capability checks, sandbox)│
└──────────────────────────────┘
             ↓         ↓
    ┌────────────┐  ┌─────────────┐
    │ Telemetry  │  │ Metrics     │
    │ Collector  │  │ Exporter    │
    └────────────┘  └─────────────┘
```

### 3.2 Key Invariants

1. **Tool Access Control**: Only agents with tool capability can invoke
   - Enforced: Capability check before invocation
   - Impact: Untrusted agents cannot abuse tools

2. **Tool Invocation Isolation**: Tool output is sandboxed (cannot access CT memory directly)
   - Enforced: Subprocess/container isolation
   - Impact: Malicious tool output cannot corrupt CT state

3. **Telemetry Non-Blocking**: Telemetry collection never blocks task execution
   - Enforced: Asynchronous event buffering
   - Impact: Tracing overhead is minimal

---

## 4. Dependencies

| Crate | Type | Layer | Why |
|-------|------|-------|-----|
| `ct_lifecycle` | Internal | L0 | Query CT for telemetry context |
| `capability_engine` | Internal | L0 | Check tool invocation capability |
| `semantic_memory` | Internal | L1 | Allocate telemetry buffers |

---

## 5. Public API Surface

```rust
/// Tool binding (bridge to external tool)
pub struct ToolBinding {
    pub id: ToolId,
    pub name: String,
    pub owner: EntityId,
    pub capability: Capability,
    pub schema: ToolSchema,
}

/// Tool schema (OpenAPI-like spec)
pub struct ToolSchema {
    pub inputs: HashMap<String, JsonSchema>,
    pub outputs: JsonSchema,
}

/// Register a new tool
pub fn tool_register(
    owner: EntityId,
    binding: ToolBinding,
) -> CsResult<ToolId>;

/// Invoke a tool (with capability check)
pub fn tool_invoke(
    task_id: TaskId,
    tool_id: ToolId,
    args: serde_json::Value,
) -> CsResult<serde_json::Value>;

/// Telemetry event
pub struct TelemetryEvent {
    pub task_id: TaskId,
    pub timestamp: Instant,
    pub event_type: EventType,
    pub data: serde_json::Value,
}

pub enum EventType {
    TaskSpawned { parent_id: TaskId },
    TaskExited { status: ExitStatus },
    ToolInvoked { tool_id: ToolId },
    PhaseTransition { from: CTPhase, to: CTPhase },
    SignalReceived { signal: CognitiveSignal },
}

/// Emit telemetry event
pub fn telemetry_emit(event: TelemetryEvent) -> CsResult<()>;

/// Get aggregated metrics
pub fn metrics_get(task_id: TaskId) -> CsResult<Metrics>;
```

---

## 6. Building & Testing

```bash
cargo build -p tool_registry_telemetry
cargo test -p tool_registry_telemetry
```

**Key Test Scenarios:**
1. Tool registration — Success/failure for valid/invalid schemas
2. Tool invocation with capabilities — Success only with capability
3. Telemetry event buffering — No message loss
4. Metrics aggregation — Correct calculation of percentiles
5. Distributed tracing — Trace context propagated across calls

---

## 7. Design Decisions Log

### 7.1 "Dynamic Tool Registry vs. Static?"

**Decision:** Dynamic tool registry allowing runtime registration.

**Alternatives:**
1. Static registry — Tools compiled into kernel
2. Manual configuration files — Tools listed in YAML/TOML

**Rationale:**
- Runtime registration allows new tools without kernel recompile
- Agents can dynamically provision tools (e.g., API clients)
- Matches modern serverless/microservices patterns (AWS Lambda layers)
- Simpler extensibility

**Date:** 2026-03-01
**Author:** Engineer 02

### 7.2 "Synchronous Telemetry vs. Async Buffering?"

**Decision:** Asynchronous buffering — emitting events never blocks.

**Alternatives:**
1. Synchronous writes — Events immediately persisted
2. Polling — Periodic batch collection

**Rationale:**
- Async ensures telemetry overhead is minimal (< 1 µs)
- Buffering reduces I/O overhead
- Batch writes more efficient than per-event I/O
- Can drop low-priority events if buffer full (graceful degradation)

**Date:** 2026-03-01
**Author:** Engineer 02

---

## 8. Performance Characteristics

| Operation | Complexity | Notes |
|-----------|-----------|-------|
| `tool_register` | O(1) | Hash table insertion |
| `tool_invoke` | O(n) | n = tool execution time |
| `telemetry_emit` | O(1) | Async buffer append |
| `metrics_get` | O(n) | n = events in window |

---

## 9. Common Pitfalls & Troubleshooting

**Mistake 1: Not defining tool schema**
```rust
// ✗ WRONG: Vague schema
let schema = ToolSchema {
    inputs: map!("query" => JsonSchema::Any),
    outputs: JsonSchema::Any,
};

// ✓ RIGHT: Explicit, typed schema
let schema = ToolSchema {
    inputs: map!(
        "query" => JsonSchema::String { pattern: Some("\\d+") }
    ),
    outputs: JsonSchema::Object {
        properties: map!("results" => JsonSchema::Array { items: ... }),
    },
};
```

**Mistake 2: Assuming tool output is trusted**
```rust
// ✗ WRONG: Using tool output directly
let result = tool_invoke(task_id, tool_id, args)?;
let count = result["count"].as_i64().unwrap();  // Can panic!

// ✓ RIGHT: Validate against schema
let result = tool_invoke(task_id, tool_id, args)?;
let validated = validate_against_schema(&result, &tool_schema)?;
let count = validated["count"].as_i64()?;
```

---

## 10. Integration Points

| Module | Integration | Protocol |
|--------|-----------|----------|
| `ct_lifecycle` | Emit task lifecycle events | Telemetry API |
| `ipc_signals_exceptions` | Emit signal events | Telemetry API |
| All L2+ services | Export Prometheus metrics | Metrics API |

---

## 11. Future Roadmap

**Planned Improvements:**
- OpenAPI schema generation — Automatically derive schemas from Rust types
- Tool marketplace — Publish/discover tools across agents
- Cost tracking — Attribute tool invocation costs to agents

**Technical Debt:**
- Telemetry buffer overflow handling is simplistic (drop old events)
- Metrics aggregation could be parallelized for large event counts

---

## 12. References

- **OpenAPI Spec:** https://spec.openapis.org/
- **Prometheus Metrics:** https://prometheus.io/docs/concepts/data_model/
- **Distributed Tracing (OpenTelemetry):** https://opentelemetry.io/

---

**README Version:** 1.0
**Last Updated:** 2026-03-01
**Owner:** Engineer 02
