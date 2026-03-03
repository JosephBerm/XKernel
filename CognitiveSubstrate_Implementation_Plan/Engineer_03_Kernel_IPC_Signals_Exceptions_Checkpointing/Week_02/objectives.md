# Engineer 3 — Kernel: IPC, Signals, Exceptions & Checkpointing — Week 02

## Phase: PHASE 0 — Formalization & Synchronous IPC

## Weekly Objective

Formalize CognitiveException (8 types with severity levels), CognitiveSignal (8 signal types), CognitiveCheckpoint (complete structure with context snapshot, chain position, and state references), and WatchdogConfig (timing and iteration parameters).

## Document References
- **Primary:** Section 2.7 (Cognitive Exception Engine), Section 2.8 (Cognitive Signal Dispatch), Section 2.9 (Cognitive Checkpointing Engine), Section 2.12 (Reasoning Watchdog)
- **Supporting:** Section 3.2.5 (Signal Dispatch), Section 3.2.6 (Exception Handling), Section 3.2.7 (Checkpointing), Section 3.2.8 (Watchdog)

## Deliverables
- [ ] CognitiveException enum with 8 types: ToolCallFailed, ReasoningDiverged, DeadlineExceeded, ContextOverflow, IpcFailure, CapabilityViolation, InconsistentState, Unknown
- [ ] Severity levels for each exception: Critical, High, Medium, Low
- [ ] CognitiveSignal enum with 8 types: SigTerminate, SigDeadlineWarn, SigCheckpoint, SigBudgetWarn, SigContextLow, SigIpcFailed, SigPreempt, SigResume
- [ ] CognitiveCheckpoint struct with: id, ct_ref, timestamp, phase, context_snapshot, chain_position, memory_refs, tool_state, capability_state, ipc_state
- [ ] WatchdogConfig struct with: deadline_ms, max_phase_iterations, tool_retry_limit, loop_detection_threshold
- [ ] Exception handler return type: Retry | Rollback(checkpoint_id) | Escalate(supervisor_ref) | Terminate(partial_results)
- [ ] Unit tests for all exception types and signal dispatch
- [ ] Documentation detailing exception semantics and signal delivery guarantees

## Technical Specifications

### CognitiveException Definition (8 Types)
```
pub enum CognitiveException {
    ToolCallFailed(ToolFailureContext),           // Tool invocation returned error
    ReasoningDiverged(DivergenceContext),         // Exceeded max phase iterations
    DeadlineExceeded(DeadlineContext),            // Wall-clock deadline breached
    ContextOverflow(MemoryContext),               // Working memory limit exceeded
    IpcFailure(IpcErrorContext),                  // IPC operation failed
    CapabilityViolation(CapabilityContext),       // Attempted unauthorized access
    InconsistentState(StateContext),              // Internal state validation failed
    Unknown(Box<dyn std::error::Error>),          // Uncategorized error
}
```

### CognitiveException Severity Levels
- **Critical:** DeadlineExceeded, InconsistentState (immediate escalation required)
- **High:** CapabilityViolation, IpcFailure (must escalate to supervisor)
- **Medium:** ToolCallFailed, ContextOverflow (retry or checkpoint)
- **Low:** ReasoningDiverged (informational, typically retried)

### CognitiveSignal Definition (8 Types)
```
pub enum CognitiveSignal {
    SigTerminate,           // Unhandleable; always results in CT termination
    SigDeadlineWarn,        // Deadline within 10% remaining time
    SigCheckpoint,          // Request immediate checkpoint
    SigBudgetWarn,          // Tool/context budget at 80% capacity
    SigContextLow,          // Working memory at 70% capacity
    SigIpcFailed,           // IPC channel error detected
    SigPreempt,             // Kernel preempting CT
    SigResume,              // CT resumed after preemption
}
```

### CognitiveCheckpoint Structure
```
pub struct CognitiveCheckpoint {
    pub id: CheckpointId,
    pub ct_ref: ContextThreadRef,
    pub timestamp: Timestamp,
    pub phase: ReasoningPhase,
    pub context_snapshot: ContextSnapshot,
    pub chain_position: usize,          // Position in checkpoint chain
    pub memory_refs: Vec<MemoryRegion>,
    pub tool_state: ToolStateSnapshot,
    pub capability_state: CapabilitySnapshot,
    pub ipc_state: IpcStateSnapshot,
    pub hash_chain: Vec<u8>,            // SHA256 of previous checkpoint
}
```

### WatchdogConfig Structure
```
pub struct WatchdogConfig {
    pub deadline_ms: u64,               // Wall-clock deadline (0 = disabled)
    pub max_phase_iterations: u32,      // Default: 10
    pub tool_retry_limit: u32,          // Default: 3
    pub loop_detection_threshold: u32,  // Iterations before signaling divergence
}
```

### Exception Handler Return Type
```
pub enum ExceptionHandlerResult {
    Retry(RetryPolicy),                // Retry with specified backoff
    Rollback(CheckpointId),            // Restore from checkpoint
    Escalate(SupervisorRef),           // Send to supervisor handler
    Terminate(PartialResults),         // Graceful shutdown with partial output
}
```

## Dependencies
- **Blocked by:** Week 1 (SemanticChannel formalization)
- **Blocking:** Week 3-4 Signal Dispatch, Week 4-5 Exception Engine, Week 5-6 Checkpointing Engine, Week 6 Watchdog

## Acceptance Criteria
1. All 8 exception types compile and include meaningful context structures
2. All 8 signal types are deliverable via interrupt handlers
3. CognitiveCheckpoint includes all required state snapshots
4. WatchdogConfig defaults align with design specifications
5. Hash-chain field in checkpoint supports tamper detection
6. Exception handler return type supports all four recovery strategies
7. Comprehensive unit tests for exception and signal handling
8. Documentation includes signal delivery timing guarantees

## Design Principles Alignment
- **Fault Tolerance:** Exception handler return type provides multiple recovery paths
- **Observability:** Each exception type captures specific context for debugging
- **Preemption Safety:** Signals delivered only at safe preemption points
- **Tamper Evidence:** Hash-chain in checkpoints prevents silent corruption
