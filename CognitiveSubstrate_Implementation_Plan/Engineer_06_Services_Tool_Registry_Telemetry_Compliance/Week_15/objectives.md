# Engineer 6 — Services: Tool Registry, Telemetry & Compliance — Week 15

## Phase: Phase 2 (Weeks 15-24)

## Weekly Objective
Begin Phase 2 compliance architecture work by elevating PolicyDecision events to first-class audit entities with redaction support. Lay groundwork for EU AI Act compliance in Weeks 16-19.

## Document References
- **Primary:** Section 6.3 (Phase 2, Week 15-16: PolicyDecision as first-class event), Section 3.3.4 (Telemetry PolicyDecision events), Section 3.3.6 (Compliance, EU AI Act Article 12(2)(a) redaction)
- **Supporting:** Week 12 (Policy Engine), Week 11 (Telemetry integration)

## Deliverables
- [ ] PolicyDecision event schema enhancement
  - Add EU AI Act Article 12(2)(a) support: optional redacted_explanation field
  - Add explainability components: decision_factors, contributing_rules, why_denied
  - Add compliance metadata: regulatory_reference, remediation_path
  - Versioning for policy decision structure evolution
- [ ] Redaction engine implementation
  - Identify sensitive data in policy decisions (PII, credentials, internal IDs)
  - Apply redaction rules per privacy policy
  - Maintain mappings for audit trail recovery (encrypted redaction key)
  - Support selective redaction (some fields fully redacted, others partly)
- [ ] PolicyDecision event emission refinement
  - Emit complete PolicyDecision with full context internally
  - Emit redacted PolicyDecision externally (for customers/regulators)
  - Dual logging: internal audit trail + external compliance log
  - Timestamp and cryptographic hash for tamper-detection
- [ ] Decision explainability API
  - Export human-readable explanation of policy decisions
  - Trace decision back to specific policy rules
  - Show decision factors and their evaluation
  - Suggest remediation if decision was deny
- [ ] Integration with telemetry
  - PolicyDecision events flow through main telemetry event stream
  - Searchable by decision_type, rule_id, outcome
  - Part of compliance tier retention (≥6 months)
- [ ] Unit and integration tests
  - Redaction rules tested (correctness, no data leakage)
  - Explainability API tested (accuracy of explanations)
  - Dual logging tested (internal vs external versions)
  - EU AI Act compliance structure verified

## Technical Specifications

### Enhanced PolicyDecision Event Schema
```rust
pub struct PolicyDecision {
    pub decision_id: String,
    pub timestamp: i64,
    pub policy_version: u64,
    pub policy_version_hash: String,

    // Input
    pub requester_agent: String,
    pub requested_capability: String,
    pub context: Map<String, String>,

    // Decision
    pub outcome: PolicyOutcome,
    pub matching_rule_id: String,
    pub matching_rule_description: String,

    // Explainability (EU AI Act Article 12(2)(a))
    pub decision_factors: Vec<DecisionFactor>,
    pub explanation: String, // Full explanation (internal only)
    pub redacted_explanation: String, // Sanitized for external use

    // Compliance metadata
    pub regulatory_reference: Option<String>, // e.g., "EU AI Act Article 12(2)(a)"
    pub remediation_path: Option<String>, // How to appeal/fix
    pub audit_hash: String, // SHA-256(decision content) for tamper detection
}

pub struct DecisionFactor {
    pub factor_name: String,
    pub factor_value: String,
    pub contribution: f32, // 0.0-1.0: how much did this influence decision?
    pub sensitivity: SensitivityLevel, // PUBLIC, INTERNAL, CONFIDENTIAL
}

pub enum SensitivityLevel {
    Public,     // Safe to share externally
    Internal,   // Shared with operators only
    Confidential, // Auditors and legal only
}

// EU AI Act Article 12(2)(a) compliance structure
pub struct AIAct12_2a_Explanation {
    pub significant_decision_factors: Vec<String>,
    pub relevant_affected_group: String,
    pub decision_logic: String,
    pub safeguards: Vec<String>,
}

impl PolicyDecision {
    pub fn to_external_format(&self) -> ExternalPolicyDecision {
        ExternalPolicyDecision {
            decision_id: self.decision_id.clone(),
            timestamp: self.timestamp,
            outcome: self.outcome.clone(),
            redacted_explanation: self.redacted_explanation.clone(),
            regulatory_reference: self.regulatory_reference.clone(),
            remediation_path: self.remediation_path.clone(),
        }
    }

    pub fn to_audit_format(&self) -> AuditPolicyDecision {
        // Full decision with audit hash for tamper detection
        AuditPolicyDecision {
            decision: self.clone(),
            audit_hash: self.audit_hash.clone(),
            sign_timestamp: now(),
        }
    }
}

pub struct ExternalPolicyDecision {
    pub decision_id: String,
    pub timestamp: i64,
    pub outcome: PolicyOutcome,
    pub redacted_explanation: String,
    pub regulatory_reference: Option<String>,
    pub remediation_path: Option<String>,
}
```

### Redaction Engine
```rust
pub struct RedactionEngine {
    rules: Vec<RedactionRule>,
}

pub struct RedactionRule {
    pub field_pattern: String, // Regex pattern for field names
    pub strategy: RedactionStrategy,
}

pub enum RedactionStrategy {
    FullRedaction,           // Replace entire value with [REDACTED]
    PartialRedaction(usize), // Keep first N chars, redact rest
    HashRedaction,           // Replace with salted hash
    Tokenization,            // Replace with opaque token
}

impl RedactionEngine {
    pub fn new() -> Self {
        Self {
            rules: vec![
                RedactionRule {
                    field_pattern: ".*password.*".to_string(),
                    strategy: RedactionStrategy::FullRedaction,
                },
                RedactionRule {
                    field_pattern: ".*api_key.*".to_string(),
                    strategy: RedactionStrategy::FullRedaction,
                },
                RedactionRule {
                    field_pattern: ".*email.*".to_string(),
                    strategy: RedactionStrategy::PartialRedaction(3), // show first 3 chars
                },
                RedactionRule {
                    field_pattern: ".*agent_id.*".to_string(),
                    strategy: RedactionStrategy::HashRedaction,
                },
            ],
        }
    }

    pub fn redact_decision(&self, decision: &PolicyDecision) -> String {
        let mut explanation = decision.explanation.clone();

        for rule in &self.rules {
            let re = regex::Regex::new(&rule.field_pattern).unwrap();
            match &rule.strategy {
                RedactionStrategy::FullRedaction => {
                    explanation = re.replace_all(&explanation, "[REDACTED]").to_string();
                }
                RedactionStrategy::PartialRedaction(keep_chars) => {
                    // Redact all but first N characters
                    explanation = re.replace_all(&explanation, |caps: &regex::Captures| {
                        let original = caps.get(0).unwrap().as_str();
                        if original.len() > *keep_chars {
                            format!("{}...", &original[..*keep_chars])
                        } else {
                            "[REDACTED]".to_string()
                        }
                    }).to_string();
                }
                RedactionStrategy::HashRedaction => {
                    // Replace with salted hash (deterministic but not reversible)
                    explanation = re.replace_all(&explanation, |caps: &regex::Captures| {
                        let original = caps.get(0).unwrap().as_str();
                        self.hash_value(original)
                    }).to_string();
                }
                RedactionStrategy::Tokenization => {
                    // Replace with opaque token (stored separately)
                    // Implementation depends on token storage backend
                }
            }
        }

        explanation
    }

    fn hash_value(&self, value: &str) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(format!("{}:{}", "salt", value).as_bytes());
        let result = hasher.finalize();
        format!("hash_{:x}", result)[..16].to_string() // Truncate for readability
    }

    pub fn verify_no_pii(&self, text: &str) -> Result<(), RedactionError> {
        // Scan text for common PII patterns
        let pii_patterns = vec![
            (r"\b\d{3}-\d{2}-\d{4}\b", "SSN"), // US SSN
            (r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b", "Email"),
            (r"\b\d{16}\b", "Credit Card"),
        ];

        for (pattern, pii_type) in pii_patterns {
            if regex::Regex::new(pattern).unwrap().is_match(text) {
                return Err(RedactionError::PiiDetected(pii_type.to_string()));
            }
        }

        Ok(())
    }
}
```

### Decision Explainability API
```rust
pub struct ExplainabilityService {
    decision_log: Arc<Mutex<VecDeque<PolicyDecision>>>,
    policy_engine: Arc<MandatoryPolicyEngine>,
    redaction_engine: Arc<RedactionEngine>,
}

pub struct DecisionExplanation {
    pub decision_id: String,
    pub summary: String,
    pub factors: Vec<ExplainedFactor>,
    pub matching_rule: ExplainedRule,
    pub why_denied: Option<String>,
    pub remediation_suggestions: Vec<String>,
    pub regulatory_references: Vec<String>,
}

pub struct ExplainedFactor {
    pub name: String,
    pub value: String,
    pub influenced_outcome: bool,
    pub weight: f32,
}

pub struct ExplainedRule {
    pub rule_id: String,
    pub description: String,
    pub conditions: Vec<String>,
    pub decision: String,
}

impl ExplainabilityService {
    pub async fn explain_decision(&self, decision_id: &str)
        -> Result<DecisionExplanation, ExplainError>
    {
        let log = self.decision_log.lock().await;
        let decision = log.iter()
            .find(|d| d.decision_id == decision_id)
            .ok_or(ExplainError::NotFound)?;

        let mut explanation = DecisionExplanation {
            decision_id: decision_id.to_string(),
            summary: format!(
                "Capability '{}' for agent '{}' was {}.",
                decision.requested_capability,
                decision.requester_agent,
                match decision.outcome {
                    PolicyOutcome::Allow => "ALLOWED",
                    PolicyOutcome::Deny => "DENIED",
                    PolicyOutcome::RequireApproval => "FLAGGED FOR APPROVAL",
                    _ => "PROCESSED",
                }
            ),
            factors: decision.decision_factors.iter()
                .map(|f| ExplainedFactor {
                    name: f.factor_name.clone(),
                    value: f.factor_value.clone(),
                    influenced_outcome: f.contribution > 0.1,
                    weight: f.contribution,
                })
                .collect(),
            matching_rule: ExplainedRule {
                rule_id: decision.matching_rule_id.clone(),
                description: decision.matching_rule_description.clone(),
                conditions: vec![], // Expanded from rule conditions
                decision: format!("{:?}", decision.outcome),
            },
            why_denied: if matches!(decision.outcome, PolicyOutcome::Deny) {
                Some("This capability violates policy rules. See remediation suggestions below.".to_string())
            } else {
                None
            },
            remediation_suggestions: vec![
                "Contact policy administrator to request exception".to_string(),
                "Modify request parameters to match policy requirements".to_string(),
            ],
            regulatory_references: vec![
                "EU AI Act Article 12(2)(a) - Right to explanation".to_string(),
            ],
        };

        Ok(explanation)
    }

    pub fn suggest_remediation(&self, decision: &PolicyDecision) -> Vec<String> {
        match decision.outcome {
            PolicyOutcome::Deny => vec![
                "Request a policy exception from policy_admin".to_string(),
                "Use a different, less-privileged capability if available".to_string(),
                "Wait and retry during allowed time window (if time-based policy)".to_string(),
            ],
            PolicyOutcome::RequireApproval => vec![
                "Wait for manual approval from policy_admin".to_string(),
                "Contact policy_admin to expedite approval".to_string(),
            ],
            _ => vec![],
        }
    }
}
```

### Dual Logging: Internal vs External
```rust
pub struct ComplianceLogger {
    internal_log: Arc<Mutex<File>>, // Full decisions
    external_log: Arc<Mutex<File>>, // Redacted decisions
    redaction_engine: Arc<RedactionEngine>,
}

impl ComplianceLogger {
    pub async fn log_policy_decision(&self, decision: &PolicyDecision)
        -> Result<(), LogError>
    {
        // Internal log: full decision with all context
        let internal_entry = serde_json::json!({
            "type": "INTERNAL_POLICY_DECISION",
            "decision": decision,
            "logged_at": now(),
        });

        self.internal_log.lock().await
            .write_all(format!("{}\n", internal_entry.to_string()).as_bytes())?;

        // External log: redacted decision suitable for regulatory review
        let redacted_explanation = self.redaction_engine.redact_decision(decision);
        let external_entry = serde_json::json!({
            "type": "POLICY_DECISION",
            "decision_id": decision.decision_id,
            "timestamp": decision.timestamp,
            "outcome": decision.outcome,
            "explanation": redacted_explanation,
            "regulatory_reference": decision.regulatory_reference,
            "audit_hash": decision.audit_hash,
        });

        self.external_log.lock().await
            .write_all(format!("{}\n", external_entry.to_string()).as_bytes())?;

        Ok(())
    }

    pub async fn export_for_audit(&self, output_path: &Path) -> Result<u64, ExportError> {
        // Export external log (suitable for regulator review)
        let content = std::fs::read_to_string(&self.external_log_path)?;
        std::fs::write(output_path, content)?;
        Ok(content.lines().count() as u64)
    }
}
```

## Dependencies
- **Blocked by:** Phase 1 (Weeks 7-14 complete), Week 12 (Policy Engine)
- **Blocking:** Week 16-20 (compliance engine and audit trail)

## Acceptance Criteria
- [ ] PolicyDecision event schema enhanced with explainability fields
- [ ] Redaction engine functional; PII detection working
- [ ] Redaction strategies tested (full, partial, hash, tokenization)
- [ ] PolicyDecision events emitted with audit_hash for tamper detection
- [ ] Explainability API returns human-readable explanations
- [ ] Decision factors extracted and weighted
- [ ] Remediation suggestions generated per outcome type
- [ ] Dual logging (internal + external) functional
- [ ] External log suitable for regulator review (no PII)
- [ ] EU AI Act Article 12(2)(a) compliance structure validated
- [ ] Unit tests cover redaction, explainability, dual logging

## Design Principles Alignment
- **Transparency:** Decisions explainable to agents and regulators
- **Privacy:** Sensitive data redacted in external logs; internal logs retained
- **Compliance:** EU AI Act Article 12(2)(a) requirements met
- **Auditability:** Tamper-detection via audit hash; cryptographic chain
- **Usability:** Remediation suggestions help agents appeal/fix denials
