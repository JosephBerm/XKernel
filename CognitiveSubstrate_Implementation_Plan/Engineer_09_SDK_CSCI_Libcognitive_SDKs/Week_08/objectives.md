# Engineer 9 — SDK: CSCI, libcognitive & SDKs — Week 08

## Phase: Phase 1

## Weekly Objective

Implement CSCI binding layer for ARM64 architecture. Create FFI layer for ARM64 `svc` instruction, ensuring parity with x86-64 implementation.

## Document References

- **Primary:** Section 3.5.1 — CSCI: Cognitive System Call Interface; Section 3.5.5 — SDKs
- **Supporting:** Section 6.2 — Phase 1; ARM64 calling convention (ARM EABI64); kernel syscall ABI

## Deliverables

- [ ] Design ARM64-specific syscall trampolines (inline assembly)
- [ ] Implement ARM64 syscall numbers mapping (assigned by kernel team)
- [ ] Implement ARM64 argument marshaling: TypeScript/C# types → ARM64 register layout (x0-x7)
- [ ] Ensure error code translation parity with x86-64 implementation
- [ ] Create unit tests validating syscall invocation on ARM64
- [ ] Ensure cross-architecture compatibility tests pass

## Technical Specifications

- ARM64 FFI uses same abstraction as x86-64 (different underlying trampolines)
- Syscall convention: arguments in x0/x1/x2/x3/x4/x5/x6/x7 (ARM EABI64)
- SVC instruction invoked with immediate = syscall number
- Return value in x0; error flag in x1 or carried by negative return values
- Unit tests run on ARM64 hardware or emulator (QEMU)

## Dependencies

- **Blocked by:** Week 7
- **Blocking:** Week 9-10 (libcognitive ReAct)

## Acceptance Criteria

ARM64 FFI layer complete; cross-architecture parity achieved

## Design Principles Alignment

- **Cognitive-Native:** All syscall interfaces designed for tight integration with CT execution engine
- **Semantic Versioning:** CSCI follows major.minor.patch; SDKs track CSCI compatibility
- **Developer Experience:** TypeScript and C# SDKs provide strongly-typed, async-first APIs with IntelliSense
- **Interoperability:** CSCI syscalls are the unified contract; SDKs bridge language ecosystems
- **Testing:** Unit tests, integration tests with kernel team, and FFI layer validation
- **Documentation:** API docs, examples, tutorials for all public surface

