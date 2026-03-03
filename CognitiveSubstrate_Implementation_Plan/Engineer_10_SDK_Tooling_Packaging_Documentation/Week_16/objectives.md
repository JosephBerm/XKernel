# Engineer 10 — SDK: Tooling, Packaging & Documentation — Week 16

## Phase: 2 (Advanced Debugging Tools & Registry)

## Weekly Objective
Refine cs-replay implementation. Optimize event stream replay performance. Add advanced debugging features (conditional breakpoints, expression evaluation). Integrate with cs-ctl.

## Document References
- **Primary:** Section 3.5.4 — cs-replay debugging tool
- **Supporting:** Section 6.3 — Phase 2, Week 15-24

## Deliverables
- [ ] cs-replay performance optimization (replay 10000+ event stream in <1 second)
- [ ] Conditional breakpoints (e.g., "break if cost_ms > 100")
- [ ] Expression evaluation in replay context (memory lookup, syscall argument inspection)
- [ ] Core dump compression (reduce size by 50%+)
- [ ] cs-ctl integration: `cs-ctl replay <core_dump>`
- [ ] Interactive debugging guide
- [ ] Replay performance benchmarks

## Technical Specifications
### Conditional Breakpoints
```bash
cs-ctl replay core_dump.cscd
> breakpoint if event.syscall == "TOOL_INVOKE" and event.cost_ms > 50
> breakpoint if event.args.capability == "memory_allocate"
> breakpoint at 25                             # Line number breakpoint
> delete breakpoint 1                          # Remove breakpoint
```

### Expression Evaluation
```bash
> print memory[0x7fff0000:0x7fff0100]         # Hex dump
> print registers                              # All CPU registers
> print syscall_args                          # Current syscall arguments
> print ct_state                               # Full CT state
> search_memory "string_pattern"               # Search memory
```

### Core Dump Compression
- Compress memory pages with zstd (compression ratio: 5:1)
- Delta compression for similar event sequences
- Target size: <10MB for typical core dump

## Dependencies
- **Blocked by:** Week 15 cs-replay prototype
- **Blocking:** Week 17-18 cs-profile, Week 19-20 cs-capgraph

## Acceptance Criteria
- [ ] Replay of 10000-event stream completes in <1 second
- [ ] Conditional breakpoints work for all syscall types
- [ ] Expression evaluation provides full CT state introspection
- [ ] Core dump compression reduces size by 50%+
- [ ] cs-ctl integration functional and well-documented
- [ ] Interactive debugging guide enables operators to debug independently

## Design Principles Alignment
- **Cognitive-Native:** Conditional breakpoints use cognitive syscall semantics
- **Debuggability:** Expression evaluation enables deep investigation of CT state
- **Isolation by Default:** Memory inspection respects CT isolation boundaries
- **Performance:** Replay performance allows interactive debugging of long executions
