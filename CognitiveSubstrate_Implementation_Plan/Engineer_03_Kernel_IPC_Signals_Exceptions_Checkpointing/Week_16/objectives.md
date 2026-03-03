# Engineer 3 — Kernel: IPC, Signals, Exceptions & Checkpointing — Week 16

## Phase: PHASE 2 — Optimization & Integration

## Weekly Objective

Optimize IPC performance for sub-microsecond latency on co-located request-response channels. Profile code paths, optimize hot paths, reduce memory allocations, and verify on reference hardware.

## Document References
- **Primary:** Section 7 (IPC Latency — Target: Sub-Microsecond)
- **Supporting:** Section 3.2.4 (Request-Response IPC), Section 6.2 (Exit Criteria)

## Deliverables
- [ ] Performance profiling: identify hot paths in request-response code
- [ ] Memory allocation optimization: pre-allocate request/response buffers
- [ ] Syscall overhead reduction: minimize context switches
- [ ] Cache optimization: align data structures for L1 cache efficiency
- [ ] Copy elimination: use move semantics instead of copy where possible
- [ ] Batching optimization: combine multiple syscalls into single transition
- [ ] Microbenchmark suite: measure each optimization independently
- [ ] Reference hardware benchmark: verify sub-microsecond on target platform
- [ ] Regression tests: ensure optimizations don't break functionality
- [ ] Documentation: performance tuning guide and optimization decisions

## Technical Specifications

### Request-Response Hot Path Optimization
```
// Original path (unoptimized)
fn chan_send_original(request: &[u8]) -> Result<ResponseId, SendError> {
    // 1. System call overhead
    let channel = get_channel()?;  // Kernel lookup

    // 2. Memory allocation
    let request_buf = Vec::from(request)?;

    // 3. Copy to kernel
    copy_to_kernel(&request_buf)?;

    // 4. Another syscall
    return_to_user()
}

// Optimized path (target < 1 microsecond)
fn chan_send_optimized(request: &[u8]) -> Result<ResponseId, SendError> {
    // 1. Pre-allocated buffer (stack or TLS)
    let mut request_buf = REQUEST_BUFFER_POOL.acquire();

    // 2. Single copy (DMA if available)
    request_buf.write_from_slice(request)?;

    // 3. Single syscall with inlined channel lookup
    unsafe {
        // Inline syscall to minimize context switch overhead
        ipc_send_fast(&request_buf)
    }
}
```

### Pre-Allocation Pool
```
pub struct RequestResponseBufferPool {
    pub request_buffers: Vec<AlignedBuffer<REQUEST_SIZE>>,
    pub response_buffers: Vec<AlignedBuffer<RESPONSE_SIZE>>,
    pub available_requests: std::sync::mpsc::Channel<*mut u8>,
    pub available_responses: std::sync::mpsc::Channel<*mut u8>,
}

pub struct AlignedBuffer<const SIZE: usize> {
    pub data: [u8; SIZE],
    // Aligned to cache line (64 bytes) for L1 cache efficiency
}

impl RequestResponseBufferPool {
    pub fn acquire_request(&self) -> *mut u8 {
        self.available_requests.recv().unwrap_or_else(|| {
            // Fallback: allocate new buffer (rare)
            allocate_aligned(REQUEST_SIZE)
        })
    }

    pub fn release_request(&self, buf: *mut u8) {
        let _ = self.available_requests.send(buf);
    }
}
```

### Syscall Optimization
```
// Standard syscall (3-5 microseconds overhead)
fn chan_send_standard(channel: ChannelId, request: &[u8]) -> Result<(), Error> {
    syscall_enter();           // Context switch to kernel
    // ... kernel processing
    syscall_exit();            // Context switch back to user
}

// Optimized fast syscall (< 100 nanoseconds)
#[inline(always)]
fn ipc_send_fast(request: &AlignedBuffer<REQUEST_SIZE>) -> Result<(), Error> {
    // Use syscall with minimum overhead:
    // - Inline assembly to eliminate function call overhead
    // - Direct register passing (no stack operations)
    // - Kernel processes in fast path (no general exception handler)

    let response_id: u64;
    unsafe {
        core::arch::asm!(
            "syscall",
            inout("rax") SYSCALL_IPC_SEND_FAST => response_id,
            in("rdi") request.as_ptr(),
            in("rsi") REQUEST_SIZE,
            in("rdx") channel_id,
            clobber_abi("C"),
        );
    }
    if response_id as i32 < 0 {
        Err(Error::from_syscall_code(response_id as i32))
    } else {
        Ok(())
    }
}
```

### Data Structure Alignment for Cache Efficiency
```
// Before: cache line misses
#[repr(C)]
pub struct RequestResponseChannel {
    pub id: u64,                    // 8 bytes
    pub requestor: u64,             // 8 bytes (16 total)
    pub requestee: u64,             // 8 bytes (24 total)
    pub pending_requests: Vec<u64>, // 24 bytes (48 total)
    pub stats: ChannelStats,        // Misaligned, cache miss
}

// After: aligned for L1 cache (64 byte lines)
#[repr(align(64))]
pub struct RequestResponseChannelOptimized {
    pub id: u64,                    // 8 bytes
    pub requestor: u64,             // 8 bytes
    pub requestee: u64,             // 8 bytes
    pub pending_requests: [u64; 6], // 48 bytes (hot path fits in 64 bytes)
    // stats moved to separate cache line
}
```

### Zero-Copy via Move Semantics
```
// Avoid copy: use move instead
fn process_request_zero_copy(request: AlignedBuffer<REQUEST_SIZE>) -> Result<(), Error> {
    // request is moved, not copied
    // Kernel accesses request.data directly
    // No memcpy() call
    ipc_send_fast(&request)
}

// Reuse buffer efficiently
fn handle_response_zero_copy(response: AlignedBuffer<RESPONSE_SIZE>) -> Result<(), Error> {
    // Process response in-place without copying
    let result = parse_response_header(&response)?;
    // Move buffer back to pool for reuse
    BUFFER_POOL.release_response(response);
    Ok(())
}
```

### Batching Optimization
```
// Single syscall for multiple operations
#[inline(always)]
fn chan_send_batch(requests: &[&AlignedBuffer<REQUEST_SIZE>]) -> Result<Vec<ResponseId>, Error> {
    // Create batch descriptor on stack
    let mut batch: [*const u8; 8] = [std::ptr::null(); 8];
    for (i, req) in requests.iter().enumerate().take(8) {
        batch[i] = req.as_ptr();
    }

    let response_ids: [u64; 8] = [0; 8];
    unsafe {
        core::arch::asm!(
            "syscall",
            inout("rax") SYSCALL_IPC_SEND_BATCH => _,
            in("rdi") batch.as_ptr(),
            in("rsi") requests.len(),
            in("rdx") channel_id,
            in("r8") response_ids.as_ptr(),
            clobber_abi("C"),
        );
    }

    Ok(response_ids[..requests.len()].to_vec())
}
```

### Microbenchmark Suite
```
#[cfg(test)]
mod benchmarks {
    use criterion::{black_box, criterion_group, criterion_main, Criterion};

    fn bench_request_response_original(c: &mut Criterion) {
        c.bench_function("chan_send_original", |b| {
            b.iter(|| {
                let request = black_box(&REQUEST_DATA);
                chan_send_original(request)
            });
        });
    }

    fn bench_request_response_optimized(c: &mut Criterion) {
        c.bench_function("chan_send_optimized", |b| {
            b.iter(|| {
                let request = black_box(&REQUEST_DATA);
                chan_send_optimized(request)
            });
        });
    }

    fn bench_buffer_pool(c: &mut Criterion) {
        c.bench_function("buffer_pool_acquire_release", |b| {
            b.iter(|| {
                let buf = BUFFER_POOL.acquire_request();
                BUFFER_POOL.release_request(buf);
            });
        });
    }

    criterion_group!(benches, bench_request_response_original, bench_request_response_optimized, bench_buffer_pool);
    criterion_main!(benches);
}
```

### Performance Validation on Reference Hardware
```
#[test]
fn test_ipc_latency_sub_microsecond() {
    // Hardware: Xeon Platinum 8280, 2.7 GHz, 32 cores, 64MB L3 cache
    let mut latencies = Vec::new();

    for _ in 0..10000 {
        let start = std::time::Instant::now();
        let _ = ipc_send_fast(&REQUEST_BUFFER);
        let elapsed = start.elapsed();
        latencies.push(elapsed.as_micros());
    }

    let p50 = percentile(&latencies, 50);
    let p99 = percentile(&latencies, 99);
    let p999 = percentile(&latencies, 99.9);

    // Target: p50 < 1 microsecond
    println!("P50: {} us, P99: {} us, P999: {} us", p50, p99, p999);
    assert!(p50 < 1, "P50 latency must be < 1 microsecond");
    assert!(p99 < 5, "P99 latency must be < 5 microseconds");
}
```

## Dependencies
- **Blocked by:** Week 3 (Request-Response IPC)
- **Blocking:** Week 17-18 (Fault Recovery Optimization)

## Acceptance Criteria
1. P50 latency for request-response < 1 microsecond
2. P99 latency < 5 microseconds
3. No memory allocations in hot path
4. Buffer pool reduces allocation overhead by > 90%
5. Data structures aligned to 64-byte cache lines
6. Optimized path > 10x faster than original
7. All functionality preserved; no correctness regressions
8. Microbenchmarks show each optimization contributes measurably
9. Profiler confirms hot paths optimized
10. Results documented with before/after comparisons

## Design Principles Alignment
- **Performance:** Sub-microsecond IPC enables fast agent coordination
- **Efficiency:** Pre-allocation eliminates allocation overhead
- **Predictability:** Cache-aligned data structures reduce latency variance
- **Optimization:** Inline assembly and fast syscalls minimize context switches
