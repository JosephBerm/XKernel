# cs-capgraph — Capability Graph Visualizer & Analyzer

**Inspect and visualize capability delegation chains and isolation boundaries**

## Overview

`cs-capgraph` provides tools to explore Cognitive Substrate's capability-based security model:

- **Capability Graph Visualization:** DOT/Graphviz rendering of all capability grants
- **Isolation Boundary Analysis:** Verify crew/agent isolation properties
- **Delegation Chain Tracing:** Follow capability provenance (who granted what, when)
- **Security Audit:** Detect over-granted capabilities or suspicious delegation patterns
- **What-If Analysis:** Simulate capability revocation or grant

```bash
$ cs-capgraph visualize agent-crew-xyz \
    --format graphviz \
    --output capability-graph.dot

# Generates interactive graph showing:
#   Nodes: Agents, with capability sets
#   Edges: Delegation chains (solid=active, dashed=revoked)
#   Colors: Capability types (memory=blue, tool=red, etc.)
```

## Features

### 1. Capability Graph Visualization

```bash
# Visualize agent's capability set
cs-capgraph visualize agent-abc --format graphviz

Output (graph.dot):

  digraph CapabilityGraph {
    rankdir=LR;

    node_root [label="Root\n(AllCapabilities)", style=filled, fillcolor=red];
    node_agent_a [label="Agent-A\n(4 capabilities)", style=filled, fillcolor=blue];
    node_agent_b [label="Agent-B\n(2 capabilities)", style=filled, fillcolor=green];

    node_root -> node_agent_a [label="cap_grant", color=green, style=solid];
    node_root -> node_agent_b [label="cap_grant", color=green, style=solid];
    node_agent_a -> node_agent_b [label="cap_delegate", color=orange, style=dashed];
  }

# Render to SVG
dot -Tsvg graph.dot -o graph.svg
open graph.svg
```

### 2. Delegation Chain Tracing

```bash
# Trace how Agent-B got a specific capability
cs-capgraph trace-delegation agent-b cap-read-memory-xyz

Output:
  Capability ID: cap-read-memory-xyz
  Type: ReadMemory(region=semantic_cache_xyz)
  Rights: {read, write}

  Delegation Chain:
    1. [2026-03-01 14:00:00] Root → Agent-A
       Rights: {read, write, delegate}
       Status: active
       Expires: never

    2. [2026-03-01 14:15:23] Agent-A → Agent-B
       Rights: {read, write}  (delegate right NOT delegated)
       Status: active
       Expires: 2026-03-02 14:15:23 (24h)

  Provenance: Root → Agent-A → Agent-B (chain length: 2)
  Audit: All delegations logged and cryptographically signed
```

### 3. Isolation Boundary Verification

```bash
# Verify crew isolation properties
cs-capgraph verify-isolation crew-xyz

Output:
  AgentCrew: crew-xyz
  Members: [Agent-A, Agent-B, Agent-C]
  Shared Memory: L2 cache (1 GB)

  Isolation Checks:

  ✓ No cross-crew memory access
    Agent-A cannot read crew-other L2 memory
    (no ReadMemory capability granted for crew-other regions)

  ✓ Crew-exclusive channels
    All inter-crew channels require explicit capability
    3 channels verified

  ✗ WARNING: Potential over-delegation
    Agent-B has InvokeTool(GPT-4) capability granted to Agent-C
    (but Agent-C is not a crew member)
    Recommendation: Revoke or document exception

  ✓ Deadline enforcement
    Watchdog enabled on all member CTs

  Summary: 4/5 checks passed (1 warning)
```

### 4. Security Audit Report

```bash
# Generate security audit report
cs-capgraph audit agent-crew-xyz

Output:
  Audit Report: crew-xyz
  Timestamp: 2026-03-01 14:30:00
  Auditor: admin

  ╔════════════════════════════════════════════════════════════╗
  ║ CAPABILITY STATISTICS                                      ║
  ╠════════════════════════════════════════════════════════════╣
  ║ Total capabilities:                            127          ║
  ║ Active:                                        118 (93%)    ║
  ║ Revoked:                                         9 (7%)     ║
  ║ Expired:                                         0 (0%)     ║
  ║ Time-limited:                                   34 (27%)    ║
  ║ Delegation chains (avg length):                 2.3        ║
  ║ Max delegation depth:                           5          ║
  ╚════════════════════════════════════════════════════════════╝

  ╔════════════════════════════════════════════════════════════╗
  ║ CAPABILITY TYPES GRANTED                                   ║
  ╠════════════════════════════════════════════════════════════╣
  ║ ReadMemory:        54 (43%)                                ║
  ║ WriteMemory:       28 (22%)                                ║
  ║ InvokeTool:        32 (25%)                                ║
  ║ SendChannel:       10 (8%)                                 ║
  ║ DelegateCapability: 3 (2%)                                 ║
  ╚════════════════════════════════════════════════════════════╝

  ╔════════════════════════════════════════════════════════════╗
  ║ ANOMALIES DETECTED                                         ║
  ╠════════════════════════════════════════════════════════════╣
  ║ 1. Agent-B granted InvokeTool(GPT-4) to Agent-D           ║
  ║    (not a crew member—potential security risk)             ║
  ║    Recommendation: Revoke or document exception            ║
  ║                                                             ║
  ║ 2. Agent-C has DelegateCapability rights (rare)            ║
  ║    (allows Agent-C to further delegate—monitor)            ║
  ║    Recommendation: Audit delegation chain                  ║
  ║                                                             ║
  ║ 3. 5 capabilities expiring in 1 hour                       ║
  ║    (Agent-A: 3, Agent-B: 2)                                ║
  ║    Recommendation: Renew before expiration                 ║
  ╚════════════════════════════════════════════════════════════╝

  Overall Risk Level: LOW (1 warning, 2 advisories)
```

### 5. What-If Analysis

```bash
# Simulate revoking a capability and check impact
cs-capgraph what-if revoke cap-invoke-tool-gpt4 --agent agent-b

Output:
  Simulating: Revoke InvokeTool(GPT-4) from Agent-B

  Affected CTs:
    - CT-1 (currently running) → will fail on next tool invocation
    - CT-5 (queued) → cannot start
    - CT-10 (completed) → no impact

  Affected delegation chains:
    - Agent-B → Agent-C: broken (C loses capability)
    - Agent-B → Agent-D: broken (D loses capability)

  Side effects:
    - 3 CTs affected
    - 2 delegation chains broken
    - 1 queued task blocked

  Recommendation: Coordinate revocation; notify affected agents
```

### 6. Policy Checking

```bash
# Check if capability grant violates mandatory policies
cs-capgraph check-policy --grantee agent-b --capability InvokeTool(shell)

Output:
  Capability: InvokeTool(shell)
  Grantee: Agent-B

  Policy Check Results:

  ✗ DENIED: MandatoryCapabilityPolicy::NoToolType(shell)
    Rule: Shell execution forbidden in production
    Reason: Command execution poses security risk

  Recommendation: Grant InvokeTool(GPT-4) instead (allowed by policy)
```

## Usage

### Basic Commands

```bash
# List all capabilities in a crew
cs-capgraph list crew-xyz

# Visualize capability graph
cs-capgraph visualize crew-xyz --format graphviz > graph.dot
dot -Tsvg graph.dot -o graph.svg

# Trace delegation chain
cs-capgraph trace-delegation agent-b cap-read-memory-xyz

# Verify isolation
cs-capgraph verify-isolation crew-xyz

# Security audit
cs-capgraph audit agent-crew-xyz --output json > audit-report.json
```

### Advanced Analysis

```bash
# Detect capability over-granting
cs-capgraph detect-over-grant crew-xyz

Output:
  Analyzing 127 capabilities for potential over-granting...

  Agent-B:
    ✗ Granted DelegateCapability (rare, 3 in entire system)
    ✗ Granted InvokeTool for 12 different tools (unusual)
    → Recommendation: Verify business case

  Agent-C:
    ✗ Has read access to 45 memory regions (high breadth)
    → Recommendation: Consider role-based access control

# Find capability delegation chains with high depth
cs-capgraph find-deep-chains crew-xyz --min-depth 4

Output:
  Chain 1: Root → Agent-A → Agent-B → Agent-C → Agent-D
    (length: 4, all active, rights preserved)

  Chain 2: Root → Agent-B → Agent-C → Agent-D
    (length: 3, but rights subset at each step)

  Recommendation: Chains >3 increase audit difficulty
```

### Temporal Analysis

```bash
# Show capability grant/revoke timeline
cs-capgraph timeline crew-xyz --format csv > timeline.csv

Output (CSV):
  timestamp,event,grantee,capability,grantor,action
  2026-03-01T14:00:00,grant,Agent-A,AllCapabilities,Root,active
  2026-03-01T14:15:23,grant,Agent-B,ReadMemory(cache),Agent-A,active
  2026-03-01T15:30:45,grant,Agent-C,InvokeTool(GPT-4),Agent-A,active
  2026-03-01T16:45:12,revoke,Agent-B,ReadMemory(cache),-,revoked
  2026-03-02T14:15:23,revoke,Agent-B,ReadMemory(cache),-,expired

# Visualize in timeline
python plot_timeline.py timeline.csv
```

## Architecture

### Capability Graph Representation

**Graph Structure:**

```
Nodes:
  - AgentID (source/target of grants)
  - CapabilityID (capability being delegated)

Edges:
  - DelegationEdge(from_agent, to_agent, cap_id, timestamp, status)

Properties:
  - Immutable log: all delegation events recorded
  - Audit trail: cryptographic signatures on edges
  - Revocation set: tracks revoked capabilities
```

### Query Engine

1. **Capability Lookup:** O(log n) via BTreeMap<AgentID, CapabilitySet>
2. **Delegation Tracing:** DFS on delegation graph O(n)
3. **Isolation Verification:** Check for cross-boundary edges O(n)
4. **Policy Checking:** Validate against MandatoryCapabilityPolicy O(1)

## Implementation Details

**See:** `/sessions/youthful-vigilant-albattani/mnt/XKernal/sdk/tools/cs-capgraph/src/`

- `main.rs` — CLI entry point
- `graph_loader.rs` — Load capability graph from kernel
- `renderer.rs` — DOT/Graphviz output
- `analyzer.rs` — Isolation, over-grant detection
- `tracer.rs` — Delegation chain tracing
- `auditor.rs` — Security audit report generation

## Use Cases

### Case 1: Audit Crew Creation

```bash
# New crew created: verify security posture
cs-capgraph audit crew-new-project --output json

# Generate report for security review
# Check: Are capabilities properly scoped? Isolation enforced?
```

### Case 2: Diagnose Capability Denial

```bash
# Agent-B failed with CapabilityDenied
# Check what capabilities it has
cs-capgraph list agent-b

# Compare with what it needs
# Was capability revoked? Expired? Never granted?
cs-capgraph trace-delegation agent-b cap-xyz

# Manually revoke if needed
cs-capgraph revoke agent-b cap-xyz
```

### Case 3: Off-board Agent Safely

```bash
# Agent leaving crew: remove all capabilities
cs-capgraph list agent-leaving

# Plan revocation
cs-capgraph what-if revoke-all agent-leaving

# Execute revocation (one per capability)
for cap in $(cs-capgraph list agent-leaving --format json | jq '.capabilities[]'); do
  cs-capgraph revoke agent-leaving $cap
done
```

### Case 4: Policy Compliance Check

```bash
# Is crew compliant with company security policy?
cs-capgraph audit crew-xyz --policy company-security-v1

# Output: List any capability grants violating policy
# E.g., "NoToolType(shell)" violations, over-delegation, etc.
```

## Integration with Other Tools

```bash
# cs-capgraph → cs-top
# Monitor capability usage in real-time
cs-top --show capabilities --graph crew-xyz

# cs-capgraph → visualization tools
# Export to external tools
cs-capgraph visualize crew-xyz --format graphviz | circo -Tsvg > circular-layout.svg
cs-capgraph visualize crew-xyz --format json | python visualize_capability_heatmap.py
```

## Related Tools

- **cs-trace** — Trace CSCI syscalls (includes capability syscalls)
- **cs-replay** — Replay tasks (debug capability issues)
- **cs-top** — Monitor real-time capability usage
- **cs-profile** — Profile capability delegation overhead

## Configuration

**Configuration File:** `~/.cs/capgraph-config.toml`

```toml
[graph]
# Refresh frequency (how often to poll kernel for latest graph)
refresh_interval_seconds = 5

# Maximum graph size (nodes)
max_nodes = 10000

[visualization]
# Default output format
format = "graphviz"  # or json, csv, text

# Graphviz rendering engine
engine = "dot"  # or circo, neato, fdp, sfdp

# Color scheme for capability types
colors = {
  ReadMemory = "blue",
  WriteMemory = "red",
  InvokeTool = "orange",
  SendChannel = "green",
  DelegateCapability = "purple"
}

[audit]
# Risk level thresholds
over_grant_threshold = 0.7  # Agent has 70%+ of all capabilities
deep_chain_threshold = 4    # Delegation chain depth >4
```

## Limitations

1. **Real-Time Consistency:** Graph is eventually consistent (may lag kernel state by ~1s)
2. **Large Graphs:** Very large capability graphs (>10k nodes) slow to visualize
3. **Graphviz Size Limits:** SVG rendering has practical limits (dense graphs hard to read)

## Roadmap

- [ ] Interactive web UI for capability graph exploration
- [ ] Heatmap of capability grant patterns (find anomalies)
- [ ] Machine learning-based anomaly detection
- [ ] Automatic capability suggestion (based on task requirements)
- [ ] Compliance reporting (GDPR, HIPAA, SOC2 policy checks)

## See Also

- **Engineering Plan v2.5:** Section 1.2 — "Capability-Based Security from Day Zero"
- **Domain Model Deep Dive:** Section 5 — Capability Graph & Isolation Boundaries
- **Best Practices:** Section 2.3 — Invariant Enforcement

---

**Status:** Design document (implementation deferred to Week 08+)
**Estimated Implementation:** 300 lines Rust (graph loading) + 400 lines Rust (analysis)
