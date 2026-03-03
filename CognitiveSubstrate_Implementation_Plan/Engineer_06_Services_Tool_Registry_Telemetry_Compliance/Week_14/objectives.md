# Engineer 6 — Services: Tool Registry, Telemetry & Compliance — Week 14

## Phase: Phase 1 (Weeks 7-14)

## Weekly Objective
Complete Phase 1 with bug fixes, performance optimization, and production-ready deployment of Tool Registry, Telemetry Engine, response caching, and Policy Engine. Transition to Phase 2 with clear handoff.

## Document References
- **Primary:** Section 6.2 (Phase 1 completion), all Weeks 7-13
- **Supporting:** Section 3.3.3-3.3.6 (all services)

## Deliverables
- [ ] Bug fixes from integration testing (Week 13)
  - Fix any failures identified in end-to-end tests
  - Address edge cases in concurrent access
  - Resolve performance regressions
- [ ] Performance optimization
  - Optimize cache key generation (currently SHA-256)
  - Optimize policy decision evaluation (short-circuit evaluation, caching)
  - Reduce telemetry event emission overhead
  - Profile and optimize critical paths
- [ ] Production hardening
  - Error handling completeness (no unwrap/panic in hot paths)
  - Graceful degradation (cache failure, policy reload failure)
  - Resource limits and backpressure (prevent OOM, handle slow consumers)
  - Monitoring and alerting (built-in health checks, metrics export)
- [ ] Deployment and operational readiness
  - Docker containerization for all services
  - Kubernetes manifests (if applicable)
  - Health check endpoints (liveness, readiness probes)
  - Metrics export (Prometheus format)
  - Logging format standardization (structured JSON logs)
- [ ] Documentation finalization
  - API documentation (Tool Registry, Telemetry, Policy Engine, response cache)
  - Deployment guide (prerequisites, configuration, security settings)
  - Monitoring guide (metrics, dashboards, alerts)
  - Troubleshooting guide (common issues, solutions, debug procedures)
  - Runbook for common operations (policy updates, cache clearing, log rotation)
- [ ] Phase 1 completion validation
  - All acceptance criteria from Weeks 7-14 verified
  - Integration tests pass on clean environment
  - Load tests pass at target throughput/latency
  - Security tests pass (sandbox, policy, audit)
- [ ] Phase 2 transition planning
  - Merge Tool Registry v1 with Compliance Engine (Week 17-20)
  - Merge Telemetry v1 with two-tier retention (Week 19-20)
  - Add Merkle-tree audit log (Phase 2 Week 17-18)
  - Known limitations documented
  - Risk assessment for Phase 2 completed
- [ ] Hand-off documentation
  - Architecture diagram (final Phase 1 state)
  - Known issues and mitigation strategies
  - Performance baselines and tuning recommendations
  - Escalation procedures for production incidents

## Technical Specifications

### Production Hardening Checklist
```rust
// Example: Cache error handling (before)
pub async fn get(&self, key: &str) -> Option<String> {
    self.cache.read().await.get(key).unwrap() // PANIC RISK!
}

// After: Graceful degradation
pub async fn get(&self, key: &str) -> Result<Option<String>, CacheError> {
    let cache = match self.cache.read().await {
        Ok(lock) => lock,
        Err(_) => {
            // Lock poisoned; return error instead of panicking
            return Err(CacheError::LockPoisoned);
        }
    };

    Ok(cache.get(key).cloned())
}

// Example: Policy Engine error handling
pub async fn evaluate_capability_request(&self, input: PolicyDecisionInput)
    -> Result<PolicyOutcome, PolicyError>
{
    let policies = match self.policies.read().await {
        Ok(lock) => lock,
        Err(_) => {
            // Policy lock poisoned; fail-safe to Deny
            return Ok(PolicyOutcome::Deny);
        }
    };

    // ... evaluation with explicit error handling for each condition
    Ok(outcome)
}

// Example: Resource limits
pub const MAX_CONCURRENT_EVALUATIONS: usize = 10_000;
pub const MAX_EVENT_BUFFER_SIZE: usize = 100_000;
pub const MAX_POLICY_RULES: usize = 1_000;

pub async fn emit_event(&self, event: CEFEvent) -> Result<(), EmitError> {
    let mut buffer = self.event_buffer.lock().await;

    if buffer.len() >= MAX_EVENT_BUFFER_SIZE {
        // Backpressure: drop oldest event
        buffer.pop_front();
        self.stats.dropped_events.fetch_add(1, Ordering::Relaxed);
    }

    buffer.push_back(event);
    Ok(())
}
```

### Metrics Export (Prometheus)
```rust
pub struct MetricsCollector {
    // Cache metrics
    cache_hits: Counter,
    cache_misses: Counter,
    cache_evictions: Counter,

    // Telemetry metrics
    events_emitted: Counter,
    events_dropped: Counter,
    event_buffer_size: Gauge,

    // Policy metrics
    decisions_made: Counter,
    decision_latency: Histogram,
    policy_version: Gauge,

    // Tool Registry metrics
    tool_invocations: Counter,
    sandbox_violations: Counter,
    mcp_reconnections: Counter,
}

impl MetricsCollector {
    pub fn export_prometheus(&self) -> String {
        format!(
            r#"
# HELP cache_hits_total Total cache hits
# TYPE cache_hits_total counter
cache_hits_total {}

# HELP cache_misses_total Total cache misses
# TYPE cache_misses_total counter
cache_misses_total {}

# HELP events_emitted_total Total events emitted
# TYPE events_emitted_total counter
events_emitted_total {}

# HELP decision_latency_seconds Policy decision latency in seconds
# TYPE decision_latency_seconds histogram
decision_latency_seconds_bucket{{le="0.001"}} {}
decision_latency_seconds_bucket{{le="0.005"}} {}
decision_latency_seconds_bucket{{le="0.01"}} {}

# HELP sandbox_violations_total Total sandbox violations
# TYPE sandbox_violations_total counter
sandbox_violations_total {}
"#,
            self.cache_hits.get(),
            self.cache_misses.get(),
            self.events_emitted.get(),
            // histogram buckets...
            self.sandbox_violations.get(),
        )
    }
}
```

### Health Check Endpoints
```rust
pub struct HealthCheckServer {
    telemetry: Arc<TelemetryEngineV2>,
    policy_engine: Arc<MandatoryPolicyEngine>,
    cache: Arc<PersistentCache>,
    tool_registry: Arc<MCPToolRegistry>,
}

#[derive(Serialize)]
pub struct HealthStatus {
    status: String,
    components: Components,
    timestamp: i64,
}

#[derive(Serialize)]
pub struct Components {
    telemetry: ComponentStatus,
    policy_engine: ComponentStatus,
    cache: ComponentStatus,
    mcp_registry: ComponentStatus,
}

#[derive(Serialize)]
pub struct ComponentStatus {
    status: String, // "healthy", "degraded", "unhealthy"
    message: Option<String>,
}

impl HealthCheckServer {
    pub async fn liveness(&self) -> HealthStatus {
        // Basic liveness: can we respond to requests?
        HealthStatus {
            status: "alive".to_string(),
            components: Components {
                telemetry: ComponentStatus { status: "alive".to_string(), message: None },
                policy_engine: ComponentStatus { status: "alive".to_string(), message: None },
                cache: ComponentStatus { status: "alive".to_string(), message: None },
                mcp_registry: ComponentStatus { status: "alive".to_string(), message: None },
            },
            timestamp: now(),
        }
    }

    pub async fn readiness(&self) -> HealthStatus {
        // Readiness: are all components functioning?
        let mut status = HealthStatus {
            status: "ready".to_string(),
            components: Components {
                telemetry: self.check_telemetry().await,
                policy_engine: self.check_policy_engine().await,
                cache: self.check_cache().await,
                mcp_registry: self.check_mcp_registry().await,
            },
            timestamp: now(),
        };

        // If any component is unhealthy, overall status is not ready
        if !matches!(status.components.telemetry.status.as_str(), "healthy") {
            status.status = "not_ready".to_string();
        }

        status
    }

    async fn check_telemetry(&self) -> ComponentStatus {
        // Verify telemetry event buffer not full
        match self.telemetry.event_buffer.lock().await.len() {
            len if len < 50_000 => ComponentStatus { status: "healthy".to_string(), message: None },
            len if len < 80_000 => ComponentStatus {
                status: "degraded".to_string(),
                message: Some(format!("Event buffer {}/100000", len)),
            },
            len => ComponentStatus {
                status: "unhealthy".to_string(),
                message: Some(format!("Event buffer full: {}/100000", len)),
            },
        }
    }

    async fn check_policy_engine(&self) -> ComponentStatus {
        // Verify policy engine can evaluate a test request
        let input = PolicyDecisionInput {
            requester_agent: "healthcheck".to_string(),
            requested_capability: "healthcheck".to_string(),
            context: Default::default(),
        };

        match self.policy_engine.evaluate_capability_request(input).await {
            Ok(_) => ComponentStatus { status: "healthy".to_string(), message: None },
            Err(e) => ComponentStatus {
                status: "unhealthy".to_string(),
                message: Some(format!("{:?}", e)),
            },
        }
    }

    async fn check_cache(&self) -> ComponentStatus {
        // Verify cache responds to get/set
        match self.cache.set(
            "healthcheck".to_string(),
            "ok".to_string(),
            60,
            FreshnessPolicy::Strict,
            "system",
        ).await {
            Ok(()) => ComponentStatus { status: "healthy".to_string(), message: None },
            Err(e) => ComponentStatus {
                status: "unhealthy".to_string(),
                message: Some(format!("{:?}", e)),
            },
        }
    }

    async fn check_mcp_registry(&self) -> ComponentStatus {
        // Verify MCP registry can list tools
        match self.tool_registry.discover_tools().await {
            Ok(tools) => ComponentStatus {
                status: "healthy".to_string(),
                message: Some(format!("{} tools available", tools.len())),
            },
            Err(e) => ComponentStatus {
                status: "unhealthy".to_string(),
                message: Some(format!("MCP disconnected: {:?}", e)),
            },
        }
    }
}
```

### Kubernetes Deployment Manifest Example
```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: engineer6-config
data:
  policies.yaml: |
    policies:
      - id: "allow-readonly"
        description: "Allow READ_ONLY"
        condition:
          type: "CapabilityMatches"
          pattern: "*.READ_ONLY"
        decision: "ALLOW"

---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: engineer6-services
spec:
  replicas: 3
  selector:
    matchLabels:
      app: engineer6
  template:
    metadata:
      labels:
        app: engineer6
    spec:
      containers:
      - name: services
        image: cognitive-substrate:engineer6-v1.0
        ports:
        - containerPort: 9000
          name: grpc
        - containerPort: 8080
          name: metrics
        env:
        - name: LOG_LEVEL
          value: "INFO"
        - name: POLICY_RELOAD_INTERVAL_SECS
          value: "300"
        livenessProbe:
          httpGet:
            path: /healthz/live
            port: 8080
          initialDelaySeconds: 10
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /healthz/ready
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 5
        resources:
          requests:
            memory: "256Mi"
            cpu: "250m"
          limits:
            memory: "512Mi"
            cpu: "500m"
        volumeMounts:
        - name: policies
          mountPath: /etc/policies
      volumes:
      - name: policies
        configMap:
          name: engineer6-config

---
apiVersion: v1
kind: Service
metadata:
  name: engineer6-services
spec:
  selector:
    app: engineer6
  ports:
  - name: grpc
    port: 9000
  - name: metrics
    port: 8080
  type: ClusterIP
```

### Documentation: API Reference
```markdown
# Engineer 6 Services API Reference

## Tool Registry API

### DiscoverTools
Discovers available tools from MCP server.

**Input:** None
**Output:** List<string> (tool names)
**Errors:** MCPConnectionError, DiscoveryTimeout

### GetBinding(tool_id: string)
Retrieves ToolBinding for a tool.

**Input:** tool_id
**Output:** ToolBinding
**Errors:** ToolNotFound, InvalidToolId

### InvokeToolWithCache(tool_id: string, input: string)
Invokes a tool with response caching.

**Input:** tool_id, input
**Output:** string (result)
**Errors:** SandboxViolation, PolicyDenial, ToolExecutionError

## Telemetry Engine API

### Subscribe(filter: SubscriptionFilter)
Subscribes to CEF events.

**Input:** SubscriptionFilter { event_types, actor_filter, resource_filter }
**Output:** Stream<CEFEvent>
**Errors:** SubscriptionError

### RecordInference(data: InferenceData)
Records model inference output.

**Input:** InferenceData { model_id, output, context_tokens, ... }
**Output:** None
**Errors:** RecordError

## Policy Engine API

### LoadPolicies(policy_file: Path)
Loads policies from file.

**Input:** policy_file
**Output:** None
**Errors:** InvalidPolicy, FileNotFound

### EvaluateCapabilityRequest(input: PolicyDecisionInput)
Evaluates capability grant request.

**Input:** PolicyDecisionInput { requester_agent, requested_capability, context }
**Output:** PolicyOutcome { Allow, Deny, RequireApproval, Audit, Warn }
**Errors:** PolicyEvaluationError

### ExportDecisionLogs(output_path: Path)
Exports policy decision logs.

**Input:** output_path
**Output:** u64 (number of decisions exported)
**Errors:** ExportError
```

## Dependencies
- **Blocked by:** Week 13 (integration testing and issue identification)
- **Blocking:** Phase 2 Week 15-24 (compliance and retention work)

## Acceptance Criteria
- [ ] All bugs from Week 13 testing fixed
- [ ] Performance optimization complete; targets met
- [ ] Graceful degradation for all failure modes
- [ ] Health check endpoints functional (liveness, readiness)
- [ ] Metrics export in Prometheus format
- [ ] Docker containerization complete
- [ ] Kubernetes manifests tested
- [ ] All documentation finalized and reviewed
- [ ] API reference complete
- [ ] Deployment guide written and tested
- [ ] Monitoring guide with sample dashboards
- [ ] Troubleshooting guide covers >20 common issues
- [ ] Phase 2 transition plan documented
- [ ] All Phase 1 acceptance criteria verified

## Design Principles Alignment
- **Production-ready:** Error handling, resource limits, monitoring built-in
- **Operational excellence:** Health checks, metrics, structured logs
- **Reliability:** Graceful degradation; no single point of failure
- **Maintainability:** Clear documentation; runbooks for common tasks
- **Scalability:** Horizontal scaling via Kubernetes, metrics-driven auto-scaling
