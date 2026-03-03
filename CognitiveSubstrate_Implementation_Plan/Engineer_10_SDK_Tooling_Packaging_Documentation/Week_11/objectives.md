# Engineer 10 — SDK: Tooling, Packaging & Documentation — Week 11

## Phase: 1 (SDK Tooling & Debugging Infrastructure)

## Weekly Objective
Begin cs-top prototype. Design real-time dashboard showing all active CTs/Agents. Implement resource utilization metrics (memory, CPU, inference cost). Build data collection infrastructure.

## Document References
- **Primary:** Section 3.5.4 — cs-top: real-time view of all active CTs/Agents with resource utilization and cost
- **Supporting:** Section 6.3 — Phase 2, Week 20-24

## Deliverables
- [ ] cs-top architecture RFC (data collection, aggregation, visualization)
- [ ] Metrics collection library (memory, CPU, inference cost per CT/Agent)
- [ ] Time-series data store design (InfluxDB or equivalent)
- [ ] Real-time dashboard prototype (CLI-based: ncurses or equivalent)
- [ ] Data API for metrics retrieval
- [ ] Test suite with synthetic CT workloads
- [ ] cs-top command design and usage documentation

## Technical Specifications
### cs-top Dashboard Layout
```
Cognitive Substrate System Dashboard          Update: 100ms
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

System Summary:
  Active CTs: 47  |  Active Agents: 12  |  Total Cost/min: $2.34

CT List (Top by Cost):
PID    NAME                      STATE       MEM(MB)  CPU%  COST($)  PHASE
1001   agent-research            running     512      45%   0.042    inference
1002   ct-summarize              suspended   256      0%    0.021    idle
1003   ct-code-gen               running     1024     78%   0.089    tool_invoke
...

Agent Summary:
NAME          REQUESTS  AVG_TIME(ms)  TOTAL_COST($)  EFFICIENCY
researcher    145       250           1.250          0.92
assistant     89        180           0.745          0.88
```

### Metrics Collection
Per CT/Agent:
- Memory usage (current, peak)
- CPU utilization (%)
- Inference cost ($)
- Tool latency (ms)
- TPC utilization (%)
- Current phase (init, inference, tool_invoke, idle, suspended)
- Execution time (ms)

## Dependencies
- **Blocked by:** Week 06 CI/CD, basic metrics infrastructure
- **Blocking:** Week 12 cs-top refinement, Week 23-24 all tools integrated

## Acceptance Criteria
- [ ] Dashboard displays 12+ metrics per CT
- [ ] Real-time updates with <500ms latency
- [ ] Handles 100+ concurrent CTs without slowdown
- [ ] Memory overhead <5% of traced system
- [ ] Data API documented with example queries
- [ ] Synthetic test workload runs 5000+ CT operations

## Design Principles Alignment
- **Cognitive-Native:** Metrics reflect cognitive resource model (cost, TPC, phase)
- **Debuggability:** Real-time dashboard enables rapid root cause analysis
- **Cost Transparency:** Cost metrics visible per CT and Agent
- **System Visibility:** All active CTs and Agents visible in single pane
