# XKernal CSCI v1.0 Specification
## Cognitive Substrate Compatibility Interface - L3 SDK Layer
**Status:** Stable Release
**Version:** 1.0.0
**Release Date:** 2026-03-02
**Compatibility:** v1.x guaranteed source + binary compatibility

---

## 1. Overview

The Cognitive Substrate Compatibility Interface (CSCI) v1.0 is the stable, immutable foundation layer for all L3 SDK interactions across the XKernal Cognitive Substrate OS. This specification locks 22 syscall families into 8 functional categories, establishes semantic versioning guarantees, and provides the authoritative reference for library implementors across Rust, TypeScript, and C#.

**Key Guarantees:**
- All 22 syscall signatures immutable through v1.x
- Binary compatibility maintained across all v1.x minor/patch releases
- Breaking changes require v2.0+ with 2-version deprecation notice
- Error codes and enumerations locked and versioned

---

## 2. Syscall Families (22 Total, 8 Categories)

### 2.1 Task Management (4 syscalls)
**Family:** `csci_task_*`

```rust
// LOCKED v1.0 Signatures
pub extern "C" fn csci_task_spawn(
    name: *const u8,
    name_len: usize,
    entrypoint: extern "C" fn(*const u8) -> i32,
    arg: *const u8,
    cpu_affinity: u32,
) -> i32;

pub extern "C" fn csci_task_yield() -> i32;

pub extern "C" fn csci_task_cancel(task_id: u32, sig: u32) -> i32;

pub extern "C" fn csci_task_properties(
    task_id: u32,
    props_out: *mut TaskProperties,
) -> i32;
```

**TypeScript Bindings:**
```typescript
export interface TaskSpawnOptions {
  name: string;
  cpu_affinity?: number;
  timeout_ms?: number;
}

export async function taskSpawn(
  name: string,
  entrypoint: () => Promise<number>,
  options?: TaskSpawnOptions
): Promise<number>;

export async function taskYield(): Promise<void>;
export async function taskCancel(taskId: number, signal: number): Promise<void>;
```

### 2.2 Memory Management (3 syscalls)
**Family:** `csci_mem_*`

```rust
pub extern "C" fn csci_mem_alloc(
    size: usize,
    alignment: usize,
    flags: u32,
) -> *mut u8;

pub extern "C" fn csci_mem_free(ptr: *mut u8) -> i32;

pub extern "C" fn csci_mem_query(
    addr: *const u8,
    info_out: *mut MemInfo,
) -> i32;
```

**Locked Flags:**
- `CSCI_MEM_SHARED` = 0x01
- `CSCI_MEM_VOLATILE` = 0x02
- `CSCI_MEM_PRIORITY` = 0x04

### 2.3 Interprocess Communication (4 syscalls)
**Family:** `csci_ipc_*`

```rust
pub extern "C" fn csci_ipc_channel_create(
    mode: u32,
    capacity: usize,
) -> i32;

pub extern "C" fn csci_ipc_send(
    channel_id: i32,
    data: *const u8,
    data_len: usize,
    timeout_ms: u32,
) -> i32;

pub extern "C" fn csci_ipc_recv(
    channel_id: i32,
    buffer: *mut u8,
    buffer_len: usize,
    timeout_ms: u32,
) -> i32;

pub extern "C" fn csci_ipc_close(channel_id: i32) -> i32;
```

**C# Wrapper:**
```csharp
public class IPCChannel : IDisposable
{
    public int ChannelId { get; private set; }

    public static IPCChannel Create(IPCMode mode, int capacity)
    {
        var id = Native.csci_ipc_channel_create((uint)mode, (UIntPtr)capacity);
        if (id < 0) throw new IPCException(id);
        return new IPCChannel { ChannelId = id };
    }

    public void Send(byte[] data, int timeoutMs = 1000)
    {
        var result = Native.csci_ipc_send(ChannelId, data, data.Length, (uint)timeoutMs);
        if (result < 0) throw new IPCException(result);
    }

    public byte[] Recv(int bufferSize, int timeoutMs = 1000)
    {
        var buffer = new byte[bufferSize];
        var len = Native.csci_ipc_recv(ChannelId, buffer, bufferSize, (uint)timeoutMs);
        if (len < 0) throw new IPCException((int)len);
        return buffer[..(int)len];
    }
}
```

### 2.4 Security & Capabilities (3 syscalls)
**Family:** `csci_sec_*`

```rust
pub extern "C" fn csci_sec_capability_grant(
    task_id: u32,
    cap_mask: u64,
    ttl_ms: u32,
) -> i32;

pub extern "C" fn csci_sec_capability_revoke(
    task_id: u32,
    cap_mask: u64,
) -> i32;

pub extern "C" fn csci_sec_audit_log(
    event_id: u32,
    data: *const u8,
    data_len: usize,
) -> i32;
```

### 2.5 Tooling & Introspection (2 syscalls)
**Family:** `csci_tool_*`

```rust
pub extern "C" fn csci_tool_enumerate_tasks(
    buffer: *mut TaskDescriptor,
    buffer_len: usize,
) -> i32;

pub extern "C" fn csci_tool_profile_region(
    name: *const u8,
    name_len: usize,
    start_addr: *const u8,
    end_addr: *const u8,
) -> i32;
```

### 2.6 Signal Handling (2 syscalls)
**Family:** `csci_sig_*`

```rust
pub extern "C" fn csci_sig_register_handler(
    signal: u32,
    handler: extern "C" fn(u32) -> i32,
) -> i32;

pub extern "C" fn csci_sig_raise(task_id: u32, signal: u32) -> i32;
```

### 2.7 Telemetry & Observability (2 syscalls)
**Family:** `csci_telemetry_*`

```rust
pub extern "C" fn csci_telemetry_emit(
    event_type: u32,
    payload: *const u8,
    payload_len: usize,
) -> i32;

pub extern "C" fn csci_telemetry_query_metrics(
    metric_id: u32,
    out_buffer: *mut u8,
    buffer_len: usize,
) -> i32;
```

### 2.8 Crew Operations (2 syscalls)
**Family:** `csci_crew_*`

```rust
pub extern "C" fn csci_crew_create(
    name: *const u8,
    name_len: usize,
    capacity: usize,
) -> i32;

pub extern "C" fn csci_crew_join(crew_id: i32, timeout_ms: u32) -> i32;
```

---

## 3. Error Code Catalog (v1.0 Locked)

All error codes are immutable and backward compatible through v1.x.

| Code | Symbol | Meaning | Recovery |
|------|--------|---------|----------|
| `0` | `CSCI_OK` | Success | N/A |
| `-1` | `CSCI_ERR_INVALID_PARAM` | Invalid parameter | Validate inputs |
| `-2` | `CSCI_ERR_NOMEM` | Out of memory | Free resources, retry |
| `-3` | `CSCI_ERR_TIMEOUT` | Operation timeout | Increase timeout or retry |
| `-4` | `CSCI_ERR_NOTFOUND` | Resource not found | Check resource existence |
| `-5` | `CSCI_ERR_PERMISSION` | Permission denied | Request capability grant |
| `-6` | `CSCI_ERR_BUSY` | Resource busy | Yield and retry |
| `-7` | `CSCI_ERR_CLOSED` | Channel/resource closed | Reopen or recreate |
| `-8` | `CSCI_ERR_BADSTATE` | Invalid state transition | Verify preconditions |
| `-9` | `CSCI_ERR_CAPACITY` | Capacity exceeded | Increase capacity |
| `-10` | `CSCI_ERR_INVALID_SIGNAL` | Unknown signal | Use registered signals |
| `-11` | `CSCI_ERR_NETWORK` | Network fault | Check connectivity |
| `-12` | `CSCI_ERR_UNSUPPORTED` | Operation unsupported | Use v2.0+ for feature |
| `-13` | `CSCI_ERR_INTERRUPTED` | Operation interrupted | Retry operation |

---

## 4. Compatibility Guarantee Specification

### 4.1 v1.x Guarantees (Immutable)

```
CSCI v1.0.0 -- v1.99.999
├─ Syscall Signatures: LOCKED (no additions, removals, reorderings)
├─ Error Codes: LOCKED (no changes, only additions with v2.0)
├─ Data Structures: LOCKED (fields immutable, only padding additions)
├─ ABI: LOCKED (no calling convention changes)
├─ Enumerations: LOCKED (values fixed)
└─ Binary Compatibility: GUARANTEED
```

### 4.2 Deprecation Policy

**Rule:** Breaking changes require 2-version deprecation notice.

```
v1.0: Feature X introduced
v1.1: Feature X marked @deprecated (warnings only)
v1.2: Feature X warnings escalate (errors in strict mode)
v2.0: Feature X removed
```

### 4.3 Versioning Scheme

- **MAJOR.MINOR.PATCH** (SemVer)
  - **MAJOR** (breaking): Requires new v2.0+ with 2-version notice
  - **MINOR** (additive): New syscalls, non-breaking extensions
  - **PATCH** (fixes): Bug fixes, documentation updates

---

## 5. v0.5 → v1.0 Migration Guide

### 5.1 Breaking Changes: NONE

v1.0 includes all v0.5 syscalls. No removal or reordering.

### 5.2 New in v1.0

- Formal error code catalog (previously informal)
- Locked compatibility guarantees (previously provisional)
- Official binary ABI specification
- Capability TTL support in `csci_sec_capability_grant`

### 5.3 Migration Checklist

**If using v0.5:**

```rust
// Before (v0.5 style - still valid in v1.0)
let task_id = csci_task_spawn(name_ptr, name_len, entry, arg, affinity);
if task_id < 0 {
    eprintln!("Error: {}", task_id); // Error code unmapped
}

// After (v1.0 best practice)
let task_id = csci_task_spawn(name_ptr, name_len, entry, arg, affinity);
match task_id {
    CSCI_OK => { /* ... */ },
    err if err == CSCI_ERR_NOMEM => { /* retry */ },
    err if err == CSCI_ERR_PERMISSION => { /* request grant */ },
    err => eprintln!("Unexpected error: {}", err),
}
```

**All v0.5 code is source-compatible with v1.0.**

---

## 6. Type Definitions (v1.0 Locked)

```rust
#[repr(C)]
pub struct TaskProperties {
    pub task_id: u32,
    pub state: u32,
    pub priority: u32,
    pub cpu_time_us: u64,
    pub creation_time_ns: u64,
}

#[repr(C)]
pub struct MemInfo {
    pub addr: *const u8,
    pub size: usize,
    pub flags: u32,
    pub allocated_by: u32,
    pub is_resident: u8,
}

#[repr(C)]
pub struct TaskDescriptor {
    pub task_id: u32,
    pub name: [u8; 64],
    pub state: u32,
    pub parent_task_id: u32,
}

#[repr(u32)]
pub enum IPCMode {
    Unbuffered = 0,
    Buffered = 1,
    Ordered = 2,
}
```

---

## 7. Official Repository Release

**v1.0 Published As:**

```
Tag: csci/v1.0.0
Path: /xkernal/specifications/csci/v1.0/
Contents:
  ├─ SPECIFICATION.md (this document)
  ├─ SYSCALLS.rs (Rust reference)
  ├─ SYSCALLS.ts (TypeScript reference)
  ├─ SYSCALLS.cs (C# reference)
  ├─ ERROR_CODES.md
  ├─ COMPATIBILITY.md
  ├─ EXAMPLES/
  ├─ FAQ.md
  └─ TROUBLESHOOTING.md
```

**Immutability:** Read-only tag. No modifications allowed post-release.

---

## 8. Testing & Validation

### 8.1 v1.0 Compliance Matrix

- [x] All 22 syscalls have locked signatures
- [x] All error codes documented and immutable
- [x] Binary ABI validated across x86_64, ARM64
- [x] TypeScript/Rust/C# bindings generate identical call semantics
- [x] v0.5 source code compiles and runs unchanged
- [x] Capability grants with TTL verified
- [x] IPC ordering guarantees tested
- [x] Signal handler registration atomic
- [x] Telemetry payload injection validated

### 8.2 Continuous Compatibility Testing

```bash
# Automated test runs on every commit
cargo test --features=csci-v1-compat
npm test --csci-v1-validation
dotnet test --filter CSCI_V1_0_Compat
```

---

## 9. Frequently Asked Questions

**Q: Can I add new syscalls in v1.x?**
A: Yes, but only via v1.MINOR. Never reorder or remove existing syscalls.

**Q: Will v1.0 binaries run on v1.5?**
A: Yes, guaranteed. v1.x maintains full binary compatibility.

**Q: How do I request a new feature?**
A: File for v2.0 planning. v1.x is locked and immutable.

**Q: What if a bug is found in a v1.0 syscall?**
A: Fix via v1.PATCH. Signature and ABI remain unchanged; only internal behavior corrected.

---

## 10. Contact & Governance

**CSCI Steward:** XKernal SDK Team
**Review Board:** libcognitive, SDK leads (Rust/TS/C#)
**Specification Lock:** This document is authoritative as of 2026-03-02
**Next Review:** v2.0 planning (earliest Q4 2026)

---

**Document Status: STABLE - v1.0.0 LOCKED**
**Last Updated:** 2026-03-02
**Approval:** Phase 2 Complete ✓
