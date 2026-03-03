# Week 11 — cs-top: Real-Time Cognitive System Dashboard Prototype

**Author:** Principal Software Engineer
**Date:** Week 11, XKernal Cognitive Substrate OS Project
**Status:** Technical Design Document
**Scope:** Real-time monitoring dashboard for cognitive system resource utilization and cost metrics

---

## Executive Summary

cs-top is a production-grade real-time monitoring dashboard that provides visibility into all active Cognitive Tasks (CTs) and Agents running on the XKernal Cognitive Substrate OS. The prototype implements a high-performance metrics collection, aggregation, and visualization pipeline capable of handling 100+ concurrent CTs with <500ms dashboard refresh latency and <5% memory overhead. This document specifies the architecture, implementation strategy, and acceptance criteria for the Week 11 prototype delivery.

---

## Problem Statement

Current XKernal deployments lack real-time observability into cognitive system execution. Engineering teams cannot:
- Monitor active CTs and Agents across distributed cognitive workloads
- Quantify per-CT resource consumption (memory, CPU, inference cost)
- Correlate execution phases with performance degradation
- Identify cost anomalies in agent-based services
- Troubleshoot latency issues without post-mortem analysis

cs-top addresses these gaps by providing a `top`-inspired CLI dashboard specifically designed for cognitive system metrics, enabling operators to make real-time decisions about resource allocation and cost optimization.

---

## Architecture

### Data Collection → Aggregation → Visualization Pipeline

```
┌─────────────────────────────────────────────────────────────┐
│                     METRICS COLLECTION LAYER                │
│  (Per-CT Memory, CPU%, Inference Cost, Tool Latency, TPC)   │
└─────────────────────────┬───────────────────────────────────┘
                          │
┌─────────────────────────v───────────────────────────────────┐
│                   TIME-SERIES DATA STORE                     │
│   (Ring buffer: 1-minute sliding window @ 500ms intervals)   │
└─────────────────────────┬───────────────────────────────────┘
                          │
┌─────────────────────────v───────────────────────────────────┐
│                      METRICS API SERVER                      │
│           (JSON endpoints: /metrics, /ct/{id}, /agents)      │
└─────────────────────────┬───────────────────────────────────┘
                          │
┌─────────────────────────v───────────────────────────────────┐
│                     NCURSES DASHBOARD                        │
│     (Real-time rendering with interactive sorting/filtering)  │
└─────────────────────────────────────────────────────────────┘
```

### Dashboard Layout (ncurses CLI)

**Header Section:**
```
XKernal cs-top [Refresh: 500ms] [Active CTs: 47] [Active Agents: 12] [Total Cost: $2.34/min]
═══════════════════════════════════════════════════════════════════════════════════════

System Summary:
  CTs Running: 47 | Peak Memory: 2.4GB | Avg CPU: 34% | Est. Monthly Cost: $33,696
```

**CT List Table (Top by Cost):**
```
  PID     NAME                    STATE        MEM(MB)  CPU%   COST($)   PHASE
  ─────────────────────────────────────────────────────────────────────────────
  1024    inference_claude_v2     INFERENCE    384.2    67%    0.0047    encode
  1025    inference_gpt4_batch    INFERENCE    512.1    72%    0.0043    forward
  1026    agent_search_synthesis  RUNNING      256.3    45%    0.0021    tool_call
  1027    inference_embedding     PROCESSING   128.4    28%    0.0008    decode
  ...
```

**Agent Summary Section:**
```
Agent Performance:
  NAME                  REQUESTS   AVG_TIME(ms)  TOTAL_COST($)  EFFICIENCY(req/$)
  ────────────────────────────────────────────────────────────────────────────
  web_search_agent      1,247      245ms         $1.84         677.2
  synthesis_agent       834        892ms         $3.12         267.3
```

---

## Implementation

### Core Rust Structures

```rust
use std::collections::VecDeque;
use std::time::{SystemTime, Duration};
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};

/// Single metric sample with timestamp
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetricsSample {
    pub timestamp: SystemTime,
    pub memory_current_mb: f64,
    pub memory_peak_mb: f64,
    pub cpu_percent: f64,
    pub inference_cost_usd: f64,
    pub tool_latency_ms: f64,
    pub tpc_utilization_percent: f64,
    pub execution_phase: ExecutionPhase,
    pub execution_time_ms: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ExecutionPhase {
    Encode,
    Tokenize,
    Forward,
    Inference,
    Decode,
    ToolCall,
    ToolExecution,
    AggregateResults,
    Complete,
}

/// Per-CT metrics tracking
#[derive(Debug)]
pub struct CtMetrics {
    pub ct_id: String,
    pub ct_name: String,
    pub state: CtState,
    pub samples: VecDeque<MetricsSample>,
    pub max_samples: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CtState {
    Pending,
    Running,
    Inference,
    Processing,
    Blocked,
    Complete,
}

/// Per-Agent aggregated metrics
#[derive(Clone, Debug, Serialize)]
pub struct AgentMetrics {
    pub agent_name: String,
    pub total_requests: u64,
    pub avg_latency_ms: f64,
    pub total_cost_usd: f64,
    pub efficiency_req_per_dollar: f64,
    pub success_rate: f64,
}

/// Time-series store with ring-buffer design
pub struct TimeSeriesStore {
    ct_metrics: RwLock<std::collections::HashMap<String, CtMetrics>>,
    agent_aggregates: RwLock<std::collections::HashMap<String, AgentMetrics>>,
}

impl TimeSeriesStore {
    pub fn new(max_samples_per_ct: usize) -> Self {
        TimeSeriesStore {
            ct_metrics: RwLock::new(std::collections::HashMap::new()),
            agent_aggregates: RwLock::new(std::collections::HashMap::new()),
        }
    }

    pub fn record_ct_sample(&self, ct_id: &str, ct_name: &str,
                            sample: MetricsSample, state: CtState) {
        let mut metrics = self.ct_metrics.write();
        let entry = metrics.entry(ct_id.to_string())
            .or_insert_with(|| CtMetrics {
                ct_id: ct_id.to_string(),
                ct_name: ct_name.to_string(),
                state: state.clone(),
                samples: VecDeque::with_capacity(120),
                max_samples: 120,
            });

        entry.state = state;
        if entry.samples.len() >= entry.max_samples {
            entry.samples.pop_front();
        }
        entry.samples.push_back(sample);
    }

    pub fn get_top_ct_by_cost(&self, limit: usize) -> Vec<(String, f64)> {
        let metrics = self.ct_metrics.read();
        let mut costs: Vec<_> = metrics
            .values()
            .filter_map(|ct| {
                ct.samples.back().map(|s| (ct.ct_id.clone(), s.inference_cost_usd))
            })
            .collect();
        costs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        costs.into_iter().take(limit).collect()
    }

    pub fn get_system_summary(&self) -> SystemSummary {
        let metrics = self.ct_metrics.read();
        let total_memory = metrics.values()
            .filter_map(|ct| ct.samples.back().map(|s| s.memory_current_mb))
            .sum::<f64>();

        let avg_cpu = metrics.values()
            .filter_map(|ct| ct.samples.back().map(|s| s.cpu_percent))
            .sum::<f64>() / metrics.len().max(1) as f64;

        let total_cost_usd = metrics.values()
            .filter_map(|ct| ct.samples.back().map(|s| s.inference_cost_usd))
            .sum::<f64>();

        SystemSummary {
            active_ct_count: metrics.len(),
            total_memory_mb: total_memory,
            avg_cpu_percent: avg_cpu,
            total_cost_per_min_usd: total_cost_usd,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SystemSummary {
    pub active_ct_count: usize,
    pub total_memory_mb: f64,
    pub avg_cpu_percent: f64,
    pub total_cost_per_min_usd: f64,
}

/// Metrics collection from CT runtime
pub struct MetricsCollector {
    store: std::sync::Arc<TimeSeriesStore>,
}

impl MetricsCollector {
    pub fn new(store: std::sync::Arc<TimeSeriesStore>) -> Self {
        MetricsCollector { store }
    }

    pub fn collect_ct_metrics(&self, ct_id: &str, ct_name: &str) {
        // Integration point with XKernal runtime to fetch:
        // - Memory (resident/peak) from process accounting
        // - CPU% from /proc/stat or similar
        // - Inference cost from token counter instrumentation
        // - Tool latency from intercepted RPC calls
        // - TPC utilization from shared GPU metrics
        // - Execution phase from CT state machine

        let sample = MetricsSample {
            timestamp: SystemTime::now(),
            memory_current_mb: 256.0,
            memory_peak_mb: 512.0,
            cpu_percent: 45.0,
            inference_cost_usd: 0.0032,
            tool_latency_ms: 234.5,
            tpc_utilization_percent: 67.0,
            execution_phase: ExecutionPhase::Inference,
            execution_time_ms: 1200.0,
        };

        self.store.record_ct_sample(ct_id, ct_name, sample, CtState::Inference);
    }
}

/// Dashboard renderer (ncurses wrapper)
pub struct DashboardRenderer {
    store: std::sync::Arc<TimeSeriesStore>,
    refresh_interval_ms: u64,
    sort_field: SortField,
}

#[derive(Clone, Copy, Debug)]
pub enum SortField {
    Cost,
    Memory,
    Cpu,
    Phase,
}

impl DashboardRenderer {
    pub fn new(store: std::sync::Arc<TimeSeriesStore>, refresh_ms: u64) -> Self {
        DashboardRenderer {
            store,
            refresh_interval_ms: refresh_ms,
            sort_field: SortField::Cost,
        }
    }

    pub fn render(&self) -> String {
        let summary = self.store.get_system_summary();
        let top_cts = self.store.get_top_ct_by_cost(20);

        format!(
            "Active CTs: {} | Memory: {:.1}MB | CPU: {:.1}% | Cost: ${:.4}/min\n\
             Top CTs: {:?}",
            summary.active_ct_count, summary.total_memory_mb,
            summary.avg_cpu_percent, summary.total_cost_per_min_usd,
            top_cts
        )
    }
}
```

### Data API Endpoints

```rust
// Metrics API server (Axum/Tokio)
pub async fn metrics_handler(
    State(store): State<Arc<TimeSeriesStore>>
) -> Json<SystemSummary> {
    Json(store.get_system_summary())
}

pub async fn ct_metrics_handler(
    State(store): State<Arc<TimeSeriesStore>>,
    Path(ct_id): Path<String>,
) -> Json<Vec<MetricsSample>> {
    // Return time-series for specific CT
    Json(vec![])
}

pub async fn agents_handler(
    State(store): State<Arc<TimeSeriesStore>>
) -> Json<Vec<AgentMetrics>> {
    let agents = store.agent_aggregates.read();
    Json(agents.values().cloned().collect())
}
```

---

## Testing Strategy

### Synthetic Workload Generator
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;

    #[test]
    fn test_concurrent_ct_metrics_5000_ops() {
        let store = Arc::new(TimeSeriesStore::new(120));
        let mut handles = vec![];

        for ct_idx in 0..100 {
            let store_clone = Arc::clone(&store);
            let handle = std::thread::spawn(move || {
                let collector = MetricsCollector::new(store_clone);
                for op in 0..50 {
                    let ct_id = format!("ct_{}", ct_idx);
                    collector.collect_ct_metrics(&ct_id, "synthetic_task");
                    std::thread::sleep(Duration::from_millis(10));
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let summary = store.get_system_summary();
        assert_eq!(summary.active_ct_count, 100);
    }

    #[test]
    fn test_memory_overhead_under_5_percent() {
        let store = Arc::new(TimeSeriesStore::new(120));
        let initial_memory = estimate_memory_usage();

        for _ in 0..5000 {
            let sample = MetricsSample {
                timestamp: SystemTime::now(),
                memory_current_mb: 256.0,
                memory_peak_mb: 512.0,
                cpu_percent: 45.0,
                inference_cost_usd: 0.0032,
                tool_latency_ms: 234.5,
                tpc_utilization_percent: 67.0,
                execution_phase: ExecutionPhase::Inference,
                execution_time_ms: 1200.0,
            };
            store.record_ct_sample("ct_test", "test", sample, CtState::Running);
        }

        let final_memory = estimate_memory_usage();
        let overhead_percent = ((final_memory - initial_memory) / initial_memory) * 100.0;
        assert!(overhead_percent < 5.0, "Memory overhead: {:.2}%", overhead_percent);
    }

    #[test]
    fn test_dashboard_refresh_under_500ms() {
        let store = Arc::new(TimeSeriesStore::new(120));
        let renderer = DashboardRenderer::new(Arc::clone(&store), 500);

        let start = SystemTime::now();
        let _output = renderer.render();
        let elapsed = start.elapsed().unwrap().as_millis();
        assert!(elapsed < 500, "Render time: {}ms", elapsed);
    }
}
```

---

## Acceptance Criteria

- **AC1**: cs-top displays system summary (active CTs, agents, total cost/min) with <500ms refresh latency
- **AC2**: CT table shows top 20 by cost with PID, NAME, STATE, MEM(MB), CPU%, COST($), PHASE columns
- **AC3**: Agent summary aggregates requests, latency, cost, and efficiency metrics per agent
- **AC4**: Handles 100+ concurrent CTs without performance degradation (<50ms dashboard latency impact)
- **AC5**: Memory overhead of metrics system <5% of traced workload memory consumption
- **AC6**: Time-series store retains 1-minute sliding window at 500ms granularity (120 samples per CT)
- **AC7**: Command-line interface supports `--interval`, `--sort`, `--filter` flags
- **AC8**: Test suite includes 5000+ synthetic operations validating concurrent collection
- **AC9**: API endpoints JSON-serializable for integration with external monitoring stacks

---

## Design Principles

1. **Zero-Copy Architecture**: Ring buffers and Arc<RwLock> minimize allocation overhead
2. **Sampling Strategy**: 500ms collection interval balances latency visibility with storage efficiency
3. **Composability**: Metrics collector, store, and renderer are independently testable
4. **Production Ready**: Graceful degradation under load; metrics collection never blocks CT execution
5. **Operator-Centric UX**: Dashboard mirrors `top` semantics for familiar cognitive system debugging

---

## Implementation Checklist

- [ ] MetricsCollector integrates with XKernal CT runtime hooks
- [ ] TimeSeriesStore ring-buffer implementation with concurrent access patterns
- [ ] Axum API server with JSON serialization for /metrics, /ct/{id}, /agents endpoints
- [ ] ncurses dashboard renderer with interactive sorting/filtering (SortField enum)
- [ ] Synthetic workload generator (100 CTs × 50 operations = 5000 ops)
- [ ] Memory profiling validation (<5% overhead)
- [ ] Latency benchmarks (<500ms refresh)
- [ ] Integration test with real XKernal cognitive tasks
- [ ] Documentation: cs-top man page and monitoring best practices guide

---

**Week 11 Delivery Target**: Fully functional cs-top prototype integrated with XKernal development cluster, with production-ready metrics collection pipeline capable of supporting 500+ concurrent CTs in scaled deployments.
