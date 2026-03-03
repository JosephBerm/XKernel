# Engineer 9 — SDK: CSCI, libcognitive & SDKs — Week 22

## Phase: Phase 2

## Weekly Objective

Polish TypeScript and C# SDKs. Conduct end-to-end integration tests combining CSCI syscalls, libcognitive patterns, and adapter bridges. Prepare SDKs for Phase 3 launch.

## Document References

- **Primary:** Section 3.5.5 — TypeScript and C# SDKs; Section 6.3 — Phase 2
- **Supporting:** SDKs v0.1 (weeks 19-20); libcognitive v0.1 (week 21); adapter bridges

## Deliverables

- [ ] Run integration tests: SDK → CSCI syscalls (all 22)
- [ ] Run integration tests: SDK → libcognitive patterns → CSCI syscalls
- [ ] Optimize SDK performance: measure FFI overhead, reduce allocations
- [ ] Add SDK examples: agent creation, memory, tools, crews, error handling
- [ ] Refactor SDK code for maintainability and consistency
- [ ] Validate SDK against CSCI v1.0 and libcognitive v0.1 contracts
- [ ] Prepare SDK release candidate (v0.1.0-rc1)
- [ ] **CSCI v1.0 Published: FROZEN ABI with calling conventions (x86-64, ARM64)**
- [ ] **x86-64 calling convention: `syscall` instruction, register allocation (rdi, rsi, rdx, r10, r8, r9)**
- [ ] **ARM64 calling convention: `svc` instruction, register allocation (x0-x5 for args, x7 for syscall_id)**
- [ ] **Struct layouts frozen: ChanID, CapID, Message, CapabilityConstraints**
- [ ] **Error code taxonomy finalized and versioned**
- [ ] **Versioning guarantees: v1.x backwards compatible, v2.0+ breaks compatibility**
- [ ] **Quality bar: seL4 formal specification model (design correctness proofs)**
- [ ] **Addendum v2.5.1 — Correction 3: CSCI ABI**

## Technical Specifications

### CSCI v1.0 ABI Specification

CSCI v1.0 is the FROZEN ABI with binary calling conventions for x86-64 and ARM64.

**x86-64 System V ABI (Linux/Unix)**
```
Syscall ID encoding (rax):
  - Lower 32 bits: syscall number (0-99 reserved, 100+ user-defined)
  - bit 63: CSCI version (0 = v1.x, 1 = reserved for v2.0)

Argument registers:
  rdi = arg0
  rsi = arg1
  rdx = arg2
  r10 = arg3  (note: not rcx, due to syscall clearing rcx)
  r8  = arg4
  r9  = arg5

Return registers:
  rax = return value or error code (negative = error)
  rdx = optional return value (for 128-bit or dual returns)

Scratch registers (caller-saved):
  rax, rcx, rdx, rsi, rdi, r8-r11

Preserved registers (callee-saved):
  rbx, rsp, rbp, r12-r15

Example x86-64 syscall:
  mov rax, 100         ; syscall ID for chan_open
  mov rdi, flags       ; arg0: flags
  syscall              ; enter kernel
  cmp rax, 0
  jl  error_handler
  mov chan_id, rax     ; return value in rax
```

**ARM64 ABI (ARMv8-A)**
```
Syscall ID encoding (x8):
  - Full 64-bit: syscall number encoded

Argument registers:
  x0 = arg0
  x1 = arg1
  x2 = arg2
  x3 = arg3
  x4 = arg4
  x5 = arg5

Return registers:
  x0 = return value or error code (negative = error)
  x1 = optional return value (for 128-bit or dual returns)

Scratch registers (caller-saved):
  x0-x18

Preserved registers (callee-saved):
  x19-x28, sp, lr

Example ARM64 syscall:
  mov x0, flags        ; arg0: flags
  mov x8, #100         ; syscall ID for chan_open (100)
  svc #0               ; supervisor call (enter kernel)
  cmp x0, #0
  blt error_handler
  mov chan_id, x0      ; return value in x0
```

**Struct Layouts (Frozen for v1.0)**
```rust
// These layouts must not change within v1.x
#[repr(C)]
pub struct ChanID {
    pub id: u64,  // Unique channel identifier
}

#[repr(C)]
pub struct CapID {
    pub id: u64,  // Unique capability identifier
}

#[repr(C)]
pub struct Message {
    pub len: u32,
    pub priority: u8,
    pub reserved: [u8; 3],
    pub data: [u8; 65536],  // Max 64KB message
}

#[repr(C)]
pub struct CapabilityConstraints {
    pub expiry_time: i64,        // Nanoseconds since epoch, -1 = no expiry
    pub rate_limit: u32,         // Operations per second, 0 = unlimited
    pub cost_limit: u64,         // Total cost (tokens), 0 = unlimited
    pub resource_tags: [u64; 4], // Bitfield for resource restrictions
}

#[repr(C)]
pub struct AgentID {
    pub id: u64,
}

#[repr(C)]
pub struct ToolBindID {
    pub id: u64,
}
```

**Error Code Taxonomy (Frozen)**
```rust
pub enum CsciError {
    // Success (0)
    Success = 0,

    // Memory errors (1-10)
    NoMemory = 1,
    ResourceExhausted = 2,

    // Validation errors (11-20)
    InvalidChannel = 11,
    InvalidCapID = 12,
    InvalidAgent = 13,
    InvalidFlags = 14,
    InvalidAttenuation = 15,
    InvalidTool = 16,
    ArgumentError = 17,
    MessageTooLarge = 18,
    NoMessage = 19,

    // Communication errors (21-30)
    ChannelClosed = 21,
    Timeout = 22,
    ConnectionRefused = 23,

    // Permission errors (31-40)
    PermissionDenied = 31,
    PolicyDenied = 32,

    // Execution errors (41-50)
    ExecutionError = 41,
    SandboxError = 42,

    // Reserved for future use (51-127)
    // User-defined errors (128-255)
}

impl CsciError {
    pub fn is_error(&self) -> bool {
        *self as i32 != 0
    }

    pub fn from_code(code: i32) -> Self {
        match code {
            0 => CsciError::Success,
            1 => CsciError::NoMemory,
            // ... etc
            _ => CsciError::Unknown,
        }
    }
}
```

**Versioning Guarantees**
- CSCI v1.0 → v1.x: 100% backwards compatible (no breaking changes)
- CSCI v1.x → v2.0: Major breaking changes allowed (new ABI, new syscalls)
- SDKs track compatible CSCI versions via semantic versioning:
  - libcognitive-rust v0.1.0 supports CSCI v1.0+
  - libcognitive-typescript v0.1.0 supports CSCI v1.0+
- Kernel advertising CSCI version via `cap_get_csci_version()` syscall

**Quality Assurance: seL4 Formal Verification Model**
- CSCI design follows seL4's formal specification approach
- Key syscalls (cap_grant, cap_delegate, cap_revoke) have invariant proofs
- Proofs verified: monotonicity of capability constraints, revocation cascade completeness
- Model checking: seL4 uses Isabelle/HOL; CSCI uses Z3 SMT solver for selected invariants
- Certification: security-critical paths formally verified before v1.0 freeze

### Integration Testing & SDK Validation

- Integration tests cover: spawning agents, memory operations, IPC, tool invocation, crew coordination
- libcognitive pattern tests: ReAct with tools, CoT with memory, Reflection refinement cycles
- Adapter tests: LangChain memory adapter, SK function adapter, CrewAI crew adapter
- Performance targets: SDK overhead < 5% of task execution; FFI overhead < 10%
- SDK code style: consistent naming, comprehensive JSDoc/XML docs, error messages

## Dependencies

- **Blocked by:** Weeks 19-21
- **Blocking:** Week 23-24 (SDK v0.1 formal release)

## Acceptance Criteria

SDKs v0.1 feature-complete, performance-optimized, integration-tested

## Design Principles Alignment

- **Cognitive-Native:** All syscall interfaces designed for tight integration with CT execution engine
- **Semantic Versioning:** CSCI follows major.minor.patch; SDKs track CSCI compatibility
- **Developer Experience:** TypeScript and C# SDKs provide strongly-typed, async-first APIs with IntelliSense
- **Interoperability:** CSCI syscalls are the unified contract; SDKs bridge language ecosystems
- **Testing:** Unit tests, integration tests with kernel team, and FFI layer validation
- **Documentation:** API docs, examples, tutorials for all public surface

