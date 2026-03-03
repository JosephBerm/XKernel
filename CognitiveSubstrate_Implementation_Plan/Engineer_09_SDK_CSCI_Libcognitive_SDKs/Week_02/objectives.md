# Engineer 9 — SDK: CSCI, libcognitive & SDKs — Week 02

## Phase: Phase 0

## Weekly Objective

Continue CSCI v0.1 with IPC (chan_open, chan_send, chan_recv), Security (cap_grant, cap_delegate, cap_revoke), and Tools (tool_bind, tool_invoke) syscall families. Finalize v0.1 draft.

## Document References

- **Primary:** Section 3.5.1 — CSCI: Cognitive System Call Interface
- **Supporting:** Section 6.1 — Phase 0; kernel team feedback; FFI layer planning

## Deliverables

- [ ] Draft IPC family syscalls (chan_open, chan_send, chan_recv) with parameters, return types, error codes
- [ ] Draft Security family syscalls (cap_grant, cap_delegate, cap_revoke) with parameters, return types, error codes
- [ ] Draft Tools family syscalls (tool_bind, tool_invoke) with parameters, return types, error codes
- [ ] Complete CSCI v0.1 draft with all 14 syscalls (8 from Week 1 + 6 new)
- [ ] Submit draft for kernel team review
- [ ] **Note: CSCI ABI specification will be a SEPARATE document (following seL4/POSIX precedent)**
- [ ] **CSCI v0.1 Draft milestone: syscall names, parameter intent, return type categories**
- [ ] **Reference Redox OS model for structured TOML syscall definitions format**

## Technical Specifications

### CSCI ABI Document Strategy

CSCI is organized as TWO SEPARATE specifications:
1. **CSCI Syscall Interface (Week 2-4, Phase 0)**: Function signatures, semantics, error codes (language-independent)
2. **CSCI ABI Specification (Week 15-22, Phase 2)**: Binary calling conventions, register allocation, struct layout (implementation-specific)

This separation follows the seL4 and POSIX model:
- POSIX defines syscalls abstractly; ABIs (x86-64 SysV, ARM64 EABI) define binary details
- seL4's capDL defines capabilities logically; calling conventions are hardware-specific
- Benefits: Interface stability across implementations; ABI can evolve without syscall changes

**CSCI v0.1 Deliverable**: Syscall names, parameter intent, return type categories
```toml
# csci_v0.1_syscalls.toml — Redox OS inspired format

[syscalls]

[syscalls.chan_open]
description = "Create a new IPC channel endpoint"
parameters = [
    { name = "flags", type = "u32", intent = "input", description = "Channel flags" },
]
return_type = "Result<ChanID, Error>"
errors = ["NoMemory", "InvalidFlags", "PermissionDenied"]

[syscalls.chan_send]
description = "Send message on IPC channel"
parameters = [
    { name = "chan_id", type = "ChanID", intent = "input" },
    { name = "message", type = "Message", intent = "input" },
    { name = "timeout_ms", type = "Option<u64>", intent = "input" },
]
return_type = "Result<(), Error>"
errors = ["InvalidChannel", "ChannelClosed", "Timeout", "MessageTooLarge"]

[syscalls.chan_recv]
description = "Receive message from IPC channel"
parameters = [
    { name = "chan_id", type = "ChanID", intent = "input" },
    { name = "timeout_ms", type = "Option<u64>", intent = "input" },
]
return_type = "Result<Message, Error>"
errors = ["InvalidChannel", "ChannelClosed", "Timeout", "NoMessage"]

[syscalls.cap_grant]
description = "Grant capability to agent"
parameters = [
    { name = "target_agent", type = "AgentID", intent = "input" },
    { name = "capability", type = "Capability", intent = "input" },
    { name = "constraints", type = "CapabilityConstraints", intent = "input" },
]
return_type = "Result<CapID, Error>"
errors = ["InvalidAgent", "InvalidCapability", "PermissionDenied", "PolicyDenied"]

[syscalls.cap_delegate]
description = "Delegate capability with attenuation"
parameters = [
    { name = "cap_id", type = "CapID", intent = "input" },
    { name = "target_agent", type = "AgentID", intent = "input" },
    { name = "attenuation", type = "CapabilityConstraints", intent = "input" },
]
return_type = "Result<CapID, Error>"
errors = ["InvalidCapID", "InvalidAgent", "InvalidAttenuation", "PermissionDenied"]

[syscalls.cap_revoke]
description = "Revoke capability (all descendants)"
parameters = [
    { name = "cap_id", type = "CapID", intent = "input" },
]
return_type = "Result<(), Error>"
errors = ["InvalidCapID", "PermissionDenied"]

[syscalls.tool_bind]
description = "Bind cognitive tool (e.g., web search, code execution)"
parameters = [
    { name = "tool_spec", type = "ToolSpec", intent = "input" },
    { name = "sandbox_config", type = "SandboxConfig", intent = "input" },
]
return_type = "Result<ToolBindID, Error>"
errors = ["InvalidTool", "SandboxError", "ResourceExhausted"]

[syscalls.tool_invoke]
description = "Invoke previously bound tool"
parameters = [
    { name = "tool_bind_id", type = "ToolBindID", intent = "input" },
    { name = "arguments", type = "Map<String, String>", intent = "input" },
    { name = "timeout_ms", type = "Option<u64>", intent = "input" },
]
return_type = "Result<String, Error>"
errors = ["InvalidTool", "ArgumentError", "Timeout", "ExecutionError"]
```

### IPC, Security, Tool Syscalls

- IPC syscalls specify channel creation, type-safe message passing, backpressure handling
- Security syscalls define capability grants with delegation chains and revocation semantics
- Tool syscalls enable dynamic binding of cognitive tools (web search, code execution, etc.)
- v0.1 draft includes all 14 syscalls; remaining 8 (Signals, Telemetry, Crews) in Week 3-4

## Dependencies

- **Blocked by:** Week 1
- **Blocking:** Week 3-4 (CSCI review)

## Acceptance Criteria

Kernel team RFC complete; all 14 syscalls drafted; ready for review cycle

## Design Principles Alignment

- **Cognitive-Native:** All syscall interfaces designed for tight integration with CT execution engine
- **Semantic Versioning:** CSCI follows major.minor.patch; SDKs track CSCI compatibility
- **Developer Experience:** TypeScript and C# SDKs provide strongly-typed, async-first APIs with IntelliSense
- **Interoperability:** CSCI syscalls are the unified contract; SDKs bridge language ecosystems
- **Testing:** Unit tests, integration tests with kernel team, and FFI layer validation
- **Documentation:** API docs, examples, tutorials for all public surface

