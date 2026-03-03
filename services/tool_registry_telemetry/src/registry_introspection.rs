// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Registry introspection and querying interface.
//!
//! This module provides high-level query and introspection APIs for the tool registry,
//! enabling discovery and filtering of tools by various criteria: effect class, capability,
//! tool type, agent, and other metadata.
//!
//! Introspection is critical for:
//! - Discovering available tools for a given task
//! - Checking effect class compatibility with context
//! - Building execution plans with appropriate tools
//! - Auditing tool availability and permissions
//!
//! See Engineering Plan § 2.11: ToolBinding Entity & Tool Registry.

use crate::effect_class::EffectClass;
use crate::error::Result;
use crate::ids::{AgentID, ToolID};
use crate::tool_binding::ToolBinding;
use crate::tool_registry::ToolRegistry;
use alloc::vec::Vec;

/// Query for filtering tool bindings by various criteria.
///
/// Enables complex queries combining multiple filter conditions.
///
/// # Example
///
/// ```ignore
/// let query = ToolQuery::new()
///     .with_effect_class(EffectClass::ReadOnly)
///     .with_agent(&agent_id);
///
/// let results = registry.query(query)?;
/// ```
#[derive(Clone, Debug)]
pub struct ToolQuery {
    /// Filter by effect class (optional)
    effect_class: Option<EffectClass>,

    /// Filter by agent (optional)
    agent: Option<AgentID>,

    /// Filter by tool ID (optional)
    tool: Option<ToolID>,

    /// Filter: only read-only tools (optional)
    read_only_only: bool,

    /// Filter: only tools requiring confirmation (optional)
    requires_confirmation: bool,

    /// Filter: only tools with commit protocol (optional)
    has_commit_protocol: bool,
}

impl ToolQuery {
    /// Creates a new empty query that matches all bindings.
    pub fn new() -> Self {
        ToolQuery {
            effect_class: None,
            agent: None,
            tool: None,
            read_only_only: false,
            requires_confirmation: false,
            has_commit_protocol: false,
        }
    }

    /// Filters by effect class.
    pub fn with_effect_class(mut self, effect_class: EffectClass) -> Self {
        self.effect_class = Some(effect_class);
        self
    }

    /// Filters by agent.
    pub fn with_agent(mut self, agent: AgentID) -> Self {
        self.agent = Some(agent);
        self
    }

    /// Filters by tool.
    pub fn with_tool(mut self, tool: ToolID) -> Self {
        self.tool = Some(tool);
        self
    }

    /// Filters to only read-only tools.
    pub fn read_only_only(mut self) -> Self {
        self.read_only_only = true;
        self
    }

    /// Filters to only tools requiring confirmation.
    pub fn requires_confirmation_only(mut self) -> Self {
        self.requires_confirmation = true;
        self
    }

    /// Filters to only tools with commit protocol.
    pub fn has_commit_protocol_only(mut self) -> Self {
        self.has_commit_protocol = true;
        self
    }

    /// Evaluates the query against a tool binding.
    ///
    /// Returns true if the binding matches all query criteria.
    fn matches(&self, binding: &ToolBinding) -> bool {
        // Check effect class
        if let Some(ec) = self.effect_class {
            if binding.effect_class != ec {
                return false;
            }
        }

        // Check agent
        if let Some(ref agent) = self.agent {
            if binding.agent != *agent {
                return false;
            }
        }

        // Check tool
        if let Some(ref tool) = self.tool {
            if binding.tool != *tool {
                return false;
            }
        }

        // Check read-only filter
        if self.read_only_only && !binding.is_read_only() {
            return false;
        }

        // Check confirmation filter
        if self.requires_confirmation && !binding.requires_confirmation() {
            return false;
        }

        // Check commit protocol filter
        if self.has_commit_protocol && binding.commit_protocol.is_none() {
            return false;
        }

        true
    }
}

impl Default for ToolQuery {
    fn default() -> Self {
        Self::new()
    }
}

/// Registry introspection interface.
///
/// Provides high-level query and discovery APIs for the tool registry.
pub trait RegistryIntrospection {
    /// Queries the registry with a tool query.
    ///
    /// # Arguments
    ///
    /// - `query`: Query with filter criteria
    ///
    /// # Returns
    ///
    /// Vector of matching tool bindings
    fn query(&self, query: ToolQuery) -> Vec<ToolBinding>;

    /// Lists all bindings with a specific effect class.
    fn list_by_effect_class(&self, effect_class: EffectClass) -> Vec<ToolBinding>;

    /// Lists all bindings for a specific agent.
    fn list_by_agent(&self, agent: &AgentID) -> Vec<ToolBinding>;

    /// Lists all bindings for a specific tool.
    fn list_by_tool(&self, tool: &ToolID) -> Vec<ToolBinding>;

    /// Gets a specific tool binding by ID.
    fn get_binding(&self, binding_id: &str) -> Option<ToolBinding>;

    /// Checks if a binding exists.
    fn binding_exists(&self, binding_id: &str) -> bool {
        self.get_binding(binding_id).is_some()
    }

    /// Returns the total number of bindings in the registry.
    fn binding_count(&self) -> usize;
}

impl RegistryIntrospection for ToolRegistry {
    fn query(&self, query: ToolQuery) -> Vec<ToolBinding> {
        self.list_all()
            .into_iter()
            .filter(|b| query.matches(b))
            .collect()
    }

    fn list_by_effect_class(&self, effect_class: EffectClass) -> Vec<ToolBinding> {
        ToolRegistry::list_by_effect_class(self, effect_class)
    }

    fn list_by_agent(&self, agent: &AgentID) -> Vec<ToolBinding> {
        self.list_all()
            .into_iter()
            .filter(|b| b.agent == *agent)
            .collect()
    }

    fn list_by_tool(&self, tool: &ToolID) -> Vec<ToolBinding> {
        ToolRegistry::list_by_tool(self, tool)
    }

    fn get_binding(&self, binding_id: &str) -> Option<ToolBinding> {
        ToolRegistry::get_binding(self, binding_id)
    }

    fn binding_count(&self) -> usize {
        ToolRegistry::binding_count(self)
    }
}

/// Registry statistics and analysis.
///
/// Provides aggregated statistics about registry contents.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegistryStats {
    /// Total number of bindings
    pub total_bindings: usize,

    /// Number of ReadOnly bindings
    pub read_only_count: usize,

    /// Number of WriteReversible bindings
    pub write_reversible_count: usize,

    /// Number of WriteCompensable bindings
    pub write_compensable_count: usize,

    /// Number of WriteIrreversible bindings
    pub write_irreversible_count: usize,

    /// Number of bindings with commit protocol
    pub with_commit_protocol: usize,

    /// Number of unique agents
    pub unique_agents: usize,

    /// Number of unique tools
    pub unique_tools: usize,
}

impl RegistryStats {
    /// Computes statistics for a registry.
    ///
    /// # Arguments
    ///
    /// - `registry`: Registry to analyze
    ///
    /// # Returns
    ///
    /// Statistics about the registry contents
    pub fn from_registry(registry: &ToolRegistry) -> Self {
        let bindings = registry.list_all();

        let mut read_only_count = 0;
        let mut write_reversible_count = 0;
        let mut write_compensable_count = 0;
        let mut write_irreversible_count = 0;
        let mut with_commit_protocol = 0;

        let mut agents = alloc::collections::BTreeSet::new();
        let mut tools = alloc::collections::BTreeSet::new();

        for binding in &bindings {
            match binding.effect_class {
                EffectClass::ReadOnly => read_only_count += 1,
                EffectClass::WriteReversible => write_reversible_count += 1,
                EffectClass::WriteCompensable => write_compensable_count += 1,
                EffectClass::WriteIrreversible => write_irreversible_count += 1,
            }

            if binding.commit_protocol.is_some() {
                with_commit_protocol += 1;
            }

            agents.insert(binding.agent.clone());
            tools.insert(binding.tool.clone());
        }

        RegistryStats {
            total_bindings: bindings.len(),
            read_only_count,
            write_reversible_count,
            write_compensable_count,
            write_irreversible_count,
            with_commit_protocol,
            unique_agents: agents.len(),
            unique_tools: tools.len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::CapID;
    use crate::schema::{SchemaDefinition, TypeSchema};
    use crate::tool_binding::ToolBinding;
    use crate::ids::ToolBindingID;
use alloc::collections::BTreeSet;

    fn create_test_binding(id: &str, agent: &str, tool: &str, effect: EffectClass) -> ToolBinding {
        let input_schema = SchemaDefinition::new("TestInput");
        let output_schema = SchemaDefinition::new("TestOutput");
        let schema = TypeSchema::new(input_schema, output_schema);

        let capability_bytes = [42u8; 32];

        let mut binding = ToolBinding::new(
            ToolBindingID::new(id),
            ToolID::new(tool),
            AgentID::new(agent),
            CapID::from_bytes(capability_bytes),
            schema,
        );
        binding.effect_class = effect;
        binding
    }

    #[test]
    fn test_tool_query_new() {
        let query = ToolQuery::new();
        assert!(query.effect_class.is_none());
        assert!(query.agent.is_none());
        assert!(query.tool.is_none());
        assert!(!query.read_only_only);
    }

    #[test]
    fn test_tool_query_with_effect_class() {
        let query = ToolQuery::new().with_effect_class(EffectClass::ReadOnly);
        assert_eq!(query.effect_class, Some(EffectClass::ReadOnly));
    }

    #[test]
    fn test_tool_query_with_agent() {
        let agent = AgentID::new("agent-1");
        let query = ToolQuery::new().with_agent(agent.clone());
        assert_eq!(query.agent, Some(agent));
    }

    #[test]
    fn test_tool_query_matches_effect_class() {
        let binding = create_test_binding("b1", "agent-1", "tool-1", EffectClass::ReadOnly);
        let query = ToolQuery::new().with_effect_class(EffectClass::ReadOnly);

        assert!(query.matches(&binding));

        let query2 = ToolQuery::new().with_effect_class(EffectClass::WriteReversible);
        assert!(!query2.matches(&binding));
    }

    #[test]
    fn test_tool_query_matches_agent() {
        let binding = create_test_binding("b1", "agent-1", "tool-1", EffectClass::ReadOnly);
        let query = ToolQuery::new().with_agent(AgentID::new("agent-1"));

        assert!(query.matches(&binding));

        let query2 = ToolQuery::new().with_agent(AgentID::new("agent-2"));
        assert!(!query2.matches(&binding));
    }

    #[test]
    fn test_tool_query_matches_tool() {
        let binding = create_test_binding("b1", "agent-1", "tool-1", EffectClass::ReadOnly);
        let query = ToolQuery::new().with_tool(ToolID::new("tool-1"));

        assert!(query.matches(&binding));

        let query2 = ToolQuery::new().with_tool(ToolID::new("tool-2"));
        assert!(!query2.matches(&binding));
    }

    #[test]
    fn test_tool_query_matches_read_only_only() {
        let readonly_binding =
            create_test_binding("b1", "agent-1", "tool-1", EffectClass::ReadOnly);
        let write_binding =
            create_test_binding("b2", "agent-1", "tool-2", EffectClass::WriteReversible);

        let query = ToolQuery::new().read_only_only();

        assert!(query.matches(&readonly_binding));
        assert!(!query.matches(&write_binding));
    }

    #[test]
    fn test_tool_query_matches_multiple_criteria() {
        let binding = create_test_binding("b1", "agent-1", "tool-1", EffectClass::ReadOnly);

        let query = ToolQuery::new()
            .with_agent(AgentID::new("agent-1"))
            .with_effect_class(EffectClass::ReadOnly)
            .with_tool(ToolID::new("tool-1"));

        assert!(query.matches(&binding));

        let query2 = ToolQuery::new()
            .with_agent(AgentID::new("agent-2")) // Different agent
            .with_effect_class(EffectClass::ReadOnly);

        assert!(!query2.matches(&binding));
    }

    #[test]
    fn test_registry_introspection_query() {
        let mut registry = ToolRegistry::new();

        let b1 = create_test_binding("b1", "agent-1", "tool-1", EffectClass::ReadOnly);
        let b2 = create_test_binding("b2", "agent-1", "tool-2", EffectClass::WriteReversible);
        let b3 = create_test_binding("b3", "agent-2", "tool-1", EffectClass::ReadOnly);

        registry.register_tool(b1).unwrap();
        registry.register_tool(b2).unwrap();
        registry.register_tool(b3).unwrap();

        let query = ToolQuery::new().with_effect_class(EffectClass::ReadOnly);
        let results = registry.query(query);

        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_registry_introspection_list_by_agent() {
        let mut registry = ToolRegistry::new();

        let b1 = create_test_binding("b1", "agent-1", "tool-1", EffectClass::ReadOnly);
        let b2 = create_test_binding("b2", "agent-1", "tool-2", EffectClass::WriteReversible);
        let b3 = create_test_binding("b3", "agent-2", "tool-1", EffectClass::ReadOnly);

        registry.register_tool(b1).unwrap();
        registry.register_tool(b2).unwrap();
        registry.register_tool(b3).unwrap();

        let results = registry.list_by_agent(&AgentID::new("agent-1"));
        assert_eq!(results.len(), 2);

        let results2 = registry.list_by_agent(&AgentID::new("agent-2"));
        assert_eq!(results2.len(), 1);
    }

    #[test]
    fn test_registry_introspection_binding_exists() {
        let mut registry = ToolRegistry::new();

        let binding = create_test_binding("b1", "agent-1", "tool-1", EffectClass::ReadOnly);
        registry.register_tool(binding).unwrap();

        assert!(registry.binding_exists("b1"));
        assert!(!registry.binding_exists("b2"));
    }

    #[test]
    fn test_registry_stats() {
        let mut registry = ToolRegistry::new();

        let b1 = create_test_binding("b1", "agent-1", "tool-1", EffectClass::ReadOnly);
        let b2 = create_test_binding("b2", "agent-1", "tool-2", EffectClass::WriteReversible);
        let b3 = create_test_binding("b3", "agent-2", "tool-1", EffectClass::WriteCompensable);
        let b4 = create_test_binding("b4", "agent-2", "tool-3", EffectClass::WriteIrreversible);

        registry.register_tool(b1).unwrap();
        registry.register_tool(b2).unwrap();
        registry.register_tool(b3).unwrap();
        registry.register_tool(b4).unwrap();

        let stats = RegistryStats::from_registry(&registry);

        assert_eq!(stats.total_bindings, 4);
        assert_eq!(stats.read_only_count, 1);
        assert_eq!(stats.write_reversible_count, 1);
        assert_eq!(stats.write_compensable_count, 1);
        assert_eq!(stats.write_irreversible_count, 1);
        assert_eq!(stats.unique_agents, 2);
        assert_eq!(stats.unique_tools, 3);
    }

    #[test]
    fn test_registry_stats_empty() {
        let registry = ToolRegistry::new();
        let stats = RegistryStats::from_registry(&registry);

        assert_eq!(stats.total_bindings, 0);
        assert_eq!(stats.unique_agents, 0);
        assert_eq!(stats.unique_tools, 0);
    }
}
