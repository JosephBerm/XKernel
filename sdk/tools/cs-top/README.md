# cs-top — Real-Time CognitiveTask Monitoring Dashboard

**Monitor live CT execution, resource usage, and system health (like `top` for CognitiveTasks)**

## Overview

`cs-top` provides a real-time dashboard for monitoring Cognitive Substrate system state:

- **Live CT Status:** Execution phase, token consumption, resource usage
- **System Metrics:** GPU utilization, memory pressure, scheduler queue depth
- **Performance Alerts:** Real-time warnings for degraded performance
- **Multi-View Modes:** CT-centric, GPU-centric, memory-centric, capability-centric
- **Interactive Controls:** Sort, filter, zoom, pause/resume monitoring

```bash
$ cs-top

# Output: TUI dashboard showing live CognitiveTask execution
```

## Features

### 1. Default View (CT-Centric)

```
Cognitive Substrate System Monitor — 2026-03-01 14:45:12

┌─ SYSTEM SUMMARY ──────────────────────────────────────────────────────────┐
│ CTs running: 24 (5 in REASON, 3 in ACT, 16 in PLAN)                      │
│ Scheduler queue depth: 18                                                  │
│ Token consumption rate: 12,500 tokens/sec                                  │
│ GPU utilization: H100[87%], MI300X[42%]                                   │
│ Memory pressure: L1[68%], L2[55%], L3[18%]                                │
│ System health: ✓ HEALTHY (1 warning: memory pressure)                     │
└───────────────────────────────────────────────────────────────────────────┘

┌─ COGNITIVE TASKS ─────────────────────────────────────────────────────────┐
│  PID    AGENT        PHASE    TOKENS/QUOTA   GPU-MS/QUOTA   WALL-TIME     │
│──────────────────────────────────────────────────────────────────────────│
│ CT-1    assistant    REASON    45,200/100K     12,000/30K    18.5s        │
│ CT-2    researcher   ACT       28,500/50K       5,200/20K    12.3s        │
│ CT-3    planner      PLAN         800/25K          0/10K     2.1s         │
│ CT-4    assistant    REASON    89,200/100K ⚠️  28,500/30K    42.1s ⚠️    │
│ CT-5    researcher   REASON    15,600/50K       2,100/20K    8.5s         │
│ CT-6    worker       PLAN       3,200/20K          0/10K     4.2s         │
│ ...     (18 more CTs)                                                     │
└───────────────────────────────────────────────────────────────────────────┘

┌─ SCHEDULER STATE ─────────────────────────────────────────────────────────┐
│ Ready queue:  CT-7, CT-11, CT-15, CT-18, CT-22, CT-25, ... (18 total)    │
│ Running:      CT-1 (REASON), CT-2 (ACT), CT-3 (PLAN), ... (5 total)      │
│ Blocked:      CT-10 (waiting for channel), CT-19 (deadline approaching)   │
│ Completed:    CT-100, CT-101, CT-102, ... (342 total)                    │
└───────────────────────────────────────────────────────────────────────────┘

Commands: [q]uit [p]ause [g]pu [m]emory [c]apability [s]ort [f]ilter [h]elp
```

### 2. GPU-Centric View

```
cs-top --gpu

┌─ GPU DEVICES ─────────────────────────────────────────────────────────────┐
│  DEVICE    UTIL%   MEM%   POWER   TEMP   KERNEL LAUNCHES/s   AVG LATENCY  │
├───────────────────────────────────────────────────────────────────────────┤
│ H100-0      87%    64%    320W    58°C          1,245         12.3 μs     │
│ H100-1      73%    51%    280W    52°C          1,089         11.8 μs     │
│ MI300X-0    42%    38%     85W    48°C            512         14.2 μs     │
│ MI300X-1    18%    22%     35W    42°C            215         15.1 μs     │
└───────────────────────────────────────────────────────────────────────────┘

┌─ TOP WORKLOADS BY GPU-MS ─────────────────────────────────────────────────┐
│ CT-1    inference (GPT-4)        8,500 GPU-ms   (H100-0)                  │
│ CT-4    fine-tuning (GPT-3.5)    6,200 GPU-ms   (H100-0)                  │
│ CT-2    embedding (BERT)         4,100 GPU-ms   (MI300X-0)                │
│ CT-5    vector search (FAISS)    2,800 GPU-ms   (H100-1)                  │
│ CT-6    inference (Llama)        1,900 GPU-ms   (MI300X-0)                │
└───────────────────────────────────────────────────────────────────────────┘

Commands: [q]uit [p]ause [c]t-view [r]efresh [d]etail [h]elp
```

### 3. Memory-Centric View

```
cs-top --memory

┌─ MEMORY TIERS ────────────────────────────────────────────────────────────┐
│  TIER    USED/TOTAL    UTILIZATION   EVICTIONS   HOT ALLOCATIONS         │
├───────────────────────────────────────────────────────────────────────────┤
│  L1      14.2/16 MB      68%          0           context (8.4MB)         │
│  L2      510/1000 MB     51%         34           arxiv (180MB)           │
│  L3      2.3/10 TB       18%          0           knowledge_base (2.1TB)  │
└───────────────────────────────────────────────────────────────────────────┘

┌─ MEMORY PRESSURE ─────────────────────────────────────────────────────────┐
│ L2 cache utilization trending UP (51% → 53% in last 10s)                  │
│ ⚠️ L1 eviction rate: 0 events (stable)                                     │
│ ⚠️ L2 eviction rate: 3.4 events/sec (INCREASING)                           │
│ ℹ️  Recommendation: Consider increasing L2 capacity or reducing working set│
└───────────────────────────────────────────────────────────────────────────┘

┌─ TOP MEMORY CONSUMERS ─────────────────────────────────────────────────────┐
│ CT-1       14.2 MB L1 + 180 MB L2 (context + cached results)              │
│ CT-4        8.9 MB L1 +  95 MB L2 (fine-tuning state)                     │
│ CT-2        6.1 MB L1 +  65 MB L2 (embedding cache)                       │
│ CT-5        4.2 MB L1 +  45 MB L2 (vector indices)                        │
│ CT-6        3.8 MB L1 +  35 MB L2 (knowledge chunks)                      │
└───────────────────────────────────────────────────────────────────────────┘

Commands: [q]uit [p]ause [d]etail [e]viction-policy [h]elp
```

### 4. Capability-Centric View

```
cs-top --capability

┌─ CAPABILITY USAGE ────────────────────────────────────────────────────────┐
│  CAP TYPE         GRANTS   ACTIVE   DENIED   USAGE TREND                  │
├───────────────────────────────────────────────────────────────────────────┤
│ ReadMemory         54        52       2      ↑ increasing (normal)         │
│ InvokeTool         32        28       4      ↓ decreasing (tools unused)   │
│ SendChannel        10        10       0      → stable                      │
│ WriteMemory        28        24       4      ↑ increasing                  │
│ DelegateCapability  3         3       0      → stable                      │
└───────────────────────────────────────────────────────────────────────────┘

┌─ RECENT CAPABILITY DENIALS ───────────────────────────────────────────────┐
│ [14:45:02] Agent-B → CapabilityDenied(InvokeTool(GPT-4))                 │
│            Reason: Capability revoked at 2026-03-01 14:45:00             │
│ [14:44:58] Agent-C → CapabilityDenied(ReadMemory(crew-b))                │
│            Reason: Cross-crew access violation (policy)                   │
│ [14:44:45] Agent-D → CapabilityDenied(DelegateCapability)                │
│            Reason: Insufficient delegation rights                         │
└───────────────────────────────────────────────────────────────────────────┘

Commands: [q]uit [p]ause [a]udit [d]etail [h]elp
```

### 5. Interactive Controls

```
• Arrow keys / j/k: scroll through CTs
• [p] Pause/resume monitoring
• [q] Quit
• [s] Sort by (tokens, time, gpu-ms, memory)
• [f] Filter (by agent, phase, crew)
• [g] GPU view
• [m] Memory view
• [c] Capability view
• [d] Show details for selected CT
• [r] Refresh rate (1s, 5s, 10s)
• [h] Help
```

### 6. Drill-Down Details

```bash
# Focus on specific CT
cs-top --follow CT-1

Output:
┌─ CT-1 (assistant) — REAL-TIME TRACE ──────────────────────────────────────┐
│ Phase:        REASON (running for 18.5s)                                  │
│ Priority:     chain_criticality=0.8, deadline_urgent=true                  │
│ Token budget: 45,200 / 100,000 (45%)  ↑ consuming 2,500 tokens/sec       │
│ GPU-ms budget: 12,000 / 30,000 (40%)  ↑ active GPU kernel running       │
│ Memory:       L1[14.2/16 MB], L2[180/1000 MB], L3[requested]             │
│ Watchdog:     deadline=120.5s, loop_detected=false, iterations=5,200     │
│                                                                             │
│ Last 5 syscalls:                                                           │
│   14:45:12.123  mem_query("arxiv")        → 42 results (8.5ms)            │
│   14:45:12.035  channel_send(agent-b)     → ok (3.2ms)                   │
│   14:45:11.998  cap_validate(read_memory) → ✓ (0.1ms)                    │
│   14:45:11.945  mem_read(context)         → 256 bytes (2.1ms)             │
│   14:45:11.901  signal_register(timeout)  → ok (0.05ms)                  │
└───────────────────────────────────────────────────────────────────────────┘
```

## Usage

### Basic Commands

```bash
# Default view (CT-centric)
cs-top

# GPU view
cs-top --gpu

# Memory view
cs-top --memory

# Capability view
cs-top --capability

# Follow specific CT
cs-top --follow CT-abc123

# Follow specific agent
cs-top --agent assistant

# Follow specific crew
cs-top --crew crew-xyz
```

### Advanced Options

```bash
# Custom refresh rate (default 1s)
cs-top --interval 5s

# Show only running CTs (hide PLAN/PLAN phases)
cs-top --filter "phase=REASON|phase=ACT"

# Show only high-priority tasks
cs-top --filter "priority.chain_criticality > 0.5"

# Trace detail (show syscall trace for selected CT)
cs-top --trace-detail

# Pause on startup (don't auto-update)
cs-top --pause

# Export metrics to external monitoring (Prometheus)
cs-top --export-prometheus > /var/lib/prometheus/node_exporter/csub.prom
```

## Metrics Exported

### CT-Level Metrics

```
# Tokens
cognitive_task_tokens_consumed{ct_id, agent, crew}
cognitive_task_tokens_budget{ct_id, agent, crew}
cognitive_task_tokens_per_second{ct_id, agent, crew}

# GPU
cognitive_task_gpu_ms_consumed{ct_id, agent, device}
cognitive_task_gpu_ms_budget{ct_id, agent, device}

# Memory
cognitive_task_memory_l1_bytes{ct_id, agent}
cognitive_task_memory_l2_bytes{ct_id, agent}

# Phases
cognitive_task_phase{ct_id, agent, phase}
cognitive_task_phase_duration_seconds{ct_id, agent, phase}

# Capabilities
cognitive_task_capability_denied_total{ct_id, agent, capability_type}
```

### System-Level Metrics

```
# Scheduler
cognitive_scheduler_queue_depth
cognitive_scheduler_running_cts_total
cognitive_scheduler_context_switches_total

# GPU
gpu_utilization_percent{device_id}
gpu_memory_utilization_percent{device_id}
gpu_kernel_launch_rate_per_second{device_id}

# Memory
semantic_memory_l1_utilization_percent
semantic_memory_l2_utilization_percent
semantic_memory_l2_eviction_rate_per_second
semantic_memory_l3_utilization_percent
```

## Architecture

### Data Collection

1. **Kernel Hooks:** Real-time counters updated on every syscall
2. **Aggregation:** Per-CT and system-level metrics computed every 100ms
3. **TUI Rendering:** Ncurses-based dashboard refreshed at configurable interval (default 1s)
4. **Metric Export:** Prometheus-compatible format for external monitoring

### Performance Overhead

- **Monitoring enabled:** ~1-2% overhead (counter updates)
- **TUI rendering:** ~50 ms per refresh (not on critical path)
- **Zero overhead mode:** Disable with `--disable-monitoring` (not recommended)

## Implementation Details

**See:** `/sessions/youthful-vigilant-albattani/mnt/XKernal/sdk/tools/cs-top/src/`

- `main.rs` — CLI entry point
- `dashboard.rs` — TUI rendering (ncurses)
- `metrics_aggregator.rs` — Real-time metric collection
- `views.rs` — View implementations (ct, gpu, memory, capability)
- `alerts.rs` — Real-time alerting and warnings

## Use Cases

### Case 1: Monitor Long-Running Task

```bash
# Task expected to run for 2 hours
cs-top --follow CT-abc123 --interval 10s

# Dashboard shows:
# - Token consumption rate (is it on track to complete within budget?)
# - GPU utilization (is GPU being fully used?)
# - Memory trend (is memory growing linearly?)
# - Watchdog state (deadline approaching?)
```

### Case 2: Diagnose Scheduler Bottleneck

```bash
# System seems slow; check scheduler
cs-top

# Observe:
# - Scheduler queue depth: 50 (very deep)
# - CTs in REASON phase: 3 (not many running)
# - Recommendation: Increase parallelism, or CTs are I/O blocked
```

### Case 3: Monitor Resource Exhaustion

```bash
# Watch for resource pressure
cs-top --memory

# Alert: L2 eviction rate increasing
# Solution: Reduce working set, increase L2 capacity, or optimize caching
```

### Case 4: Performance Regression Detection

```bash
# Export baseline metrics
cs-top --export-prometheus > baseline.prom
# Let it run for 5 minutes

# Compare with current
# If token_consumption_rate increased >10%, alert
```

## Integration with Other Tools

```bash
# cs-top → cs-profile
# Start monitoring, then run profiling
cs-top --metric tokens --agent assistant &
cs-profile run assistant
# Watch real-time token consumption in cs-top

# cs-top → cs-capgraph
# Monitor capability denials
cs-top --capability --alert "CapabilityDenied > 5/min"
cs-capgraph audit --output json > audit.json

# cs-top → external monitoring (Prometheus + Grafana)
cs-top --export-prometheus &
# Scrape metrics in Prometheus, visualize in Grafana
```

## Related Tools

- **cs-trace** — Detailed syscall tracing
- **cs-profile** — Performance profiling
- **cs-replay** — Replay and debug
- **cs-capgraph** — Capability analysis

## Configuration

**Configuration File:** `~/.cs/top-config.toml`

```toml
[ui]
refresh_interval_ms = 1000  # 1 second
default_view = "ct"  # or gpu, memory, capability
color_scheme = "dark"  # or light

[alerts]
token_warning_percent = 85      # Warn at 85% of quota
memory_warning_percent = 80     # Warn at 80% utilization
gpu_warning_percent = 90        # Warn at 90% utilization
deadline_warning_percent = 90   # Warn at 90% time elapsed

[export]
prometheus_enabled = true
prometheus_port = 9090
```

## Limitations

1. **Terminal Size:** Dashboard assumes 80x24 or larger terminal
2. **Performance at Scale:** Monitoring 1000+ CTs may show latency
3. **Local Only:** Does not support remote monitoring (deferred)

## Roadmap

- [ ] Remote monitoring (connect to remote kernel)
- [ ] Web-based dashboard (alternative to TUI)
- [ ] Machine learning-based anomaly detection
- [ ] Custom dashboard layouts (user-defined metrics)
- [ ] Integration with Grafana for historical analysis

## See Also

- **Engineering Plan v2.5:** Section 1.2 — "Observable by Default"
- **Domain Model Deep Dive:** Section 6 — Resource Accounting
- **GPU Architecture Review:** Section 3 — Performance Metrics

---

**Status:** Design document (implementation in progress for Week 04)
**Estimated Implementation:** 600 lines Rust (TUI) + 300 lines Rust (metrics collection)
