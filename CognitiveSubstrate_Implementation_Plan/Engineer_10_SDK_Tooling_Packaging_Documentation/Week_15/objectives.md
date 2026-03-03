# Engineer 10 — SDK: Tooling, Packaging & Documentation — Week 15

## Phase: 2 (Advanced Debugging Tools & Registry)

## Weekly Objective
Begin cs-replay implementation. Design cognitive core dump format. Implement event stream replay mechanism. Enable stepping through reasoning chains from failed CTs.

## Document References
- **Primary:** Section 3.5.4 — cs-replay: replay failed CT from cognitive core dump, stepping through reasoning chain
- **Supporting:** Section 6.3 — Phase 2, Week 15-24

## Deliverables
- [ ] Cognitive core dump format specification (RFC)
- [ ] Core dump collection mechanism integrated with CT lifecycle
- [ ] Event stream replay library (Rust)
- [ ] Stepping mechanism (next, continue, breakpoint)
- [ ] Memory reconstruction from core dump
- [ ] cs-replay CLI design
- [ ] Test suite with recorded core dumps from various failure modes

## Technical Specifications
### Cognitive Core Dump Format
```
[Header]
format_version: 1
timestamp: 2026-03-01T12:34:56Z
ct_id: 1001
reason: inference_failure
csci_version: 1.0.0

[CT State]
memory_snapshot: base64(...)
registers: {...}
phase: inference
current_step: 42

[Event Stream]
event_0: {timestamp: 0ms, syscall: SYSCALL_CAPABILITY_QUERY, args: {...}}
event_1: {timestamp: 15ms, syscall: SYSCALL_TOOL_INVOKE, args: {...}}
...
event_final: {timestamp: 1250ms, syscall: SYSCALL_ERROR, reason: inference_timeout}
```

### cs-replay CLI
```bash
cs-ctl replay core_dump_001.cscd              # Load core dump
> next                                        # Execute next syscall
> breakpoint 25                               # Set breakpoint at event 25
> continue                                    # Run until breakpoint
> inspect memory 0x7fff0000                   # View memory state
> print registers                             # Show register contents
> backward                                    # Step backward (if supported)
```

### Replay Architecture
- Event stream parser (extract from core dump)
- VM-like execution engine (no actual model invocation)
- State reconstruction at each step
- Breakpoint and stepping support

## Dependencies
- **Blocked by:** Week 06 CI/CD, CSCI core dump mechanism stable
- **Blocking:** Week 16 cs-replay refinement, Week 17-18 cs-profile integration

## Acceptance Criteria
- [ ] Core dump format captures complete CT state
- [ ] Can replay 20+ different failure scenarios
- [ ] Stepping mechanism allows forward and backward navigation
- [ ] Memory state reconstruction accurate to within 1%
- [ ] Replay executes without model invocation (100x faster than real execution)
- [ ] Documentation sufficient for debugging operators

## Design Principles Alignment
- **Cognitive-Native:** Core dump captures full reasoning chain and model invocations
- **Debuggability:** Stepping through reasoning chain enables root cause analysis
- **Isolation by Default:** Core dump redaction prevents unauthorized data exposure
- **Determinism:** Replay produces identical behavior to original execution
