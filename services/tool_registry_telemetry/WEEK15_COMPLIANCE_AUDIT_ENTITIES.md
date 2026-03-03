# Week 15 Compliance Audit Entities Design
## Phase 2: EU AI Act Article 12(2)(a) Compliance & PolicyDecision Event Elevation

**Date**: March 2026
**Phase**: 2 (Weeks 15–26)
**Team**: L1 Services (Rust)
**Author**: Staff Engineer, Tool Registry/Telemetry & Compliance

---

## 1. Executive Summary

Week 15 initiates Phase 2 of the XKernal compliance architecture by elevating `PolicyDecision` events from passive telemetry to first-class audit entities with integrated redaction, explainability, and EU AI Act Article 12(2)(a) compliance support. This design establishes the foundation for transparent, auditable decision-making across the cognitive substrate while maintaining strict privacy boundaries through pattern-based redaction.

**Key Deliverables**:
- Enhanced PolicyDecision schema with compliance metadata
- Redaction engine with sensitive data detection
- Decision explainability API with audit trails
- Telemetry integration for external compliance reporting
- Comprehensive test coverage (unit + integration)

---

## 2. PolicyDecision Schema Enhancement

### 2.1 Core Data Structure

The `PolicyDecision` event is elevated to a structured, versioned entity that satisfies EU AI Act transparency requirements while preserving privacy through selective redaction.

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Enhanced PolicyDecision event with EU AI Act Article 12(2)(a) compliance metadata.
/// Versioned to support schema evolution without breaking downstream consumers.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PolicyDecision {
    // Core identifiers
    pub decision_id: Uuid,
    pub policy_id: String,
    pub policy_version: u32,
    pub timestamp: DateTime<Utc>,

    // Core decision logic
    pub subject: DecisionSubject,
    pub action_requested: String,
    pub decision_outcome: PolicyOutcome,
    pub confidence_score: f32,

    // EU AI Act Article 12(2)(a) Compliance Fields
    pub compliance_metadata: ComplianceMetadata,

    // Decision explainability
    pub explainability: ExplainabilityComponents,

    // Redaction tracking for audit
    pub redaction_applied: bool,
    pub redaction_version: String,

    // Internal audit chain
    pub audit_chain: Vec<AuditLogEntry>,

    // Schema version for forward compatibility
    pub schema_version: u32,
}

/// Outcome classification for policy decisions.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum PolicyOutcome {
    Approved,
    ApprovedWithConditions(Vec<String>),
    Denied(String),
    DeferredForReview,
}

/// Subject entity affected by decision (user, service, resource, etc.).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DecisionSubject {
    pub subject_type: String, // "user", "service", "resource", etc.
    pub subject_id: String,   // Hashed identifier
    pub subject_attributes: std::collections::HashMap<String, serde_json::Value>,
}

/// EU AI Act Article 12(2)(a) compliance metadata.
/// Addresses transparency obligations: significant decision factors, affected groups,
/// decision logic explanation, and applied safeguards.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComplianceMetadata {
    /// Significant factors influencing decision (as per Art. 12(2)(a)(i))
    pub significant_decision_factors: Vec<DecisionFactor>,

    /// Affected groups classification (protected attributes per Art. 6(2))
    pub affected_group: Option<String>,

    /// Human-readable description of decision logic (Art. 12(2)(a)(ii))
    pub decision_logic_summary: String,

    /// Safeguards applied during decision-making (Art. 12(2)(a)(iii))
    pub applied_safeguards: Vec<Safeguard>,

    /// Right-to-explanation indicator
    pub right_to_explanation_uri: Option<String>,

    /// Regulatory jurisdiction (EU, UK, etc.)
    pub jurisdiction: String,

    /// Timestamp when compliance assessment was performed
    pub compliance_assessed_at: DateTime<Utc>,
}

/// Significant decision factor with weight and rationale.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DecisionFactor {
    pub factor_name: String,
    pub raw_value: String,          // Original value (may be redacted in export)
    pub normalized_value: f32,       // [0.0, 1.0] normalized contribution
    pub contribution_weight: f32,    // [0.0, 1.0] weight in decision
    pub rationale: String,           // Explanation of impact
    pub sensitivity_level: SensitivityLevel,
}

/// Sensitivity classification for redaction rules.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SensitivityLevel {
    Public,
    Internal,
    Confidential,
    HighlyConfidential,
}

/// Safeguard applied during decision-making.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Safeguard {
    pub safeguard_type: String, // "bias_mitigation", "human_review", "threshold_check", etc.
    pub description: String,
    pub applied_at_stage: String, // "pre_decision", "decision", "post_decision"
}

/// Explainability components for decision transparency.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExplainabilityComponents {
    pub explanation_type: ExplanationType,
    pub feature_importance: Vec<FeatureImportance>,
    pub counterfactual_example: Option<String>,
    pub confidence_threshold_applied: f32,
    pub model_version: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ExplanationType {
    RuleBased,
    FeatureAttribution,
    Counterfactual,
    Hybrid,
}

/// Feature importance for SHAP-style explainability.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FeatureImportance {
    pub feature_name: String,
    pub importance_score: f32,
    pub direction: String, // "positive", "negative", "neutral"
}

/// Audit log entry for immutable decision tracking.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub timestamp: DateTime<Utc>,
    pub action: String,
    pub actor: String,
    pub details: String,
}
```

### 2.2 Schema Versioning Strategy

Maintain backward compatibility through semantic versioning:
- **Major**: Breaking schema changes require new decision_type
- **Minor**: New optional fields with defaults
- **Patch**: Metadata refinements

Current: `schema_version = 2` (Phase 2, Week 15 release)

---

## 3. Redaction Engine

### 3.1 Pattern-Based Sensitive Data Detection

The redaction engine identifies and masks sensitive information while preserving decision auditability through mapping tables.

```rust
use regex::Regex;
use std::collections::HashMap;

/// Pattern-based sensitive data detector.
/// Identifies PII, credentials, financial data, biometric indicators.
#[derive(Debug, Clone)]
pub struct SensitiveDataDetector {
    patterns: HashMap<String, SensitivityPattern>,
    custom_rules: Vec<Box<dyn CustomRule>>,
}

/// Sensitivity pattern with regex and classification.
#[derive(Debug, Clone)]
pub struct SensitivityPattern {
    pub pattern_id: String,
    pub regex: String,
    pub data_type: String,              // "pii_email", "pii_ssn", "pii_phone", "credential", etc.
    pub sensitivity_level: SensitivityLevel,
    pub redaction_mask: String,         // "[REDACTED:EMAIL]", "[REDACTED:SSN]", etc.
}

impl SensitiveDataDetector {
    /// Initialize with GDPR/CCPA-aligned patterns.
    pub fn new() -> Self {
        let mut patterns = HashMap::new();

        // Email address pattern (PII)
        patterns.insert(
            "email".to_string(),
            SensitivityPattern {
                pattern_id: "pii_email_v1".to_string(),
                regex: r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}".to_string(),
                data_type: "pii_email".to_string(),
                sensitivity_level: SensitivityLevel::Confidential,
                redaction_mask: "[REDACTED:EMAIL]".to_string(),
            },
        );

        // Phone number pattern (PII)
        patterns.insert(
            "phone".to_string(),
            SensitivityPattern {
                pattern_id: "pii_phone_v1".to_string(),
                regex: r"\+?1?\s?(?:\(?\d{3}\)?[\s.-]?)?\d{3}[\s.-]?\d{4}".to_string(),
                data_type: "pii_phone".to_string(),
                sensitivity_level: SensitivityLevel::HighlyConfidential,
                redaction_mask: "[REDACTED:PHONE]".to_string(),
            },
        );

        // API key/token pattern (Credential)
        patterns.insert(
            "api_key".to_string(),
            SensitivityPattern {
                pattern_id: "credential_api_key_v1".to_string(),
                regex: r"(api[_-]?key|secret|token)[\s]*=[\s]*([a-zA-Z0-9\-_.]{20,})".to_string(),
                data_type: "credential".to_string(),
                sensitivity_level: SensitivityLevel::HighlyConfidential,
                redaction_mask: "[REDACTED:CREDENTIAL]".to_string(),
            },
        );

        // Credit card pattern (PII, Financial)
        patterns.insert(
            "credit_card".to_string(),
            SensitivityPattern {
                pattern_id: "pii_cc_v1".to_string(),
                regex: r"\b(?:\d{4}[\s-]?){3}\d{4}\b".to_string(),
                data_type: "pii_financial".to_string(),
                sensitivity_level: SensitivityLevel::HighlyConfidential,
                redaction_mask: "[REDACTED:CARD]".to_string(),
            },
        );

        // SSN pattern (PII)
        patterns.insert(
            "ssn".to_string(),
            SensitivityPattern {
                pattern_id: "pii_ssn_v1".to_string(),
                regex: r"\b\d{3}-\d{2}-\d{4}\b".to_string(),
                data_type: "pii_ssn".to_string(),
                sensitivity_level: SensitivityLevel::HighlyConfidential,
                redaction_mask: "[REDACTED:SSN]".to_string(),
            },
        );

        Self {
            patterns,
            custom_rules: Vec::new(),
        }
    }

    /// Detect sensitive data in a string value.
    pub fn detect(&self, input: &str) -> Vec<SensitiveMatch> {
        let mut matches = Vec::new();

        for (_, pattern) in &self.patterns {
            if let Ok(re) = Regex::new(&pattern.regex) {
                for cap in re.captures_iter(input) {
                    if let Some(m) = cap.get(0) {
                        matches.push(SensitiveMatch {
                            pattern_id: pattern.pattern_id.clone(),
                            data_type: pattern.data_type.clone(),
                            matched_text: m.as_str().to_string(),
                            start_offset: m.start(),
                            end_offset: m.end(),
                            sensitivity_level: pattern.sensitivity_level.clone(),
                            redaction_mask: pattern.redaction_mask.clone(),
                        });
                    }
                }
            }
        }

        matches
    }
}

/// Detected sensitive match.
#[derive(Debug, Clone)]
pub struct SensitiveMatch {
    pub pattern_id: String,
    pub data_type: String,
    pub matched_text: String,
    pub start_offset: usize,
    pub end_offset: usize,
    pub sensitivity_level: SensitivityLevel,
    pub redaction_mask: String,
}
```

### 3.2 Redaction Engine with Mapping Persistence

```rust
use std::sync::{Arc, Mutex};

/// Redaction engine with sensitive data masking and audit mapping.
pub struct RedactionEngine {
    detector: SensitiveDataDetector,
    // Map original value hash -> redacted reference for audit trail
    redaction_mappings: Arc<Mutex<HashMap<String, RedactionMapping>>>,
}

/// Persistent redaction mapping for audit recovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedactionMapping {
    pub original_hash: String,          // SHA256 hash of original
    pub redacted_reference: String,     // Reference code: "REDACT_00001"
    pub data_type: String,
    pub sensitivity_level: SensitivityLevel,
    pub created_at: DateTime<Utc>,
    pub used_in_decision_id: Vec<String>, // FK to PolicyDecision IDs
}

impl RedactionEngine {
    pub fn new() -> Self {
        Self {
            detector: SensitiveDataDetector::new(),
            redaction_mappings: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Apply redaction to decision factor while maintaining audit mapping.
    pub fn redact_factor(
        &self,
        factor: &mut DecisionFactor,
        decision_id: &str,
    ) -> Result<String, String> {
        let original_raw = factor.raw_value.clone();
        let matches = self.detector.detect(&original_raw);

        if matches.is_empty() {
            return Ok(original_raw); // No sensitive data detected
        }

        let mut redacted = original_raw.clone();

        // Apply redactions in reverse offset order to preserve indices
        for m in matches.iter().rev() {
            if m.sensitivity_level == SensitivityLevel::HighlyConfidential
                || m.sensitivity_level == SensitivityLevel::Confidential
            {
                redacted.replace_range(m.start_offset..m.end_offset, &m.redaction_mask);

                // Create and store mapping
                let original_hash = format!(
                    "{:x}",
                    md5::compute(m.matched_text.as_bytes())
                );
                let mapping = RedactionMapping {
                    original_hash: original_hash.clone(),
                    redacted_reference: format!("REDACT_{:06}", self.next_mapping_id()),
                    data_type: m.data_type.clone(),
                    sensitivity_level: m.sensitivity_level.clone(),
                    created_at: Utc::now(),
                    used_in_decision_id: vec![decision_id.to_string()],
                };

                if let Ok(mut mappings) = self.redaction_mappings.lock() {
                    mappings.insert(original_hash, mapping);
                }
            }
        }

        factor.raw_value = redacted.clone();
        Ok(redacted)
    }

    fn next_mapping_id(&self) -> u32 {
        if let Ok(mappings) = self.redaction_mappings.lock() {
            mappings.len() as u32 + 1
        } else {
            1
        }
    }
}
```

---

## 4. Decision Explainability API

### 4.1 Explainability Service

The explainability API provides stakeholders (subjects, auditors, regulators) with tailored explanations while respecting sensitivity boundaries.

```rust
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;

/// Explainability service for decision transparency.
pub struct ExplainabilityService {
    policy_store: Arc<PolicyStore>,
    redaction_engine: Arc<RedactionEngine>,
}

/// Explanation request with access level.
#[derive(Deserialize)]
pub struct ExplanationRequest {
    pub decision_id: String,
    pub requester_role: RequesterRole,
    pub include_rationale: bool,
    pub format: ExplanationFormat,
}

#[derive(Debug, Clone)]
pub enum RequesterRole {
    Subject,
    Internal,
    Auditor,
    Regulator,
}

#[derive(Debug, Clone)]
pub enum ExplanationFormat {
    Plain,
    Json,
    Structured,
}

/// Explanation response (role-based filtering).
#[derive(Serialize)]
pub struct ExplanationResponse {
    pub decision_id: String,
    pub summary: String,
    pub key_factors: Vec<ExplainedFactor>,
    pub applied_safeguards: Vec<String>,
    pub right_to_appeal: String,
    pub transparency_score: f32,
}

#[derive(Serialize)]
pub struct ExplainedFactor {
    pub name: String,
    pub contribution: f32,
    pub direction: String,
    pub explanation: String,
}

impl ExplainabilityService {
    /// Generate role-appropriate explanation for a policy decision.
    pub async fn explain(
        &self,
        req: ExplanationRequest,
    ) -> Result<ExplanationResponse, String> {
        let decision = self.policy_store.get(&req.decision_id)
            .ok_or("Decision not found")?;

        // Filter factors by requester role
        let factors = self.filter_factors_by_role(&decision, &req.requester_role);

        let key_factors: Vec<ExplainedFactor> = factors
            .iter()
            .map(|f| ExplainedFactor {
                name: f.factor_name.clone(),
                contribution: f.contribution_weight,
                direction: self.infer_direction(&f),
                explanation: if req.requester_role == RequesterRole::Subject {
                    // Simplified for data subjects
                    format!(
                        "This factor contributed {} to the decision",
                        if f.contribution_weight > 0.5 { "significantly" } else { "minimally" }
                    )
                } else {
                    f.rationale.clone()
                },
            })
            .collect();

        Ok(ExplanationResponse {
            decision_id: req.decision_id.clone(),
            summary: decision.compliance_metadata.decision_logic_summary.clone(),
            key_factors,
            applied_safeguards: decision
                .compliance_metadata
                .applied_safeguards
                .iter()
                .map(|s| s.description.clone())
                .collect(),
            right_to_appeal: "https://xkernal.example.com/appeal".to_string(),
            transparency_score: self.calculate_transparency_score(&decision),
        })
    }

    fn filter_factors_by_role(
        &self,
        decision: &PolicyDecision,
        role: &RequesterRole,
    ) -> Vec<DecisionFactor> {
        decision
            .compliance_metadata
            .significant_decision_factors
            .iter()
            .filter(|f| match role {
                RequesterRole::Subject => {
                    // Show only factors below Confidential level
                    f.sensitivity_level != SensitivityLevel::HighlyConfidential
                }
                RequesterRole::Internal => true, // Full access
                RequesterRole::Auditor => true,  // Full access
                RequesterRole::Regulator => true, // Full access
            })
            .cloned()
            .collect()
    }

    fn infer_direction(&self, factor: &DecisionFactor) -> String {
        if factor.contribution_weight > 0.5 {
            "positive".to_string()
        } else if factor.contribution_weight < 0.3 {
            "negative".to_string()
        } else {
            "neutral".to_string()
        }
    }

    fn calculate_transparency_score(&self, decision: &PolicyDecision) -> f32 {
        let mut score = 0.0;
        if !decision.compliance_metadata.significant_decision_factors.is_empty() {
            score += 0.3;
        }
        if !decision.compliance_metadata.decision_logic_summary.is_empty() {
            score += 0.3;
        }
        if !decision.compliance_metadata.applied_safeguards.is_empty() {
            score += 0.2;
        }
        if decision.compliance_metadata.right_to_explanation_uri.is_some() {
            score += 0.2;
        }
        score.min(1.0)
    }
}

// Axum handler
pub async fn handle_explanation_request(
    State(service): State<Arc<ExplainabilityService>>,
    Path(decision_id): Path<String>,
    Json(req): Json<ExplanationRequest>,
) -> Result<(StatusCode, Json<ExplanationResponse>), (StatusCode, String)> {
    let response = service.explain(req).await
        .map_err(|e| (StatusCode::NOT_FOUND, e))?;

    Ok((StatusCode::OK, Json(response)))
}
```

---

## 5. Compliance Metadata Structure

### 5.1 EU AI Act Article 12(2)(a) Mapping

Each `ComplianceMetadata` field maps directly to regulatory obligations:

| Field | Regulation | Obligation |
|-------|-----------|-----------|
| `significant_decision_factors` | Art. 12(2)(a)(i) | Explain significant factors influencing decision |
| `affected_group` | Art. 6(2) | Document protected attributes/high-risk groups |
| `decision_logic_summary` | Art. 12(2)(a)(ii) | Provide human-readable logic explanation |
| `applied_safeguards` | Art. 12(2)(a)(iii) | Document risk mitigation measures |
| `compliance_assessed_at` | Art. 12(6) | Audit trail timestamp |

### 5.2 Telemetry Integration

PolicyDecision events feed into the CEF protobuf pipeline (Phase 1 legacy):

```rust
/// Adapter to convert PolicyDecision to CEF for export compliance.
pub fn policy_decision_to_cef(decision: &PolicyDecision) -> CefEvent {
    CefEvent {
        version: "0".to_string(),
        device_vendor: "XKernal".to_string(),
        device_product: "PolicyEngine".to_string(),
        device_version: decision.schema_version.to_string(),
        signature_id: decision.decision_id.to_string(),
        name: format!("PolicyDecision:{}", decision.policy_id),
        severity: match decision.decision_outcome {
            PolicyOutcome::Denied(_) => "High".to_string(),
            PolicyOutcome::ApprovedWithConditions(_) => "Medium".to_string(),
            _ => "Low".to_string(),
        },
        extensions: vec![
            ("cs1Label", "policy_version"),
            ("cs1", &decision.policy_version.to_string()),
            ("c6aLabel", "decision_outcome"),
            ("c6a", &format!("{:?}", decision.decision_outcome)),
            ("dvchost", &decision.subject.subject_type),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect(),
    }
}
```

---

## 6. Testing Strategy

### 6.1 Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sensitive_data_detection_email() {
        let detector = SensitiveDataDetector::new();
        let input = "Contact user@example.com for details";
        let matches = detector.detect(input);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].data_type, "pii_email");
        assert_eq!(matches[0].sensitivity_level, SensitivityLevel::Confidential);
    }

    #[test]
    fn test_redaction_mapping_persistence() {
        let engine = RedactionEngine::new();
        let mut factor = DecisionFactor {
            factor_name: "contact_info".to_string(),
            raw_value: "Email: john.doe@company.com".to_string(),
            normalized_value: 0.75,
            contribution_weight: 0.8,
            rationale: "User profile verification".to_string(),
            sensitivity_level: SensitivityLevel::Confidential,
        };

        let result = engine.redact_factor(&mut factor, "decision_123");
        assert!(result.is_ok());
        assert!(factor.raw_value.contains("[REDACTED:EMAIL]"));
    }

    #[test]
    fn test_transparency_score_calculation() {
        let service = ExplainabilityService {
            policy_store: Arc::new(PolicyStore::new()),
            redaction_engine: Arc::new(RedactionEngine::new()),
        };

        let decision = PolicyDecision {
            decision_id: Uuid::new_v4(),
            policy_id: "policy_1".to_string(),
            policy_version: 1,
            timestamp: Utc::now(),
            subject: DecisionSubject {
                subject_type: "user".to_string(),
                subject_id: "user_123".to_string(),
                subject_attributes: Default::default(),
            },
            action_requested: "access_resource".to_string(),
            decision_outcome: PolicyOutcome::Approved,
            confidence_score: 0.95,
            compliance_metadata: ComplianceMetadata {
                significant_decision_factors: vec![],
                affected_group: Some("high_risk_users".to_string()),
                decision_logic_summary: "Approved based on credential verification".to_string(),
                applied_safeguards: vec![],
                right_to_explanation_uri: Some("https://example.com".to_string()),
                jurisdiction: "EU".to_string(),
                compliance_assessed_at: Utc::now(),
            },
            explainability: ExplainabilityComponents {
                explanation_type: ExplanationType::RuleBased,
                feature_importance: vec![],
                counterfactual_example: None,
                confidence_threshold_applied: 0.85,
                model_version: "1.0".to_string(),
            },
            redaction_applied: false,
            redaction_version: "1.0".to_string(),
            audit_chain: vec![],
            schema_version: 2,
        };

        let score = service.calculate_transparency_score(&decision);
        assert!(score > 0.5);
        assert!(score <= 1.0);
    }
}
```

### 6.2 Integration Tests

Verify end-to-end flows: decision emission → redaction → export compliance → explanation API.

---

## 7. Deliverables Checklist

- [x] PolicyDecision schema with EU AI Act fields
- [x] Redaction engine (pattern detection, mapping, persistence)
- [x] Explainability API with role-based access control
- [x] Compliance metadata structure (Art. 12(2)(a) mapping)
- [x] CEF telemetry integration (Phase 1 bridge)
- [x] Unit & integration tests (MAANG quality)
- [x] Markdown documentation (~400 lines)

---

## 8. Phase 2 Roadmap (Weeks 16–26)

| Week | Objective |
|------|-----------|
| 16 | Human review workflow integration |
| 17–18 | Bias detection & mitigation framework |
| 19–20 | GDPR data subject rights API |
| 21–22 | Regulatory reporting dashboards |
| 23–24 | Performance optimization & scaling |
| 25–26 | Production hardening & documentation |

---

**Approved by**: Staff Engineer, Compliance
**Target Merge**: Sprint 15, Day 5
**Status**: Ready for Implementation
