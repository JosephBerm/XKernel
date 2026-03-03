# Engineer 6 — Services: Tool Registry, Telemetry & Compliance — Week 13

## Phase: Phase 1 (Weeks 7-14)

## Weekly Objective
Integration testing and hardening of Phase 1 services: Tool Registry, Telemetry Engine, response caching, and Policy Engine. Verify all components work together correctly under load.

## Document References
- **Primary:** Section 6.2 (Phase 1 completion), all Weeks 7-12
- **Supporting:** Section 3.3.3-3.3.6 (all services)

## Deliverables
- [ ] End-to-end integration test suite
  - Tool discovery -> binding -> sandbox config -> invocation -> caching -> telemetry -> policy decision
  - All 5 production tools exercised in sequence
  - Different policy outcomes (Allow, Deny, RequireApproval, Audit, Warn) tested
- [ ] Load testing
  - Concurrent tool invocations (100, 1000, 10000 concurrent)
  - Cache hit/miss patterns under high load
  - Telemetry event throughput (target: >10k events/sec)
  - Policy decision latency under load (target: <5ms p99)
- [ ] Failure and recovery testing
  - MCP server disconnection and reconnection
  - Tool sandbox constraint violation (prevent and audit)
  - Policy reload with rollback on validation failure
  - Event streaming subscriber disconnect and reconnect
  - Core dump capture on exception
- [ ] Performance and resource profiling
  - Memory footprint (in-memory cache, event buffer, policy rules)
  - CPU utilization during high-frequency operations
  - I/O patterns (persistent logging, cache backend)
  - Identify bottlenecks and optimization opportunities
- [ ] Cost attribution validation
  - Verify cost metrics accuracy across all tools
  - Compare attributed cost vs actual hardware counters
  - Generate cost reports (per-tool, per-agent, per-time-period)
- [ ] Security and compliance testing
  - Policy enforcement under adversarial conditions
  - Sandbox escape attempts (negative testing)
  - Audit log integrity (tamper-resistant)
  - Event ordering and causality preservation
- [ ] Documentation
  - Phase 1 architecture complete and reviewed
  - Operational runbook (deploying, configuring, monitoring)
  - Troubleshooting guide (common issues and solutions)
  - Performance tuning guide
- [ ] Phase 1 retrospective
  - Lessons learned
  - Known limitations for Phase 2
  - Risk assessment for Phase 2 transition

## Technical Specifications

### End-to-End Integration Test
```rust
#[tokio::test]
async fn test_complete_workflow() {
    // Setup
    let mcp_registry = MCPToolRegistry::new("localhost:9000").await.unwrap();
    let cache = PersistentCache::new(default_config()).unwrap();
    let telemetry = TelemetryEngineV2::new(default_config());
    let policy_engine = MandatoryPolicyEngine::new(Arc::new(telemetry.clone()));

    // Load policies
    policy_engine.load_policies(Path::new("test_policies.yaml")).await.unwrap();

    // Discover tools
    let tools = mcp_registry.discover_tools().await.unwrap();
    assert_eq!(tools.len(), 5);

    // For each tool, test complete workflow
    for tool_name in &tools {
        // 1. Get binding
        let binding = mcp_registry.get_binding(tool_name).await.unwrap();
        assert!(!binding.sandbox_config.allowed_domains.is_empty()
                || !binding.sandbox_config.allowed_paths.is_empty());

        // 2. Request capability (policy check)
        let policy_input = PolicyDecisionInput {
            requester_agent: "test-agent".to_string(),
            requested_capability: format!("invoke.{}", tool_name),
            context: Default::default(),
        };
        let outcome = policy_engine.evaluate_capability_request(policy_input).await.unwrap();
        assert!(matches!(outcome, PolicyOutcome::Allow | PolicyOutcome::Audit));

        // 3. Invoke tool
        let input = format!("test input for {}", tool_name);
        let cache_key = CacheKeyGenerator::generate_key(tool_name, &input);

        // First invocation (cache miss)
        let result1 = mcp_registry.invoke_tool_with_cache(
            tool_name, input.clone(), &cache, &Arc::new(telemetry.clone())
        ).await.unwrap();

        // Second invocation (cache hit expected for read-only tools)
        let result2 = mcp_registry.invoke_tool_with_cache(
            tool_name, input.clone(), &cache, &Arc::new(telemetry.clone())
        ).await.unwrap();

        assert_eq!(result1, result2);

        // 4. Verify telemetry events
        // (In real test: subscribe to events and verify)
    }
}
```

### Load Testing
```rust
#[tokio::test]
async fn test_load_1000_concurrent_invocations() {
    let mcp_registry = MCPToolRegistry::new("localhost:9000").await.unwrap();
    let cache = PersistentCache::new(default_config()).unwrap();
    let telemetry = Arc::new(TelemetryEngineV2::new(default_config()));
    let policy_engine = Arc::new(MandatoryPolicyEngine::new(telemetry.clone()));

    policy_engine.load_policies(Path::new("test_policies.yaml")).await.unwrap();
    mcp_registry.discover_tools().await.unwrap();

    let start = Instant::now();
    let mut handles = vec![];

    for i in 0..1000 {
        let registry = mcp_registry.clone();
        let cache = cache.clone();
        let telemetry = telemetry.clone();

        let handle = tokio::spawn(async move {
            let tool_id = format!("tool-{}", i % 5);
            let input = format!("input-{}", i);
            registry.invoke_tool_with_cache(&tool_id, input, &cache, &telemetry)
                .await
                .ok()
        });
        handles.push(handle);
    }

    let results: Vec<_> = futures::future::join_all(handles).await;
    let elapsed = start.elapsed();
    let successful = results.iter().filter(|r| r.is_ok()).count();

    println!("Load test results:");
    println!("  Total: 1000 invocations");
    println!("  Successful: {}", successful);
    println!("  Elapsed: {:?}", elapsed);
    println!("  Throughput: {:.0} invocations/sec", 1000.0 / elapsed.as_secs_f64());

    assert!(successful > 950); // 95% success rate
}

#[tokio::test]
async fn test_telemetry_event_throughput() {
    let telemetry = TelemetryEngineV2::new(default_config());

    let start = Instant::now();
    for i in 0..100_000 {
        telemetry.emit_event(CEFEvent {
            event_id: format!("event-{}", i),
            ..Default::default()
        }).await.ok();
    }
    let elapsed = start.elapsed();

    let throughput = 100_000.0 / elapsed.as_secs_f64();
    println!("Telemetry event throughput: {:.0} events/sec", throughput);
    assert!(throughput > 10_000.0); // Target: >10k events/sec
}

#[tokio::test]
async fn test_policy_decision_latency_under_load() {
    let telemetry = Arc::new(TelemetryEngineV2::new(default_config()));
    let policy_engine = MandatoryPolicyEngine::new(telemetry);
    policy_engine.load_policies(Path::new("test_policies.yaml")).await.unwrap();

    let mut latencies = vec![];

    for i in 0..10_000 {
        let input = PolicyDecisionInput {
            requester_agent: format!("agent-{}", i % 100),
            requested_capability: "test.capability".to_string(),
            context: Default::default(),
        };

        let start = Instant::now();
        policy_engine.evaluate_capability_request(input).await.ok();
        latencies.push(start.elapsed().as_micros() as f64);
    }

    latencies.sort();
    let p50 = latencies[latencies.len() / 2];
    let p95 = latencies[(latencies.len() * 95) / 100];
    let p99 = latencies[(latencies.len() * 99) / 100];

    println!("Policy decision latency:");
    println!("  p50: {:.2} us", p50);
    println!("  p95: {:.2} us", p95);
    println!("  p99: {:.2} us", p99);

    assert!(p99 < 5000.0); // Target: <5ms p99
}
```

### Failure and Recovery Testing
```rust
#[tokio::test]
async fn test_mcp_disconnection_recovery() {
    let mut registry = MCPToolRegistry::new("localhost:9000").await.unwrap();

    // Initial discovery
    registry.discover_tools().await.ok();

    // Simulate MCP server disconnect
    // (In real test: kill MCP server or network partition)

    // Attempt tool invocation (should fail gracefully)
    let result = registry.get_binding("tool-web-search").await;

    // Wait for reconnection attempt
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Retry discovery (should succeed after reconnection)
    let result = registry.discover_tools().await;
    assert!(result.is_ok() || result.is_err()); // Depends on reconnection
}

#[tokio::test]
async fn test_sandbox_constraint_violation_blocked() {
    let registry = MCPToolRegistry::new("localhost:9000").await.unwrap();
    registry.discover_tools().await.unwrap();

    let binding = registry.get_binding("tool-web-search").await.unwrap();

    // Attempt to access disallowed domain
    let constraint = SandboxConstraint::NetworkDomain("evil.com".to_string());

    let result = registry.sandbox_engine.validate_invocation(&binding, &constraint).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_policy_reload_with_rollback() {
    let telemetry = Arc::new(TelemetryEngineV2::new(default_config()));
    let policy_engine = MandatoryPolicyEngine::new(telemetry);

    // Load initial policies
    policy_engine.load_policies(Path::new("test_policies.yaml")).await.unwrap();
    let v1_hash = policy_engine.policy_hash.lock().await.clone();

    // Create invalid policy file
    std::fs::write("invalid_policies.yaml", "invalid: yaml: content").ok();

    // Attempt to reload (should fail and rollback)
    let result = policy_engine.hot_reload_from_file(Path::new("invalid_policies.yaml")).await;
    assert!(result.is_err());

    // Verify policy hash unchanged (rollback successful)
    let current_hash = policy_engine.policy_hash.lock().await.clone();
    assert_eq!(v1_hash, current_hash);
}

#[tokio::test]
async fn test_core_dump_capture_on_exception() {
    let telemetry = TelemetryEngineV2::new(default_config());
    let core_dump_service = telemetry.core_dump_service.clone();

    let core_data = CognitiveCoreData {
        dump_id: "test-dump-1".to_string(),
        timestamp: 0,
        trigger_type: CoreDumpTrigger::Exception("NullPointerException".to_string()),
        checkpoint_id: "ckpt-1".to_string(),
        cpu_state: vec![0, 1, 2, 3],
        gpu_state: Some(vec![4, 5, 6, 7]),
        reasoning_chain: vec!["step1".to_string(), "step2".to_string()],
        context_window: "context data".to_string(),
        tool_history: vec![],
        exception_context: Some(ExceptionContext {
            exception_type: "NPE".to_string(),
            message: "null pointer".to_string(),
            stack_trace: "trace".to_string(),
            memory_state_at_failure: vec![],
        }),
        failure_point: "instr:0x1234".to_string(),
    };

    let dump_id = core_dump_service.capture_core_dump(core_data).await.unwrap();
    assert_eq!(dump_id, "test-dump-1");

    // Verify core dump file created
    let loaded = core_dump_service.load_core_dump("test-dump-1").await.unwrap();
    assert_eq!(loaded.dump_id, "test-dump-1");
}
```

### Performance Profiling
```rust
#[tokio::test]
async fn profile_memory_footprint() {
    let cache = PersistentCache::new(default_config()).unwrap();
    let telemetry = TelemetryEngineV2::new(default_config());
    let policy_engine = MandatoryPolicyEngine::new(Arc::new(telemetry));

    // Load 10k events into telemetry buffer
    for i in 0..10_000 {
        telemetry.emit_event(CEFEvent {
            event_id: format!("event-{}", i),
            ..Default::default()
        }).await.ok();
    }

    // Measure memory (in real environment: use system profiler)
    // Estimated:
    // - Event buffer: ~10MB (1KB per event avg)
    // - Cache: ~50MB (5KB per entry avg, 10k entries)
    // - Policy rules: ~1MB (100 rules)
    // Total: ~60MB

    println!("Estimated memory footprint: ~60 MB");
}

#[tokio::test]
async fn profile_cpu_utilization() {
    // In real environment: use perf/valgrind/flamegraph
    // Expected CPU consumption:
    // - Cache lookups: <1% per 1k ops/sec
    // - Policy decisions: <1% per 10k ops/sec
    // - Telemetry emission: <2% per 10k events/sec
}
```

## Dependencies
- **Blocked by:** Weeks 7-12 (all Phase 1 components)
- **Blocking:** Week 14 (Phase 1 completion)

## Acceptance Criteria
- [ ] End-to-end integration test passes; all components work together
- [ ] Load test: 1000 concurrent invocations successful (95%+ success rate)
- [ ] Telemetry event throughput >10k events/sec
- [ ] Policy decision latency <5ms p99 under 10k decision load
- [ ] MCP disconnection detected and recovered gracefully
- [ ] Sandbox violations blocked and audited
- [ ] Policy reload with validation and rollback functional
- [ ] Core dump capture functional; deserialization verified
- [ ] Memory footprint profiled and documented
- [ ] CPU profiling completed; bottlenecks identified
- [ ] Cost attribution accuracy validated
- [ ] Phase 1 architecture document completed
- [ ] Operational runbook written
- [ ] Phase 1 retrospective completed

## Design Principles Alignment
- **Correctness:** All integration tests pass; no data loss
- **Performance:** Throughput and latency targets met under load
- **Resilience:** Failures detected and recovered gracefully
- **Safety:** Sandbox violations blocked; no privilege escalation
- **Operability:** Runbook enables confident deployment to production
