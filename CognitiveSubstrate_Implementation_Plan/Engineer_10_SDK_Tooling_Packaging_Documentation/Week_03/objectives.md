# Engineer 10 — SDK: Tooling, Packaging & Documentation — Week 03

## Phase: 0 (Foundation & Monorepo Setup)

## Weekly Objective
Implement monorepo structure in code. Create all layer directories, BUILD file stubs, and baseline module structure. Begin CSCI library stubs. Establish code organization that will support all 36 weeks of development.

## Document References
- **Primary:** Section 6.1 — Phase 0, Week 5-6 (Monorepo, Bazel, CI/CD)
- **Supporting:** Section 3.5.3 (cs-pkg design), Section 3.5.4 (Debugging Tools architecture)

## Deliverables
- [ ] All monorepo directories created with BUILD file structure
- [ ] /sdk/csci/ module with syscall interface stubs (Rust, header comments only)
- [ ] /sdk/libcognitive/ module with CT lifecycle hooks
- [ ] /sdk/tools/ directory structure for 5 debugging tools
- [ ] /sdk/cs-pkg/ package manager stubs
- [ ] /docs/ directory structure (portal architecture, guides)
- [ ] Verification: `bazel build //...` works (no-op)

## Technical Specifications
### CSCI Library Stubs (Rust)
```rust
// /sdk/csci/src/lib.rs
pub mod syscall {
    // SYSCALL_CAPABILITY_QUERY
    // SYSCALL_CAPABILITY_GRANT
    // SYSCALL_MEMORY_ALLOCATE
    // SYSCALL_MEMORY_DEALLOCATE
    // SYSCALL_COMPUTE_RESERVE
    // SYSCALL_COMPUTE_RELEASE
    // SYSCALL_TOOL_INVOKE
    // SYSCALL_AGENT_SPAWN
    // [additional syscalls]
}

pub mod ct_lifecycle {
    // CT::new() -> CT
    // CT::execute() -> Result
    // CT::suspend() -> Result
    // CT::resume() -> Result
    // CT::complete() -> Result
}
```

### Debugging Tools Directory Structure
```
/sdk/tools/
├── cs-trace/
│   ├── src/
│   ├── BUILD
│   └── Cargo.toml
├── cs-replay/
├── cs-profile/
├── cs-capgraph/
├── cs-top/
└── BUILD
```

### Documentation Portal Structure
```
/docs/
├── portal/
│   ├── csci-reference/
│   ├── getting-started/
│   ├── migration-guides/
│   ├── policy-cookbook/
│   ├── adrs/
│   └── api-playground/
└── BUILD
```

## Dependencies
- **Blocked by:** Week 02 dependency policy approval
- **Blocking:** Week 04 Bazel workspace setup, Week 05-06 CI/CD pipeline

## Acceptance Criteria
- [ ] All directories created with proper BUILD files
- [ ] `bazel build //...` completes successfully (stub targets only)
- [ ] CSCI library interface documented via code comments
- [ ] 5 debugging tools have build targets (empty implementations)
- [ ] No circular dependencies detected by Bazel

## Design Principles Alignment
- **Cognitive-Native:** Monorepo mirrors cognitive stack layers
- **Debuggability:** Separate build targets for each debugging tool enable independent iteration
- **Packaging Simplicity:** cs-pkg stubs ready for registry design in Week 07-08
- **Documentation-First:** Portal structure ready for content in Phase 3
