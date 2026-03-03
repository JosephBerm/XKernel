# Engineer 10 — SDK: Tooling, Packaging & Documentation — Week 04

## Phase: 0 (Foundation & Monorepo Setup)

## Weekly Objective
Complete monorepo implementation with functional build targets. Finalize Bazel workspace structure, establish release configuration, and enable cross-platform builds. Prepare infrastructure for Phase 1 tooling development.

## Document References
- **Primary:** Section 6.1 — Phase 0, Week 5-6 (Monorepo, Bazel, CI/CD)
- **Supporting:** Section 5 (Build System: Bazel)

## Deliverables
- [ ] WORKSPACE file with all dependencies (Rust, TypeScript, C++ for kernel)
- [ ] BUILD files for all SDK layer modules with proper visibility rules
- [ ] Bazel configuration for release and debug builds
- [ ] Platform configurations (Linux x86_64, ARM64, RISC-V)
- [ ] Linker scripts for bare-metal kernel builds
- [ ] Integration test framework setup

## Technical Specifications
### WORKSPACE Configuration
- Rust toolchain pinning (specify version)
- TypeScript/Node.js dependencies (ts-sdk)
- C++ for kernel L0 components
- Benchmark framework (criterion for Rust, custom for cognitive workloads)
- Test framework unification

### Bazel Build Configuration
```
bazel build //sdk:all           # All SDK components
bazel build //sdk/tools:all     # All debugging tools
bazel build //sdk/csci:csci_lib # CSCI library only
bazel test //...                # All tests
```

### Platform Support
- Linux x86_64: primary development platform
- Linux ARM64: cloud deployment target
- RISC-V: future extensibility

## Dependencies
- **Blocked by:** Week 03 monorepo structure implementation
- **Blocking:** Week 05-06 CI/CD pipeline, Week 07-08 cs-pkg design

## Acceptance Criteria
- [ ] `bazel build //...` builds all targets without errors
- [ ] `bazel test //...` framework functional (empty test suite OK)
- [ ] Cross-platform build targets configured (at least x86_64 and ARM64)
- [ ] Visibility rules enforced: /sdk/tools cannot depend on /docs
- [ ] Release and debug build configurations work correctly
- [ ] All dependencies pinned to specific versions

## Design Principles Alignment
- **Cognitive-Native:** Build system mirrors cognitive stack isolation
- **Debuggability:** Platform-specific debugging symbols supported
- **Packaging Simplicity:** cs-pkg will consume build artifacts from Bazel
- **Open-Source Ready:** Bazel workspace suitable for external contributor setup
