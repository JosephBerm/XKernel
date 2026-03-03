# cs-trace — Cognitive Task Tracer

**Like `strace` for CognitiveTasks**

## Overview

`cs-trace` is a comprehensive system call tracing tool for Cognitive Substrate. It captures and displays all CSCI syscalls executed by a CognitiveTask, with full context, timing, and arguments.

```bash
$ cs-trace run my-agent --task "reason about climate policy"
```

Output:

```
 time(ms)  duration(μs)  phase     syscall          args                         result
────────────────────────────────────────────────────────────────────────────────────
    10.2         150     SPAWN     ct_spawn         agent=my-agent               CT-xyz
    12.5          85     PLAN      mem_alloc        tier=L1, capacity=16MB       region-1
    13.1       2500     PLAN      cap_validate     cap=ReadMemory               ✓
    15.2       8000     REASON    signal_register   signal=MessageArrived        ✓
    18.7      45000     REASON    mem_query        query="climate+change"       42 results
    45.2      12000     REASON    channel_send     channel=agent-b              ✓
```

## Features

### 1. Real-Time Tracing

- Capture every CSCI syscall with microsecond precision
- Show call duration and return values
- Display arguments in human-readable format

### 2. Filtering & Selection

```bash
# Trace only memory syscalls
cs-trace run agent --filter "mem_*"

# Trace only failures
cs-trace run agent --filter-status "error"

# Trace syscalls taking >10ms
cs-trace run agent --filter-duration ">10ms"
```

### 3. Output Formats

```bash
# Default: human-readable table
cs-trace run agent

# JSON (for log aggregation)
cs-trace run agent --output json > trace.json

# CSV (for spreadsheet analysis)
cs-trace run agent --output csv > trace.csv

# Flamegraph (visualize time spent in syscalls)
cs-trace run agent --output flamegraph > trace.svg

# Binary (for replay with cs-replay)
cs-trace run agent --output binary > trace.bin
```

### 4. Statistics & Aggregation

```bash
# Summary statistics
cs-trace run agent --summary

Output:
  Total syscalls: 1,245
  Total duration: 23.5 seconds

  Top 5 syscalls by duration:
    1. mem_query      8,500 ms (36%)
    2. channel_send   4,200 ms (18%)
    3. ct_spawn       3,100 ms (13%)
    4. cap_grant      2,800 ms (12%)
    5. mem_write      1,900 ms (8%)

  Failures:
    TokenBudgetExhausted: 3
    CapabilityDenied:     1
    ChannelClosed:        2
```

### 5. Interactive Mode

```bash
# Pause/resume tracing, live filtering
cs-trace attach CT-xyz --interactive

Commands:
  > pause              # Pause tracing
  > resume             # Resume tracing
  > filter mem_*       # Show only memory syscalls
  > stats              # Show aggregated stats
  > export json        # Export current trace to JSON
  > quit               # Exit (trace saved)
```

## Usage

### Basic Usage

```bash
# Trace a new agent execution
cs-trace run my-agent --task "your task description"

# Trace an existing CT (attach)
cs-trace attach CT-abc123def456

# Trace with timeout (auto-stop after 1 hour)
cs-trace run agent --timeout 3600s
```

### Advanced Options

```bash
# Follow child CTs spawned during execution
cs-trace run agent --follow-spawns

# Include memory snapshots at each checkpoint
cs-trace run agent --capture-checkpoints

# Trace with full argument/return value logging
cs-trace run agent --verbose-args

# Sample syscalls (every 10th syscall)
cs-trace run agent --sample 10

# Store trace in persistent L3 storage
cs-trace run agent --persist
```

### Output Examples

**Default (Table Format):**

```
ct-id: CT-abc123def456
phase: REASON
start-time: 2026-03-01T14:23:45.123Z
end-time: 2026-03-01T14:45:12.456Z
duration: 21m 27s

 time(s)   duration(ms)   syscall              args                              result
─────────────────────────────────────────────────────────────────────────────────────
  10.234        0.150     ct_spawn             agent=assistant                   CT-1
  10.385        0.085     mem_alloc            tier=L1, capacity=16MB            region-1
  10.470        2.500     mem_read             region=region-1, key="prompt"    256 bytes
  13.000       45.000     signal_register      signal=MessageArrived             ✓
  58.234        8.500     mem_query            query="arxiv papers", limit=50    42 results
  66.745       12.000     channel_send         channel=agent-b, msg_size=512B   ✓
 234.567     8000.000     cap_grant            grantee=agent-b, cap=ReadMemory   cap-xyz
```

**JSON Format:**

```json
{
  "trace_metadata": {
    "ct_id": "CT-abc123def456",
    "start_time": "2026-03-01T14:23:45.123Z",
    "end_time": "2026-03-01T14:45:12.456Z",
    "duration_ms": 1287123
  },
  "syscalls": [
    {
      "time_ms": 10234,
      "duration_ms": 0.150,
      "syscall": "ct_spawn",
      "args": {
        "parent_agent": "assistant",
        "task_spec": { ... }
      },
      "result": {
        "status": "success",
        "return_value": "CT-1"
      }
    },
    ...
  ],
  "summary": {
    "total_syscalls": 1245,
    "total_duration_ms": 1287123,
    "failures": 2,
    "top_syscalls": [
      { "name": "mem_query", "count": 234, "total_ms": 8500 }
    ]
  }
}
```

## Architecture

### Data Collection

1. **Kernel Instrumentation:** CSCI syscall entry/exit hooks
2. **Trace Buffer:** Per-CT ring buffer in L1 (prevents unbounded memory)
3. **Overflow Handling:** Oldest entries discarded if buffer full (configurable strategy)
4. **Flush to L3:** Periodically flush to persistent storage for long-running tasks

### Performance Overhead

- **Minimal overhead:** ~2-5% slowdown (syscall timing + buffer writes)
- **Zero overhead mode:** `--sample 1000` (every 1000th syscall)
- **Ring buffer:** Bounded memory (default: 10MB per CT)

### Implementation Details

**See:** `/sessions/youthful-vigilant-albattani/mnt/XKernal/sdk/tools/cs-trace/src/`

- `main.rs` — CLI entry point
- `tracer.rs` — Trace capture and buffering
- `output.rs` — Formatter (table, JSON, CSV, flamegraph)
- `filter.rs` — Syscall filtering and selection
- `stats.rs` — Aggregation and statistics

## Integration

### Observe via CS-TOP

```bash
# Terminal 1: Run task with tracing
cs-trace run agent --persist

# Terminal 2: Monitor in real-time
cs-top --trace-detail
```

### Export for Analysis

```bash
# Export to JSON
cs-trace export CT-abc123 --format json > trace.json

# Analyze with custom scripts
python analyze_trace.py trace.json

# Load into Jupyter for visualization
import json
with open('trace.json') as f:
    trace = json.load(f)
    # Analyze syscall patterns, latency distributions, etc.
```

### CI/CD Integration

```bash
# Fail if any syscall takes >100ms
cs-trace run agent --assert-latency-max 100ms

# Fail if TokenBudgetExhausted occurs
cs-trace run agent --assert-no-error TokenBudgetExhausted

# Export metrics for observability platform
cs-trace run agent --export-prometheus > metrics.json
```

## Configuration

**Configuration File:** `~/.cs/trace-config.toml`

```toml
[trace]
buffer_size_mb = 10
sample_rate = 1  # 1 = all syscalls, 10 = every 10th

[output]
format = "table"  # or json, csv, flamegraph
colorize = true

[filter]
default_filter = "*"  # or "mem_*", "cap_*", etc.

[persistence]
auto_flush_enabled = true
flush_interval_ms = 5000
```

## Related Tools

- **cs-replay** — Replay a traced task execution (deterministic replay)
- **cs-profile** — CPU/GPU/memory profiling
- **cs-capgraph** — Visualize capability delegation chains
- **cs-top** — Real-time task monitoring

## Examples

### Example 1: Debug Memory Leak

```bash
# Trace task with memory focus
cs-trace run agent --filter "mem_*" --summary

# Output shows excessive mem_write calls
# Investigate with cs-capgraph to check for permission issues
```

### Example 2: Performance Analysis

```bash
# Export as flamegraph
cs-trace run agent --output flamegraph > trace.svg
open trace.svg

# Visualize time spent in each syscall category
# Identify bottlenecks (e.g., channel_send taking 50% of time)
```

### Example 3: Troubleshoot Capability Denial

```bash
# Find all failed capability checks
cs-trace run agent --filter-status error

# Output:
#  cap_validate    cap=InvokeTool(GPT-4)   ✗ CapabilityDenied
#  cap_grant       cap=InvokeTool(GPT-4)   ✓ (granted)
#  cap_validate    cap=InvokeTool(GPT-4)   ✓ (now valid)

# Audit: capability was not initially granted, had to be delegated
```

## Roadmap

- [ ] Distributed tracing (multi-CT correlation)
- [ ] Trace compression (delta encoding)
- [ ] Conditional breakpoints
- [ ] Hardware performance counter integration (perf events)
- [ ] Machine learning-based anomaly detection

## See Also

- **Engineering Plan v2.5:** Section 3.5.6 — "Developer Tools"
- **CSCI Syscall Reference:** `docs/domain_model_deep_dive.md` Section 4
- **Observable by Default Design:** Engineering Plan Section 1.2 — P5

---

**Status:** Design document (implementation deferred to Week 05+)
**Estimated Implementation:** 200 lines Rust (capture) + 300 lines Rust (output formatting)
