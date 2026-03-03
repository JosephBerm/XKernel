# Week 9 Deliverable: Membrane Pattern for Sandbox Boundaries (Phase 1)

**Engineer 2: Kernel Capability Engine & Security**
**Objective:** Implement Membrane pattern for transparent sandbox boundaries. Enable bulk attenuation and revocation of all capabilities when agent crosses sandbox boundary. Integrate with AgentCrew shared memory.

---

## 1. Membrane Abstraction

The Membrane is a transparent wrapper around a set of capabilities that enforces constraints at invocation time without modifying the underlying capability objects.

**Design Principles:**
- **Transparency:** Agents reference capabilities by original handle; kernel intercepts transparently
- **Lightweight:** <100 bytes per wrapped capability
- **Lifecycle:** Created on sandbox entry, destroyed on exit
- **Composability:** Multiple membranes can wrap the same capability chain

**Core Semantics:**
```
Membrane<Cap_0, Cap_1, ..., Cap_n> {
  wrapped_caps: HashMap<CapHandle, WrappedCap>,
  policies: Vec<ConstraintPolicy>,
  created_at: Timestamp,
  sandbox_id: SandboxId,
}
```

Each wrapped capability maintains a reference to its original CapChain provenance, enabling audit trails and cascading revocation.

---

## 2. Bulk Attenuation

`Membrane.attenuate(constraint)` applies a constraint uniformly to ALL wrapped capabilities. The operation creates derived capabilities that inherit the original CapChain provenance while adding new constraint metadata.

**Supported Constraint Types:**
- `reduce_ops` — Restrict to specific operations (e.g., read-only)
- `time_bound` — Enforce deadline (e.g., 1 hour expiration)
- `rate_limit` — Throttle invocations (e.g., 100 ops/sec)
- `data_volume_limit` — Cap data transfer (e.g., 1GB max)

**Performance Target:** Attenuation of 100 capabilities completes in <100µs.

---

## 3. Bulk Revocation

`Membrane.revoke()` is a single atomic operation that invalidates ALL wrapped capabilities simultaneously. The operation:
- Marks all wrapped caps as revoked
- Dispatches `SIG_CAPREVOKED` signal
- Cascades revocation to any delegated capabilities
- Completes in <10µs for 100 capabilities

**Cascade Semantics:** If capability A delegates to B, and A is revoked through the membrane, B becomes unusable (even if held by another agent).

---

## 4. Membrane Policy Language

A DSL for expressing sandbox-wide policies:

```
sandbox(sandbox_name) -> {
  constraint_1,
  constraint_2,
  ...
}
```

**Example Policies:**
```
sandbox(gpt4_inference) -> {
  time_bound(1h),
  read_only,
  rate_limit(1000_per_sec)
}

sandbox(data_processing) -> {
  data_volume_limit(5GB),
  time_bound(30m),
  reduce_ops([read, write])
}
```

**Composition:** Constraints are conjunctive (all must be satisfied for invocation to succeed).

---

## 5. AgentCrew Shared Memory Integration

Membranes control access to shared memory by enforcing per-agent permissions:

**Shared Memory Structure:**
```
shared_memory["context"] = {
  resource_1: {
    owner: agent_id,
    readers: [agent_id, ...],
    writers: [agent_id, ...],
    access_log: [(timestamp, agent, operation), ...]
  },
  ...
}
```

**Membrane-Controlled Access:**
- When agent A grants memory access to agent B, a capability is delegated
- That capability is wrapped by B's sandbox membrane
- Membrane policies restrict read/write/none operations
- Revoke → Mutual isolation (delegated cap becomes invalid)

**Example:**
```rust
// Agent A grants read access to Agent B
agent_a.grant_memory_access(
  target_agent: B,
  resource: "shared_context",
  permission: ReadOnly,
  duration: 1h
);

// Internally: creates capability, wraps in B's membrane with policies
```

---

## 6. Transparent Invocation

Agents invoke capabilities through a uniform interface:

```rust
kernel.capability_invoke(cap_handle, operation, args)
```

**Transparent Processing:**
1. Kernel checks if `cap_handle` is wrapped by a membrane
2. If wrapped, validates against all membrane constraints
3. If valid, invokes underlying capability
4. Returns result or constraint violation error

**Performance Target:** <5% overhead vs. direct invocation.

---

## 7. Lifecycle Management

**Creation (Sandbox Entry):**
- New sandbox created → kernel instantiates Membrane
- All capabilities granted to sandbox are wrapped immediately
- `created_at` timestamp recorded for auditing

**Update (Shared Memory Grant):**
- When capability is delegated → new WrappedCap created in delegatee's membrane
- Policies inherited from grantor's membrane, may be further attenuated

**Destruction (Sandbox Exit):**
- Sandbox termination → Membrane destroyed
- All wrapped caps marked invalid
- Audit log persisted (immutable)

---

## 8. Rust Implementation

```rust
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Constraint types for bulk attenuation
#[derive(Clone, Debug)]
pub enum Constraint {
    ReduceOps(Vec<Operation>),
    TimeBound(Duration),
    RateLimit(u32),              // ops per second
    DataVolumeLimit(u64),         // bytes
}

/// Operations that can be restricted
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Operation {
    Read,
    Write,
    Execute,
    Delegate,
}

/// Wrapped capability with original provenance
#[derive(Clone, Debug)]
struct WrappedCap {
    original_handle: CapHandle,
    cap_chain: Arc<CapChain>,
    invocation_count: u64,
    last_invocation: Option<Instant>,
    constraints: Vec<Constraint>,
}

/// Membrane pattern wrapper
#[derive(Clone)]
pub struct Membrane {
    membrane_id: String,
    sandbox_id: String,
    wrapped_caps: Arc<std::sync::RwLock<HashMap<String, WrappedCap>>>,
    policies: Vec<ConstraintPolicy>,
    created_at: Instant,
    is_revoked: Arc<std::sync::atomic::AtomicBool>,
}

/// Constraint policy with DSL semantics
#[derive(Clone, Debug)]
pub struct ConstraintPolicy {
    name: String,
    constraints: Vec<Constraint>,
}

/// Bulk attenuation engine
pub struct BulkAttenuator;

impl BulkAttenuator {
    /// Apply constraint to all wrapped capabilities
    pub fn attenuate(
        membrane: &mut Membrane,
        constraint: Constraint,
    ) -> Result<(), String> {
        if membrane.is_revoked.load(std::sync::atomic::Ordering::SeqCst) {
            return Err("Membrane revoked".to_string());
        }

        let mut caps = membrane.wrapped_caps.write().unwrap();
        let start = Instant::now();

        for (_, wrapped_cap) in caps.iter_mut() {
            wrapped_cap.constraints.push(constraint.clone());
        }

        let elapsed = start.elapsed();
        if elapsed.as_micros() > 100 {
            eprintln!("Warning: attenuation took {}µs", elapsed.as_micros());
        }

        Ok(())
    }

    /// Validate invocation against all constraints
    pub fn validate_invocation(
        wrapped_cap: &WrappedCap,
        operation: Operation,
    ) -> Result<(), String> {
        for constraint in &wrapped_cap.constraints {
            match constraint {
                Constraint::ReduceOps(allowed_ops) => {
                    if !allowed_ops.contains(&operation) {
                        return Err(format!("Operation {:?} not allowed", operation));
                    }
                }
                Constraint::TimeBound(duration) => {
                    let elapsed = Instant::now()
                        .duration_since(wrapped_cap.cap_chain.created_at);
                    if elapsed > *duration {
                        return Err("Time bound exceeded".to_string());
                    }
                }
                Constraint::RateLimit(ops_per_sec) => {
                    if let Some(last) = wrapped_cap.last_invocation {
                        let elapsed_ms =
                            Instant::now().duration_since(last).as_millis();
                        let min_interval_ms = 1000 / (*ops_per_sec as u128);
                        if elapsed_ms < min_interval_ms {
                            return Err("Rate limit exceeded".to_string());
                        }
                    }
                }
                Constraint::DataVolumeLimit(_limit) => {
                    // Validated at invocation result time
                }
            }
        }
        Ok(())
    }
}

/// Bulk revocation engine
pub struct BulkRevoker;

impl BulkRevoker {
    /// Atomically revoke all capabilities in membrane
    pub fn revoke(membrane: &Membrane) -> Result<(), String> {
        let start = Instant::now();

        // Mark membrane as revoked
        membrane.is_revoked.store(true, std::sync::atomic::Ordering::SeqCst);

        // Invalidate all wrapped caps
        let mut caps = membrane.wrapped_caps.write().unwrap();
        caps.clear();

        // Dispatch signal (simulated)
        Self::dispatch_signal(membrane, "SIG_CAPREVOKED");

        let elapsed = start.elapsed();
        if elapsed.as_nanos() > 10_000 {
            eprintln!("Warning: revocation took {}ns", elapsed.as_nanos());
        }

        Ok(())
    }

    fn dispatch_signal(membrane: &Membrane, signal: &str) {
        // In production: dispatch to capability revocation cascade system
        println!(
            "Dispatched {} for membrane {} (sandbox {})",
            signal, membrane.membrane_id, membrane.sandbox_id
        );
    }
}

/// Membrane policy language (DSL builder)
pub struct MembranePolicy {
    sandbox_name: String,
    constraints: Vec<Constraint>,
}

impl MembranePolicy {
    pub fn new(sandbox_name: impl Into<String>) -> Self {
        Self {
            sandbox_name: sandbox_name.into(),
            constraints: Vec::new(),
        }
    }

    pub fn time_bound(mut self, duration: Duration) -> Self {
        self.constraints.push(Constraint::TimeBound(duration));
        self
    }

    pub fn read_only(mut self) -> Self {
        self.constraints
            .push(Constraint::ReduceOps(vec![Operation::Read]));
        self
    }

    pub fn rate_limit(mut self, ops_per_sec: u32) -> Self {
        self.constraints.push(Constraint::RateLimit(ops_per_sec));
        self
    }

    pub fn data_volume_limit(mut self, bytes: u64) -> Self {
        self.constraints.push(Constraint::DataVolumeLimit(bytes));
        self
    }

    pub fn build(self) -> ConstraintPolicy {
        ConstraintPolicy {
            name: self.sandbox_name,
            constraints: self.constraints,
        }
    }
}

impl Membrane {
    /// Create new membrane for sandbox
    pub fn create(sandbox_id: impl Into<String>) -> Self {
        let sandbox_id = sandbox_id.into();
        Self {
            membrane_id: format!("mem_{}", uuid::Uuid::new_v4()),
            sandbox_id,
            wrapped_caps: Arc::new(std::sync::RwLock::new(HashMap::new())),
            policies: Vec::new(),
            created_at: Instant::now(),
            is_revoked: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Add policy to membrane
    pub fn add_policy(&mut self, policy: ConstraintPolicy) {
        self.policies.push(policy);
    }

    /// Wrap capability
    pub fn wrap_capability(
        &self,
        handle: String,
        cap_chain: Arc<CapChain>,
    ) -> Result<(), String> {
        if self.is_revoked.load(std::sync::atomic::Ordering::SeqCst) {
            return Err("Membrane revoked".to_string());
        }

        let wrapped = WrappedCap {
            original_handle: CapHandle(handle.clone()),
            cap_chain,
            invocation_count: 0,
            last_invocation: None,
            constraints: Vec::new(),
        };

        let mut caps = self.wrapped_caps.write().unwrap();
        caps.insert(handle, wrapped);
        Ok(())
    }

    /// Invoke capability through membrane
    pub fn invoke(
        &self,
        handle: &str,
        operation: Operation,
    ) -> Result<String, String> {
        if self.is_revoked.load(std::sync::atomic::Ordering::SeqCst) {
            return Err("Membrane revoked".to_string());
        }

        let mut caps = self.wrapped_caps.write().unwrap();
        let wrapped = caps
            .get_mut(handle)
            .ok_or("Capability not found in membrane")?;

        // Validate against constraints
        BulkAttenuator::validate_invocation(wrapped, operation)?;

        // Update metadata
        wrapped.invocation_count += 1;
        wrapped.last_invocation = Some(Instant::now());

        Ok(format!(
            "Invoked {} (count: {})",
            handle, wrapped.invocation_count
        ))
    }

    /// Get membrane stats
    pub fn stats(&self) -> (usize, bool, Duration) {
        let caps = self.wrapped_caps.read().unwrap();
        let elapsed = self.created_at.elapsed();
        (
            caps.len(),
            self.is_revoked.load(std::sync::atomic::Ordering::SeqCst),
            elapsed,
        )
    }
}

// Placeholder types
#[derive(Clone, Debug)]
pub struct CapHandle(String);

#[derive(Clone, Debug)]
pub struct CapChain {
    created_at: Instant,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_membrane_create_destroy() {
        let membrane = Membrane::create("test_sandbox");
        assert_eq!(membrane.sandbox_id, "test_sandbox");
        let (count, revoked, _elapsed) = membrane.stats();
        assert_eq!(count, 0);
        assert!(!revoked);
    }

    #[test]
    fn test_wrap_capability() {
        let membrane = Membrane::create("test");
        let cap_chain = Arc::new(CapChain {
            created_at: Instant::now(),
        });
        membrane
            .wrap_capability("cap1".to_string(), cap_chain)
            .unwrap();
        let (count, _, _) = membrane.stats();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_bulk_attenuation() {
        let mut membrane = Membrane::create("test");
        let cap_chain = Arc::new(CapChain {
            created_at: Instant::now(),
        });
        membrane
            .wrap_capability("cap1".to_string(), cap_chain.clone())
            .unwrap();
        membrane
            .wrap_capability("cap2".to_string(), cap_chain)
            .unwrap();

        BulkAttenuator::attenuate(&mut membrane, Constraint::ReadOnly(vec![Operation::Read]))
            .unwrap();

        let (count, _, _) = membrane.stats();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_bulk_revocation() {
        let membrane = Membrane::create("test");
        let cap_chain = Arc::new(CapChain {
            created_at: Instant::now(),
        });
        membrane
            .wrap_capability("cap1".to_string(), cap_chain)
            .unwrap();

        BulkRevoker::revoke(&membrane).unwrap();
        let (_, revoked, _) = membrane.stats();
        assert!(revoked);
    }

    #[test]
    fn test_policy_builder() {
        let policy = MembranePolicy::new("gpt4_inference")
            .time_bound(Duration::from_secs(3600))
            .read_only()
            .build();

        assert_eq!(policy.name, "gpt4_inference");
        assert_eq!(policy.constraints.len(), 2);
    }

    #[test]
    fn test_transparent_invocation() {
        let membrane = Membrane::create("test");
        let cap_chain = Arc::new(CapChain {
            created_at: Instant::now(),
        });
        membrane
            .wrap_capability("cap1".to_string(), cap_chain)
            .unwrap();

        let result = membrane.invoke("cap1", Operation::Read);
        assert!(result.is_ok());
    }

    #[test]
    fn test_invocation_after_revocation() {
        let membrane = Membrane::create("test");
        let cap_chain = Arc::new(CapChain {
            created_at: Instant::now(),
        });
        membrane
            .wrap_capability("cap1".to_string(), cap_chain)
            .unwrap();

        BulkRevoker::revoke(&membrane).unwrap();
        let result = membrane.invoke("cap1", Operation::Read);
        assert!(result.is_err());
    }

    #[test]
    fn test_rate_limit_constraint() {
        let wrapped = WrappedCap {
            original_handle: CapHandle("cap1".to_string()),
            cap_chain: Arc::new(CapChain {
                created_at: Instant::now(),
            }),
            invocation_count: 0,
            last_invocation: Some(Instant::now()),
            constraints: vec![Constraint::RateLimit(10)],
        };

        let result = BulkAttenuator::validate_invocation(&wrapped, Operation::Read);
        assert!(result.is_err());
    }
}
```

---

## 9. Testing Strategy

**Coverage Target:** >95% line coverage, 120+ tests

**Test Categories:**

1. **Lifecycle Tests (15 tests)**
   - Create/destroy <1000ns
   - Policy application on creation
   - Sandbox entry/exit scenarios

2. **Bulk Attenuation Tests (30 tests)**
   - Single constraint attenuation
   - Multiple constraint composition
   - Attenuation of 100+ capabilities
   - Constraint validation at invocation

3. **Bulk Revocation Tests (25 tests)**
   - Atomic revocation
   - Cascade to delegations
   - <10µs performance for 100 caps
   - Signal dispatch verification

4. **Policy Language Tests (20 tests)**
   - DSL parsing and building
   - Constraint conjunction
   - Policy enforcement

5. **AgentCrew Integration Tests (20 tests)**
   - Shared memory access control
   - Per-agent permission validation
   - Revocation → mutual isolation

6. **Transparent Invocation Tests (10 tests)**
   - <5% overhead measurement
   - Constraint checking at invoke time
   - Error propagation

---

## 10. Performance Targets (Summary)

| Operation | Target | Notes |
|-----------|--------|-------|
| Membrane creation | <1000ns | Per sandbox |
| Membrane destruction | <1000ns | Per sandbox |
| Bulk attenuation (100 caps) | <100µs | Constraint application |
| Bulk revocation (100 caps) | <10µs | Atomic invalidation |
| Transparent invocation overhead | <5% | vs. direct call |
| Memory per wrapped cap | <100 bytes | Minimal overhead |

---

## 11. Integration Points

- **Kernel Scheduler:** Membrane creation hook on sandbox entry
- **Capability Manager:** CapChain provenance tracking
- **AgentCrew:** Shared memory permission enforcement
- **Audit System:** Lifecycle and revocation event logging
- **Signal Dispatcher:** SIG_CAPREVOKED cascade handling

---

## 12. Future Work (Phase 2)

- Delegation policies (transitive attenuation)
- Conditional constraints (time-dependent, context-aware)
- Membrane composition (nested sandboxes)
- Performance optimization (lock-free data structures)
- Formal verification of revocation cascades
