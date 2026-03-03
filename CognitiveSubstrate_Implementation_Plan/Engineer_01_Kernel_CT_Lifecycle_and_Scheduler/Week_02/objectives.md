# Engineer 1 — Kernel: CT Lifecycle & Scheduler — Week 02

## Phase: PHASE 0 — Domain Model + Kernel Skeleton (Weeks 1-6)

## Weekly Objective
Complete domain model formalization and begin CT lifecycle state machine implementation. Implement phase transition rules with compile-time validation via Rust type state pattern to ensure illegal state transitions are impossible.

## Document References
- **Primary:** Section 2.1 (CTPhase enum transitions), Section 2.4 (Capability entity and seL4-inspired OCap model), Section 2.10 (MandatoryCapabilityPolicy)
- **Supporting:** Section 3.2.1 (Boot Sequence overview), Section 3.2.4 (Capability Enforcement Engine)

## Deliverables
- [ ] Rust module `ct_phase_machine.rs` — type-state pattern implementation ensuring valid phase transitions at compile time
- [ ] CTPhase enum with 8 variants: spawn | plan | reason | act | reflect | yield | complete | failed
- [ ] Phase transition validation — reject invalid transitions (e.g., reason → act without planning)
- [ ] Test suite proving illegal transitions fail to compile (compile_fail tests)
- [ ] Capability struct with all 9 properties (id: CapID, target: ResourceRef, operations: Set<Operation>, constraints: CapConstraints, provenance: CapChain, revocable_by: Set<AgentID>, attenuation: AttenuationPolicy)
- [ ] Capability OCap model — capabilities unforgeable, cryptographically signed at distributed boundaries
- [ ] MandatoryCapabilityPolicy struct with all 5 properties (id, rule, scope, enforcement: deny|audit|warn, exceptions)
- [ ] Draft CSCI v0.1 specification (22 system calls) — coordinate with SDK stream

## Technical Specifications
**Phase Transitions (Section 2.1):**
- spawn → plan (CT created, dependencies validated)
- plan → reason (planning complete, dependencies satisfied)
- reason → act (reasoning complete, action plan determined)
- act → reflect (actions executed, gathering results)
- reflect → yield (reflection complete, yielding to scheduler) or complete (all done)
- yield → plan (new reasoning cycle) or complete (task finished)
- any phase → failed (exception or error occurred)
- complete/failed → terminal (no further transitions)

**Capability Properties (Section 2.4):**
- id: CapID — unforgeable kernel-space handle
- target: ResourceRef — tool, memory region, agent, or data domain
- operations: Set<Operation> — read, write, execute, invoke, subscribe
- constraints: CapConstraints — time-bound, rate-limited, data-volume-limited, chain-depth-limited
- provenance: CapChain — full delegation chain from root authority to current holder
- revocable_by: Set<AgentID> — which agents can revoke this capability
- attenuation: AttenuationPolicy — how this can be further delegated (membrane pattern)

**Type State Pattern (Rust Example Pseudocode):**
```rust
struct CTState<S: CTPhaseMarker> { /* ... */ }
impl CTState<Spawn> {
  fn transition_to_plan(self) -> CTState<Plan> { /* ... */ }
  // Cannot call transition_to_reason, transition_to_act, etc. directly
}
impl CTState<Plan> {
  fn transition_to_reason(self) -> CTState<Reason> { /* ... */ }
}
// Illegal transitions fail at compile time
```

**CSCI v0.1 Syscalls (Section 3.5.1) — 22 system calls:**
- Task: ct_spawn, ct_yield, ct_checkpoint, ct_resume
- Memory: mem_alloc, mem_read, mem_write, mem_mount
- IPC: chan_open, chan_send, chan_recv
- Security: cap_grant, cap_delegate, cap_revoke
- Tools: tool_bind, tool_invoke
- Signals/Exceptions: sig_register, exc_register
- Telemetry: trace_emit
- Crews: crew_create, crew_join

## Dependencies
- **Blocked by:** Week 01 domain model completion
- **Blocking:** Week 03 (round-robin scheduler and boot sequence need phase machine), Week 02 SDK work on CSCI spec

## Acceptance Criteria
- [ ] Type-state machine compiles and all phase transitions type-check
- [ ] At least 5 illegal transition attempts fail with clear compile-time errors
- [ ] Capability struct fully documents OCap semantics with seL4 reference
- [ ] MandatoryCapabilityPolicy enforcement framework in place
- [ ] CSCI v0.1 spec draft complete with all 22 syscalls documented
- [ ] Code review by one other kernel engineer confirms phase machine correctness

## Design Principles Alignment
- **P2 — Cognitive Primitives as Kernel Abstractions:** Phase machine is fundamental kernel abstraction for CT scheduling
- **P3 — Capability-Based Security from Day Zero:** Capability OCap model prevents ambient authority
- **P5 — Observable by Default:** Phase transitions provide full CT lifecycle traceability
