// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! # Cognitive Substrate Capability Graph (cs-capgraph)
//!
//! The cs-capgraph crate provides capability analysis and visualization for the
//! Cognitive Substrate, enabling analysis of task permissions and resource access.
//!
//! ## Architecture
//!
//! This crate defines:
//! - **CapGraph**: Capability graph data structure
//! - **GraphNode**: Capability node representation
//! - **GraphEdge**: Capability delegation edges
//! - **GraphQuery**: Query interface for capability analysis
//! - **GraphVisualization**: Output formats for visualization


#![forbid(unsafe_code)]
#![warn(missing_docs)]





use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};




/// Capability type
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CapabilityType {
    /// Read capability
    Read,
    /// Write capability
    Write,
    /// Execute capability
    Execute,
    /// Admin capability
    Admin,
    /// Custom capability
    Custom(String),
}

/// Capability delegation edge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    /// Source node ID
    pub from: String,
    /// Target node ID
    pub to: String,
    /// Capability type
    pub capability: CapabilityType,
    /// Is transitive delegation
    pub transitive: bool,
}

/// Capability graph node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    /// Node ID
    pub id: String,
    /// Node type (task, process, service)
    pub node_type: String,
    /// Capabilities held
    pub capabilities: Vec<CapabilityType>,
    /// Parent node ID
    pub parent: Option<String>,
    /// Metadata
    pub metadata: BTreeMap<String, String>,
}

/// Graph query for capability analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQuery {
    /// Source node
    pub source: Option<String>,
    /// Target node
    pub target: Option<String>,
    /// Capability filter
    pub capability: Option<CapabilityType>,
    /// Max depth for traversal
    pub max_depth: Option<u32>,
}

/// Graph visualization output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GraphVisualization {
    /// DOT format for Graphviz
    Dot(String),
    /// JSON format
    Json(String),
    /// Text format
    Text(String),
}

/// Capability access graph
#[derive(Debug)]
pub struct CapGraph {
    /// Graph nodes
    pub nodes: BTreeMap<String, GraphNode>,
    /// Graph edges
    pub edges: Vec<GraphEdge>,
}

impl CapGraph {
    /// Create new capability graph
    pub fn new() -> Self {
        CapGraph {
            nodes: BTreeMap::new(),
            edges: Vec::new(),
        }
    }

    /// Add node to graph
    pub fn add_node(&mut self, node: GraphNode) -> bool {
        if self.nodes.contains_key(&node.id) {
            return false;
        }
        self.nodes.insert(node.id.clone(), node);
        true
    }

    /// Remove node from graph
    pub fn remove_node(&mut self, node_id: &str) -> bool {
        if self.nodes.remove(node_id).is_none() {
            return false;
        }
        self.edges.retain(|e| e.from != node_id && e.to != node_id);
        true
    }

    /// Add edge to graph
    pub fn add_edge(&mut self, edge: GraphEdge) -> bool {
        if !self.nodes.contains_key(&edge.from) || !self.nodes.contains_key(&edge.to) {
            return false;
        }
        self.edges.push(edge);
        true
    }

    /// Get outgoing edges for node
    pub fn get_outgoing_edges(&self, node_id: &str) -> Vec<&GraphEdge> {
        self.edges.iter()
            .filter(|e| e.from == node_id)
            .collect()
    }

    /// Get incoming edges for node
    pub fn get_incoming_edges(&self, node_id: &str) -> Vec<&GraphEdge> {
        self.edges.iter()
            .filter(|e| e.to == node_id)
            .collect()
    }

    /// Check if capability is held transitively
    pub fn has_capability_transitive(&self, node_id: &str, cap: &CapabilityType) -> bool {
        // Direct check
        if let Some(node) = self.nodes.get(node_id) {
            if node.capabilities.contains(cap) {
                return true;
            }
        }

        // Transitive check through edges
        for edge in self.get_incoming_edges(node_id) {
            if edge.transitive && edge.capability == *cap {
                if self.has_capability_transitive(&edge.from, cap) {
                    return true;
                }
            }
        }

        false
    }

    /// Query graph for capability paths
    pub fn query(&self, query: &GraphQuery) -> Vec<Vec<String>> {
        let mut paths = Vec::new();

        if let (Some(ref source), Some(ref target)) = (&query.source, &query.target) {
            self.find_paths(source, target, &mut Vec::new(), &mut paths, query.max_depth.unwrap_or(10));
        }

        paths
    }

    fn find_paths(&self, current: &str, target: &str, path: &mut Vec<String>, all_paths: &mut Vec<Vec<String>>, depth: u32) {
        if depth == 0 {
            return;
        }

        path.push(current.to_string());

        if current == target {
            all_paths.push(path.clone());
        } else {
            for edge in self.get_outgoing_edges(current) {
                if !path.contains(&edge.to) {
                    self.find_paths(&edge.to, target, path, all_paths, depth - 1);
                }
            }
        }

        path.pop();
    }

    /// Generate visualization
    pub fn visualize(&self, format: &str) -> GraphVisualization {
        match format {
            "dot" => self.to_dot(),
            "json" => self.to_json(),
            _ => self.to_text(),
        }
    }

    fn to_dot(&self) -> GraphVisualization {
        let mut dot = String::new();
        dot.push_str("digraph CapabilityGraph {\n");

        for node in self.nodes.values() {
            dot.push_str(&format!("  \"{}\" [label=\"{}\"];\n", node.id, node.id));
        }

        for edge in &self.edges {
            let label = format!("{:?}", edge.capability);
            dot.push_str(&format!("  \"{}\" -> \"{}\" [label=\"{}\"];\n", edge.from, edge.to, label));
        }

        dot.push_str("}\n");

        GraphVisualization::Dot(dot)
    }

    fn to_json(&self) -> GraphVisualization {
        let mut json = String::new();
        json.push_str("{\n");
        json.push_str(&format!("  \"nodes\": {},\n", self.nodes.len()));
        json.push_str(&format!("  \"edges\": {}\n", self.edges.len()));
        json.push_str("}\n");

        GraphVisualization::Json(json)
    }

    fn to_text(&self) -> GraphVisualization {
        let mut text = String::new();
        text.push_str("Capability Graph\n");
        text.push_str("================\n\n");

        text.push_str("Nodes:\n");
        for node in self.nodes.values() {
            text.push_str(&format!("  {}: {}\n", node.id, node.node_type));
        }

        text.push_str("\nEdges:\n");
        for edge in &self.edges {
            text.push_str(&format!("  {} -> {} ({:?})\n", edge.from, edge.to, edge.capability));
        }

        GraphVisualization::Text(text)
    }

    /// Get node by ID
    pub fn get_node(&self, node_id: &str) -> Option<&GraphNode> {
        self.nodes.get(node_id)
    }

    /// Get node count
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get edge count
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
}

impl Default for CapGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capgraph_creation() {
        let graph = CapGraph::new();
        assert_eq!(graph.node_count(), 0);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_add_node() {
        let mut graph = CapGraph::new();
        let node = GraphNode {
            id: "task1".to_string(),
            node_type: "task".to_string(),
            capabilities: vec![CapabilityType::Read],
            parent: None,
            metadata: BTreeMap::new(),
        };

        assert!(graph.add_node(node));
        assert_eq!(graph.node_count(), 1);
    }

    #[test]
    fn test_add_duplicate_node() {
        let mut graph = CapGraph::new();
        let node = GraphNode {
            id: "task1".to_string(),
            node_type: "task".to_string(),
            capabilities: Vec::new(),
            parent: None,
            metadata: BTreeMap::new(),
        };

        graph.add_node(node.clone());
        assert!(!graph.add_node(node));
    }

    #[test]
    fn test_remove_node() {
        let mut graph = CapGraph::new();
        let node = GraphNode {
            id: "task1".to_string(),
            node_type: "task".to_string(),
            capabilities: Vec::new(),
            parent: None,
            metadata: BTreeMap::new(),
        };

        graph.add_node(node);
        assert!(graph.remove_node("task1"));
        assert_eq!(graph.node_count(), 0);
    }

    #[test]
    fn test_add_edge() {
        let mut graph = CapGraph::new();

        let node1 = GraphNode {
            id: "task1".to_string(),
            node_type: "task".to_string(),
            capabilities: Vec::new(),
            parent: None,
            metadata: BTreeMap::new(),
        };

        let node2 = GraphNode {
            id: "task2".to_string(),
            node_type: "task".to_string(),
            capabilities: Vec::new(),
            parent: None,
            metadata: BTreeMap::new(),
        };

        graph.add_node(node1);
        graph.add_node(node2);

        let edge = GraphEdge {
            from: "task1".to_string(),
            to: "task2".to_string(),
            capability: CapabilityType::Read,
            transitive: false,
        };

        assert!(graph.add_edge(edge));
        assert_eq!(graph.edge_count(), 1);
    }

    #[test]
    fn test_add_edge_missing_nodes() {
        let mut graph = CapGraph::new();

        let edge = GraphEdge {
            from: "task1".to_string(),
            to: "task2".to_string(),
            capability: CapabilityType::Read,
            transitive: false,
        };

        assert!(!graph.add_edge(edge));
    }

    #[test]
    fn test_get_outgoing_edges() {
        let mut graph = CapGraph::new();

        let node1 = GraphNode {
            id: "task1".to_string(),
            node_type: "task".to_string(),
            capabilities: Vec::new(),
            parent: None,
            metadata: BTreeMap::new(),
        };

        let node2 = GraphNode {
            id: "task2".to_string(),
            node_type: "task".to_string(),
            capabilities: Vec::new(),
            parent: None,
            metadata: BTreeMap::new(),
        };

        graph.add_node(node1);
        graph.add_node(node2);

        let edge = GraphEdge {
            from: "task1".to_string(),
            to: "task2".to_string(),
            capability: CapabilityType::Read,
            transitive: false,
        };

        graph.add_edge(edge);

        let outgoing = graph.get_outgoing_edges("task1");
        assert_eq!(outgoing.len(), 1);
    }

    #[test]
    fn test_get_incoming_edges() {
        let mut graph = CapGraph::new();

        let node1 = GraphNode {
            id: "task1".to_string(),
            node_type: "task".to_string(),
            capabilities: Vec::new(),
            parent: None,
            metadata: BTreeMap::new(),
        };

        let node2 = GraphNode {
            id: "task2".to_string(),
            node_type: "task".to_string(),
            capabilities: Vec::new(),
            parent: None,
            metadata: BTreeMap::new(),
        };

        graph.add_node(node1);
        graph.add_node(node2);

        let edge = GraphEdge {
            from: "task1".to_string(),
            to: "task2".to_string(),
            capability: CapabilityType::Read,
            transitive: false,
        };

        graph.add_edge(edge);

        let incoming = graph.get_incoming_edges("task2");
        assert_eq!(incoming.len(), 1);
    }

    #[test]
    fn test_has_capability_direct() {
        let mut graph = CapGraph::new();
        let node = GraphNode {
            id: "task1".to_string(),
            node_type: "task".to_string(),
            capabilities: vec![CapabilityType::Read, CapabilityType::Write],
            parent: None,
            metadata: BTreeMap::new(),
        };

        graph.add_node(node);
        assert!(graph.has_capability_transitive("task1", &CapabilityType::Read));
        assert!(!graph.has_capability_transitive("task1", &CapabilityType::Execute));
    }

    #[test]
    fn test_visualization_dot() {
        let graph = CapGraph::new();
        match graph.visualize("dot") {
            GraphVisualization::Dot(s) => {
                assert!(s.contains("digraph"));
            }
            _ => panic!("Expected Dot format"),
        }
    }

    #[test]
    fn test_visualization_json() {
        let graph = CapGraph::new();
        match graph.visualize("json") {
            GraphVisualization::Json(s) => {
                assert!(s.contains("nodes"));
            }
            _ => panic!("Expected Json format"),
        }
    }

    #[test]
    fn test_visualization_text() {
        let graph = CapGraph::new();
        match graph.visualize("text") {
            GraphVisualization::Text(s) => {
                assert!(s.contains("Capability Graph"));
            }
            _ => panic!("Expected Text format"),
        }
    }

    #[test]
    fn test_get_node() {
        let mut graph = CapGraph::new();
        let node = GraphNode {
            id: "task1".to_string(),
            node_type: "task".to_string(),
            capabilities: Vec::new(),
            parent: None,
            metadata: BTreeMap::new(),
        };

        graph.add_node(node);
        assert!(graph.get_node("task1").is_some());
        assert!(graph.get_node("task2").is_none());
    }

    #[test]
    fn test_capability_type_equality() {
        let cap1 = CapabilityType::Read;
        let cap2 = CapabilityType::Read;
        let cap3 = CapabilityType::Write;

        assert_eq!(cap1, cap2);
        assert_ne!(cap1, cap3);
    }

    #[test]
    fn test_query_graph() {
        let mut graph = CapGraph::new();

        let node1 = GraphNode {
            id: "task1".to_string(),
            node_type: "task".to_string(),
            capabilities: Vec::new(),
            parent: None,
            metadata: BTreeMap::new(),
        };

        let node2 = GraphNode {
            id: "task2".to_string(),
            node_type: "task".to_string(),
            capabilities: Vec::new(),
            parent: None,
            metadata: BTreeMap::new(),
        };

        graph.add_node(node1);
        graph.add_node(node2);

        let edge = GraphEdge {
            from: "task1".to_string(),
            to: "task2".to_string(),
            capability: CapabilityType::Read,
            transitive: false,
        };

        graph.add_edge(edge);

        let query = GraphQuery {
            source: Some("task1".to_string()),
            target: Some("task2".to_string()),
            capability: None,
            max_depth: None,
        };

        let paths = graph.query(&query);
        assert!(!paths.is_empty());
    }
}
