# Engineer 1 — Kernel: CT Lifecycle & Scheduler — Week 01

## Phase: PHASE 0 — Domain Model + Kernel Skeleton (Weeks 1-6)

## Weekly Objective
Formalize all 12 domain model entities in Rust types with complete properties and kernel-enforced invariants. Establish the foundational type system that the entire Cognitive Substrate OS depends on.

## Document References
- **Primary:** Section 2.1 (CognitiveTask entity, all properties and 6 invariants), Section 2.2 (Agent entity), Section 2.3 (AgentCrew entity)
- **Supporting:** Section 2.4-2.12 (Capability, SemanticMemory, SemanticChannel, CognitiveException, CognitiveSignal, CognitiveCheckpoint, MandatoryCapabilityPolicy, ToolBinding, WatchdogConfig), Section 2.13 (Entity Relationship Summary)

## Deliverables
- [ ] Rust module `cognitive_task.rs` — CognitiveTask struct with all 19 properties (id: ULID, parent_agent: AgentRef, crew: Option<CrewRef>, phase: CTPhase enum, priority: CognitivePriority, capabilities: CapabilitySet, context_window: WorkingMemoryRef, dependencies: DAG<CTRef>, resource_budget: ResourceQuota, trace_log: TraceRef, checkpoint_refs: Vec<CheckpointRef>, signal_handlers: SignalHandlerTable, exception_handler: Option<ExcHandlerRef>, watchdog_config: WatchdogConfig)
- [ ] CTPhase enum: spawn | plan | reason | act | reflect | yield | complete | failed with compile-time validation
- [ ] Invariant enforcement types — all 6 invariants compile-checked via Rust type system
- [ ] Rust module `agent.rs` — Agent struct with all 12 properties (id, capabilities, memory_state, trust_level, framework_adapter, active_tasks, communication_protocols, resource_quota, lifecycle_config)
- [ ] Rust module `agent_crew.rs` — AgentCrew struct with all 8 properties (id, mission, members, coordinator, collective_budget, shared_memory, scheduling_affinity)
- [ ] Complete domain model review document (markdown) — cross-reference every property to the engineering plan Section 2

## Technical Specifications
**CognitiveTask Properties (Section 2.1):**
- id: ULID — globally unique, time-sortable
- parent_agent: AgentRef — kernel reference type ensuring validity
- crew: Option<CrewRef> — optional crew membership
- phase: CTPhase — 8 discrete phases with transition rules
- priority: CognitivePriority — 4-dimensional scoring struct (chain_criticality: f32 [0,1], resource_efficiency: f32 [0,1], deadline_pressure: f32 [0,1], capability_cost: f32 [0,1])
- capabilities: CapabilitySet — immutable set granted at spawn; type-enforced subset of parent Agent
- context_window: WorkingMemoryRef — allocated L1 semantic memory region
- dependencies: DAG<CTRef> — acyclic directed graph; cycle-check at spawn time
- resource_budget: ResourceQuota — max tokens, GPU-ms, wall-clock, memory bytes, tool calls
- trace_log: TraceRef — kernel telemetry stream reference
- checkpoint_refs: Vec<CheckpointRef> — ordered list of cognitive state snapshots for resume-from-failure
- signal_handlers: SignalHandlerTable — map of signal type to handler function references
- exception_handler: Option<ExcHandlerRef> — custom handler; kernel default if None
- watchdog_config: WatchdogConfig — deadline timeout (u64 ms), max iterations (u32), loop detection threshold

**Domain Model Invariants (Section 2.1):**
1. Capabilities always subset of parent Agent's capability graph
2. Resource budget cannot exceed parent Agent's total quota
3. All dependencies must complete before CT enters reason phase
4. All phase transitions must be logged to trace_log immediately
5. Dependency DAG cycle-checked at spawn time; circular dependencies rejected with error
6. Watchdog enforces deadline and loop detection; violations trigger exceptions

**Agent Properties (Section 2.2):**
- id: AgentID — cryptographically derived identity
- capabilities: CapabilityGraph — full provenance chain
- memory_state: SemanticMemoryState — references to L1/L2/L3 tiers
- trust_level: TrustScore — dynamic score based on behavior history
- framework_adapter: RuntimeAdapterRef — which framework adapter translates for this agent
- active_tasks: Set<CTRef> — currently running CognitiveTasks
- communication_protocols: Set<ProtocolID> — declared IPC protocols (ReAct, structured-data, event-stream)
- resource_quota: AgentQuota — total budget across all tasks
- lifecycle_config: LifecycleConfig — health check endpoint, restart policy, dependency ordering

**AgentCrew Properties (Section 2.3):**
- id: CrewID — unique crew identifier
- mission: MissionSpec — typed objective statement
- members: Set<AgentRef> — current member agents
- coordinator: AgentRef — designated coordinator agent (must be a member)
- collective_budget: ResourceQuota — shared resource budget across all members
- shared_memory: L3MemoryRef — crew-wide shared knowledge region
- scheduling_affinity: AffinityPolicy — scheduler hint: prefer co-scheduling crew members to same NUMA node

## Dependencies
- **Blocked by:** Nothing — this is the foundational work for all other streams
- **Blocking:** All subsequent kernel work (Weeks 2-6 and beyond depend on type-safe domain model); all services work (L1 services must implement against these types); all runtime work (L2 adapters must translate to these types); all SDK work (L3 SDKs expose these types)

## Acceptance Criteria
- [ ] All Rust types compile without warnings
- [ ] All 6 CognitiveTask invariants have compile-time or runtime checks that cannot be bypassed
- [ ] Property access is type-safe (no string keys, no unsafe casts)
- [ ] Every property has documentation comments referencing the engineering plan section
- [ ] Domain model review meeting completed with all 10 engineers signing off
- [ ] Zero unresolved questions about entity semantics or property types

## Design Principles Alignment
- **P2 — Cognitive Primitives as Kernel Abstractions:** CognitiveTask replaces POSIX process; Agent replaces user account; AgentCrew replaces process group
- **P3 — Capability-Based Security from Day Zero:** CapabilitySet property with typed enforcement
- **P5 — Observable by Default:** trace_log and checkpoint_refs properties enable full cognitive traceability
- **P8 — Fault-Tolerant by Design:** checkpoint_refs and watchdog_config properties provide checkpointing and watchdog foundations
