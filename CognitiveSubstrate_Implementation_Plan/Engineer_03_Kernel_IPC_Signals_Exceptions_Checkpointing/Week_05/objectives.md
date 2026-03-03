# Engineer 3 — Kernel: IPC, Signals, Exceptions & Checkpointing — Week 05

## Phase: PHASE 0 — Formalization & Synchronous IPC

## Weekly Objective

Implement the Cognitive Exception Engine: kernel exception handling on CT pause with custom handler invocation, context capture, and four recovery strategies (Retry, Rollback, Escalate, Terminate).

## Document References
- **Primary:** Section 3.2.6 (Exception Handling Engine)
- **Supporting:** Section 2.7 (Cognitive Exception Engine), Section 6.2 (Exit Criteria)

## Deliverables
- [ ] Exception context capture: full register state, working memory snapshot, tool call context
- [ ] exc_register syscall: register custom exception handler per CT
- [ ] exc_unregister syscall: remove exception handler
- [ ] Exception dispatch on CT pause: kernel calls handler with captured context
- [ ] Handler return type implementation: Retry, Rollback, Escalate, Terminate
- [ ] Retry strategy: exponential backoff with max retries
- [ ] Rollback strategy: restore CT state from checkpoint
- [ ] Escalate strategy: bubble exception to supervisor CT
- [ ] Terminate strategy: graceful shutdown with partial results
- [ ] Unit tests for all four recovery strategies

## Technical Specifications

### Exception Context Capture
```
pub struct ExceptionContext {
    pub exception: CognitiveException,
    pub registers: RegisterSnapshot,      // CPU register state at pause
    pub working_memory: MemorySnapshot,   // Working memory contents
    pub tool_state: ToolCallContext,      // Tool invocation details
    pub ipc_state: IpcStateSnapshot,      // IPC channel state
    pub checkpoint_available: Option<CheckpointId>,  // Last checkpoint if available
    pub timestamp: Timestamp,
}

pub struct RegisterSnapshot {
    pub rax: u64, pub rbx: u64, pub rcx: u64, pub rdx: u64,
    pub rsi: u64, pub rdi: u64, pub rbp: u64, pub rsp: u64,
    pub r8: u64, pub r9: u64, pub r10: u64, pub r11: u64,
    pub r12: u64, pub r13: u64, pub r14: u64, pub r15: u64,
    pub rip: u64, pub rflags: u64,
}
```

### Custom Exception Handler
```
type ExceptionHandler = unsafe extern "C" fn(&ExceptionContext) -> ExceptionHandlerResult;

pub enum ExceptionHandlerResult {
    Retry(RetryPolicy),
    Rollback(CheckpointId),
    Escalate(SupervisorRef),
    Terminate(PartialResults),
}

pub struct RetryPolicy {
    pub backoff_ms: u64,          // Initial backoff duration
    pub max_retries: u32,          // Maximum retry attempts
    pub backoff_multiplier: f32,   // Exponential backoff: next_backoff = backoff * multiplier
}

pub struct PartialResults {
    pub status: TerminationStatus,
    pub output: Vec<u8>,           // Partial output to return
}
```

### Exception Engine State
```
pub struct ExceptionEngine {
    pub ct_id: ContextThreadId,
    pub handler: Option<ExceptionHandler>,
    pub exception_history: VecDeque<ExceptionContext>,  // Last 10 exceptions
    pub in_exception_handler: bool,  // Prevent recursive exception handling
}
```

### exc_register Syscall
```
syscall fn exc_register(handler: *const ExceptionHandler) -> Result<(), RegisterError> {
    // Validation:
    // - handler must point to valid executable memory
    // - no existing handler for this CT
    // Atomically store handler in exception engine
}
```

### Exception Handling Flow
1. **Kernel Detects Exception:** CT encounters error condition (tool fails, deadline exceeded, etc.)
2. **CT Pause:** Kernel preempts CT via timer interrupt or exception handler invocation
3. **Context Capture:** Kernel captures registers, working memory, tool state, IPC state
4. **Create ExceptionContext:** Package all captured state into ExceptionContext struct
5. **Invoke Handler:** If custom handler registered, call it with ExceptionContext
6. **Process Result:**
   - **Retry:** Sleep for backoff_ms, increment retry counter, resume CT at same instruction
   - **Rollback:** Restore CT state from checkpoint, resume execution
   - **Escalate:** Send exception + context to supervisor CT, suspend current CT
   - **Terminate:** Clean shutdown, preserve partial results for caller

### Recovery Strategies

#### Retry Strategy
- Used for transient failures (tool network timeout, temporary resource shortage)
- Exponential backoff prevents hammering on failed resource
- Max retries prevents infinite loops
- Implementation: schedule CT to resume after backoff duration

#### Rollback Strategy
- Used for inconsistent state or application bugs
- Requires valid checkpoint with ID
- Restore: copy checkpoint data back to CT's memory
- Resume: set instruction pointer to next operation after checkpoint

#### Escalate Strategy
- Used for exceptions handler cannot resolve (capability violations, inconsistent state)
- Supervisor ref must be valid CT with escalation permission
- Send: ExceptionContext + original CT ref to supervisor's IPC channel
- Suspend: original CT remains paused until supervisor responds

#### Terminate Strategy
- Used when recovery is impossible (resource exhaustion, fatal error)
- Partial results: data structure returned to caller for partial results
- Status: termination reason (success, user_stop, timeout, out_of_memory, etc.)
- Clean shutdown: free CT resources, update kernel state tables

## Dependencies
- **Blocked by:** Week 1-4 (Formalization, Signals)
- **Blocking:** Week 5-6 Checkpointing, Week 13-14 Full Fault Tolerance Demo

## Acceptance Criteria
1. Exception context capture is complete and accurate
2. All four recovery strategies (Retry, Rollback, Escalate, Terminate) work correctly
3. exc_register/unregister syscalls work atomically and safely
4. Exponential backoff prevents excessive retries
5. Rollback correctly restores CT state from checkpoint
6. Escalation properly bubbles exception to supervisor
7. No recursive exception handling (in_exception_handler flag prevents loops)
8. Unit tests cover: basic exception delivery, all four recovery strategies, exception history tracking
9. Stress test: 100+ exceptions/second handled without kernel panics

## Design Principles Alignment
- **Fault Tolerance:** Multiple recovery strategies support different failure modes
- **Observability:** Exception history enables debugging and analytics
- **Safety:** Context capture prevents state corruption during recovery
- **Capability-Based:** Supervisor escalation requires valid capability
