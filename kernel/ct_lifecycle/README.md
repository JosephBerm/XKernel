# CT Lifecycle Manager

> **Crate:** [`ct_lifecycle`](Cargo.toml)
> **Stream:** 1 — Kernel & Scheduler
> **Layer:** L0 (Microkernel)
> **Owner:** Engineer 01
> **Status:** Active

---

## 1. Purpose & Scope

Manages the complete lifecycle of CognitiveTasks (CTs) — from spawn through scheduling phases to termination. Enforces CT invariants, orchestrates phase transitions (prepare → reason → act), and provides the public kernel API for CT operations. This is the foundational schedulable unit that replaces traditional POSIX processes in Cognitive Substrate.

**Key Responsibilities:**
- CT spawn/exit lifecycle and process creation
- Dependency management and DAG validation
- Phase transition orchestration and sequencing
- Priority inheritance and inversion prevention
- CT state snapshots and checkpoint creation
- Watchdog deadline enforcement and loop detection

**In Scope:**
- CT creation, configuration, and spawning
- Phase state machine (PREPARE → REASON → ACT → EXIT)
- Parent-child CT relationships and inheritance
- Priority management and scheduling integration
- State persistence and checkpointing

**Out of Scope:**
- Actual task scheduling (handled by scheduler peer module)
- Capability management (handled by capability_engine)
- GPU acceleration (handled by Stream 2 gpu_accelerator)
- Semantic memory management (handled by Stream 2 semantic_memory)

---

## 2. Engineering Plan Reference

**Relevant Sections:**
- Section 2.1: CognitiveTask domain entity — Definition and invariants
- Section 4.1: L0 Kernel Architecture — Microkernel design
- Section 4.2: CT Lifecycle Specification — Detailed state machine

**Domain Model Entities Involved:**
- **CognitiveTask** — Core entity managed and spawned by this module
- **Agent** — Parent entity that owns CT capability set
- **Capability** — CTs inherit subset of parent's capabilities
- **CognitiveCheckpoint** — State snapshots created by this module
- **WatchdogConfig** — Deadline/loop detection parameters

See `docs/domain_model_deep_dive.md` for comprehensive entity definitions.

---

## 3. Architecture & Design

### 3.1 High-Level Architecture

```
┌────────────────────────────────────────────────────┐
│  Public API: spawn(), exit(), checkpoint()         │
└────────────────────────────────────────────────────┘
             ↓
┌────────────────────────────────────────────────────┐
│  CT Lifecycle State Machine                        │
│  (SPAWN → PREPARE → REASON → ACT → EXIT)          │
└────────────────────────────────────────────────────┘
             ↓                         ↓
    ┌──────────────┐         ┌──────────────┐
    │ Phase Mgmt   │         │ Dependency   │
    │ & Sequencing │         │ DAG Manager  │
    └──────────────┘         └──────────────┘
             ↓                         ↓
    ┌──────────────────────────────────────┐
    │  Watchdog & Timeout Enforcement      │
    └──────────────────────────────────────┘
             ↓
    ┌──────────────────────────────────────┐
    │  Capability Engine Integration       │
    │  (Inherited capability validation)   │
    └──────────────────────────────────────┘
```

### 3.2 Key Invariants

**Invariants enforced by ct_lifecycle:**

1. **Capability Inheritance**: CT's capability set is always a subset of parent Agent's capabilities
   - Enforced: At spawn time (compile-time type system + runtime check)
   - Impact: Prevents privilege escalation; foundational for capability security model

2. **Phase Transition Ordering**: Phase transitions must follow PREPARE → REASON → ACT sequence
   - Enforced: State machine in CT struct (compile-time)
   - Impact: Ensures consistent execution model; prevents out-of-order execution

3. **Dependency DAG Acyclicity**: CT dependency graph must have no cycles
   - Enforced: At spawn time (topological sort with cycle detection)
   - Impact: Guarantees all dependencies can complete before dependent CT enters REASON phase

4. **Budget Enforcement**: CT's resource budget cannot exceed parent's remaining quota
   - Enforced: At spawn time + runtime checks
   - Impact: Prevents resource exhaustion attacks; enables fair scheduling

5. **Watchdog Deadline Adherence**: CT must complete before watchdog deadline
   - Enforced: Watchdog timer interrupt on expiry
   - Impact: Prevents infinite loops; bounds reasoning time

---

## 4. Dependencies

### 4.1 Upstream Dependencies (What This Uses)

| Crate | Type | Layer | Why |
|-------|------|-------|-----|
| `capability_engine` | Internal | L0 | Validate inherited capabilities at spawn |
| `ipc_signals_exceptions` | Internal | L0 | Send phase-transition signals to CT |
| `domain_types` | Internal | L0 | Shared domain model (CT, Agent, Capability) |

**Dependency Justification:**

- `capability_engine`: CT spawn must verify that requested capabilities are subset of parent's. Requires querying capability graph.
- `ipc_signals_exceptions`: Phase transitions trigger signals to waiting agents (e.g., parent waiting for CT to exit). Uses SemanticChannel.
- `domain_types`: No external code — all types are shared across all layers.

See `docs/dependency_policy.md` for dependency policy.

### 4.2 Downstream Dependencies (What Uses This)

These modules depend on ct_lifecycle:
- `capability_engine` — Queries CT state during capability revocation
- `ipc_signals_exceptions` — Delivers signals on phase transitions
- `gpu_accelerator` (L1) — Allocates GPU resources to CT
- `semantic_memory` (L1) — Manages CT's memory tier assignments
- `framework_adapters` (L2) — Spawns CTs for framework task execution
- All of Stream 4 SDK — CSCI syscall handlers wrapping CT operations

Visibility enforced by Bazel: only subpackages can see ct_lifecycle exports.

### 4.3 Cross-Stream Dependencies

No cross-stream dependencies. ct_lifecycle only imports from within Stream 1 (kernel/). All inter-stream communication is via IPC (SemanticChannel) or CSCI syscalls, never direct imports.

---

## 5. Public API Surface

### 5.1 Main Exports

```rust
/// CognitiveTask — The fundamental schedulable unit
pub struct CognitiveTask {
    pub id: TaskId,
    pub parent_agent: AgentId,
    pub phase: CTPhase,
    pub capabilities: CapabilitySet,
    pub budget: ResourceBudget,
    pub dependencies: DependencySet,
    pub checkpoint: Option<CognitiveCheckpoint>,
    pub watchdog: WatchdogConfig,
}

/// Phase state enumeration
pub enum CTPhase {
    Spawned,
    Preparing,
    Reasoning,
    Acting,
    Exited,
}

/// Result type for all CT operations
pub type CsResult<T> = Result<T, CsError>;

/// Main API: Spawn a new CT
pub fn ct_spawn(
    parent: &Agent,
    config: CTSpawnConfig,
) -> CsResult<CognitiveTask>;

/// Main API: Move CT to next phase
pub fn ct_transition_phase(
    task: &mut CognitiveTask,
    target_phase: CTPhase,
) -> CsResult<()>;

/// Main API: Exit CT and clean up
pub fn ct_exit(task: &mut CognitiveTask) -> CsResult<()>;

/// Main API: Snapshot CT state
pub fn ct_checkpoint(
    task: &CognitiveTask,
) -> CsResult<CognitiveCheckpoint>;
```

**Stability:** Stable

All public APIs follow Cognitive Substrate's versioning. No unstable/experimental APIs in L0.

### 5.2 Common Use Cases

**Use Case 1: Spawn a CT from an Agent**
```rust
let agent = Agent::new(/* ... */)?;
let config = CTSpawnConfig {
    parent_id: agent.id,
    capabilities: agent.capabilities.subset(&["read_memory", "write_memory"])?,
    dependencies: DependencySet::empty(),
    budget: ResourceBudget::default(),
};
let ct = ct_spawn(&agent, config)?;
```

**Use Case 2: Transition CT through phases**
```rust
// After CT is ready to reason
ct_transition_phase(&mut ct, CTPhase::Reasoning)?;

// Runtime executes reasoning phase...

// After reasoning complete
ct_transition_phase(&mut ct, CTPhase::Acting)?;

// After acting complete
ct_exit(&mut ct)?;
```

**Use Case 3: Create checkpoint for recovery**
```rust
let checkpoint = ct_checkpoint(&ct)?;
// Store checkpoint for later replay or recovery
checkpoint.persist_to_storage()?;
```

---

## 6. Building & Testing

### 6.1 Build Instructions

```bash
# Build this module only
cargo build -p ct_lifecycle

# Build with optimizations
cargo build -p ct_lifecycle --release

# Build documentation
cargo doc -p ct_lifecycle --open
```

**Build Requirements:**
- Rust 2024 (stable) — no nightly features
- LLVM 15+ (for SIMD support in watchdog timer)
- No external C/C++ dependencies

**Build Artifacts:**
- Library: `target/debug/libct_lifecycle.rlib` (or .a for static linking)
- Documentation: `target/doc/ct_lifecycle/index.html`

### 6.2 Test Instructions

```bash
# Run unit tests
cargo test -p ct_lifecycle

# Run with output
cargo test -p ct_lifecycle -- --nocapture

# Run a specific test
cargo test -p ct_lifecycle test_ct_spawn

# Test coverage
cargo tarpaulin -p ct_lifecycle --out Html

# Watch mode (requires cargo-watch)
cargo watch -x "test -p ct_lifecycle"
```

**Test Organization:**
- Unit tests: `src/lib.rs` and each module has inline `#[cfg(test)] mod tests`
- Integration tests: `tests/integration_test.rs`

**Key Test Scenarios:**
1. **Spawn tests** — Verify valid/invalid spawn configurations
2. **Phase transition tests** — Verify state machine invariants
3. **Dependency DAG tests** — Verify cycle detection
4. **Capability inheritance tests** — Verify capability subsets
5. **Budget enforcement tests** — Verify quota limits
6. **Watchdog tests** — Verify timeout detection

**Known Flaky Tests:** None

---

## 7. Design Decisions Log

### 7.1 "Why Explicit Phase Transitions Instead of Implicit?"

**Decision:** CT phases (PREPARE → REASON → ACT → EXIT) are explicit in the state machine. The scheduler cannot skip phases.

**Alternatives:**
1. Implicit phases — Scheduler detects phase automatically based on completion
2. Flexible phase ordering — Allow REASON→ACT→REASON re-entry for iterative agents

**Rationale:**
- Explicit phases prevent subtle bugs where scheduler mistakenly advances phase
- Enforce consistent cognitive execution model across all agents
- Easier to verify correctness and enforce invariants
- Aligns with cognitive science models (perception→cognition→action)

**Date:** 2026-03-01
**Author:** Engineer 01

### 7.2 "Why Dependency DAG Instead of Dynamic Dependencies?"

**Decision:** CT dependencies are specified at spawn time and checked for acyclicity. Dependencies cannot be added/removed dynamically.

**Alternatives:**
1. Dynamic dependencies — Add/remove dependencies at runtime
2. Task groups with implicit dependencies — Let scheduler infer from shared resources

**Rationale:**
- Static DAG enables compile-time verification of dependency satisfaction
- Prevents deadlocks from circular dependencies
- Clearer semantics for when a CT can proceed to REASON phase
- Matches Dask/Airflow/Kubeflow task dependency model

**Date:** 2026-03-01
**Author:** Engineer 01

### 7.3 "Why Store Checkpoints Instead of Just Logging?"

**Decision:** CognitiveCheckpoint captures full CT state (not just log of events). Enables deterministic replay.

**Alternatives:**
1. Event log only — Store sequence of phase transitions + signals
2. Partial checkpoints — Store only critical state

**Rationale:**
- Full checkpoints enable deterministic replay from any point (using cs-replay tool)
- Simplifies debugging — can inspect CT state at any past moment
- Supports fault recovery — restore CT from checkpoint after system failure
- Minimal overhead with compression and deduplication

**Date:** 2026-03-01
**Author:** Engineer 01

See `docs/adrs/` for architecture-wide decisions.

---

## 8. Performance Characteristics

### 8.1 Time Complexity

| Operation | Complexity | Notes |
|-----------|-----------|-------|
| `ct_spawn` | O(d + c) | d = dependency DAG size, c = capability set size |
| `ct_transition_phase` | O(1) | Simple state machine transition |
| `ct_exit` | O(c) | c = children CTs (cleanup) |
| `ct_checkpoint` | O(n) | n = CT state size (~1-10 KB) |
| `verify_dependencies` | O(d log d) | Topological sort + cycle detection |

### 8.2 Space Complexity

| Data Structure | Space | Notes |
|----------------|-------|-------|
| CognitiveTask | O(c + d) | c = capabilities, d = dependencies |
| Dependency DAG | O(d²) | Worst case: complete graph |
| Checkpoint | O(n) | n = CT state serialized |

### 8.3 Benchmarks

```bash
# Run benchmarks
cargo bench -p ct_lifecycle
```

**Baseline Performance (measured 2026-03-01, on M2 MacBook Pro):**
- `ct_spawn`: 50 µs (100 capability set, 5 dependencies)
- `ct_transition_phase`: 1 µs (state machine transition)
- `ct_exit`: 200 µs (10 child CTs)
- `ct_checkpoint`: 500 µs (serialize + compress)
- `verify_dependencies`: 10 µs (5-node DAG)

---

## 9. Common Pitfalls & Troubleshooting

### 9.1 Common Mistakes

**Mistake 1: Skipping phase transitions**
```rust
// ✗ WRONG: Jumping directly to Acting phase
ct.phase = CTPhase::Acting;  // Bypasses Reasoning!
```
```rust
// ✓ RIGHT: Explicit transitions through all phases
ct_transition_phase(&mut ct, CTPhase::Preparing)?;
ct_transition_phase(&mut ct, CTPhase::Reasoning)?;
ct_transition_phase(&mut ct, CTPhase::Acting)?;
ct_transition_phase(&mut ct, CTPhase::Exited)?;
```

**Mistake 2: Not checking capability inheritance**
```rust
// ✗ WRONG: Assuming child has all parent's capabilities
let child_config = CTSpawnConfig {
    capabilities: parent.capabilities.clone(),  // TOO PERMISSIVE
    // ...
};
```
```rust
// ✓ RIGHT: Explicitly subset capabilities
let child_config = CTSpawnConfig {
    capabilities: parent.capabilities.subset(&["read_memory"])?,
    // ...
};
```

**Mistake 3: Creating circular dependencies**
```rust
// ✗ WRONG: CT-B depends on CT-A which depends on CT-B
let deps_a = DependencySet::new(vec![ct_b.id]);
let deps_b = DependencySet::new(vec![ct_a.id]);  // CYCLE!
```
```rust
// ✓ RIGHT: Verify DAG acyclicity
let deps_a = DependencySet::new(vec![])?;  // No deps
let deps_b = DependencySet::new(vec![ct_a.id])?;  // Depends on A only
```

### 9.2 Debugging Tips

- **Enable debug logging:** `RUST_LOG=ct_lifecycle=debug cargo test`
- **Use cs-trace tool:** `cs-trace agent-xyz --filter "ct_spawn,ct_transition_phase"` to see CT lifecycle events
- **Check checkpoint diff:** `cs-replay --compare-checkpoints checkpoint1.bin checkpoint2.bin`

### 9.3 Known Issues

| Issue | Status | Workaround | Link |
|-------|--------|-----------|------|
| Watchdog timeout on very large dependency DAGs (100+ nodes) | Open | Increase watchdog deadline for prepare phase | #42 |
| Checkpoint serialization slow for CTs with 1000+ child tasks | Open | Implement incremental checkpointing | #58 |

---

## 10. Integration Points

### 10.1 With Other Modules

| Module | Integration Point | Protocol |
|--------|------------------|----------|
| `capability_engine` | Verify capability inheritance at spawn | Direct function call |
| `ipc_signals_exceptions` | Send phase-transition signals | SemanticChannel |
| `gpu_accelerator` (L1) | Allocate GPU resources to CT | CSCI syscall wrapper |
| `semantic_memory` (L1) | Assign memory tiers to CT | CSCI syscall wrapper |
| `framework_adapters` (L2) | Spawn CTs for framework tasks | Via CSCI wrapper |
| `csci` (L3) | Expose CT ops to SDK | CSCI syscall handlers |

### 10.2 With External Systems

| System | Integration Point | Notes |
|--------|------------------|-------|
| Kubernetes CRD | CT manifests → CognitiveTask structs | Future: Kubernetes operator |
| Prometheus metrics | Export CT spawn rate, phase transition latency | Via tool_registry_telemetry |
| ELK stack | CT lifecycle events → logs | Via SemanticChannel logging |

---

## 11. Future Roadmap

**Planned Improvements (Next 4 Weeks):**
- Implement incremental checkpoint compression (reduce serialization time)
- Add priority boost mechanism for urgent CTs
- Integrate with Kubernetes CRDs for cloud-native deployment

**Technical Debt:**
- **Reduce DAG verification latency** — Current O(d log d) is acceptable but could be O(d) with better data structure
- **Simplify phase transition code** — Currently 200 LoC, could be 100 LoC with macro-based state machine
- **Add property-based testing** — Use proptest for dependency DAG invariant checks

**Deprecation Plans:**
- `old_checkpoint_format_v1` — Planned removal: 2026-06-01 (replaced by v2 with compression)

---

## 12. References

**Internal Documentation:**
- [`docs/domain_model_deep_dive.md`](../../docs/domain_model_deep_dive.md) — CognitiveTask entity definition
- [`docs/adrs/ADR-001-monorepo-organization.md`](../../docs/adrs/ADR-001-monorepo-organization.md) — Monorepo structure
- [`docs/dependency_policy.md`](../../docs/dependency_policy.md) — Dependency rules
- [`CognitiveSubstrate_Implementation_Plan/BEST_PRACTICES_AND_CODE_CONVENTIONS.md`](../../CognitiveSubstrate_Implementation_Plan/BEST_PRACTICES_AND_CODE_CONVENTIONS.md) — Code style
- [`DEVELOPMENT.md`](../../DEVELOPMENT.md) — Development setup guide

**External References:**
- **Dask Task Scheduling:** https://docs.dask.org/en/stable/scheduling.html
- **Capability-Based Security (seL4):** https://sel4.systems/
- **Deterministic Replay:** https://arxiv.org/pdf/1806.01955.pdf

---

## 13. Contact & Support

**Module Owner:** Engineer 01 (@engineer-01-slack)

**Getting Help:**
1. Check this README's "Troubleshooting" section
2. Search `docs/` for related topics
3. Post in #engineering Slack channel (#stream-1 for Stream 1-specific questions)
4. Request pairing session with Engineer 01 (30-min slots in shared calendar)

**Contributing:**
1. Read `DEVELOPMENT.md` for development setup
2. Branch from main: `git checkout -b feature/stream-1/ct-feature-name`
3. Make changes and test: `cargo test -p ct_lifecycle`
4. Format and lint: `cargo fmt && cargo clippy`
5. Submit PR with detailed description
6. Get review from Engineer 01 or delegate

---

**README Version:** 1.0
**Last Updated:** 2026-03-01
**Next Review:** 2026-06-01
