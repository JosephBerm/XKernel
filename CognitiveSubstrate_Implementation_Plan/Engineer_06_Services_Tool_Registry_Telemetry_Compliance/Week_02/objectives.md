# Engineer 6 — Services: Tool Registry, Telemetry & Compliance — Week 2

## Phase: Phase 0 (Weeks 1-6)

## Weekly Objective
Finalize ToolBinding entity design from Week 1 and formalize all 10 CEF (Common Event Format) event types with complete field specifications. Establish the telemetry event backbone that will capture kernel behavior across all phases.

## Document References
- **Primary:** Section 2.11 (ToolBinding completed), Section 3.3.4 (Cognitive Telemetry Engine, CEF events)
- **Supporting:** Section 3.3.5 (Compliance audit trails), Section 3.3.6 (Policy events)

## Deliverables
- [ ] Finalize ToolBinding design from Week 1 based on review feedback
- [ ] Define all 10 CEF event types with complete specifications:
  1. ThoughtStep
  2. ToolCallRequested
  3. ToolCallCompleted
  4. PolicyDecision
  5. MemoryAccess
  6. IPCMessage
  7. PhaseTransition
  8. CheckpointCreated
  9. SignalDispatched
  10. ExceptionRaised
- [ ] CEF event schema document (base and per-event-type fields)
- [ ] Event serialization format specification (JSON/binary/protobuf)
- [ ] Event correlation and tracing ID scheme
- [ ] Timestamp and wall-clock time precision requirements
- [ ] Cost attribution metadata structure (tokens, GPU-ms, wall-clock, TPC-hours)

## Technical Specifications

### Base CEF Event Structure
```
struct CEFEvent {
    event_id: string,              // Unique event ID (UUID)
    event_type: EventType,         // One of 10 types
    timestamp_utc: i64,            // Unix microseconds
    wall_clock_ms: u64,            // Wall-clock duration from start
    actor: string,                 // Agent/component generating event
    resource: string,              // Resource being acted upon
    action: string,                // Action performed
    result: EventResult,           // COMPLETED | FAILED | DENIED
    context: Map<string, string>,  // Event-specific context
    cost_attribution: CostMetrics, // Cost tracking
    trace_id: string,              // For correlation
    parent_event_id: Option<string> // Causality chain
}

enum EventType {
    ThoughtStep,
    ToolCallRequested,
    ToolCallCompleted,
    PolicyDecision,
    MemoryAccess,
    IPCMessage,
    PhaseTransition,
    CheckpointCreated,
    SignalDispatched,
    ExceptionRaised
}

struct CostMetrics {
    input_tokens: u64,
    output_tokens: u64,
    gpu_milliseconds: f64,
    wall_clock_milliseconds: f64,
    tpc_hours: f64 // TPC = Token Processing Cost
}
```

### Event-Type Specific Fields

#### ThoughtStep
- reasoning_text: string
- model_id: string
- context_window_tokens: u64
- decisions_considered: u32

#### ToolCallRequested
- tool_binding_id: string
- input_schema: string
- estimated_cost: CostMetrics
- capability_required: string

#### ToolCallCompleted
- tool_binding_id: string
- actual_cost: CostMetrics
- output_schema: string
- execution_time_ms: f64
- response_cached: bool

#### PolicyDecision
- decision_type: string
- rule_id: string
- policy_version_hash: string
- inputs: Map<string, string>
- outcome: ALLOW | DENY | REQUIRE_APPROVAL | AUDIT | WARN
- reason_code: string
- explanation_redacted: Option<string> // Article 12(2)(a)

#### MemoryAccess
- address_range: string
- access_type: READ | WRITE
- size_bytes: u64
- checkpoint_ref: Option<string>

#### IPCMessage
- sender_agent: string
- receiver_agent: string
- message_size_bytes: u64
- priority: u8
- delivery_status: SENT | RECEIVED | FAILED

#### PhaseTransition
- old_phase: string
- new_phase: string
- transition_reason: string
- agent_count: u32

#### CheckpointCreated
- checkpoint_id: string
- checkpoint_size_bytes: u64
- memory_committed: u64
- gpu_state_committed: bool
- cpu_state_committed: bool

#### SignalDispatched
- signal_type: string
- target_agent: string
- signal_payload_size: u64
- async_mode: bool

#### ExceptionRaised
- exception_type: string
- error_message: string
- stack_trace: string
- recovery_attempted: bool
- recovery_outcome: Option<string>

## Dependencies
- **Blocked by:** Week 1 (ToolBinding formalization)
- **Blocking:** Week 3-4 (telemetry CEF format design), Week 5-6 (telemetry engine implementation)

## Acceptance Criteria
- [ ] All 10 event types defined with mandatory and optional fields
- [ ] CEF event schema passes validation against industry CEF standards
- [ ] Serialization format chosen and documented
- [ ] Event correlation mechanism defined
- [ ] Cost metrics structure supports all planned attribution scenarios
- [ ] Timestamp precision justification documented (microsecond vs millisecond)
- [ ] Design review sign-off from telemetry architect

## Design Principles Alignment
- **Structured observability:** Every event self-describing and independently parseable
- **Immutability:** Events are append-only; no retroactive modification
- **Causality:** trace_id and parent_event_id enable full request flow reconstruction
- **Cost transparency:** Every event carries cost metrics for billing and optimization
- **Regulatory alignment:** PolicyDecision events support EU AI Act Article 12(2)(a) requirements
