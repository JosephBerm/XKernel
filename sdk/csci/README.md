# CSCI — Cognitive Substrate Computation Interface

> **Crate:** [`csci`](Cargo.toml)
> **Stream:** 4 — SDK & Tooling
> **Layer:** L3 (SDK)
> **Owner:** Engineer 10
> **Status:** Active

---

## 1. Purpose & Scope

CSCI (Cognitive Substrate Computation Interface) is the primary API for applications running on Cognitive Substrate. It provides syscall-like interfaces for spawning CTs, managing resources, using tools, and accessing semantic memory. CSCI is the boundary between user code (applications, frameworks) and the kernel.

**Key Responsibilities:**
- Syscall interface definition (CT spawn, phase transitions, etc.)
- Syscall dispatch and handler routing
- Error translation and reporting
- Capability validation at syscall boundary
- SDK bindings for Rust, TypeScript, and C#
- Documentation and code examples

**In Scope:**
- CSCI syscall definitions (in Cap'n Proto schema)
- Syscall handlers (wrappers around kernel APIs)
- SDK bindings for three languages
- API versioning and compatibility

**Out of Scope:**
- Syscall scheduling (handled by kernel scheduler)
- Actual resource allocation (handled by kernel services)
- Application logic (handled by user code)

---

## 2. Engineering Plan Reference

**Relevant Sections:**
- Section 3.5: SDK Architecture
- Section 3.5.1: CSCI Specification
- Section 3.5.2-3.5.5: Language-Specific SDKs

**Domain Model Entities Involved:**
- All 12 core entities are exposed via CSCI syscalls

See `docs/domain_model_deep_dive.md` for entity definitions.

---

## 3. Architecture & Design

### 3.1 High-Level Architecture

```
┌────────────────────────────────┐
│  User Application Code         │
│  (Rust/TypeScript/C#)          │
└────────────────────────────────┘
             ↓
┌────────────────────────────────┐
│  Language-Specific SDK Bindings│
├──────────┬──────────┬──────────┤
│  Rust    │ TypeScript │  C#    │
│   SDK    │   SDK     │  SDK   │
└──────────┴──────────┴──────────┘
             ↓
┌────────────────────────────────┐
│  CSCI Syscall Interface        │
│  (Cap'n Proto RPC)             │
└────────────────────────────────┘
             ↓
┌────────────────────────────────┐
│  Syscall Dispatch & Handlers   │
│  (Kernel entry points)         │
└────────────────────────────────┘
             ↓
┌────────────────────────────────┐
│  L0/L1/L2 Kernel & Services    │
│  (CT lifecycle, memory, etc.)  │
└────────────────────────────────┘
```

### 3.2 Key Invariants

1. **API Stability**: CSCI versions are stable and backward-compatible
   - Enforced: Semantic versioning, version negotiation at connect
   - Impact: Applications continue to work across OS upgrades

2. **Capability Enforcement at Syscall Boundary**: All syscalls check caller's capabilities
   - Enforced: Capability check in every handler
   - Impact: Untrusted code cannot escape isolation

3. **Error Transparency**: Syscall errors clearly indicate root cause
   - Enforced: Detailed error types in CSCI schema
   - Impact: Debugging easier; clear error codes for monitoring

---

## 4. Dependencies

| Crate | Type | Layer | Why |
|-------|------|-------|-----|
| `ct_lifecycle` | Internal | L0 | Implement ct_spawn, ct_phase_transition syscalls |
| `capability_engine` | Internal | L0 | Implement cap_grant, cap_revoke syscalls |
| `ipc_signals_exceptions` | Internal | L0 | Implement channel operations |
| `semantic_memory` | Internal | L1 | Implement mem_alloc, mem_free syscalls |
| `gpu_accelerator` | Internal | L1 | Implement gpu_exec syscalls |
| `tool_registry_telemetry` | Internal | L1 | Implement tool_invoke syscalls |
| `framework_adapters` | Internal | L2 | No direct dependency (used by frameworks) |
| `semantic_fs_agent_lifecycle` | Internal | L2 | Implement agent_init, fs_open syscalls |
| `libcognitive` | Internal | L3 | Higher-level SDK (built on CSCI) |

---

## 5. Public API Surface

CSCI defines syscalls via Cap'n Proto schema. Example syscalls:

```rust
/// Spawn a new CognitiveTask
pub fn csci_ct_spawn(
    request: CTSpawnRequest,
) -> CsResult<CTSpawnResponse>;

/// Transition CT to next phase
pub fn csci_ct_phase_transition(
    request: CTPhaseTransitionRequest,
) -> CsResult<CTPhaseTransitionResponse>;

/// Allocate memory
pub fn csci_mem_alloc(
    request: MemAllocRequest,
) -> CsResult<MemAllocResponse>;

/// Invoke external tool
pub fn csci_tool_invoke(
    request: ToolInvokeRequest,
) -> CsResult<ToolInvokeResponse>;

/// Open file from semantic filesystem
pub fn csci_fs_open(
    request: FSOpenRequest,
) -> CsResult<FSOpenResponse>;

/// Send message on SemanticChannel
pub fn csci_channel_send(
    request: ChannelSendRequest,
) -> CsResult<ChannelSendResponse>;

// ... ~20 more syscalls covering all major kernel operations
```

**SDK Wrapper (Rust Example):**
```rust
pub struct CognitiveSubstrate {
    connection: RpcConnection,
}

impl CognitiveSubstrate {
    pub async fn spawn_task(&self, config: CTSpawnConfig) -> CsResult<TaskHandle> {
        let req = CTSpawnRequest::from(config);
        let resp = self.connection.call(req).await?;
        Ok(TaskHandle::from(resp))
    }

    pub async fn allocate_memory(&self, size: usize) -> CsResult<MemoryHandle> {
        // ...
    }
}
```

**SDK Documentation:**
- Rust SDK: `sdk/rust/cognitive_substrate_rs/docs/`
- TypeScript SDK: `sdk/ts/packages/sdk/README.md`
- C# SDK: `sdk/csharp/CognitiveSubstrate.SDK/docs/`

---

## 6. Building & Testing

```bash
cargo build -p csci
cargo test -p csci

# Build all SDK bindings
cargo build -p cognitive_substrate_rs
npm run build --workspace  # TypeScript
dotnet build CognitiveSubstrate.SDK.sln  # C#
```

**Key Test Scenarios:**
1. Syscall dispatch — Requests routed to correct handler
2. Capability validation — Unauthorized syscalls rejected
3. Error handling — Errors translated correctly
4. Round-trip serialization — Requests/responses serialize/deserialize correctly
5. SDK bindings — All three SDKs work correctly

---

## 7. Design Decisions Log

### 7.1 "CSCI Syscalls vs. OOP SDK?"

**Decision:** CSCI defines syscalls (low-level, explicit), SDKs wrap with OOP abstractions.

**Alternatives:**
1. Pure OOP — SDK directly exposes class-based API
2. Direct function calls — No RPC, just Rust functions

**Rationale:**
- Syscalls are the natural kernel interface (like Linux)
- RPC layer enables future kernel isolation (separate processes/machines)
- SDKs can be optimized per-language (Rust → functions, Python → classes)
- Clear separation of concerns (syscall layer vs. developer API)

**Date:** 2026-03-01
**Author:** Engineer 10

### 7.2 "Cap'n Proto vs. Protobuf/gRPC?"

**Decision:** Cap'n Proto for RPC (zero-copy, pointer-based serialization).

**Alternatives:**
1. Protocol Buffers + gRPC — More mature, wider adoption
2. JSON over HTTP — Simpler but less efficient

**Rationale:**
- Cap'n Proto has near-zero deserialization overhead (pointer validation, not copying)
- Schema-driven, like Protobuf, but faster
- Native Rust support via capnp-rpc crate
- Aligns with capability-oriented architecture

**Date:** 2026-03-01
**Author:** Engineer 10

---

## 8. Performance Characteristics

| Syscall | Latency | Notes |
|---------|---------|-------|
| `ct_spawn` | ~50 µs | Includes capability validation |
| `ct_phase_transition` | ~1 µs | Just state machine update |
| `mem_alloc` | ~100 µs | Memory allocator + visibility |
| `tool_invoke` | ~10 ms | Tool execution dominates |
| `channel_send` | ~10 µs | Serialize + queue |

---

## 9. Common Pitfalls & Troubleshooting

**Mistake 1: Not checking syscall errors**
```rust
// ✗ WRONG: Ignoring errors
let handle = client.ct_spawn(config).await?;

// ✓ RIGHT: Pattern-match on error
match client.ct_spawn(config).await {
    Ok(handle) => { /* ... */ },
    Err(CsError::InsufficientCapability) => { /* Request more caps */ },
    Err(CsError::OutOfMemory) => { /* Release resources */ },
    Err(e) => { /* Log and propagate */ },
}
```

**Mistake 2: Assuming syscalls are synchronous**
```rust
// ✗ WRONG: Not awaiting
let handle = client.ct_spawn(config);  // Returns Future, doesn't execute!

// ✓ RIGHT: Await the syscall
let handle = client.ct_spawn(config).await?;
```

**Mistake 3: Not versioning API calls**
```rust
// ✗ WRONG: No version check
let client = connect_to_kernel()?;

// ✓ RIGHT: Check API version
let client = connect_to_kernel()?;
if client.api_version() < (1, 0) {
    return Err("Kernel API too old");
}
```

---

## 10. Integration Points

| Module | Integration | Protocol |
|--------|-----------|----------|
| `ct_lifecycle` | Syscall handlers for CT spawn/phase | Direct call |
| All L0/L1/L2 | Syscall handlers route to each | Direct call |
| `libcognitive` | Higher-level SDK built on CSCI | CSCI syscalls |
| All developer tools | Use CSCI to interact with kernel | CSCI syscalls |

---

## 11. API Versioning & Stability

**Semantic Versioning:**
- **Major:** Breaking changes to syscall signatures (rare)
- **Minor:** New syscalls, backward-compatible changes
- **Patch:** Bug fixes, no API changes

**Compatibility Policy:**
- CSCI v1.x guaranteed compatible with applications built for v1.0
- v2.0 may break compatibility (only if significant improvement)
- Deprecation warnings at least 6 months before removal

**Version Negotiation:**
```rust
pub fn csci_handshake(client_version: ApiVersion) -> CsResult<ApiVersion> {
    let server_version = CSCI_VERSION;
    if !compatible(client_version, server_version) {
        return Err(CsError::ApiVersionMismatch);
    }
    Ok(server_version)
}
```

---

## 12. Future Roadmap

**Planned Improvements:**
- Batch syscalls — Multiple syscalls in one RPC call
- Async syscalls — Non-blocking syscalls with callbacks
- Streaming APIs — Long-lived channels for streaming data

**Technical Debt:**
- Cap'n Proto integration still maturing (complex build rules)
- Syscall tracing overhead high when all syscalls traced
- Error types could be more specific (currently 30+ errors)

---

## 13. References

- **CSCI Specification:** `docs/csci_v0.1_specification.md`
- **Cap'n Proto:** https://capnproto.org/
- **Linux Syscalls:** https://man7.org/linux/man-pages/man2/syscalls.2.html

---

## 14. SDK Documentation

- **Rust SDK:** `sdk/libcognitive/README.md`
- **TypeScript SDK:** `sdk/ts/packages/sdk/README.md`
- **C# SDK:** `sdk/csharp/CognitiveSubstrate.SDK/README.md`

---

**README Version:** 1.0
**Last Updated:** 2026-03-01
**Owner:** Engineer 10
