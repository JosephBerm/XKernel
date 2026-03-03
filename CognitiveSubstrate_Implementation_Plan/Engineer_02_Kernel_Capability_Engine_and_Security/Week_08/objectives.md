# Engineer 2 — Kernel: Capability Engine & Security — Week 8

## Phase: PHASE 1 - Core Services + Multi-Agent

## Weekly Objective
Complete advanced delegation chain scenarios including revocation callbacks, multi-level delegation, constraint composition across 3+ delegation hops, and production-grade error handling and recovery.

## Document References
- **Primary:** Section 2.4 (Capability Delegation & Attenuation), Section 3.2.3 (Revoke Operation with Cascade)
- **Supporting:** Section 2.4 (CapChain Provenance), Architecture documentation on delegation patterns

## Deliverables
- [ ] Revocation callback registration mechanism (agents notified when delegated capability revoked)
- [ ] Multi-level delegation support (Agent A → B → C → D with correct attenuation stacking)
- [ ] Constraint composition across 3+ delegation hops with accumulating restrictions
- [ ] Cascade revocation for multi-level delegations (revoke at any level invalidates all descendants)
- [ ] Error handling and recovery scenarios (failed delegations, constraint violations, race conditions)
- [ ] Production-grade logging and monitoring for delegation operations
- [ ] Integration test suite (150+ tests covering complex multi-agent scenarios)
- [ ] Documentation of delegation patterns and best practices
- [ ] Performance profiling for deep delegation chains
- [ ] **Cognitive Policy Language (CPL) design — declarative DSL for MandatoryCapabilityPolicies**
- [ ] **CPL specification document with formal grammar and evaluation semantics**
- [ ] **CPL examples and use cases (production_db_access, api_rate_limits, sandbox_containment)**
- [ ] **Reference implementation: CPL parser and validator (Rust)**
- [ ] **Alignment with seL4's capDL model (APSYS 2010) as proven precedent**
- [ ] **Addendum v2.5.1 — Correction 4: Policy DSL**

## Technical Specifications

### Cognitive Policy Language (CPL)

CPL is a declarative, statically verifiable DSL for writing MandatoryCapabilityPolicies in formal syntax instead of ad-hoc code. Inspired by seL4's capDL (Capability Distribution Language), CPL enables policies to be written, reviewed, versioned, and formally verified.

**CPL Example: Production Database Access Policy**
```
policy production_db_access {
  scope: all_agents
  enforcement: deny
  rule: capability.target.type == "database"
        AND capability.target.tags contains "production"
        AND NOT agent.has_approval("human_admin")
  audit: always
  exception_requires: human_approval
}

policy api_rate_limits {
  scope: [agent_web_service, agent_batch_processor]
  enforcement: enforce
  rule: capability.target.service == "external_api"
  rate_limit: 1000 calls per hour
  cost_limit: 10000 tokens per day
  audit: on_denial
  exception_requires: none
}

policy sandbox_containment {
  scope: agent_untrusted_plugin
  enforcement: deny
  rule: capability.target.type == "file_system"
        OR capability.target.type == "network"
        OR capability.target.type == "subprocess"
  audit: always
  exception_requires: manager_approval
}
```

**CPL Grammar (EBNF subset)**
```
policy_def = "policy" IDENTIFIER "{" policy_body "}"
policy_body = scope_clause enforcement_clause rule_clause [audit_clause] [exception_clause]

scope_clause = "scope:" (IDENTIFIER | "[" IDENTIFIER_LIST "]" | "all_agents")
enforcement_clause = "enforcement:" ("deny" | "enforce" | "audit" | "warn")
rule_clause = "rule:" boolean_expr
audit_clause = "audit:" ("always" | "on_denial" | "on_approval" | "never")
exception_clause = "exception_requires:" ("none" | "human_approval" | "manager_approval")

boolean_expr = comparison ("AND" | "OR" boolean_expr)?
comparison = member_access operator value
member_access = ("capability" | "agent") "." field_name
operator = "==" | "!=" | "contains" | "startswith"
```

**CPL Evaluation Semantics**
- Policies are evaluated in declaration order (first match wins)
- Each policy returns a decision: ALLOW, DENY, REQUIRE_APPROVAL, AUDIT, WARN
- Policies are NOT short-circuiting; all matching policies are logged
- Scope restricts which agents/capabilities the policy applies to
- Enforcement determines action on match:
  - deny: block the capability grant
  - enforce: apply rate limits, cost limits, or other constraints
  - audit: allow but log at audit level
  - warn: allow but log at warning level

**Reference: seL4's capDL Model**
- seL4 capDL (APSYS 2010) defines capabilities declaratively in structured text
- Advantages: static verification, version control, audit trails, no code review overhead
- CPL adopts this model for cognitive policies, proving the concept in production OS design

### Revocation Callbacks:
  - On delegation: delegating_agent can register callback(agent_id, callback_fn)
  - Callback triggered when capability revoked at any level
  - Callback function: fn(revoked_capid, revocation_reason, timestamp) → ()
  - Executed synchronously in kernel context before Revoke returns
  - Enables agents to clean up delegated resources (e.g., revoke sub-delegations)
  - Latency target: callback execution <500ns per registered callback
- **Multi-Level Delegation:**
  - Delegation chain: A.cap[0] → B.cap[1] → C.cap[2] → D.cap[3]
  - Each step creates new CapID with new entry in CapChain
  - Each step validates attenuation is monotonic reduction
  - Constraint composition: constraints from ALL previous hops apply
  - Agent D's cap[3] operations ⊆ A's original operations (transitive)
  - Time bounds: cap[3].expiry ≤ cap[0].expiry (transitivity)
- **Constraint Composition Across Hops:**
  - Example 1: A grants {read, write}, B attenuates to {read}, C attenuates to {read} → D gets {read}
  - Example 2: A grants (expiry=T), B attenuates (expiry=T-10min), C attenuates (expiry=T-20min) → D gets (expiry=T-20min)
  - Example 3: A grants (rate=1000/sec), B attenuates (rate=500/sec), C attenuates (rate=250/sec) → D gets (rate=250/sec)
  - All compositions correctly implement set/value intersection semantics
- **Cascade Revocation:**
  - Revoke at level 2 (B's capability) invalidates level 3+ (C, D)
  - Example: Revoke(cap[1]) → invalidates cap[2], cap[3], and all further descendants
  - Dispatch SIG_CAPREVOKED to C and D with root_cause=cap[1]
  - Callback chains: C's callback fires first, then D's
  - Rollback: all page table unmappings happen in cascade
- **Error Handling:**
  - Constraint violation: delegation with invalid composition → return error, no CapID created
  - Race condition: concurrent delegations from same parent → serialize via per-capability lock
  - Callback failure: callback throws exception → log error, continue Revoke (revocation succeeds regardless)
  - Persistent store failure: capability service unavailable → return error, keep in-memory state consistent
- **Production-Grade Logging:**
  - Structured logs: (timestamp, agent_id, operation, capid, status, latency_ns)
  - Debug level: full constraint details, callback execution
  - Info level: success/failure summary
  - Error level: constraint violations, callback failures, persistence failures
  - Logging latency: <100ns (async via ring buffer)
- **Monitoring:**
  - Metrics: delegation_operations_per_sec, cascade_revocation_depth_histogram, callback_latency_histogram
  - Alerts: slow delegations (>5000ns p99), cascade revocation chain length (>20 levels)
  - Dashboards: delegation throughput, cascade revocation performance, error rates

## Dependencies
- **Blocked by:** Week 7 (basic delegation chains), Week 6 (capability table optimization)
- **Blocking:** Week 9 (Membrane pattern for sandboxes), Week 13-14 (multi-agent demo)

## Acceptance Criteria
- Revocation callbacks execute correctly and in order
- Multi-level delegations (4+ hops) work correctly with proper attenuation
- Constraint composition is transitive and monotonically restrictive
- Cascade revocation invalidates all descendants at any depth
- All 150+ integration tests pass (multi-agent scenarios)
- Error cases handled gracefully with proper error messages
- Structured logging captures all relevant delegation operations
- Monitoring shows <5000ns p99 latency for typical delegations
- Code review completed by security and performance teams

## Design Principles Alignment
- **P1 (Security-First):** Constraint composition is monotonic, preventing privilege elevation
- **P2 (Transparency):** Callbacks enable agents to audit delegation revocations
- **P3 (Granular Control):** Revocation at any level enables fine-grained revocation policies
- **P4 (Performance):** Callback latency <500ns enables high-throughput delegation
- **P5 (Formal Verification):** Cascade revocation can be formally verified as complete
- **P6 (Compliance & Audit):** Logging and monitoring support regulatory requirements
- **P7 (Multi-Agent Harmony):** Multi-level delegation enables complex agent hierarchies
