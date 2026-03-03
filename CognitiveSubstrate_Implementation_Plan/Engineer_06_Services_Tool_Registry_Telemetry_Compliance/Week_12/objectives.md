# Engineer 6 — Services: Tool Registry, Telemetry & Compliance — Week 12

## Phase: Phase 1 (Weeks 7-14)

## Weekly Objective
Implement Mandatory Policy Engine (consulted on every capability grant) with hot-reloadable policies and deep integration with telemetry. Complete Phase 1 core services.

## Document References
- **Primary:** Section 6.2 (Phase 1, Week 11-14: Mandatory Policy Engine, consulted on every cap grant), Section 3.3.6 (Mandatory Policy Engine, enforces policies, consulted on every cap grant)
- **Supporting:** Section 3.3.4 (Telemetry PolicyDecision events), Week 11 (telemetry integration)

## Deliverables
- [ ] Policy Engine core architecture
  - Policy storage and versioning (in-memory + file-backed)
  - Policy version hashing (for audit trail)
  - Hot-reload mechanism (watch policy files for changes)
  - Policy DSL or configuration format (YAML or custom)
- [ ] Capability grant workflow
  - Before any capability grant: consult Policy Engine
  - Input: requester_agent, requested_capability, context
  - Output: ALLOW | DENY | REQUIRE_APPROVAL | AUDIT | WARN
  - Optional reason code and human-readable explanation
- [ ] Policy decision as first-class event
  - Emit PolicyDecision event for every capability decision
  - Include decision_type, rule_id, policy_version_hash
  - Include inputs (requester, capability), outcome, reason_code
  - Optional redacted explanation (EU AI Act Article 12(2)(a))
- [ ] Policy hot-reload system
  - Watch policy files for modifications
  - Validate new policies before applying
  - Atomic swap of policy version (no requests processed during swap)
  - Rollback on validation failure
- [ ] Capability grant enforcement
  - Invoke Policy Engine before granting any kernel capability
  - Page table mapping only after policy approval (explicit ordering)
  - Log all grant decisions and denials
- [ ] Advanced policy features
  - Policy composition (AND, OR, NOT combinations)
  - Time-based policies (allow during business hours, deny after)
  - Rate limiting policies (e.g., max N calls per hour)
  - Agent history policies (deny if agent has violated policy before)
  - Resource-based policies (deny if insufficient quota)
- [ ] Audit and logging
  - All policy decisions logged to event stream
  - Policy changes logged (who changed what, when)
  - Rollback events logged
- [ ] Integration with telemetry
  - Emit PolicyDecision event for every decision
  - Attach policy version hash and rule ID
  - Cost attribution for policy evaluation (negligible, but tracked)
- [ ] Policy advisor/explainability
  - Export policy decision logs for compliance review
  - Explain why a request was denied (for humans)
  - Suggest policy changes for blocked requests
- [ ] **CEF Event Schema specification in Protocol Buffers format**
- [ ] **CEF Event Schema specification in JSON Schema format**
- [ ] **Export API contract: `/api/v1/events/stream` (WebSocket, real-time CEF events)**
- [ ] **Export API contract: `/api/v1/events/query` (POST, historical CEF event search)**
- [ ] **Export API contract: `/api/v1/events/export` (POST, bulk export JSON/Parquet/OTLP)**
- [ ] **Export API contract: `/api/v1/audit/verify` (POST, cryptographic verification of events)**
- [ ] Unit and integration tests
  - Policy loading and hot-reload
  - Decision making (all outcome types)
  - Policy composition logic
  - Time-based and rate-limiting policies
  - Capability grant blocking
  - CEF export API endpoints

## Technical Specifications

### CEF Event Schema

**Protocol Buffers Schema (cef_event.proto)**
```protobuf
syntax = "proto3";

package cognition.telemetry;

message CEFEvent {
    // Base CEF fields
    string event_id = 1;                    // ULID
    string timestamp_utc = 2;               // RFC3339 with nanosecond precision
    string event_type = 3;                  // ThoughtStep, ToolCallRequested, ToolCallCompleted, etc.
    string actor = 4;                       // Agent ID
    string resource = 5;                    // Resource identifier
    string action = 6;                      // READ, WRITE, INVOKE, GRANT, etc.
    string result = 7;                      // COMPLETED, FAILED, DENIED, TIMEOUT

    // OpenTelemetry semantic conventions
    string trace_id = 8;                    // 128-bit hex (W3C Trace Context)
    string span_id = 9;                     // 64-bit hex
    string parent_span_id = 10;             // Optional parent for causality
    string ct_id = 11;                      // Cognitive Thread ID
    string agent_id = 12;                   // Redundant with actor, for clarity
    string crew_id = 13;                    // Multi-agent crew identifier
    string phase = 14;                      // reasoning, tool_call, response, etc.
    string data_classification = 15;        // public, internal, restricted, confidential

    // Cost metrics
    int64 cost_input_tokens = 16;
    int64 cost_output_tokens = 17;
    double cost_gpu_milliseconds = 18;
    double cost_wall_clock_milliseconds = 19;
    double cost_tpc_hours = 20;

    // Context key-value pairs
    map<string, string> context = 21;

    // Optional cryptographic fields
    string event_hash = 22;                 // SHA256 hash for tamper detection
    string signature = 23;                  // Optional HMAC-SHA256 for audit
}

message CEFEventBatch {
    repeated CEFEvent events = 1;
    string batch_id = 2;                    // ULID for batch deduplication
    string batch_signature = 3;             // Hash of all event hashes
}
```

**JSON Schema (cef_event.schema.json)**
```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "CEF Event",
  "description": "Common Event Format for Cognitive Substrate telemetry",
  "type": "object",
  "required": [
    "event_id", "timestamp_utc", "event_type", "actor", "resource",
    "action", "result", "trace_id", "span_id", "ct_id", "agent_id", "crew_id"
  ],
  "properties": {
    "event_id": {
      "type": "string",
      "description": "ULID for event deduplication",
      "pattern": "^[0-7][0-9A-HJKMNP-TV-Z]{25}$"
    },
    "timestamp_utc": {
      "type": "string",
      "format": "date-time",
      "description": "RFC3339 with nanosecond precision"
    },
    "event_type": {
      "type": "string",
      "enum": [
        "ThoughtStep", "ToolCallRequested", "ToolCallCompleted",
        "CapabilityGranted", "CapabilityDenied", "PolicyDecision",
        "IpcMessage", "ContextCheckpoint", "SignalReceived",
        "PolicyReloaded", "PolicyReloadFailed"
      ]
    },
    "actor": { "type": "string" },
    "resource": { "type": "string" },
    "action": { "type": "string" },
    "result": {
      "type": "string",
      "enum": ["COMPLETED", "FAILED", "DENIED", "TIMEOUT", "PENDING"]
    },
    "trace_id": {
      "type": "string",
      "pattern": "^[0-9a-f]{32}$",
      "description": "128-bit hex trace ID"
    },
    "span_id": {
      "type": "string",
      "pattern": "^[0-9a-f]{16}$",
      "description": "64-bit hex span ID"
    },
    "parent_span_id": {
      "type": ["string", "null"],
      "pattern": "^[0-9a-f]{16}$"
    },
    "ct_id": { "type": "string" },
    "agent_id": { "type": "string" },
    "crew_id": { "type": "string" },
    "phase": {
      "type": "string",
      "enum": ["reasoning", "tool_call", "response", "delegation", "revocation"]
    },
    "data_classification": {
      "type": "string",
      "enum": ["public", "internal", "restricted", "confidential"]
    },
    "cost": {
      "type": "object",
      "properties": {
        "input_tokens": { "type": "integer" },
        "output_tokens": { "type": "integer" },
        "gpu_milliseconds": { "type": "number" },
        "wall_clock_milliseconds": { "type": "number" },
        "tpc_hours": { "type": "number" }
      }
    },
    "context": {
      "type": "object",
      "additionalProperties": { "type": "string" }
    },
    "event_hash": { "type": "string" },
    "signature": { "type": "string" }
  }
}
```

### Export API Contract

**HTTP/gRPC API Endpoints**

**1. WebSocket Stream: `/api/v1/events/stream`**
```
GET /api/v1/events/stream?event_types=ThoughtStep,ToolCallCompleted&trace_id=abc123
Upgrade: websocket
Connection: Upgrade
Sec-WebSocket-Key: ...

Server sends (one per line):
{"event_id": "...", "trace_id": "abc123", ...}
{"event_id": "...", "trace_id": "abc123", ...}
```
- Real-time streaming of CEF events matching filter
- Filters: event_types (comma-separated), trace_id, agent_id, crew_id, phase
- Backpressure: client drops connection if overwhelmed
- Latency target: <100ms from event emission to client receive

**2. Historical Query: `/api/v1/events/query` (POST)**
```
POST /api/v1/events/query
Content-Type: application/json

{
  "filters": {
    "trace_id": "abc123",
    "event_types": ["ThoughtStep", "ToolCallCompleted"],
    "timestamp_min": "2026-03-01T00:00:00Z",
    "timestamp_max": "2026-03-01T23:59:59Z",
    "agent_id": "agent_123"
  },
  "sort": "timestamp_asc",
  "limit": 1000,
  "offset": 0
}

Response:
{
  "events": [{...}, {...}],
  "total_count": 5000,
  "query_time_ms": 125,
  "has_more": true
}
```
- Query historical events from persistent storage
- Supports pagination, sorting, filtering
- Response time target: <500ms for typical queries

**3. Bulk Export: `/api/v1/events/export` (POST)**
```
POST /api/v1/events/export
Content-Type: application/json

{
  "filters": { ... },
  "format": "json" | "parquet" | "otlp",
  "compression": "none" | "gzip"
}

Response: 200 OK + binary file
```
- Export filtered events in multiple formats
- JSON: one event per line (NDJSON)
- Parquet: columnar format for data warehouses
- OTLP: OpenTelemetry Protocol format for collectors
- Compression: optional gzip
- Streaming response for large exports

**4. Audit Verification: `/api/v1/audit/verify` (POST)**
```
POST /api/v1/audit/verify
Content-Type: application/json

{
  "trace_id": "abc123",
  "verify_signatures": true,
  "verify_chain": true
}

Response:
{
  "verified": true,
  "signature_valid": true,
  "chain_valid": true,
  "events": [{...}, {...}],
  "verification_timestamp": "2026-03-01T12:34:56Z"
}
```
- Cryptographically verify event integrity
- Verify HMAC signatures if enabled
- Verify event chain causality (parent/child span_ids)
- Return verification metadata

### Policy Engine Core
```rust
pub struct MandatoryPolicyEngine {
    policies: Arc<RwLock<PolicySet>>,
    policy_version: Arc<AtomicU64>,
    policy_hash: Arc<Mutex<String>>,
    decision_log: Arc<Mutex<VecDeque<PolicyDecision>>>,
    telemetry: Arc<TelemetryEngineV2>,
}

pub struct PolicySet {
    rules: Vec<PolicyRule>,
    version: u64,
    timestamp: i64,
}

pub struct PolicyRule {
    pub id: String,
    pub description: String,
    pub condition: PolicyCondition,
    pub decision: PolicyOutcome,
    pub explanation: Option<String>,
}

pub enum PolicyCondition {
    AllOf(Vec<PolicyCondition>),
    AnyOf(Vec<PolicyCondition>),
    Not(Box<PolicyCondition>),
    AgentMatches(String),
    CapabilityMatches(String),
    TimeWindow { start_hour: u8, end_hour: u8 },
    RateLimit { max_calls: u32, window_seconds: u64 },
    AgentHistoryOk,
    ResourceAvailable { resource_name: String },
}

pub enum PolicyOutcome {
    Allow,
    Deny,
    RequireApproval,
    Audit,
    Warn,
}

pub struct PolicyDecisionInput {
    pub requester_agent: String,
    pub requested_capability: String,
    pub context: Map<String, String>,
}

pub struct PolicyDecision {
    pub decision_id: String,
    pub timestamp: i64,
    pub input: PolicyDecisionInput,
    pub outcome: PolicyOutcome,
    pub matching_rule_id: Option<String>,
    pub policy_version_hash: String,
    pub explanation: Option<String>,
}

impl MandatoryPolicyEngine {
    pub fn new(telemetry: Arc<TelemetryEngineV2>) -> Self {
        Self {
            policies: Arc::new(RwLock::new(PolicySet {
                rules: vec![],
                version: 0,
                timestamp: 0,
            })),
            policy_version: Arc::new(AtomicU64::new(0)),
            policy_hash: Arc::new(Mutex::new(String::new())),
            decision_log: Arc::new(Mutex::new(VecDeque::new())),
            telemetry,
        }
    }

    pub async fn load_policies(&self, policy_file: &Path) -> Result<(), PolicyError> {
        let yaml_str = tokio::fs::read_to_string(policy_file).await?;
        let policy_set: PolicySet = serde_yaml::from_str(&yaml_str)?;

        // Validate policies
        self.validate_policies(&policy_set)?;

        // Update policies atomically
        {
            let mut policies = self.policies.write().await;
            let old_version = policies.version;
            *policies = policy_set;
            policies.version = old_version + 1;
            policies.timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;
        }

        // Update hash
        let hash = self.compute_policy_hash().await;
        *self.policy_hash.lock().await = hash;
        self.policy_version.fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    pub async fn evaluate_capability_request(&self, input: PolicyDecisionInput)
        -> Result<PolicyOutcome, PolicyError>
    {
        let decision_id = uuid::Uuid::new_v4().to_string();
        let policies = self.policies.read().await;
        let policy_hash = self.policy_hash.lock().await.clone();

        let mut outcome = PolicyOutcome::Deny; // Default deny
        let mut matching_rule_id = None;

        // Evaluate rules in order; first match wins
        for rule in &policies.rules {
            if self.evaluate_condition(&rule.condition, &input).await? {
                outcome = rule.decision.clone();
                matching_rule_id = Some(rule.id.clone());
                break;
            }
        }

        let decision = PolicyDecision {
            decision_id: decision_id.clone(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            input: input.clone(),
            outcome: outcome.clone(),
            matching_rule_id: matching_rule_id.clone(),
            policy_version_hash: policy_hash.clone(),
            explanation: None, // Populated for user-facing decisions
        };

        // Log decision
        {
            let mut log = self.decision_log.lock().await;
            if log.len() >= 100_000 {
                log.pop_front();
            }
            log.push_back(decision.clone());
        }

        // Emit telemetry event
        self.emit_policy_decision_event(&decision).await.ok();

        Ok(outcome)
    }

    async fn evaluate_condition(&self, condition: &PolicyCondition, input: &PolicyDecisionInput)
        -> Result<bool, PolicyError>
    {
        match condition {
            PolicyCondition::AllOf(conditions) => {
                for cond in conditions {
                    if !self.evaluate_condition(cond, input).await? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            PolicyCondition::AnyOf(conditions) => {
                for cond in conditions {
                    if self.evaluate_condition(cond, input).await? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            PolicyCondition::Not(cond) => {
                let result = self.evaluate_condition(cond, input).await?;
                Ok(!result)
            }
            PolicyCondition::AgentMatches(pattern) => {
                Ok(self.pattern_match(&input.requester_agent, pattern))
            }
            PolicyCondition::CapabilityMatches(pattern) => {
                Ok(self.pattern_match(&input.requested_capability, pattern))
            }
            PolicyCondition::TimeWindow { start_hour, end_hour } => {
                let now = chrono::Local::now();
                let hour = now.hour() as u8;
                Ok(hour >= *start_hour && hour < *end_hour)
            }
            PolicyCondition::RateLimit { max_calls, window_seconds } => {
                self.check_rate_limit(&input.requester_agent, *max_calls, *window_seconds)
                    .await
            }
            PolicyCondition::AgentHistoryOk => {
                self.check_agent_history(&input.requester_agent).await
            }
            PolicyCondition::ResourceAvailable { resource_name } => {
                self.check_resource_quota(resource_name, &input.context).await
            }
        }
    }

    fn pattern_match(&self, text: &str, pattern: &str) -> bool {
        if pattern == "*" {
            return true;
        }
        text.contains(pattern) || text == pattern
    }

    async fn check_rate_limit(&self, agent_id: &str, max_calls: u32, window_seconds: u64)
        -> Result<bool, PolicyError>
    {
        let log = self.decision_log.lock().await;
        let cutoff_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64 - (window_seconds as i64);

        let count = log.iter()
            .filter(|d| d.input.requester_agent == agent_id && d.timestamp > cutoff_time)
            .count();

        Ok((count as u32) < max_calls)
    }

    async fn check_agent_history(&self, agent_id: &str) -> Result<bool, PolicyError> {
        // Check if agent has violated policies in past
        // Placeholder: always allow for now
        Ok(true)
    }

    async fn check_resource_quota(&self, resource_name: &str, context: &Map<String, String>)
        -> Result<bool, PolicyError>
    {
        // Check resource availability from context
        Ok(true)
    }

    async fn emit_policy_decision_event(&self, decision: &PolicyDecision)
        -> Result<(), EmitError>
    {
        let event = CEFEvent {
            event_type: EventType::PolicyDecision,
            actor: "policy_engine",
            resource: decision.input.requested_capability.clone(),
            action: "GRANT_CAPABILITY",
            result: match decision.outcome {
                PolicyOutcome::Allow => EventResult::COMPLETED,
                _ => EventResult::DENIED,
            },
            context: {
                "decision_id": decision.decision_id.clone(),
                "requester_agent": decision.input.requester_agent.clone(),
                "decision_type": format!("{:?}", decision.outcome),
                "rule_id": decision.matching_rule_id.clone().unwrap_or_default(),
                "policy_version_hash": decision.policy_version_hash.clone(),
                "explanation": decision.explanation.clone().unwrap_or_default(),
            }.into(),
            ..Default::default()
        };

        self.telemetry.emit_event(event).await
    }

    fn validate_policies(&self, policies: &PolicySet) -> Result<(), PolicyError> {
        // Validate all rules are well-formed
        for rule in &policies.rules {
            if rule.id.is_empty() {
                return Err(PolicyError::InvalidPolicy("Rule ID cannot be empty".to_string()));
            }
        }
        Ok(())
    }

    async fn compute_policy_hash(&self) -> String {
        use sha2::{Sha256, Digest};

        let policies = self.policies.read().await;
        let serialized = serde_json::to_string(&*policies).unwrap_or_default();

        let mut hasher = Sha256::new();
        hasher.update(serialized.as_bytes());
        let result = hasher.finalize();
        format!("{:x}", result)
    }

    pub async fn hot_reload_from_file(&self, policy_file: &Path) -> Result<(), PolicyError> {
        // Watch for file changes and reload
        match self.load_policies(policy_file).await {
            Ok(()) => {
                self.telemetry.emit_event(CEFEvent {
                    event_type: EventType::PolicyReloaded,
                    actor: "policy_engine",
                    resource: policy_file.to_string_lossy().to_string(),
                    action: "RELOAD",
                    result: EventResult::COMPLETED,
                    ..Default::default()
                }).await.ok();
                Ok(())
            }
            Err(e) => {
                self.telemetry.emit_event(CEFEvent {
                    event_type: EventType::PolicyReloadFailed,
                    actor: "policy_engine",
                    resource: policy_file.to_string_lossy().to_string(),
                    action: "RELOAD",
                    result: EventResult::FAILED,
                    context: {
                        "error": format!("{:?}", e),
                    }.into(),
                    ..Default::default()
                }).await.ok();
                Err(e)
            }
        }
    }

    pub async fn export_decision_logs(&self, output_path: &Path) -> Result<u64, ExportError> {
        let log = self.decision_log.lock().await;
        let json = serde_json::to_string(&*log)?;
        tokio::fs::write(output_path, json).await?;
        Ok(log.len() as u64)
    }
}
```

### Capability Grant Integration
```rust
pub struct CapabilityGrantor {
    policy_engine: Arc<MandatoryPolicyEngine>,
    page_table: Arc<PageTable>,
}

impl CapabilityGrantor {
    pub async fn grant_capability(&self, agent_id: &str, capability: &str,
                                  context: Map<String, String>)
        -> Result<(), GrantError>
    {
        // Step 1: Consult Policy Engine
        let input = PolicyDecisionInput {
            requester_agent: agent_id.to_string(),
            requested_capability: capability.to_string(),
            context,
        };

        let outcome = self.policy_engine.evaluate_capability_request(input).await?;

        // Step 2: Apply policy decision
        match outcome {
            PolicyOutcome::Allow => {
                // Step 3: Map capability in page tables
                self.page_table.map_capability(agent_id, capability).await?;
                Ok(())
            }
            PolicyOutcome::Deny => {
                Err(GrantError::PolicyDenial("Policy denies capability".to_string()))
            }
            PolicyOutcome::RequireApproval => {
                Err(GrantError::RequiresApproval("Human approval required".to_string()))
            }
            PolicyOutcome::Audit => {
                // Log audit event, but allow
                self.page_table.map_capability(agent_id, capability).await?;
                Ok(())
            }
            PolicyOutcome::Warn => {
                // Log warning, but allow
                self.page_table.map_capability(agent_id, capability).await?;
                Ok(())
            }
        }
    }
}
```

### Policy YAML Example
```yaml
policies:
  - id: "allow-readonly"
    description: "Allow READ_ONLY capability for all agents"
    condition:
      type: "CapabilityMatches"
      pattern: "*.READ_ONLY"
    decision: "ALLOW"

  - id: "require-approval-write"
    description: "Require approval for WRITE capabilities"
    condition:
      type: "CapabilityMatches"
      pattern: "*.WRITE"
    decision: "REQUIRE_APPROVAL"

  - id: "deny-after-hours"
    description: "Deny all external API calls after 8 PM"
    condition:
      type: "AllOf"
      conditions:
        - type: "CapabilityMatches"
          pattern: "external.*"
        - type: "TimeWindow"
          start_hour: 20
          end_hour: 24
    decision: "DENY"
    explanation: "External API access restricted after business hours"

  - id: "rate-limit-api-calls"
    description: "Rate limit API calls to 100 per hour"
    condition:
      type: "RateLimit"
      max_calls: 100
      window_seconds: 3600
    decision: "DENY"
    explanation: "API call rate limit exceeded"
```

## Dependencies
- **Blocked by:** Week 11 (telemetry infrastructure), Week 7-10 (Tool Registry and caching)
- **Blocking:** Week 13-14 (Phase 1 completion and testing)

## Acceptance Criteria
- [ ] Policy Engine core implementation (load, evaluate, hot-reload)
- [ ] Capability grant workflow integrated with page table mapping
- [ ] PolicyDecision event emitted for every decision; policy version hash included
- [ ] Policy hot-reload functional; atomic swaps without request processing
- [ ] All policy condition types implemented (AllOf, AnyOf, Not, TimeWindow, RateLimit, etc.)
- [ ] Rate limiting tested; prevents exceeding max_calls per window
- [ ] Time-based policies tested; allow/deny based on hour
- [ ] Agent history tracking functional
- [ ] Resource quota checking implemented
- [ ] Policy advisor/explainability API available
- [ ] Decision logs exported for compliance review
- [ ] Unit tests cover all decision outcomes and policy conditions
- [ ] Integration tests verify capability grant blocking on policy denial

## Design Principles Alignment
- **Security-first:** Default deny; explicit allow required
- **Auditability:** Every decision logged and exported
- **Flexibility:** Policy composition allows complex rules
- **Hot-reload:** Policies change without restarting kernel
- **Transparency:** Policies visible and explainable to operators
