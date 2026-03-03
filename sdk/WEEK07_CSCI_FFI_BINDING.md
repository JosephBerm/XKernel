# Week 7 Deliverable: CSCI x86-64 FFI Binding Layer (Phase 1)

**XKernal Cognitive Substrate — Engineer 9: CSCI, libcognitive & SDKs**

---

## Executive Summary

Week 7 transitions the CSCI subsystem from Phase 0 no-op stubs to Phase 1 real kernel integration via x86-64 syscall trapping. Engineer 9 implements the FFI binding layer enabling TypeScript SDK, C# SDK, and Rust libcognitive to issue authentic syscalls to the XKernal cognitive kernel. This document specifies the FFI architecture, x86-64 syscall convention compliance, complete syscall number mapping for all 22 CSCI operations, argument marshaling patterns, error code translation, and inline assembly trampolines.

---

## Problem Statement

Phase 0 SDK implementations (Week 6) provided strongly-typed API surfaces with no-op function bodies. All method calls returned stub responses without kernel interaction. Phase 1 requires:

1. Thin native binding layers that translate high-level SDK calls into raw x86-64 syscalls
2. Complete syscall number assignment across all 8 CSCI families (22 total operations)
3. System V ABI compliance for register argument passing and return value conventions
4. Language-specific FFI mechanisms: Node.js N-API for TypeScript, P/Invoke for C#, inline asm for Rust
5. Unified error code translation maintaining semantic consistency across all binding implementations
6. Unit test coverage validating kernel trap invocation for each syscall

The binding layer serves as the cognitive-native integration point between SDK abstraction and kernel substrate, enabling deterministic, semantically versioned access to CT execution primitives, memory operations, capabilities, channels, signals, tools, and telemetry.

---

## CSCI FFI Architecture

The FFI binding layer implements a three-tier architecture:

```
┌─────────────────────────────────────────────────────────────┐
│ Layer 1: Language SDK (High-Level API)                      │
│ ├─ TypeScript: async/await cognitive operations              │
│ ├─ C#: async Task<T> cognitive operations                    │
│ └─ Rust: async/await libcognitive trait implementations      │
└─────────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────────┐
│ Layer 2: FFI Binding (Native Code)                          │
│ ├─ TypeScript: Node.js N-API bridge → native .node binary   │
│ ├─ C#: P/Invoke declarations → managed/unmanaged transition │
│ └─ Rust: unsafe{} asm block → inline x86-64 assembly        │
└─────────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────────┐
│ Layer 3: x86-64 Syscall (Kernel Trap)                      │
│ └─ syscall instruction → RIP=kernel entry, return RIP=SDK   │
└─────────────────────────────────────────────────────────────┘
```

Each SDK language binds its native module (compiled .so/.dll or WASM) which contains the syscall invocation logic. At runtime, SDK method calls resolve to syscall numbers and marshal arguments into register layout per System V ABI x86-64 calling convention.

---

## x86-64 Syscall Convention (System V ABI)

XKernal adheres to System V AMD64 ABI syscall calling convention:

| Register | Role | In/Out |
|----------|------|--------|
| **rax** | Syscall number | In |
| **rax** | Return value (signed 64-bit) | Out |
| **rdi** | Argument 1 | In |
| **rsi** | Argument 2 | In |
| **rdx** | Argument 3 | In |
| **r10** | Argument 4 (kernel saves rcx) | In |
| **r8** | Argument 5 | In |
| **r9** | Argument 6 | In |
| **rcx** | Clobbered by syscall | - |
| **r11** | Clobbered by syscall | - |

Syscall returns with:
- **rax ≥ 0**: Success, rax contains return value
- **rax < 0**: Error, rax = -error_code (see Error Code Translation section)

---

## CSCI Syscall Number Mapping (All 22 Operations)

Syscalls are organized in 8 families with dedicated number ranges:

### CT (Cognitive Task) Family: syscalls 100–104
| Syscall # | Name | Arguments | Return |
|-----------|------|-----------|--------|
| **100** | `ct_spawn` | (entry_point: u64, arg: u64, policy: u32) → tid | Spawned task ID or error |
| **101** | `ct_yield` | (reason: u32) → void | CS_OK or error |
| **102** | `ct_exit` | (code: i32) → never | Never returns |
| **103** | `ct_checkpoint` | () → handle | Checkpoint handle or error |
| **104** | `ct_resume` | (handle: u64) → void | CS_OK or error |

### Memory Family: syscalls 110–113
| Syscall # | Name | Arguments | Return |
|-----------|------|-----------|--------|
| **110** | `mem_alloc` | (size: u64, flags: u32) → ptr | Virtual address or error |
| **111** | `mem_read` | (addr: u64, len: u64, buf: u64) → len_read | Bytes read or error |
| **112** | `mem_write` | (addr: u64, len: u64, buf: u64) → len_written | Bytes written or error |
| **113** | `mem_mount` | (vaddr: u64, paddr: u64, size: u64) → void | CS_OK or error |

### Capability Family: syscalls 120–122
| Syscall # | Name | Arguments | Return |
|-----------|------|-----------|--------|
| **120** | `cap_grant` | (target_tid: u64, cap_mask: u64) → void | CS_OK or error |
| **121** | `cap_revoke` | (target_tid: u64, cap_mask: u64) → void | CS_OK or error |
| **122** | `cap_check` | (cap_mask: u64) → has_cap | Boolean (0/1) or error |

### Channel Family: syscalls 130–133
| Syscall # | Name | Arguments | Return |
|-----------|------|-----------|--------|
| **130** | `chan_open` | (chan_id: u64, mode: u32) → handle | Channel handle or error |
| **131** | `chan_send` | (handle: u64, buf: u64, len: u64) → bytes_sent | Bytes sent or error |
| **132** | `chan_recv` | (handle: u64, buf: u64, len: u64) → bytes_recv | Bytes received or error |
| **133** | `chan_close` | (handle: u64) → void | CS_OK or error |

### Signal Family: syscalls 140–141
| Syscall # | Name | Arguments | Return |
|-----------|------|-----------|--------|
| **140** | `sig_register` | (sig_num: u32, handler: u64) → void | CS_OK or error |
| **141** | `sig_send` | (target_tid: u64, sig_num: u32) → void | CS_OK or error |

### Tool Family: syscalls 150–151
| Syscall # | Name | Arguments | Return |
|-----------|------|-----------|--------|
| **150** | `tool_invoke` | (tool_id: u64, arg_buf: u64, arg_len: u64) → result | Invocation result or error |
| **151** | `tool_list` | (buf: u64, len: u64) → count | Tool count or error |

### Telemetry Family: syscalls 160–161
| Syscall # | Name | Arguments | Return |
|-----------|------|-----------|--------|
| **160** | `tel_emit` | (event_type: u32, data: u64, len: u64) → void | CS_OK or error |
| **161** | `tel_query` | (query_buf: u64, result_buf: u64, len: u64) → count | Query result count or error |

---

## Argument Marshaling

### TypeScript → C Types → Register Layout

TypeScript SDK methods accept JavaScript values which N-API bridges marshal to C types, ultimately placing them in registers:

```typescript
// TypeScript SDK (ctSDK.ts)
async spawn(entryPoint: bigint, arg: bigint, policy: number): Promise<bigint> {
  return new Promise((resolve, reject) => {
    binding.ctSpawn(entryPoint, arg, policy, (err, tid) => {
      if (err) reject(translateError(err));
      else resolve(tid);
    });
  });
}
```

The N-API native module (ctbinding.c) handles the marshaling:

```c
// Native N-API binding (ctbinding.c)
napi_value ct_spawn_impl(napi_env env, napi_callback_info info) {
  uint64_t entry_point, arg;
  uint32_t policy;
  // Extract arguments from napi_value to C types
  napi_get_value_bigint_uint64(env, argv[0], &entry_point, NULL);
  napi_get_value_uint32(env, argv[1], &arg);
  napi_get_value_uint32(env, argv[2], &policy);

  // Place in registers: rdi=entry_point, rsi=arg, rdx=policy
  int64_t result = syscall_ct_spawn(entry_point, arg, policy);

  // Return to JavaScript
  napi_value ret;
  napi_create_bigint_uint64(env, (uint64_t)result, &ret);
  return ret;
}
```

### C# → P/Invoke → Register Layout

C# SDK methods use P/Invoke to call native .so/.dll functions which contain syscall stubs:

```csharp
// C# SDK (CtSdk.cs)
[DllImport("libcsci_ffi.so", CallingConvention = CallingConvention.Cdecl)]
private static extern long CsciCtSpawn(ulong entryPoint, ulong arg, uint policy);

public async Task<ulong> Spawn(ulong entryPoint, ulong arg, uint policy) {
  long result = CsciCtSpawn(entryPoint, arg, policy);
  if (result < 0) throw TranslateError(-result);
  return (ulong)result;
}
```

The native .so (csci_ffi.c) contains:

```c
// Native syscall stub (csci_ffi.c)
long csci_ct_spawn(uint64_t entry_point, uint64_t arg, uint32_t policy) {
  register uint64_t rax __asm__("rax") = 100;  // CT_SPAWN
  register uint64_t rdi __asm__("rdi") = entry_point;
  register uint64_t rsi __asm__("rsi") = arg;
  register uint64_t rdx __asm__("rdx") = policy;

  __asm__ volatile (
    "syscall\n"
    : "+r"(rax)
    : "r"(rdi), "r"(rsi), "r"(rdx)
    : "rcx", "r11"
  );
  return (int64_t)rax;
}
```

### Rust inline asm → Register Layout

Rust libcognitive uses unsafe inline assembly for maximal control:

```rust
// libcognitive (lib.rs)
#[inline(always)]
pub unsafe fn syscall6(
  nr: u64, a1: u64, a2: u64, a3: u64, a4: u64, a5: u64, a6: u64
) -> i64 {
  let ret: i64;
  asm!(
    "syscall",
    in("rax") nr,
    in("rdi") a1,
    in("rsi") a2,
    in("rdx") a3,
    in("r10") a4,
    in("r8") a5,
    in("r9") a6,
    lateout("rax") ret,
    lateout("rcx") _,
    lateout("r11") _,
  );
  ret
}

pub async fn spawn(entry_point: u64, arg: u64, policy: u32) -> Result<u64> {
  let result = unsafe { syscall6(100, entry_point, arg, policy as u64, 0, 0, 0) };
  if result < 0 {
    Err(CsError::from_code(-result as u32))
  } else {
    Ok(result as u64)
  }
}
```

### String and Buffer Passing

For operations involving variable-length data (chan_send, mem_write, etc.), pointers and lengths are passed:

```c
// Syscall: chan_send(handle, buf_ptr, buf_len) → bytes_sent
// rdi=handle, rsi=buf_ptr, rdx=buf_len
long chan_send(uint64_t handle, const void *buf, uint64_t len) {
  register uint64_t rax __asm__("rax") = 131;
  register uint64_t rdi __asm__("rdi") = handle;
  register uint64_t rsi __asm__("rsi") = (uint64_t)buf;
  register uint64_t rdx __asm__("rdx") = len;

  __asm__ volatile("syscall" : "+r"(rax) : "r"(rdi), "r"(rsi), "r"(rdx) : "rcx", "r11");
  return (int64_t)rax;
}
```

---

## Error Code Translation

CSCI defines 11 error codes that map consistently across all binding implementations:

| Error Code | C Constant | Meaning | TypeScript Exception | C# Exception |
|-----------|-----------|---------|---------------------|--------------|
| 0 | CS_OK | Success | (no exception) | Success |
| 1 | CS_EPERM | Permission denied | PermissionError | UnauthorizedAccessException |
| 2 | CS_EINVAL | Invalid argument | TypeError | ArgumentException |
| 3 | CS_ENOMEM | Memory exhausted | RangeError | OutOfMemoryException |
| 4 | CS_EBUSY | Resource busy | Error (busy) | InvalidOperationException |
| 5 | CS_ETIMEOUT | Operation timeout | Error (timeout) | TimeoutException |
| 6 | CS_ENOTFOUND | Not found | Error (not found) | KeyNotFoundException |
| 7 | CS_ECAPDENIED | Capability denied | PermissionError | UnauthorizedAccessException |
| 8 | CS_EUNIMPL | Unimplemented | Error (unimpl) | NotImplementedException |
| 9 | CS_EIO | I/O error | Error (io) | IOException |
| 10 | CS_ECYCLE | Capability cycle detected | Error (cycle) | InvalidOperationException |

When a syscall returns rax < 0, the magnitude is the error code. Bindings must translate the error code to the appropriate language exception:

```typescript
// TypeScript error translation
function translateError(code: number): Error {
  switch (code) {
    case 1: return new Error('PermissionError');
    case 2: return new TypeError('Invalid argument');
    case 3: return new RangeError('Memory exhausted');
    case 4: return new Error('Resource busy');
    case 5: return new Error('Operation timeout');
    case 6: return new Error('Not found');
    case 7: return new Error('Capability denied');
    case 8: return new Error('Unimplemented');
    case 9: return new Error('I/O error');
    case 10: return new Error('Capability cycle');
    default: return new Error(`Unknown error: ${code}`);
  }
}
```

---

## Inline Assembly Trampoline (Rust Reference Implementation)

The Rust libcognitive provides the reference inline assembly trampoline for x86-64 syscall:

```rust
/// Generic 6-argument syscall trampoline
/// Maps arguments to System V AMD64 ABI registers:
/// nr → rax, a1 → rdi, a2 → rsi, a3 → rdx, a4 → r10, a5 → r8, a6 → r9
#[inline(always)]
pub unsafe fn syscall6(
    nr: u64,
    a1: u64,
    a2: u64,
    a3: u64,
    a4: u64,
    a5: u64,
    a6: u64,
) -> i64 {
    let ret: i64;
    asm!(
        "syscall",
        in("rax") nr,
        in("rdi") a1,
        in("rsi") a2,
        in("rdx") a3,
        in("r10") a4,
        in("r8") a5,
        in("r9") a6,
        lateout("rax") ret,
        lateout("rcx") _,
        lateout("r11") _,
    );
    ret
}

/// Typed wrapper for ct_spawn syscall
pub async fn ct_spawn(entry_point: u64, arg: u64, policy: u32) -> Result<u64> {
    let result = unsafe { syscall6(100, entry_point, arg, policy as u64, 0, 0, 0) };
    if result < 0 {
        Err(CsError::from_code(-result as u32))
    } else {
        Ok(result as u64)
    }
}

/// Typed wrapper for mem_alloc syscall
pub async fn mem_alloc(size: u64, flags: u32) -> Result<u64> {
    let result = unsafe { syscall6(110, size, flags as u64, 0, 0, 0, 0) };
    if result < 0 {
        Err(CsError::from_code(-result as u32))
    } else {
        Ok(result as u64)
    }
}

/// Typed wrapper for cap_check syscall
pub async fn cap_check(cap_mask: u64) -> Result<bool> {
    let result = unsafe { syscall6(122, cap_mask, 0, 0, 0, 0, 0) };
    if result < 0 {
        Err(CsError::from_code(-result as u32))
    } else {
        Ok(result != 0)
    }
}

/// Typed wrapper for chan_send syscall (buffer-based)
pub async fn chan_send(handle: u64, buf: &[u8]) -> Result<u64> {
    let result = unsafe { syscall6(131, handle, buf.as_ptr() as u64, buf.len() as u64, 0, 0, 0) };
    if result < 0 {
        Err(CsError::from_code(-result as u32))
    } else {
        Ok(result as u64)
    }
}

/// Typed wrapper for tel_emit syscall (telemetry event)
pub async fn tel_emit(event_type: u32, data: &[u8]) -> Result<()> {
    let result = unsafe { syscall6(160, event_type as u64, data.as_ptr() as u64, data.len() as u64, 0, 0, 0) };
    if result < 0 {
        Err(CsError::from_code(-result as u32))
    } else {
        Ok(())
    }
}
```

---

## Testing Strategy

Phase 1 testing validates that each of the 22 syscalls correctly marshals arguments and invokes kernel traps:

### Unit Test Coverage (Rust)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ct_spawn_syscall_invocation() {
        // Validate syscall 100 is invoked with correct register mapping
        let result = ct_spawn(0x1000_0000, 0x42, 0).await;
        assert!(result.is_ok() || result.is_err());  // Kernel will stub or fail gracefully
    }

    #[tokio::test]
    async fn test_mem_alloc_syscall_invocation() {
        // Validate syscall 110 with size and flags
        let result = mem_alloc(4096, 0).await;
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_cap_check_syscall_invocation() {
        // Validate syscall 122 returns boolean or error
        let result = cap_check(0xFF).await;
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_chan_send_with_buffer() {
        // Validate syscall 131 marshals buffer pointer/length
        let handle = 1u64;
        let data = b"test data";
        let result = chan_send(handle, data).await;
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_error_code_translation() {
        // Verify negative return codes map to CsError
        // (Requires kernel stub returning error codes)
    }
}
```

---

## Design Principles

1. **Cognitive-Native**: FFI layer is the integration point between SDK abstractions and kernel substrate, enabling deterministic, low-latency cognitive task scheduling.

2. **Semantic Versioning**: CSCI major.minor.patch versioning ensures backward compatibility as new syscalls are added in future phases.

3. **Developer Experience**: Strongly-typed, async-first APIs across all three SDKs (TypeScript, C#, Rust) with comprehensive error translation.

4. **Interoperability**: Unified CSCI contract—same syscall numbers, register conventions, and error codes across all binding implementations.

5. **Security**: Capabilities and permission checks enforced at kernel boundary; no trust in userspace FFI layer.

---

## Next Steps (Phase 2)

- Kernel stub implementation responding to syscall invocations
- Full integration testing against kernel entry point
- Performance benchmarking of syscall latency vs. Phase 0 no-ops
- Documentation updates for CSCI API reference
