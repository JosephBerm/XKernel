# Week 13 — Multi-Agent Capability Demo: Full Lifecycle & CPL Policy Integration

**Document Version:** 1.0
**Status:** Engineering Design
**Author:** Principal Software Engineer
**Date:** 2026-03-02
**Project:** XKernal Cognitive Substrate OS

---

## Executive Summary

Week 13 validates the complete capability lifecycle across a three-kernel AgentCrew cluster (K1, K2, K3) with grant, delegate, attenuate, and revoke semantics. This phase introduces **Cognitive Policy Language (CPL) integration with the Mandatory Policy Engine**, providing O(1) fast-path policy lookups via compiled decision tables and full policy evaluation on slow paths. Five critical scenarios demonstrate end-to-end capability flows: immediate grant-use, multi-hop delegation with attenuation, cascading time-bound policies, revocation propagation with signal broadcast, and distributed IPC with mid-flight revocation. All operations must meet strict latency SLAs (grant <1µs, delegation <2µs/hop, revocation <2µs cluster-wide, IPC <10µs p99).

---

## Problem Statement

**Core Challenges:**
1. **Lifecycle Completeness:** Current single-kernel capability engine lacks multi-agent grant-delegate-revoke workflows.
2. **Policy Enforcement Efficiency:** Mandatory Policy Engine evaluation must support both fast-path (common case) and slow-path (complex policy) execution without sacrificing correctness.
3. **Distributed Consistency:** Revocation signals must propagate to delegated agents and invalidate in-flight capabilities consistently.
4. **Attenuation Chain Semantics:** Multi-hop delegation requires transitive attenuation of time windows and rate limits across agent boundaries.
5. **Latency Guarantees:** MAANG-scale systems require sub-microsecond grant/revoke, microsecond-scale delegation.

---

## Architecture

### 3-Kernel Cluster Topology
```
┌─────────────────┐       ┌─────────────────┐       ┌─────────────────┐
│   K1 (Host)     │       │   K2 (Agent B)  │       │   K3 (Agent C)  │
│  CapEngine      │◄─────►│  CapEngine      │◄─────►│  CapEngine      │
│  PolicyEngine   │ IPC   │  PolicyEngine   │ IPC   │  PolicyEngine   │
└─────────────────┘       └─────────────────┘       └─────────────────┘
     Agent A                   Agent B                    Agent C
```

### CPL Policy Integration Layers

**Fast Path (O(1) lookup):**
- Pre-compiled PolicyDecisionTable with HashMap<(subject, resource, action), CplPolicy>
- Immediate policy hit returns decision without evaluation

**Slow Path (full evaluation):**
- Vec<CplPolicy> for complex predicates (temporal, contextual, delegation-bound)
- Evaluated on cache miss or policy update

**Cache Invalidation:**
- Hot-reload triggers complete recompilation
- Delegated capabilities inherit parent policy fingerprints
- Revocation updates policy version atomically

### PolicyDecisionTable Structure
```rust
pub struct PolicyDecisionTable {
    // Fast-path O(1) lookup: (subject_id, resource_id, action) -> policy decision
    fast_path: HashMap<(u64, u64, u32), PolicyDecision>,

    // Slow-path full evaluation for complex predicates
    slow_path: Vec<CplPolicy>,

    // Hit counters for telemetry
    fast_hit_count: Arc<AtomicU64>,
    slow_hit_count: Arc<AtomicU64>,

    // Policy version for cache coherence
    policy_version: Arc<AtomicU32>,
}

pub enum PolicyDecision {
    Allow,
    Deny(DenyReason),
    RequireSlowPath, // Delegate to slow-path evaluator
}
```

### PolicyEnforcer with Dual Paths
```rust
pub struct PolicyEnforcer {
    decision_table: Arc<RwLock<PolicyDecisionTable>>,
    cpl_compiler: CplCompiler,
    audit_trail: Arc<AuditTrail>,
    fast_misses: Arc<AtomicU64>,
    slow_timeouts: Arc<AtomicU64>,
}

impl PolicyEnforcer {
    pub fn enforce_capability(&self, grant: &CapabilityGrant, action: &CapAction)
        -> Result<PermitDecision, PolicyViolation> {
        // Fast-path attempt
        let key = (grant.subject_id, grant.resource_id, action.code);
        if let Some(decision) = self.decision_table.read().fast_path.get(&key) {
            match decision {
                PolicyDecision::Allow => return Ok(PermitDecision::Allow),
                PolicyDecision::Deny(reason) => {
                    self.audit_trail.log_denial(grant, reason);
                    return Err(PolicyViolation::from(reason));
                },
                PolicyDecision::RequireSlowPath => {},
            }
        }

        // Slow-path evaluation (full CPL interpretation)
        self.slow_path_evaluate(grant, action)
    }

    fn slow_path_evaluate(&self, grant: &CapabilityGrant, action: &CapAction)
        -> Result<PermitDecision, PolicyViolation> {
        let table = self.decision_table.read();
        for policy in &table.slow_path {
            if policy.matches(grant) && policy.permits_action(action) {
                return Ok(PermitDecision::Allow);
            }
        }
        Err(PolicyViolation::PolicyNotMatched)
    }
}
```

---

## Five Demonstration Scenarios

### Scenario 1: Grant and Immediate Use (A→B)
**Flow:** K1 grants read-only capability to K2; K2 uses immediately.
```rust
pub fn scenario_1_grant_immediate_use(env: &mut DemoEnvironment) -> TestResult {
    let start = Instant::now();

    // K1 grants read-only on /data/config to K2
    let grant = env.k1.grant_capability(
        subject: K2_ID,
        resource: "/data/config",
        mode: CapabilityMode::ReadOnly,
        attenuations: vec![],
    )?;

    let grant_latency = start.elapsed();
    assert!(grant_latency < Duration::from_nanos(1000), "Grant SLA violation");

    // K2 immediately exercises capability
    let exec_start = Instant::now();
    let data = env.k2.read_resource(&grant, "/data/config")?;
    let exec_latency = exec_start.elapsed();
    assert!(exec_latency < Duration::from_micros(10), "Execution SLA");

    // Audit verification
    env.audit_verifier.verify_audit_trail(&[
        AuditEvent::CapabilityGranted { from: K1_ID, to: K2_ID },
        AuditEvent::CapabilityUsed { by: K2_ID, resource: "/data/config" },
    ])?;

    Ok(TestResult::Pass)
}
```

### Scenario 2: Multi-Hop Delegation (A→B→C)
**Flow:** K1 grants to K2; K2 delegates attenuated to K3 (no further delegation).
```rust
pub fn scenario_2_multihop_delegation(env: &mut DemoEnvironment) -> TestResult {
    // K1 → K2: read-only, delegatable
    let parent_grant = env.k1.grant_capability(
        subject: K2_ID,
        resource: "/data/ledger",
        mode: CapabilityMode::ReadOnly,
        attenuations: vec![Attenuation::Delegatable(true)],
    )?;

    // K2 → K3: re-delegate with added constraint (no further delegation)
    let start = Instant::now();
    let child_grant = env.k2.delegate_capability(
        parent: &parent_grant,
        to: K3_ID,
        additional_attenuations: vec![Attenuation::Delegatable(false)],
    )?;
    let delegation_latency = start.elapsed();
    assert!(delegation_latency < Duration::from_nanos(2000), "Delegation SLA");

    // K3 uses capability
    let data = env.k3.read_resource(&child_grant, "/data/ledger")?;

    // K3 cannot further delegate (delegatable=false)
    let redel_result = env.k3.delegate_capability(&child_grant, K1_ID, vec![]);
    assert!(redel_result.is_err(), "Re-delegation should fail");

    env.audit_verifier.verify_delegation_chain(&[
        (K1_ID, K2_ID, parent_grant.id),
        (K2_ID, K3_ID, child_grant.id),
    ])?;

    Ok(TestResult::Pass)
}
```

### Scenario 3: Attenuation Chain (Time-Bound + Rate-Limit)
**Flow:** Multi-hop with cascading time windows (5m → 4m) and rate limits (100/s → 50/s).
```rust
pub fn scenario_3_attenuation_chain(env: &mut DemoEnvironment) -> TestResult {
    let now = SystemTime::now();

    // K1 → K2: 5 minute window, 100 req/sec limit
    let grant_k2 = env.k1.grant_capability(
        subject: K2_ID,
        resource: "/api/analytics",
        mode: CapabilityMode::ReadWrite,
        attenuations: vec![
            Attenuation::TimeWindow(now, now + Duration::from_secs(300)),
            Attenuation::RateLimit { ops_per_sec: 100, burst: 200 },
        ],
    )?;

    // K2 → K3: further attenuate to 4m and 50 req/sec
    let grant_k3 = env.k2.delegate_capability(
        parent: &grant_k2,
        to: K3_ID,
        additional_attenuations: vec![
            Attenuation::TimeWindow(now, now + Duration::from_secs(240)),
            Attenuation::RateLimit { ops_per_sec: 50, burst: 100 },
        ],
    )?;

    // Verify attenuation intersection: K3 has 4m + 50/s
    assert_eq!(grant_k3.attenuations.time_window_end, now + Duration::from_secs(240));
    assert_eq!(grant_k3.attenuations.effective_rate_limit, 50);

    // Rate limit enforcement
    for i in 0..51 {
        let result = env.k3.api_call(&grant_k3, "POST /api/analytics/record");
        if i >= 50 {
            assert!(result.is_err() || result.unwrap().rate_limited,
                   "Rate limit should trigger at 51st request");
        }
    }

    Ok(TestResult::Pass)
}
```

### Scenario 4: Revocation Propagation (A revokes → B,C invalidated)
**Flow:** K1 revokes capability; K2 and K3 receive SIG_CAPREVOKED and reject usage.
```rust
pub fn scenario_4_revocation_propagation(env: &mut DemoEnvironment) -> TestResult {
    // Setup: K1 → K2 → K3
    let parent_grant = env.k1.grant_capability(K2_ID, "/data/sensitive", CapMode::ReadOnly, vec![])?;
    let child_grant = env.k2.delegate_capability(&parent_grant, K3_ID, vec![])?;

    // All agents can use
    let _ = env.k2.read_resource(&parent_grant, "/data/sensitive")?;
    let _ = env.k3.read_resource(&child_grant, "/data/sensitive")?;

    // K1 revokes parent capability
    let start = Instant::now();
    env.k1.revoke_capability(&parent_grant)?;
    let revoke_latency = start.elapsed();
    assert!(revoke_latency < Duration::from_nanos(2000), "Revocation SLA");

    // Broadcast SIG_CAPREVOKED to all holding agents
    env.broadcast_signal(Signal::CapRevoked(parent_grant.id))?;

    // Verify K2 rejects usage
    let k2_result = env.k2.read_resource(&parent_grant, "/data/sensitive");
    assert!(k2_result.is_err());
    assert_eq!(k2_result.unwrap_err(), Error::CapabilityRevoked);

    // Verify K3 rejects usage (delegated chain invalidated)
    let k3_result = env.k3.read_resource(&child_grant, "/data/sensitive");
    assert!(k3_result.is_err());

    env.audit_verifier.verify_revocation_audit(parent_grant.id)?;

    Ok(TestResult::Pass)
}
```

### Scenario 5: Distributed IPC with Mid-Flight Revocation
**Flow:** K2 initiates IPC call with capability; K1 revokes during flight; K3 rejects with SIG_CAPREVOKED.
```rust
pub fn scenario_5_ipc_midair_revocation(env: &mut DemoEnvironment) -> TestResult {
    // K1 grants capability to K2
    let grant = env.k1.grant_capability(K2_ID, "/rpc/execute", CapMode::ReadWrite, vec![])?;

    // K2 begins async RPC to K3, passing capability
    let (rpc_handle, mut rx) = env.k2.async_ipc_call(
        target: K3_ID,
        method: "process_data",
        cap_grant: &grant,
        payload: vec![1, 2, 3, 4, 5],
    )?;

    // Simulate mid-flight delay
    std::thread::sleep(Duration::from_millis(5));

    // K1 revokes capability while RPC is in transit
    env.k1.revoke_capability(&grant)?;
    env.broadcast_signal(Signal::CapRevoked(grant.id))?;

    // K3 receives revocation signal, checks capability freshness
    let ipc_start = Instant::now();
    let result = env.k3.receive_ipc_call(&grant)?;
    let ipc_latency = ipc_start.elapsed();
    assert!(ipc_latency < Duration::from_micros(10), "IPC SLA");

    // K3 must reject due to revocation
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), RpcError::CapabilityRevoked);

    // Audit trail captures full sequence
    env.audit_verifier.verify_ipc_rejection_audit(grant.id)?;

    Ok(TestResult::Pass)
}
```

---

## CPL Compiler & Policy Decision Table Integration

```rust
pub struct CplCompiler {
    policies: Vec<CplPolicy>,
    config: CompilerConfig,
}

impl CplCompiler {
    pub fn compile(&self) -> Result<PolicyDecisionTable, CompileError> {
        let mut fast_path = HashMap::new();
        let slow_path = Vec::new();

        // Pre-compile simple (subject, resource, action) → decision mappings
        for policy in &self.policies {
            if policy.is_simple_predicate() {
                let key = (policy.subject, policy.resource, policy.action);
                fast_path.insert(key, PolicyDecision::Allow);
            } else {
                // Complex policies deferred to slow path
                slow_path.push(policy.clone());
            }
        }

        Ok(PolicyDecisionTable {
            fast_path,
            slow_path,
            fast_hit_count: Arc::new(AtomicU64::new(0)),
            slow_hit_count: Arc::new(AtomicU64::new(0)),
            policy_version: Arc::new(AtomicU32::new(1)),
        })
    }
}

pub struct AuditTrailVerifier {
    events: Arc<Mutex<Vec<AuditEvent>>>,
}

impl AuditTrailVerifier {
    pub fn verify_audit_trail(&self, expected: &[AuditEvent]) -> Result<(), AuditError> {
        let actual = self.events.lock().unwrap();
        // Verify sequence and timestamps
        for (i, exp) in expected.iter().enumerate() {
            if !actual.iter().any(|e| e.matches(exp)) {
                return Err(AuditError::MissingEvent(*exp));
            }
        }
        Ok(())
    }
}
```

---

## Testing & Validation

### ScenarioRunner Orchestration
```rust
pub struct ScenarioRunner {
    environment: DemoEnvironment,
    results: Vec<ScenarioResult>,
}

impl ScenarioRunner {
    pub fn run_all_scenarios(&mut self) -> Result<TestSummary, RunnerError> {
        let scenarios = vec![
            ("Scenario 1: Grant & Immediate Use", scenario_1_grant_immediate_use),
            ("Scenario 2: Multi-Hop Delegation", scenario_2_multihop_delegation),
            ("Scenario 3: Attenuation Chain", scenario_3_attenuation_chain),
            ("Scenario 4: Revocation Propagation", scenario_4_revocation_propagation),
            ("Scenario 5: IPC Mid-Air Revocation", scenario_5_ipc_midair_revocation),
        ];

        for (name, runner) in scenarios {
            let result = runner(&mut self.environment);
            self.results.push(ScenarioResult { name, result });
        }

        self.summarize()
    }
}
```

---

## Acceptance Criteria

| Criterion | Target | Validation |
|-----------|--------|-----------|
| **Grant Latency** | <1000 ns | Measure via `Instant::now()` in scenario 1 |
| **Delegation Latency** | <2000 ns/hop | Multi-hop timing in scenario 2 |
| **Revocation Latency** | <2000 ns all agents | Broadcast + enforcement in scenario 4 |
| **IPC Latency (p99)** | <10 µs | Network timing in scenario 5 |
| **Policy Hit Rate** | >95% fast-path | Count `fast_hit_count` vs. total |
| **Audit Completeness** | 100% events logged | Verify all scenarios in audit trail |
| **Attenuation Enforcement** | Intersection enforced | Verify tightest bounds in scenario 3 |
| **Revocation Propagation** | 100% chain coverage | Ensure K2, K3 reject in scenario 4 |

---

## Design Principles

1. **Dual-Path Efficiency:** O(1) fast-path for common policy patterns; full evaluation only when necessary.
2. **Distributed Consistency:** Revocation signals broadcast atomically; capability versioning prevents use-after-revoke.
3. **Transitive Attenuation:** Delegation always narrows (intersects) constraints; no capability amplification.
4. **Audit Completeness:** Every grant, delegation, use, revocation logged with monotonic timestamps.
5. **Latency Precision:** Sub-microsecond grants/revokes via lock-free atomics; IPC bounded by network only.

---

## Deliverables

- **DemoEnvironment:** 3-kernel cluster with K1, K2, K3 AgentCrew coordination.
- **CplCompiler:** Compile CPL policies to fast-path HashMap and slow-path Vec.
- **PolicyDecisionTable:** Dual-path decision engine with cache invalidation.
- **PolicyEnforcer:** Grant enforcement with fast/slow hit telemetry.
- **AuditTrailVerifier:** Replay and verify all five scenario event sequences.
- **ScenarioRunner:** Execute all five scenarios with latency measurement and result reporting.

---

**Next Steps:** Implement Rust modules, validate against 3-kernel cluster, profile fast-path hit rate, tune slow-path evaluation for <100µs worst case.
