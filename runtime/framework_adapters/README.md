# Framework Adapters

> **Crate:** [`framework_adapters`](Cargo.toml)
> **Stream:** 3 — Runtime & Orchestration
> **Layer:** L2 (Runtime)
> **Owner:** Engineer 03
> **Status:** Active

---

## 1. Purpose & Scope

Bridges Cognitive Substrate's CT execution model with popular agent frameworks (LangChain, AutoGen, CrewAI). Adapters translate framework-native constructs (chains, agents, tasks) into CTs, enabling existing frameworks to run on Cognitive Substrate while gaining isolation, observability, and resource control.

**Key Responsibilities:**
- LangChain chain → CT spawning and orchestration
- AutoGen agent → Cognitive Substrate agent mapping
- CrewAI crew → AgentCrew native translation
- Async/await → CT phase transitions
- Exception handling and recovery from framework errors
- Metrics and tracing export to observability systems

**In Scope:**
- Adapter implementations for major frameworks
- Mapping framework concepts to domain model
- Error propagation and recovery
- Performance profiling and optimization

**Out of Scope:**
- Framework development (LangChain, AutoGen, CrewAI themselves)
- LLM serving (inference handled by semantic_memory + gpu_accelerator)
- Database/API connectivity (handled by tool_registry)

---

## 2. Engineering Plan Reference

**Relevant Sections:**
- Section 4.8: Runtime Architecture
- Section 3.5.5: Framework Integration Specification

**Domain Model Entities Involved:**
- **CognitiveTask** — Spawned from framework tasks
- **Agent** — Mapped from framework agents
- **AgentCrew** — Mapped from CrewAI crews
- **SemanticChannel** — IPC between framework components

---

## 3. Architecture & Design

### 3.1 High-Level Architecture

```
┌────────────────────────────────────────┐
│  Framework Adapter API                 │
│  chain_to_ct(), agent_to_agent(), ... │
└────────────────────────────────────────┘
             ↓
┌────────────────────────────────────────┐
│  Framework-Specific Adapters           │
├──────────┬────────────┬────────────────┤
│ LangChain│  AutoGen  │  CrewAI        │
│ Adapter  │  Adapter  │  Adapter       │
└──────────┴────────────┴────────────────┘
             ↓
┌────────────────────────────────────────┐
│  Execution Engine                      │
│  (Map to CT lifecycle phases)          │
└────────────────────────────────────────┘
             ↓
    ┌──────────────┬──────────────────┐
    │ Tracing      │ Error Recovery   │
    │ & Metrics    │ & Checkpointing  │
    └──────────────┴──────────────────┘
```

### 3.2 Key Invariants

1. **Framework Transparency**: Frameworks work unchanged (except imports)
   - Enforced: Drop-in adapter layer
   - Impact: Minimal rewrite needed to migrate frameworks

2. **Isolation**: Framework agents cannot interfere with each other
   - Enforced: Separate CTs with isolated memory/capabilities
   - Impact: Untrusted agents sandboxed from each other

3. **Deterministic Execution**: CT lifecycle is deterministic (for replay)
   - Enforced: No global state, all I/O through SemanticChannels
   - Impact: Can use cs-replay for debugging

---

## 4. Dependencies

| Crate | Type | Layer | Why |
|-------|------|-------|-----|
| `ct_lifecycle` | Internal | L0 | Spawn/manage CTs from framework tasks |
| `ipc_signals_exceptions` | Internal | L0 | IPC between framework components |
| `capability_engine` | Internal | L0 | Enforce capability-based access |
| `semantic_memory` | Internal | L1 | Allocate memory for framework state |
| `gpu_accelerator` | Internal | L1 | GPU for LLM inference during REASON |
| `tool_registry_telemetry` | Internal | L1 | Trace and metrics export |
| `semantic_fs_agent_lifecycle` | Internal | L2 | Manage agent filesystem and state |

---

## 5. Public API Surface

```rust
/// Map LangChain chain to CT spawning
pub fn chain_to_ct_spawn(
    agent: &Agent,
    chain: &LangchainChain,
    input: serde_json::Value,
) -> CsResult<CognitiveTask>;

/// Map AutoGen agent to Cognitive Substrate Agent
pub fn autogen_agent_to_cs_agent(
    config: &AutogenAgentConfig,
) -> CsResult<Agent>;

/// Map CrewAI crew to AgentCrew
pub fn crewai_crew_to_cs_crew(
    crew: &CrewAICrew,
) -> CsResult<AgentCrew>;

/// Execute framework code in CT context
pub async fn framework_run(
    task: &mut CognitiveTask,
    framework_code: &FrameworkCode,
) -> CsResult<FrameworkOutput>;

/// Translate framework exceptions to CognitiveExceptions
pub fn framework_exception_to_cs(
    exc: FrameworkException,
) -> CognitiveException;
```

---

## 6. Building & Testing

```bash
cargo build -p framework_adapters
cargo test -p framework_adapters
```

**Build Requirements:**
- LangChain Python SDK (for language interop)
- AutoGen Python SDK
- CrewAI Python SDK

**Key Test Scenarios:**
1. LangChain chain execution — Maps correctly to CTs
2. AutoGen agent spawn — Creates proper Agent structures
3. CrewAI crew isolation — Agents isolated with correct capabilities
4. Exception propagation — Framework errors become CognitiveExceptions
5. Tracing end-to-end — Metrics exported correctly

---

## 7. Design Decisions Log

### 7.1 "Adapter Layer vs. Framework Forks?"

**Decision:** Adapter layer (minimal wrapper) instead of forking frameworks.

**Alternatives:**
1. Fork frameworks — Modify LangChain/AutoGen/CrewAI for CS
2. Direct compilation — Compile Python frameworks to Rust

**Rationale:**
- Adapter layer means upstream updates automatically inherited
- No need to maintain forks
- Decouples Cognitive Substrate from framework roadmaps
- Easier to support multiple frameworks in parallel

**Date:** 2026-03-01
**Author:** Engineer 03

### 7.2 "Synchronous vs. Asynchronous Framework Interface?"

**Decision:** Async interface via tokio (L2 provides async runtime).

**Alternatives:**
1. Synchronous blocking — Simpler but blocks scheduler
2. Pure Rust async — Harder to integrate with Python frameworks

**Rationale:**
- Async allows L2 to interleave many framework tasks
- tokio is proven, production-grade async runtime
- Python frameworks via PyO3 bindings can use async wrappers
- Matches modern framework expectations (async LLM APIs)

**Date:** 2026-03-01
**Author:** Engineer 03

---

## 8. Performance Characteristics

| Operation | Complexity | Notes |
|-----------|-----------|-------|
| `chain_to_ct_spawn` | O(n) | n = chain steps |
| `framework_run` (CT execution) | O(t) | t = reasoning time |
| Exception translation | O(1) | Constant-time mapping |

---

## 9. Common Pitfalls & Troubleshooting

**Mistake 1: Assuming framework memory is isolated**
```python
# ✗ WRONG: Shared global state
_shared_state = {}

def my_agent():
    _shared_state["value"] = ...  # Not isolated!
```

```python
# ✓ RIGHT: Use agent memory
class MyAgent(Agent):
    def __init__(self):
        self.state = {}  # Per-agent state
```

**Mistake 2: Blocking during async execution**
```python
# ✗ WRONG: Blocking I/O in async context
async def reason(self):
    result = requests.get(url)  # BLOCKS! (not awaited)

# ✓ RIGHT: Use async client
async def reason(self):
    async with aiohttp.ClientSession() as session:
        result = await session.get(url)
```

---

## 10. Integration Points

| Module | Integration | Protocol |
|--------|-----------|----------|
| `ct_lifecycle` | Spawn CTs from framework tasks | Direct call |
| `semantic_fs_agent_lifecycle` | Manage agent state/lifecycle | Direct call |
| `semantic_memory` | Allocate memory for framework reasoning | Direct call |
| `gpu_accelerator` | GPU for LLM inference | CSCI wrapper |
| `tool_registry_telemetry` | Export traces and metrics | Telemetry API |

---

## 11. Future Roadmap

**Planned Improvements:**
- LangChain 2.0 adapter — Support latest LangChain APIs
- LlamaIndex adapter — Add document indexing framework
- Pydantic validation — Auto-validate framework outputs

**Technical Debt:**
- PyO3 bindings complex and fragile (consider alternative)
- Framework exception translation incomplete (only common cases)

---

## 12. References

- **LangChain Concepts:** https://python.langchain.com/docs/concepts/
- **AutoGen Documentation:** https://microsoft.github.io/autogen/
- **CrewAI Documentation:** https://docs.crewai.com/

---

**README Version:** 1.0
**Last Updated:** 2026-03-01
**Owner:** Engineer 03
