# Engineer 6 — Services: Tool Registry, Telemetry & Compliance — Week 16

## Phase: Phase 2 (Weeks 15-24)

## Weekly Objective
Complete PolicyDecision infrastructure from Week 15 with full integration into telemetry event stream and advanced compliance features. Establish foundation for Compliance Engine in Weeks 17-20.

## Document References
- **Primary:** Section 6.3 (Phase 2, Week 15-16: PolicyDecision completion), Section 3.3.4 (Telemetry), Section 3.3.5 (Compliance)
- **Supporting:** Week 15 (PolicyDecision events), Week 12 (Policy Engine), Week 11 (Telemetry)

## Deliverables
- [ ] PolicyDecision event integration into main telemetry stream
  - PolicyDecision events flow through TelemetryEngineV2.emit_event()
  - Searchable and filterable like other CEF events
  - Included in compliance tier retention (≥6 months)
  - Indexed for audit queries
- [ ] Advanced compliance features
  - Policy decision appeals process (log appeal request, generate appeal ID)
  - Policy exception workflow (request, review, approve/deny, record decision)
  - Automatic escalation for repeated denials (threshold: 3 denials in 1 hour)
  - Policy recommendation engine (suggest policy changes based on denial patterns)
- [ ] Audit trail enhancement
  - Link PolicyDecision to Tool Registry actions (sandbox checks, capability grants)
  - Link PolicyDecision to Telemetry events (cost, tool calls leading to denial)
  - Create decision dependency graph (which decisions triggered other decisions)
  - Export dependency graphs for audit review
- [ ] Compliance metadata enrichment
  - Map PolicyDecision to applicable regulations (GDPR, EU AI Act, SOC2, ISO27001)
  - Automatically populate regulatory_reference based on context
  - Generate compliance report: coverage of decisions against regulations
- [ ] Performance and scalability
  - PolicyDecision evaluation latency <5ms p99
  - PolicyDecision storage and indexing (searchable by decision_id, rule_id, outcome, time range)
  - Export decisions in bulk (100k+ decisions per request)
- [ ] Testing and validation
  - End-to-end: decision -> telemetry event -> audit log -> export
  - Appeal workflow tested
  - Automatic escalation tested
  - Regulation mapping validated
  - Performance tested at scale (10k decisions/hour)

## Technical Specifications

### PolicyDecision Integration into Telemetry
```rust
impl TelemetryEngineV2 {
    pub async fn record_policy_decision(&self, decision: &PolicyDecision)
        -> Result<(), RecordError>
    {
        // Create CEF event from policy decision
        let event = CEFEvent {
            event_id: decision.decision_id.clone(),
            event_type: EventType::PolicyDecision,
            timestamp_utc: decision.timestamp * 1_000_000, // Convert to microseconds
            actor: "policy_engine".to_string(),
            resource: decision.requested_capability.clone(),
            action: "EVALUATE_CAPABILITY".to_string(),
            result: match decision.outcome {
                PolicyOutcome::Allow | PolicyOutcome::Audit => EventResult::COMPLETED,
                PolicyOutcome::Deny | PolicyOutcome::RequireApproval => EventResult::DENIED,
                PolicyOutcome::Warn => EventResult::COMPLETED,
            },
            context: {
                "decision_id": decision.decision_id.clone(),
                "requester_agent": decision.requester_agent.clone(),
                "rule_id": decision.matching_rule_id.clone(),
                "policy_version_hash": decision.policy_version_hash.clone(),
                "outcome": format!("{:?}", decision.outcome),
                "audit_hash": decision.audit_hash.clone(),
                "regulatory_reference": decision.regulatory_reference.clone().unwrap_or_default(),
            }.into(),
            ..Default::default()
        };

        self.emit_event(event).await
    }
}

// Ensure PolicyDecision events are part of compliance tier (≥6 months retention)
pub struct ComplianceTierEventStore {
    event_log: PathBuf,
}

impl ComplianceTierEventStore {
    pub async fn store_policy_decision(&self, decision: &PolicyDecision)
        -> Result<(), StoreError>
    {
        // Store in compliance tier (not just operational tier)
        let json = serde_json::to_string(decision)?;
        self.write_compliance_tier_log(&json).await?;

        // Index for fast queries
        self.index_decision(decision).await?;

        Ok(())
    }

    async fn index_decision(&self, decision: &PolicyDecision) -> Result<(), StoreError> {
        // Create indexes for common queries
        // Index by: decision_id, rule_id, outcome, timestamp, requester_agent
        // Implementation: in-memory index + periodic flush to disk
        Ok(())
    }
}
```

### Policy Decision Appeals Workflow
```rust
pub struct AppealWorkflow {
    decision_log: Arc<Mutex<VecDeque<PolicyDecision>>>,
    appeal_log: Arc<Mutex<VecDeque<AppealRequest>>>,
}

pub struct AppealRequest {
    pub appeal_id: String,
    pub decision_id: String,
    pub requester_agent: String,
    pub appeal_reason: String,
    pub requested_at: i64,
    pub reviewed_at: Option<i64>,
    pub reviewed_by: Option<String>,
    pub approval_outcome: Option<AppealOutcome>,
}

pub enum AppealOutcome {
    Approved,  // Grant exception to policy
    Denied,    // Uphold original decision
    Escalated, // Forward to human reviewer
}

impl AppealWorkflow {
    pub async fn submit_appeal(&self, decision_id: &str, reason: &str,
                              requester_agent: &str) -> Result<String, AppealError>
    {
        let appeal_id = uuid::Uuid::new_v4().to_string();
        let appeal = AppealRequest {
            appeal_id: appeal_id.clone(),
            decision_id: decision_id.to_string(),
            requester_agent: requester_agent.to_string(),
            appeal_reason: reason.to_string(),
            requested_at: now(),
            reviewed_at: None,
            reviewed_by: None,
            approval_outcome: None,
        };

        self.appeal_log.lock().await.push_back(appeal);
        Ok(appeal_id)
    }

    pub async fn resolve_appeal(&self, appeal_id: &str, outcome: AppealOutcome,
                               reviewed_by: &str) -> Result<(), AppealError>
    {
        let mut log = self.appeal_log.lock().await;
        if let Some(appeal) = log.iter_mut().find(|a| a.appeal_id == appeal_id) {
            appeal.reviewed_at = Some(now());
            appeal.reviewed_by = Some(reviewed_by.to_string());
            appeal.approval_outcome = Some(outcome);

            match outcome {
                AppealOutcome::Approved => {
                    // Grant exception to policy
                    // Implementation: add exemption rule or temporary whitelist
                }
                _ => {}
            }
        }

        Ok(())
    }

    pub async fn export_appeals_for_audit(&self, output_path: &Path)
        -> Result<u64, ExportError>
    {
        let appeals = self.appeal_log.lock().await;
        let json = serde_json::to_string(&*appeals)?;
        tokio::fs::write(output_path, json).await?;
        Ok(appeals.len() as u64)
    }
}
```

### Automatic Escalation on Repeated Denials
```rust
pub struct EscalationService {
    decision_log: Arc<Mutex<VecDeque<PolicyDecision>>>,
    escalation_threshold: u32, // Default: 3 denials in 1 hour
    escalation_log: Arc<Mutex<VecDeque<EscalationEvent>>>,
}

pub struct EscalationEvent {
    pub escalation_id: String,
    pub triggered_at: i64,
    pub requester_agent: String,
    pub denial_count: u32,
    pub decision_ids: Vec<String>,
    pub escalated_to: Option<String>, // Policy admin ID
    pub status: EscalationStatus,
}

pub enum EscalationStatus {
    Pending,
    Reviewed,
    Resolved,
}

impl EscalationService {
    pub async fn check_and_escalate(&self, decision: &PolicyDecision)
        -> Result<(), EscalationError>
    {
        if !matches!(decision.outcome, PolicyOutcome::Deny) {
            return Ok(());
        }

        let log = self.decision_log.lock().await;
        let cutoff_time = now() - 3600; // Last 1 hour

        let recent_denials: Vec<_> = log.iter()
            .filter(|d| {
                d.requester_agent == decision.requester_agent
                    && matches!(d.outcome, PolicyOutcome::Deny)
                    && d.timestamp > cutoff_time
            })
            .collect();

        if recent_denials.len() >= self.escalation_threshold as usize {
            // Create escalation event
            let escalation = EscalationEvent {
                escalation_id: uuid::Uuid::new_v4().to_string(),
                triggered_at: now(),
                requester_agent: decision.requester_agent.clone(),
                denial_count: recent_denials.len() as u32,
                decision_ids: recent_denials.iter().map(|d| d.decision_id.clone()).collect(),
                escalated_to: None,
                status: EscalationStatus::Pending,
            };

            self.escalation_log.lock().await.push_back(escalation);

            // Emit escalation event to telemetry
            // Implementation: trigger alert to policy_admin
        }

        Ok(())
    }

    pub async fn export_escalations_for_review(&self, output_path: &Path)
        -> Result<u64, ExportError>
    {
        let escalations = self.escalation_log.lock().await;
        let json = serde_json::to_string(&*escalations)?;
        tokio::fs::write(output_path, json).await?;
        Ok(escalations.len() as u64)
    }
}
```

### Regulation Mapping
```rust
pub enum ApplicableRegulation {
    GDPR,
    EUAIAct,
    SOC2,
    ISO27001,
    HIPAA,
    CCPA,
}

pub struct RegulationMapper;

impl RegulationMapper {
    pub fn map_decision_to_regulations(decision: &PolicyDecision) -> Vec<ApplicableRegulation> {
        let mut regs = vec![];

        match decision.outcome {
            PolicyOutcome::Deny | PolicyOutcome::RequireApproval => {
                // Right to explanation: EU AI Act Article 12(2)(a)
                regs.push(ApplicableRegulation::EUAIAct);
            }
            _ => {}
        }

        // Data processing decisions: GDPR
        if decision.requested_capability.contains("data") {
            regs.push(ApplicableRegulation::GDPR);
        }

        // Security-related: SOC2, ISO27001
        if decision.requested_capability.contains("security") ||
           decision.requested_capability.contains("access_control") {
            regs.push(ApplicableRegulation::SOC2);
            regs.push(ApplicableRegulation::ISO27001);
        }

        regs
    }

    pub fn get_compliance_report(decisions: &[PolicyDecision]) -> ComplianceReport {
        let mut coverage = HashMap::new();

        for decision in decisions {
            let regs = Self::map_decision_to_regulations(decision);
            for reg in regs {
                coverage.entry(format!("{:?}", reg))
                    .or_insert(0)
                    .increment();
            }
        }

        ComplianceReport {
            total_decisions: decisions.len(),
            regulations_covered: coverage,
            coverage_percentage: 85.0, // Example
        }
    }
}

pub struct ComplianceReport {
    pub total_decisions: usize,
    pub regulations_covered: HashMap<String, usize>,
    pub coverage_percentage: f32,
}
```

### Audit Trail and Dependency Graph
```rust
pub struct AuditTrail {
    decision_log: Arc<Mutex<VecDeque<PolicyDecision>>>,
    event_log: Arc<Mutex<VecDeque<CEFEvent>>>,
}

pub struct DecisionDependencyGraph {
    pub decision_id: String,
    pub triggered_events: Vec<String>,      // CEF event IDs
    pub triggered_tool_calls: Vec<String>,  // Tool invocation IDs
    pub triggered_decisions: Vec<String>,   // Cascading policy decisions
}

impl AuditTrail {
    pub async fn build_dependency_graph(&self, decision_id: &str)
        -> Result<DecisionDependencyGraph, AuditError>
    {
        let decisions = self.decision_log.lock().await;
        let events = self.event_log.lock().await;

        let decision = decisions.iter()
            .find(|d| d.decision_id == decision_id)
            .ok_or(AuditError::NotFound)?;

        // Find events triggered by this decision
        let triggered_events: Vec<String> = events.iter()
            .filter(|e| {
                e.timestamp_utc > decision.timestamp as u64 * 1_000_000 &&
                e.timestamp_utc < (decision.timestamp as u64 + 1000) * 1_000_000 // Within 1 second
            })
            .map(|e| e.event_id.clone())
            .collect();

        Ok(DecisionDependencyGraph {
            decision_id: decision_id.to_string(),
            triggered_events,
            triggered_tool_calls: vec![],
            triggered_decisions: vec![],
        })
    }

    pub async fn export_audit_trail(&self, start_time: i64, end_time: i64,
                                    output_path: &Path) -> Result<u64, ExportError>
    {
        let decisions = self.decision_log.lock().await;
        let events = self.event_log.lock().await;

        let filtered_decisions: Vec<_> = decisions.iter()
            .filter(|d| d.timestamp >= start_time && d.timestamp <= end_time)
            .collect();

        let filtered_events: Vec<_> = events.iter()
            .filter(|e| {
                let ts = e.timestamp_utc / 1_000_000;
                ts >= start_time && ts <= end_time
            })
            .collect();

        let export = serde_json::json!({
            "time_range": { "start": start_time, "end": end_time },
            "decisions": filtered_decisions,
            "events": filtered_events,
        });

        tokio::fs::write(output_path, export.to_string()).await?;
        Ok((filtered_decisions.len() + filtered_events.len()) as u64)
    }
}
```

## Dependencies
- **Blocked by:** Week 15 (PolicyDecision foundation)
- **Blocking:** Week 17-20 (Compliance Engine and retention)

## Acceptance Criteria
- [ ] PolicyDecision events integrated into main telemetry stream
- [ ] PolicyDecision events searchable by decision_id, rule_id, outcome, timestamp
- [ ] Appeals workflow functional (submit, review, approve/deny)
- [ ] Automatic escalation triggered on 3+ denials per hour
- [ ] Escalation events exported for audit review
- [ ] Regulation mapping generates compliance reports
- [ ] Audit trail building and export functional
- [ ] Dependency graphs show triggered events and cascading decisions
- [ ] Performance tested at 10k decisions/hour; latency <5ms p99
- [ ] Unit tests cover appeals, escalation, regulation mapping, audit trails
- [ ] Integration tests cover end-to-end workflow

## Design Principles Alignment
- **Auditability:** Every decision traceable through audit trail
- **Appealability:** Agents can challenge decisions; appeals logged and reviewed
- **Escalation:** Repeated denials escalate to human review
- **Compliance:** Decisions mapped to applicable regulations; reports generated
- **Transparency:** Dependency graphs show decision causality
