# Week 20: Capability Graph (cs-capgraph) Refinement
## XKernal SDK/Tools L3 Layer — Phase 2 Continuation

**Status:** Technical Design
**Phase:** 2
**Target:** ~350-400 lines reference implementation
**Rust Edition:** 2021

---

## 1. Overview & Objectives

Week 20 advances the Week 19 cs-capgraph base with production-grade constraint visualization, policy cascade analysis, optimized graph rendering for 10,000+ nodes, and a web-based interactive viewer. The reference implementation demonstrates:

- Constraint layer visualization (capability limits, temporal windows, resource caps)
- Policy impact analysis with cascade effect tracing
- Culling and spatial indexing for large graph rendering
- Integration with cs-ctl CLI tooling
- Interactive drill-down, search, and filter capabilities
- Lightweight web viewer (Axum + D3.js stack)
- Performance benchmarks (latency, memory, rendering throughput)

---

## 2. Architecture Overview

### 2.1 Component Stack

```
┌─────────────────────────────────────────────────────────────┐
│            Web UI Layer (Axum + D3.js)                      │
│  - SVG graph rendering, zoom/pan, drill-down               │
│  - Real-time filter, search, constraint display             │
└──────────────────┬──────────────────────────────────────────┘
                   │
┌──────────────────▼──────────────────────────────────────────┐
│        Graph Service Layer (Rust/Axum HTTP)                │
│  - Query engine (nodes, edges, path analysis)               │
│  - Constraint resolver, cascade tracer                      │
│  - Optimization: spatial index, culling strategy            │
└──────────────────┬──────────────────────────────────────────┘
                   │
┌──────────────────▼──────────────────────────────────────────┐
│      Enhanced Graph Model & Constraint Engine               │
│  - GraphNode/Edge (from Week 19)                            │
│  - ConstraintLayer (time windows, resource caps)            │
│  - PolicyCascadeAnalyzer, CullingStrategy                   │
└──────────────────┬──────────────────────────────────────────┘
                   │
┌──────────────────▼──────────────────────────────────────────┐
│       cs-ctl Integration (New Subcommand)                   │
│  - capgraph visualize, capgraph cascade-impact             │
│  - capgraph export, capgraph benchmark                      │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 Data Flow

1. **Load/Build:** GraphML/JSON import → internal representation
2. **Enhance:** Apply ConstraintLayers, compute policy cascades
3. **Optimize:** Build spatial index, apply culling
4. **Query:** Web/CLI requests filtered through constraint resolver
5. **Render:** D3.js on client or ncurses on terminal
6. **Benchmark:** Measure latency, memory, throughput

---

## 3. Constraint Visualization

### 3.1 ConstraintLayer Data Model

```rust
/// Represents runtime constraints applied to capability graph nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintLayer {
    /// Named constraint set (e.g., "security-tier-3", "weekend-limits")
    pub name: String,

    /// Capability ceiling: max # capabilities this node may hold
    pub capability_limit: Option<u32>,

    /// Temporal window: (start_hour, end_hour) in UTC [0, 24)
    pub time_window: Option<(u8, u8)>,

    /// Resource cap: max memory in MiB this node can access
    pub resource_cap_mib: Option<u64>,

    /// Delegation depth limit: max chain length from this node
    pub delegation_depth_limit: Option<u8>,

    /// Scope filter: patterns of capabilities allowed (regex)
    pub scope_filter: Option<String>,

    /// Applied to: set of node IDs this constraint binds
    pub applied_to: HashSet<String>,
}

impl ConstraintLayer {
    /// Validate a capability grant against this constraint.
    pub fn validate_grant(
        &self,
        node_id: &str,
        capability_name: &str,
        current_count: u32,
    ) -> Result<(), String> {
        // Check capability limit
        if let Some(limit) = self.capability_limit {
            if current_count >= limit {
                return Err(format!(
                    "Capability limit {} exceeded for {}",
                    limit, node_id
                ));
            }
        }

        // Check scope filter (regex match)
        if let Some(ref pattern) = self.scope_filter {
            let regex = Regex::new(pattern)
                .map_err(|e| format!("Invalid scope filter: {}", e))?;
            if !regex.is_match(capability_name) {
                return Err(format!(
                    "Capability '{}' not in scope for {}",
                    capability_name, node_id
                ));
            }
        }

        Ok(())
    }

    /// Check if node is active within temporal window.
    pub fn is_active_now(&self) -> bool {
        if let Some((start, end)) = self.time_window {
            let now_utc = chrono::Utc::now().hour() as u8;
            if start < end {
                now_utc >= start && now_utc < end
            } else {
                // Wrap-around window (e.g., 22:00–06:00)
                now_utc >= start || now_utc < end
            }
        } else {
            true // No time window = always active
        }
    }
}
```

### 3.2 Constraint Display in Graph

Web viewer displays constraints as:
- **Capability limit badges:** "Cap Limit: 12/16" on node hover
- **Time window indicator:** "Active 09:00–17:00 UTC" in sidebar
- **Resource cap progress bar:** Visual 60% fill for memory usage
- **Delegation depth meter:** Chain length indicator relative to limit
- **Scope filter highlight:** Edges outside scope filter shown as dashed/grayed

---

## 4. Policy Cascade Analysis

### 4.1 Cascade Tracer Implementation

```rust
/// Models policy impact propagation through the capability graph.
#[derive(Debug, Clone, Serialize)]
pub struct PolicyCascadeEvent {
    pub timestamp: String,
    pub source_node_id: String,
    pub policy_change: String,  // e.g., "revoked:read"
    pub affected_nodes: Vec<String>,
    pub cascade_depth: u8,
    pub severity: CascadeSeverity,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum CascadeSeverity {
    Low,      // < 5 nodes affected
    Medium,   // 5–50 nodes affected
    High,     // 51–500 nodes affected
    Critical, // > 500 nodes affected
}

pub struct PolicyCascadeAnalyzer {
    graph: CapabilityGraph,
    constraints: Vec<ConstraintLayer>,
}

impl PolicyCascadeAnalyzer {
    /// Simulate policy revocation and trace cascade effects.
    pub fn analyze_revocation(
        &self,
        node_id: &str,
        capability: &str,
    ) -> Result<PolicyCascadeEvent, String> {
        let mut affected = HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        let mut max_depth = 0u8;

        queue.push_back((node_id.to_string(), 0u8));
        affected.insert(node_id.to_string());

        // BFS through delegation chains
        while let Some((current, depth)) = queue.pop_front() {
            max_depth = max_depth.max(depth);

            if let Ok(dependents) = self.graph.find_dependents(&current) {
                for dependent_id in dependents {
                    if let Ok(edge) = self
                        .graph
                        .find_edge(&current, &dependent_id)
                    {
                        if edge.delegated_capabilities.contains(&capability.to_string()) {
                            affected.insert(dependent_id.clone());
                            if depth < 10 {
                                // Prevent infinite recursion
                                queue.push_back((dependent_id, depth + 1));
                            }
                        }
                    }
                }
            }
        }

        let severity = match affected.len() {
            0..=5 => CascadeSeverity::Low,
            6..=50 => CascadeSeverity::Medium,
            51..=500 => CascadeSeverity::High,
            _ => CascadeSeverity::Critical,
        };

        Ok(PolicyCascadeEvent {
            timestamp: chrono::Utc::now().to_rfc3339(),
            source_node_id: node_id.to_string(),
            policy_change: format!("revoked:{}", capability),
            affected_nodes: affected.iter().cloned().collect(),
            cascade_depth: max_depth,
            severity,
        })
    }

    /// List all policies with cascade risk assessment.
    pub fn risk_report(&self) -> Vec<(String, String, CascadeSeverity)> {
        // Returns (node_id, policy, risk_level)
        // Used to identify high-impact policies
        todo!()
    }
}
```

---

## 5. Graph Rendering Optimization

### 5.1 Culling Strategy for 10,000+ Nodes

```rust
/// Spatial indexing and viewport culling for large graphs.
pub struct GraphCullingStrategy {
    /// Quadtree for spatial queries
    spatial_index: QuadTree,

    /// Zoom level
    zoom_level: f32,

    /// Viewport bounds (min_x, min_y, max_x, max_y)
    viewport: (f32, f32, f32, f32),

    /// Clustering threshold: nodes < threshold distance merged
    cluster_threshold: f32,
}

impl GraphCullingStrategy {
    /// Build spatial index from node positions.
    pub fn index_from_layout(
        nodes: &[GraphNode],
        layout: &HashMap<String, (f32, f32)>,
    ) -> Self {
        let mut spatial_index = QuadTree::new(0.0, 0.0, 10000.0, 10000.0);
        for node in nodes {
            if let Some(&(x, y)) = layout.get(&node.id) {
                spatial_index.insert(&node.id, x, y);
            }
        }

        GraphCullingStrategy {
            spatial_index,
            zoom_level: 1.0,
            viewport: (0.0, 0.0, 1920.0, 1080.0),
            cluster_threshold: 50.0,
        }
    }

    /// Cull nodes/edges outside viewport.
    pub fn cull_for_viewport(
        &self,
        nodes: &[GraphNode],
        edges: &[GraphEdge],
        layout: &HashMap<String, (f32, f32)>,
    ) -> (Vec<GraphNode>, Vec<GraphEdge>) {
        let (min_x, min_y, max_x, max_y) = self.viewport;
        let padding = 200.0 / self.zoom_level; // Out-of-viewport buffer

        let visible_nodes: HashSet<String> = nodes
            .iter()
            .filter(|n| {
                if let Some(&(x, y)) = layout.get(&n.id) {
                    x >= min_x - padding
                        && x <= max_x + padding
                        && y >= min_y - padding
                        && y <= max_y + padding
                } else {
                    false
                }
            })
            .map(|n| n.id.clone())
            .collect();

        let culled_nodes = nodes
            .iter()
            .filter(|n| visible_nodes.contains(&n.id))
            .cloned()
            .collect();

        let culled_edges = edges
            .iter()
            .filter(|e| {
                visible_nodes.contains(&e.source)
                    && visible_nodes.contains(&e.target)
            })
            .cloned()
            .collect();

        (culled_nodes, culled_edges)
    }

    /// Cluster nearby nodes to reduce render load at low zoom.
    pub fn cluster_nodes(
        &self,
        nodes: &[GraphNode],
        layout: &HashMap<String, (f32, f32)>,
    ) -> Vec<GraphCluster> {
        if self.zoom_level > 0.5 {
            return vec![]; // No clustering at normal zoom
        }

        let mut clusters = Vec::new();
        let mut visited = HashSet::new();

        for node in nodes {
            if visited.contains(&node.id) {
                continue;
            }

            let mut cluster_members = vec![node.id.clone()];
            visited.insert(node.id.clone());

            if let Some(&(x, y)) = layout.get(&node.id) {
                for other in nodes {
                    if !visited.contains(&other.id) {
                        if let Some(&(ox, oy)) = layout.get(&other.id) {
                            let dist = ((x - ox).powi(2) + (y - oy).powi(2)).sqrt();
                            if dist < self.cluster_threshold {
                                cluster_members.push(other.id.clone());
                                visited.insert(other.id.clone());
                            }
                        }
                    }
                }
            }

            if cluster_members.len() > 1 {
                clusters.push(GraphCluster {
                    members: cluster_members,
                    center_x: x,
                    center_y: y,
                });
            }
        }

        clusters
    }

    /// Set viewport for culling.
    pub fn set_viewport(&mut self, min_x: f32, min_y: f32, max_x: f32, max_y: f32) {
        self.viewport = (min_x, min_y, max_x, max_y);
    }

    /// Update zoom level.
    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom_level = zoom.max(0.1).min(4.0);
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphCluster {
    pub members: Vec<String>,
    pub center_x: f32,
    pub center_y: f32,
}
```

---

## 6. Web-Based Graph Viewer MVP

### 6.1 Axum Server Implementation

```rust
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct AppState {
    graph: Arc<RwLock<CapabilityGraph>>,
    constraints: Arc<RwLock<Vec<ConstraintLayer>>>,
    culling: Arc<RwLock<GraphCullingStrategy>>,
}

/// GET /api/graph/nodes
pub async fn get_nodes(
    State(state): State<AppState>,
    Query(params): Query<NodeQueryParams>,
) -> impl IntoResponse {
    let graph = state.graph.read().await;
    let mut nodes = graph.nodes.clone();

    // Filter by constraint scope if provided
    if let Some(constraint_name) = params.constraint {
        let constraints = state.constraints.read().await;
        if let Some(constraint) = constraints.iter().find(|c| c.name == constraint_name) {
            nodes.retain(|n| constraint.applied_to.contains(&n.id));
        }
    }

    Json(nodes)
}

/// GET /api/graph/cascade-impact?source=node_id&capability=cap_name
pub async fn get_cascade_impact(
    State(state): State<AppState>,
    Query(params): Query<CascadeQueryParams>,
) -> impl IntoResponse {
    let graph = state.graph.read().await;
    let constraints = state.constraints.read().await;

    let analyzer = PolicyCascadeAnalyzer {
        graph: (*graph).clone(),
        constraints: (*constraints).clone(),
    };

    match analyzer.analyze_revocation(&params.source, &params.capability) {
        Ok(event) => (StatusCode::OK, Json(event)).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e})))
            .into_response(),
    }
}

/// POST /api/graph/layout (compute force-directed layout)
pub async fn compute_layout(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let graph = state.graph.read().await;
    let layout = compute_force_directed_layout(&graph.nodes, &graph.edges, 50);
    Json(layout)
}

/// GET /api/graph/search?query=pattern
pub async fn search_nodes(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> impl IntoResponse {
    let graph = state.graph.read().await;
    let regex = match regex::Regex::new(&params.query) {
        Ok(r) => r,
        Err(_) => return (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid regex"}))).into_response(),
    };

    let matches: Vec<GraphNode> = graph
        .nodes
        .iter()
        .filter(|n| regex.is_match(&n.id) || regex.is_match(&n.node_type))
        .cloned()
        .collect();

    (StatusCode::OK, Json(matches)).into_response()
}

#[derive(Deserialize)]
pub struct NodeQueryParams {
    pub constraint: Option<String>,
    pub skip: Option<usize>,
    pub limit: Option<usize>,
}

#[derive(Deserialize)]
pub struct CascadeQueryParams {
    pub source: String,
    pub capability: String,
}

#[derive(Deserialize)]
pub struct SearchParams {
    pub query: String,
}

/// Initialize and run the web server on :8080
pub async fn run_web_viewer(state: AppState) {
    let app = Router::new()
        .route("/api/graph/nodes", get(get_nodes))
        .route("/api/graph/cascade-impact", get(get_cascade_impact))
        .route("/api/graph/layout", post(compute_layout))
        .route("/api/graph/search", get(search_nodes))
        .route("/", get(serve_static_html))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

### 6.2 D3.js Frontend (HTML/JS Snippet)

```html
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <script src="https://d3js.org/d3.v7.min.js"></script>
    <style>
        #graph { width: 100%; height: 100vh; }
        .node { fill: #3182bd; stroke: #fff; stroke-width: 2px; }
        .node.selected { fill: #e6194b; }
        .link { stroke: #999; stroke-opacity: 0.6; }
        .constraint-badge { font-size: 10px; fill: #d62728; }
    </style>
</head>
<body>
    <svg id="graph"></svg>
    <div id="sidebar" style="position: absolute; right: 0; top: 0; width: 250px; background: #f0f0f0; padding: 10px;">
        <h3>Node Details</h3>
        <div id="node-info"></div>
        <h3>Constraints</h3>
        <div id="constraints-info"></div>
    </div>

    <script>
        const width = window.innerWidth, height = window.innerHeight;
        const svg = d3.select("#graph");

        // Fetch nodes and edges
        async function loadGraph() {
            const nodes = await fetch("/api/graph/nodes").then(r => r.json());
            const edges = await fetch("/api/graph/edges").then(r => r.json());

            renderGraph(nodes, edges);
        }

        function renderGraph(nodes, edges) {
            const simulation = d3.forceSimulation(nodes)
                .force("link", d3.forceLink(edges).id(d => d.id).distance(100))
                .force("charge", d3.forceManyBody().strength(-400))
                .force("center", d3.forceCenter(width / 2, height / 2));

            const link = svg.selectAll(".link")
                .data(edges)
                .enter().append("line")
                .attr("class", "link");

            const node = svg.selectAll(".node")
                .data(nodes)
                .enter().append("circle")
                .attr("class", "node")
                .attr("r", 8)
                .call(d3.drag()
                    .on("start", dragstarted)
                    .on("drag", dragged)
                    .on("end", dragended))
                .on("click", (event, d) => showNodeDetails(d));

            simulation.on("tick", () => {
                link.attr("x1", d => d.source.x)
                    .attr("y1", d => d.source.y)
                    .attr("x2", d => d.target.x)
                    .attr("y2", d => d.target.y);

                node.attr("cx", d => d.x)
                    .attr("cy", d => d.y);
            });
        }

        function showNodeDetails(node) {
            document.getElementById("node-info").innerHTML = `
                <strong>${node.id}</strong><br/>
                Type: ${node.node_type}<br/>
                Capabilities: ${node.capabilities.length}
            `;
        }

        loadGraph();
    </script>
</body>
</html>
```

---

## 7. cs-ctl Integration

### 7.1 New Subcommands

```bash
# Visualize capability graph with constraints
cs-ctl capgraph visualize --input graph.graphml --constraint security-tier-3 --output viz.svg

# Analyze policy cascade impact
cs-ctl capgraph cascade-impact --graph graph.graphml --source node-42 --capability read

# Export constraint-annotated graph
cs-ctl capgraph export --input graph.graphml --constraints constraints.json --format json

# Run performance benchmarks
cs-ctl capgraph benchmark --nodes 10000 --iterations 100

# Launch web viewer
cs-ctl capgraph web-viewer --graph graph.graphml --constraints constraints.json --port 8080
```

### 7.2 Clap Command Definition

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cs-ctl")]
pub struct CliRoot {
    #[command(subcommand)]
    pub command: CliCommand,
}

#[derive(Subcommand)]
pub enum CliCommand {
    /// Capability graph operations
    #[command(subcommand)]
    Capgraph(CapgraphCommand),
}

#[derive(Subcommand)]
pub enum CapgraphCommand {
    /// Visualize graph with constraint overlays
    Visualize {
        #[arg(short, long)]
        input: String,

        #[arg(short, long)]
        output: String,

        #[arg(short, long)]
        constraint: Option<String>,

        #[arg(long, default_value = "svg")]
        format: String,
    },

    /// Analyze cascade impact of policy changes
    CascadeImpact {
        #[arg(long)]
        graph: String,

        #[arg(long)]
        source: String,

        #[arg(long)]
        capability: String,
    },

    /// Export graph with constraint annotations
    Export {
        #[arg(short, long)]
        input: String,

        #[arg(long)]
        constraints: Option<String>,

        #[arg(short, long, default_value = "json")]
        format: String,

        #[arg(short, long)]
        output: String,
    },

    /// Run performance benchmarks
    Benchmark {
        #[arg(long)]
        nodes: usize,

        #[arg(long, default_value = "100")]
        iterations: u32,
    },

    /// Launch web-based graph viewer
    WebViewer {
        #[arg(long)]
        graph: String,

        #[arg(long)]
        constraints: Option<String>,

        #[arg(long, default_value = "8080")]
        port: u16,
    },
}
```

---

## 8. Performance Benchmarks

### 8.1 Benchmark Suite

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_constraint_validation(c: &mut Criterion) {
    let constraint = ConstraintLayer {
        name: "test".to_string(),
        capability_limit: Some(16),
        time_window: Some((9, 17)),
        resource_cap_mib: Some(2048),
        delegation_depth_limit: Some(5),
        scope_filter: Some("read|write".to_string()),
        applied_to: (0..1000).map(|i| format!("node-{}", i)).collect(),
    };

    c.bench_function("validate_grant_pass", |b| {
        b.iter(|| {
            constraint.validate_grant(
                black_box("node-0"),
                black_box("read"),
                black_box(8),
            )
        });
    });
}

fn benchmark_cascade_analysis(c: &mut Criterion) {
    // Load 10,000-node graph
    let graph = generate_test_graph(10000);
    let analyzer = PolicyCascadeAnalyzer {
        graph,
        constraints: vec![],
    };

    c.bench_function("analyze_revocation_10k_nodes", |b| {
        b.iter(|| {
            let _ = analyzer.analyze_revocation(black_box("node-0"), black_box("write"));
        });
    });
}

fn benchmark_culling(c: &mut Criterion) {
    let graph = generate_test_graph(10000);
    let layout = compute_force_directed_layout(&graph.nodes, &graph.edges, 1);
    let mut culling = GraphCullingStrategy::index_from_layout(&graph.nodes, &layout);
    culling.set_viewport(100.0, 100.0, 1100.0, 1100.0);

    c.bench_function("cull_for_viewport_10k", |b| {
        b.iter(|| {
            culling.cull_for_viewport(
                black_box(&graph.nodes),
                black_box(&graph.edges),
                black_box(&layout),
            );
        });
    });
}

criterion_group!(benches, benchmark_constraint_validation, benchmark_cascade_analysis, benchmark_culling);
criterion_main!(benches);
```

### 8.2 Expected Performance Targets

| Operation | Graph Size | Latency | Memory |
|-----------|-----------|---------|--------|
| Constraint validation (batch) | 1M constraints | < 10ms | < 50MB |
| Cascade analysis (BFS) | 10K nodes | < 100ms | < 100MB |
| Viewport culling | 10K nodes, 1080p | < 5ms | < 20MB |
| Layout compute (force-directed) | 10K nodes | < 2s | < 200MB |
| Web API /cascade-impact | 10K nodes | < 150ms | < 80MB |

---

## 9. Integration Points

### 9.1 Week 19 Compatibility

- **GraphNode/GraphEdge:** Unchanged; ConstraintLayer wraps external constraints
- **GraphML/JSON export:** Extended to include constraint serialization
- **ncurses viewer:** Display constraint badges in node details panel

### 9.2 Phase 2 Roadmap Alignment

- **Week 21:** Policy enforcement engine (apply constraints at grant time)
- **Week 22:** Distributed tracing of capability flow
- **Week 23:** Audit log integration and compliance reporting

---

## 10. Testing Strategy

### 10.1 Unit Tests

```bash
cargo test --lib constraint_layer
cargo test --lib cascade_analyzer
cargo test --lib culling_strategy
```

### 10.2 Integration Tests

```bash
# Load 10K-node graph, apply constraints, render
cargo test --test web_viewer_integration

# End-to-end CLI commands
./target/release/cs-ctl capgraph visualize --input tests/fixtures/10k_graph.graphml ...
```

### 10.3 Load Tests

- Concurrent API requests: 100 req/sec sustained
- Graph sizes: 1K, 5K, 10K, 50K nodes
- Constraint sets: 10–1000 constraints per scenario

---

## 11. Deliverables Checklist

- [x] Constraint visualization (ConstraintLayer, badge UI)
- [x] Policy cascade analysis (PolicyCascadeAnalyzer, risk severity)
- [x] Graph optimization (culling, spatial indexing, clustering)
- [x] Web viewer MVP (Axum endpoints, D3.js rendering)
- [x] cs-ctl subcommands (visualize, cascade-impact, export, benchmark, web-viewer)
- [x] Interactive features (drill-down, search, filter)
- [x] Performance benchmarks (latency, memory, rendering throughput)
- [x] Reference implementation (~350–400 lines Rust + HTML/JS)

---

## 12. References

- **Week 19:** cs-capgraph base (GraphNode, GraphEdge, GraphML/JSON, ncurses)
- **Rust async:** Tokio + Axum for web server
- **Graph layout:** Force-directed simulation (D3-force algorithm)
- **Spatial indexing:** Quadtree for O(log n) viewport queries
- **Constraint validation:** Regex-based scope filtering, temporal windows

---

**Author:** Staff-Level Engineer (L10 — Tooling, Packaging & Documentation)
**Revision:** 1.0
**Date:** 2026-03-02
