// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Integration tests for tool_registry_telemetry L1 scaffolds.

use cs_tool_registry_telemetry::compliance_l1::{
    AuditEntry, ComplianceEngine, MerkleNode, PolicyContext, PolicyDecision, RetentionPolicy,
};
use cs_tool_registry_telemetry::journal::{CognitiveJournal, JournalEntry, RedactionLevel};
use cs_tool_registry_telemetry::registry::{McpRegistry, SandboxLevel, SandboxPolicy, ToolBinding};
use cs_tool_registry_telemetry::telemetry::{CefEvent, CostAttribution, StreamingProcessor};

#[test]
fn test_tool_binding_creation() {
    let binding = ToolBinding::new(1, String::from("test_tool"), SandboxLevel::Process)
        .with_mcp(true)
        .with_hash(12345);

    assert_eq!(binding.tool_id, 1);
    assert!(binding.is_mcp_native);
    assert_eq!(binding.capability_hash, 12345);
}

#[test]
fn test_mcp_registry_operations() {
    let mut registry = McpRegistry::new(10);

    let b1 = ToolBinding::new(1, String::from("tool1"), SandboxLevel::Process);
    let b2 = ToolBinding::new(2, String::from("tool2"), SandboxLevel::Container);

    assert!(registry.register(b1).is_ok());
    assert!(registry.register(b2).is_ok());

    assert_eq!(registry.total_tools(), 2);
    assert!(registry.get(1).is_some());
    assert!(registry.get_by_name("tool1").is_some());
}

#[test]
fn test_registry_sandbox_filtering() {
    let mut registry = McpRegistry::new(10);

    registry.register(ToolBinding::new(1, String::from("t1"), SandboxLevel::Process)).unwrap();
    registry.register(ToolBinding::new(2, String::from("t2"), SandboxLevel::Process)).unwrap();
    registry.register(ToolBinding::new(3, String::from("t3"), SandboxLevel::Container)).unwrap();

    assert_eq!(registry.count_by_sandbox(SandboxLevel::Process), 2);
    assert_eq!(registry.count_by_sandbox(SandboxLevel::Container), 1);
}

#[test]
fn test_sandbox_policy_enforcement() {
    let policy = SandboxPolicy::new(SandboxLevel::Container);

    assert!(policy.is_allowed(SandboxLevel::Container));
    assert!(policy.is_allowed(SandboxLevel::Virtual));
    assert!(!policy.is_allowed(SandboxLevel::Process));
    assert!(!policy.is_allowed(SandboxLevel::None));

    assert!(policy.meets_minimum(SandboxLevel::Virtual));
    assert!(!policy.meets_minimum(SandboxLevel::Process));
}

#[test]
fn test_cef_event_creation() {
    let event = CefEvent::new(String::from("device1"), 1, 5);
    assert_eq!(event.event_id, 1);
    assert_eq!(event.severity, 5);

    let mut ext = event.extension.clone();
    ext.source_ip = String::from("192.168.1.1");
    assert_eq!(ext.source_ip, "192.168.1.1");
}

#[test]
fn test_cost_attribution_tracking() {
    let mut cost = CostAttribution::new(1);
    cost.add_compute(50.0);
    cost.add_memory(30.0);
    cost.add_io(15.0);
    cost.add_network(5.0);

    assert_eq!(cost.total_cost, 100.0);

    let (comp, mem, io, net) = cost.cost_breakdown();
    assert!(comp > 49.0 && comp < 51.0);
    assert!(mem > 29.0 && mem < 31.0);
}

#[test]
fn test_streaming_processor() {
    let mut processor = StreamingProcessor::new(10, 1000);

    for i in 0..5 {
        let event = CefEvent::new(String::from("device1"), i, 5);
        processor.add_event(event).unwrap();
    }

    assert_eq!(processor.buffer_size(), 5);
    processor.flush().unwrap();
    assert_eq!(processor.buffer_size(), 0);
}

#[test]
fn test_policy_context_and_decision() {
    let ctx = PolicyContext::new(1, 100, 5, 1000);
    assert_eq!(ctx.user_id, 1);
    assert_eq!(ctx.tool_id, 100);

    let decision = PolicyDecision::Allow;
    assert_eq!(decision, PolicyDecision::Allow);
}

#[test]
fn test_merkle_tree_integrity() {
    let leaf1 = MerkleNode::new_leaf(12345);
    let leaf2 = MerkleNode::new_leaf(67890);
    let parent = MerkleNode::new_parent(leaf1.hash, leaf2.hash);

    assert!(leaf1.verify());
    assert!(parent.verify());
}

#[test]
fn test_audit_entry_logging() {
    let mut engine = ComplianceEngine::default();

    let entry = AuditEntry::new(1, 1, 5, PolicyDecision::Allow);
    assert!(engine.log_entry(entry).is_ok());

    assert_eq!(engine.audit_count(), 1);
    assert!(engine.verify_integrity());

    let retrieved = engine.get_audit_entry(1).unwrap();
    assert_eq!(retrieved.user_id, 1);
}

#[test]
fn test_retention_policy() {
    let policy = RetentionPolicy::new(90);
    assert!(policy.is_retention_valid());
    assert!(!policy.should_purge(80));
    assert!(policy.should_purge(100));
}

#[test]
fn test_journal_entry_creation() {
    let entry = JournalEntry::new(1, String::from("context"), RedactionLevel::Public)
        .with_observation(String::from("observed X"))
        .with_decision(String::from("decide Y"))
        .with_action(String::from("do Z"))
        .with_outcome(String::from("result W"));

    assert_eq!(entry.entry_id, 1);
    assert_eq!(entry.observation, "observed X");
}

#[test]
fn test_redaction_levels() {
    let public = JournalEntry::new(1, String::from("public"), RedactionLevel::Public);
    let confidential =
        JournalEntry::new(2, String::from("secret"), RedactionLevel::HighlyConfidential);

    let pub_redacted = public.redacted();
    assert_eq!(pub_redacted.context, "public"); // Unchanged

    let conf_redacted = confidential.redacted();
    assert_eq!(conf_redacted.context, "[REDACTED]"); // Fully redacted
}

#[test]
fn test_cognitive_journal_workflow() {
    let mut journal = CognitiveJournal::new(100);

    let entry1 = JournalEntry::new(0, String::from("ctx1"), RedactionLevel::Public);
    let entry2 = JournalEntry::new(0, String::from("ctx2"), RedactionLevel::Internal);

    let id1 = journal.record(entry1).unwrap();
    let id2 = journal.record(entry2).unwrap();

    assert_eq!(journal.entry_count(), 2);

    assert!(journal.get(id1).is_some());
    let redacted = journal.get_redacted(id2).unwrap();
    assert_eq!(redacted.redaction_level, RedactionLevel::Internal);
}

#[test]
fn test_integrated_registry_and_journal() {
    let mut registry = McpRegistry::new(10);
    let mut journal = CognitiveJournal::new(100);

    // Register tools
    registry.register(ToolBinding::new(1, String::from("t1"), SandboxLevel::Process)).unwrap();

    // Record journal entry for tool execution
    let entry = JournalEntry::new(0, String::from("tool 1 execution"), RedactionLevel::Internal);
    let _ = journal.record(entry).unwrap();

    assert_eq!(registry.total_tools(), 1);
    assert_eq!(journal.entry_count(), 1);
}

#[test]
fn test_compliance_with_telemetry() {
    let mut engine = ComplianceEngine::default();
    let mut processor = StreamingProcessor::default();

    // Log compliance event
    let entry = AuditEntry::new(1, 1, 5, PolicyDecision::Allow);
    engine.log_entry(entry).unwrap();

    // Add telemetry event
    let event = CefEvent::new(String::from("device1"), 1, 5);
    processor.add_event(event).unwrap();

    assert_eq!(engine.audit_count(), 1);
    assert_eq!(processor.buffer_size(), 1);
}

#[test]
fn test_full_workflow() {
    // Registry setup
    let mut registry = McpRegistry::new(10);
    registry.register(ToolBinding::new(1, String::from("compute_tool"), SandboxLevel::Container)).unwrap();

    // Policy setup
    let policy = SandboxPolicy::new(SandboxLevel::Process);
    assert!(policy.is_allowed(SandboxLevel::Container));

    // Compliance setup
    let mut compliance = ComplianceEngine::default();
    let audit = AuditEntry::new(1, 1, 1, PolicyDecision::Allow);
    compliance.log_entry(audit).unwrap();

    // Journal setup
    let mut journal = CognitiveJournal::new(100);
    let entry = JournalEntry::new(0, String::from("workflow"), RedactionLevel::Internal);
    journal.record(entry).unwrap();

    // Telemetry
    let mut processor = StreamingProcessor::default();
    let event = CefEvent::new(String::from("system"), 1, 3);
    processor.add_event(event).unwrap();

    // Verify all components
    assert_eq!(registry.total_tools(), 1);
    assert!(policy.meets_minimum(SandboxLevel::Container));
    assert!(compliance.verify_integrity());
    assert_eq!(journal.entry_count(), 1);
    assert_eq!(processor.buffer_size(), 1);
}

#[test]
fn test_error_handling() {
    // Registry full
    let mut registry = McpRegistry::new(1);
    registry.register(ToolBinding::new(1, String::from("t1"), SandboxLevel::Process)).unwrap();
    assert!(registry.register(ToolBinding::new(2, String::from("t2"), SandboxLevel::Process)).is_err());

    // Journal full
    let mut journal = CognitiveJournal::new(1);
    journal.record(JournalEntry::new(0, String::from("e1"), RedactionLevel::Public)).unwrap();
    assert!(journal.record(JournalEntry::new(0, String::from("e2"), RedactionLevel::Public)).is_err());
}

#[test]
fn test_compound_efficiency_constant() {
    const COMPOUND_EFFICIENCY: f64 = 0.581;
    assert!(COMPOUND_EFFICIENCY > 0.0 && COMPOUND_EFFICIENCY < 1.0);

    // Use in a realistic scenario
    let base_efficiency = 0.85;
    let adjusted_efficiency = base_efficiency * COMPOUND_EFFICIENCY;
    assert!(adjusted_efficiency < base_efficiency);
}
