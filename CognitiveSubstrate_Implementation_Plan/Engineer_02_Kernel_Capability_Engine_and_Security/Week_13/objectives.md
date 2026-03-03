# Engineer 2 — Kernel: Capability Engine & Security — Week 13

## Phase: PHASE 1 - Core Services + Multi-Agent

## Weekly Objective
Begin preparation and implementation of comprehensive multi-agent demonstration. Showcase full capability lifecycle across 3-agent crew with Grant, Delegate, Attenuate, Revoke, and distributed IPC operations. Set up test environment and define demo scenarios.

## Document References
- **Primary:** Section 2.1 (Architecture Overview), Section 3.2.3 (Capability Enforcement), Section 3.2.4 (Distributed IPC)
- **Supporting:** Engineer 7 (AgentCrew), Engineer 3 (Context Isolation), Week 1-12 (all Phase 1 implementations)

## Deliverables
- [ ] Test environment setup (3-kernel cluster, AgentCrew coordination)
- [ ] Demo scenario definition document (5 primary scenarios, 10 secondary scenarios)
- [ ] 3-agent scenario 1: grant and immediate use (Agent A grants to Agent B)
- [ ] 3-agent scenario 2: multi-hop delegation (Agent A → B → C)
- [ ] 3-agent scenario 3: attenuation chain (each agent attenuates further)
- [ ] 3-agent scenario 4: revocation propagation (revoke at A → cascade to B, C)
- [ ] 3-agent scenario 5: distributed IPC with revocation (capability revoked mid-flight)
- [ ] Audit trail verification for all scenarios
- [ ] Performance metrics collection (grant latency, delegation latency, revocation latency)
- [ ] Documentation of demo execution and results
- [ ] **CPL integration with Mandatory Policy Engine — policies loaded from CPL files**
- [ ] **CPL parser and compiler (Rust) — validates and compiles policies to decision tables**
- [ ] **Fast-path enforcement: cached policy decisions for repeated capability grants (O(1) lookup)**
- [ ] **Slow-path enforcement: full CPL evaluation for novel capability requests**
- [ ] **Policy cache invalidation on hot-reload or policy version change**
- [ ] **Integration tests: 3-agent demo with CPL policies enforced at each grant**

## Technical Specifications

### CPL Integration with Mandatory Policy Engine

**CPL Compilation to Decision Tables**
```rust
pub struct PolicyDecisionTable {
    // Fast-path: O(1) lookup for (agent_id, capability_type) tuples
    fast_path: HashMap<(String, String), PolicyDecision>,

    // Slow-path: full CPL policies for novel combinations
    slow_path: Vec<CplPolicy>,

    // Metadata
    policy_version_hash: String,
    compilation_timestamp: i64,
}

pub struct CplCompiler;

impl CplCompiler {
    pub fn compile(cpl_source: &str) -> Result<PolicyDecisionTable, CompileError> {
        // 1. Parse CPL into AST
        let policies = cpl_parser::parse(cpl_source)?;

        // 2. Validate policies (no conflicts, well-formed rules)
        cpl_validator::validate(&policies)?;

        // 3. Extract common patterns for fast-path table
        let fast_path = Self::extract_fast_patterns(&policies);

        // 4. Compute policy version hash
        let hash = Self::compute_hash(cpl_source);

        Ok(PolicyDecisionTable {
            fast_path,
            slow_path: policies,
            policy_version_hash: hash,
            compilation_timestamp: now_ms(),
        })
    }

    fn extract_fast_patterns(policies: &[CplPolicy]) -> FastPathTable {
        // Extract simple equality checks: agent == X AND capability == Y
        // Build HashMap for O(1) lookup
        let mut table = HashMap::new();
        for policy in policies {
            if let Some((agent_pat, cap_pat)) = Self::extract_simple_rule(&policy.rule) {
                table.insert((agent_pat, cap_pat), policy.decision);
            }
        }
        table
    }
}
```

**Fast-Path vs Slow-Path Enforcement**
```rust
pub struct PolicyEnforcer {
    decision_table: Arc<RwLock<PolicyDecisionTable>>,
    fast_hits: Arc<AtomicU64>,
    slow_hits: Arc<AtomicU64>,
}

impl PolicyEnforcer {
    pub async fn evaluate(&self, agent_id: &str, capability: &str) -> PolicyDecision {
        let table = self.decision_table.read().await;

        // Fast-path: O(1) lookup
        if let Some(decision) = table.fast_path.get(&(agent_id.to_string(), capability.to_string())) {
            self.fast_hits.fetch_add(1, Ordering::Relaxed);
            return decision.clone();
        }

        // Slow-path: full CPL evaluation
        self.slow_hits.fetch_add(1, Ordering::Relaxed);

        let mut decision = PolicyDecision::default_deny();
        for policy in &table.slow_path {
            if policy.scope_matches(agent_id, capability) && policy.rule_matches(agent_id, capability) {
                decision = policy.decision.clone();
                break; // First match wins
            }
        }

        decision
    }
}
```

**Cache Invalidation on Hot-Reload**
- On CPL file change: recompile policies, atomic swap of PolicyDecisionTable
- In-flight requests: use old table (RwLock read guard prevents concurrent writes)
- New requests: use new table immediately after swap
- Metrics: recompilation time, fast-path hit rate, slow-path hit rate

### Test Environment Setup:
  - 3 separate kernel instances (K1, K2, K3) running on same system or separate machines
  - AgentCrew coordination: agents A, B, C distributed across kernels
  - Shared memory regions: crew shared context for inter-agent communication
  - Network IPC: all kernels connected via loopback or Ethernet
  - Clock synchronization: NTP or local clock skew <1ms
  - Logging: central log aggregation for audit trail verification
- **Demo Scenario 1: Grant and Immediate Use**
  - Setup: Agent A has read-write capability to resource R
  - Action: Agent A grants read-only capability to Agent B
  - Verification:
    - Agent B can read resource R
    - Agent B cannot write to resource R (write operation rejected)
    - Audit trail shows: A → B grant with {read} operations
  - Latency measurement: grant operation latency (target <1000ns)
- **Demo Scenario 2: Multi-Hop Delegation**
  - Setup: Agent A has read-write capability to resource R
  - Actions:
    - A grants {read, write} to B
    - B delegates {read} to C (attenuation: remove write)
  - Verification:
    - Agent C can read resource R
    - Agent C cannot write (write operation rejected)
    - Audit trail: A → B (read, write) → C (read only)
    - CapChain shows full 2-hop delegation with attenuation
  - Latency: multi-hop delegation latency (target <2000ns per hop)
- **Demo Scenario 3: Attenuation Chain**
  - Setup: Agent A has time-unlimited, rate-unlimited capability
  - Actions:
    - A → B: add 1-hour time bound
    - B → C: add rate limit (100 ops/sec)
  - Verification:
    - Agent C can use capability for 1 hour (time bound applies)
    - Agent C is limited to 100 ops/sec (rate limit applies)
    - After 1 hour: C's capability automatically expires
    - Audit trail shows all constraint compositions
  - Latency: constraint composition (target <1000ns)
- **Demo Scenario 4: Revocation Propagation**
  - Setup: A → B → C capability chain
  - Action: Agent A revokes capability
  - Verification:
    - Agent B receives SIG_CAPREVOKED immediately
    - Agent C receives SIG_CAPREVOKED immediately
    - Agent B's capability is invalidated
    - Agent C's capability is invalidated
    - All agents cannot use revoked capability
    - Audit trail shows revocation timestamp and originator
  - Latency: revocation propagation (target <2000ns for all agents)
- **Demo Scenario 5: Distributed IPC with Revocation**
  - Setup: K1.Agent_A wants to delegate capability to K3.Agent_C (via K2)
  - Actions:
    - A sends delegated capability to B via IPC
    - B receives and verifies signature
    - B forwards to C
    - Midway: A revokes original capability
  - Verification:
    - C's capability is invalidated even if packet already received
    - Revocation service notifies all kernels
    - C's audit trail shows revocation-before-use
  - Latency: IPC latency (target <10000ns p99)

## Dependencies
- **Blocked by:** Week 1-12 (all Phase 1 implementations), Engineer 7 (AgentCrew)
- **Blocking:** Week 14 (demo completion and results)

## Acceptance Criteria
- Test environment successfully runs 3-kernel cluster with AgentCrew
- All 5 primary scenarios execute successfully
- Audit trails correctly capture all operations
- Latency targets met for all scenarios
- No security violations detected (no unauthorized access)
- All 150+ integration tests pass
- Performance metrics collected and analyzed
- Demo documentation complete and reviewed

## Design Principles Alignment
- **P1 (Security-First):** Scenarios verify no unauthorized access
- **P2 (Transparency):** Audit trails document all capability operations
- **P3 (Granular Control):** Delegation and attenuation scenarios showcase fine-grained control
- **P4 (Performance):** Latency metrics validate performance targets
- **P7 (Multi-Agent Harmony):** Multi-agent scenarios demonstrate crew cooperation
