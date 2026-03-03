# Engineer 10 — SDK: Tooling, Packaging & Documentation — Week 23

## Phase: 2 (Advanced Debugging Tools & Registry)

## Weekly Objective
Final stabilization of Phase 2. Complete all tool integrations and resolve open issues. Prepare comprehensive documentation and examples. Begin Phase 3 documentation portal planning.

## Document References
- **Primary:** Section 6.3 — Phase 2, Week 20-24
- **Supporting:** Section 3.5.6 (Documentation Portal design for Phase 3)

## Deliverables
- [ ] Bug triage and resolution for all Phase 2 components
- [ ] End-to-end integration tests (all tools working together)
- [ ] Comprehensive example scripts demonstrating all tools
- [ ] Troubleshooting guide for common issues
- [ ] Performance optimization for registry and debugging tools
- [ ] Documentation portal structure and content planning
- [ ] Checklist for Phase 3 documentation tasks

## Technical Specifications
### Example Scripts
```bash
#!/bin/bash
# example_debug_session.sh - Complete debugging workflow

CT_ID=1001

# 1. Monitor system with cs-top
echo "=== Monitoring system ==="
cs-ctl top &
TOP_PID=$!

# 2. Trace CT syscalls
echo "=== Tracing CT syscalls ==="
cs-ctl trace $CT_ID --output json > trace.json &

# 3. After CT failure, replay core dump
echo "=== Replaying failed CT ==="
cs-ctl replay core_dump_$CT_ID.cscd

# 4. Profile cost and performance
echo "=== Profiling agent ==="
cs-ctl profile agent_001 --export json > profile.json

# 5. Visualize capability graph
echo "=== Visualizing capability graph ==="
cs-ctl capgraph show agent_001 --export graphml > capgraph.graphml

# 6. Cleanup
kill $TOP_PID
```

### Troubleshooting Guide
```markdown
## Common Issues

### cs-trace: "Failed to attach to CT"
- Check if CT is still running: cs-ctl top | grep 1001
- Verify capabilities: cs-ctl capgraph show agent_001
- Check isolation boundaries

### cs-replay: "Core dump corrupted"
- Verify checksum: sha256sum core_dump.cscd
- Check available disk space
- Ensure CSCI version matches

### cs-profile: "Missing cost metrics"
- Verify cost accounting enabled in agent
- Check tool integration (web_search, etc.)
- Review cs-profile log: journalctl -u cs-profile
```

### Integration Test Suite
```
Phase 2 Integration Tests:
├── cs-trace:
│   ├── test_attach_running_ct
│   ├── test_syscall_filtering
│   └── test_multiple_ct_tracing
├── cs-replay:
│   ├── test_replay_inference_failure
│   ├── test_stepping_functionality
│   └── test_core_dump_compression
├── cs-profile:
│   ├── test_cost_attribution
│   ├── test_tool_breakdown
│   └── test_optimization_recommendations
├── cs-capgraph:
│   ├── test_graph_construction
│   ├── test_delegation_chains
│   └── test_isolation_boundaries
├── cs-top:
│   ├── test_metrics_collection
│   ├── test_dashboard_rendering
│   └── test_concurrent_agents
└── cs-pkg:
    ├── test_package_publish
    ├── test_package_install
    └── test_registry_search
```

## Dependencies
- **Blocked by:** Week 15-22 all Phase 2 implementations
- **Blocking:** Week 24 final Phase 2 wrap-up

## Acceptance Criteria
- [ ] All Phase 2 components pass integration tests
- [ ] Example scripts run without errors
- [ ] Troubleshooting guide addresses 80% of real issues
- [ ] Documentation portal structure complete and reviewed
- [ ] Zero critical bugs in Phase 2 tools
- [ ] Phase 3 checklist ready for team distribution

## Design Principles Alignment
- **Cognitive-Native:** Examples demonstrate real cognitive workloads
- **Debuggability:** Troubleshooting guide enables operators to solve problems independently
- **Documentation:** Examples and guides critical for adoption
- **Quality:** Integration tests ensure tools work together correctly
