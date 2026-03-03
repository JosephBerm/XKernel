# cs-replay — Cognitive Task Replay Engine

**Deterministic replay of reasoning chains and task execution**

## Overview

`cs-replay` reconstructs and replays a CognitiveTask's complete execution from a checkpoint or trace log. It enables:

- **Deterministic Debugging:** Replay a task with breakpoints, stepping through reasoning
- **What-If Analysis:** Replay with modified capabilities or resource quotas
- **Validation:** Confirm that task reruns produce identical outputs (determinism testing)
- **Audit Trail:** Walk through historical reasoning decisions step-by-step

```bash
$ cs-replay checkpoint CT-abc123-checkpoint-5 \
    --breakpoint "mem_query:arxiv" \
    --interactive
```

Output:

```
Loading checkpoint CT-abc123-checkpoint-5 (2026-03-01 14:45:12)
Restored task state: 45,200 tokens consumed, reasoning phase active

Breakpoint triggered: syscall=mem_query, key contains "arxiv"

  Phase: REASON
  Token budget: 92,000 / 100,000 remaining
  Current memory state:
    - context_window: 14.2 MB / 16 MB
    - cached_results: {"arxiv_papers": [42 entries]}

  Next instruction:
    10:   mem_query(region=cache, key="arxiv_papers", limit=50) → [42 results]

  Options:
    > continue       # Resume execution
    > step           # Single-step next syscall
    > inspect        # Examine memory/capability state
    > modify-args    # Change next syscall arguments
    > rewind         # Go back to last checkpoint
```

## Features

### 1. Checkpoint-Based Replay

```bash
# Replay from most recent checkpoint
cs-replay checkpoint CT-abc123

# Replay from specific checkpoint timestamp
cs-replay checkpoint CT-abc123 --at "2026-03-01T14:45:00Z"

# Replay entire task history
cs-replay from-beginning CT-abc123
```

### 2. Interactive Debugging

```bash
# Step through execution interactively
cs-replay checkpoint CT-abc123 --interactive

(replay)> info memory
  L1 context: 14.2 MB / 16 MB (89%)
  L2 cache:   450 MB / 1000 MB (45%)

(replay)> info capabilities
  ReadMemory(all regions)    ✓
  InvokeTool(GPT-4)          ✓
  SendChannel(agent-b)       ✗ (expired)

(replay)> step
  Executing: mem_query(cache, "arxiv_papers", limit=50)
  Result: 42 documents

(replay)> continue
  [task resumes until next breakpoint or completion]
```

### 3. Breakpoints

```bash
# Break on syscall type
cs-replay checkpoint CT-abc123 --breakpoint "syscall=cap_grant"

# Break on capability denial
cs-replay checkpoint CT-abc123 --breakpoint "error=CapabilityDenied"

# Break on token budget approaching limit
cs-replay checkpoint CT-abc123 --breakpoint "tokens > 90000"

# Break on syscall latency (identify slow paths)
cs-replay checkpoint CT-abc123 --breakpoint "duration > 100ms"

# Multiple breakpoints
cs-replay checkpoint CT-abc123 \
  --breakpoint "syscall=mem_query" \
  --breakpoint "duration > 50ms"
```

### 4. What-If Analysis

```bash
# Replay with different capability set (what if agent had no tool access?)
cs-replay checkpoint CT-abc123 \
  --modify-capability "-InvokeTool(*)" \
  --observe "what happens without tools?"

# Replay with reduced token budget
cs-replay checkpoint CT-abc123 \
  --modify-quota "tokens=50000" \
  --observe "task succeeds with half tokens?"

# Replay with additional memory
cs-replay checkpoint CT-abc123 \
  --modify-quota "memory_bytes=64GB" \
  --observe "more memory helps?"
```

### 5. Determinism Testing

```bash
# Replay twice, compare outputs (should be identical)
cs-replay checkpoint CT-abc123 --output trace1.json
cs-replay checkpoint CT-abc123 --output trace2.json

# Compare traces
diff trace1.json trace2.json
# Exit code 0 = identical (deterministic) ✓
# Exit code 1 = different (non-deterministic) ✗

# Continuous determinism validation in CI
cs-replay checkpoint CT-abc123 --validate-determinism --runs 5
# Fails if any run differs from others
```

### 6. Trace Comparison

```bash
# Compare original execution with replay (audit)
cs-replay checkpoint CT-abc123 \
  --compare-with trace-original.json

Output:
  Syscall count: 1,245 (original) vs 1,245 (replay) ✓
  Token usage:   82,340 (original) vs 82,340 (replay) ✓
  Return values: all match ✓

  Timeline comparison (divergence detected):
    Original:  mem_query at t=58.234s, returned 42 docs
    Replay:    mem_query at t=58.235s, returned 42 docs
    Δ: +1ms (within tolerance)
```

## Usage

### Checkpoint Replay

```bash
# List available checkpoints for a CT
cs-replay list CT-abc123

Output:
  Checkpoint ID                         Timestamp             Phase    Tokens Used
  ─────────────────────────────────────────────────────────────────────────────
  CT-abc123-checkpoint-1               2026-03-01 14:23:45   PLAN     1,200
  CT-abc123-checkpoint-2               2026-03-01 14:24:10   REASON   15,300
  CT-abc123-checkpoint-3               2026-03-01 14:35:20   REASON   48,900
  CT-abc123-checkpoint-4               2026-03-01 14:45:00   ACT      72,100
  CT-abc123-checkpoint-5               2026-03-01 14:45:12   REFLECT  82,340

# Replay from checkpoint-3 (mid-reasoning)
cs-replay checkpoint CT-abc123-checkpoint-3 --fast
# --fast: replay without breakpoints, just show result
```

### Historical Replay

```bash
# Load trace from cs-trace export
cs-replay from-trace trace.json \
  --breakpoint "syscall=channel_send" \
  --interactive

# Walk through task from the beginning
# This is slower (must simulate each syscall) but doesn't require checkpoint
```

### Output Formats

```bash
# Replay and export modified trace
cs-replay checkpoint CT-abc123 \
  --modify-capability "-InvokeTool(*)" \
  --output json > trace-no-tools.json

# Compare JSON traces programmatically
python compare_traces.py trace-original.json trace-no-tools.json
```

## Architecture

### Replay Engine

1. **Load Checkpoint:** Restore CT state (memory, capabilities, phase)
2. **Initialize:** Set up syscall re-execution environment
3. **Replay Loop:** Execute syscalls sequentially with same arguments
4. **Compare:** Validate output matches original (if determinism check enabled)
5. **Report:** Display divergences or confirmations

### Determinism Assumptions

Replay is deterministic if:

- **Syscall arguments identical** — (enforced)
- **Kernel state identical** — (restored from checkpoint)
- **External tool responses identical** — (either replayed from cache or live)
- **Timing-sensitive code absent** — (rare; flagged if detected)

**Non-Deterministic Sources (Known):**

- Tool invocations with non-deterministic output (e.g., web scraping)
- Timing-dependent behaviors (wall-clock time syscalls)
- Random number generation (seed not recorded)

### Memory Requirements

- **Checkpoint storage:** ~100 MB per checkpoint (L1 + L2 snapshot)
- **Replay overhead:** ~10% extra memory (trace buffer for comparison)

## Implementation Details

**See:** `/sessions/youthful-vigilant-albattani/mnt/XKernal/sdk/tools/cs-replay/src/`

- `main.rs` — CLI entry point
- `replay_engine.rs` — Core replay logic
- `checkpoint_loader.rs` — Checkpoint deserialization
- `breakpoint.rs` — Breakpoint evaluation
- `trace_compare.rs` — Trace comparison and divergence detection
- `interactive.rs` — Interactive REPL for debugging

## Use Cases

### Case 1: Debug Token Exhaustion

```bash
# Task mysteriously ran out of tokens
cs-replay checkpoint CT-abc123-token-exhausted \
  --breakpoint "tokens > 95000" \
  --interactive

(replay)> inspect memory
  [identify excessive mem_query calls]

(replay)> modify-capability "-mem_query"
# Replay without query capability
# Observe: does task succeed without queries?
```

### Case 2: Understand Capability Denial

```bash
# Task failed with CapabilityDenied
cs-replay from-beginning CT-abc123 \
  --breakpoint "error=CapabilityDenied" \
  --interactive

(replay)> info capabilities
  [check when capability was revoked]

(replay)> rewind CT-abc123-checkpoint-2
  [go back to before revocation]

(replay)> continue
  [see what task does with capability still valid]
```

### Case 3: Validate Reasoning Reproducibility

```bash
# Multi-agent task: ensure reasoning is reproducible
cs-replay from-trace agent-a-trace.json --validate-determinism --runs 3

Output:
  Run 1: 1,245 syscalls, 82,340 tokens, 2 tool errors
  Run 2: 1,245 syscalls, 82,340 tokens, 2 tool errors
  Run 3: 1,245 syscalls, 82,340 tokens, 2 tool errors
  ✓ Deterministic (3/3 runs identical)
```

### Case 4: Performance Regression Analysis

```bash
# Baseline checkpoint from 1 month ago
cs-replay checkpoint CT-abc123-v1.0 --fast --output baseline.json

# Current version
cs-replay checkpoint CT-abc123-v1.1 --fast --output current.json

# Compare
cs-replay compare baseline.json current.json

Output:
  Token usage: 82,340 (baseline) → 91,200 (current) [+10.7% regression]
  Syscall count: 1,245 → 1,580 [+26% more syscalls]
  Duration: 23.5s → 26.1s [+11%]

  Top regression sources:
    1. mem_query() now called 3x more often
    2. cap_grant() latency increased 2x
```

## Configuration

**Configuration File:** `~/.cs/replay-config.toml`

```toml
[replay]
determinism_check = true
max_deviation_ms = 5  # Allow 5ms timing deviation

[performance]
# Limit peak memory usage during replay
max_memory_mb = 4096

# Limit replay duration (prevent infinite loops)
timeout_seconds = 300
```

## Integration with Other Tools

```bash
# cs-trace → cs-replay pipeline
cs-trace run agent --output binary > task.trace
cs-replay from-trace task.trace --interactive

# cs-replay → cs-profile pipeline
# Replay with profiling enabled
cs-replay checkpoint CT-abc123 --profile --output json | cs-profile analyze

# cs-replay → cs-top pipeline
# Monitor real-time replay progress
cs-replay checkpoint CT-abc123 --interactive &
cs-top --follow CT-abc123
```

## Related Tools

- **cs-trace** — Capture task execution trace
- **cs-profile** — CPU/GPU/memory profiling
- **cs-top** — Real-time task monitoring
- **cs-capgraph** — Inspect capability delegation

## Limitations

1. **External State:** If task calls external APIs, replay returns original response (no live re-call)
2. **Non-Determinism:** Some tasks are inherently non-deterministic (flagged in output)
3. **Large Tasks:** Replaying multi-hour tasks requires significant memory
4. **Tool Availability:** If tool is no longer available, replay cannot complete (but fails gracefully)

## Roadmap

- [ ] Conditional breakpoints (e.g., `tokens > 90000 AND mem_query`)
- [ ] Time-travel debugging (jump to any syscall number)
- [ ] Collaborative replay (shared debugging session)
- [ ] Trace diffing UI (visual side-by-side comparison)
- [ ] Machine learning-based anomaly detection (identify suspicious syscall patterns)

## See Also

- **cs-trace:** Capture execution traces
- **Engineering Plan v2.5:** Section 3.5.6 — "Observable by Default"
- **Domain Model Deep Dive:** Section 4 — CSCI Syscalls

---

**Status:** Design document (implementation deferred to Week 06+)
**Estimated Implementation:** 400 lines Rust (replay engine) + 300 lines Rust (interactive REPL)
