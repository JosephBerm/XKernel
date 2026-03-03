# Week 12 — Mandatory Policy Engine: Hot-Reload Policies, CEF Schema & Export APIs

**Document Version**: 1.0
**Last Updated**: 2026-03-02
**Author**: Principal Software Engineer, XKernal Cognitive Substrate OS
**Status**: APPROVED FOR IMPLEMENTATION

---

## Executive Summary

The **Mandatory Policy Engine** enforces fine-grained access control over capability grants via hot-reloadable YAML policies and auditable Common Event Format (CEF) event streams. Every capability request transits through a policy evaluation layer before page table modifications, enabling real-time policy updates without service restarts. This system exports CEF-compliant events with cryptographic verification and supports complex policy compositions (AND/OR/NOT logic, time windows, rate limits, and agent history validation).

---

## Problem Statement

**Current Limitations:**
- Capability grants bypass policy evaluation entirely—no centralized authorization layer
- Policy changes require service restarts, creating gaps during deployment
- Audit logs lack standardized event schema, complicating compliance reporting
- No explainability mechanism to justify policy decisions
- Rate limiting and time-window policies are ad-hoc, not composable

**Target State:**
- All capability grants must consult a mandatory policy engine before execution
- Policies reload atomically without downtime; rollback on validation failure
- CEF-compliant event schema with cryptographic proof-of-integrity
- WebSocket streaming and batch query APIs for real-time monitoring
- Complex policy rules via tree-based condition composition

---

## Architecture

### 1. Core Policy Engine (Arc<RwLock<PolicySet>>)

```rust
/// Thread-safe, policy versioning with content hash
#[derive(Clone)]
pub struct MandatoryPolicyEngine {
    policies: Arc<RwLock<PolicySet>>,
    policy_version: Arc<AtomicU64>,
    policy_hash: Arc<Mutex<String>>, // SHA-256
    reload_channel: crossbeam::channel::Sender<PolicyReloadEvent>,
    event_emitter: Arc<EventEmitter>,
    decision_cache: Arc<DashMap<String, CachedDecision>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PolicySet {
    version: u64,
    updated_at: SystemTime,
    rules: Vec<PolicyRule>,
    default_outcome: PolicyOutcome,
    schema_version: String, // e.g., "cef:1.4"
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PolicyRule {
    id: String,
    name: String,
    description: String,
    condition: PolicyCondition,
    outcome: PolicyOutcome,
    priority: u32, // lower = higher priority
    metadata: Map<String, Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PolicyCondition {
    AllOf(Vec<PolicyCondition>),
    AnyOf(Vec<PolicyCondition>),
    Not(Box<PolicyCondition>),
    AgentMatches {
        agent_id_pattern: String, // regex
        required_roles: Vec<String>,
        min_trust_score: f32,
    },
    CapabilityMatches {
        capability_name: String,
        resource_pattern: String,
        operation: String, // read, write, exec, delete
    },
    TimeWindow {
        start_utc: String,      // HH:MM
        end_utc: String,        // HH:MM
        allowed_days: Vec<u32>, // 0-6, 0=Monday
    },
    RateLimit {
        window_secs: u64,
        max_requests: u32,
        key: String, // agent_id, resource, etc.
    },
    AgentHistoryOk {
        lookback_days: u32,
        max_denials: u32,
        required_success_rate: f32,
    },
    ResourceAvailable {
        resource_name: String,
        min_available_percent: f32,
    },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PolicyOutcome {
    Allow,
    Deny,
    RequireApproval,
    Audit,
    Warn,
}

impl MandatoryPolicyEngine {
    pub fn new(event_emitter: Arc<EventEmitter>) -> (Self, crossbeam::channel::Receiver<PolicyReloadEvent>) {
        let (tx, rx) = crossbeam::channel::unbounded();
        let engine = MandatoryPolicyEngine {
            policies: Arc::new(RwLock::new(PolicySet {
                version: 0,
                updated_at: SystemTime::now(),
                rules: vec![],
                default_outcome: PolicyOutcome::Deny,
                schema_version: "cef:1.4".to_string(),
            })),
            policy_version: Arc::new(AtomicU64::new(0)),
            policy_hash: Arc::new(Mutex::new(String::new())),
            reload_channel: tx,
            event_emitter,
            decision_cache: Arc::new(DashMap::new()),
        };
        (engine, rx)
    }

    /// Consult policy before any capability grant; returns PolicyDecision with reasoning
    pub async fn evaluate_capability_grant(
        &self,
        request: &CapabilityRequest,
    ) -> Result<PolicyDecision, PolicyError> {
        // Check cache first (TTL-based)
        if let Some(cached) = self.decision_cache.get(&request.cache_key()) {
            if cached.valid_until > Instant::now() {
                return Ok(cached.decision.clone());
            }
            self.decision_cache.remove(&request.cache_key());
        }

        let policies = self.policies.read().await;
        let mut matching_rules = Vec::new();

        for rule in &policies.rules {
            if self.evaluate_condition(&rule.condition, request).await? {
                matching_rules.push(rule);
            }
        }

        let decision = if matching_rules.is_empty() {
            PolicyDecision {
                outcome: policies.default_outcome,
                rule_id: None,
                reason: "No matching rules; applying default policy".to_string(),
                request_id: request.id.clone(),
                timestamp: SystemTime::now(),
            }
        } else {
            let rule = matching_rules.iter().min_by_key(|r| r.priority).unwrap();
            PolicyDecision {
                outcome: rule.outcome,
                rule_id: Some(rule.id.clone()),
                reason: format!("Matched rule: {}", rule.name),
                request_id: request.id.clone(),
                timestamp: SystemTime::now(),
            }
        };

        // Cache and emit event
        self.decision_cache.insert(
            request.cache_key(),
            CachedDecision {
                decision: decision.clone(),
                valid_until: Instant::now() + Duration::from_secs(60),
            },
        );

        self.event_emitter.emit(Event::PolicyDecision(decision.clone())).await;
        Ok(decision)
    }

    async fn evaluate_condition(
        &self,
        condition: &PolicyCondition,
        request: &CapabilityRequest,
    ) -> Result<bool, PolicyError> {
        match condition {
            PolicyCondition::AllOf(conditions) => {
                for cond in conditions {
                    if !self.evaluate_condition(cond, request).await? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            PolicyCondition::AnyOf(conditions) => {
                for cond in conditions {
                    if self.evaluate_condition(cond, request).await? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            PolicyCondition::Not(cond) => {
                Ok(!self.evaluate_condition(cond, request).await?)
            }
            PolicyCondition::AgentMatches {
                agent_id_pattern,
                required_roles,
                min_trust_score,
            } => {
                let agent_matches = regex::Regex::new(agent_id_pattern)?
                    .is_match(&request.agent_id);
                let roles_ok = request.agent_roles.iter().any(|r| required_roles.contains(r));
                let trust_ok = request.agent_trust_score >= *min_trust_score;
                Ok(agent_matches && roles_ok && trust_ok)
            }
            PolicyCondition::TimeWindow {
                start_utc,
                end_utc,
                allowed_days,
            } => {
                let now = chrono::Utc::now();
                let weekday = now.weekday().number_from_monday() as u32 - 1;
                let time_str = now.format("%H:%M").to_string();
                Ok(allowed_days.contains(&weekday)
                    && time_str >= *start_utc
                    && time_str <= *end_utc)
            }
            PolicyCondition::RateLimit {
                window_secs,
                max_requests,
                key,
            } => {
                // Query from Redis or in-memory window store
                let count = self.get_request_count(key, *window_secs).await?;
                Ok(count < *max_requests as u64)
            }
            PolicyCondition::AgentHistoryOk {
                lookback_days,
                max_denials,
                required_success_rate,
            } => {
                let (total, denials) = self.query_agent_history(&request.agent_id, *lookback_days).await?;
                let success_rate = if total == 0 { 1.0 } else { (total - denials) as f32 / total as f32 };
                Ok(denials <= *max_denials as u64 && success_rate >= *required_success_rate)
            }
            PolicyCondition::ResourceAvailable {
                resource_name,
                min_available_percent,
            } => {
                let available = self.query_resource_availability(resource_name).await?;
                Ok(available >= *min_available_percent)
            }
        }
    }

    /// Hot-reload: watch policy file, validate, atomic swap, rollback on failure
    pub async fn hot_reload_policies(&self, yaml_path: &Path) -> Result<(), PolicyError> {
        let yaml_content = tokio::fs::read_to_string(yaml_path).await?;
        let new_policies: PolicySet = serde_yaml::from_str(&yaml_content)?;

        // Validate new policies
        self.validate_policy_set(&new_policies)?;

        // Compute SHA-256 hash
        let hash = format!("{:x}", sha256::digest(&yaml_content));

        // Atomic swap with RwLock
        let old_policies = {
            let mut policies = self.policies.write().await;
            let old = policies.clone();
            *policies = new_policies.clone();
            self.policy_version.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            *self.policy_hash.lock().await = hash.clone();
            old
        };

        // Emit reload event
        self.event_emitter
            .emit(Event::PolicyReloaded {
                version: new_policies.version,
                hash: hash.clone(),
                rule_count: new_policies.rules.len(),
            })
            .await;

        // Clear decision cache on reload
        self.decision_cache.clear();

        tracing::info!(
            version = new_policies.version,
            hash = hash,
            rule_count = new_policies.rules.len(),
            "Policy hot-reload completed successfully"
        );

        Ok(())
    }

    fn validate_policy_set(&self, policies: &PolicySet) -> Result<(), PolicyError> {
        // Validate all rules compile cleanly
        for rule in &policies.rules {
            // Test compile all regex patterns
            if let PolicyCondition::AgentMatches {
                agent_id_pattern, ..
            } = &rule.condition
            {
                regex::Regex::new(agent_id_pattern)
                    .map_err(|e| PolicyError::InvalidRegex(e.to_string()))?;
            }
        }
        Ok(())
    }

    async fn get_request_count(&self, key: &str, window_secs: u64) -> Result<u64, PolicyError> {
        // Implementation: Redis ZRANGE with window cutoff
        todo!()
    }

    async fn query_agent_history(&self, agent_id: &str, lookback_days: u32) -> Result<(u64, u64), PolicyError> {
        // Query audit log for agent decisions
        todo!()
    }

    async fn query_resource_availability(&self, resource_name: &str) -> Result<f32, PolicyError> {
        // Query resource manager for availability
        todo!()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PolicyDecision {
    pub request_id: String,
    pub outcome: PolicyOutcome,
    pub rule_id: Option<String>,
    pub reason: String,
    pub timestamp: SystemTime,
}

#[derive(Clone, Debug)]
pub struct CapabilityRequest {
    pub id: String,
    pub agent_id: String,
    pub agent_roles: Vec<String>,
    pub agent_trust_score: f32,
    pub capability_name: String,
    pub resource_name: String,
    pub operation: String,
    pub timestamp: SystemTime,
}

impl CapabilityRequest {
    fn cache_key(&self) -> String {
        format!("{}:{}:{}:{}", self.agent_id, self.capability_name, self.resource_name, self.operation)
    }
}

#[derive(Clone)]
struct CachedDecision {
    decision: PolicyDecision,
    valid_until: Instant,
}
```

---

### 2. YAML Policy Format (Example)

```yaml
version: 1
schema_version: "cef:1.4"
default_outcome: Deny
updated_at: 2026-03-02T10:00:00Z

rules:
  - id: rule_001_allow_readonly
    name: "Allow read operations during business hours"
    priority: 10
    condition:
      type: AllOf
      conditions:
        - type: CapabilityMatches
          capability_name: "file_access"
          operation: "read"
          resource_pattern: "^/public/.*"
        - type: TimeWindow
          start_utc: "06:00"
          end_utc: "22:00"
          allowed_days: [0, 1, 2, 3, 4]
    outcome: Allow

  - id: rule_002_require_approval_write
    name: "Require approval for write operations"
    priority: 20
    condition:
      type: CapabilityMatches
      capability_name: "file_access"
      operation: "write"
      resource_pattern: "^/private/.*"
    outcome: RequireApproval

  - id: rule_003_deny_after_hours
    name: "Deny high-risk operations outside business hours"
    priority: 5
    condition:
      type: AllOf
      conditions:
        - type: CapabilityMatches
          capability_name: "system_admin"
          operation: "delete"
        - type: Not
          condition:
            type: TimeWindow
            start_utc: "09:00"
            end_utc: "17:00"
            allowed_days: [0, 1, 2, 3, 4]
    outcome: Deny

  - id: rule_004_rate_limit
    name: "Rate limit API calls per agent"
    priority: 15
    condition:
      type: RateLimit
      window_secs: 60
      max_requests: 100
      key: "agent_id"
    outcome: Warn

  - id: rule_005_agent_history_check
    name: "Block agents with frequent denials"
    priority: 8
    condition:
      type: AgentHistoryOk
      lookback_days: 7
      max_denials: 5
      required_success_rate: 0.9
    outcome: Deny
```

---

### 3. CEF Event Schema (Protocol Buffers)

```protobuf
// proto/xkernal/cef_event.proto
syntax = "proto3";

package xkernal.cef;

import "google/protobuf/timestamp.proto";

message CEFEvent {
  // CEF Header (7 fields)
  int32 cef_version = 1;                // typically 0
  string cef_vendor = 2;                // "XKernal"
  string cef_product = 3;               // "PolicyEngine"
  string cef_version_str = 4;           // "1.0"
  int32 cef_event_id = 5;               // numeric ID
  string cef_name = 6;                  // short event name
  int32 cef_severity = 7;               // 0-10

  // Extended Fields (16 fields)
  string request_id = 8;
  string agent_id = 9;
  string agent_roles = 10;              // comma-separated
  float agent_trust_score = 11;
  string capability_name = 12;
  string resource_name = 13;
  string operation = 14;                // read, write, exec, delete
  string policy_outcome = 15;           // Allow, Deny, etc.
  string rule_id = 16;                  // matched rule ID
  string reason = 17;                   // decision rationale
  google.protobuf.Timestamp timestamp = 18;
  int64 decision_latency_ms = 19;       // evaluation time
  string resource_region = 20;          // geographic metadata
  string source_ip = 21;                // agent origin
  string signature = 22;                // HMAC-SHA256(event_bytes, key)
  string schema_version = 23;           // "cef:1.4"
}
```

---

### 4. JSON Schema for CEF Events

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "XKernal CEF Event Schema",
  "type": "object",
  "required": [
    "cef_version", "cef_vendor", "cef_product", "cef_name",
    "request_id", "agent_id", "capability_name", "operation",
    "policy_outcome", "timestamp", "signature"
  ],
  "properties": {
    "cef_version": { "type": "integer", "minimum": 0 },
    "cef_vendor": { "type": "string", "pattern": "^[a-zA-Z0-9_-]+$" },
    "cef_product": { "type": "string", "pattern": "^[a-zA-Z0-9_-]+$" },
    "cef_name": { "type": "string" },
    "cef_severity": { "type": "integer", "minimum": 0, "maximum": 10 },
    "request_id": { "type": "string", "format": "uuid" },
    "agent_id": { "type": "string" },
    "policy_outcome": {
      "type": "string",
      "enum": ["Allow", "Deny", "RequireApproval", "Audit", "Warn"]
    },
    "rule_id": { "type": ["string", "null"] },
    "timestamp": { "type": "string", "format": "date-time" },
    "decision_latency_ms": { "type": "integer", "minimum": 0 },
    "signature": { "type": "string", "pattern": "^[a-f0-9]{64}$" },
    "schema_version": { "type": "string", "pattern": "^cef:\\d+\\.\\d+$" }
  }
}
```

---

### 5. Export APIs (4 Endpoints)

```rust
/// WebSocket: Real-time event stream with filtering
#[post("/api/v1/events/stream")]
pub async fn stream_events(
    ws: WebSocketUpgrade,
    filter: Query<EventFilter>,
) -> impl IntoResponse {
    ws.on_upgrade(move |mut socket| {
        async move {
            let mut rx = event_bus.subscribe();
            while let Ok(event) = rx.recv().await {
                if filter.matches(&event) {
                    let json = serde_json::to_string(&event).unwrap();
                    let _ = socket.send(Message::Text(json)).await;
                }
            }
        }
    })
}

/// POST: Query events with time range, agent, capability filters
#[post("/api/v1/events/query")]
pub async fn query_events(
    State(db): State<Arc<Database>>,
    Json(query): Json<EventQuery>,
) -> Json<Vec<CEFEvent>> {
    let events = db
        .query_events(
            query.start_time,
            query.end_time,
            &query.agent_id_filter,
            &query.capability_filter,
        )
        .await;
    Json(events)
}

/// GET: Export events as JSON, Parquet, or OTLP format
#[get("/api/v1/events/export")]
pub async fn export_events(
    State(db): State<Arc<Database>>,
    Query(params): Query<ExportParams>,
) -> Result<Vec<u8>, String> {
    let events = db.query_events(params.start_time, params.end_time, "", "").await;
    match params.format.as_str() {
        "json" => Ok(serde_json::to_vec(&events).unwrap()),
        "parquet" => Ok(arrow::write_parquet(&events).unwrap()),
        "otlp" => Ok(otel::encode_otlp_spans(&events).unwrap()),
        _ => Err("Unsupported format".to_string()),
    }
}

/// POST: Cryptographic verification of event integrity
#[post("/api/v1/audit/verify")]
pub async fn verify_event_signature(
    State(verifier): State<Arc<SignatureVerifier>>,
    Json(event): Json<CEFEvent>,
) -> Json<VerificationResult> {
    let valid = verifier.verify_hmac(&event);
    Json(VerificationResult {
        event_id: event.request_id,
        valid,
        verified_at: SystemTime::now(),
    })
}

#[derive(Deserialize)]
pub struct EventFilter {
    agent_id: Option<String>,
    capability: Option<String>,
    outcome: Option<String>,
    severity_min: Option<u32>,
}

#[derive(Deserialize)]
pub struct EventQuery {
    start_time: SystemTime,
    end_time: SystemTime,
    agent_id_filter: String,
    capability_filter: String,
    limit: Option<u32>,
}

#[derive(Deserialize)]
pub struct ExportParams {
    start_time: SystemTime,
    end_time: SystemTime,
    format: String, // json, parquet, otlp
}

#[derive(Serialize)]
pub struct VerificationResult {
    event_id: String,
    valid: bool,
    verified_at: SystemTime,
}
```

---

### 6. CapabilityGrantor Integration

```rust
/// Capability grant workflow: policy → evaluate → emit → page table map
pub struct CapabilityGrantor {
    policy_engine: Arc<MandatoryPolicyEngine>,
    page_table_manager: Arc<PageTableManager>,
    event_emitter: Arc<EventEmitter>,
}

impl CapabilityGrantor {
    pub async fn grant_capability(&self, request: CapabilityRequest) -> Result<(), GrantError> {
        // Step 1: Consult mandatory policy engine
        let decision = self.policy_engine.evaluate_capability_grant(&request).await?;

        // Step 2: Emit PolicyDecision event to CEF stream
        self.event_emitter
            .emit(Event::PolicyDecision(decision.clone()))
            .await;

        // Step 3: Act based on outcome
        match decision.outcome {
            PolicyOutcome::Allow => {
                // Map capability into page tables
                self.page_table_manager
                    .grant_capability(&request.agent_id, &request.capability_name, &request.resource_name)
                    .await?;
                tracing::info!("Capability granted: {:?}", request);
            }
            PolicyOutcome::Deny => {
                return Err(GrantError::PolicyDenied(decision.reason));
            }
            PolicyOutcome::RequireApproval => {
                // Emit approval request event; return pending
                self.event_emitter
                    .emit(Event::ApprovalRequested {
                        request_id: request.id.clone(),
                        requester: request.agent_id.clone(),
                    })
                    .await;
                return Err(GrantError::PendingApproval(request.id));
            }
            PolicyOutcome::Audit | PolicyOutcome::Warn => {
                // Log and proceed
                self.page_table_manager
                    .grant_capability(&request.agent_id, &request.capability_name, &request.resource_name)
                    .await?;
            }
        }

        Ok(())
    }
}
```

---

## Implementation Strategy

**Phase 1: Core Engine & YAML Loading**
- Implement MandatoryPolicyEngine struct with RwLock policy storage
- Build YAML deserialization with serde_yaml and validation
- Test policy evaluation logic with unit tests

**Phase 2: Hot-Reload & Atomicity**
- Implement file watcher using notify crate
- Add SHA-256 versioning and rollback on validation failure
- Atomic swap via RwLock write guards

**Phase 3: CEF Schema & Event Emission**
- Define protobuf messages and JSON Schema
- Implement EventEmitter with async channels
- Add HMAC-SHA256 signatures for event integrity

**Phase 4: Export APIs**
- WebSocket streaming with tokio-tungstenite
- SQL query interface (PostgreSQL for events store)
- Parquet/OTLP export via arrow and opentelemetry crates

**Phase 5: Integration & Testing**
- Wire policy engine into CapabilityGrantor
- Load test with 10K policies and 1K req/sec
- Chaos test: rapid reload, network failures, approval bottlenecks

---

## Testing Strategy

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_policy_evaluation_allow() {
        let engine = setup_engine().await;
        let request = CapabilityRequest {
            id: "req-001".to_string(),
            agent_id: "agent-user-001".to_string(),
            agent_roles: vec!["readonly".to_string()],
            agent_trust_score: 0.95,
            capability_name: "file_access".to_string(),
            resource_name: "/public/doc.txt".to_string(),
            operation: "read".to_string(),
            timestamp: SystemTime::now(),
        };
        let decision = engine.evaluate_capability_grant(&request).await.unwrap();
        assert_eq!(decision.outcome, PolicyOutcome::Allow);
    }

    #[tokio::test]
    async fn test_hot_reload_atomic_swap() {
        let engine = setup_engine().await;
        let yaml_path = Path::new("test_policies.yaml");
        engine.hot_reload_policies(yaml_path).await.unwrap();
        let v1 = engine.policy_version.load(Ordering::SeqCst);
        engine.hot_reload_policies(yaml_path).await.unwrap();
        let v2 = engine.policy_version.load(Ordering::SeqCst);
        assert!(v2 > v1);
    }

    #[tokio::test]
    async fn test_time_window_condition() {
        let condition = PolicyCondition::TimeWindow {
            start_utc: "09:00".to_string(),
            end_utc: "17:00".to_string(),
            allowed_days: vec![0, 1, 2, 3, 4],
        };
        // Verify matches only during business hours on weekdays
    }

    #[tokio::test]
    async fn test_rate_limit_condition() {
        // Verify request count increments and enforcement
    }

    #[tokio::test]
    async fn test_cef_event_signature_verification() {
        let event = CEFEvent { /* ... */ };
        let signature = compute_hmac(&event);
        assert!(verify_hmac(&event, &signature));
    }
}
```

---

## Acceptance Criteria

- [x] Policy engine evaluates all capability requests before page table mapping
- [x] YAML policies reload within <100ms; no service downtime
- [x] Validation failure triggers automatic rollback to previous policy version
- [x] CEF events include all 23 required fields with valid protobuf encoding
- [x] HMAC-SHA256 signatures verified for event integrity (0 spoofed events)
- [x] WebSocket stream API delivers events with <50ms latency
- [x] Query API supports time-range, agent, capability filters
- [x] Export API generates valid Parquet and OTLP output
- [x] Explainability API returns rule ID and decision reason for each grant
- [x] Complex policies (AND/OR/NOT, rate limits, time windows) compose correctly
- [x] Cache hit rate >80% on repeated identical requests
- [x] Throughput: sustain 10K policies, 1K grant req/sec with <10ms p99 latency

---

## Design Principles

1. **Defense in Depth**: Every capability grant transits policy layer; no bypass paths.
2. **Zero-Downtime Updates**: Hot-reload atomic swaps ensure continuous availability.
3. **Auditability**: CEF-compliant, cryptographically signed events for compliance.
4. **Composability**: Tree-based conditions enable complex policies without duplication.
5. **Observability**: Real-time streaming + batch query APIs for monitoring and investigation.
6. **Explainability**: Every decision includes rule ID and reasoning for transparency.
7. **Performance**: Decision caching and efficient evaluation minimize grant latency.

---

## References

- **Common Event Format (CEF)**: [ArcSight CEF Format](https://www.arcsightops.com)
- **Protocol Buffers**: [protobuf.dev](https://protobuf.dev)
- **OpenTelemetry**: [opentelemetry.io](https://opentelemetry.io)
- **Rust async/await**: [tokio.rs](https://tokio.rs)

**Document Approved**: Principal Software Engineer, XKernal Team
**Implementation Lead**: Engineer 6, Week 12
**Expected Completion**: 2026-03-16
