//! Phase 1 Readiness Assessment for Cognitive Substrate OS
//!
//! Comprehensive readiness assessment including gap analysis, dependency mapping,
//! risk register, migration checklist, and Phase 0 completion verification.
//! Identifies requirements for transition from Phase 0 to Phase 1.
//! See RFC: Week 6 Phase 1 Readiness Assessment design.

use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::error::{LifecycleError, Result};
use std::collections::BTreeMap as HashMap;

/// Readiness status for a single component.
///
/// Indicates whether a component is ready for Phase 1 deployment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadinessStatus {
    /// Component is ready
    Ready,
    /// Component is partially ready (needs work)
    Partial,
    /// Component is not ready
    NotReady,
    /// Status unknown/not assessed
    Unknown,
}

impl ReadinessStatus {
    /// Returns true if status indicates readiness.
    pub fn is_ready(&self) -> bool {
        matches!(self, ReadinessStatus::Ready)
    }

    /// Returns string representation.
    pub fn as_str(&self) -> &str {
        match self {
            ReadinessStatus::Ready => "Ready",
            ReadinessStatus::Partial => "Partial",
            ReadinessStatus::NotReady => "NotReady",
            ReadinessStatus::Unknown => "Unknown",
        }
    }
}

/// Gap in Phase 1 readiness.
///
/// Represents a missing capability or incomplete feature required for Phase 1.
#[derive(Debug, Clone)]
pub struct ReadinessGap {
    /// Unique gap identifier
    pub id: String,
    /// Gap title
    pub title: String,
    /// Detailed description
    pub description: String,
    /// Component this gap affects
    pub affected_component: String,
    /// Priority (1=critical, 2=high, 3=medium, 4=low)
    pub priority: u8,
    /// Estimated work items needed
    pub work_items: Vec<String>,
    /// Estimated effort in story points
    pub estimated_effort: u32,
}

impl ReadinessGap {
    /// Create a new readiness gap.
    pub fn new(id: String, title: String, affected_component: String) -> Self {
        Self {
            id,
            title,
            description: String::new(),
            affected_component,
            priority: 3,
            work_items: Vec::new(),
            estimated_effort: 0,
        }
    }

    /// Set gap description.
    pub fn with_description(mut self, desc: String) -> Self {
        self.description = desc;
        self
    }

    /// Set gap priority.
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority.min(4).max(1);
        self
    }

    /// Add a work item.
    pub fn add_work_item(mut self, item: String) -> Self {
        self.work_items.push(item);
        self
    }

    /// Set estimated effort.
    pub fn with_effort(mut self, effort: u32) -> Self {
        self.estimated_effort = effort;
        self
    }

    /// Get priority label.
    pub fn priority_label(&self) -> &str {
        match self.priority {
            1 => "CRITICAL",
            2 => "HIGH",
            3 => "MEDIUM",
            _ => "LOW",
        }
    }
}

/// Dependency between components.
///
/// Tracks dependencies required for Phase 1.
#[derive(Debug, Clone)]
pub struct ComponentDependency {
    /// Component that has the dependency
    pub dependent: String,
    /// Component that is depended upon
    pub dependency: String,
    /// Is dependency satisfied?
    pub satisfied: bool,
    /// Notes about the dependency
    pub notes: String,
}

impl ComponentDependency {
    /// Create a new component dependency.
    pub fn new(dependent: String, dependency: String) -> Self {
        Self {
            dependent,
            dependency,
            satisfied: false,
            notes: String::new(),
        }
    }

    /// Mark dependency as satisfied.
    pub fn with_satisfied(mut self, satisfied: bool) -> Self {
        self.satisfied = satisfied;
        self
    }

    /// Add notes about the dependency.
    pub fn with_notes(mut self, notes: String) -> Self {
        self.notes = notes;
        self
    }
}

/// Risk item in Phase 1 readiness.
///
/// Identifies potential risks and mitigation strategies.
#[derive(Debug, Clone)]
pub struct RiskItem {
    /// Unique risk identifier
    pub id: String,
    /// Risk description
    pub description: String,
    /// Probability (1=low, 2=medium, 3=high)
    pub probability: u8,
    /// Impact (1=low, 2=medium, 3=high)
    pub impact: u8,
    /// Mitigation strategy
    pub mitigation: String,
}

impl RiskItem {
    /// Create a new risk item.
    pub fn new(id: String, description: String) -> Self {
        Self {
            id,
            description,
            probability: 2,
            impact: 2,
            mitigation: String::new(),
        }
    }

    /// Set probability (1-3).
    pub fn with_probability(mut self, prob: u8) -> Self {
        self.probability = prob.min(3).max(1);
        self
    }

    /// Set impact (1-3).
    pub fn with_impact(mut self, imp: u8) -> Self {
        self.impact = imp.min(3).max(1);
        self
    }

    /// Set mitigation strategy.
    pub fn with_mitigation(mut self, strategy: String) -> Self {
        self.mitigation = strategy;
        self
    }

    /// Calculate risk score (probability * impact).
    pub fn score(&self) -> u8 {
        self.probability * self.impact
    }

    /// Get risk severity label.
    pub fn severity(&self) -> &str {
        match self.score() {
            9 => "CRITICAL",
            6..=8 => "HIGH",
            3..=5 => "MEDIUM",
            _ => "LOW",
        }
    }
}

/// Migration checklist item for Phase 0 to Phase 1 transition.
#[derive(Debug, Clone)]
pub struct MigrationChecklistItem {
    /// Item identifier
    pub id: String,
    /// Item description
    pub description: String,
    /// Is this item completed?
    pub completed: bool,
    /// Associated component
    pub component: String,
    /// Completion notes
    pub notes: Option<String>,
}

impl MigrationChecklistItem {
    /// Create a new checklist item.
    pub fn new(id: String, description: String, component: String) -> Self {
        Self {
            id,
            description,
            completed: false,
            component,
            notes: None,
        }
    }

    /// Mark item as completed.
    pub fn completed_with_notes(mut self, notes: String) -> Self {
        self.completed = true;
        self.notes = Some(notes);
        self
    }
}

/// Phase 0 completion verification.
///
/// Tracks whether Phase 0 deliverables have been completed.
#[derive(Debug, Clone)]
pub struct Phase0Completion {
    /// Basic lifecycle state machine: Ready?
    pub state_machine_ready: bool,
    /// Start/stop operations: Ready?
    pub start_stop_ready: bool,
    /// Resource cleanup: Ready?
    pub resource_cleanup_ready: bool,
    /// Health checks: Ready?
    pub health_checks_ready: bool,
    /// Restart policies: Ready?
    pub restart_policies_ready: bool,
    /// Unit file parsing: Ready?
    pub unit_file_ready: bool,
    /// CT spawn integration: Ready?
    pub ct_spawn_ready: bool,
    /// Notes on any gaps
    pub notes: Option<String>,
}

impl Phase0Completion {
    /// Create new Phase 0 completion tracker.
    pub fn new() -> Self {
        Self {
            state_machine_ready: false,
            start_stop_ready: false,
            resource_cleanup_ready: false,
            health_checks_ready: false,
            restart_policies_ready: false,
            unit_file_ready: false,
            ct_spawn_ready: false,
            notes: None,
        }
    }

    /// Check if all Phase 0 items are complete.
    pub fn all_complete(&self) -> bool {
        self.state_machine_ready &&
        self.start_stop_ready &&
        self.resource_cleanup_ready &&
        self.health_checks_ready &&
        self.restart_policies_ready &&
        self.unit_file_ready &&
        self.ct_spawn_ready
    }

    /// Count completed items.
    pub fn completed_count(&self) -> usize {
        vec![
            self.state_machine_ready,
            self.start_stop_ready,
            self.resource_cleanup_ready,
            self.health_checks_ready,
            self.restart_policies_ready,
            self.unit_file_ready,
            self.ct_spawn_ready,
        ]
        .iter()
        .filter(|&&x| x)
        .count()
    }

    /// Total count of items.
    pub fn total_count(&self) -> usize {
        7
    }
}

impl Default for Phase0Completion {
    fn default() -> Self {
        Self::new()
    }
}

/// Comprehensive Phase 1 readiness assessment.
///
/// Aggregates all readiness information and provides reports.
pub struct Phase1ReadinessAssessment {
    /// Assessment timestamp
    pub timestamp: SystemTime,
    /// Overall readiness status
    pub overall_status: ReadinessStatus,
    /// Phase 0 completion tracker
    pub phase0: Phase0Completion,
    /// Identified readiness gaps
    pub gaps: HashMap<String, ReadinessGap>,
    /// Component dependencies
    pub dependencies: Vec<ComponentDependency>,
    /// Risk register
    pub risks: HashMap<String, RiskItem>,
    /// Migration checklist
    pub checklist: Vec<MigrationChecklistItem>,
}

impl Phase1ReadinessAssessment {
    /// Create a new Phase 1 readiness assessment.
    pub fn new() -> Self {
        Self {
            timestamp: SystemTime::now(),
            overall_status: ReadinessStatus::Unknown,
            phase0: Phase0Completion::new(),
            gaps: HashMap::new(),
            dependencies: Vec::new(),
            risks: HashMap::new(),
            checklist: Vec::new(),
        }
    }

    /// Add a readiness gap.
    ///
    /// # Arguments
    /// * `gap` - Gap to add
    ///
    /// # Returns
    /// Result indicating success.
    pub fn add_gap(&mut self, gap: ReadinessGap) -> Result<()> {
        if self.gaps.insert(gap.id.clone(), gap).is_some() {
            return Err(LifecycleError::AssessmentError("Gap ID already exists".to_string()));
        }
        Ok(())
    }

    /// Add a component dependency.
    pub fn add_dependency(&mut self, dep: ComponentDependency) -> Result<()> {
        self.dependencies.push(dep);
        Ok(())
    }

    /// Add a risk item.
    ///
    /// # Arguments
    /// * `risk` - Risk to add
    ///
    /// # Returns
    /// Result indicating success.
    pub fn add_risk(&mut self, risk: RiskItem) -> Result<()> {
        if self.risks.insert(risk.id.clone(), risk).is_some() {
            return Err(LifecycleError::AssessmentError("Risk ID already exists".to_string()));
        }
        Ok(())
    }

    /// Add a checklist item.
    pub fn add_checklist_item(&mut self, item: MigrationChecklistItem) -> Result<()> {
        self.checklist.push(item);
        Ok(())
    }

    /// Calculate overall readiness status.
    ///
    /// # Returns
    /// ReadinessStatus based on assessment results.
    pub fn calculate_overall_status(&mut self) -> ReadinessStatus {
        // If Phase 0 not complete, cannot be ready
        if !self.phase0.all_complete() {
            self.overall_status = ReadinessStatus::NotReady;
            return self.overall_status;
        }

        // If critical risks exist, not ready
        let has_critical_risks = self.risks.values()
            .any(|r| r.severity() == "CRITICAL");
        if has_critical_risks {
            self.overall_status = ReadinessStatus::NotReady;
            return self.overall_status;
        }

        // If critical gaps exist, not ready
        let has_critical_gaps = self.gaps.values()
            .any(|g| g.priority == 1);
        if has_critical_gaps {
            self.overall_status = ReadinessStatus::NotReady;
            return self.overall_status;
        }

        // If unsatisfied dependencies, partial at best
        let unsatisfied_deps = self.dependencies.iter()
            .filter(|d| !d.satisfied)
            .count();
        if unsatisfied_deps > 0 {
            self.overall_status = ReadinessStatus::Partial;
            return self.overall_status;
        }

        // If there are gaps, partial
        if !self.gaps.is_empty() {
            self.overall_status = ReadinessStatus::Partial;
            return self.overall_status;
        }

        self.overall_status = ReadinessStatus::Ready;
        self.overall_status
    }

    /// Get critical gaps.
    pub fn critical_gaps(&self) -> Vec<&ReadinessGap> {
        self.gaps.values()
            .filter(|g| g.priority == 1)
            .collect()
    }

    /// Get critical risks.
    pub fn critical_risks(&self) -> Vec<&RiskItem> {
        self.risks.values()
            .filter(|r| r.severity() == "CRITICAL")
            .collect()
    }

    /// Get unsatisfied dependencies.
    pub fn unsatisfied_dependencies(&self) -> Vec<&ComponentDependency> {
        self.dependencies.iter()
            .filter(|d| !d.satisfied)
            .collect()
    }

    /// Get incomplete checklist items.
    pub fn incomplete_checklist_items(&self) -> Vec<&MigrationChecklistItem> {
        self.checklist.iter()
            .filter(|item| !item.completed)
            .collect()
    }

    /// Get checklist completion percentage.
    pub fn checklist_completion_percent(&self) -> f64 {
        if self.checklist.is_empty() {
            return 0.0;
        }
        let completed = self.checklist.iter()
            .filter(|item| item.completed)
            .count() as f64;
        (completed / self.checklist.len() as f64) * 100.0
    }

    /// Generate readiness report.
    pub fn generate_report(&self) -> String {
        let mut report = String::new();

        report.push_str("=== PHASE 1 READINESS ASSESSMENT REPORT ===\n\n");

        report.push_str(&format!("Overall Status: {}\n", self.overall_status.as_str()));
        report.push_str(&format!("Phase 0 Completion: {}/{}\n\n",
            self.phase0.completed_count(),
            self.phase0.total_count()));

        // Phase 0 status
        report.push_str("--- Phase 0 Status ---\n");
        report.push_str(&format!("State Machine: {}\n", if self.phase0.state_machine_ready { "Ready" } else { "Not Ready" }));
        report.push_str(&format!("Start/Stop Ops: {}\n", if self.phase0.start_stop_ready { "Ready" } else { "Not Ready" }));
        report.push_str(&format!("Resource Cleanup: {}\n", if self.phase0.resource_cleanup_ready { "Ready" } else { "Not Ready" }));
        report.push_str(&format!("Health Checks: {}\n", if self.phase0.health_checks_ready { "Ready" } else { "Not Ready" }));
        report.push_str(&format!("Restart Policies: {}\n", if self.phase0.restart_policies_ready { "Ready" } else { "Not Ready" }));
        report.push_str(&format!("Unit Files: {}\n", if self.phase0.unit_file_ready { "Ready" } else { "Not Ready" }));
        report.push_str(&format!("CT Spawn: {}\n\n", if self.phase0.ct_spawn_ready { "Ready" } else { "Not Ready" }));

        // Gaps
        if !self.gaps.is_empty() {
            report.push_str(&format!("--- Readiness Gaps ({}) ---\n", self.gaps.len()));
            for gap in self.gaps.values() {
                report.push_str(&format!("[{}] {} ({})\n", gap.priority_label(), gap.title, gap.affected_component));
            }
            report.push_str("\n");
        }

        // Risks
        if !self.risks.is_empty() {
            report.push_str(&format!("--- Risk Register ({}) ---\n", self.risks.len()));
            for risk in self.risks.values() {
                report.push_str(&format!("[{}] {} (Score: {})\n", risk.severity(), risk.description, risk.score()));
            }
            report.push_str("\n");
        }

        // Dependencies
        if !self.dependencies.is_empty() {
            let unsatisfied_count = self.unsatisfied_dependencies().len();
            report.push_str(&format!("--- Dependencies ({} unsatisfied of {}) ---\n",
                unsatisfied_count,
                self.dependencies.len()));
            for dep in &self.dependencies {
                let status = if dep.satisfied { "OK" } else { "UNMET" };
                report.push_str(&format!("{} => {} [{}]\n", dep.dependent, dep.dependency, status));
            }
            report.push_str("\n");
        }

        // Checklist
        let completion = self.checklist_completion_percent();
        report.push_str(&format!("--- Migration Checklist ({:.1}% complete) ---\n",
            completion));
        let incomplete_count = self.incomplete_checklist_items().len();
        report.push_str(&format!("Outstanding Items: {}/{}\n\n",
            incomplete_count,
            self.checklist.len()));

        report
    }
}

impl Default for Phase1ReadinessAssessment {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_readiness_status_is_ready() {
        assert!(ReadinessStatus::Ready.is_ready());
        assert!(!ReadinessStatus::Partial.is_ready());
        assert!(!ReadinessStatus::NotReady.is_ready());
        assert!(!ReadinessStatus::Unknown.is_ready());
    }

    #[test]
    fn test_readiness_gap_creation() {
        let gap = ReadinessGap::new(
            "gap-1".to_string(),
            "Health check timeout config".to_string(),
            "health_checks".to_string(),
        );

        assert_eq!(gap.id, "gap-1");
        assert_eq!(gap.priority, 3);
        assert_eq!(gap.estimated_effort, 0);
    }

    #[test]
    fn test_readiness_gap_builder() {
        let gap = ReadinessGap::new(
            "gap-1".to_string(),
            "Health check timeout config".to_string(),
            "health_checks".to_string(),
        )
        .with_priority(1)
        .with_effort(13);

        assert_eq!(gap.priority, 1);
        assert_eq!(gap.estimated_effort, 13);
        assert_eq!(gap.priority_label(), "CRITICAL");
    }

    #[test]
    fn test_component_dependency_creation() {
        let dep = ComponentDependency::new(
            "health_checks".to_string(),
            "lifecycle_manager".to_string(),
        );

        assert_eq!(dep.dependent, "health_checks");
        assert_eq!(dep.dependency, "lifecycle_manager");
        assert!(!dep.satisfied);
    }

    #[test]
    fn test_risk_item_creation() {
        let risk = RiskItem::new(
            "risk-1".to_string(),
            "Timeout during agent startup".to_string(),
        );

        assert_eq!(risk.id, "risk-1");
        assert_eq!(risk.probability, 2);
        assert_eq!(risk.impact, 2);
        assert_eq!(risk.score(), 4);
    }

    #[test]
    fn test_risk_severity() {
        let high_risk = RiskItem::new("r1".to_string(), "High".to_string())
            .with_probability(3)
            .with_impact(3);
        assert_eq!(high_risk.severity(), "CRITICAL");

        let medium_risk = RiskItem::new("r2".to_string(), "Medium".to_string())
            .with_probability(2)
            .with_impact(2);
        assert_eq!(medium_risk.severity(), "MEDIUM");
    }

    #[test]
    fn test_migration_checklist_item() {
        let item = MigrationChecklistItem::new(
            "item-1".to_string(),
            "Verify state machine transitions".to_string(),
            "lifecycle_manager".to_string(),
        );

        assert!(!item.completed);
        assert_eq!(item.component, "lifecycle_manager");
    }

    #[test]
    fn test_phase0_completion_creation() {
        let phase0 = Phase0Completion::new();
        assert!(!phase0.all_complete());
        assert_eq!(phase0.completed_count(), 0);
    }

    #[test]
    fn test_phase0_completion_all_complete() {
        let mut phase0 = Phase0Completion::new();
        phase0.state_machine_ready = true;
        phase0.start_stop_ready = true;
        phase0.resource_cleanup_ready = true;
        phase0.health_checks_ready = true;
        phase0.restart_policies_ready = true;
        phase0.unit_file_ready = true;
        phase0.ct_spawn_ready = true;

        assert!(phase0.all_complete());
        assert_eq!(phase0.completed_count(), 7);
    }

    #[test]
    fn test_phase1_readiness_assessment_creation() {
        let assessment = Phase1ReadinessAssessment::new();
        assert_eq!(assessment.overall_status, ReadinessStatus::Unknown);
        assert!(assessment.gaps.is_empty());
    }

    #[test]
    fn test_phase1_readiness_add_gap() {
        let mut assessment = Phase1ReadinessAssessment::new();
        let gap = ReadinessGap::new(
            "gap-1".to_string(),
            "Test".to_string(),
            "component".to_string(),
        );
        assert!(assessment.add_gap(gap).is_ok());
        assert_eq!(assessment.gaps.len(), 1);
    }

    #[test]
    fn test_phase1_readiness_add_duplicate_gap() {
        let mut assessment = Phase1ReadinessAssessment::new();
        let gap = ReadinessGap::new(
            "gap-1".to_string(),
            "Test".to_string(),
            "component".to_string(),
        );
        assessment.add_gap(gap).unwrap();

        let dup_gap = ReadinessGap::new(
            "gap-1".to_string(),
            "Test 2".to_string(),
            "component".to_string(),
        );
        assert!(assessment.add_gap(dup_gap).is_err());
    }

    #[test]
    fn test_phase1_readiness_calculate_overall_not_ready() {
        let mut assessment = Phase1ReadinessAssessment::new();
        // Phase 0 not complete
        let status = assessment.calculate_overall_status();
        assert_eq!(status, ReadinessStatus::NotReady);
    }

    #[test]
    fn test_phase1_readiness_calculate_overall_ready() {
        let mut assessment = Phase1ReadinessAssessment::new();
        let mut phase0 = Phase0Completion::new();
        phase0.state_machine_ready = true;
        phase0.start_stop_ready = true;
        phase0.resource_cleanup_ready = true;
        phase0.health_checks_ready = true;
        phase0.restart_policies_ready = true;
        phase0.unit_file_ready = true;
        phase0.ct_spawn_ready = true;
        assessment.phase0 = phase0;

        let status = assessment.calculate_overall_status();
        assert_eq!(status, ReadinessStatus::Ready);
    }

    #[test]
    fn test_phase1_readiness_critical_gaps() {
        let mut assessment = Phase1ReadinessAssessment::new();
        let critical_gap = ReadinessGap::new(
            "gap-1".to_string(),
            "Critical".to_string(),
            "component".to_string(),
        ).with_priority(1);
        assessment.add_gap(critical_gap).unwrap();

        let critical = assessment.critical_gaps();
        assert_eq!(critical.len(), 1);
    }

    #[test]
    fn test_phase1_readiness_unsatisfied_deps() {
        let mut assessment = Phase1ReadinessAssessment::new();
        let dep = ComponentDependency::new(
            "a".to_string(),
            "b".to_string(),
        );
        assessment.add_dependency(dep).unwrap();

        let unsatisfied = assessment.unsatisfied_dependencies();
        assert_eq!(unsatisfied.len(), 1);
    }

    #[test]
    fn test_phase1_readiness_checklist_completion() {
        let mut assessment = Phase1ReadinessAssessment::new();
        assessment.add_checklist_item(
            MigrationChecklistItem::new(
                "item-1".to_string(),
                "Test".to_string(),
                "component".to_string(),
            )
        ).unwrap();

        let completion = assessment.checklist_completion_percent();
        assert_eq!(completion, 0.0);
    }

    #[test]
    fn test_phase1_readiness_generate_report() {
        let assessment = Phase1ReadinessAssessment::new();
        let report = assessment.generate_report();
        assert!(report.contains("PHASE 1 READINESS ASSESSMENT"));
        assert!(report.contains("Overall Status"));
    }
}
