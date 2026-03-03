# Engineer 3 — Kernel: IPC, Signals, Exceptions & Checkpointing — Week 04

## Phase: PHASE 0 — Formalization & Synchronous IPC

## Weekly Objective

Implement signal dispatch table and interrupt handler infrastructure to deliver 8 cognitive signals at safe preemption points. Enforce that SIG_TERMINATE cannot be caught or ignored.

## Document References
- **Primary:** Section 3.2.5 (Signal Dispatch Table)
- **Supporting:** Section 2.8 (Cognitive Signal Dispatch), Section 6.2 (Exit Criteria)

## Deliverables
- [ ] Signal dispatch table: per-CT mapping of signal -> handler function pointer
- [ ] Hardware interrupt handlers for all 8 signal types
- [ ] Preemption point detection: identify safe CT state transitions
- [ ] sig_register syscall: handler registration with validation
- [ ] sig_unregister syscall: clean removal of signal handlers
- [ ] SIG_TERMINATE special case: cannot be caught, always terminates CT
- [ ] Signal delivery mechanism: synchronous at preemption points, no race conditions
- [ ] Unit tests for signal delivery and handler invocation
- [ ] Stress test: signal delivery under load without missed signals

## Technical Specifications

### Signal Dispatch Table
```
pub struct SignalDispatchTable {
    pub ct_id: ContextThreadId,
    pub handlers: [Option<SignalHandler>; 8],  // One per signal type
    pub pending_signals: VecDeque<CognitiveSignal>,
    pub signal_mask: u8,  // Bitmask of blocked signals
}

type SignalHandler = unsafe extern "C" fn(&CognitiveSignal) -> SignalHandlerResult;

pub enum SignalHandlerResult {
    Continue,      // Resume CT execution
    Restart,       // Restart current operation
    Escalate,      // Pass to supervisor
}
```

### 8 Signal Types with Delivery Semantics
1. **SIG_TERMINATE:** Immediate termination, cannot be caught/ignored
2. **SIG_DEADLINE_WARN:** Delivered when deadline within 10% remaining
3. **SIG_CHECKPOINT:** Request immediate checkpoint; handler may return Continue or Escalate
4. **SIG_BUDGET_WARN:** Tool/context budget at 80%; may be ignored
5. **SIG_CONTEXT_LOW:** Working memory at 70%; informational
6. **SIG_IPC_FAILED:** IPC channel failure; typically escalates
7. **SIG_PREEMPT:** CT about to be preempted; allows cleanup
8. **SIG_RESUME:** CT resumed after preemption; recovery handler

### Preemption Point Detection
Safe preemption points identified by:
- After syscall completion (before returning to user code)
- Between reasoning phases (transition from Observe to Act)
- At timer interrupt boundaries
- Before context switch

### sig_register Syscall
```
syscall fn sig_register(signal: CognitiveSignal, handler: *const SignalHandler) -> Result<(), RegisterError> {
    // Validation:
    // - signal must be non-TERMINATE or return PERMISSION_DENIED
    // - handler must point to valid executable memory
    // - handler signature must match expected type
    // Atomically update signal dispatch table
}
```

### sig_unregister Syscall
```
syscall fn sig_unregister(signal: CognitiveSignal) -> Result<(), UnregisterError> {
    // Remove handler for signal; subsequent deliveries ignored
    // If signal pending, it is discarded
}
```

### SIG_TERMINATE Handling
- Cannot be registered (sig_register returns PERMISSION_DENIED)
- Cannot be masked (signal_mask bit ignored)
- Delivered synchronously at next preemption point
- Always results in CT termination, no handler invocation
- Kernel captures state before termination for debugging

### Signal Delivery Algorithm
```
fn deliver_pending_signals(ct: &ContextThread) {
    while let Some(signal) = ct.signal_dispatch_table.pending_signals.pop_front() {
        // Check if signal is masked
        if (ct.signal_dispatch_table.signal_mask & signal_bit(signal)) != 0 {
            continue;  // Skip masked signal
        }

        // Special case: SIG_TERMINATE always kills CT
        if signal == CognitiveSignal::SigTerminate {
            terminate_ct_immediate(ct);
            return;
        }

        // Invoke handler at preemption point
        if let Some(handler) = ct.signal_dispatch_table.handlers[signal_index(signal)] {
            let result = unsafe { handler(&signal) };
            match result {
                SignalHandlerResult::Continue => {},
                SignalHandlerResult::Restart => ct.restart_current_operation(),
                SignalHandlerResult::Escalate => escalate_to_supervisor(ct, signal),
            }
        }
    }
}
```

## Dependencies
- **Blocked by:** Week 1-2 (Formalization), Week 3 (Request-Response IPC)
- **Blocking:** Week 4-5 Exception Engine, Week 6 Watchdog

## Acceptance Criteria
1. All 8 signal types deliver correctly at preemption points
2. SIG_TERMINATE cannot be registered or masked (validation tests pass)
3. No signal delivery races (concurrency tests pass)
4. Handler invocation preserves CT state (register contents, instruction pointer)
5. sig_register/unregister syscalls work atomically
6. Stress test delivers 1000+ signals/second without missing any
7. Unit tests cover: basic delivery, masking, handler errors, SIG_TERMINATE special cases
8. No kernel panics on invalid handler addresses

## Design Principles Alignment
- **Safety:** Preemption points ensure CT is in consistent state before signal delivery
- **Capability-Based:** Only CT can register its own handlers; cross-CT signal delivery requires capability
- **Determinism:** Delivery order is FIFO; handler results are deterministic
- **Reliability:** SIG_TERMINATE enforcement prevents misbehaving handlers from indefinite loops
