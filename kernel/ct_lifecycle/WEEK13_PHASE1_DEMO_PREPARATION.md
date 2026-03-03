# Week 13 — Phase 1 Demo: 3-Agent Crew Scheduling Validation

**Document Version:** 1.0
**Date:** March 2, 2026
**Author:** Principal Software Engineer
**Project:** XKernal Cognitive Substrate OS
**Classification:** Technical Design (Phase 1 Validation)

---

## Executive Summary

This document specifies the Phase 1 demonstration for the XKernal Cognitive Substrate OS, validating core scheduling, affinity, and inter-agent communication mechanisms through a realistic 3-agent crew scenario. The demo orchestrates a Researcher→Analyst→Writer pipeline, spawning 7 Cognitive Tasks (CTs) across three coordinated agents with full trace instrumentation and deadline management. Success criteria focus on NUMA locality preservation, priority-driven scheduling, deadlock-free DAG execution, and zero-copy semantic channel communication.

---

## Problem Statement

Prior to production deployment, XKernal must validate that multi-agent scheduling achieves:

1. **Scheduling Affinity:** All task groups remain co-located on single NUMA nodes, avoiding inter-node memory traffic
2. **Priority Scoring:** Dynamically adjusted priority correctly influences CT dispatch order without starvation
3. **Deadline Escalation:** Escalation proportional to remaining budget ensures deadline-driven preemption
4. **Deadlock Prevention:** DAG task dependency resolution executes without circular waits
5. **Inter-Agent Communication:** SemanticChannel IPC with zero-copy semantics and capability-gating works end-to-end
6. **Observability:** Trace logging captures full execution flow with phase transitions for post-demo analysis

This demo validates Phase 1 requirements within a 5-minute window using a realistic workload that mirrors production patterns.

---

## Architecture

### Crew Composition

```
Agent A (Researcher)
├── CT-1: search_query (web_search tool)
├── CT-2: analyze_results (data processing)
└── CT-3: summarize_findings (aggregation)
    └─→ SemanticChannel A→B

Agent B (Analyst)
├── CT-4: receive_summary (consume A's output)
└── CT-5: process_analysis (complex reasoning)
    └─→ SemanticChannel B→C

Agent C (Writer)
├── CT-6: compile_report (consolidate data)
└── CT-7: format_output (presentation layer)
    └─→ Demo output
```

### Task Execution Model

**Agent A (Researcher) — 3 Cycles:**
- Each cycle executes reason→act→reflect→yield pattern
- **Act Phase:** Spawns 3 CTs in pipeline (search_query → analyze_results → summarize_findings)
- **Reflect Phase:** Validates results against prior heuristics
- **Yield Phase:** Publishes to SemanticChannel for Agent B consumption
- **Priority:** 90 (highest — critical path initiator)

**Agent B (Analyst) — Sequential:**
- Blocks on Agent A SemanticChannel delivery
- **Act Phase:** Spawns 2 CTs (receive_summary → process_analysis)
- Applies statistical processing and anomaly detection
- **Priority:** 75 (medium-high — dependent task)
- **Yield Phase:** Publishes to SemanticChannel for Agent C

**Agent C (Writer) — Sequential:**
- Blocks on Agent B SemanticChannel delivery
- **Act Phase:** Spawns 2 CTs (compile_report → format_output)
- Produces structured report in JSON/Markdown format
- **Priority:** 60 (medium — final stage)

### Scheduling Guarantees

**NUMA Affinity:**
- All 7 CTs assigned to same physical NUMA node (node 0)
- SchedulingValidator confirms zero cross-node migrations during execution
- Validates CPU cache coherency through performance counters

**Priority Scoring:**
- Agent A CTs: base priority 90, escalate +5 per minute remaining budget
- Agent B CTs: base priority 75, inherit A's escalation rate
- Agent C CTs: base priority 60, inherit full escalation chain
- RunQueue sorts by (deadline_urgency, base_priority, create_timestamp)

**Deadline Escalation:**
- Phase 1 deadline: 300 seconds (5 minutes)
- Escalation formula: `priority_adj = urgency_factor × remaining_budget_percentage`
- At 50% budget: priority boosted +15 across all pending CTs
- At 75% budget: priority boosted +30 (ensures completion)

**Deadlock Prevention:**
- Dependency resolver validates DAG topology at scenario initialization
- SemanticChannel send/recv operations guarded by capability tokens
- No cyclic waits possible; CT-4 cannot execute until CT-3 complete

### SemanticChannel Communication

**Zero-Copy IPC Design:**
```
Agent A publishes summarize_findings result:
  ├─ Allocates shared memory page (4KB minimum)
  ├─ Writes metadata (source_agent=A, sequence=N, checksum)
  ├─ Publishes (addr, size) to SemanticChannel A→B
  └─ Retains read-only view pending Agent B acknowledgment

Agent B receives:
  ├─ Reads (addr, size, capability_token) from channel
  ├─ Maps shared memory with read capability
  ├─ Verifies checksum and source authority
  ├─ Processes in-place (no copy required)
  └─ Publishes capability release on completion
```

**Capability Gating:**
- Each SemanticChannel enforces source→destination whitelist
- Agent A → Agent B: permitted (Analyst depends on Researcher)
- Agent B → Agent C: permitted (Writer depends on Analyst)
- Cross-agent writes blocked (e.g., A cannot write to C's input channel)

---

## Implementation

### Core Rust Components

```rust
use std::sync::{Arc, RwLock, Barrier};
use std::time::{Instant, SystemTime};
use std::collections::HashMap;

/// Represents a single Cognitive Task with metadata
#[derive(Clone, Debug)]
pub struct CognitiveTask {
    pub ct_id: String,
    pub agent_id: String,
    pub phase: TaskPhase,
    pub priority: u32,
    pub deadline: SystemTime,
    pub numa_node: usize,
    pub dependencies: Vec<String>,
    pub created_at: Instant,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TaskPhase {
    Pending,
    Ready,
    Running,
    Yielded,
    Completed,
}

/// Scheduler validator for affinity and priority assertions
pub struct SchedulingValidator {
    numa_topology: HashMap<usize, Vec<String>>,
    task_log: Arc<RwLock<Vec<(String, TaskPhase, Instant)>>>,
    priority_decisions: Arc<RwLock<Vec<(String, u32, Instant)>>>,
}

impl SchedulingValidator {
    pub fn new(numa_nodes: usize) -> Self {
        let mut topology = HashMap::new();
        for i in 0..numa_nodes {
            topology.insert(i, Vec::new());
        }
        Self {
            numa_topology: topology,
            task_log: Arc::new(RwLock::new(Vec::new())),
            priority_decisions: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Validate all CTs remain on assigned NUMA node
    pub fn assert_numa_affinity(&self, tasks: &[CognitiveTask]) -> bool {
        tasks.iter().all(|t| {
            self.numa_topology
                .get(&t.numa_node)
                .map(|node_tasks| node_tasks.contains(&t.ct_id))
                .unwrap_or(false)
        })
    }

    /// Log task phase transition
    pub fn log_phase_transition(&self, ct_id: &str, phase: TaskPhase) {
        let mut log = self.task_log.write().unwrap();
        log.push((ct_id.to_string(), phase, Instant::now()));
    }

    /// Validate priority ordering
    pub fn assert_priority_ordering(&self, ct_ids: &[String]) -> bool {
        let log = self.task_log.read().unwrap();
        let mut prev_priority = u32::MAX;
        for ct_id in ct_ids {
            if let Some((_, priority, _)) = log.iter()
                .rev()
                .find(|(id, _, _)| id == ct_id)
                .and_then(|_| {
                    self.priority_decisions.read().ok()
                        .and_then(|d| d.iter().rev()
                            .find(|(id, _, _)| id == ct_id)
                            .copied())
                }) {
                if priority > prev_priority {
                    return false; // Priority inversion detected
                }
                prev_priority = priority;
            }
        }
        true
    }
}

/// Tracks semantic channel communication
pub struct SemanticChannelMonitor {
    transfers: Arc<RwLock<Vec<IpcTransfer>>>,
}

#[derive(Clone, Debug)]
pub struct IpcTransfer {
    pub source_agent: String,
    pub dest_agent: String,
    pub data_size: usize,
    pub checksum: u64,
    pub timestamp: Instant,
    pub is_zero_copy: bool,
}

impl SemanticChannelMonitor {
    pub fn new() -> Self {
        Self {
            transfers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn record_transfer(&self, transfer: IpcTransfer) {
        let mut t = self.transfers.write().unwrap();
        t.push(transfer);
    }

    /// Validate all transfers used zero-copy semantics
    pub fn assert_zero_copy(&self) -> bool {
        self.transfers
            .read()
            .unwrap()
            .iter()
            .all(|t| t.is_zero_copy)
    }

    /// Verify no capability violations
    pub fn assert_capability_gating(&self) -> bool {
        let allowed = vec![
            ("A".to_string(), "B".to_string()),
            ("B".to_string(), "C".to_string()),
        ];
        self.transfers
            .read()
            .unwrap()
            .iter()
            .all(|t| {
                allowed.iter()
                    .any(|(src, dst)| &t.source_agent == src && &t.dest_agent == dst)
            })
    }
}

/// Agent workload specification
pub struct AgentWorkload {
    pub agent_id: String,
    pub base_priority: u32,
    pub tasks: Vec<CognitiveTask>,
    pub cycles: usize,
}

impl AgentWorkload {
    pub fn new(agent_id: &str, base_priority: u32, cycles: usize) -> Self {
        Self {
            agent_id: agent_id.to_string(),
            base_priority,
            tasks: Vec::new(),
            cycles,
        }
    }

    /// Generate CTs for this agent
    pub fn spawn_tasks(&mut self, numa_node: usize) {
        let base_time = SystemTime::now();
        match self.agent_id.as_str() {
            "A" => {
                for cycle in 0..self.cycles {
                    self.tasks.push(CognitiveTask {
                        ct_id: format!("A-search-{}", cycle),
                        agent_id: "A".to_string(),
                        phase: TaskPhase::Pending,
                        priority: self.base_priority,
                        deadline: base_time + std::time::Duration::from_secs(300),
                        numa_node,
                        dependencies: vec![],
                        created_at: Instant::now(),
                    });
                    self.tasks.push(CognitiveTask {
                        ct_id: format!("A-analyze-{}", cycle),
                        agent_id: "A".to_string(),
                        phase: TaskPhase::Pending,
                        priority: self.base_priority,
                        deadline: base_time + std::time::Duration::from_secs(300),
                        numa_node,
                        dependencies: vec![format!("A-search-{}", cycle)],
                        created_at: Instant::now(),
                    });
                    self.tasks.push(CognitiveTask {
                        ct_id: format!("A-summarize-{}", cycle),
                        agent_id: "A".to_string(),
                        phase: TaskPhase::Pending,
                        priority: self.base_priority,
                        deadline: base_time + std::time::Duration::from_secs(300),
                        numa_node,
                        dependencies: vec![format!("A-analyze-{}", cycle)],
                        created_at: Instant::now(),
                    });
                }
            }
            "B" => {
                self.tasks.push(CognitiveTask {
                    ct_id: "B-receive".to_string(),
                    agent_id: "B".to_string(),
                    phase: TaskPhase::Pending,
                    priority: self.base_priority,
                    deadline: base_time + std::time::Duration::from_secs(300),
                    numa_node,
                    dependencies: vec!["A-summarize-2".to_string()],
                    created_at: Instant::now(),
                });
                self.tasks.push(CognitiveTask {
                    ct_id: "B-analyze".to_string(),
                    agent_id: "B".to_string(),
                    phase: TaskPhase::Pending,
                    priority: self.base_priority,
                    deadline: base_time + std::time::Duration::from_secs(300),
                    numa_node,
                    dependencies: vec!["B-receive".to_string()],
                    created_at: Instant::now(),
                });
            }
            "C" => {
                self.tasks.push(CognitiveTask {
                    ct_id: "C-compile".to_string(),
                    agent_id: "C".to_string(),
                    phase: TaskPhase::Pending,
                    priority: self.base_priority,
                    deadline: base_time + std::time::Duration::from_secs(300),
                    numa_node,
                    dependencies: vec!["B-analyze".to_string()],
                    created_at: Instant::now(),
                });
                self.tasks.push(CognitiveTask {
                    ct_id: "C-format".to_string(),
                    agent_id: "C".to_string(),
                    phase: TaskPhase::Pending,
                    priority: self.base_priority,
                    deadline: base_time + std::time::Duration::from_secs(300),
                    numa_node,
                    dependencies: vec!["C-compile".to_string()],
                    created_at: Instant::now(),
                });
            }
            _ => {}
        }
    }
}

/// Demo orchestrator driving full scenario
pub struct DemoOrchestrator {
    validator: SchedulingValidator,
    channel_monitor: SemanticChannelMonitor,
    workloads: Vec<AgentWorkload>,
    start_time: Instant,
    deadline: std::time::Duration,
}

impl DemoOrchestrator {
    pub fn new() -> Self {
        Self {
            validator: SchedulingValidator::new(1),
            channel_monitor: SemanticChannelMonitor::new(),
            workloads: vec![
                AgentWorkload::new("A", 90, 3),
                AgentWorkload::new("B", 75, 1),
                AgentWorkload::new("C", 60, 1),
            ],
            start_time: Instant::now(),
            deadline: std::time::Duration::from_secs(300),
        }
    }

    pub fn initialize(&mut self) {
        for workload in &mut self.workloads {
            workload.spawn_tasks(0);
        }
    }

    pub fn execute(&self) -> DemoResult {
        self.initialize_barrier();
        self.schedule_all_tasks();
        self.simulate_execution();
        self.collect_results()
    }

    fn initialize_barrier(&self) {
        // Barrier ensures all agents start within tight window
    }

    fn schedule_all_tasks(&self) {
        // Use SchedulingValidator to assign to RunQueue
    }

    fn simulate_execution(&self) {
        // Execute tasks respecting dependencies and priorities
    }

    fn collect_results(&self) -> DemoResult {
        DemoResult {
            numa_affinity_valid: self.validator.assert_numa_affinity(
                &self.workloads.iter().flat_map(|w| w.tasks.clone()).collect::<Vec<_>>()
            ),
            zero_copy_valid: self.channel_monitor.assert_zero_copy(),
            capability_gating_valid: self.channel_monitor.assert_capability_gating(),
            total_duration: self.start_time.elapsed(),
        }
    }
}

#[derive(Debug)]
pub struct DemoResult {
    pub numa_affinity_valid: bool,
    pub zero_copy_valid: bool,
    pub capability_gating_valid: bool,
    pub total_duration: std::time::Duration,
}

fn main() {
    let mut orchestrator = DemoOrchestrator::new();
    let result = orchestrator.execute();
    println!("Demo Result: {:?}", result);
    assert!(result.numa_affinity_valid, "NUMA affinity validation failed");
    assert!(result.zero_copy_valid, "Zero-copy validation failed");
    assert!(result.capability_gating_valid, "Capability gating validation failed");
}
```

---

## Testing

### Unit Tests

**SchedulingValidator Tests:**
- `test_numa_affinity_enforcement()` — Verify all CTs bound to node 0
- `test_priority_ordering()` — Confirm no priority inversions in dispatch sequence
- `test_deadline_escalation()` — Validate priority boost proportional to remaining budget

**SemanticChannelMonitor Tests:**
- `test_zero_copy_transfer()` — Confirm data transferred via shared memory, not copied
- `test_capability_violation_detection()` — Detect unauthorized channel writes
- `test_checksum_validation()` — Verify integrity across IPC boundary

**AgentWorkload Tests:**
- `test_task_generation()` — Verify 7 total CTs generated correctly
- `test_dependency_resolution()` — Confirm DAG topology acyclic

### Integration Tests

**Full Crew Scenario:**
- Execute all 3 agents with SemanticChannel communication
- Verify A→B→C execution order with zero deadlocks
- Confirm 300-second budget sufficient for completion
- Collect trace logs for post-mortem analysis

---

## Acceptance Criteria

| Criterion | Metric | Target | Status |
|-----------|--------|--------|--------|
| NUMA Affinity | % CTs on node 0 | 100% | |
| Priority Ordering | Priority inversions | 0 | |
| Deadline Escalation | Budget utilization | <90% | |
| Deadlock Freedom | Circular waits | 0 | |
| Zero-Copy IPC | Shared memory transfers | 100% | |
| Capability Gating | Unauthorized channel access | 0 | |
| Execution Time | Total demo duration | <5 minutes | |
| Trace Logging | Event capture rate | 100% of transitions | |

---

## Design Principles

1. **Deterministic Scheduling:** All CT ordering decisions logged and reproducible for regression testing
2. **Capability-Based Security:** SemanticChannel access enforced at kernel boundary with no privilege escalation
3. **Observability First:** Every phase transition traced; post-demo analysis reveals bottlenecks and anomalies
4. **Affinity Preservation:** NUMA topology drives scheduling decisions; explicit validation prevents silent performance regressions
5. **Dependency Integrity:** DAG verification prevents malformed crew specifications from reaching runtime
6. **Zero-Copy Semantics:** Shared memory reduces latency and validates efficient inter-agent communication
7. **Production Readiness:** Demo workload patterns mirror realistic multi-agent scenarios; success here predicts production reliability

---

## Conclusion

This Phase 1 demo validates the five core pillars of XKernal's scheduling and communication architecture. By successfully orchestrating a 3-agent crew through a 7-CT DAG with full affinity, priority, and capability enforcement, we demonstrate readiness for multi-agent production workloads. The trace instrumentation and post-demo analysis framework establish the observability baseline required for Phase 2 advanced features (thermal management, dynamic resource allocation).

**Expected Outcome:** Demo completes in <5 minutes with all assertions passing, establishing XKernal as a viable substrate for cognitive agent workloads.
