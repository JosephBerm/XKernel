# Engineer 10 — SDK: Tooling, Packaging & Documentation — Week 19

## Phase: 2 (Advanced Debugging Tools & Registry)

## Weekly Objective
Begin cs-capgraph implementation. Design capability graph visualization. Map isolation boundaries, delegation chains, and revocation paths. Create interactive graph tool.

## Document References
- **Primary:** Section 3.5.4 — cs-capgraph: visualize capability graph with page-table-backed isolation boundaries
- **Supporting:** Section 6.3 — Phase 2, Week 15-24

## Deliverables
- [ ] Capability graph data structure (Rust)
- [ ] Visualization format specification (GraphML/JSON)
- [ ] Isolation boundary detection and rendering
- [ ] Delegation chain visualization
- [ ] Revocation path analysis
- [ ] cs-capgraph CLI design
- [ ] Interactive graph viewer (ncurses or web-based MVP)
- [ ] Test suite with complex capability graphs

## Technical Specifications
### Capability Graph Structure
```rust
pub struct CapabilityGraph {
    nodes: Vec<Node>,           // Agents, Capabilities, Resources
    edges: Vec<Edge>,           // Delegation, Grant, Revoke
    isolation_domains: Vec<Domain>,
    page_table_mappings: Vec<Mapping>,
}

pub enum Node {
    Agent { id: u64, name: String },
    Capability { id: u64, name: String },
    Resource { id: u64, resource_type: ResourceType },
}

pub enum Edge {
    Delegation { from: u64, to: u64, transitive: bool },
    Grant { agent: u64, capability: u64, constraints: Vec<Constraint> },
    Revoke { agent: u64, capability: u64, timestamp: u64 },
}
```

### Visualization Output
```
Capability Graph: research_agent

Isolation Domains:
┌─ Domain 001 (research_agent) ─────────────┐
│ ┌─ Agent: research (owner) ────────────┐  │
│ │ Capabilities: tool_invoke, memory_*  │  │
│ │ Resources: web_search_tool, db_query │  │
│ └──────────────────────────────────────┘  │
│                                             │
│ ┌─ Agent: assistant (delegated) ──────┐  │
│ │ Capabilities: tool_invoke (subset)   │  │
│ │ Resources: web_search_tool           │  │
│ └──────────────────────────────────────┘  │
└─────────────────────────────────────────────┘

Delegation Chain: research → assistant → (revoked at 2026-02-28 10:15:22Z)

Page Table Mappings:
- research_agent:     0x7fff0000 - 0x7fffffff (read+write)
- assistant:          0x7fff1000 - 0x7fff8000 (read-only, delegated)
- revoked_thread:     (unmapped, revocation active)
```

### cs-capgraph CLI
```bash
cs-ctl capgraph show <agent_id>              # Show capability graph
cs-ctl capgraph show --isolation-only        # Show isolation domains
cs-ctl capgraph show --delegation-chain <a> <b>  # Path from A to B
cs-ctl capgraph show --export graphml        # Export to GraphML
```

## Dependencies
- **Blocked by:** Week 06 CI/CD, capability system finalized
- **Blocking:** Week 20 cs-capgraph refinement, Week 21-22 registry

## Acceptance Criteria
- [ ] Capability graph captures all agent relationships
- [ ] Isolation boundaries clearly visualized and accurate
- [ ] Delegation chains show transitive capabilities and constraints
- [ ] Revocation paths highlight impact of capability removal
- [ ] Interactive viewer responsive with 1000+ nodes
- [ ] Export formats (GraphML, JSON) compatible with standard tools

## Design Principles Alignment
- **Cognitive-Native:** Capability graph reflects agent communication model
- **Isolation by Default:** Isolation boundaries prominently displayed
- **Debuggability:** Delegation chains enable understanding of capability flow
- **Security:** Revocation paths show impact of security decisions
