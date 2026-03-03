# Engineer 10 — SDK: Tooling, Packaging & Documentation — Week 20

## Phase: 2 (Advanced Debugging Tools & Registry)

## Weekly Objective
Refine cs-capgraph implementation. Optimize graph rendering performance. Add constraint visualization and policy impact analysis. Integrate with cs-ctl. Prepare for Phase 3 documentation.

## Document References
- **Primary:** Section 3.5.4 — cs-capgraph debugging tool
- **Supporting:** Section 6.3 — Phase 2, Week 15-24

## Deliverables
- [ ] Constraint visualization (capability limits, time windows, resource caps)
- [ ] Policy impact analysis (show impact of adding/removing agents)
- [ ] Graph rendering optimization (handle 10000+ nodes)
- [ ] cs-ctl integration: `cs-ctl capgraph`
- [ ] Interactive features (drill-down, search, filter)
- [ ] Web-based graph viewer (optional MVP)
- [ ] Performance benchmarks

## Technical Specifications
### Constraint Visualization
```
Capability Grant: research_agent → assistant → tool_invoke

Constraints:
├─ Time Window: 2026-02-01 to 2026-03-01 (active)
├─ Rate Limit: 100 invocations/hour
├─ Resource Cap: $10.00/day
├─ Allowed Tools: [web_search, code_execution] (subset)
└─ Audit Required: yes (log all invocations)

Status: ACTIVE (46 hours remaining, 67 invocations/hour used)
```

### Policy Impact Analysis
```
Scenario: Revoke tool_invoke capability from assistant

Impact Analysis:
├─ Direct: assistant loses tool_invoke (1 agent affected)
├─ Transitive: No downstream delegations
├─ Resources: web_search_tool (orphaned, can be reassigned)
├─ Cost: $0.05/hour savings
└─ Risk: Research agent must perform all tool calls (overhead: +15%)

Recommendation: Implement and monitor for 1 hour before finalizing
```

### Graph Rendering Optimization
- Hierarchical layout (agents at top, capabilities below)
- Edge bundling to reduce visual clutter
- Incremental rendering for large graphs
- Cache computed layouts

### Interactive Features
```bash
cs-ctl capgraph show                              # Full graph
cs-ctl capgraph show --filter "tool_invoke"       # Filter by capability
cs-ctl capgraph show --search "research"          # Search for agent/capability
cs-ctl capgraph drill-down agent:001              # Expand agent details
```

## Dependencies
- **Blocked by:** Week 19 cs-capgraph prototype
- **Blocking:** Week 21-22 cs-pkg registry integration

## Acceptance Criteria
- [ ] Constraint visualization clear and complete
- [ ] Policy impact analysis predicts changes accurately
- [ ] Graph rendering handles 10000+ nodes smoothly
- [ ] cs-ctl integration functional and intuitive
- [ ] Interactive features enable rapid exploration
- [ ] Web viewer loads graphs in <2 seconds

## Design Principles Alignment
- **Cognitive-Native:** Constraint visualization reflects capability system
- **Isolation by Default:** Policy analysis respects isolation boundaries
- **Security:** Impact analysis prevents unintended capability loss
- **Debuggability:** Interactive features enable understanding of complex graphs
