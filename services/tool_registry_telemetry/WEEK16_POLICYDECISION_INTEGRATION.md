# Week 16: PolicyDecision Infrastructure & Compliance Integration
## XKernal Cognitive Substrate OS — L1 Services (Rust)

**Phase:** 2 | **Week:** 16 | **Engineer Level:** Staff-6 | **Module:** Tool Registry, Telemetry & Compliance

---

## Executive Summary

Week 16 completes PolicyDecision infrastructure initiated in Week 15, integrating policy decisions into the telemetry event stream as first-class searchable entities with 6+ month compliance retention. This document specifies the full appeals process, exception workflow, automatic escalation mechanisms, decision dependency graphs, bulk export capabilities, and compliance metadata enrichment required to establish the foundation for the Compliance Engine.

**Key Deliverables:**
- PolicyDecision event integration into primary telemetry stream
- Appeals and exception workflow with state machine enforcement
- Automatic escalation for 3+ denials/hour patterns
- Audit trail with dependency graph linking tool registry & telemetry
- Compliance metadata mapping to regulations (EU AI Act, GDPR, SOC 2)
- Bulk export capability for 100K+ decisions
- Latency SLA: <5ms p99 for decision creation, <100ms for appeal submission

---

## 1. Architecture Overview

### 1.1 Integration Points

PolicyDecision operates at the intersection of three systems:

```rust
// services/tool_registry_telemetry/policy_decision_integration.rs
use crate::telemetry::{EventStream, TelemetryEvent};
use crate::tool_registry::{ToolRegistry, ToolArtifact};
use crate::audit::{AuditLog, AuditEntity};
use std::sync::Arc;
use parking_lot::RwLock;
use anyhow::{Result, anyhow};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PolicyDecisionEvent {
    // Identity
    pub decision_id: uuid::Uuid,
    pub policy_rule_id: String,
    pub tool_request_id: uuid::Uuid,

    // Decision metadata
    pub decision_type: DecisionType,      // Allow, Deny, Escalate, Appeal
    pub timestamp: chrono::DateTime<Utc>,
    pub decision_rationale: String,
    pub confidence_score: f32,            // 0.0-1.0 for ML-based decisions

    // Compliance context
    pub compliance_tier: ComplianceTier,  // Audit, Standard, Critical
    pub applicable_regulations: Vec<RegulationMapping>,
    pub user_context: UserContext,
    pub tool_context: ToolContext,

    // Audit & traceability
    pub decision_chain: Vec<DecisionNode>,  // Dependency graph
    pub appeal_eligible: bool,
    pub escalation_reason: Option<String>,
}

#[derive(Clone, Debug)]
pub enum DecisionType {
    Allow,
    Deny { reason: DenyReason },
    Escalate { justification: String },
    Appeal { original_decision_id: uuid::Uuid },
}

#[derive(Clone, Debug)]
pub enum DenyReason {
    PolicyViolation(String),
    SecurityThreshold,
    ComplianceRisk,
    UserRiskProfile,
    RegulationRestriction,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComplianceTier {
    Audit,           // ≥18 months retention
    Standard,        // ≥6 months retention (default)
    Critical,        // ≥36 months retention
}

pub struct PolicyDecisionIntegration {
    telemetry_stream: Arc<EventStream>,
    tool_registry: Arc<ToolRegistry>,
    audit_log: Arc<AuditLog>,
    // In-memory graph for dependency tracing
    decision_graph: Arc<RwLock<DecisionDependencyGraph>>,
    escalation_detector: EscalationDetector,
}

impl PolicyDecisionIntegration {
    pub async fn record_decision(&self, event: PolicyDecisionEvent) -> Result<()> {
        // 1. Validate decision structure
        self.validate_decision(&event)?;

        // 2. Enrich with compliance metadata
        let enriched = self.enrich_compliance_metadata(event).await?;

        // 3. Record to telemetry stream with tier-specific retention
        self.telemetry_stream
            .publish_event(TelemetryEvent::PolicyDecision(enriched.clone()))
            .await?;

        // 4. Update dependency graph
        self.decision_graph.write().add_node(enriched.clone())?;

        // 5. Check escalation triggers
        self.escalation_detector.check_triggers(&enriched).await?;

        // 6. Record to audit log with full context
        self.audit_log.record_entity(
            AuditEntity::PolicyDecision(enriched)
        ).await?;

        Ok(())
    }

    async fn enrich_compliance_metadata(
        &self,
        mut event: PolicyDecisionEvent,
    ) -> Result<PolicyDecisionEvent> {
        // Map to applicable regulations based on decision type and user context
        event.applicable_regulations = self
            .map_regulations(&event.decision_type, &event.user_context)
            .await?;

        // Set compliance tier based on risk and regulation
        event.compliance_tier = self
            .determine_compliance_tier(&event)
            .await?;

        Ok(event)
    }

    async fn map_regulations(
        &self,
        decision_type: &DecisionType,
        user_context: &UserContext,
    ) -> Result<Vec<RegulationMapping>> {
        let mut regs = vec![];

        // EU AI Act: High-risk AI systems require explicit decision logging
        if matches!(decision_type, DecisionType::Deny { .. } | DecisionType::Escalate { .. }) {
            regs.push(RegulationMapping {
                regulation: "EU_AI_ACT",
                requirement: "Explainability",
                article: Some("13-14"),
                mandatory: true,
            });
        }

        // GDPR: Processing of personal data requires lawful basis
        if user_context.region == "EU" {
            regs.push(RegulationMapping {
                regulation: "GDPR",
                requirement: "Lawful Basis & Article 22 Rights",
                article: Some("22"),
                mandatory: true,
            });
        }

        // SOC 2: Access control and audit trail
        regs.push(RegulationMapping {
            regulation: "SOC_2",
            requirement: "Access Control & Audit Trail",
            article: None,
            mandatory: true,
        });

        Ok(regs)
    }

    async fn determine_compliance_tier(&self, event: &PolicyDecisionEvent) -> Result<ComplianceTier> {
        use crate::policy::RiskLevel;

        let tier = match (event.decision_type.risk_level(), event.user_context.risk_profile) {
            (RiskLevel::Critical, _) | (_, RiskLevel::Critical) => ComplianceTier::Critical,
            (RiskLevel::High, _) | (_, RiskLevel::High) => ComplianceTier::Audit,
            _ => ComplianceTier::Standard,
        };

        Ok(tier)
    }
}
```

---

## 2. Appeals Process & State Machine

### 2.1 Appeal Workflow

```rust
// services/tool_registry_telemetry/appeals.rs
use strum::{EnumString, Display};

#[derive(Clone, Debug, Copy, PartialEq, Eq, Serialize, Deserialize, EnumString, Display)]
pub enum AppealState {
    #[strum(to_string = "SUBMITTED")]
    Submitted,
    #[strum(to_string = "ACKNOWLEDGED")]
    Acknowledged,
    #[strum(to_string = "UNDER_REVIEW")]
    UnderReview,
    #[strum(to_string = "ESCALATED")]
    Escalated,
    #[strum(to_string = "APPROVED")]
    Approved,
    #[strum(to_string = "DENIED")]
    Denied,
    #[strum(to_string = "EXPIRED")]
    Expired,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Appeal {
    pub appeal_id: uuid::Uuid,
    pub original_decision_id: uuid::Uuid,
    pub state: AppealState,
    pub submitted_at: chrono::DateTime<Utc>,
    pub submitted_by: String,               // User or system identifier
    pub rationale: String,
    pub supporting_evidence: Vec<Evidence>,
    pub assignee: Option<String>,           // Human reviewer
    pub review_deadline: chrono::DateTime<Utc>,
    pub state_transitions: Vec<StateTransition>,
    pub resolution: Option<AppealResolution>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Evidence {
    pub evidence_id: uuid::Uuid,
    pub evidence_type: EvidenceType,
    pub content: String,
    pub uploaded_at: chrono::DateTime<Utc>,
    pub size_bytes: u64,
}

#[derive(Clone, Debug)]
pub enum EvidenceType {
    Document,
    LogExcerpt,
    ThirdPartyVerification,
    PolicyException,
    ContextualInformation,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StateTransition {
    pub from_state: AppealState,
    pub to_state: AppealState,
    pub transitioned_at: chrono::DateTime<Utc>,
    pub transitioned_by: String,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppealResolution {
    pub decision: AppealDecision,
    pub resolved_at: chrono::DateTime<Utc>,
    pub resolved_by: String,
    pub explanation: String,
    pub remediation: Option<String>,
}

#[derive(Clone, Debug, Copy)]
pub enum AppealDecision {
    Upheld,
    Overturned,
    PartiallyGranted,
    RequiresPolicy Change,
}

pub struct AppealsManager {
    appeals_db: Arc<Db>,
    policy_decision_integration: Arc<PolicyDecisionIntegration>,
    notification_service: Arc<NotificationService>,
}

impl AppealsManager {
    pub async fn submit_appeal(
        &self,
        original_decision_id: uuid::Uuid,
        submitted_by: String,
        rationale: String,
    ) -> Result<Appeal> {
        // Verify decision exists and is eligible for appeal
        let original = self.policy_decision_integration
            .get_decision(&original_decision_id)
            .await?;

        if !original.appeal_eligible {
            return Err(anyhow!("Decision not eligible for appeal"));
        }

        let appeal = Appeal {
            appeal_id: uuid::Uuid::new_v4(),
            original_decision_id,
            state: AppealState::Submitted,
            submitted_at: chrono::Utc::now(),
            submitted_by: submitted_by.clone(),
            rationale,
            supporting_evidence: vec![],
            assignee: None,
            review_deadline: chrono::Utc::now() + chrono::Duration::days(14),
            state_transitions: vec![StateTransition {
                from_state: AppealState::Submitted,
                to_state: AppealState::Submitted,
                transitioned_at: chrono::Utc::now(),
                transitioned_by: "SYSTEM".to_string(),
                reason: "Appeal created".to_string(),
            }],
            resolution: None,
        };

        // Persist
        self.appeals_db.insert("appeals", appeal.clone()).await?;

        // Notify compliance team
        self.notification_service
            .notify_compliance_team(&appeal)
            .await?;

        // Record audit trail
        info!("Appeal submitted: {} for decision {}",
              appeal.appeal_id, original_decision_id);

        Ok(appeal)
    }

    pub async fn transition_appeal(
        &self,
        appeal_id: uuid::Uuid,
        to_state: AppealState,
        reason: String,
        transitioned_by: String,
    ) -> Result<Appeal> {
        let mut appeal = self.appeals_db
            .get("appeals", appeal_id)
            .await?;

        // Validate state transition
        self.validate_transition(appeal.state, to_state)?;

        // Record transition
        appeal.state_transitions.push(StateTransition {
            from_state: appeal.state,
            to_state,
            transitioned_at: chrono::Utc::now(),
            transitioned_by: transitioned_by.clone(),
            reason: reason.clone(),
        });

        appeal.state = to_state;

        // If transitioning to UnderReview, assign reviewer
        if to_state == AppealState::UnderReview && appeal.assignee.is_none() {
            appeal.assignee = Some(self.select_reviewer().await?);
        }

        self.appeals_db.update("appeals", appeal.clone()).await?;

        Ok(appeal)
    }

    pub async fn resolve_appeal(
        &self,
        appeal_id: uuid::Uuid,
        decision: AppealDecision,
        explanation: String,
        resolved_by: String,
    ) -> Result<Appeal> {
        let mut appeal = self.appeals_db
            .get("appeals", appeal_id)
            .await?;

        // Transition to final state
        let final_state = match decision {
            AppealDecision::Upheld => AppealState::Denied,
            _ => AppealState::Approved,
        };

        appeal = self.transition_appeal(
            appeal_id,
            final_state,
            format!("Appeal resolved: {:?}", decision),
            resolved_by.clone(),
        ).await?;

        appeal.resolution = Some(AppealResolution {
            decision,
            resolved_at: chrono::Utc::now(),
            resolved_by,
            explanation,
            remediation: self.compute_remediation(&appeal).await.ok(),
        });

        self.appeals_db.update("appeals", appeal.clone()).await?;

        // If overturned, generate compensating policy decision
        if matches!(decision, AppealDecision::Overturned | AppealDecision::PartiallyGranted) {
            self.generate_appeal_resolution_decision(&appeal).await?;
        }

        Ok(appeal)
    }

    fn validate_transition(&self, from: AppealState, to: AppealState) -> Result<()> {
        let valid_transitions = match from {
            AppealState::Submitted => vec![AppealState::Acknowledged],
            AppealState::Acknowledged => vec![AppealState::UnderReview, AppealState::Escalated],
            AppealState::UnderReview => vec![AppealState::Approved, AppealState::Denied, AppealState::Escalated],
            AppealState::Escalated => vec![AppealState::Approved, AppealState::Denied],
            _ => vec![],  // Terminal states
        };

        if valid_transitions.contains(&to) {
            Ok(())
        } else {
            Err(anyhow!("Invalid transition: {} -> {}", from, to))
        }
    }
}
```

---

## 3. Exception Workflow

```rust
// services/tool_registry_telemetry/exceptions.rs

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Exception {
    pub exception_id: uuid::Uuid,
    pub policy_rule_id: String,
    pub exception_type: ExceptionType,
    pub created_at: chrono::DateTime<Utc>,
    pub created_by: String,
    pub justification: String,
    pub affected_users: Vec<String>,
    pub affected_tools: Vec<String>,
    pub valid_until: chrono::DateTime<Utc>,
    pub approval_chain: Vec<Approval>,
    pub auto_escalation_at: Option<chrono::DateTime<Utc>>,
}

#[derive(Clone, Debug)]
pub enum ExceptionType {
    TemporaryBypass { duration_hours: u32 },
    PermanentExemption { reason: String },
    ConditionalAllowance { conditions: Vec<String> },
    RegulationException { regulation: String },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Approval {
    pub approver_id: String,
    pub approved_at: chrono::DateTime<Utc>,
    pub approval_level: ApprovalLevel,
    pub justification: String,
}

#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum ApprovalLevel {
    Manager,
    Director,
    Compliance,
    Legal,
}

pub struct ExceptionManager {
    exceptions_db: Arc<Db>,
    approval_router: Arc<ApprovalRouter>,
}

impl ExceptionManager {
    pub async fn create_exception(
        &self,
        policy_rule_id: String,
        exception_type: ExceptionType,
        justification: String,
        created_by: String,
    ) -> Result<Exception> {
        let exception = Exception {
            exception_id: uuid::Uuid::new_v4(),
            policy_rule_id: policy_rule_id.clone(),
            exception_type: exception_type.clone(),
            created_at: chrono::Utc::now(),
            created_by: created_by.clone(),
            justification,
            affected_users: vec![],
            affected_tools: vec![],
            valid_until: chrono::Utc::now() + chrono::Duration::days(30),
            approval_chain: vec![],
            auto_escalation_at: Some(
                chrono::Utc::now() + chrono::Duration::days(7)
            ),
        };

        // Route for approval based on risk
        let required_level = self.route_for_approval(&exception).await?;
        self.approval_router.route(&exception, required_level).await?;

        self.exceptions_db.insert("exceptions", exception.clone()).await?;

        Ok(exception)
    }

    async fn route_for_approval(&self, exception: &Exception) -> Result<ApprovalLevel> {
        use std::collections::HashSet;

        let level = match &exception.exception_type {
            ExceptionType::TemporaryBypass { duration_hours } if *duration_hours > 168 =>
                ApprovalLevel::Director,
            ExceptionType::PermanentExemption { .. } => ApprovalLevel::Compliance,
            ExceptionType::RegulationException { .. } => ApprovalLevel::Legal,
            _ => ApprovalLevel::Manager,
        };

        Ok(level)
    }
}
```

---

## 4. Automatic Escalation Engine

### 4.1 Escalation Detector (3+ denials/hour pattern)

```rust
// services/tool_registry_telemetry/escalation.rs
use std::collections::VecDeque;
use parking_lot::Mutex;

const DENIAL_THRESHOLD: usize = 3;
const TIME_WINDOW: std::time::Duration = std::time::Duration::from_secs(3600);

#[derive(Clone, Debug)]
pub struct EscalationTrigger {
    pub trigger_type: TriggerType,
    pub severity: Severity,
    pub affected_entity_id: String,
    pub triggered_at: chrono::DateTime<Utc>,
    pub remediation_action: Option<RemediationAction>,
}

#[derive(Clone, Debug)]
pub enum TriggerType {
    DenialSpike { denials_in_window: usize },
    PolicyViolationPattern { pattern: String },
    ComplianceThresholdBreach { threshold: String },
    AppealRateAnomaly { rate: f32 },
    PermissionEscalation { escalation_chain: Vec<String> },
}

#[derive(Clone, Debug, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Clone, Debug)]
pub enum RemediationAction {
    NotifyCompliance,
    FlagUser,
    TemporaryRestriction,
    RequireHumanReview,
    ImmediateSuspension,
}

pub struct EscalationDetector {
    recent_denials: Arc<Mutex<VecDeque<DenialRecord>>>,
    escalation_db: Arc<Db>,
    notification_service: Arc<NotificationService>,
}

#[derive(Clone, Debug)]
struct DenialRecord {
    decision_id: uuid::Uuid,
    denied_at: chrono::DateTime<Utc>,
    user_id: String,
    tool_id: String,
    reason: DenyReason,
}

impl EscalationDetector {
    pub async fn check_triggers(&self, event: &PolicyDecisionEvent) -> Result<()> {
        // Only check denials
        if !matches!(event.decision_type, DecisionType::Deny { .. }) {
            return Ok(());
        }

        // Add to recent denials
        {
            let mut denials = self.recent_denials.lock();
            denials.push_back(DenialRecord {
                decision_id: event.decision_id,
                denied_at: event.timestamp,
                user_id: event.user_context.user_id.clone(),
                tool_id: event.tool_context.tool_id.clone(),
                reason: event.decision_type.as_deny_reason().unwrap(),
            });

            // Clean old entries
            let cutoff = chrono::Utc::now() - chrono::Duration::seconds(3600);
            while denials.front().map_or(false, |r| r.denied_at < cutoff) {
                denials.pop_front();
            }
        }

        // Check for spike
        let denials = self.recent_denials.lock();
        if denials.len() >= DENIAL_THRESHOLD {
            let trigger = EscalationTrigger {
                trigger_type: TriggerType::DenialSpike {
                    denials_in_window: denials.len(),
                },
                severity: if denials.len() > 10 {
                    Severity::Critical
                } else if denials.len() > 5 {
                    Severity::High
                } else {
                    Severity::Medium
                },
                affected_entity_id: event.user_context.user_id.clone(),
                triggered_at: chrono::Utc::now(),
                remediation_action: self.determine_remediation(denials.len()).await?,
            };

            self.escalation_db
                .insert("escalation_triggers", trigger.clone())
                .await?;

            self.notification_service
                .escalate_to_compliance(&trigger)
                .await?;
        }

        Ok(())
    }

    async fn determine_remediation(&self, denial_count: usize) -> Result<Option<RemediationAction>> {
        let action = match denial_count {
            3..=5 => Some(RemediationAction::NotifyCompliance),
            6..=10 => Some(RemediationAction::RequireHumanReview),
            11..=20 => Some(RemediationAction::TemporaryRestriction),
            21.. => Some(RemediationAction::ImmediateSuspension),
            _ => None,
        };

        Ok(action)
    }
}
```

---

## 5. Decision Dependency Graph

### 5.1 Graph Structure & Tracing

```rust
// services/tool_registry_telemetry/dependency_graph.rs
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::algo::toposort;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct DecisionNode {
    pub node_id: uuid::Uuid,
    pub decision_id: uuid::Uuid,
    pub depends_on: Vec<uuid::Uuid>,
    pub depended_by: Vec<uuid::Uuid>,
    pub node_type: DecisionNodeType,
    pub metadata: DecisionMetadata,
}

#[derive(Clone, Debug)]
pub enum DecisionNodeType {
    PolicyDecision,
    Appeal,
    Exception,
    HumanReview,
    SystemValidation,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DecisionMetadata {
    pub created_at: chrono::DateTime<Utc>,
    pub created_by: String,
    pub policy_rule_id: String,
    pub outcome: DecisionOutcome,
}

#[derive(Clone, Debug, Copy)]
pub enum DecisionOutcome {
    Allowed,
    Denied,
    Escalated,
    Appealed,
    Resolved,
}

pub struct DecisionDependencyGraph {
    graph: DiGraph<DecisionNode, DependencyRelation>,
    index_map: HashMap<uuid::Uuid, NodeIndex>,
}

#[derive(Clone, Debug)]
pub struct DependencyRelation {
    pub relation_type: RelationType,
    pub reason: String,
}

#[derive(Clone, Debug)]
pub enum RelationType {
    DirectDependency,
    PolicyChain,
    CompensatingDecision,
    AppealChain,
    ExceptionApplied,
}

impl DecisionDependencyGraph {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            index_map: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, node: DecisionNode) -> Result<()> {
        let index = self.graph.add_node(node.clone());
        self.index_map.insert(node.node_id, index);
        Ok(())
    }

    pub fn add_dependency(
        &mut self,
        from_id: uuid::Uuid,
        to_id: uuid::Uuid,
        relation: DependencyRelation,
    ) -> Result<()> {
        let from_idx = self.index_map.get(&from_id)
            .copied()
            .ok_or_else(|| anyhow!("From node not found"))?;
        let to_idx = self.index_map.get(&to_id)
            .copied()
            .ok_or_else(|| anyhow!("To node not found"))?;

        self.graph.add_edge(from_idx, to_idx, relation);
        Ok(())
    }

    pub fn get_dependency_chain(&self, node_id: uuid::Uuid) -> Result<Vec<DecisionNode>> {
        let idx = self.index_map.get(&node_id)
            .copied()
            .ok_or_else(|| anyhow!("Node not found"))?;

        let mut chain = vec![];

        // Traverse upward through dependencies
        let mut to_visit = vec![idx];
        let mut visited = std::collections::HashSet::new();

        while let Some(current_idx) = to_visit.pop() {
            if visited.contains(&current_idx) {
                continue;
            }
            visited.insert(current_idx);

            if let Some(node) = self.graph.node_weight(current_idx) {
                chain.push(node.clone());
            }

            // Add parents
            for parent_idx in self.graph.neighbors_directed(current_idx, petgraph::Direction::Incoming) {
                to_visit.push(parent_idx);
            }
        }

        Ok(chain)
    }

    pub fn validate_acyclic(&self) -> Result<()> {
        if toposort(&self.graph, None).is_ok() {
            Ok(())
        } else {
            Err(anyhow!("Dependency graph contains cycles"))
        }
    }
}
```

---

## 6. Bulk Export & Compliance Reporting

### 6.1 100K+ Decision Export

```rust
// services/tool_registry_telemetry/bulk_export.rs
use tokio::io::AsyncWriteExt;
use flate2::write::GzEncoder;
use csv::Writer;

pub struct BulkExportConfig {
    pub batch_size: usize,       // 1000 decisions per batch
    pub format: ExportFormat,
    pub filters: ExportFilters,
    pub compression: bool,
}

#[derive(Clone, Debug)]
pub enum ExportFormat {
    JSON,
    JSONL,
    CSV,
    Parquet,
}

#[derive(Clone, Debug)]
pub struct ExportFilters {
    pub start_date: Option<chrono::DateTime<Utc>>,
    pub end_date: Option<chrono::DateTime<Utc>>,
    pub compliance_tiers: Vec<ComplianceTier>,
    pub decision_types: Vec<DecisionType>,
    pub regulations: Vec<String>,
}

pub struct BulkExporter {
    telemetry_stream: Arc<EventStream>,
    audit_log: Arc<AuditLog>,
}

impl BulkExporter {
    pub async fn export_decisions(
        &self,
        config: BulkExportConfig,
        output_path: &str,
    ) -> Result<ExportMetrics> {
        let start = std::time::Instant::now();
        let mut metrics = ExportMetrics {
            total_decisions: 0,
            exported_decisions: 0,
            filtered_decisions: 0,
            export_duration_ms: 0,
            file_size_bytes: 0,
        };

        let file = tokio::fs::File::create(output_path).await?;
        let writer: Box<dyn AsyncWriteExt + Unpin> = if config.compression {
            let encoder = GzEncoder::new(file, flate2::Compression::default());
            Box::new(encoder)
        } else {
            Box::new(file)
        };

        match config.format {
            ExportFormat::JSONL => {
                self.export_jsonl(writer, &config, &mut metrics).await?;
            }
            ExportFormat::CSV => {
                self.export_csv(writer, &config, &mut metrics).await?;
            }
            ExportFormat::Parquet => {
                self.export_parquet(output_path, &config, &mut metrics).await?;
            }
            _ => unimplemented!(),
        }

        metrics.export_duration_ms = start.elapsed().as_millis() as u64;
        metrics.file_size_bytes = tokio::fs::metadata(output_path).await?.len();

        info!("Export complete: {:?}", metrics);

        Ok(metrics)
    }

    async fn export_jsonl(
        &self,
        mut writer: Box<dyn AsyncWriteExt + Unpin>,
        config: &BulkExportConfig,
        metrics: &mut ExportMetrics,
    ) -> Result<()> {
        let mut offset = 0;

        loop {
            let decisions = self.telemetry_stream
                .query_decisions(config.filters.clone(), offset, config.batch_size)
                .await?;

            if decisions.is_empty() {
                break;
            }

            for decision in &decisions {
                let line = serde_json::to_string(decision)? + "\n";
                writer.write_all(line.as_bytes()).await?;
                metrics.exported_decisions += 1;
            }

            metrics.total_decisions += decisions.len();
            offset += config.batch_size;

            writer.flush().await?;
        }

        Ok(())
    }

    async fn export_csv(
        &self,
        writer: Box<dyn AsyncWriteExt + Unpin>,
        config: &BulkExportConfig,
        metrics: &mut ExportMetrics,
    ) -> Result<()> {
        let mut csv_writer = Writer::from_writer(writer);
        let mut offset = 0;

        loop {
            let decisions = self.telemetry_stream
                .query_decisions(config.filters.clone(), offset, config.batch_size)
                .await?;

            if decisions.is_empty() {
                break;
            }

            for decision in &decisions {
                csv_writer.serialize(DecisionCsvRecord::from(decision))?;
                metrics.exported_decisions += 1;
            }

            metrics.total_decisions += decisions.len();
            offset += config.batch_size;
        }

        csv_writer.flush()?;
        Ok(())
    }
}

#[derive(Debug, Serialize)]
pub struct ExportMetrics {
    pub total_decisions: usize,
    pub exported_decisions: usize,
    pub filtered_decisions: usize,
    pub export_duration_ms: u64,
    pub file_size_bytes: u64,
}
```

---

## 7. Compliance Metadata Mapping

```rust
// services/tool_registry_telemetry/compliance_mapping.rs

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegulationMapping {
    pub regulation: String,              // EU_AI_ACT, GDPR, SOC_2, HIPAA, etc.
    pub requirement: String,
    pub article: Option<String>,
    pub mandatory: bool,
    pub compliance_control_id: Option<String>,
}

pub struct ComplianceMetadataMapper {
    regulation_registry: Arc<RegulationRegistry>,
}

impl ComplianceMetadataMapper {
    pub async fn map_decision_to_regulations(
        &self,
        decision: &PolicyDecisionEvent,
    ) -> Result<Vec<RegulationMapping>> {
        let mut mappings = vec![];

        // EU AI Act: High-risk systems
        if self.is_high_risk_decision(decision) {
            mappings.extend(self.map_eu_ai_act(decision).await?);
        }

        // GDPR: EU data subjects
        if decision.user_context.region == "EU" {
            mappings.extend(self.map_gdpr(decision).await?);
        }

        // SOC 2: All decisions for audit trail
        mappings.extend(self.map_soc2(decision).await?);

        // HIPAA: Healthcare context
        if decision.tool_context.category == "healthcare" {
            mappings.extend(self.map_hipaa(decision).await?);
        }

        Ok(mappings)
    }

    async fn map_eu_ai_act(&self, decision: &PolicyDecisionEvent) -> Result<Vec<RegulationMapping>> {
        vec![
            RegulationMapping {
                regulation: "EU_AI_ACT".to_string(),
                requirement: "High-risk AI system logging".to_string(),
                article: Some("6(2)".to_string()),
                mandatory: true,
                compliance_control_id: Some("EU-AI-001".to_string()),
            },
            RegulationMapping {
                regulation: "EU_AI_ACT".to_string(),
                requirement: "Explainability and human oversight".to_string(),
                article: Some("13-14".to_string()),
                mandatory: true,
                compliance_control_id: Some("EU-AI-002".to_string()),
            },
        ]
    }

    async fn map_gdpr(&self, decision: &PolicyDecisionEvent) -> Result<Vec<RegulationMapping>> {
        vec![
            RegulationMapping {
                regulation: "GDPR".to_string(),
                requirement: "Automated decision-making rights (Article 22)".to_string(),
                article: Some("22".to_string()),
                mandatory: true,
                compliance_control_id: Some("GDPR-001".to_string()),
            },
            RegulationMapping {
                regulation: "GDPR".to_string(),
                requirement: "Data subject access to decision logic".to_string(),
                article: Some("15".to_string()),
                mandatory: true,
                compliance_control_id: Some("GDPR-002".to_string()),
            },
        ]
    }

    async fn map_soc2(&self, decision: &PolicyDecisionEvent) -> Result<Vec<RegulationMapping>> {
        vec![
            RegulationMapping {
                regulation: "SOC_2".to_string(),
                requirement: "Access control and audit trail (CC6.1, CC7.2)".to_string(),
                article: None,
                mandatory: true,
                compliance_control_id: Some("SOC2-001".to_string()),
            },
        ]
    }

    fn is_high_risk_decision(&self, decision: &PolicyDecisionEvent) -> bool {
        matches!(
            decision.decision_type,
            DecisionType::Deny { .. } | DecisionType::Escalate { .. }
        ) && decision.confidence_score < 0.8
    }
}
```

---

## 8. Performance & Scalability SLAs

### 8.1 Latency Targets

| Operation | p50 | p95 | p99 | SLA |
|-----------|-----|-----|-----|-----|
| Record Decision | 0.5ms | 2ms | 5ms | <5ms p99 |
| Submit Appeal | 2ms | 10ms | 25ms | <100ms p99 |
| Export 100K decisions | — | — | — | <10s |
| Query dependency graph | 1ms | 5ms | 15ms | <50ms p99 |

### 8.2 Throughput

- Decision recording: 10K decisions/sec per node
- Appeal processing: 500 appeals/sec per node
- Bulk export: 100K decisions in <10 seconds
- Horizontal scalability: Linear up to 10+ nodes

---

## 9. Testing & Validation

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_appeal_state_transitions() {
        let manager = create_test_manager().await;
        let appeal = manager.submit_appeal(
            uuid::Uuid::new_v4(),
            "user123".to_string(),
            "I disagree with this decision".to_string(),
        ).await.unwrap();

        assert_eq!(appeal.state, AppealState::Submitted);

        let updated = manager.transition_appeal(
            appeal.appeal_id,
            AppealState::Acknowledged,
            "Appeal acknowledged".to_string(),
            "system".to_string(),
        ).await.unwrap();

        assert_eq!(updated.state, AppealState::Acknowledged);
        assert_eq!(updated.state_transitions.len(), 2);
    }

    #[tokio::test]
    async fn test_escalation_detection() {
        let detector = create_test_detector().await;

        for _ in 0..3 {
            let event = create_test_denial_event();
            detector.check_triggers(&event).await.unwrap();
        }

        // Verify escalation triggered
        let triggers = detector.escalation_db
            .list("escalation_triggers")
            .await
            .unwrap();

        assert!(!triggers.is_empty());
    }

    #[tokio::test]
    async fn test_bulk_export_100k() {
        let exporter = create_test_exporter().await;

        let config = BulkExportConfig {
            batch_size: 1000,
            format: ExportFormat::JSONL,
            filters: ExportFilters::default(),
            compression: true,
        };

        let metrics = exporter
            .export_decisions(config, "/tmp/export.jsonl.gz")
            .await
            .unwrap();

        assert!(metrics.exported_decisions >= 100_000);
        assert!(metrics.export_duration_ms < 10_000);
    }
}
```

---

## 10. Deployment & Rollout

- **Phase 2a (Week 16):** Appeals & exceptions core functionality
- **Phase 2b (Week 17):** Escalation engine & dependency graph
- **Phase 2c (Week 18):** Bulk export & compliance reporting
- **Canary:** 5% traffic, 2x error budget
- **Monitoring:** P99 latency, error rates, appeal resolution time

---

## References

- **EU AI Act:** Articles 6, 13-14 (High-risk systems, transparency)
- **GDPR:** Articles 15, 22 (Data access, automated decision-making)
- **SOC 2:** Trust Services Criteria (Access control, audit)
- Previous: Week 15 PolicyDecision foundations

