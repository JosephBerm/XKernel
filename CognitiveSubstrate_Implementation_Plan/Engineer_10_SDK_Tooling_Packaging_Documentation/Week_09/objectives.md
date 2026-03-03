# Engineer 10 — SDK: Tooling, Packaging & Documentation — Week 09

## Phase: 1 (SDK Tooling & Debugging Infrastructure)

## Weekly Objective
Begin cs-trace prototype. Design attachment mechanism for running CT tracing. Implement CSCI syscall capture infrastructure. Create strace-like output format for cognitive operations.

## Document References
- **Primary:** Section 3.5.4 — cs-trace: trace running CT operations in real-time (strace analog for CSCI syscalls)
- **Supporting:** Section 6.3 — Phase 2, Week 15-24 (full debugging tools suite)

## Deliverables
- [ ] cs-trace architecture design RFC (attachment mechanism, syscall hooking)
- [ ] CSCI syscall capture library (Rust)
- [ ] strace-like output formatter for cognitive operations
- [ ] Prototype: attach to running CT and capture syscalls
- [ ] Test suite: synthetic CT with traced syscalls
- [ ] Documentation: cs-trace usage guide

## Technical Specifications
### cs-trace Architecture
```
cs-trace (CLI)
    ↓
Attachment Mechanism (inject tracer into CT)
    ↓
CSCI Syscall Hook Layer
    ↓
Syscall Buffer (ring buffer for efficiency)
    ↓
Formatter & Output (strace-like text)
```

### Syscall Tracing Output Format
```
[CT-001] 12.345ms SYSCALL_CAPABILITY_QUERY(cap="tool_invoke") -> granted
[CT-001] 12.456ms SYSCALL_MEMORY_ALLOCATE(size=4096) -> 0x7fff0000
[CT-001] 12.567ms SYSCALL_TOOL_INVOKE(tool="summarizer", input_len=1024) -> success
[CT-001] 12.890ms SYSCALL_MEMORY_DEALLOCATE(ptr=0x7fff0000) -> success
```

### Attachment Mechanism Design
- File descriptor-based tracing (Linux ptrace-like)
- Event stream protocol (binary format for efficiency)
- Minimal overhead: <5% slowdown when tracing
- Works with suspended and running CTs

## Dependencies
- **Blocked by:** Week 06 CI/CD, CSCI syscall interface stable
- **Blocking:** Week 10 cs-trace refinement, Week 11-12 cs-top integration

## Acceptance Criteria
- [ ] Can attach to running CT without termination
- [ ] All CSCI syscalls captured with microsecond precision
- [ ] Output format readable and comparable to strace
- [ ] Attachment overhead measured (<5%)
- [ ] Test suite demonstrates 20+ traced syscalls
- [ ] Documentation sufficient for operators to use

## Design Principles Alignment
- **Cognitive-Native:** Trace format reflects CSCI semantics, not generic syscalls
- **Debuggability:** strace-like format familiar to infrastructure engineers
- **Isolation by Default:** Tracing cannot access data outside CT isolation boundaries
- **Cost Transparency:** Trace includes timing for cost analysis
