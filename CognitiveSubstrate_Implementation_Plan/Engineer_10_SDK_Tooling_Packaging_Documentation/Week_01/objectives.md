# Engineer 10 — SDK: Tooling, Packaging & Documentation — Week 01

## Phase: 0 (Foundation & Monorepo Setup)

## Weekly Objective
Deep domain model review and architectural alignment. Establish shared understanding of Cognitive Substrate core concepts (CSCI, CT lifecycle, capability model, resource accounting). Design monorepo structure strategy for SDK+Infra stream.

## Document References
- **Primary:** Section 6.1 — Phase 0, Week 5-6 (Monorepo, Bazel, CI/CD)
- **Supporting:** Section 3.5.3 (cs-pkg), Section 3.5.4 (Debugging Tools), Section 3.5.6 (Documentation Portal)

## Deliverables
- [ ] Domain model deep-dive documentation (CT lifecycle, CSCI syscalls, capability graph, resource accounting)
- [ ] Monorepo structure RFC with rationale for SDK+Infra stream
- [ ] Design review with core infrastructure team
- [ ] Architecture decision record (ADR-001: Monorepo Organization)

## Technical Specifications
### Domain Model Review
- Understand CT (Cognitive Task) lifecycle from creation → execution → suspension → completion
- Map all CSCI syscall categories: capability, memory, compute resource, tool invocation
- Review capability graph implementation and isolation boundaries
- Study cost model and resource accounting mechanisms

### Monorepo Structure Candidate
```
/sdk/
  /csci/              # CSCI library (Rust)
  /libcognitive/      # Core cognitive runtime (Rust)
  /ts-sdk/            # TypeScript SDK
  /cs-sdk/            # Generic language SDK
  /cs-pkg/            # Package manager
  /tools/             # Debugging tools (cs-trace, cs-replay, cs-profile, cs-capgraph, cs-top)
  /cs-ctl/            # CLI tool
```

## Dependencies
- **Blocked by:** Core infrastructure team architecture lock-in
- **Blocking:** Weeks 2-4 monorepo setup, all subsequent phases

## Acceptance Criteria
- [ ] All 5 stream engineers understand CT lifecycle and CSCI model
- [ ] Monorepo structure documented and approved by steering committee
- [ ] Dependency graph between SDK components clearly mapped
- [ ] ADR-001 rationale documented with alternatives considered

## Design Principles Alignment
- **Cognitive-Native:** Deep understanding of CT semantics and cognitive resource model
- **Isolation by Default:** Clear boundaries between SDK layers and tooling
- **Debuggability:** Foundation for trace/replay/profile tools established
- **Packaging Simplicity:** cs-pkg design influenced by monorepo structure
