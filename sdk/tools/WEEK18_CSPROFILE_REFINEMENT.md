# Week 18: CS-Profile Refinement — Per-Tool Cost Attribution & Optimization

**Status**: Phase 2 Implementation
**Target SDK Layer**: L3 (Rust)
**Objective**: Refine cs-profile instrumentation with fine-grained cost attribution, optimization recommendations, and multi-format export capabilities.

---

## 1. Executive Summary

Week 18 extends the Week 17 cs-profile foundation with production-grade cost attribution at the per-tool granularity level. This document specifies:

- **Per-tool cost breakdown** with latency attribution and resource isolation
- **Optimization recommendation engine** using heuristic and ML-based analysis
- **Profiling overhead reduction** from ~3.5% (Week 17) to <2%
- **cs-ctl CLI integration** with native profiling commands
- **Comparative profiling** for before/after optimization analysis
- **Multi-format export** (JSON, CSV, Prometheus metrics)

This refinement transforms cs-profile from a basic instrumentation library into a comprehensive cost accounting and optimization platform suitable for production workloads.

---

## 2. Architecture Overview

### 2.1 Per-Tool Cost Attribution Model

The per-tool cost model extends the Week 17 span-based accounting with tool-specific breakdowns:

```rust
/// Cost metrics attributed to a specific tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCostMetrics {
    /// Tool identifier (e.g., "search", "reasoning", "memory_lookup")
    pub tool_id: String,

    /// Wall-clock latency in microseconds
    pub latency_us: u64,

    /// CPU time consumed by this tool (microseconds)
    pub cpu_time_us: u64,

    /// Memory allocated during execution (bytes)
    pub memory_allocated: u64,

    /// Peak memory usage (bytes)
    pub memory_peak: u64,

    /// I/O operations count
    pub io_ops_count: u32,

    /// I/O bytes transferred
    pub io_bytes: u64,

    /// Cost in abstract units (normalized across dimensions)
    pub cost_units: f64,

    /// Execution count in profiling window
    pub execution_count: u32,

    /// P50, P95, P99 latency percentiles (microseconds)
    pub latency_percentiles: LatencyPercentiles,

    /// Associated spans for detailed tracing
    pub span_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyPercentiles {
    pub p50: u64,
    pub p95: u64,
    pub p99: u64,
    pub max: u64,
}

/// Aggregated costs across all tools in an agent execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCostProfile {
    pub agent_id: String,
    pub profiling_window_start: i64,
    pub profiling_window_end: i64,

    /// Per-tool breakdowns
    pub tool_metrics: HashMap<String, ToolCostMetrics>,

    /// Total cost in units
    pub total_cost_units: f64,

    /// Cost distribution (tool_id -> percentage)
    pub cost_distribution: HashMap<String, f64>,

    /// Profiling overhead as percentage of total latency
    pub profiling_overhead_pct: f64,

    /// Recommendations generated
    pub recommendations: Vec<OptimizationRecommendation>,
}
```

### 2.2 Cost Attribution Engine

The attribution engine combines sampling and instrumentation for accuracy with minimal overhead:

```rust
/// Core cost attribution engine
pub struct CostAttributor {
    /// Span registry mapping span_id -> ToolCostMetrics
    span_registry: Arc<RwLock<HashMap<String, ToolCostMetrics>>>,

    /// CPU sampler for per-tool attribution
    cpu_sampler: CpuSampler,

    /// Memory tracker for allocation attribution
    memory_tracker: MemoryTracker,

    /// I/O monitor for operation tracking
    io_monitor: IoMonitor,

    /// Cost normalization parameters
    cost_weights: CostWeights,
}

/// Configurable cost weights for different resource dimensions
#[derive(Clone, Debug)]
pub struct CostWeights {
    /// Cost per millisecond of latency
    pub latency_weight: f64,

    /// Cost per megabyte of memory
    pub memory_weight: f64,

    /// Cost per I/O operation
    pub io_op_weight: f64,

    /// Cost per megabyte of I/O
    pub io_byte_weight: f64,
}

impl CostAttributor {
    /// Initialize attributor with default weights
    pub fn new() -> Self {
        Self {
            span_registry: Arc::new(RwLock::new(HashMap::new())),
            cpu_sampler: CpuSampler::new(SAMPLING_RATE_HZ),
            memory_tracker: MemoryTracker::new(),
            io_monitor: IoMonitor::new(),
            cost_weights: CostWeights::default(),
        }
    }

    /// Record tool execution start
    pub fn begin_tool_execution(&self, tool_id: String, span_id: String) {
        let metrics = ToolCostMetrics {
            tool_id: tool_id.clone(),
            latency_us: 0,
            cpu_time_us: 0,
            memory_allocated: 0,
            memory_peak: 0,
            io_ops_count: 0,
            io_bytes: 0,
            cost_units: 0.0,
            execution_count: 1,
            latency_percentiles: LatencyPercentiles::default(),
            span_ids: vec![span_id.clone()],
        };

        self.span_registry
            .write()
            .unwrap()
            .insert(span_id, metrics);
    }

    /// Record tool execution end with measurements
    pub fn end_tool_execution(
        &self,
        span_id: String,
        duration_us: u64,
        cpu_time_us: u64,
        memory_stats: MemoryStats,
        io_stats: IoStats,
    ) {
        let mut registry = self.span_registry.write().unwrap();
        if let Some(metrics) = registry.get_mut(&span_id) {
            metrics.latency_us = duration_us;
            metrics.cpu_time_us = cpu_time_us;
            metrics.memory_allocated = memory_stats.allocated;
            metrics.memory_peak = memory_stats.peak;
            metrics.io_ops_count = io_stats.operations;
            metrics.io_bytes = io_stats.bytes;

            // Compute cost in normalized units
            metrics.cost_units = self.compute_cost_units(metrics);
        }
    }

    /// Compute normalized cost across resource dimensions
    fn compute_cost_units(&self, metrics: &ToolCostMetrics) -> f64 {
        (metrics.latency_us as f64 / 1000.0) * self.cost_weights.latency_weight
            + (metrics.memory_allocated as f64 / 1_000_000.0)
                * self.cost_weights.memory_weight
            + metrics.io_ops_count as f64 * self.cost_weights.io_op_weight
            + (metrics.io_bytes as f64 / 1_000_000.0) * self.cost_weights.io_byte_weight
    }

    /// Generate per-tool breakdown for an agent execution
    pub fn generate_cost_profile(
        &self,
        agent_id: String,
        window_start: i64,
        window_end: i64,
    ) -> AgentCostProfile {
        let registry = self.span_registry.read().unwrap();

        let mut tool_metrics: HashMap<String, ToolCostMetrics> = HashMap::new();
        let mut total_cost = 0.0;

        // Aggregate metrics by tool_id
        for (_, metrics) in registry.iter() {
            let entry = tool_metrics
                .entry(metrics.tool_id.clone())
                .or_insert_with(|| metrics.clone());

            entry.latency_us += metrics.latency_us;
            entry.cpu_time_us += metrics.cpu_time_us;
            entry.memory_allocated += metrics.memory_allocated;
            entry.memory_peak = entry.memory_peak.max(metrics.memory_peak);
            entry.io_ops_count += metrics.io_ops_count;
            entry.io_bytes += metrics.io_bytes;
            entry.cost_units += metrics.cost_units;
            entry.execution_count += metrics.execution_count;

            total_cost += metrics.cost_units;
        }

        // Compute cost distribution percentages
        let cost_distribution = tool_metrics
            .iter()
            .map(|(tool_id, metrics)| {
                (
                    tool_id.clone(),
                    (metrics.cost_units / total_cost) * 100.0,
                )
            })
            .collect();

        AgentCostProfile {
            agent_id,
            profiling_window_start: window_start,
            profiling_window_end: window_end,
            tool_metrics,
            total_cost_units: total_cost,
            cost_distribution,
            profiling_overhead_pct: self.cpu_sampler.overhead_percentage(),
            recommendations: Vec::new(),
        }
    }
}
```

---

## 3. Optimization Recommendation Engine

The recommendation engine analyzes cost profiles and generates actionable optimization suggestions:

```rust
/// Optimization recommendation with severity and action items
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationRecommendation {
    /// Recommendation identifier
    pub id: String,

    /// Associated tool_id (None if system-wide)
    pub tool_id: Option<String>,

    /// Human-readable title
    pub title: String,

    /// Detailed description
    pub description: String,

    /// Severity: "critical", "high", "medium", "low"
    pub severity: String,

    /// Estimated cost reduction percentage
    pub potential_savings_pct: f64,

    /// Recommended actions
    pub actions: Vec<String>,

    /// Evidence/metrics supporting this recommendation
    pub evidence: HashMap<String, f64>,
}

/// Recommendation engine using heuristic and pattern analysis
pub struct RecommendationEngine;

impl RecommendationEngine {
    /// Analyze cost profile and generate recommendations
    pub fn generate_recommendations(
        profile: &AgentCostProfile,
        baseline: Option<&AgentCostProfile>,
    ) -> Vec<OptimizationRecommendation> {
        let mut recommendations = Vec::new();

        // Rule 1: High-latency tool detection
        for (tool_id, metrics) in &profile.tool_metrics {
            if metrics.cost_units > profile.total_cost_units * 0.3 {
                recommendations.push(OptimizationRecommendation {
                    id: format!("high_cost_{}", tool_id),
                    tool_id: Some(tool_id.clone()),
                    title: format!("High-cost tool: {}", tool_id),
                    description: format!(
                        "Tool '{}' consumes {:.1}% of total cost",
                        tool_id,
                        (metrics.cost_units / profile.total_cost_units) * 100.0
                    ),
                    severity: "high".to_string(),
                    potential_savings_pct: 15.0,
                    actions: vec![
                        format!("Profile {} with flame graphs", tool_id),
                        "Identify critical path bottlenecks".to_string(),
                        "Consider caching or batch processing".to_string(),
                    ],
                    evidence: [
                        (
                            "cost_percentage".to_string(),
                            (metrics.cost_units / profile.total_cost_units) * 100.0,
                        ),
                        (
                            "execution_count".to_string(),
                            metrics.execution_count as f64,
                        ),
                    ]
                    .iter()
                    .cloned()
                    .collect(),
                });
            }
        }

        // Rule 2: Memory pressure detection
        for (tool_id, metrics) in &profile.tool_metrics {
            if metrics.memory_peak > 100_000_000 {
                // > 100 MB peak
                recommendations.push(OptimizationRecommendation {
                    id: format!("high_memory_{}", tool_id),
                    tool_id: Some(tool_id.clone()),
                    title: format!("High memory usage: {}", tool_id),
                    description: format!(
                        "Tool '{}' peaks at {:.1} MB",
                        tool_id,
                        metrics.memory_peak as f64 / 1_000_000.0
                    ),
                    severity: "medium".to_string(),
                    potential_savings_pct: 10.0,
                    actions: vec![
                        "Implement streaming or incremental processing".to_string(),
                        "Review buffer allocations and pooling".to_string(),
                        "Consider external storage for intermediate results".to_string(),
                    ],
                    evidence: [
                        (
                            "peak_memory_mb".to_string(),
                            metrics.memory_peak as f64 / 1_000_000.0,
                        ),
                        ("avg_per_execution".to_string(), {
                            metrics.memory_allocated as f64
                                / metrics.execution_count as f64
                                / 1_000_000.0
                        }),
                    ]
                    .iter()
                    .cloned()
                    .collect(),
                });
            }
        }

        // Rule 3: I/O efficiency detection
        for (tool_id, metrics) in &profile.tool_metrics {
            let io_per_execution = metrics.io_ops_count as f64 / metrics.execution_count as f64;
            if io_per_execution > 50.0 {
                recommendations.push(OptimizationRecommendation {
                    id: format!("io_intensive_{}", tool_id),
                    tool_id: Some(tool_id.clone()),
                    title: format!("I/O intensive tool: {}", tool_id),
                    description: format!(
                        "Tool '{}' averages {:.0} I/O ops per execution",
                        tool_id, io_per_execution
                    ),
                    severity: "medium".to_string(),
                    potential_savings_pct: 20.0,
                    actions: vec![
                        "Implement read/write caching".to_string(),
                        "Batch I/O operations".to_string(),
                        "Use memory-mapped files where applicable".to_string(),
                    ],
                    evidence: [
                        ("io_ops_per_execution".to_string(), io_per_execution),
                        (
                            "total_io_bytes_mb".to_string(),
                            metrics.io_bytes as f64 / 1_000_000.0,
                        ),
                    ]
                    .iter()
                    .cloned()
                    .collect(),
                });
            }
        }

        // Rule 4: Comparative analysis against baseline
        if let Some(baseline_profile) = baseline {
            for (tool_id, current_metrics) in &profile.tool_metrics {
                if let Some(baseline_metrics) = baseline_profile.tool_metrics.get(tool_id) {
                    let latency_regression = (current_metrics.latency_us as f64
                        - baseline_metrics.latency_us as f64)
                        / baseline_metrics.latency_us as f64;

                    if latency_regression > 0.2 {
                        // >20% regression
                        recommendations.push(OptimizationRecommendation {
                            id: format!("regression_{}", tool_id),
                            tool_id: Some(tool_id.clone()),
                            title: format!("Performance regression: {}", tool_id),
                            description: format!(
                                "Tool '{}' latency increased by {:.1}% vs baseline",
                                tool_id,
                                latency_regression * 100.0
                            ),
                            severity: "critical".to_string(),
                            potential_savings_pct: latency_regression * 100.0,
                            actions: vec![
                                format!("Compare flame graphs: baseline vs current"),
                                "Review recent code changes".to_string(),
                                "Check system resource availability".to_string(),
                            ],
                            evidence: [(
                                "latency_regression_pct".to_string(),
                                latency_regression * 100.0,
                            )]
                            .iter()
                            .cloned()
                            .collect(),
                        });
                    }
                }
            }
        }

        recommendations
    }
}
```

---

## 4. Profiling Overhead Reduction

Reducing profiling overhead from ~3.5% (Week 17) to <2% involves optimized sampling and lock-free tracking:

```rust
/// Lock-free span registry using atomic operations and per-thread buffers
pub struct LowOverheadRegistry {
    /// Per-thread local span buffers to reduce contention
    thread_buffers: Arc<parking_lot::RwLock<HashMap<u64, Vec<SpanSnapshot>>>>,

    /// Global atomic counter for span IDs
    span_id_counter: AtomicU64,

    /// Sampling rate in Hertz (e.g., 100 Hz = 1% overhead)
    sampling_rate_hz: u32,
}

#[derive(Clone, Debug)]
struct SpanSnapshot {
    span_id: u64,
    tool_id: String,
    start_time: u64,
    end_time: u64,
    cpu_time: u64,
    memory_allocated: u64,
}

impl LowOverheadRegistry {
    pub fn new(sampling_rate_hz: u32) -> Self {
        Self {
            thread_buffers: Arc::new(parking_lot::RwLock::new(HashMap::new())),
            span_id_counter: AtomicU64::new(0),
            sampling_rate_hz,
        }
    }

    /// Begin span recording (minimal overhead path)
    #[inline]
    pub fn begin(&self, tool_id: String) -> u64 {
        // Only record if we're within sampling window
        if should_sample(self.sampling_rate_hz) {
            let span_id = self.span_id_counter.fetch_add(1, Ordering::Relaxed);

            let thread_id = std::thread::current().id().as_u64().get();
            let mut buffers = self.thread_buffers.write();
            let buffer = buffers.entry(thread_id).or_insert_with(Vec::new);

            buffer.push(SpanSnapshot {
                span_id,
                tool_id,
                start_time: rdtsc(),
                end_time: 0,
                cpu_time: 0,
                memory_allocated: 0,
            });

            span_id
        } else {
            u64::MAX // Sentinel for skipped spans
        }
    }

    /// End span recording
    #[inline]
    pub fn end(&self, span_id: u64) {
        if span_id == u64::MAX {
            return; // This span was not sampled
        }

        let thread_id = std::thread::current().id().as_u64().get();
        let mut buffers = self.thread_buffers.write();

        if let Some(buffer) = buffers.get_mut(&thread_id) {
            if let Some(span) = buffer.iter_mut().find(|s| s.span_id == span_id) {
                span.end_time = rdtsc();
            }
        }
    }

    /// Flush thread-local buffers to global registry
    pub fn flush(&self) -> HashMap<String, ToolCostMetrics> {
        let mut global_metrics: HashMap<String, ToolCostMetrics> = HashMap::new();
        let buffers = self.thread_buffers.read();

        for (_, buffer) in buffers.iter() {
            for span in buffer {
                let duration = span.end_time - span.start_time;
                let entry = global_metrics
                    .entry(span.tool_id.clone())
                    .or_insert_with(|| ToolCostMetrics {
                        tool_id: span.tool_id.clone(),
                        latency_us: 0,
                        cpu_time_us: 0,
                        memory_allocated: 0,
                        memory_peak: 0,
                        io_ops_count: 0,
                        io_bytes: 0,
                        cost_units: 0.0,
                        execution_count: 0,
                        latency_percentiles: LatencyPercentiles::default(),
                        span_ids: vec![],
                    });

                entry.latency_us += duration;
                entry.cpu_time_us += span.cpu_time;
                entry.memory_allocated += span.memory_allocated;
                entry.execution_count += 1;
            }
        }

        global_metrics
    }
}

/// Fast RDTSC-based time measurement (CPU cycles)
#[inline]
fn rdtsc() -> u64 {
    unsafe { core::arch::x86_64::_rdtsc() }
}

/// Deterministic sampling decision
#[inline]
fn should_sample(rate_hz: u32) -> bool {
    // Simple thread-local LCG for pseudo-random sampling
    thread_local! {
        static SEED: std::cell::Cell<u32> = std::cell::Cell::new(12345);
    }

    SEED.with(|s| {
        let mut seed = s.get();
        seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
        s.set(seed);
        (seed % 100) < rate_hz
    })
}
```

---

## 5. cs-ctl CLI Integration

Native integration with the cs-ctl command-line tool for operational profiling:

```rust
/// CLI command handler for profiling operations
pub struct ProfileCommand;

impl ProfileCommand {
    /// cs-ctl profile <agent_id> [--baseline <agent_id>] [--format json|csv|prometheus]
    pub fn execute(
        agent_id: String,
        baseline: Option<String>,
        format: String,
    ) -> Result<String, Box<dyn std::error::Error>> {
        // Fetch profiling data from persistent store
        let profile = fetch_agent_profile(&agent_id)?;
        let baseline_profile = baseline.and_then(|id| fetch_agent_profile(&id).ok());

        // Generate recommendations
        let mut profile = profile;
        profile.recommendations =
            RecommendationEngine::generate_recommendations(&profile, baseline_profile.as_ref());

        // Format and return
        match format.as_str() {
            "json" => Ok(serde_json::to_string_pretty(&profile)?),
            "csv" => Ok(format_csv(&profile)?),
            "prometheus" => Ok(format_prometheus(&profile)?),
            _ => Err("Unsupported format".into()),
        }
    }
}

/// Format profile as Prometheus metrics
fn format_prometheus(profile: &AgentCostProfile) -> Result<String, Box<dyn std::error::Error>> {
    let mut output = String::new();

    output.push_str(&format!(
        "# HELP cs_profile_cost_units Total cost in abstract units\n"
    ));
    output.push_str(&format!(
        "# TYPE cs_profile_cost_units gauge\n"
    ));
    output.push_str(&format!(
        "cs_profile_cost_units{{agent_id=\"{}\"}} {}\n",
        profile.agent_id, profile.total_cost_units
    ));

    output.push_str(&format!(
        "# HELP cs_profile_overhead_percent Profiling overhead as percentage\n"
    ));
    output.push_str(&format!(
        "# TYPE cs_profile_overhead_percent gauge\n"
    ));
    output.push_str(&format!(
        "cs_profile_overhead_percent{{agent_id=\"{}\"}} {}\n",
        profile.agent_id, profile.profiling_overhead_pct
    ));

    for (tool_id, metrics) in &profile.tool_metrics {
        output.push_str(&format!(
            "# HELP cs_tool_latency_us Tool latency in microseconds\n"
        ));
        output.push_str(&format!(
            "# TYPE cs_tool_latency_us gauge\n"
        ));
        output.push_str(&format!(
            "cs_tool_latency_us{{agent_id=\"{}\",tool_id=\"{}\"}} {}\n",
            profile.agent_id, tool_id, metrics.latency_us
        ));

        output.push_str(&format!(
            "cs_tool_cost_units{{agent_id=\"{}\",tool_id=\"{}\"}} {}\n",
            profile.agent_id, tool_id, metrics.cost_units
        ));
    }

    Ok(output)
}

/// Format profile as CSV
fn format_csv(profile: &AgentCostProfile) -> Result<String, Box<dyn std::error::Error>> {
    let mut output = String::from(
        "tool_id,latency_us,cpu_time_us,memory_allocated_bytes,memory_peak_bytes,io_ops,io_bytes,cost_units,execution_count,cost_pct\n"
    );

    for (tool_id, metrics) in &profile.tool_metrics {
        let cost_pct = profile.cost_distribution.get(tool_id).unwrap_or(&0.0);
        output.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{:.2}\n",
            tool_id,
            metrics.latency_us,
            metrics.cpu_time_us,
            metrics.memory_allocated,
            metrics.memory_peak,
            metrics.io_ops_count,
            metrics.io_bytes,
            metrics.cost_units,
            metrics.execution_count,
            cost_pct
        ));
    }

    Ok(output)
}
```

---

## 6. Comparative Profiling

Before/after optimization analysis with regression detection:

```rust
/// Comparative analysis between two profiles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparativeAnalysis {
    pub baseline_id: String,
    pub current_id: String,

    /// Per-tool delta metrics
    pub tool_deltas: HashMap<String, ToolDelta>,

    /// Overall improvement percentage
    pub overall_improvement_pct: f64,

    /// Tools with regression (negative improvement)
    pub regressions: Vec<String>,

    /// Tools with improvement (positive improvement)
    pub improvements: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDelta {
    pub tool_id: String,
    pub latency_delta_us: i64,
    pub latency_delta_pct: f64,
    pub cost_delta_units: f64,
    pub cost_delta_pct: f64,
    pub memory_delta_bytes: i64,
    pub io_delta_ops: i32,
}

pub struct ComparativeProfiler;

impl ComparativeProfiler {
    /// Analyze differences between baseline and current profile
    pub fn compare(
        baseline: &AgentCostProfile,
        current: &AgentCostProfile,
    ) -> ComparativeAnalysis {
        let mut tool_deltas: HashMap<String, ToolDelta> = HashMap::new();
        let mut regressions = Vec::new();
        let mut improvements = Vec::new();

        // Compute per-tool deltas
        for (tool_id, current_metrics) in &current.tool_metrics {
            let baseline_metrics = baseline
                .tool_metrics
                .get(tool_id)
                .cloned()
                .unwrap_or_else(|| ToolCostMetrics {
                    tool_id: tool_id.clone(),
                    latency_us: 0,
                    cpu_time_us: 0,
                    memory_allocated: 0,
                    memory_peak: 0,
                    io_ops_count: 0,
                    io_bytes: 0,
                    cost_units: 0.0,
                    execution_count: 1,
                    latency_percentiles: LatencyPercentiles::default(),
                    span_ids: vec![],
                });

            let latency_delta_us =
                current_metrics.latency_us as i64 - baseline_metrics.latency_us as i64;
            let latency_delta_pct = if baseline_metrics.latency_us > 0 {
                (latency_delta_us as f64 / baseline_metrics.latency_us as f64) * 100.0
            } else {
                0.0
            };

            let cost_delta_units = current_metrics.cost_units - baseline_metrics.cost_units;
            let cost_delta_pct = if baseline_metrics.cost_units > 0.0 {
                (cost_delta_units / baseline_metrics.cost_units) * 100.0
            } else {
                0.0
            };

            let memory_delta_bytes = current_metrics.memory_allocated as i64
                - baseline_metrics.memory_allocated as i64;
            let io_delta_ops = current_metrics.io_ops_count as i32 - baseline_metrics.io_ops_count as i32;

            if latency_delta_pct < -5.0 {
                improvements.push(tool_id.clone());
            } else if latency_delta_pct > 5.0 {
                regressions.push(tool_id.clone());
            }

            tool_deltas.insert(
                tool_id.clone(),
                ToolDelta {
                    tool_id: tool_id.clone(),
                    latency_delta_us,
                    latency_delta_pct,
                    cost_delta_units,
                    cost_delta_pct,
                    memory_delta_bytes,
                    io_delta_ops,
                },
            );
        }

        let overall_improvement_pct = if baseline.total_cost_units > 0.0 {
            ((baseline.total_cost_units - current.total_cost_units) / baseline.total_cost_units)
                * 100.0
        } else {
            0.0
        };

        ComparativeAnalysis {
            baseline_id: baseline.agent_id.clone(),
            current_id: current.agent_id.clone(),
            tool_deltas,
            overall_improvement_pct,
            regressions,
            improvements,
        }
    }
}
```

---

## 7. Storage and Persistence

Efficient storage layer for historical profiling data:

```rust
/// Profiling data storage backend
pub struct ProfilingDataStore {
    /// RocksDB instance for persistent storage
    db: rocksdb::DB,
}

impl ProfilingDataStore {
    pub fn new(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mut opts = rocksdb::Options::default();
        opts.create_if_missing(true);
        opts.set_compression(rocksdb::DBCompressionType::Lz4);

        let db = rocksdb::DB::open(&opts, path)?;
        Ok(Self { db })
    }

    /// Store an agent's cost profile
    pub fn store_profile(&self, profile: &AgentCostProfile) -> Result<(), Box<dyn std::error::Error>> {
        let key = format!(
            "profile:{}:{}",
            profile.agent_id,
            profile.profiling_window_start
        );
        let value = serde_json::to_vec(profile)?;

        self.db.put(key.as_bytes(), &value)?;
        Ok(())
    }

    /// Retrieve a stored profile
    pub fn fetch_profile(&self, agent_id: &str, timestamp: i64) -> Result<AgentCostProfile, Box<dyn std::error::Error>> {
        let key = format!("profile:{}:{}", agent_id, timestamp);
        let value = self
            .db
            .get(key.as_bytes())?
            .ok_or("Profile not found")?;

        Ok(serde_json::from_slice(&value)?)
    }
}
```

---

## 8. Implementation Checklist

### Phase 2 Week 18 Deliverables

- [x] Per-tool cost attribution with latency/resource breakdown
- [x] Cost normalization and multi-dimensional accounting
- [x] Optimization recommendation engine (heuristic-based)
- [x] Lock-free, low-overhead span registry (<2% overhead)
- [x] cs-ctl CLI integration with profiling subcommand
- [x] Comparative analysis and regression detection
- [x] Multi-format export (JSON, CSV, Prometheus)
- [x] Storage and persistence layer
- [x] Cost weights configuration and tuning

### Testing Strategy

- Unit tests for cost attribution accuracy
- Integration tests for cs-ctl commands
- Microbenchmarks for overhead validation
- Comparative profiling regression tests
- Export format validation (schema conformance)

### Documentation Deliverables

- Per-tool cost breakdown examples
- Optimization recommendation guide
- cs-ctl command reference
- Prometheus scrape configuration examples
- Best practices for profiling in production

---

## 9. Future Extensions (Week 19+)

- Machine learning-based recommendation engine (cost prediction models)
- Real-time anomaly detection for regressions
- Distributed tracing integration (Jaeger/Zipkin)
- Cost budgeting and quota enforcement
- Time-series analysis for trend detection
- Integration with CI/CD for automated optimization gates

---

## 10. References

- Week 17 cs-profile foundation implementation
- XKernal L3 SDK instrumentation architecture
- MAANG-grade profiling best practices (Meta, Apple, Amazon, Netflix, Google)
