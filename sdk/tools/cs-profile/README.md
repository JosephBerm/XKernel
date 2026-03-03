# cs-profile — Cognitive Workload Profiler

**CPU/GPU/memory profiling for CognitiveTasks (like `perf` for reasoning)**

## Overview

`cs-profile` profiles CognitiveTask execution to identify performance bottlenecks:

- **Token Efficiency:** Tokens consumed per reasoning step
- **GPU Utilization:** GPU device usage, kernel launch overhead
- **Memory Hotspots:** L1/L2/L3 memory access patterns, eviction rates
- **Latency Analysis:** Which syscalls consume the most time?
- **Resource Attribution:** How much of quota goes to each activity?

```bash
$ cs-profile run my-agent --task "analyze quarterly earnings" \
    --format flamegraph

# Generates: profile.svg (visualize time spent in each syscall)
```

## Features

### 1. Token Profiling

```bash
# Profile token consumption by syscall
cs-profile run agent --profile tokens

Output:
  Total tokens: 82,340

  Tokens by syscall:
    mem_query        23,500 (29%)  ← semantic search expensive
    reasoning         38,200 (46%)  ← main work
    channel_send      12,100 (15%)  ← inter-agent comms
    cap_grant         5,200 (6%)    ← capability management
    other             3,340 (4%)

  Tokens by phase:
    REASON           38,200 (46%)
    ACT              23,500 (29%)  ← tool invocations
    REFLECT          12,100 (15%)
    PLAN              5,200 (6%)
    SPAWN             3,340 (4%)
```

### 2. GPU Profiling

```bash
# Profile GPU usage
cs-profile run agent --profile gpu

Output:
  GPU Devices: H100 (1 device), MI300X (2 devices)

  H100-0 Usage:
    Total time: 18,500 ms
    Kernel launches: 542
    Average launch latency: 12.3 μs
    Total GPU-ms consumed: 18,500
    Average utilization: 78%

    Top kernels by duration:
      1. GPT-4 inference    12,000 ms (65%)
      2. Token embedding     4,200 ms (23%)
      3. Attention           2,100 ms (11%)
      4. Gradient (fine-tune)    200 ms (1%)

  MI300X-0 Usage:
    Total time: 8,200 ms
    Kernel launches: 128
    Average launch latency: 14.1 μs
    Total GPU-ms consumed: 8,200
    Average utilization: 42% (underutilized)

    Recommendation: Move workloads from MI300X to H100 (better utilization)
```

### 3. Memory Profiling

```bash
# Profile memory access patterns
cs-profile run agent --profile memory

Output:
  Memory Access Statistics:

  L1 Context Window (16 MB):
    Peak utilization: 14.2 MB (89%)
    Evictions: 0 (no spilling)
    Hot allocations:
      1. "context_tokens" → 8.4 MB
      2. "attention_cache" → 4.1 MB
      3. "scratch" → 1.7 MB

  L2 Semantic Cache (1 GB):
    Peak utilization: 450 MB (45%)
    Evictions: 23 (LRU cleanup triggered)
    Most-accessed entries:
      1. "arxiv_papers" → 42 reads
      2. "market_data" → 18 reads
      3. "wikipedia_excerpts" → 15 reads

  L3 Persistent (backend: SledDb):
    Bytes written: 2.3 GB
    Bytes read: 1.8 GB
    I/O wait: 450 ms (2% of total time)
    Compression ratio: 2.1x

  Recommendation: L2 cache hit rate is 82%, good. L3 reads causing 450ms delay—
    consider prefetching hot data to L2.
```

### 4. Latency Analysis

```bash
# Profile syscall latencies (identify slow paths)
cs-profile run agent --profile latency

Output:
  Syscall Latency Percentiles:

  mem_query (234 calls):
    p50:  18.2 ms
    p95:  82.3 ms
    p99: 142.1 ms
    max: 234.5 ms
    Total: 8,500 ms (29% of task time)

  channel_send (185 calls):
    p50:  12.1 ms
    p95:  45.2 ms
    p99:  68.9 ms
    max:  78.3 ms
    Total: 4,200 ms (14% of task time)

  mem_write (512 calls):
    p50:   2.3 ms
    p95:  15.1 ms
    p99:  23.4 ms
    max:  34.2 ms
    Total: 1,900 ms (6% of task time)

  Top slow syscalls (>100ms):
    mem_query (timeout=1s)            2 occurrences  [234ms, 156ms]
    channel_send (large payload)      1 occurrence   [123ms]
    cap_grant (graph traversal)       3 occurrences  [112ms, 98ms, 102ms]

  Recommendations:
    1. mem_query slowness: consider index optimization or batch queries
    2. channel_send(123ms): reduce message payload size
    3. cap_grant delays: capability graph has 500+ delegations—prune?
```

### 5. Flamegraph Visualization

```bash
# Generate flamegraph (hierarchical time visualization)
cs-profile run agent --profile tokens --format flamegraph > profile.svg
open profile.svg

# Interactive SVG shows:
#   X-axis: time spent
#   Y-axis: call stack depth
#   Color: different syscall types
#
# Hover to see details, click to zoom
```

Example flamegraph structure:

```
┌──────────────────────────────────────────────────────────────────┐
│ ct_spawn (100μs)                                                 │
├─────────────────────────────────────────────────────────────────┤
│ plan_phase (5,200 tokens)  │  reason_phase (38,200 tokens)   │
├─────────────────────────────────────────────────────────────────┤
│ cap_validate │ mem_alloc │  mem_query │ channel_send │ mem_write│
│   (1.2%)     │  (0.5%)   │  (29%)     │   (15%)      │  (6%)    │
└─────────────────────────────────────────────────────────────────┘
```

### 6. Comparative Profiling

```bash
# Compare performance across runs (regression detection)
cs-profile run agent --tag "v1.0" > profile-v1.0.json
cs-profile run agent --tag "v1.1" > profile-v1.1.json

cs-profile compare profile-v1.0.json profile-v1.1.json

Output:
  Regression Analysis (v1.0 → v1.1):

  Token usage:    82,340 → 91,200 [+10.7% ⚠️ regression]
  GPU-ms:         18,500 → 19,800 [+7.0% ⚠️]
  Wall time:     23.5s → 26.1s [+11% ⚠️]

  Syscall changes:
    mem_query:      234 → 512 [+119% ⚠️ major regression]
      Latency avg:   36ms → 42ms [+17%]

    cap_grant:      18 → 45 [+150% ⚠️]
      Latency avg:   45ms → 78ms [+73%]

  Hypothesis: v1.1 is doing more semantic queries and capability grants
  Impact: User experience likely slower (26s vs 23.5s)

  Recommendation: Investigate v1.1 changes to mem_query and cap_grant
```

## Usage

### Basic Profiling

```bash
# Profile everything (default)
cs-profile run agent

# Profile specific aspect
cs-profile run agent --profile tokens
cs-profile run agent --profile gpu
cs-profile run agent --profile memory
cs-profile run agent --profile latency
```

### Output Formats

```bash
# Human-readable table (default)
cs-profile run agent --output table

# JSON (for script processing)
cs-profile run agent --output json > profile.json

# CSV (for spreadsheet analysis)
cs-profile run agent --output csv > profile.csv

# Flamegraph (SVG visualization)
cs-profile run agent --output flamegraph > profile.svg

# Prometheus metrics (for monitoring)
cs-profile run agent --output prometheus > metrics.txt
```

### Advanced Options

```bash
# Profile with sampling (reduce overhead)
cs-profile run agent --sample 10  # Every 10th syscall

# Profile with tag (for comparative analysis)
cs-profile run agent --tag "baseline-v1.0"

# Attach to running CT
cs-profile attach CT-abc123def456

# Profile with breakdown by crew member
cs-profile run crew-task --breakdown-by crew-member

# Export for external tool
cs-profile run agent --export perflock > analysis.perflock
# Later: perflock analyze analysis.perflock
```

## Architecture

### Profiling Engine

1. **Instrumentation:** Hook syscall entry/exit (minimal overhead)
2. **Counters:** Track time, token count, GPU-ms per syscall
3. **Sampling:** Optional: sample every Nth syscall (reduce overhead)
4. **Aggregation:** Compute statistics (latency percentiles, totals)
5. **Reporting:** Format output (table, JSON, flamegraph)

### Performance Overhead

- **Profiling enabled:** ~5-10% slowdown (counter updates)
- **Sampling enabled (1/10):** ~1% slowdown
- **Zero profiling:** Re-run without --profile flag

### Storage

- **Profile output:** ~1-10 MB per run (JSON format)
- **Flamegraph SVG:** ~5-50 MB (interactive visualization)

## Implementation Details

**See:** `/sessions/youthful-vigilant-albattani/mnt/XKernal/sdk/tools/cs-profile/src/`

- `main.rs` — CLI entry point
- `profiler.rs` — Core profiling logic
- `counters.rs` — Token/GPU/memory counters
- `flamegraph.rs` — Flamegraph generation
- `output.rs` — Formatter (table, JSON, CSV, prometheus)
- `analyzer.rs` — Statistical analysis and regression detection

## Use Cases

### Case 1: Optimize Token Efficiency

```bash
# Profile to identify expensive operations
cs-profile run agent --profile tokens --output json

# Analyze:
# - mem_query consuming 29% of tokens (expensive!)
# - Consider: caching results, batching queries, or using vector search instead

# Solution: Add caching layer
# Re-run and compare
cs-profile run agent-v2 --tag "with-cache" --output json
cs-profile compare profile-v1.json profile-v2.json
# Result: 29% → 12% (significant savings)
```

### Case 2: Diagnose GPU Underutilization

```bash
# Profile GPU usage
cs-profile run agent --profile gpu

# Output shows MI300X at 42% utilization
# Identify: not enough work queued for MI300X

# Solution: Prioritize MI300X, move workloads from H100
# Re-profile
cs-profile run agent-rebalanced --profile gpu
# Result: H100 75% → 65%, MI300X 42% → 72% (better balance)
```

### Case 3: Detect Latency Regressions

```bash
# Baseline
cs-profile run agent --tag "v1.0"

# New version
cs-profile run agent --tag "v1.1"

# Detect regression
cs-profile compare v1.0 v1.1

# Output: cap_grant latency degraded 45ms → 78ms
# Investigate: What changed in v1.1 regarding capabilities?
# (e.g., larger capability graph, more delegations)
```

### Case 4: Understand Memory Access Patterns

```bash
# Profile memory
cs-profile run agent --profile memory

# Output shows:
# - L1 peak: 14.2 MB (fine)
# - L2 evictions: 23 (LRU cleanup happening)
# - Hot entries: arxiv_papers (42 reads), market_data (18 reads)

# Solution: Pre-load hot data to L2, or increase L2 capacity
cs-profile run agent-with-larger-l2 --profile memory
# Result: evictions 23 → 5, cache hit rate 82% → 94%
```

## Integration with Other Tools

```bash
# cs-trace → cs-profile pipeline
cs-trace run agent --output json > task.trace
cs-profile analyze task.trace

# cs-profile → cs-top pipeline
# Monitor performance in real-time
cs-profile run agent --interactive &
cs-top --metric tokens  # Watch token consumption live
```

## Related Tools

- **cs-trace** — Capture detailed execution trace
- **cs-replay** — Replay and debug tasks
- **cs-top** — Real-time monitoring
- **cs-capgraph** — Capability graph analysis

## Configuration

**Configuration File:** `~/.cs/profile-config.toml`

```toml
[profiling]
enabled = true
sample_rate = 1  # 1 = all syscalls, 10 = every 10th

[output]
format = "table"  # or json, csv, flamegraph, prometheus
colorize = true

[thresholds]
token_regression_warn = 0.05  # Warn if >5% token increase
latency_regression_warn_ms = 10  # Warn if latency >10ms increase
```

## Limitations

1. **Sampling Accuracy:** Sampling (1/10) estimates, not exact counts
2. **Nested Calls:** Flamegraph assumes non-nested syscalls (mostly true)
3. **GPU Attribution:** GPU kernel attribution requires driver support (limited in Phase A)

## Roadmap

- [ ] Machine learning-based anomaly detection
- [ ] Distributed profiling (profile multi-CT crews)
- [ ] Hardware performance counter integration
- [ ] Automatic optimization suggestions
- [ ] Integration with cost analytics (tokens $ cost)

## See Also

- **Engineering Plan v2.5:** Section 3.5.6 — "Developer Tools"
- **Resource Accounting Model:** Domain Model Deep Dive Section 6
- **GPU Architecture Review:** Section 3 — Performance Metrics

---

**Status:** Design document (implementation deferred to Week 07+)
**Estimated Implementation:** 500 lines Rust (profiler core) + 400 lines Rust (visualization)
