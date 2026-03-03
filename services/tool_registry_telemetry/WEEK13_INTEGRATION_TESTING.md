# Week 13 — Integration Testing & Hardening: Phase 1 Services Validation

**Principal Software Engineer Technical Design Document**
**XKernal Cognitive Substrate OS Project**
**Date: Week 13 | Status: Production Readiness**

---

## Executive Summary

Week 13 validates the Phase 1 service architecture through comprehensive end-to-end integration testing and performance hardening. This document specifies the complete testing strategy for the Tool Registry, Telemetry Engine, Response Caching, and Policy Engine, exercising all five production tools through realistic workloads. The validation ensures sub-5ms policy decision latency, >10k events/sec telemetry throughput, cache efficiency metrics, accurate cost attribution, and resilience against both infrastructure failures and adversarial scenarios. Successful completion of Week 13 acceptance criteria enables Phase 2 feature development with confidence in foundational architecture.

---

## Problem Statement

Phase 1 services have been developed in isolation. Integration risks include:
- **Service coupling failures**: Undefined behavior at service boundaries; race conditions in distributed event ordering
- **Performance unknowns**: Latency propagation through policy → cache → telemetry pipeline; memory footprint under load
- **Reliability gaps**: MCP server disconnections, sandbox violations, policy reload atomicity, event streaming resilience
- **Cost attribution accuracy**: Per-tool/per-agent/per-time-period granularity; hardware counter validation
- **Security blind spots**: Adversarial policy enforcement, sandbox escape attempts, audit log integrity

This document establishes the testing framework to eliminate these risks before Phase 2.

---

## Architecture

### Phase 1 Service Integration Pipeline

```
┌─────────────────┐
│  Tool Request   │
└────┬────────────┘
     │
┌────▼──────────────────────────────────┐
│  Policy Engine                         │
│  • Adversarial detection               │
│  • Rate limiting decision              │
│  • Sandbox constraint validation       │
└────┬──────────────────────────────────┘
     │
┌────▼──────────────────────────────────┐
│  Tool Registry                         │
│  • MCP binding resolution              │
│  • Sandbox config composition          │
│  • Tool discovery caching              │
└────┬──────────────────────────────────┘
     │
┌────▼──────────────────────────────────┐
│  Tool Invocation                       │
│  • Sandbox enforcement (seccomp)       │
│  • Resource isolation (cgroups)        │
│  • MCP RPC communication               │
└────┬──────────────────────────────────┘
     │
┌────▼──────────────────────────────────┐
│  Response Caching                      │
│  • Key derivation (input+context)      │
│  • LRU eviction                        │
│  • TTL enforcement                     │
└────┬──────────────────────────────────┘
     │
┌────▼──────────────────────────────────┐
│  Telemetry Engine                      │
│  • Event streaming (buffering)         │
│  • Cost attribution (per-dimension)    │
│  • Audit log (immutable sequence)      │
└────┬──────────────────────────────────┘
     │
┌────▼──────────────────────────────────┐
│  Storage/Analytics                     │
│  • DuckDB analytics (local)            │
│  • S3 export (long-term)               │
│  • Real-time dashboards                │
└────────────────────────────────────────┘
```

### Service Dependencies

| Service | Depends On | Interface | SLA |
|---------|-----------|-----------|-----|
| Policy Engine | — | gRPC, local decision | <5ms p99 |
| Tool Registry | MCP server, local storage | gRPC, environment vars | <20ms p99 |
| Tool Invocation | Sandbox runtime (seccomp, cgroups) | subprocess, IPC | <500ms p99 |
| Response Cache | Local memory, persistent KV | HashMap, RocksDB | <2ms p99 |
| Telemetry Engine | Event queue, file I/O | Channel, mmap | <1ms p99 |

---

## Implementation

### Test Infrastructure Setup

```rust
#[cfg(test)]
mod integration_tests {
    use xkernal::{
        policy_engine::PolicyEngine,
        tool_registry::ToolRegistry,
        response_cache::ResponseCache,
        telemetry_engine::TelemetryEngine,
        sandbox::SandboxRuntime,
    };
    use std::sync::Arc;
    use tokio::sync::RwLock;

    struct TestEnvironment {
        policy_engine: Arc<PolicyEngine>,
        tool_registry: Arc<ToolRegistry>,
        cache: Arc<ResponseCache>,
        telemetry: Arc<TelemetryEngine>,
        sandbox: Arc<SandboxRuntime>,
    }

    impl TestEnvironment {
        async fn new() -> Self {
            let policy_engine = Arc::new(PolicyEngine::default());
            let tool_registry = Arc::new(ToolRegistry::new());
            let cache = Arc::new(ResponseCache::with_capacity(10000));
            let telemetry = Arc::new(TelemetryEngine::with_buffer(50000));
            let sandbox = Arc::new(SandboxRuntime::initialize().unwrap());

            Self {
                policy_engine,
                tool_registry,
                cache,
                telemetry,
                sandbox,
            }
        }

        async fn teardown(&self) {
            self.telemetry.flush().await;
            self.cache.clear();
            self.sandbox.cleanup().unwrap();
        }
    }
}
```

### Complete Workflow Test

```rust
#[tokio::test]
async fn test_complete_workflow() {
    let env = TestEnvironment::new().await;

    // Scenario: Agent invokes calculator tool with add operation
    let tool_request = ToolRequest {
        tool_name: "calculator".into(),
        agent_id: "agent_001".into(),
        params: serde_json::json!({
            "operation": "add",
            "operands": [42, 8]
        }),
    };

    // 1. Policy Engine: Authorize request
    let policy_decision = env.policy_engine
        .evaluate(&tool_request)
        .await
        .expect("policy decision failed");

    assert!(policy_decision.allowed);
    assert_eq!(policy_decision.sandbox_constraints.memory_limit, 256 * 1024 * 1024);

    // 2. Tool Registry: Resolve MCP binding
    let tool_metadata = env.tool_registry
        .discover_tool("calculator")
        .await
        .expect("tool discovery failed");

    assert_eq!(tool_metadata.tool_name, "calculator");
    assert!(tool_metadata.mcp_servers.len() > 0);

    // 3. Response Cache: Check for cached result
    let cache_key = env.cache.derive_key(&tool_request);
    let cached = env.cache.get(&cache_key).await;
    assert!(cached.is_none());

    // 4. Tool Invocation: Execute with sandbox
    let result = env.sandbox
        .invoke_tool(
            &tool_metadata,
            &tool_request.params,
            &policy_decision.sandbox_constraints,
        )
        .await
        .expect("tool invocation failed");

    assert_eq!(result.output, "50");

    // 5. Response Cache: Store result
    env.cache.set(&cache_key, &result, std::time::Duration::from_secs(300)).await;
    let cached_again = env.cache.get(&cache_key).await;
    assert!(cached_again.is_some());

    // 6. Telemetry Engine: Record metrics
    let telemetry_event = TelemetryEvent {
        timestamp: std::time::SystemTime::now(),
        event_type: EventType::ToolInvocation,
        agent_id: tool_request.agent_id.clone(),
        tool_name: tool_request.tool_name.clone(),
        status: "success".into(),
        duration_ms: 45,
        cache_hit: false,
        tokens_used: 150,
        cost_usd: 0.0015,
    };

    env.telemetry.emit(&telemetry_event).await;

    // 7. Policy Decision: Log audit trail
    let audit_event = AuditEvent {
        timestamp: std::time::SystemTime::now(),
        decision: "allowed".into(),
        agent_id: tool_request.agent_id.clone(),
        tool_name: tool_request.tool_name.clone(),
        reason: "policy_approved".into(),
    };

    env.telemetry.emit_audit(&audit_event).await;

    env.teardown().await;
}
```

### Load Testing: 1000 Concurrent Invocations

```rust
#[tokio::test(flavor = "multi_thread", worker_threads = 16)]
async fn test_load_1000_concurrent() {
    let env = Arc::new(TestEnvironment::new().await);
    let mut handles = vec![];

    let start = std::time::Instant::now();

    for agent_id in 0..1000 {
        let env_clone = Arc::clone(&env);
        let handle = tokio::spawn(async move {
            for iteration in 0..10 {
                let tool_request = ToolRequest {
                    tool_name: format!("tool_{}", agent_id % 5),
                    agent_id: format!("agent_{}", agent_id),
                    params: serde_json::json!({"iteration": iteration}),
                };

                let policy_decision = env_clone.policy_engine.evaluate(&tool_request).await;
                if !policy_decision.is_ok() {
                    return Err("policy failed");
                }

                let tool_metadata = env_clone.tool_registry
                    .discover_tool(&tool_request.tool_name)
                    .await;
                if tool_metadata.is_err() {
                    return Err("discovery failed");
                }

                let result = env_clone.sandbox
                    .invoke_tool(
                        &tool_metadata.unwrap(),
                        &tool_request.params,
                        &policy_decision.unwrap().sandbox_constraints,
                    )
                    .await;

                if result.is_err() {
                    return Err("invocation failed");
                }

                env_clone.telemetry.emit(&TelemetryEvent {
                    timestamp: std::time::SystemTime::now(),
                    event_type: EventType::ToolInvocation,
                    agent_id: tool_request.agent_id.clone(),
                    tool_name: tool_request.tool_name.clone(),
                    status: "success".into(),
                    duration_ms: 12,
                    cache_hit: false,
                    tokens_used: 100,
                    cost_usd: 0.001,
                }).await;
            }
            Ok::<_, &str>(())
        });

        handles.push(handle);
    }

    let mut success_count = 0;
    for handle in handles {
        if handle.await.unwrap().is_ok() {
            success_count += 1;
        }
    }

    let elapsed = start.elapsed();
    println!("1000 agents × 10 invocations = 10,000 total in {:.2}s", elapsed.as_secs_f64());
    println!("Throughput: {:.0} invocations/sec", 10000.0 / elapsed.as_secs_f64());

    assert!(success_count >= 990);
    assert!(elapsed.as_secs_f64() < 120.0);

    env.teardown().await;
}
```

### Telemetry Throughput Test

```rust
#[tokio::test]
async fn test_telemetry_throughput() {
    let env = TestEnvironment::new().await;
    let event_count = 50000;

    let start = std::time::Instant::now();

    for i in 0..event_count {
        let event = TelemetryEvent {
            timestamp: std::time::SystemTime::now(),
            event_type: EventType::ToolInvocation,
            agent_id: format!("agent_{}", i % 100),
            tool_name: format!("tool_{}", i % 5),
            status: "success".into(),
            duration_ms: 25,
            cache_hit: i % 3 == 0,
            tokens_used: 200 + (i as u32 % 100),
            cost_usd: 0.002,
        };

        env.telemetry.emit(&event).await;
    }

    env.telemetry.flush().await;
    let elapsed = start.elapsed();

    let throughput = event_count as f64 / elapsed.as_secs_f64();
    println!("Telemetry throughput: {:.0} events/sec", throughput);

    assert!(throughput > 10000.0, "Must exceed 10k events/sec; got {:.0}", throughput);

    env.teardown().await;
}
```

### Policy Latency Test

```rust
#[tokio::test]
async fn test_policy_latency() {
    let env = TestEnvironment::new().await;
    let mut latencies = vec![];

    for i in 0..10000 {
        let tool_request = ToolRequest {
            tool_name: format!("tool_{}", i % 5),
            agent_id: format!("agent_{}", i % 100),
            params: serde_json::json!({"index": i}),
        };

        let start = std::time::Instant::now();
        let _ = env.policy_engine.evaluate(&tool_request).await;
        latencies.push(start.elapsed().as_micros());
    }

    latencies.sort();
    let p50 = latencies[5000];
    let p99 = latencies[9900];

    println!("Policy latency — p50: {}μs, p99: {}μs", p50, p99);

    assert!(p99 < 5000, "p99 must be <5ms; got {}μs", p99);

    env.teardown().await;
}
```

### MCP Disconnection & Recovery Test

```rust
#[tokio::test]
async fn test_mcp_disconnection() {
    let env = TestEnvironment::new().await;

    // Simulate MCP server disconnect
    env.tool_registry.inject_disconnect("server_001").await;

    // Attempt invocation should fail with explicit error
    let result = env.sandbox
        .invoke_tool(
            &ToolMetadata {
                tool_name: "calculator".into(),
                mcp_servers: vec!["server_001".into()],
                ..Default::default()
            },
            &serde_json::json!({"x": 1}),
            &Default::default(),
        )
        .await;

    assert!(matches!(result, Err(SandboxError::MCPServerUnavailable)));

    // Reconnection should restore service
    env.tool_registry.inject_reconnect("server_001").await;

    let retry = env.sandbox
        .invoke_tool(
            &ToolMetadata {
                tool_name: "calculator".into(),
                mcp_servers: vec!["server_001".into()],
                ..Default::default()
            },
            &serde_json::json!({"x": 1}),
            &Default::default(),
        )
        .await;

    assert!(retry.is_ok());

    env.teardown().await;
}
```

### Sandbox Violation Detection Test

```rust
#[tokio::test]
async fn test_sandbox_violation() {
    let env = TestEnvironment::new().await;

    // Inject adversarial request attempting container escape
    let adversarial_payload = serde_json::json!({
        "command": "cat /etc/shadow"
    });

    let tight_constraints = SandboxConstraints {
        memory_limit: 64 * 1024 * 1024,
        allowed_syscalls: vec!["read".into(), "write".into()],
        network_access: false,
        filesystem_access: false,
    };

    let result = env.sandbox
        .invoke_tool(
            &ToolMetadata {
                tool_name: "restricted_tool".into(),
                ..Default::default()
            },
            &adversarial_payload,
            &tight_constraints,
        )
        .await;

    // Should block and emit audit event
    assert!(matches!(result, Err(SandboxError::ViolationDetected(_))));

    // Verify audit log captured violation
    let audit_logs = env.telemetry.get_audit_logs().await;
    assert!(audit_logs.iter().any(|e| e.event_type == "sandbox_violation"));

    env.teardown().await;
}
```

---

## Testing Strategy

### Load Testing Phases

| Phase | Concurrency | Tools Exercised | Duration | Key Metrics |
|-------|-------------|-----------------|----------|-------------|
| Phase A | 100 agents | All 5 production | <10m | <500ms p99, zero errors |
| Phase B | 1000 agents | All 5 production | <30m | <1s p99, >98% success |
| Phase C | 10000 agents | All 5 production | <60m | <5s p99, >95% success |
| Chaos | 1-100 agents | All 5 production | <30m | Recovery <30s, zero data loss |

### Failure Recovery Scenarios

1. **MCP Server Disconnect/Reconnect**: Verify automatic recovery within 30 seconds
2. **Sandbox Violation Attempt**: Confirm 100% prevention; audit logging
3. **Policy Reload with Rollback**: Validate atomic reload; zero downtime
4. **Event Streaming Disconnect**: Verify queue backpressure; no event loss
5. **Core Dump Capture**: Confirm forensics data collection on panic

---

## Acceptance Criteria

**Latency & Throughput:**
- Policy decision latency: p99 <5ms
- Tool Registry discovery: p99 <20ms
- Response Cache lookup: p99 <2ms
- Telemetry emission: >10,000 events/sec
- Tool invocation: p99 <500ms

**Reliability:**
- 1000 concurrent agents: >99% success rate
- MCP disconnection recovery: <30 seconds
- Sandbox violation detection: 100% (zero escapes)
- Policy reload: zero downtime, rollback on error
- Event ordering: causally consistent across 10,000 events

**Cost Attribution:**
- Per-tool accuracy: ±2% vs hardware counters
- Per-agent granularity: millisecond precision
- Per-time-period reports: available within 5 seconds

**Security & Compliance:**
- Audit log immutability: append-only, verified
- Policy enforcement: zero bypass attempts succeed
- Sandbox integrity: zero container escapes

---

## Design Principles

**1. Observability First**
Every integration point emits structured telemetry: latency, errors, resource usage. Real-time dashboards enable rapid issue diagnosis.

**2. Fail Fast, Recover Gracefully**
Services detect failures within critical path and propagate errors immediately. Background services implement retry queues to avoid request loss.

**3. Cost Attribution Precision**
Every tool invocation maps to agent ID, time period, and resource usage. Per-dimension cost rollups support accurate chargeback.

**4. Security by Default**
Sandbox constraints are strict by default. Only agents with explicit policy approval access network or unrestricted filesystem. Audit logs capture all policy decisions and violations immutably.

**5. Performance Under Concurrency**
Lockfree data structures (Arc, RwLock) minimize contention. Per-worker buffers in telemetry engine aggregate events without global locks. Cache uses consistent hashing for horizontal scalability.

---

## Deliverables

**1. Test Suite** (Rust)
- test_complete_workflow() — Full 5-tool pipeline validation
- test_load_1000_concurrent() — Sustained 1000-agent workload
- test_telemetry_throughput() — >10k events/sec validation
- test_policy_latency() — p99 <5ms latency
- test_mcp_disconnection() — Failure and recovery scenarios
- test_sandbox_violation() — Adversarial escape attempts blocked

**2. Phase 1 Architecture Document**
- Service dependency graph
- Data flow diagrams
- Interface specifications (gRPC, IPC, file formats)

**3. Operational Runbook**
- Deployment checklist
- Monitoring dashboards and alerts
- Troubleshooting decision tree
- On-call escalation procedures

**4. Performance Tuning Guide**
- Cache eviction policies and sizing
- Telemetry buffer configuration
- Policy engine optimization (decision caching)
- Sandbox constraint presets by agent profile

**5. Phase 1 Retrospective**
- Known limitations: single-host telemetry, in-memory cache (no distributed state)
- Lessons learned: sandbox initialization overhead (500ms), policy reload atomicity critical
- Phase 2 risk assessment: distributed cache coherency, cross-region policy propagation

---

**Document Version**: 1.0
**Date**: Week 13, Phase 1 Completion
**Status**: Final — Ready for Production Review
**Author**: Principal Software Engineer, XKernal Cognitive Substrate OS Team
