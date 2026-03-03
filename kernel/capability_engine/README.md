# Capability Engine

> **Crate:** [`capability_engine`](Cargo.toml)
> **Stream:** 1 — Kernel & Scheduler
> **Layer:** L0 (Microkernel)
> **Owner:** Engineer 01
> **Status:** Active

---

## 1. Purpose & Scope

Implements Cognitive Substrate's capability-based security model — an object-capability approach inspired by seL4. Manages unforgeable capability tokens, enforces delegation rules, and prevents unauthorized privilege escalation across agents. Capabilities are the foundation of isolation between agents and crews.

**Key Responsibilities:**
- Capability token generation and validation
- Capability delegation and revocation
- Isolation boundary enforcement (agent ↔ agent, crew ↔ crew)
- Capability graph management and analysis
- Delegation chain tracking and provenance

**In Scope:**
- Capability object representation and API
- Delegation semantics and rules
- Revocation mechanisms and impact analysis
- Capability graph queries and traversal

**Out of Scope:**
- Actual resource enforcement (handled by service modules)
- Scheduling decisions based on capabilities (handled by scheduler)
- Capability revocation execution (handled by kernel signal system)

---

## 2. Engineering Plan Reference

**Relevant Sections:**
- Section 2.2: Capability domain entity — Definition and invariants
- Section 4.1: Capability-Based Security Model — Architecture
- Section 4.3: Isolation Enforcement — Crew/agent isolation

**Domain Model Entities Involved:**
- **Capability** — Core entity managed by this module
- **MandatoryCapabilityPolicy** — System-wide unoverridable rules
- **CognitiveTask** — Inherit capability subsets from parent Agent
- **Agent** — Own capabilities granted by parent Agent
- **AgentCrew** — Shared capability pool for crew members

See `docs/domain_model_deep_dive.md` for comprehensive entity definitions.

---

## 3. Architecture & Design

### 3.1 High-Level Architecture

```
┌────────────────────────────────────────────────┐
│  Capability API: grant(), delegate(), revoke() │
└────────────────────────────────────────────────┘
             ↓
┌────────────────────────────────────────────────┐
│  Capability Graph Manager                      │
│  (Nodes: agents, Edges: delegation)            │
└────────────────────────────────────────────────┘
             ↓                      ↓
    ┌─────────────────┐   ┌────────────────┐
    │ Delegation      │   │ Revocation     │
    │ Rules Engine    │   │ Propagation    │
    └─────────────────┘   └────────────────┘
             ↓                      ↓
    ┌────────────────────────────────────────┐
    │ Mandatory Capability Policy Enforcer   │
    │ (System-wide immutable rules)          │
    └────────────────────────────────────────┘
```

### 3.2 Key Invariants

1. **Capability Unforgability**: Capabilities cannot be created without proper authority
   - Enforced: Compile-time opaque type + cryptographic verification
   - Impact: Prevents capability spoofing attacks

2. **Delegation Subset Property**: Delegated capabilities ⊆ delegator's capabilities
   - Enforced: Runtime checks on every delegation
   - Impact: Prevents privilege escalation through delegation

3. **Revocation Completeness**: When a capability is revoked, all transitive delegations are revoked
   - Enforced: Graph traversal on revocation + signal propagation
   - Impact: Ensures compromised capability can be contained

4. **Mandatory Policy Enforcement**: System policies cannot be overridden by user code
   - Enforced: Compile-time + runtime checks on every operation
   - Impact: Guarantees system-wide isolation invariants

---

## 4. Dependencies

### 4.1 Upstream Dependencies

| Crate | Type | Layer | Why |
|-------|------|-------|-----|
| `ct_lifecycle` | Internal | L0 | Query CT capability set |
| `ipc_signals_exceptions` | Internal | L0 | Send revocation signals |
| `domain_types` | Internal | L0 | Shared types (Capability, etc.) |

---

## 5. Public API Surface

```rust
/// Unique identifier for a capability
pub struct CapabilityId(Ulid);

/// Represents an unforgeable capability token
pub struct Capability {
    pub id: CapabilityId,
    pub owner: EntityId,  // Agent or Crew that owns this capability
    pub kind: CapabilityKind,
    pub delegated_from: Option<CapabilityId>,  // Provenance
}

pub enum CapabilityKind {
    ReadMemory { tier: MemoryTier },
    WriteMemory { tier: MemoryTier },
    SpawnTask,
    ManageGpu,
    ManageTelemetry,
}

/// Delegate a capability (create a derived capability)
pub fn cap_delegate(
    from: &Capability,
    to: EntityId,
    kind: CapabilityKind,
) -> CsResult<Capability>;

/// Revoke a capability and all derived capabilities
pub fn cap_revoke(cap: &Capability) -> CsResult<()>;

/// Query capability graph
pub fn cap_query_graph(owner: EntityId) -> CsResult<CapabilityGraph>;

/// Verify capability against mandatory policy
pub fn cap_verify_policy(cap: &Capability) -> CsResult<()>;
```

---

## 6. Building & Testing

```bash
cargo build -p capability_engine
cargo test -p capability_engine
cargo doc -p capability_engine --open
```

**Key Test Scenarios:**
1. Capability delegation tests — Verify subset property
2. Revocation propagation tests — Verify transitive revocation
3. Policy enforcement tests — Verify mandatory policies
4. Capability graph queries — Verify delegation chain tracing

---

## 7. Design Decisions Log

### 7.1 "Object-Capability Model vs. ACL?"

**Decision:** Use object-capability (OCap) model instead of Access Control Lists (ACL).

**Alternatives:**
1. ACL-based — Central policy server validates every access
2. Role-based (RBAC) — Assign roles to agents, check role permissions

**Rationale:**
- OCap is naturally compositional — can delegate without central authority
- No revocation coordination needed for most cases (only on explicit revoke)
- Better matches distributed, decentralized agent networks
- seL4 kernel success with OCap provides validation

**Date:** 2026-03-01
**Author:** Engineer 01

### 7.2 "Eager Revocation vs. Lazy?"

**Decision:** Eager revocation — immediately invalidate all transitive capabilities.

**Alternatives:**
1. Lazy revocation — Mark revoked, but allow continuing execution until checked
2. Staged revocation — Notify agents, wait for ack, then revoke

**Rationale:**
- Security: Compromised capability immediately contained
- Simplicity: No complex state machines for revocation stages
- Predictability: Agents know they're revoked immediately

**Date:** 2026-03-01
**Author:** Engineer 01

---

## 8. Performance Characteristics

| Operation | Complexity | Notes |
|-----------|-----------|-------|
| `cap_delegate` | O(log n) | Graph lookup + policy check |
| `cap_revoke` | O(n) | n = transitive delegations |
| `cap_verify_policy` | O(1) | Constant-time policy check |
| `cap_query_graph` | O(n + e) | n = nodes, e = edges in graph |

---

## 9. Common Pitfalls & Troubleshooting

**Mistake 1: Trying to delegate beyond your capability**
```rust
// ✗ WRONG: Delegating memory tier you don't have
let my_cap = Capability { kind: ReadMemory(L1), ... };
let delegated = cap_delegate(&my_cap, other_id, ReadMemory(L3))?;  // ERROR
```

**Mistake 2: Assuming revocation is async**
```rust
// ✗ WRONG: Using capability after revocation request
cap_revoke(&cap)?;
// cap is still valid here — need to wait for signal

// ✓ RIGHT: Capability invalid after revoke
cap_revoke(&cap)?;
// Any subsequent use of cap fails immediately
```

---

## 10. Integration Points

| Module | Integration | Protocol |
|--------|-----------|----------|
| `ct_lifecycle` | Validate CT capability inheritance | Direct call |
| `ipc_signals_exceptions` | Distribute revocation signals | SemanticChannel |
| `gpu_accelerator` (L1) | Verify GPU access capability | CSCI wrapper |

---

## 11. Future Roadmap

**Planned Improvements:**
- Capability attenuation — Reduce capability's scope without full revocation
- Capability lending — Temporary capability grant with automatic revocation
- Capability market — Dynamic capability trading between agents

**Technical Debt:**
- Optimize revocation graph traversal for large capability graphs (1000+ capabilities)

---

## 12. References

- [`docs/domain_model_deep_dive.md`](../../docs/domain_model_deep_dive.md)
- **Object-Capability Model:** https://en.wikipedia.org/wiki/Capability-based_security
- **seL4 Capability System:** https://sel4.systems/papers/sel4-focs2009.pdf

---

**README Version:** 1.0
**Last Updated:** 2026-03-01
**Owner:** Engineer 01
