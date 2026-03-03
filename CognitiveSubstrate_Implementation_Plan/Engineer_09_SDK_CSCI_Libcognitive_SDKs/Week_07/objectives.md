# Engineer 9 — SDK: CSCI, libcognitive & SDKs — Week 07

## Phase: Phase 1

## Weekly Objective

Implement CSCI binding layer for x86-64 architecture. Create thin FFI layer that traps into kernel via x86-64 `syscall` instruction, translating SDK method calls into raw syscall invocations.

## Document References

- **Primary:** Section 3.5.1 — CSCI: Cognitive System Call Interface; Section 3.5.5 — SDKs
- **Supporting:** Section 6.2 — Phase 1; kernel syscall numbers and ABI specification; x86-64 calling convention

## Deliverables

- [ ] Design CSCI binding layer architecture (FFI abstraction, error mapping, argument marshaling)
- [ ] Implement x86-64 syscall trampolines (inline assembly or ctypes/P/Invoke)
- [ ] Map all 22 CSCI syscalls to x86-64 syscall numbers (assigned by kernel team)
- [ ] Implement argument marshaling: TypeScript/C# types → x86-64 register/stack layout
- [ ] Implement error code translation: kernel error codes → language exceptions
- [ ] Create unit tests validating syscall invocation for each of 22 syscalls

## Technical Specifications

- TypeScript FFI uses WebAssembly + WASM bindings or Node.js N-API for native calls
- C# FFI uses P/Invoke with DllImport for native .so/.dll
- Error mapping: CS_EUNIMPL → NotImplementedError, CS_EPERM → PermissionError, etc.
- Syscall convention: arguments in rdi/rsi/rdx/rcx/r8/r9 (System V x86-64 ABI)
- All 22 syscalls testable without full kernel (stub kernel traps)

## Dependencies

- **Blocked by:** Week 6
- **Blocking:** Week 8 (ARM64 FFI); Week 9-10 (libcognitive ReAct)

## Acceptance Criteria

x86-64 FFI layer complete; all 22 syscalls callable from TypeScript and C#

## Design Principles Alignment

- **Cognitive-Native:** All syscall interfaces designed for tight integration with CT execution engine
- **Semantic Versioning:** CSCI follows major.minor.patch; SDKs track CSCI compatibility
- **Developer Experience:** TypeScript and C# SDKs provide strongly-typed, async-first APIs with IntelliSense
- **Interoperability:** CSCI syscalls are the unified contract; SDKs bridge language ecosystems
- **Testing:** Unit tests, integration tests with kernel team, and FFI layer validation
- **Documentation:** API docs, examples, tutorials for all public surface

