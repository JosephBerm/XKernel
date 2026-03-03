// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.

//! # Chain-to-CT DAG Translation
//!
//! Translates framework-specific chain definitions into Cognitive Task Directed Acyclic Graphs (CT DAGs).
//! Supports multiple chain patterns: sequential, conditional routing, and map-reduce parallelism.
//!
//! Each chain type is translated into an equivalent DAG representation that the kernel can execute,
//! with explicit node dependencies, data flow edges, and conditional branch semantics.
//!
//! Sec 4.2: Chain-to-DAG Translation
//! Sec 4.3: Chain Pattern Mapping

use alloc::{string::String, vec::Vec, collections::BTreeMap};
use crate::{AdapterError, framework_type::FrameworkType};

/// Unique identifier for a Cognitive Task
pub type CTID = u64;

/// Chain type classification for translation strategy selection.
/// Sec 4.2: Chain Type Enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChainType {
    /// Linear sequential execution: step1 -> step2 -> step3
    Sequential,
    /// Conditional branching: if/else decision points
    Router,
    /// Parallel fan-out with single merge: fork -> [parallel tasks] -> join
    MapReduce,
    /// Custom or hybrid chain pattern
    Custom,
}

impl ChainType {
    /// Returns string representation of the chain type.
    pub fn as_str(&self) -> &'static str {
        match self {
            ChainType::Sequential => "sequential",
            ChainType::Router => "router",
            ChainType::MapReduce => "map_reduce",
            ChainType::Custom => "custom",
        }
    }
}

/// Input/output mapping specification for a chain step.
/// Sec 4.2: Data Flow Mapping
#[derive(Debug, Clone)]
pub struct DataMapping {
    /// Source field or variable name
    pub source: String,
    /// Target field or variable name
    pub target: String,
    /// Optional transformation or projection
    pub transform: Option<String>,
}

impl DataMapping {
    /// Creates a new data mapping.
    pub fn new(source: String, target: String) -> Self {
        DataMapping {
            source,
            target,
            transform: None,
        }
    }

    /// Sets the transformation specification.
    pub fn with_transform(mut self, transform: String) -> Self {
        self.transform = Some(transform);
        self
    }
}

/// Single step in a chain execution.
/// Sec 4.2: Chain Step Definition
#[derive(Debug, Clone)]
pub struct ChainStep {
    /// Step identifier within the chain
    pub name: String,
    /// Tool or capability binding
    pub tool_binding: String,
    /// Input data mappings
    pub input_mapping: Vec<DataMapping>,
    /// Output data mappings
    pub output_mapping: Vec<DataMapping>,
    /// Optional conditional predicate for execution
    pub condition: Option<String>,
    /// Optional step description
    pub description: Option<String>,
}

impl ChainStep {
    /// Creates a new chain step.
    pub fn new(name: String, tool_binding: String) -> Self {
        ChainStep {
            name,
            tool_binding,
            input_mapping: Vec::new(),
            output_mapping: Vec::new(),
            condition: None,
            description: None,
        }
    }

    /// Adds an input mapping.
    pub fn add_input_mapping(&mut self, mapping: DataMapping) {
        self.input_mapping.push(mapping);
    }

    /// Adds an output mapping.
    pub fn add_output_mapping(&mut self, mapping: DataMapping) {
        self.output_mapping.push(mapping);
    }

    /// Sets the conditional predicate.
    pub fn set_condition(&mut self, condition: String) {
        self.condition = Some(condition);
    }

    /// Sets the description.
    pub fn set_description(&mut self, description: String) {
        self.description = Some(description);
    }
}

/// Framework-specific chain definition to be translated.
/// Sec 4.2: Chain Definition Structure
#[derive(Debug, Clone)]
pub struct ChainDefinition {
    /// Chain type classification
    pub chain_type: ChainType,
    /// Ordered list of execution steps
    pub steps: Vec<ChainStep>,
    /// Input schema specification (JSON schema format)
    pub input_schema: String,
    /// Output schema specification (JSON schema format)
    pub output_schema: String,
    /// Optional chain description
    pub description: Option<String>,
    /// Optional timeout in milliseconds
    pub timeout_ms: Option<u64>,
}

impl ChainDefinition {
    /// Creates a new chain definition.
    pub fn new(chain_type: ChainType, input_schema: String, output_schema: String) -> Self {
        ChainDefinition {
            chain_type,
            steps: Vec::new(),
            input_schema,
            output_schema,
            description: None,
            timeout_ms: None,
        }
    }

    /// Adds a step to the chain.
    pub fn add_step(&mut self, step: ChainStep) {
        self.steps.push(step);
    }

    /// Sets the description.
    pub fn set_description(&mut self, description: String) {
        self.description = Some(description);
    }

    /// Sets the timeout in milliseconds.
    pub fn set_timeout_ms(&mut self, timeout_ms: u64) {
        self.timeout_ms = Some(timeout_ms);
    }
}

/// Directed Acyclic Graph edge type classification.
/// Sec 4.2: DAG Edge Types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EdgeType {
    /// Direct dependency: target depends on source completion
    Dependency,
    /// Data flow: output of source feeds input to target
    DataFlow,
    /// Conditional: execution depends on branch condition
    Conditional,
}

impl EdgeType {
    /// Returns string representation of the edge type.
    pub fn as_str(&self) -> &'static str {
        match self {
            EdgeType::Dependency => "dependency",
            EdgeType::DataFlow => "data_flow",
            EdgeType::Conditional => "conditional",
        }
    }
}

/// DAG edge connecting two nodes.
/// Sec 4.2: DAG Edge Definition
#[derive(Debug, Clone)]
pub struct DagEdge {
    /// Source node identifier
    pub from: CTID,
    /// Target node identifier
    pub to: CTID,
    /// Edge type classification
    pub edge_type: EdgeType,
    /// Optional edge label or metadata
    pub label: Option<String>,
}

impl DagEdge {
    /// Creates a new DAG edge.
    pub fn new(from: CTID, to: CTID, edge_type: EdgeType) -> Self {
        DagEdge {
            from,
            to,
            edge_type,
            label: None,
        }
    }

    /// Sets the edge label.
    pub fn with_label(mut self, label: String) -> Self {
        self.label = Some(label);
        self
    }
}

/// CT node configuration for DAG execution.
/// Sec 4.2: CT Node Structure
#[derive(Debug, Clone)]
pub struct CtNode {
    /// Node identifier (CT ID)
    pub ct_id: CTID,
    /// Node name
    pub name: String,
    /// CT configuration JSON
    pub ct_config: String,
    /// List of dependent CT IDs
    pub dependencies: Vec<CTID>,
    /// Tool binding configuration
    pub tool_bindings: Vec<String>,
    /// Memory reference identifiers
    pub memory_refs: Vec<String>,
}

impl CtNode {
    /// Creates a new CT node.
    pub fn new(ct_id: CTID, name: String, ct_config: String) -> Self {
        CtNode {
            ct_id,
            name,
            ct_config,
            dependencies: Vec::new(),
            tool_bindings: Vec::new(),
            memory_refs: Vec::new(),
        }
    }

    /// Adds a dependency.
    pub fn add_dependency(&mut self, ct_id: CTID) {
        if !self.dependencies.contains(&ct_id) {
            self.dependencies.push(ct_id);
        }
    }

    /// Adds a tool binding.
    pub fn add_tool_binding(&mut self, binding: String) {
        self.tool_bindings.push(binding);
    }

    /// Adds a memory reference.
    pub fn add_memory_ref(&mut self, memory_ref: String) {
        self.memory_refs.push(memory_ref);
    }
}

/// Cognitive Task Directed Acyclic Graph (CT DAG).
/// Sec 4.2: CT DAG Structure
#[derive(Debug, Clone)]
pub struct CtDag {
    /// Mapping of CT IDs to nodes
    pub nodes: BTreeMap<CTID, CtNode>,
    /// List of edges defining dependencies and data flow
    pub edges: Vec<DagEdge>,
    /// Root CT ID (entry point)
    pub root_ct: CTID,
    /// Optional DAG description
    pub description: Option<String>,
}

impl CtDag {
    /// Creates a new CT DAG.
    pub fn new(root_ct: CTID) -> Self {
        CtDag {
            nodes: BTreeMap::new(),
            edges: Vec::new(),
            root_ct,
            description: None,
        }
    }

    /// Adds a node to the DAG.
    pub fn add_node(&mut self, node: CtNode) {
        self.nodes.insert(node.ct_id, node);
    }

    /// Adds an edge to the DAG.
    pub fn add_edge(&mut self, edge: DagEdge) {
        self.edges.push(edge);
    }

    /// Sets the description.
    pub fn set_description(&mut self, description: String) {
        self.description = Some(description);
    }

    /// Returns the node with the given ID.
    pub fn get_node(&self, ct_id: CTID) -> Option<&CtNode> {
        self.nodes.get(&ct_id)
    }

    /// Returns the number of nodes in the DAG.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Returns the number of edges in the DAG.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
}

/// Translates a sequential chain to a linear DAG.
/// Sec 4.2: Sequential Chain Translation
pub fn translate_sequential(chain: &ChainDefinition) -> Result<CtDag, AdapterError> {
    if chain.steps.is_empty() {
        return Err(AdapterError::TranslationError(
            "Sequential chain must have at least one step".into(),
        ));
    }

    let mut dag = CtDag::new(1);

    let mut prev_ct_id: Option<CTID> = None;

    for (idx, step) in chain.steps.iter().enumerate() {
        let ct_id = (idx + 1) as CTID;
        let node = CtNode::new(
            ct_id,
            step.name.clone(),
            format!(r#"{{"tool_binding": "{}"}}"#, step.tool_binding),
        );
        dag.add_node(node);

        if let Some(prev_id) = prev_ct_id {
            dag.add_edge(DagEdge::new(prev_id, ct_id, EdgeType::Dependency));
            dag.add_edge(DagEdge::new(prev_id, ct_id, EdgeType::DataFlow));
        }

        prev_ct_id = Some(ct_id);
    }

    Ok(dag)
}

/// Translates a router (conditional) chain to a branching DAG.
/// Sec 4.2: Router Chain Translation
pub fn translate_router(chain: &ChainDefinition) -> Result<CtDag, AdapterError> {
    if chain.steps.is_empty() {
        return Err(AdapterError::TranslationError(
            "Router chain must have at least one step".into(),
        ));
    }

    let mut dag = CtDag::new(1);

    for (idx, step) in chain.steps.iter().enumerate() {
        let ct_id = (idx + 1) as CTID;
        let node = CtNode::new(
            ct_id,
            step.name.clone(),
            format!(r#"{{"tool_binding": "{}"}}"#, step.tool_binding),
        );
        dag.add_node(node);

        if let Some(condition) = &step.condition {
            if idx > 0 {
                let prev_id = idx as CTID;
                let edge = DagEdge::new(prev_id, ct_id, EdgeType::Conditional)
                    .with_label(condition.clone());
                dag.add_edge(edge);
            }
        } else if idx > 0 {
            let prev_id = idx as CTID;
            dag.add_edge(DagEdge::new(prev_id, ct_id, EdgeType::Dependency));
        }
    }

    Ok(dag)
}

/// Translates a map-reduce chain to a fan-out/fan-in DAG.
/// Sec 4.2: Map-Reduce Chain Translation
pub fn translate_map_reduce(chain: &ChainDefinition) -> Result<CtDag, AdapterError> {
    if chain.steps.len() < 3 {
        return Err(AdapterError::TranslationError(
            "Map-reduce chain requires at least mapper, multiple workers, and reducer".into(),
        ));
    }

    let mut dag = CtDag::new(1);

    // Add mapper node (step 0)
    let mapper_id = 1u64;
    let mapper = CtNode::new(
        mapper_id,
        chain.steps[0].name.clone(),
        format!(r#"{{"tool_binding": "{}"}}"#, chain.steps[0].tool_binding),
    );
    dag.add_node(mapper);

    // Add worker nodes (steps 1..n-1)
    let mut worker_ids = Vec::new();
    for (idx, step) in chain.steps.iter().enumerate().skip(1).take(chain.steps.len() - 2) {
        let worker_id = (idx + 1) as CTID;
        let worker = CtNode::new(
            worker_id,
            step.name.clone(),
            format!(r#"{{"tool_binding": "{}"}}"#, step.tool_binding),
        );
        dag.add_node(worker);
        worker_ids.push(worker_id);

        // Connect mapper to each worker
        dag.add_edge(DagEdge::new(mapper_id, worker_id, EdgeType::DataFlow));
    }

    // Add reducer node (last step)
    let reducer_id = chain.steps.len() as CTID;
    let reducer = CtNode::new(
        reducer_id,
        chain.steps[chain.steps.len() - 1].name.clone(),
        format!(
            r#"{{"tool_binding": "{}"}}"#,
            chain.steps[chain.steps.len() - 1].tool_binding
        ),
    );
    dag.add_node(reducer);

    // Connect all workers to reducer
    for worker_id in worker_ids {
        dag.add_edge(DagEdge::new(worker_id, reducer_id, EdgeType::DataFlow));
    }

    Ok(dag)
}

/// Validates the DAG for structural correctness.
/// Sec 4.2: DAG Validation
pub fn validate_dag(dag: &CtDag) -> Result<(), AdapterError> {
    // Check for cycles (since it's supposed to be a DAG)
    if has_cycle(dag) {
        return Err(AdapterError::TranslationError(
            "DAG contains cycle: not a valid acyclic graph".into(),
        ));
    }

    // Check connectivity: all nodes should be reachable from root or have a path
    if !is_connected(dag) {
        return Err(AdapterError::TranslationError(
            "DAG contains disconnected subgraph".into(),
        ));
    }

    // Check that all edges reference valid nodes
    for edge in &dag.edges {
        if !dag.nodes.contains_key(&edge.from) {
            return Err(AdapterError::TranslationError(
                format!("Edge references non-existent source node: {}", edge.from),
            ));
        }
        if !dag.nodes.contains_key(&edge.to) {
            return Err(AdapterError::TranslationError(
                format!("Edge references non-existent target node: {}", edge.to),
            ));
        }
    }

    Ok(())
}

/// Checks if the DAG contains a cycle using DFS.
fn has_cycle(dag: &CtDag) -> bool {
    let mut visited = BTreeMap::new();
    let mut rec_stack = BTreeMap::new();

    for ct_id in dag.nodes.keys() {
        if !visited.contains_key(ct_id) {
            if has_cycle_dfs(dag, *ct_id, &mut visited, &mut rec_stack) {
                return true;
            }
        }
    }

    false
}

/// DFS helper for cycle detection.
fn has_cycle_dfs(
    dag: &CtDag,
    node: CTID,
    visited: &mut BTreeMap<CTID, bool>,
    rec_stack: &mut BTreeMap<CTID, bool>,
) -> bool {
    visited.insert(node, true);
    rec_stack.insert(node, true);

    for edge in &dag.edges {
        if edge.from == node {
            let next = edge.to;
            if !visited.contains_key(&next) {
                if has_cycle_dfs(dag, next, visited, rec_stack) {
                    return true;
                }
            } else if rec_stack.contains_key(&next) && rec_stack[&next] {
                return true;
            }
        }
    }

    rec_stack.insert(node, false);
    false
}

/// Checks if the DAG is connected (all nodes reachable from root).
fn is_connected(dag: &CtDag) -> bool {
    if dag.nodes.is_empty() {
        return true;
    }

    let mut visited = BTreeMap::new();
    dfs_visit(dag, dag.root_ct, &mut visited);

    visited.len() == dag.nodes.len()
}

/// DFS helper for connectivity check.
fn dfs_visit(dag: &CtDag, node: CTID, visited: &mut BTreeMap<CTID, bool>) {
    visited.insert(node, true);

    for edge in &dag.edges {
        if edge.from == node && !visited.contains_key(&edge.to) {
            dfs_visit(dag, edge.to, visited);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

    #[test]
    fn test_chain_type_as_str() {
        assert_eq!(ChainType::Sequential.as_str(), "sequential");
        assert_eq!(ChainType::Router.as_str(), "router");
        assert_eq!(ChainType::MapReduce.as_str(), "map_reduce");
        assert_eq!(ChainType::Custom.as_str(), "custom");
    }

    #[test]
    fn test_data_mapping_creation() {
        let mapping = DataMapping::new("input".into(), "output".into());
        assert_eq!(mapping.source, "input");
        assert_eq!(mapping.target, "output");
        assert_eq!(mapping.transform, None);
    }

    #[test]
    fn test_data_mapping_with_transform() {
        let mapping = DataMapping::new("input".into(), "output".into())
            .with_transform("uppercase".into());
        assert_eq!(mapping.transform, Some("uppercase".into()));
    }

    #[test]
    fn test_chain_step_creation() {
        let step = ChainStep::new("step1".into(), "tool1".into());
        assert_eq!(step.name, "step1");
        assert_eq!(step.tool_binding, "tool1");
        assert!(step.condition.is_none());
    }

    #[test]
    fn test_chain_step_configuration() {
        let mut step = ChainStep::new("step1".into(), "tool1".into());
        step.add_input_mapping(DataMapping::new("a".into(), "b".into()));
        step.set_condition("x > 5".into());
        step.set_description("Test step".into());

        assert_eq!(step.input_mapping.len(), 1);
        assert_eq!(step.condition, Some("x > 5".into()));
        assert_eq!(step.description, Some("Test step".into()));
    }

    #[test]
    fn test_chain_definition_creation() {
        let chain = ChainDefinition::new(
            ChainType::Sequential,
            "{}".into(),
            "{}".into(),
        );
        assert_eq!(chain.chain_type, ChainType::Sequential);
        assert!(chain.steps.is_empty());
    }

    #[test]
    fn test_chain_definition_with_steps() {
        let mut chain = ChainDefinition::new(
            ChainType::Sequential,
            "{}".into(),
            "{}".into(),
        );
        chain.add_step(ChainStep::new("step1".into(), "tool1".into()));
        chain.add_step(ChainStep::new("step2".into(), "tool2".into()));

        assert_eq!(chain.steps.len(), 2);
    }

    #[test]
    fn test_edge_type_as_str() {
        assert_eq!(EdgeType::Dependency.as_str(), "dependency");
        assert_eq!(EdgeType::DataFlow.as_str(), "data_flow");
        assert_eq!(EdgeType::Conditional.as_str(), "conditional");
    }

    #[test]
    fn test_dag_edge_creation() {
        let edge = DagEdge::new(1, 2, EdgeType::Dependency);
        assert_eq!(edge.from, 1);
        assert_eq!(edge.to, 2);
        assert_eq!(edge.edge_type, EdgeType::Dependency);
    }

    #[test]
    fn test_dag_edge_with_label() {
        let edge = DagEdge::new(1, 2, EdgeType::Conditional)
            .with_label("condition".into());
        assert_eq!(edge.label, Some("condition".into()));
    }

    #[test]
    fn test_ct_node_creation() {
        let node = CtNode::new(1, "node1".into(), "{}".into());
        assert_eq!(node.ct_id, 1);
        assert_eq!(node.name, "node1");
    }

    #[test]
    fn test_ct_node_management() {
        let mut node = CtNode::new(1, "node1".into(), "{}".into());
        node.add_dependency(2);
        node.add_tool_binding("tool1".into());
        node.add_memory_ref("mem1".into());

        assert_eq!(node.dependencies.len(), 1);
        assert_eq!(node.tool_bindings.len(), 1);
        assert_eq!(node.memory_refs.len(), 1);
    }

    #[test]
    fn test_ct_dag_creation() {
        let dag = CtDag::new(1);
        assert_eq!(dag.root_ct, 1);
        assert!(dag.nodes.is_empty());
        assert!(dag.edges.is_empty());
    }

    #[test]
    fn test_ct_dag_add_node() {
        let mut dag = CtDag::new(1);
        dag.add_node(CtNode::new(1, "node1".into(), "{}".into()));
        dag.add_node(CtNode::new(2, "node2".into(), "{}".into()));

        assert_eq!(dag.node_count(), 2);
        assert!(dag.get_node(1).is_some());
    }

    #[test]
    fn test_ct_dag_add_edge() {
        let mut dag = CtDag::new(1);
        dag.add_edge(DagEdge::new(1, 2, EdgeType::Dependency));
        assert_eq!(dag.edge_count(), 1);
    }

    #[test]
    fn test_translate_sequential_empty_chain() {
        let chain = ChainDefinition::new(
            ChainType::Sequential,
            "{}".into(),
            "{}".into(),
        );
        let result = translate_sequential(&chain);
        assert!(result.is_err());
    }

    #[test]
    fn test_translate_sequential_single_step() {
        let mut chain = ChainDefinition::new(
            ChainType::Sequential,
            "{}".into(),
            "{}".into(),
        );
        chain.add_step(ChainStep::new("step1".into(), "tool1".into()));

        let dag = translate_sequential(&chain).expect("translation failed");
        assert_eq!(dag.node_count(), 1);
        assert_eq!(dag.edge_count(), 0);
    }

    #[test]
    fn test_translate_sequential_multiple_steps() {
        let mut chain = ChainDefinition::new(
            ChainType::Sequential,
            "{}".into(),
            "{}".into(),
        );
        chain.add_step(ChainStep::new("step1".into(), "tool1".into()));
        chain.add_step(ChainStep::new("step2".into(), "tool2".into()));
        chain.add_step(ChainStep::new("step3".into(), "tool3".into()));

        let dag = translate_sequential(&chain).expect("translation failed");
        assert_eq!(dag.node_count(), 3);
        assert_eq!(dag.edge_count(), 4); // 2 edges per dependency pair
    }

    #[test]
    fn test_validate_dag_valid() {
        let mut dag = CtDag::new(1);
        dag.add_node(CtNode::new(1, "node1".into(), "{}".into()));
        dag.add_node(CtNode::new(2, "node2".into(), "{}".into()));
        dag.add_edge(DagEdge::new(1, 2, EdgeType::Dependency));

        assert!(validate_dag(&dag).is_ok());
    }

    #[test]
    fn test_validate_dag_disconnected() {
        let mut dag = CtDag::new(1);
        dag.add_node(CtNode::new(1, "node1".into(), "{}".into()));
        dag.add_node(CtNode::new(3, "node3".into(), "{}".into())); // Not connected to root

        assert!(validate_dag(&dag).is_err());
    }

    #[test]
    fn test_translate_router_chain() {
        let mut chain = ChainDefinition::new(
            ChainType::Router,
            "{}".into(),
            "{}".into(),
        );
        let mut step1 = ChainStep::new("step1".into(), "tool1".into());
        step1.set_condition("condition1".into());
        chain.add_step(step1);
        chain.add_step(ChainStep::new("step2".into(), "tool2".into()));

        let dag = translate_router(&chain).expect("translation failed");
        assert_eq!(dag.node_count(), 2);
    }

    #[test]
    fn test_translate_map_reduce_chain() {
        let mut chain = ChainDefinition::new(
            ChainType::MapReduce,
            "{}".into(),
            "{}".into(),
        );
        chain.add_step(ChainStep::new("mapper".into(), "map_tool".into()));
        chain.add_step(ChainStep::new("worker1".into(), "work_tool".into()));
        chain.add_step(ChainStep::new("worker2".into(), "work_tool".into()));
        chain.add_step(ChainStep::new("reducer".into(), "reduce_tool".into()));

        let dag = translate_map_reduce(&chain).expect("translation failed");
        assert_eq!(dag.node_count(), 4);
    }

    #[test]
    fn test_translate_map_reduce_insufficient_steps() {
        let mut chain = ChainDefinition::new(
            ChainType::MapReduce,
            "{}".into(),
            "{}".into(),
        );
        chain.add_step(ChainStep::new("step1".into(), "tool1".into()));
        chain.add_step(ChainStep::new("step2".into(), "tool2".into()));

        let result = translate_map_reduce(&chain);
        assert!(result.is_err());
    }
}
