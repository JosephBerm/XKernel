// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! Dependency ordering and agent crew membership.
//!
//! Defines dependency specifications, ordering constraints, dependency graph
//! analysis with cycle detection, and crew membership for agent organization.
//!
//! Reference: Engineering Plan § Agent Lifecycle Management § Dependencies

use std::collections::{BTreeMap, BTreeSet};
use crate::{LifecycleError, Result};

/// State of a dependency in the dependency graph.
///
/// Tracks the current state of an agent's dependencies during resolution.
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Dependencies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyState {
    /// Dependency not yet started.
    Pending,

    /// Dependency startup is in progress.
    Starting,

    /// Dependency is ready for dependent agents.
    Ready,

    /// Dependency failed to start.
    Failed,
}

/// Reference to an agent (typically ULID or name).
pub type AgentRef = String;

/// Ordering constraint for agent startup.
///
/// Specifies dependency relationships between agents to enforce startup ordering.
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Dependencies
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrderingConstraint {
    /// This agent must start before the specified agent.
    ///
    /// Use case: Data store must be up before services that use it.
    /// Example: Before("cache-service") means this agent starts first
    Before(AgentRef),

    /// This agent must start after the specified agent.
    ///
    /// Use case: Service depends on another service being available.
    /// Example: After("database") means this agent starts after database
    After(AgentRef),

    /// These agents can start concurrently with this agent.
    ///
    /// Use case: Establishing parallel startup groups.
    /// Example: Concurrent(vec!["agent_a", "agent_b"]) means can start together
    Concurrent(Vec<AgentRef>),
}

impl OrderingConstraint {
    /// Returns true if this is a Before constraint.
    pub fn is_before(&self) -> bool {
        matches!(self, Self::Before(_))
    }

    /// Returns true if this is an After constraint.
    pub fn is_after(&self) -> bool {
        matches!(self, Self::After(_))
    }

    /// Returns true if this is a Concurrent constraint.
    pub fn is_concurrent(&self) -> bool {
        matches!(self, Self::Concurrent(_))
    }
}

/// Dependency specification for an agent.
///
/// Declares all dependencies an agent has on other agents and services,
/// plus ordering constraints for startup sequencing.
///
/// # Fields
///
/// - `required_agents`: Agent IDs that must be available
/// - `required_services`: External service names that must be available
/// - `ordering_constraints`: Startup ordering constraints
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Dependencies
#[derive(Debug, Clone, Default)]
pub struct DependencySpec {
    /// Required agents that must be started before this agent.
    pub required_agents: Vec<AgentRef>,

    /// Required external services (not managed by lifecycle).
    pub required_services: Vec<String>,

    /// Ordering constraints for startup sequencing.
    pub ordering_constraints: Vec<OrderingConstraint>,
}

impl DependencySpec {
    /// Creates a new empty dependency specification.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a required agent dependency.
    pub fn with_required_agent(mut self, agent: impl Into<AgentRef>) -> Self {
        self.required_agents.push(agent.into());
        self
    }

    /// Adds a required service dependency.
    pub fn with_required_service(mut self, service: impl Into<String>) -> Self {
        self.required_services.push(service.into());
        self
    }

    /// Adds an ordering constraint.
    pub fn with_constraint(mut self, constraint: OrderingConstraint) -> Self {
        self.ordering_constraints.push(constraint);
        self
    }

    /// Adds a Before constraint (this agent before other).
    pub fn before(self, agent: impl Into<AgentRef>) -> Self {
        self.with_constraint(OrderingConstraint::Before(agent.into()))
    }

    /// Adds an After constraint (this agent after other).
    pub fn after(self, agent: impl Into<AgentRef>) -> Self {
        self.with_constraint(OrderingConstraint::After(agent.into()))
    }

    /// Returns all agent references mentioned in constraints.
    fn constraint_agents(&self) -> Vec<AgentRef> {
        let mut agents = Vec::new();
        for constraint in &self.ordering_constraints {
            match constraint {
                OrderingConstraint::Before(agent) => agents.push(agent.clone()),
                OrderingConstraint::After(agent) => agents.push(agent.clone()),
                OrderingConstraint::Concurrent(agent_list) => agents.extend(agent_list.clone()),
            }
        }
        agents
    }

    /// Returns all agent dependencies (both required and constrained).
    pub fn all_agent_dependencies(&self) -> Vec<AgentRef> {
        let mut all = self.required_agents.clone();
        all.extend(self.constraint_agents());
        all.sort();
        all.dedup();
        all
    }
}

/// Dependency graph for analyzing agent startup order.
///
/// Provides cycle detection and topological analysis of agent dependencies.
/// Used to verify that startup ordering is valid and compute optimal startup sequences.
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Dependencies
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    /// Map of agent to its dependencies.
    edges: BTreeMap<AgentRef, Vec<AgentRef>>,
}

impl DependencyGraph {
    /// Creates a new empty dependency graph.
    pub fn new() -> Self {
        Self {
            edges: BTreeMap::new(),
        }
    }

    /// Adds an agent and its dependencies to the graph.
    ///
    /// Arguments:
    /// - `agent`: The agent to add
    /// - `dependencies`: Agents that must start before this agent
    pub fn add_agent(&mut self, agent: AgentRef, dependencies: Vec<AgentRef>) {
        self.edges.insert(agent, dependencies);
    }

    /// Adds a dependency relationship to the graph.
    ///
    /// Indicates that `dependent` depends on `dependency`.
    pub fn add_dependency(&mut self, dependent: AgentRef, dependency: AgentRef) {
        self.edges
            .entry(dependent)
            .or_insert_with(Vec::new)
            .push(dependency);
    }

    /// Checks if the graph contains an agent.
    pub fn contains_agent(&self, agent: &AgentRef) -> bool {
        self.edges.contains_key(agent)
    }

    /// Gets the direct dependencies of an agent.
    pub fn get_dependencies(&self, agent: &AgentRef) -> Option<&[AgentRef]> {
        self.edges.get(agent).map(|v| v.as_slice())
    }

    /// Detects cycles in the dependency graph using DFS.
    ///
    /// Returns `Ok(())` if no cycle is found, or `Err(LifecycleError::DependencyCycle)`
    /// if a cycle is detected with the agents involved in the cycle.
    ///
    /// Uses depth-first search with three colors:
    /// - White: not yet visited
    /// - Gray: currently being processed
    /// - Black: finished processing
    ///
    /// Reference: Engineering Plan § Agent Lifecycle Management § Dependencies
    pub fn detect_cycle(&self) -> Result<()> {
        let mut visited = BTreeMap::new();
        let mut rec_stack = BTreeMap::new();
        let mut cycle = Vec::new();

        for agent in self.edges.keys() {
            if !visited.contains_key(agent) {
                self.dfs_cycle_detect(agent, &mut visited, &mut rec_stack, &mut cycle)?;
            }
        }

        Ok(())
    }

    fn dfs_cycle_detect(
        &self,
        agent: &AgentRef,
        visited: &mut BTreeMap<AgentRef, bool>,
        rec_stack: &mut BTreeMap<AgentRef, bool>,
        cycle: &mut Vec<AgentRef>,
    ) -> Result<()> {
        visited.insert(agent.clone(), true);
        rec_stack.insert(agent.clone(), true);

        if let Some(dependencies) = self.edges.get(agent) {
            for dep in dependencies {
                if let Some(true) = rec_stack.get(dep) {
                    // Found a cycle
                    cycle.push(dep.clone());
                    cycle.push(agent.clone());
                    return Err(LifecycleError::DependencyCycle {
                        agents: cycle.clone(),
                    });
                }

                if !visited.contains_key(dep) || !visited.get(dep).copied().unwrap_or(false) {
                    self.dfs_cycle_detect(dep, visited, rec_stack, cycle)?;
                }
            }
        }

        rec_stack.insert(agent.clone(), false);
        Ok(())
    }

    /// Computes a topological sort of agents (startup order).
    ///
    /// Returns agents in order such that all dependencies of an agent
    /// appear before the agent itself in the result.
    ///
    /// Returns error if cycle is detected.
    pub fn topological_sort(&self) -> Result<Vec<AgentRef>> {
        // First check for cycles
        self.detect_cycle()?;

        let mut visited = BTreeMap::new();
        let mut result = Vec::new();

        for agent in self.edges.keys() {
            if !visited.contains_key(agent) {
                self.dfs_topo(&agent, &mut visited, &mut result);
            }
        }

        result.reverse();
        Ok(result)
    }

    fn dfs_topo(&self, agent: &AgentRef, visited: &mut BTreeMap<AgentRef, bool>, result: &mut Vec<AgentRef>) {
        visited.insert(agent.clone(), true);

        if let Some(dependencies) = self.edges.get(agent) {
            for dep in dependencies {
                if !visited.contains_key(dep) || !visited.get(dep).copied().unwrap_or(false) {
                    self.dfs_topo(dep, visited, result);
                }
            }
        }

        result.push(agent.clone());
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Parallel startup groups for efficient agent initialization.
///
/// Groups agents that have no dependencies on each other and can be started
/// concurrently, optimizing startup time for systems with multiple independent agents.
///
/// # Example
///
/// For dependency graph: A depends on B, C and D have no dependencies
/// Result: [[B], [A, C, D]]
/// - Group 0: Start B
/// - Group 1: Start A, C, D in parallel
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Dependencies
#[derive(Debug, Clone)]
pub struct ParallelStartGroups {
    /// List of agent groups where agents within each group can start in parallel.
    ///
    /// Each inner Vec contains agents with no inter-dependencies that can start together.
    /// Groups are ordered such that all dependencies of a group appear in earlier groups.
    pub groups: Vec<Vec<AgentRef>>,
}

impl ParallelStartGroups {
    /// Creates a new empty parallel start groups.
    pub fn new() -> Self {
        Self {
            groups: Vec::new(),
        }
    }

    /// Adds a group of agents that can start in parallel.
    pub fn add_group(&mut self, group: Vec<AgentRef>) {
        self.groups.push(group);
    }

    /// Returns the number of parallel groups.
    pub fn group_count(&self) -> usize {
        self.groups.len()
    }

    /// Returns an iterator over the groups.
    pub fn iter_groups(&self) -> impl Iterator<Item = &Vec<AgentRef>> {
        self.groups.iter()
    }

    /// Flattens all agents in all groups into a single Vec.
    pub fn all_agents(&self) -> Vec<AgentRef> {
        let mut all = Vec::new();
        for group in &self.groups {
            all.extend(group.clone());
        }
        all
    }

    /// Computes parallel startup groups from a dependency graph.
    ///
    /// Performs topological sort and groups agents by their distance from leaf nodes.
    /// Agents at the same distance can start in parallel.
    ///
    /// Arguments:
    /// - `graph`: The dependency graph to analyze
    ///
    /// Returns `Ok(ParallelStartGroups)` with properly ordered groups, or error if cycle detected.
    ///
    /// Reference: Engineering Plan § Agent Lifecycle Management § Dependencies
    pub fn from_dependency_graph(graph: &DependencyGraph) -> Result<Self> {
        // First verify no cycles exist
        graph.detect_cycle()?;

        // Compute depth of each agent (distance from leaf nodes)
        let mut depths = BTreeMap::new();
        for agent in graph.edges.keys() {
            if !depths.contains_key(agent) {
                Self::compute_depth(agent, graph, &mut depths);
            }
        }

        // Group agents by depth
        let mut groups_by_depth = BTreeMap::new();
        for (agent, depth) in depths {
            groups_by_depth
                .entry(depth)
                .or_insert_with(Vec::new)
                .push(agent);
        }

        // Create groups ordered by depth
        let mut groups = Vec::new();
        for depth in 0..groups_by_depth.len() {
            if let Some(mut group) = groups_by_depth.remove(&(depth as u32)) {
                group.sort(); // For deterministic ordering
                groups.push(group);
            }
        }

        Ok(ParallelStartGroups { groups })
    }

    fn compute_depth(
        agent: &AgentRef,
        graph: &DependencyGraph,
        depths: &mut BTreeMap<AgentRef, u32>,
    ) -> u32 {
        if let Some(&depth) = depths.get(agent) {
            return depth;
        }

        let max_dep_depth = if let Some(deps) = graph.get_dependencies(agent) {
            deps.iter()
                .map(|dep| Self::compute_depth(dep, graph, depths))
                .max()
                .unwrap_or(0)
        } else {
            0
        };

        let depth = max_dep_depth + 1;
        depths.insert(agent.clone(), depth);
        depth
    }
}

impl Default for ParallelStartGroups {
    fn default() -> Self {
        Self::new()
    }
}

/// Dependency resolver context for crew-based dependency resolution.
///
/// Manages dependency resolution within a crew (group of related agents).
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Dependencies
#[derive(Debug, Clone)]
pub struct CrewDependencyContext {
    /// Crew identifier.
    pub crew_id: String,

    /// Dependency graph for agents in this crew.
    pub dependency_graph: DependencyGraph,

    /// Current state of each dependency.
    pub dependency_states: BTreeMap<AgentRef, DependencyState>,
}

impl CrewDependencyContext {
    /// Creates a new crew dependency context.
    pub fn new(crew_id: impl Into<String>) -> Self {
        Self {
            crew_id: crew_id.into(),
            dependency_graph: DependencyGraph::new(),
            dependency_states: BTreeMap::new(),
        }
    }

    /// Adds an agent to the context with its dependencies.
    pub fn add_agent(&mut self, agent: AgentRef, dependencies: Vec<AgentRef>) {
        self.dependency_graph.add_agent(agent.clone(), dependencies);
        self.dependency_states.insert(agent, DependencyState::Pending);
    }

    /// Updates the state of a dependency.
    pub fn set_dependency_state(&mut self, agent: &AgentRef, state: DependencyState) {
        self.dependency_states.insert(agent.clone(), state);
    }

    /// Gets the state of a dependency.
    pub fn get_dependency_state(&self, agent: &AgentRef) -> Option<DependencyState> {
        self.dependency_states.get(agent).copied()
    }

    /// Checks if all dependencies of an agent are ready.
    pub fn dependencies_ready(&self, agent: &AgentRef) -> bool {
        if let Some(deps) = self.dependency_graph.get_dependencies(agent) {
            deps.iter().all(|dep| {
                self.dependency_states
                    .get(dep)
                    .copied()
                    .map(|state| state == DependencyState::Ready)
                    .unwrap_or(false)
            })
        } else {
            true
        }
    }

    /// Computes the startup order for agents in this crew.
    pub fn compute_startup_order(&self) -> Result<Vec<AgentRef>> {
        self.dependency_graph.topological_sort()
    }

    /// Computes parallel startup groups for this crew.
    pub fn compute_parallel_groups(&self) -> Result<ParallelStartGroups> {
        ParallelStartGroups::from_dependency_graph(&self.dependency_graph)
    }
}

/// Crew membership information for an agent.
///
/// Describes an agent's membership in a crew (group of coordinated agents)
/// along with its role within that crew.
///
/// # Fields
///
/// - `crew_id`: Identifier of the crew this agent belongs to
/// - `agent_id`: Identifier of this agent
/// - `role`: Role or function of this agent within the crew
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Crews
#[derive(Debug, Clone)]
pub struct CrewMembership {
    /// Crew identifier that this agent belongs to.
    pub crew_id: String,

    /// Agent identifier.
    pub agent_id: AgentRef,

    /// Role or function within the crew (e.g., "leader", "worker", "monitor").
    pub role: String,
}

impl CrewMembership {
    /// Creates a new crew membership.
    pub fn new(crew_id: impl Into<String>, agent_id: impl Into<AgentRef>, role: impl Into<String>) -> Self {
        Self {
            crew_id: crew_id.into(),
            agent_id: agent_id.into(),
            role: role.into(),
        }
    }

    /// Returns true if this agent is a crew leader.
    pub fn is_leader(&self) -> bool {
        self.role.eq_ignore_ascii_case("leader")
    }

    /// Returns true if this agent is a crew worker.
    pub fn is_worker(&self) -> bool {
        self.role.eq_ignore_ascii_case("worker")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use std::collections::BTreeMap;

    #[test]
    fn test_ordering_constraint_before() {
        let constraint = OrderingConstraint::Before("other_agent".to_string());
        assert!(constraint.is_before());
        assert!(!constraint.is_after());
        assert!(!constraint.is_concurrent());
    }

    #[test]
    fn test_ordering_constraint_after() {
        let constraint = OrderingConstraint::After("dependency".to_string());
        assert!(!constraint.is_before());
        assert!(constraint.is_after());
        assert!(!constraint.is_concurrent());
    }

    #[test]
    fn test_ordering_constraint_concurrent() {
        let agents = vec!["a".to_string(), "b".to_string()];
        let constraint = OrderingConstraint::Concurrent(agents);
        assert!(!constraint.is_before());
        assert!(!constraint.is_after());
        assert!(constraint.is_concurrent());
    }

    #[test]
    fn test_dependency_spec_new() {
        let spec = DependencySpec::new();
        assert!(spec.required_agents.is_empty());
        assert!(spec.required_services.is_empty());
        assert!(spec.ordering_constraints.is_empty());
    }

    #[test]
    fn test_dependency_spec_builder() {
        let spec = DependencySpec::new()
            .with_required_agent("db")
            .with_required_agent("cache")
            .with_required_service("ssl")
            .after("db");

        assert_eq!(spec.required_agents.len(), 2);
        assert_eq!(spec.required_services.len(), 1);
        assert_eq!(spec.ordering_constraints.len(), 1);
    }

    #[test]
    fn test_dependency_spec_all_agent_dependencies() {
        let spec = DependencySpec::new()
            .with_required_agent("db")
            .after("cache");

        let deps = spec.all_agent_dependencies();
        assert!(deps.contains(&"db".to_string()));
        assert!(deps.contains(&"cache".to_string()));
    }

    #[test]
    fn test_dependency_graph_add_agent() {
        let mut graph = DependencyGraph::new();
        graph.add_agent("app".to_string(), vec!["db".to_string()]);
        assert!(graph.contains_agent(&"app".to_string()));
    }

    #[test]
    fn test_dependency_graph_no_cycle() {
        let mut graph = DependencyGraph::new();
        graph.add_agent("app".to_string(), vec!["db".to_string()]);
        graph.add_agent("db".to_string(), vec![]);
        assert!(graph.detect_cycle().is_ok());
    }

    #[test]
    fn test_dependency_graph_simple_cycle() {
        let mut graph = DependencyGraph::new();
        graph.add_agent("a".to_string(), vec!["b".to_string()]);
        graph.add_agent("b".to_string(), vec!["a".to_string()]);
        assert!(graph.detect_cycle().is_err());
    }

    #[test]
    fn test_dependency_graph_self_cycle() {
        let mut graph = DependencyGraph::new();
        graph.add_agent("a".to_string(), vec!["a".to_string()]);
        assert!(graph.detect_cycle().is_err());
    }

    #[test]
    fn test_dependency_graph_topological_sort() {
        let mut graph = DependencyGraph::new();
        graph.add_agent("app".to_string(), vec!["db".to_string()]);
        graph.add_agent("db".to_string(), vec![]);
        graph.add_agent("cache".to_string(), vec!["db".to_string()]);

        let result = graph.topological_sort().expect("should succeed");
        let db_idx = result.iter().position(|a| a == "db").unwrap();
        let app_idx = result.iter().position(|a| a == "app").unwrap();
        let cache_idx = result.iter().position(|a| a == "cache").unwrap();

        assert!(db_idx < app_idx);
        assert!(db_idx < cache_idx);
    }

    #[test]
    fn test_dependency_graph_topological_sort_with_cycle() {
        let mut graph = DependencyGraph::new();
        graph.add_agent("a".to_string(), vec!["b".to_string()]);
        graph.add_agent("b".to_string(), vec!["a".to_string()]);
        assert!(graph.topological_sort().is_err());
    }

    #[test]
    fn test_crew_membership_new() {
        let membership = CrewMembership::new("crew1", "agent1", "leader");
        assert_eq!(membership.crew_id, "crew1");
        assert_eq!(membership.agent_id, "agent1");
        assert_eq!(membership.role, "leader");
    }

    #[test]
    fn test_crew_membership_is_leader() {
        let membership = CrewMembership::new("crew1", "agent1", "leader");
        assert!(membership.is_leader());
        assert!(!membership.is_worker());
    }

    #[test]
    fn test_crew_membership_is_worker() {
        let membership = CrewMembership::new("crew1", "agent1", "worker");
        assert!(!membership.is_leader());
        assert!(membership.is_worker());
    }

    #[test]
    fn test_crew_membership_case_insensitive_role() {
        let membership = CrewMembership::new("crew1", "agent1", "LEADER");
        assert!(membership.is_leader());
    }

    // DependencyState tests
    #[test]
    fn test_dependency_state_pending() {
        let state = DependencyState::Pending;
        assert_eq!(state, DependencyState::Pending);
    }

    #[test]
    fn test_dependency_state_starting() {
        let state = DependencyState::Starting;
        assert_eq!(state, DependencyState::Starting);
    }

    #[test]
    fn test_dependency_state_ready() {
        let state = DependencyState::Ready;
        assert_eq!(state, DependencyState::Ready);
    }

    #[test]
    fn test_dependency_state_failed() {
        let state = DependencyState::Failed;
        assert_eq!(state, DependencyState::Failed);
    }

    // ParallelStartGroups tests
    #[test]
    fn test_parallel_start_groups_new() {
        let groups = ParallelStartGroups::new();
        assert_eq!(groups.group_count(), 0);
    }

    #[test]
    fn test_parallel_start_groups_add_group() {
        let mut groups = ParallelStartGroups::new();
        groups.add_group(vec!["agent1".to_string(), "agent2".to_string()]);
        assert_eq!(groups.group_count(), 1);
    }

    #[test]
    fn test_parallel_start_groups_all_agents() {
        let mut groups = ParallelStartGroups::new();
        groups.add_group(vec!["a".to_string(), "b".to_string()]);
        groups.add_group(vec!["c".to_string()]);

        let all = groups.all_agents();
        assert_eq!(all.len(), 3);
        assert!(all.contains(&"a".to_string()));
        assert!(all.contains(&"b".to_string()));
        assert!(all.contains(&"c".to_string()));
    }

    #[test]
    fn test_parallel_start_groups_from_linear_dependency() {
        let mut graph = DependencyGraph::new();
        // Linear chain: db <- cache <- app
        graph.add_agent("db".to_string(), vec![]);
        graph.add_agent("cache".to_string(), vec!["db".to_string()]);
        graph.add_agent("app".to_string(), vec!["cache".to_string()]);

        let groups = ParallelStartGroups::from_dependency_graph(&graph)
            .expect("should compute groups");

        assert_eq!(groups.group_count(), 3);
        // Each level forms its own group
        assert!(groups.groups[0].contains(&"db".to_string()));
        assert!(groups.groups[1].contains(&"cache".to_string()));
        assert!(groups.groups[2].contains(&"app".to_string()));
    }

    #[test]
    fn test_parallel_start_groups_from_parallel_dependencies() {
        let mut graph = DependencyGraph::new();
        // Both cache and worker depend on db; cache and worker can start in parallel
        graph.add_agent("db".to_string(), vec![]);
        graph.add_agent("cache".to_string(), vec!["db".to_string()]);
        graph.add_agent("worker".to_string(), vec!["db".to_string()]);

        let groups = ParallelStartGroups::from_dependency_graph(&graph)
            .expect("should compute groups");

        assert_eq!(groups.group_count(), 2);
        assert!(groups.groups[0].contains(&"db".to_string()));
        // cache and worker should be in same group
        assert_eq!(groups.groups[1].len(), 2);
        assert!(groups.groups[1].contains(&"cache".to_string()));
        assert!(groups.groups[1].contains(&"worker".to_string()));
    }

    #[test]
    fn test_parallel_start_groups_from_complex_dependency() {
        let mut graph = DependencyGraph::new();
        // Complex: db -> {cache, queue}, app depends on both cache and queue
        graph.add_agent("db".to_string(), vec![]);
        graph.add_agent("cache".to_string(), vec!["db".to_string()]);
        graph.add_agent("queue".to_string(), vec!["db".to_string()]);
        graph.add_agent("app".to_string(), vec!["cache".to_string(), "queue".to_string()]);

        let groups = ParallelStartGroups::from_dependency_graph(&graph)
            .expect("should compute groups");

        // db -> [cache, queue] -> [app]
        assert_eq!(groups.group_count(), 3);
        assert!(groups.groups[0].contains(&"db".to_string()));
        assert!(groups.groups[1].contains(&"cache".to_string()));
        assert!(groups.groups[1].contains(&"queue".to_string()));
        assert!(groups.groups[2].contains(&"app".to_string()));
    }

    #[test]
    fn test_parallel_start_groups_detects_cycles() {
        let mut graph = DependencyGraph::new();
        graph.add_agent("a".to_string(), vec!["b".to_string()]);
        graph.add_agent("b".to_string(), vec!["a".to_string()]);

        let result = ParallelStartGroups::from_dependency_graph(&graph);
        assert!(result.is_err());
    }

    // CrewDependencyContext tests
    #[test]
    fn test_crew_dependency_context_new() {
        let context = CrewDependencyContext::new("crew1");
        assert_eq!(context.crew_id, "crew1");
        assert_eq!(context.dependency_states.len(), 0);
    }

    #[test]
    fn test_crew_dependency_context_add_agent() {
        let mut context = CrewDependencyContext::new("crew1");
        context.add_agent("app".to_string(), vec!["db".to_string()]);

        assert!(context.dependency_graph.contains_agent(&"app".to_string()));
        assert_eq!(
            context.get_dependency_state(&"app".to_string()),
            Some(DependencyState::Pending)
        );
    }

    #[test]
    fn test_crew_dependency_context_set_dependency_state() {
        let mut context = CrewDependencyContext::new("crew1");
        context.add_agent("db".to_string(), vec![]);
        context.set_dependency_state(&"db".to_string(), DependencyState::Ready);

        assert_eq!(
            context.get_dependency_state(&"db".to_string()),
            Some(DependencyState::Ready)
        );
    }

    #[test]
    fn test_crew_dependency_context_dependencies_ready() {
        let mut context = CrewDependencyContext::new("crew1");
        context.add_agent("db".to_string(), vec![]);
        context.add_agent("app".to_string(), vec!["db".to_string()]);

        // db not ready yet
        assert!(!context.dependencies_ready(&"app".to_string()));

        // Mark db as ready
        context.set_dependency_state(&"db".to_string(), DependencyState::Ready);
        assert!(context.dependencies_ready(&"app".to_string()));
    }

    #[test]
    fn test_crew_dependency_context_compute_startup_order() {
        let mut context = CrewDependencyContext::new("crew1");
        context.add_agent("db".to_string(), vec![]);
        context.add_agent("cache".to_string(), vec!["db".to_string()]);
        context.add_agent("app".to_string(), vec!["cache".to_string()]);

        let order = context
            .compute_startup_order()
            .expect("should compute order");

        let db_idx = order.iter().position(|a| a == "db").unwrap();
        let cache_idx = order.iter().position(|a| a == "cache").unwrap();
        let app_idx = order.iter().position(|a| a == "app").unwrap();

        assert!(db_idx < cache_idx);
        assert!(cache_idx < app_idx);
    }

    #[test]
    fn test_crew_dependency_context_compute_parallel_groups() {
        let mut context = CrewDependencyContext::new("crew1");
        context.add_agent("db".to_string(), vec![]);
        context.add_agent("cache".to_string(), vec!["db".to_string()]);
        context.add_agent("worker".to_string(), vec!["db".to_string()]);

        let groups = context
            .compute_parallel_groups()
            .expect("should compute groups");

        assert_eq!(groups.group_count(), 2);
    }

    #[test]
    fn test_crew_dependency_context_no_dependencies() {
        let mut context = CrewDependencyContext::new("crew1");
        context.add_agent("standalone".to_string(), vec![]);

        assert!(context.dependencies_ready(&"standalone".to_string()));
    }

    #[test]
    fn test_crew_dependency_context_multiple_dependencies() {
        let mut context = CrewDependencyContext::new("crew1");
        context.add_agent("db".to_string(), vec![]);
        context.add_agent("cache".to_string(), vec![]);
        context.add_agent("app".to_string(), vec!["db".to_string(), "cache".to_string()]);

        // Neither ready
        assert!(!context.dependencies_ready(&"app".to_string()));

        // Only one ready
        context.set_dependency_state(&"db".to_string(), DependencyState::Ready);
        assert!(!context.dependencies_ready(&"app".to_string()));

        // Both ready
        context.set_dependency_state(&"cache".to_string(), DependencyState::Ready);
        assert!(context.dependencies_ready(&"app".to_string()));
    }
}
