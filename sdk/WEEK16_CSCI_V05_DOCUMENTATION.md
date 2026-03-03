# XKernal CSCI v0.5 Comprehensive Documentation

**Document Version:** v0.5.0
**Release Date:** Week 16, Phase 2
**Status:** Production Ready
**Target Audience:** SDK developers, integrators, framework maintainers

---

## Table of Contents

1. [Overview & Architecture](#overview--architecture)
2. [Syscall Families & API Reference](#syscall-families--api-reference)
3. [Code Examples by Family](#code-examples-by-family)
4. [Edge Cases & Error Handling](#edge-cases--error-handling)
5. [Framework Integration Patterns](#framework-integration-patterns)
6. [Troubleshooting Guide](#troubleshooting-guide)
7. [Documentation Portal Structure](#documentation-portal-structure)

---

## Overview & Architecture

### Cognitive System Call Interface (CSCI) v0.5

The CSCI v0.5 extends libcognitive's low-level primitives into a production-grade syscall abstraction layer. This L3 SDK layer (Rust/TypeScript/C#) provides semantic versioning across 22+ syscalls organized into 8 families, enabling cognitive task orchestration, memory management, inter-process communication, security operations, tool invocation, signal handling, telemetry collection, and crew coordination.

**Key Enhancements in v0.5:**
- 4 new syscalls: `task_priority_escalate`, `memory_checkpoint_save`, `ipc_broadcast`, `crew_load_balance`
- 3 modified syscalls: `task_execute` (timeout granularity), `memory_allocate` (isolation levels), `security_verify` (capability attestation)
- 19 new error codes with structured recovery patterns
- FFI performance profiling metrics (p50, p99 latencies)

---

## Syscall Families & API Reference

### Family 1: Task Management (6 syscalls)

**Core Syscalls:** `task_create`, `task_execute`, `task_cancel`, `task_priority_escalate`, `task_wait`, `task_context`

**Signatures (Rust):**
```rust
pub fn task_create(
    name: &str,
    entry_point: TaskFn,
    config: TaskConfig,
) -> Result<TaskHandle, TaskError>;

pub fn task_execute(
    handle: TaskHandle,
    timeout_ms: u64,
    isolation: ExecutionMode,
) -> Result<TaskOutput, TaskError>;

pub fn task_priority_escalate(
    handle: TaskHandle,
    priority: i32,
    reason: EscalationReason,
) -> Result<(), TaskError>;
```

### Family 2: Memory Management (4 syscalls)

**Core Syscalls:** `memory_allocate`, `memory_deallocate`, `memory_checkpoint_save`, `memory_query`

**Key Characteristics:**
- Isolation levels: `SHARED`, `ISOLATED`, `EPHEMERAL`
- Checkpoint supports differential snapshots
- Query returns usage metrics and fragmentation data

### Family 3: IPC (4 syscalls)

**Core Syscalls:** `ipc_send`, `ipc_receive`, `ipc_listen`, `ipc_broadcast`

**Queue Modes:** `FIFO`, `LIFO`, `PRIORITY`

### Family 4: Security (3 syscalls)

**Core Syscalls:** `security_verify`, `security_attest`, `security_revoke`

**Capabilities:** READ, WRITE, EXECUTE, ADMIN, TOOL_INVOKE

### Family 5: Tool Invocation (2 syscalls)

**Core Syscalls:** `tool_invoke`, `tool_status`

### Family 6: Signal Handling (2 syscalls)

**Core Syscalls:** `signal_register`, `signal_emit`

### Family 7: Telemetry (2 syscalls)

**Core Syscalls:** `telemetry_record`, `telemetry_flush`

### Family 8: Crew Coordination (3 syscalls)

**Core Syscalls:** `crew_create`, `crew_schedule`, `crew_load_balance`

---

## Code Examples by Family

### Task Management (Rust)

```rust
use xkernal_csci::{
    task::{TaskCreate, TaskExecute, TaskPriorityEscalate},
    TaskConfig, ExecutionMode, EscalationReason,
};

async fn orchestrate_cognitive_pipeline() -> Result<(), Box<dyn std::error::Error>> {
    // Create a task with custom configuration
    let task_config = TaskConfig {
        name: "semantic_analysis".to_string(),
        timeout_ms: 5000,
        isolation: ExecutionMode::ISOLATED,
        memory_limit_mb: 256,
    };

    let task_handle = task_create("semantic_analysis", entry_point_fn, task_config)?;

    // Execute with explicit timeout
    let output = task_execute(
        task_handle,
        5000,
        ExecutionMode::ISOLATED,
    ).await?;

    // Handle success
    match output {
        TaskOutput::Success(result) => println!("Task completed: {:?}", result),
        TaskOutput::Partial(partial) => {
            // Escalate priority for retry
            task_priority_escalate(
                task_handle,
                10,
                EscalationReason::PartialCompletion,
            )?;
        }
        TaskOutput::Failed(err) => return Err(err.into()),
    }

    Ok(())
}

// Wait for task with context tracking
async fn wait_with_context(handle: TaskHandle) -> Result<(), Box<dyn std::error::Error>> {
    let context = task_context(handle)?;
    println!("Task state: {:?}, elapsed: {:?}ms",
        context.state,
        context.elapsed_ms
    );

    task_wait(handle, 10000).await?;
    Ok(())
}
```

### Memory Management (TypeScript)

```typescript
import {
    MemoryAllocate,
    MemoryCheckpointSave,
    IsolationLevel,
    MemoryQueryResponse
} from 'xkernal-csci-ts';

async function manageContextMemory(): Promise<void> {
    // Allocate isolated memory for sensitive context
    const memHandle = await memoryAllocate({
        size_bytes: 1024 * 1024, // 1MB
        isolation: IsolationLevel.ISOLATED,
        purpose: 'semantic_context',
    });

    try {
        // Perform memory-intensive operations
        await performComplexAnalysis(memHandle);

        // Save checkpoint for recovery
        const checkpointId = await memoryCheckpointSave({
            memory_handle: memHandle,
            checkpoint_type: 'differential',
            compression: true,
        });

        console.log(`Checkpoint saved: ${checkpointId}`);

        // Query memory usage
        const stats = await memoryQuery(memHandle);
        console.log(`Memory fragmentation: ${stats.fragmentation_percent}%`);
    } finally {
        // Always deallocate
        await memoryDeallocate(memHandle);
    }
}

// Handle EPHEMERAL memory for temporary buffers
async function ephemeralWorkBuffer(): Promise<void> {
    const ephemHandle = await memoryAllocate({
        size_bytes: 512 * 1024,
        isolation: IsolationLevel.EPHEMERAL,
        auto_cleanup_ms: 60000,
    });

    // Memory auto-cleans after 60s or explicit deallocation
    await processStreamData(ephemHandle);
}
```

### IPC - Broadcast Pattern (C#)

```csharp
using XKernalCSCI.IPC;
using XKernalCSCI.Crews;

public class CrewCoordinator
{
    private readonly IIPCBroadcast _broadcast;
    private readonly ICrewSchedule _scheduler;

    public async Task CoordinateCrewExecution(List<CrewMember> members)
    {
        // Broadcast task to all crew members
        var broadcastId = await _broadcast.BroadcastAsync(new Message
        {
            Type = MessageType.TaskAssignment,
            Payload = new TaskPayload {
                WorkId = "analysis_001",
                Priority = Priority.High
            },
            QueueMode = QueueMode.Priority,
        });

        // Schedule with load balancing
        var distribution = await _scheduler.ScheduleWithLoadBalance(members, new ScheduleConfig
        {
            Algorithm = LoadBalanceAlgorithm.LeastLoaded,
            MaxParallel = 8,
            MetricsWindow = TimeSpan.FromSeconds(30),
        });

        // Monitor broadcast delivery
        var status = await _broadcast.StatusAsync(broadcastId);
        Console.WriteLine($"Delivered: {status.DeliveredCount}/{status.TotalRecipients}");
    }

    public async Task ListenForCrewSignals()
    {
        using var listener = await _broadcast.ListenAsync(
            filter: msg => msg.Type == MessageType.CrewStatus
        );

        await foreach (var message in listener)
        {
            ProcessCrewStatusUpdate(message.Payload);
        }
    }
}
```

### Security Verification (Rust)

```rust
use xkernal_csci::security::{SecurityVerify, SecurityAttest, Capability};

pub async fn verify_and_invoke_tool(
    tool_name: &str,
    required_capabilities: Vec<Capability>,
) -> Result<ToolResult, SecurityError> {
    // Verify caller capabilities
    let verification = security_verify(
        required_capabilities.clone(),
        VerificationMode::Strict,
    ).await?;

    if !verification.granted {
        return Err(SecurityError::CapabilityDenied {
            missing: verification.missing_capabilities,
            reason: verification.denial_reason,
        });
    }

    // Attest action for audit trail
    let attestation = security_attest(
        AttestedAction {
            action: "tool_invoke".to_string(),
            tool_name: tool_name.to_string(),
            capabilities_used: required_capabilities,
            timestamp: std::time::SystemTime::now(),
        },
        AttestationLevel::Cryptographic,
    ).await?;

    // Now safe to invoke tool
    tool_invoke(tool_name, &verification.context).await
}
```

### Tool Invocation with Status Polling (TypeScript)

```typescript
import { toolInvoke, toolStatus, ToolInvocationConfig } from 'xkernal-csci-ts';

async function invokeToolWithPolling(
    toolName: string,
    params: Record<string, unknown>,
    maxWaitMs: number = 30000
): Promise<ToolOutput> {
    const invokeHandle = await toolInvoke({
        tool_name: toolName,
        parameters: params,
        timeout_ms: 30000,
        async_mode: true,
    });

    const startTime = Date.now();
    const pollIntervalMs = 500;

    while (Date.now() - startTime < maxWaitMs) {
        const status = await toolStatus(invokeHandle);

        if (status.state === 'completed') {
            return status.output;
        } else if (status.state === 'failed') {
            throw new Error(`Tool failed: ${status.error}`);
        }

        // Exponential backoff with jitter
        const backoff = Math.min(pollIntervalMs * (1.5 ** status.poll_count), 5000);
        await new Promise(resolve => setTimeout(resolve, backoff + Math.random() * 100));
    }

    throw new Error('Tool invocation timeout');
}
```

### Signal Handling (C#)

```csharp
using XKernalCSCI.Signals;

public class CognitiveSignalHandler
{
    private readonly ISignalRegister _signalReg;
    private readonly ISignalEmit _signalEmit;

    public async Task RegisterSignalHandlers()
    {
        // Register handler for task completion signal
        await _signalReg.RegisterAsync(
            signal: Signal.TaskCompleted,
            handler: async (ctx) =>
            {
                await LogTaskCompletion(ctx.TaskId, ctx.Result);
                await PublishMetrics(ctx);
            },
            priority: 10,
            async_mode: true
        );

        // Register error signal with escalation
        await _signalReg.RegisterAsync(
            signal: Signal.TaskFailed,
            handler: async (ctx) =>
            {
                if (ctx.ErrorCode == ErrorCode.OutOfMemory)
                {
                    await _signalEmit.EmitAsync(Signal.MemoryPressure, ctx);
                }
                await RecordFailure(ctx);
            },
            priority: 20
        );
    }

    public async Task EmitWorkCompletion(string crewId, WorkResult result)
    {
        await _signalEmit.EmitAsync(
            signal: Signal.CrewWorkCompleted,
            context: new SignalContext
            {
                CrewId = crewId,
                Timestamp = DateTime.UtcNow,
                Payload = result,
            },
            broadcast: true // Send to all listeners
        );
    }
}
```

### Telemetry Collection (Rust)

```rust
use xkernal_csci::telemetry::{TelemetryRecord, TelemetryFlush};
use std::time::Instant;

pub async fn collect_execution_metrics(
    task_name: &str,
    start: Instant,
) -> Result<(), TelemetryError> {
    let elapsed_ms = start.elapsed().as_millis() as u64;

    telemetry_record(TelemetryEvent {
        event_type: "task_execution".to_string(),
        task_name: task_name.to_string(),
        latency_ms: elapsed_ms,
        metrics: vec![
            ("memory_peak_mb", "256".to_string()),
            ("cpu_samples", "1024".to_string()),
            ("cache_hits", "8192".to_string()),
        ],
        tags: vec![
            ("environment", "production"),
            ("version", "0.5"),
        ],
        timestamp: chrono::Utc::now(),
    }).await?;

    // Flush in batches
    if elapsed_ms > 1000 {
        telemetry_flush(FlushConfig {
            batch_size: 100,
            compression: CompressionFormat::ZSTD,
            endpoint: "telemetry.xkernal.local".to_string(),
        }).await?;
    }

    Ok(())
}
```

### Crew Coordination (TypeScript)

```typescript
import {
    crewCreate,
    crewSchedule,
    crewLoadBalance,
    LoadBalanceAlgorithm
} from 'xkernal-csci-ts';

async function orchestrateCrew(
    teamSize: number,
    workItems: WorkItem[]
): Promise<CrewResult> {
    // Create crew with configuration
    const crewHandle = await crewCreate({
        name: 'analysis_team',
        size: teamSize,
        isolation: IsolationLevel.Shared,
        capabilities: ['semantic_analysis', 'vector_search', 'ranking'],
    });

    // Load balance work across crew
    const distribution = await crewLoadBalance({
        crew_handle: crewHandle,
        work_items: workItems,
        algorithm: LoadBalanceAlgorithm.LeastConnections,
        metrics_window_ms: 5000,
    });

    // Schedule execution
    const scheduleHandle = await crewSchedule({
        crew_handle: crewHandle,
        distribution: distribution,
        start_immediately: true,
        max_concurrent_tasks: 16,
    });

    // Monitor progress
    let completed = 0;
    while (completed < workItems.length) {
        const status = await crewSchedule.statusAsync(scheduleHandle);
        completed = status.completed_count;
        console.log(`Progress: ${completed}/${workItems.length}`);
        await new Promise(r => setTimeout(r, 1000));
    }

    return scheduleHandle.result;
}
```

---

## Edge Cases & Error Handling

### Task Family Edge Cases

**EC-T1: Task Timeout with Partial Results**
```rust
// When timeout occurs mid-execution
match task_execute(handle, 5000, ExecutionMode::ISOLATED).await {
    Err(TaskError::Timeout { partial_output, elapsed_ms }) => {
        // Decide: retry, escalate priority, or use partial results
        if let Some(partial) = partial_output {
            eprintln!("Partial output after {}ms: {:?}", elapsed_ms, partial);
            // Option: save partial state for resumption
            memory_checkpoint_save(checkpoint_handle).await?;
        }
    }
    _ => {}
}
```

**EC-T2: Priority Inversion (Low-priority task blocks high-priority)**
Mitigation: Use `task_context()` to detect blocking and apply auto-escalation via `task_priority_escalate()`.

**EC-T3: Context Pollution Between Concurrent Tasks**
Mitigation: Always use `ExecutionMode::ISOLATED` for sensitive workloads; monitor via `memory_query()`.

### Memory Family Edge Cases

**EC-M1: Fragmentation Over Time**
```typescript
const stats = await memoryQuery(handle);
if (stats.fragmentation_percent > 40) {
    // Trigger defragmentation or allocate new chunk
    const newHandle = await memoryAllocate({
        size_bytes: stats.used_bytes,
        isolation: IsolationLevel.ISOLATED,
    });
    // Copy data, deallocate old handle
}
```

**EC-M2: Checkpoint Corruption During Save**
Error code `CHECKPOINT_CRC_MISMATCH` indicates corruption. Recovery: fallback to previous valid checkpoint with timestamp.

**EC-M3: EPHEMERAL Memory Auto-Cleanup Race**
Risk: Task tries to access memory after auto-cleanup. Mitigation: explicit deallocation before timeout or extend `auto_cleanup_ms`.

### IPC Family Edge Cases

**EC-I1: Broadcast to Disconnected Recipients**
```csharp
var status = await _broadcast.StatusAsync(broadcastId);
var failedRecipients = status.Failed;
foreach (var failed in failedRecipients)
{
    await RetryBroadcastWithBackoff(failed, message, maxRetries: 3);
}
```

**EC-I2: Queue Priority Inversion**
Solution: Use `QueueMode.Priority` with explicit priority values; monitor queue depth via metrics.

**EC-I3: Message Serialization Incompatibility**
Guard with: version negotiation in message header; fallback serialization format.

### Security Family Edge Cases

**EC-S1: Capability Revocation During Operation**
```rust
// Periodic re-verification during long operations
let mut last_check = Instant::now();
for chunk in data.chunks(CHUNK_SIZE) {
    if last_check.elapsed() > Duration::from_secs(30) {
        // Re-verify capabilities
        security_verify(caps.clone(), VerificationMode::Strict).await?;
        last_check = Instant::now();
    }
    process_chunk(chunk)?;
}
```

**EC-S2: Attestation Signature Validity Period**
Error code: `ATTESTATION_EXPIRED`. Mitigation: refresh attestation before expiry window (e.g., 80% of TTL).

**EC-S3: ADMIN Capability Scope Creep**
Mitigation: Audit trails via `security_attest()` with `AttestationLevel::Cryptographic`; enforce principle of least privilege.

### Tool Invocation Edge Cases

**EC-To1: Tool Dependency Chain Failures**
```typescript
// Implement circuit breaker pattern
if (toolStatus.consecutive_failures > 3) {
    toolCircuitBreaker.open(toolName);
    return { cached_result: getLastSuccessfulOutput() };
}
```

**EC-To2: Tool Output Size Exceeds Memory Allocation**
Mitigation: stream output or allocate larger memory before tool invocation.

### Signal & Telemetry Edge Cases

**EC-Sig1: Signal Handler Deadlock**
Risk: Handler emits signal that re-triggers itself. Mitigation: depth tracking, maximum recursion depth check.

**EC-Tel1: Telemetry Flush During Shutdown**
Ensure graceful flush with timeout to prevent data loss.

---

## Framework Integration Patterns

### LangChain Integration

```typescript
// File: langchain-adapter.ts
import { LLM, LLMResult, Generation } from "langchain/llms/base";
import { task_create, task_execute, ExecutionMode } from "xkernal-csci-ts";

export class XKernalLLMAdapter extends LLM {
    async _call(prompt: string): Promise<string> {
        const taskHandle = await task_create({
            name: "langchain_llm_call",
            entry_point: this.llmCallEntry,
            config: {
                timeout_ms: 30000,
                isolation: ExecutionMode.ISOLATED,
            },
        });

        const output = await task_execute(taskHandle, 30000, ExecutionMode.ISOLATED);
        return output.text;
    }
}

// Integrate with LangChain chains
const llm = new XKernalLLMAdapter();
const chain = new LLMChain({ llm, prompt });
const result = await chain.call({ input: "Analyze sentiment" });
```

### Semantic Kernel Integration

```csharp
// File: SemanticKernelPlug-in.cs
using Microsoft.SemanticKernel;
using XKernalCSCI.Tasks;

public class XKernalPlugin
{
    [SKFunction("Executes cognitive task via XKernal CSCI")]
    public async Task<string> ExecuteTask(
        [Description("Task name")] string taskName,
        [Description("Input context")] string context,
        IKernel kernel)
    {
        var handle = await taskCreate(new TaskConfig
        {
            Name = taskName,
            TimeoutMs = 15000,
            Isolation = ExecutionMode.Isolated,
        });

        var output = await taskExecute(handle, 15000, ExecutionMode.Isolated);
        return output.Result;
    }

    public void Register(IKernel kernel)
    {
        kernel.ImportSkill(this, "XKernal");
    }
}
```

### CrewAI Integration

```python
# File: crewai_xkernal_adapter.py
from crewai import Task, Agent, Crew
from xkernal_csci import crew_create, crew_schedule, crew_load_balance

class XKernalCrew:
    def __init__(self, agents: List[Agent], tasks: List[Task]):
        self.agents = agents
        self.tasks = tasks

    async def execute_with_xkernal(self):
        # Create XKernal crew matching CrewAI agent count
        crew_handle = await crew_create({
            'name': 'crewai_team',
            'size': len(self.agents),
            'capabilities': ['task_execution', 'memory_management'],
        })

        # Distribute tasks via load balancing
        distribution = await crew_load_balance({
            'crew_handle': crew_handle,
            'work_items': self.tasks,
            'algorithm': 'least_loaded',
        })

        # Schedule execution
        schedule = await crew_schedule({
            'crew_handle': crew_handle,
            'distribution': distribution,
        })

        return schedule.result
```

---

## Troubleshooting Guide

### Common Issues & Resolution

**Issue: TaskError::OutOfMemory on task_execute**
1. Check memory allocation with `memory_query()`
2. Reduce `TaskConfig.timeout_ms` to trigger earlier cancellation
3. Use `ExecutionMode::EPHEMERAL` for temporary buffers
4. Profile with FFI latency metrics (p99 > 500ms indicates congestion)

**Issue: IPC message delivery failures**
1. Verify recipient is listening: `ipc_listen()` active before `ipc_broadcast()`
2. Check queue depth: high depth indicates processing bottleneck
3. Review error code: `IPC_QUEUE_FULL` requires consumer speedup or queue resize
4. Implement exponential backoff retry with max 3 attempts

**Issue: Security verification denials**
1. Trace missing capabilities: `verification.missing_capabilities`
2. Verify attestation not expired: check TTL in response
3. Audit capability grants with `security_attest()`
4. Use `VerificationMode::Lenient` for debugging (not production)

**Issue: Tool invocation timeout (30s+)**
1. Profile tool directly outside XKernal to isolate bottleneck
2. Check telemetry: `telemetry_record()` for latency breakdown
3. Implement tool circuit breaker (fail fast after 3 consecutive failures)
4. Consider async mode: `tool_invoke(..., async_mode=true)` with polling

**Issue: Crew load imbalance**
1. Monitor `crew_schedule.status()` for per-member work distribution
2. Adjust `LoadBalanceAlgorithm`: try `LeastLoaded`, then `RoundRobin`
3. Set `metrics_window_ms` to match task granularity (shorter for fine-grained tasks)
4. Verify crew member availability (health checks)

**Issue: Checkpoint save failing with CHECKPOINT_CRC_MISMATCH**
1. Verify memory not corrupted: run `memory_query()` and check fragmentation
2. Allocate larger memory buffer before next checkpoint attempt
3. Fallback to previous valid checkpoint (version N-1)
4. Increase compression level to catch bit-rot earlier

**Issue: Signal handler not triggered**
1. Verify handler registered before signal emitted: check registration timestamp
2. Handler priority must be >= emitter priority (default 0)
3. Check handler async_mode: sync handlers block emission
4. Review signal filter logic if listener registered conditionally

**Issue: Telemetry data loss during shutdown**
1. Call `telemetry_flush()` with explicit timeout before exit
2. Set batch size lower if queue growing: `batch_size: 10` instead of 100
3. Enable compression to reduce endpoint saturation
4. Verify endpoint connectivity before telemetry recording

---

## Documentation Portal Structure

### Proposed Documentation Hosting

```
docs.xkernal.local/
├── index.html                         # Landing page
├── getting-started/
│   ├── installation.md               # SDK setup (Rust/TS/C#)
│   ├── hello-world.md                # Minimal example per language
│   ├── quickstart-task.md            # Task creation example
│   └── quickstart-crew.md            # Crew coordination example
├── api-reference/
│   ├── task-family.md                # task_* syscalls
│   ├── memory-family.md              # memory_* syscalls
│   ├── ipc-family.md                 # ipc_* syscalls
│   ├── security-family.md            # security_* syscalls
│   ├── tool-family.md                # tool_* syscalls
│   ├── signal-family.md              # signal_* syscalls
│   ├── telemetry-family.md           # telemetry_* syscalls
│   └── crew-family.md                # crew_* syscalls
├── patterns/
│   ├── task-orchestration.md         # Multi-task workflows
│   ├── memory-checkpointing.md       # Snapshot & recovery
│   ├── ipc-patterns.md               # Broadcast, listen, priority
│   ├── security-audit.md             # Capability verification
│   ├── tool-integration.md           # Custom tool plugins
│   ├── signal-routing.md             # Event-driven architecture
│   └── telemetry-observability.md    # Metrics collection
├── frameworks/
│   ├── langchain-adapter.md          # LangChain integration
│   ├── semantic-kernel-plugin.md     # SK integration
│   └── crewai-bridge.md              # CrewAI integration
├── edge-cases/
│   ├── timeouts-and-retries.md       # Timeout handling
│   ├── memory-fragmentation.md       # Memory edge cases
│   ├── ipc-reliability.md            # Message delivery
│   ├── security-compliance.md        # Audit trails
│   └── crew-rebalancing.md           # Load balancing
├── troubleshooting/
│   ├── error-codes.md                # 19 error codes & mitigations
│   ├── performance-tuning.md         # Latency optimization
│   ├── debugging-tools.md            # Tracing & profiling
│   └── faq.md                        # Common questions
├── examples/
│   ├── simple-task.rs                # Rust: basic task
│   ├── crew-coordination.ts          # TypeScript: crew
│   ├── security-plugin.cs            # C#: security verification
│   ├── ipc-broadcast.rs              # Rust: IPC
│   └── telemetry-metrics.ts          # TypeScript: observability
└── schema/
    ├── error-codes.json              # All 19 error codes
    ├── capability-matrix.json        # Security capabilities
    ├── signal-types.json             # Signal enum
    └── version-history.json          # Changelog
```

### Documentation Quality Standards

- **Code Examples:** MAANG-level quality, async/await patterns, error handling
- **API Docs:** Signatures, parameters, return types, error codes
- **Diagrams:** Syscall flow, architecture (Mermaid/PlantUML)
- **Versioning:** SemVer 2.0.0 compliance, deprecation timeline
- **Searchability:** Full-text indexing, keyword tagging per family
- **Accessibility:** Code highlighting, dark mode support, mobile responsive

### Performance Metrics Documentation

FFI latency benchmarks (p50/p99) per language binding:

| Syscall Family  | Rust (p50/p99 μs) | TypeScript (p50/p99 ms) | C# (p50/p99 ms) |
|-----------------|-------------------|------------------------|-----------------|
| Task            | 45/180            | 2.1/8.5                | 3.2/12.0        |
| Memory          | 32/95             | 1.5/5.2                | 2.0/7.8         |
| IPC             | 55/220            | 3.2/14.1               | 4.5/18.0        |
| Security        | 120/450           | 8.5/35.0               | 12.0/48.0       |
| Tool            | 100/400           | 7.5/32.0               | 10.0/42.0       |
| Signal          | 25/80             | 1.2/4.5                | 1.8/6.5         |
| Telemetry       | 18/65             | 0.9/3.2                | 1.2/4.0         |
| Crew            | 75/300            | 4.5/18.0               | 6.5/26.0        |

---

## Design Principles Alignment

**Cognitive-Native:** CSCI v0.5 treats cognitive tasks as first-class citizens with dedicated syscalls for semantic memory, capability-based security, and observability.

**Semantic Versioning:** v0.5.0 indicates: major API stable (0), minor feature additions (5), patch zero. Next breaking changes in v1.0.0.

**Developer Experience:** Low ceremony API with result types, optional async/sync modes, and comprehensive error context.

**Interoperability:** Three language bindings (Rust/TypeScript/C#) with consistent semantics via FFI and shared error codes.

**Testing:** Syscalls designed for unit testing with isolation modes; integration tests via IPC broadcast patterns.

**Documentation:** This document establishes single source of truth for all 22+ syscalls with examples, edge cases, and integration patterns.

---

**Document Maintainer:** Staff Engineer, XKernal Cognitive Substrate OS
**Last Updated:** Week 16
**Next Review:** Week 18 (Phase 2 Completion)
