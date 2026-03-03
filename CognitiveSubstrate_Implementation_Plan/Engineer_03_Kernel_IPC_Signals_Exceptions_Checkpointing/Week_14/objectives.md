# Engineer 3 — Kernel: IPC, Signals, Exceptions & Checkpointing — Week 14

## Phase: PHASE 1 — Advanced IPC & Distributed Communication

## Weekly Objective

Demonstrate full fault tolerance through integrated system: tool call failure triggers retry, context overflow triggers eviction, budget exhaustion triggers checkpoint+suspend, deadlock detection and resolution.

## Document References
- **Primary:** Section 6.2 (Exit Criteria — Fault Tolerance Demo)
- **Supporting:** All Sections 2.6-2.12, Section 3.2.4-3.2.8

## Deliverables
- [ ] Tool retry logic: on ToolCallFailed exception, invoke retry handler with exponential backoff
- [ ] Context overflow eviction: when working memory approaches limit, evict oldest data
- [ ] Budget exhaustion checkpoint: when tool/context budget at 95%, trigger checkpoint + suspend
- [ ] Deadlock detection: monitor CT wait chains, detect cycles
- [ ] Deadlock resolution: escalate to supervisor or force preemption
- [ ] Integrated demo: multi-agent system exercising all fault recovery paths
- [ ] Scenario testing: 5+ realistic failure scenarios with correct recovery
- [ ] Performance validation: recovery time < 100ms for transient failures
- [ ] Integration tests: all IPC types, signals, exceptions working together
- [ ] Documentation: fault recovery architecture and decision tree

## Technical Specifications

### Tool Retry Logic
```
fn handle_tool_call_failed_exception(
    ct: &mut ContextThread,
    context: &ExceptionContext,
) -> ExceptionHandlerResult {
    let tool_context = &context.tool_state;
    let retry_policy = RetryPolicy {
        backoff_ms: 100,
        max_retries: 3,
        backoff_multiplier: 2.0,
    };

    if tool_context.attempt < retry_policy.max_retries {
        // Schedule retry after backoff period
        let backoff_ms = (retry_policy.backoff_ms as f32
            * retry_policy.backoff_multiplier.powi(tool_context.attempt as i32)) as u64;
        ExceptionHandlerResult::Retry(retry_policy.with_delay_ms(backoff_ms))
    } else {
        // Max retries exceeded; escalate
        ExceptionHandlerResult::Escalate(ct.supervisor_ref)
    }
}

#[test]
fn test_tool_retry_on_failure() {
    // 1. Create CT with tool that fails
    // 2. Invoke tool, observe ToolCallFailed exception
    // 3. Verify exception handler triggers retry
    // 4. Verify first retry after 100ms
    // 5. Verify second retry after 200ms
    // 6. Verify escalation after max retries exceeded
}
```

### Context Overflow Eviction
```
pub struct ContextMemoryManager {
    pub working_memory: Vec<u8>,
    pub capacity: usize,
    pub current_usage: usize,
    pub eviction_threshold: f32,  // 0.9 = evict at 90%
    pub eviction_policy: EvictionPolicy,
}

pub enum EvictionPolicy {
    Lru,        // Evict least-recently-used
    Fifo,       // Evict first-in-first-out
    Priority,   // Evict lowest-priority items
}

impl ContextMemoryManager {
    pub fn on_memory_pressure(&mut self, ct: &ContextThread) -> Result<(), EvictionError> {
        if (self.current_usage as f32 / self.capacity as f32) > self.eviction_threshold {
            // Trigger eviction
            match self.eviction_policy {
                EvictionPolicy::Lru => self.evict_lru()?,
                EvictionPolicy::Fifo => self.evict_fifo()?,
                EvictionPolicy::Priority => self.evict_lowest_priority()?,
            }

            // Send signal to CT
            send_signal(ct.id, CognitiveSignal::SigContextLow);
        }
        Ok(())
    }

    fn evict_lru(&mut self) -> Result<(), EvictionError> {
        // Find least-recently-used item
        let lru_key = self.working_memory_lru_cache.lru_key()?;
        let evicted_size = self.working_memory.remove(&lru_key).ok_or(EvictionError::NotFound)?;
        self.current_usage -= evicted_size;
        Ok(())
    }
}

#[test]
fn test_context_overflow_eviction() {
    // 1. Create CT with limited working memory (100MB)
    // 2. Allocate data until 90% full
    // 3. Allocate more data to trigger eviction
    // 4. Verify SIG_CONTEXT_LOW sent
    // 5. Verify LRU item evicted
    // 6. Verify CT can continue execution
}
```

### Budget Exhaustion Checkpoint + Suspend
```
pub struct BudgetMonitor {
    pub tool_budget: u32,               // Maximum tool calls
    pub tool_used: u32,
    pub context_budget_bytes: u64,      // Maximum working memory
    pub context_used: u64,
    pub exhaustion_threshold: f32,      // 0.95 = checkpoint at 95%
}

impl BudgetMonitor {
    pub fn check_budgets(&mut self, ct: &mut ContextThread) -> Result<(), BudgetError> {
        let tool_usage = self.tool_used as f32 / self.tool_budget as f32;
        let context_usage = self.context_used as f32 / self.context_budget_bytes as f32;

        if tool_usage > self.exhaustion_threshold || context_usage > self.exhaustion_threshold {
            // Trigger checkpoint
            let checkpoint_id = ct.create_checkpoint()?;

            // Send warning signal
            send_signal(ct.id, CognitiveSignal::SigBudgetWarn);

            // Suspend CT if still near limit
            if tool_usage > 0.99 || context_usage > 0.99 {
                ct.suspend_until_checkpoint_safe()?;
            }
        }
        Ok(())
    }
}

#[test]
fn test_budget_exhaustion_checkpoint_suspend() {
    // 1. Create CT with tool budget = 10
    // 2. Call tool 9 times
    // 3. Verify SIG_BUDGET_WARN sent at 8 calls (80%)
    // 4. Verify checkpoint triggered at 9.5 calls (95%)
    // 5. Call tool 10th time
    // 6. Verify CT suspended
    // 7. Verify CT can be resumed from checkpoint
}
```

### Deadlock Detection
```
pub struct DeadlockDetector {
    pub wait_chains: HashMap<ContextThreadId, Vec<ContextThreadId>>,
}

impl DeadlockDetector {
    pub fn on_ct_block_for_ipc(
        &mut self,
        blocking_ct: ContextThreadId,
        on_channel: ChannelId,
    ) -> Result<(), DeadlockError> {
        // 1. Find which CT holds the resource
        let held_by = find_ct_holding_channel(on_channel)?;

        // 2. Check if held_by is waiting for blocking_ct (cycle)
        if self.has_cycle(blocking_ct, held_by) {
            return Err(DeadlockError::Detected);
        }

        // 3. Add edge: blocking_ct -> held_by
        self.wait_chains.entry(blocking_ct).or_insert_with(Vec::new).push(held_by);
        Ok(())
    }

    fn has_cycle(&self, from: ContextThreadId, to: ContextThreadId) -> bool {
        // DFS to detect cycle from -> ... -> to
        let mut visited = std::collections::HashSet::new();
        self.dfs(from, to, &mut visited)
    }

    fn dfs(
        &self,
        current: ContextThreadId,
        target: ContextThreadId,
        visited: &mut std::collections::HashSet<ContextThreadId>,
    ) -> bool {
        if current == target {
            return true;
        }
        if visited.contains(&current) {
            return false;
        }
        visited.insert(current);

        if let Some(waiting_on) = self.wait_chains.get(&current) {
            for next in waiting_on {
                if self.dfs(*next, target, visited) {
                    return true;
                }
            }
        }
        false
    }
}
```

### Deadlock Resolution
```
fn resolve_deadlock(
    deadlocked_cts: &[ContextThreadId],
) -> Result<(), ResolutionError> {
    // Strategy 1: Escalate to supervisor
    for ct_id in deadlocked_cts {
        let ct = get_ct(ct_id)?;
        send_exception_to_supervisor(ct, CognitiveException::InconsistentState)?;
    }

    // Strategy 2: Force preemption of lowest-priority CT
    let victim_ct = deadlocked_cts
        .iter()
        .min_by_key(|ct_id| get_ct(ct_id).priority)
        .ok_or(ResolutionError::NoCandidateForPreemption)?;

    preempt_ct(*victim_ct)?;

    Ok(())
}

#[test]
fn test_deadlock_detection_and_resolution() {
    // 1. Create 2 CTs
    // 2. CT1 waits for response from CT2
    // 3. CT2 waits for response from CT1 (circular wait)
    // 4. Verify deadlock detected within 1 second
    // 5. Verify escalation to supervisor or preemption
}
```

### Full Integration Test: Multi-Agent Fault Recovery Demo
```
#[test]
fn test_full_fault_tolerance_demo() {
    // Setup: 3 agents with supervisor
    // Agent1: coordinator
    // Agent2: worker with tool call
    // Agent3: data provider
    // Supervisor: handles exceptions and escalations

    // Scenario 1: Tool call failure -> retry -> success
    agent2.call_external_tool("fetch_data");
    // Tool fails on first attempt (mocked)
    // Handler triggers retry
    // Tool succeeds on second attempt
    assert!(agent2.has_data());

    // Scenario 2: Context overflow -> eviction
    agent1.allocate_large_context(500_000);
    // Working memory approaches limit
    // SIG_CONTEXT_LOW sent
    // LRU data evicted
    assert!(agent1.working_memory.usage() < agent1.capacity);

    // Scenario 3: Budget exhaustion -> checkpoint -> suspend
    for _ in 0..9 {
        agent1.call_tool();
    }
    // 9th call triggers SIG_BUDGET_WARN
    // 10th call would exceed budget
    agent1.try_call_tool();  // Blocks/suspended
    // Verify checkpoint created
    assert!(agent1.has_checkpoint());

    // Scenario 4: IPC failure -> escalate
    agent2.send_request_to_agent3();
    // Mock network failure
    // IpcFailure exception escalates to supervisor
    supervisor.verify_received_exception(CognitiveException::IpcFailure);

    // Scenario 5: Deadlock detection
    agent1.wait_for_agent2();
    agent2.wait_for_agent1();
    // Deadlock detected
    // Supervisor notified
    supervisor.verify_deadlock_detected();

    // Verify all agents recovered and working
    assert!(agent1.is_running());
    assert!(agent2.is_running());
    assert!(agent3.is_running());
}
```

### Fault Recovery Decision Tree
```
Exception Type              Recovery Strategy              Timeout
================================================================================
ToolCallFailed              Retry (exponential backoff)    Escalate after 3 retries
ContextOverflow             Evict LRU data                 Escalate if unrecoverable
DeadlineExceeded            Checkpoint + Suspend           Force preemption if timeout
IpcFailure                  Retry channel + Escalate       Escalate immediately
CapabilityViolation         Escalate to supervisor         Immediate
InconsistentState           Rollback to checkpoint         Escalate if no checkpoint
ReasoningDiverged           Checkpoint + Suspend           Escalate to supervisor
Unknown                     Escalate to supervisor         Immediate

Supervisor Actions:
  - Receive exception + context
  - Decide: Retry, Compensate, Abort, Restart
  - Implement compensation handler if available
  - Report results back to faulted CT
```

## Dependencies
- **Blocked by:** Week 1-13 (All prior work)
- **Blocking:** Week 15-24 (PHASE 2 — Optimization)

## Acceptance Criteria
1. Tool retry succeeds after transient failures
2. Context eviction prevents OOM
3. Budget exhaustion triggers checkpoint + suspend
4. Deadlock detection finds cycles within 1 second
5. Deadlock resolution prevents indefinite hangs
6. Multi-agent demo exercises all 5 fault recovery paths
7. No data corruption during recovery
8. All exception types properly handled
9. Supervisor receives and responds to escalations
10. Recovery time < 100ms for transient failures (network, tool timeout)
11. Comprehensive documentation of fault recovery decision tree

## Design Principles Alignment
- **Fault Tolerance:** Multiple recovery strategies handle different failure modes
- **Observability:** Exceptions capture full context for debugging
- **Reliability:** Checkpointing enables graceful recovery
- **Scalability:** Multi-agent system demonstrates distributed fault handling
