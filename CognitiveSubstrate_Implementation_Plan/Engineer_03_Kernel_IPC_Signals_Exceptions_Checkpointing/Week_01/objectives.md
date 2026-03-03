# Engineer 3 — Kernel: IPC, Signals, Exceptions & Checkpointing — Week 01

## Phase: PHASE 0 — Formalization & Synchronous IPC

## Weekly Objective

Formalize the core type system for Semantic IPC: SemanticChannel data structure with complete protocol, endpoint, delivery guarantee, backpressure, and context-sharing specifications. Establish the foundation for all subsequent IPC subsystem work.

## Document References
- **Primary:** Section 2.6 (Semantic IPC Subsystem Design)
- **Supporting:** Section 3.2.4 (Request-Response IPC), Section 6.2 (Exit Criteria)

## Deliverables
- [ ] SemanticChannel struct definition with all fields: id, protocol, endpoints, delivery, backpressure, context_sharing, distributed
- [ ] ProtocolSpec enum with semantic negotiation variants
- [ ] DeliveryGuarantee enum: at_most_once, at_least_once, exactly_once_local
- [ ] BackpressurePolicy enum: drop, suspend, signal_warn
- [ ] ContextMode enum: none, read_only, read_write
- [ ] DistributedConfig type definition for cross-machine channels
- [ ] Unit tests for SemanticChannel validation
- [ ] Rust trait implementations (Debug, Clone, PartialEq)
- [ ] Design document detailing rationale for each field

## Technical Specifications

### SemanticChannel Definition
```
pub struct SemanticChannel {
    pub id: ChannelId,
    pub protocol: ProtocolSpec,
    pub endpoints: EndpointPair,
    pub delivery: DeliveryGuarantee,
    pub backpressure: BackpressurePolicy,
    pub context_sharing: ContextMode,
    pub distributed: Option<DistributedConfig>,
}
```

### ProtocolSpec Variants
- ReAct: Standard reasoning/action/observation loop
- StructuredData: Strongly-typed schema-driven messaging
- EventStream: Fire-and-forget event publishing
- Custom(String): User-defined protocol identifier

### DeliveryGuarantee Semantics
- **at_most_once:** Single delivery attempt, may be lost on failure
- **at_least_once:** Retried until ACKed, may deliver duplicates
- **exactly_once_local:** Exactly once within single-machine, requires idempotency keys across machines

### BackpressurePolicy Variants
- Drop: Silently drop messages if buffer full
- Suspend: Block sender until buffer has space
- SignalWarn: Send SIG_BUDGET_WARN, then drop if not heeded

### ContextMode Semantics
- **none:** No working memory sharing between sender/receiver
- **read_only:** Receiver sees sender's working memory as immutable snapshot
- **read_write:** Both agents map same physical pages; CRDT resolves conflicts

## Dependencies
- **Blocked by:** None (foundational work)
- **Blocking:** Week 2-3 Synchronous IPC, Week 7-8 Pub/Sub IPC, Week 9-10 Shared Context IPC

## Acceptance Criteria
1. SemanticChannel compiles without warnings in Rust stable
2. All enum variants properly represent intended semantics
3. DistributedConfig includes capability re-verification fields
4. Unit tests achieve 95%+ code coverage
5. Design document includes rationale for delivery guarantee choices
6. Type system prevents invalid configuration combinations (e.g., exactly_once_local with distributed)

## Design Principles Alignment
- **Capability-Based Security:** ProtocolSpec and context_sharing ensure minimal privilege by default
- **Explicit Semantics:** Each enum variant has clear, unambiguous behavior
- **Testability:** Types are simple, immutable after construction
- **Performance:** Zero-cost abstractions; enums compile to efficient bit patterns
