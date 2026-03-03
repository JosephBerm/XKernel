# Engineer 6 — Services: Tool Registry, Telemetry & Compliance — Week 21

## Phase: Phase 3 (Weeks 25-36) Prep / Phase 2 Integration

## Weekly Objective
Integrate all Phase 2 components (Compliance Engine, retention policies, export APIs) and begin Phase 3 planning. Prepare for telemetry benchmarks and adversarial testing.

## Document References
- **Primary:** Weeks 15-20 (Phase 2 completion), Section 6.3 (Phase 3 planning)
- **Supporting:** All service components from Weeks 7-20

## Deliverables
- [ ] Phase 2 integration testing
  - All services (Tool Registry, Telemetry, Compliance) working together
  - End-to-end workflows (tool invocation -> telemetry -> compliance event -> retention)
  - Performance under combined load
- [ ] Phase 2 documentation finalization
  - Complete architecture diagrams
  - Deployment guide for Phase 2 services
  - Operational procedures (policy updates, compliance reports, exports)
  - Known limitations and Phase 3 improvements
- [ ] Phase 3 planning: Benchmarks (Week 25-28)
  - Design cost attribution accuracy test plan (>99% target)
  - Design Tool Registry throughput benchmark (target: 1M invocations/hour)
  - Design telemetry latency benchmark (target: <100ms end-to-end)
  - Identify hardware/software instrumentation needed
- [ ] Phase 3 planning: Adversarial Testing (Week 29-30)
  - Tool sandbox escape attempts (simulated attacks)
  - Telemetry tampering (attempt to modify event logs)
  - Audit log integrity attacks (attempt to forge entries)
  - Policy engine attacks (privilege escalation attempts)
  - Test plan and expected outcomes documented
- [ ] Phase 3 planning: Compliance Validation (Week 31-32)
  - EU AI Act compliance validation plan
  - External counsel engagement plan
  - Security audit scope and timeline
  - Risk assessment and mitigation
- [ ] Begin Week 21 optimization work
  - Identify performance bottlenecks from Phase 2 testing
  - Optimize critical paths (cache lookup, policy evaluation, event emission)
  - Profile memory and CPU usage
  - Document optimization recommendations for Phase 3

## Technical Specifications

### Phase 2 Integration Test
```rust
#[tokio::test]
async fn test_phase2_full_integration() {
    // Setup all services
    let registry = MCPToolRegistry::new("localhost:9000").await.unwrap();
    let telemetry = TelemetryEngineV2::new(default_config());
    let policy_engine = MandatoryPolicyEngine::new(Arc::new(telemetry.clone()));
    let compliance_engine = ComplianceEngine::new(Arc::new(telemetry.clone()));
    let cache = PersistentCache::new(default_config()).unwrap();

    // Load policies and discover tools
    policy_engine.load_policies(Path::new("policies.yaml")).await.unwrap();
    registry.discover_tools().await.unwrap();

    // Workflow: Policy check -> Tool invocation -> Telemetry -> Compliance recording
    for i in 0..100 {
        let tool_id = format!("tool-{}", i % 5);

        // 1. Policy check
        let input = PolicyDecisionInput {
            requester_agent: format!("agent-{}", i % 10),
            requested_capability: format!("invoke.{}", tool_id),
            context: Default::default(),
        };
        let outcome = policy_engine.evaluate_capability_request(input).await.unwrap();

        // 2. Tool invocation (if allowed)
        if matches!(outcome, PolicyOutcome::Allow | PolicyOutcome::Audit) {
            let input = format!("input-{}", i);
            let result = registry.invoke_tool_with_cache(&tool_id, input, &cache, &Arc::new(telemetry.clone()))
                .await
                .unwrap();

            // 3. Verify telemetry events recorded
            // (In real test: subscribe and verify)

            // 4. Verify compliance events recorded
            let compliance_entries = compliance_engine.execute_compliance_query(&ComplianceQuery {
                regulation: Some(ApplicableRegulation::EUAIAct),
                ..Default::default()
            }).await.unwrap();

            assert!(compliance_entries.matching_entries > 0);
        }
    }
}
```

### Phase 3 Benchmark Plan Document
```
PHASE 3 BENCHMARK PLAN

1. Cost Attribution Accuracy (Target: >99%)
   - Measure actual GPU-ms, token counts, wall-clock time
   - Compare to attributed costs
   - Test on all 5 tools
   - Run 10k invocations per tool
   - Report p50, p95, p99 accuracy
   - Expected: >99% accuracy across all metrics

2. Tool Registry Throughput (Target: 1M invocations/hour)
   - Concurrent invocation test
   - Measure requests/second for all 5 tools
   - Cache hit/miss scenarios
   - Policy decision latency included
   - Expected: >277 invocations/sec (1M/3600)

3. Telemetry End-to-End Latency (Target: <100ms)
   - Emit event -> subscriber receives
   - Include serialization, network transport
   - Measure p50, p95, p99
   - Expected: <100ms p99

4. Compliance Engine Query Latency
   - Query by regulation, time range, decision type
   - Measure on 1M+ events
   - Expected: <1 second for any query

5. Instrumentation Needed
   - Hardware performance counters (GPU, CPU, memory)
   - Network instrumentation (latency, throughput)
   - Storage instrumentation (IOPS, latency)
   - Profiling tools (perf, flamegraph, etc.)
```

## Dependencies
- **Blocked by:** Weeks 7-20 (all Phase 1 and Phase 2 complete)
- **Blocking:** Phase 3 Week 22-24 (optimization and deployment)

## Acceptance Criteria
- [ ] Phase 2 integration test passes (policy -> tool -> telemetry -> compliance)
- [ ] All services operational under combined load
- [ ] Phase 2 architecture documentation complete
- [ ] Phase 2 deployment guide written and tested
- [ ] Phase 3 benchmark plans documented (cost, throughput, latency)
- [ ] Phase 3 adversarial testing plans documented
- [ ] Phase 3 compliance validation plan documented
- [ ] Performance bottlenecks identified
- [ ] Optimization recommendations documented
- [ ] Phase 3 transition preparation complete

## Design Principles Alignment
- **Integration:** All services work together seamlessly
- **Performance visibility:** Benchmarks enable continuous optimization
- **Security robustness:** Adversarial testing uncovers edge cases
- **Compliance excellence:** Validation ensures regulatory requirements met
- **Operational readiness:** Complete documentation enables production deployment
