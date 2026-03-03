# XKernal — The AI-Native Operating System

[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Rust](https://img.shields.io/badge/Rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![Python](https://img.shields.io/badge/Python-3.10%2B-blue.svg)](https://python.org)
[![Build](https://img.shields.io/badge/Build-Passing-brightgreen.svg)](#building-from-source)

**XKernal is an operating system built from the ground up for AI agents.** It treats AI agents as first-class citizens — the way Unix treats processes and Kubernetes treats containers. Agents get their own lifecycle management, capability-based security, inter-process communication, memory hierarchies, and process supervision.

This is not a wrapper around Linux. It is a complete 4-layer microkernel architecture with 21 Rust crates, a real process supervisor daemon, a Python SDK, and a 22-syscall kernel interface specification.

```
  ┌──────────────────────────────────────────────────────────────────────┐
  │  YOUR AI AGENTS                                                      │
  │  LangChain  ·  CrewAI  ·  AutoGen  ·  Semantic Kernel  ·  Custom    │
  ├──────────────────────────────────────────────────────────────────────┤
  │  L3  SDK & Tools          Python SDK  ·  CLI  ·  TypeScript  ·  C#   │
  ├──────────────────────────────────────────────────────────────────────┤
  │  L2  Runtime               Framework Adapters  ·  Agent Lifecycle    │
  ├──────────────────────────────────────────────────────────────────────┤
  │  L1  Services              GPU Accelerator  ·  Memory  ·  Telemetry  │
  ├──────────────────────────────────────────────────────────────────────┤
  │  L0  Microkernel           Capabilities  ·  Scheduler  ·  IPC        │
  └──────────────────────────────────────────────────────────────────────┘
```

---

## Why XKernal Exists

Today's AI agents run as scripts on top of general-purpose operating systems that have no concept of what an "agent" is. The OS doesn't know about agent capabilities, can't enforce security boundaries between agents, doesn't understand agent-to-agent communication, and has no native support for checkpointing cognitive state.

XKernal changes this. It provides:

- **Process supervision** — Agents are real OS processes with captured I/O, restart policies, and health monitoring
- **Capability-based security** — Unforgeable tokens control what each agent can do (seL4/OCap model)
- **Native IPC** — Typed message-passing channels between agents with delivery guarantees
- **Cognitive task scheduling** — 4-dimensional priority scheduling (criticality, deadline, efficiency, cost)
- **Framework translation** — LangChain, CrewAI, AutoGen, and Semantic Kernel concepts map to kernel primitives
- **Structured telemetry** — Cognitive Event Format (CEF) traces every agent action for observability
- **Declarative lifecycle** — Kubernetes-inspired unit files define agents, resources, health checks, and dependencies

---

## What XKernal Can Do Today

### Real Process Supervision

The `cs-daemon` is a real process supervisor. When you create an agent with an entrypoint, it:

1. **Spawns a real OS process** via `tokio::process::Command`
2. **Captures stdout/stderr** in real-time via piped I/O
3. **Monitors exit status** with 500ms polling
4. **Auto-restarts on failure** with exponential backoff (1s → 2s → 4s → ... → 32s)
5. **Enforces restart policies** — `never`, `on_failure`, or `always`
6. **Kills processes cleanly** on shutdown via SIGTERM

This means **any program that can be launched from a command line can be an XKernal agent** — Python scripts, Node.js servers, Docker containers, shell scripts, compiled binaries, or even VM management commands.

### Can AI Agents Manage VMs?

**Yes.** The supervisor spawns any command you give it as the entrypoint. An agent with `entrypoint: "VBoxManage startvm my-sandbox --type headless"` will launch a VirtualBox VM. An agent with `entrypoint: "docker run -d my-agent-image"` will start a Docker container. An agent with `entrypoint: "qemu-system-x86_64 -hda disk.img"` will boot a QEMU virtual machine.

The daemon doesn't care what the process does — it supervises it. Your AI agents can:

- **Launch and manage VMs** (VirtualBox, QEMU, Hyper-V, Firecracker)
- **Orchestrate Docker containers** with captured logs and restart policies
- **Spin up inference servers** (vLLM, Ollama, TGI) and monitor their health
- **Execute sandboxed code** in isolated environments
- **Run multi-agent pipelines** where each agent is a separate supervised process communicating via IPC

### 20-Endpoint REST API

The daemon exposes a Kubernetes-style REST API on port 7600:

| Resource | Endpoints | Operations |
|----------|-----------|------------|
| **Agents** | `/api/v1/agents` | Create, list, get, delete, signal, logs |
| **Channels** | `/api/v1/channels` | Create, list, send message, receive message |
| **Memory** | `/api/v1/memory` | Stats, allocate pages, free pages |
| **Tools** | `/api/v1/tools` | Register, list, unregister |
| **System** | `/api/v1/metrics`, `/api/v1/events` | Metrics dashboard, telemetry events |
| **Health** | `/healthz`, `/readyz` | Liveness and readiness probes |

### Python SDK — `pip install xkernal`

```python
import xkernal

@xkernal.tool(name="web_search", effect_class="read_only")
def web_search(query: str, max_results: int = 10) -> list[dict]:
    """Search the web for information."""
    # JSON schema auto-extracted from type hints
    return search_api(query, max_results)

@xkernal.agent(
    name="researcher",
    capabilities=["task", "tool", "channel"],
    tools=[web_search],
    restart_policy="on_failure",
)
async def researcher(ctx: xkernal.AgentContext):
    ctx.log.info("Starting research...")
    results = web_search("latest AI safety papers")
    await ctx.send("writer-agent-id", {"findings": results})
    ctx.log.info(f"Sent {len(results)} results to writer")

@xkernal.agent(
    name="writer",
    capabilities=["task", "channel"],
)
async def writer(ctx: xkernal.AgentContext):
    data = await ctx.receive("researcher-agent-id")
    report = generate_report(data["findings"])
    ctx.log.info(f"Generated {len(report)} word report")
    return report

# Register with daemon, execute, clean up
xkernal.run(researcher, writer)
```

The SDK provides:
- **`@agent` decorator** — Declares agents with capabilities, priority, restart policy
- **`@tool` decorator** — Declares tools with automatic JSON schema extraction from type hints
- **`xkernal.run()`** — Orchestrates the full lifecycle: health check → register tools → register agents → execute → heartbeat → cleanup
- **`AgentContext`** — Injected context with `send()`, `receive()`, `log`, and `get_status()`
- **`DaemonClient`** — Async HTTP client covering all 20 daemon endpoints
- **CLI** — `xkernal run module:agent`, `xkernal status`, `xkernal agents`, `xkernal logs <id>`
- **Structured JSON logging** — Agent stdout is captured as structured log lines by the daemon

---

## Architecture Deep Dive

### L0 — Capability-Based Microkernel

The kernel layer provides three foundational subsystems:

#### Capability Engine (`kernel/capability_engine`)

Implements an **seL4-inspired Object Capability (OCap) model**:

- **Unforgeable tokens** — `CapabilityToken(id: u64, generation: u32)` cannot be forged or guessed
- **Permission flags** — READ, WRITE, EXECUTE, DELEGATE, REVOKE as bitfields
- **Attenuation** — Delegated capabilities can only have equal or fewer permissions (intersection)
- **Revocation** — Immediate, delayed, or conditional revocation policies
- **Delegation chains** — Full provenance tracking with max-depth enforcement
- **Policy engine** — Mandatory policies evaluated in priority order; ALL must pass
- **Verification** — Proof-based verification with HMAC, Signature, ZeroKnowledge, and MerkleProof types

When an agent delegates a capability to another agent, the derived capability is **mathematically guaranteed** to be no more powerful than the original. Revocation cascades through the entire delegation chain.

#### Cognitive Task Lifecycle (`kernel/ct_lifecycle`)

Every unit of AI agent work is a **Cognitive Task (CT)** with an enforced state machine:

```
Spawn → Plan → Reason → Act → Reflect → Yield ─→ Plan (loop)
                                                └→ Complete (terminal)
                                                └→ Failed (terminal)
```

Each CT carries:
- **4-dimensional priority** — chain criticality, deadline pressure, resource efficiency, capability cost
- **Resource budget** — tokens, GPU-ms, wall-clock-ms, memory, tool calls
- **Capabilities** — subset of parent agent's capabilities (invariant enforced)
- **Dependencies** — DAG with cycle detection (DFS-based)
- **Watchdog** — deadline and iteration limits with warning signals

The **Priority Scheduler** is a real binary heap with O(log n) enqueue/dequeue, dynamic priority updates, and yield support.

The **Arena Allocator** provides first-fit block allocation with free-block coalescing to prevent fragmentation.

#### IPC, Signals & Exceptions (`kernel/ipc_signals_exceptions`)

- **Channels** — FIFO message queues with capacity limits, backpressure, and close semantics
- **Semantic Channels** — Configurable delivery guarantees (AtMostOnce, AtLeastOnce, ExactlyOnce), backpressure policies (Drop, Block, Reject), and protocol types (ByteStream, MessageBased, RequestResponse, PubSub)
- **8 Signal types** — SigTerminate (unblockable), SigDeadlineWarn, SigCheckpoint, SigBudgetWarn, SigContextLow, SigIpcFailed, SigPreempt, SigResume
- **7 Exception types** — DeadlineExceeded, InconsistentState, IpcFailure, CapabilityViolation, ToolCallFailed, ContextOverflow, ReasoningDiverged — each with severity-constrained recovery strategies
- **Checkpointing** — Immutable snapshots with version tracking, CRDT vector clocks for distributed reconciliation

### L1 — Runtime Services

#### GPU Accelerator (`services/gpu_accelerator`)

Abstraction layer for NVIDIA (CUDA) and AMD (ROCm) GPUs:

- **Device discovery** — Enumerate NVIDIA H100/H200/B200, AMD MI300X
- **VRAM management** — Page-based allocation, isolation per agent, coherency verification
- **Kernel launch queue** — FIFO with atomization and preemption points
- **Model registry** — Track loaded models, VRAM footprints, CT bindings
- **Async execution** — Event-based completion notification to resume CTs
- **Error recovery** — Fault codes, leak detection, recovery strategies
- **Telemetry** — Utilization, latency, throughput, thermal metrics

*Note: GPU service defines the correct CUDA/ROCm interfaces. Actual driver FFI bindings are planned for Phase 1.*

#### Semantic Memory (`services/semantic_memory`)

3-tier hierarchical memory designed for AI workloads:

| Tier | Speed | Location | Purpose |
|------|-------|----------|---------|
| **L1** | Microseconds | HBM / GPU-local | Working memory (per-CT context window) |
| **L2** | Milliseconds | Host DRAM | Episodic memory (per-agent) |
| **L3** | Seconds | NVMe / Persistent | Long-term memory (shared, replicated) |

- **L1 Allocator** — Fully functional page allocator (4KB granule) with ref counting, pinning, resize
- **Capability-based access** — MemoryCapabilityFlags (ALLOCATE, READ, WRITE, EVICT, MIGRATE, QUERY, SNAPSHOT)
- **3 isolation levels** — PerAgent, PerCrew, SharedReadOnly
- **Vector index** — Distance metrics (Cosine, L2, Manhattan) and quantization (int8, uint8) for semantic search

#### Tool Registry & Telemetry (`services/tool_registry_telemetry`)

- **Tool bindings** — Name, description, input/output schemas, effect class, sandbox config
- **4 effect classes** — ReadOnly, WriteReversible, WriteCompensable, WriteIrreversible
- **Sandbox configuration** — Network policy, filesystem policy, syscall limits, memory limits
- **Cognitive Event Format (CEF)** — 14 event types including ToolCallRequested, MemoryAccess, PhaseTransition, CheckpointCreated, ExceptionRaised
- **Multiple encodings** — JSON and Binary CEF encoders with schema versioning
- **Cost attribution** — Token counting, GPU time tracking, TPC cost calculation
- **Retention policies** — Hot/Warm/Cold/Archive tiers with TTL and redaction rules

### L2 — Runtime & Adapters

#### Framework Adapters (`runtime/framework_adapters`)

Translation layer mapping external AI framework concepts to kernel primitives:

| Framework | Agent Concept | Task Concept | Memory Concept | Tool Concept |
|-----------|--------------|--------------|----------------|--------------|
| **LangChain** | Agent | Chain/Step | ConversationBuffer | Tool |
| **CrewAI** | Agent/Role | Task | SharedMemory | Tool |
| **AutoGen** | AssistantAgent | Message | ChatHistory | FunctionCall |
| **Semantic Kernel** | Kernel | Plan/Step | SemanticMemory | Plugin/Function |
| **Custom** | (user-defined) | (user-defined) | (user-defined) | (user-defined) |

- **Concept mapping matrix** — Bidirectional mapping with fidelity levels (Full, Partial, Limited, NotSupported)
- **Chain-to-DAG translation** — Converts sequential chains to parallel task graphs
- **SK advanced adapter** — Plugin/skill mapping, planner translation, memory tier mapping
- **Translation caching** — LRU cache for translated artifacts

#### Agent Lifecycle (`runtime/semantic_fs_agent_lifecycle`)

Kubernetes-inspired declarative agent management:

**Unit Files** (TOML format):
```toml
[Agent]
name = "researcher"
framework = "langchain"
model = "claude-3-opus"
entrypoint = "python researcher.py"

[Resources]
cpu_millicores = 2000
memory_mb = 4096
max_tokens = 100000
gpu_time_ms = 60000

[HealthCheck]
readiness_probe = { type = "http", path = "/ready", port = 8080 }
liveness_probe = { type = "exec", command = "python -c 'import agent; agent.ping()'" }

[Restart]
policy = "on_failure"
max_retries = 5
backoff_base_ms = 1000
backoff_max_ms = 60000

[Dependencies]
requires = ["embedding-service", "vector-store"]
```

- **Unit file parser** — Full TOML parsing with serde
- **Validation engine** — 8 rule types (required fields, health check syntax, capability existence, etc.)
- **Health probes** — HTTP, TCP, CSCI syscall, and exec-based health checks
- **Restart policies** — Never, OnFailure, Always with exponential backoff
- **Dependency graph** — Topological sort with cycle detection for startup ordering
- **Parallel start groups** — Agents without dependencies start concurrently
- **Semantic filesystem** — Query agents by attributes, tags, crew membership, resource usage

### L3 — SDK & Daemon

#### CSCI — Cognitive Substrate Syscall Interface (`sdk/csci`)

The formalized kernel ABI — **22 syscalls across 9 families**:

| Family | Syscalls | Purpose |
|--------|----------|---------|
| **Task** | `ct_spawn`, `ct_yield`, `ct_checkpoint`, `ct_resume` | Cognitive task lifecycle |
| **Memory** | `mem_alloc`, `mem_read`, `mem_write`, `mem_mount` | Semantic memory operations |
| **Tool** | `tool_bind`, `tool_invoke` | Tool registry and execution |
| **Channel** | `chan_open`, `chan_send`, `chan_recv` | Inter-agent communication |
| **Capability** | `cap_grant`, `cap_delegate`, `cap_revoke` | Security model |
| **Signals** | `sig_register`, `exc_register` | Signal and exception handlers |
| **Crew** | `crew_create`, `crew_join`, `crew_leave`, `crew_query` | Agent crew coordination |
| **Telemetry** | `trace_emit`, `trace_query` | CEF event logging |

**20 error codes** following POSIX conventions: CS_EPERM(1), CS_ENOENT(2), CS_ENOMEM(12), CS_EBUSY(16), CS_EEXIST(17), CS_EINVAL(22), CS_ETIMEOUT(110), plus 11 CSCI-specific codes for budget exhaustion, cycles, closed channels, sandbox violations, and policy violations.

#### cs-daemon — The Control Plane

The daemon integrates real kernel objects into a supervised runtime:

- **Kernel objects in state** — Real `TaskStateMachine`, `PriorityScheduler`, `PolicyEngine`, `Channel`, `ToolRegistry` instances from L0/L1
- **Process supervisor** — `tokio::process::Command` with piped I/O, exit monitoring, restart logic
- **Telemetry recording** — All operations emit events (17 event types)
- **Axum HTTP server** — Async, non-blocking API on configurable host/port

#### CLI Tools

| Tool | Purpose | Status |
|------|---------|--------|
| **cs-ctl** | Full HTTP client for daemon management | Working |
| **cs-capgraph** | Capability delegation chain visualization (Graphviz DOT) | Structure defined |
| **cs-profile** | CPU/memory profiling with flamegraph generation | Data structures defined |
| **cs-replay** | Deterministic replay debugging from checkpoints | Configuration defined |
| **cs-top** | Real-time system monitor (like Unix `top`) | Data structures defined |
| **cs-trace** | Distributed syscall tracing | Trace format defined |

---

## Project Structure

```
XKernal/
├── kernel/                                # L0: Capability-Based Microkernel
│   ├── capability_engine/                 #   Unforgeable tokens, delegation, policy engine
│   ├── ct_lifecycle/                      #   Task state machine, priority scheduler, DAG
│   └── ipc_signals_exceptions/            #   Channels, signals, exceptions, checkpoints
│
├── services/                              # L1: Runtime Services
│   ├── gpu_accelerator/                   #   CUDA/ROCm abstraction, VRAM, model registry
│   ├── semantic_memory/                   #   3-tier memory (L1/L2/L3), page allocator
│   └── tool_registry_telemetry/           #   Tool bindings, CEF telemetry, cost tracking
│
├── runtime/                               # L2: Runtime & Translation
│   ├── framework_adapters/                #   LangChain, CrewAI, AutoGen, SK adapters
│   └── semantic_fs_agent_lifecycle/       #   Unit files, health checks, restart policies
│
├── daemon/                                # Control Plane
│   └── cs-daemon/                         #   REST API + process supervisor (Axum + Tokio)
│
├── sdk/                                   # L3: Developer SDKs
│   ├── csci/                              #   CSCI syscall specification (22 syscalls, Rust)
│   ├── python-sdk/                        #   Python SDK — pip install xkernal
│   ├── ts-sdk/                            #   TypeScript SDK (types + client structure)
│   ├── cs-sdk/                            #   C# / .NET SDK (interfaces + async signatures)
│   └── tools/                             #   CLI tools (cs-ctl, cs-capgraph, cs-profile, ...)
│
└── examples/
    └── sdk-demo/                          #   Multi-agent IPC demo (producer/consumer)
```

**21 Rust crates** · **~50,000 lines of Rust** · **12 Python modules** · **60 Python tests** · **100% safe Rust (no unsafe)**

---

## Quick Start

### Prerequisites

- **Rust 1.70+** with Cargo
- **Python 3.10+** (for Python SDK)

### 1. Build the Kernel and Daemon

```bash
git clone https://github.com/JosephBerm/XKernel.git
cd XKernel

# Build everything
cargo build

# Run kernel tests
cargo test
```

### 2. Start the Daemon

```bash
# Default: http://127.0.0.1:7600
cargo run -p cs-daemon

# Custom port and logging
CS_PORT=8080 CS_LOG=debug cargo run -p cs-daemon
```

### 3. Install the Python SDK

```bash
cd sdk/python-sdk
pip install -e ".[dev]"
```

### 4. Run Your First Agent

```python
import xkernal

@xkernal.agent(name="hello", capabilities=["task"])
async def hello(ctx):
    ctx.log.info("Hello from XKernal!")
    return "Hello, World!"

xkernal.run(hello)
```

### 5. Use the CLI

```bash
# Check daemon status
xkernal status

# List running agents
xkernal agents

# View agent logs
xkernal logs <agent-id>

# Or use cs-ctl for full control
cargo run -p cs-ctl -- status
cargo run -p cs-ctl -- agent list
cargo run -p cs-ctl -- channel create --from agent-1 --to agent-2
```

---

## Use Cases

### Multi-Agent Research Pipelines

Deploy a team of specialized agents — researcher, analyst, writer — that communicate via IPC channels, each supervised with restart policies and structured logging.

### Sandboxed Code Execution

Launch agents that execute untrusted code in isolated processes. The daemon captures all output, enforces timeouts via restart policies, and kills runaway processes.

### AI-Managed Infrastructure

Agents can manage VMs, containers, and cloud resources through their entrypoints. The daemon supervises these management processes with health monitoring and auto-restart.

### LLM Inference Server Orchestration

Use agents to manage vLLM, Ollama, or TGI inference servers. The daemon monitors process health, captures logs, and automatically restarts crashed servers with exponential backoff.

### Framework-Agnostic Agent Coordination

Run LangChain agents alongside CrewAI crews and AutoGen groups. The framework adapter layer translates each framework's concepts to common kernel primitives for unified scheduling and IPC.

### Auditable AI Operations

Every agent action is recorded as a CEF telemetry event. The tool registry tracks which tools each agent can access and what effect class they carry. Full audit trail from creation to termination.

---

## Implementation Status

### What's Real and Working

| Component | Status | Details |
|-----------|--------|---------|
| **Process Supervisor** | **Production** | Real OS process spawning, I/O capture, restart logic |
| **REST API** | **Production** | All 20 endpoints verified end-to-end |
| **Python SDK** | **Production** | 60 tests passing, decorators, runtime, CLI |
| **Task State Machine** | **Production** | Enforced phase transitions with history |
| **Priority Scheduler** | **Production** | Binary heap, O(log n), dynamic priorities |
| **IPC Channels** | **Production** | Real message queuing with kernel Channel objects |
| **Capability Model** | **Production** | Tokens, attenuation, revocation, delegation chains |
| **Policy Engine** | **Production** | Mandatory policies evaluated in priority order |
| **Checkpointing** | **Production** | Immutable snapshots with versioning |
| **Signal Delivery** | **Production** | Priority-ordered dispatch with unblockable signals |
| **Tool Registry** | **Production** | Real kernel ToolRegistry with binding management |
| **CEF Telemetry** | **Production** | 14 event types, JSON + Binary encoding |
| **Unit File System** | **Production** | TOML parsing, validation, health checks, restart policies |
| **L1 Memory Allocator** | **Production** | Page allocator with ref counting and pinning |
| **Dependency DAG** | **Production** | Cycle detection, topological sort |
| **cs-ctl CLI** | **Production** | Full HTTP client for all daemon operations |

### What's Architected (Phase 1 Targets)

| Component | Status | Details |
|-----------|--------|---------|
| GPU Driver Bindings | Designed | CUDA/ROCm trait interfaces defined, FFI pending |
| L2/L3 Memory Tiers | Designed | Architecture specified, backends pending |
| Framework Parsing | Designed | Adapter interfaces defined, library integration pending |
| Distributed IPC | Designed | Protocol specified, network transport pending |
| Persistent State | Designed | Checkpoint format defined, disk I/O pending |
| Authentication | Planned | Daemon API currently open |
| Multi-Node | Planned | Single-daemon architecture, federation pending |

---

## Engineering Principles

1. **Agents are processes, not threads** — Real OS isolation, real PIDs, real I/O capture
2. **Capabilities, not ACLs** — Unforgeable tokens that can only be attenuated, never amplified
3. **Lazy registration** — Decorators capture metadata; no network calls until `run()`
4. **Async-first** — Tokio runtime for the daemon, asyncio for the Python SDK, sync functions auto-wrapped
5. **Single dependency** — Python SDK depends only on `httpx`; no Pydantic, no heavy frameworks
6. **Kernel objects in state** — The daemon holds real L0/L1 kernel instances, not mocks
7. **No unsafe** — Every crate uses `#![forbid(unsafe_code)]`
8. **Structured observability** — JSON-line stdout logging, CEF telemetry events, queryable event store

---

## Contributing

We welcome contributions. The codebase is well-structured with clear layer boundaries:

- **L0 kernel** — Core algorithms, data structures, state machines
- **L1 services** — GPU, memory, telemetry infrastructure
- **L2 runtime** — Framework adapters, agent lifecycle
- **L3 SDK** — Developer experience, CLI tools, language bindings

### Development Setup

```bash
# Build and test everything
cargo build && cargo test

# Python SDK development
cd sdk/python-sdk
pip install -e ".[dev]"
pytest tests/ -v
```

---

## License

Apache License 2.0. See [LICENSE](./LICENSE).

## Citation

```bibtex
@software{xkernal2026,
  title     = {XKernal: AI-Native Cognitive Substrate Operating System},
  author    = {Bermudez, Joseph and Contributors},
  year      = {2026},
  url       = {https://github.com/JosephBerm/XKernel}
}
```
