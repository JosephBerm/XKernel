# Week 15: GPU Checkpoint/Restore Integration
## XKernal Cognitive Substrate OS - L1 Services (Rust)

**Phase:** 2 (Week 1)
**Author:** Staff-Level Engineer (GPU/Accelerator Manager)
**Date:** 2026-03-02
**Status:** Design Specification

---

## Executive Summary

Week 15 establishes GPU checkpoint/restore (C/R) as a non-blocking service for continuous inference execution. Inspired by PhoenixOS's checkpoint model, we implement concurrent C/R that operates transparently alongside active kernel execution without latency impact. Target: sub-100ms checkpoint latency for 20GB VRAM with concurrent speculative detection of GPU memory mutations via CUDA API interception.

---

## 1. Architecture Overview

### 1.1 Design Principles

**Non-Blocking C/R:** Checkpoint operations execute asynchronously while kernels run. No inference pause.

**Transparent Optimization:** CUDA API interception detects memory mutations without explicit tracking instrumentation.

**Memory Efficiency:** Soft Copy-on-Write (CoW) for GPU memory reduces checkpoint footprint and DMA bandwidth.

### 1.2 System Architecture

```
┌─────────────────────────────────────────────────────────┐
│         GPU Checkpoint/Restore Service                  │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ┌──────────────────┐  ┌──────────────────────┐        │
│  │  C/R State       │  │  CUDA API            │        │
│  │  Machine (FSM)   │  │  Interception Layer  │        │
│  └──────────────────┘  └──────────────────────┘        │
│         │                        │                      │
│         ├────────────┬───────────┤                      │
│         │            │           │                      │
│  ┌──────▼──────┐ ┌──▼────────┐ ┌▼─────────────┐        │
│  │ Checkpoint  │ │  Soft CoW │ │ Mutation     │        │
│  │ Buffer Pool │ │  Manager  │ │ Detection    │        │
│  └─────────────┘ └───────────┘ └──────────────┘        │
│         │              │              │                │
│         └──────────────┼──────────────┘                │
│                        │                               │
│         ┌──────────────▼──────────────┐                │
│         │  Checkpoint Format Engine   │                │
│         │  (Serialization/Streaming)  │                │
│         └─────────────────────────────┘                │
│                        │                               │
│         ┌──────────────▼──────────────┐                │
│         │  GPU Memory Persistence     │                │
│         │  (Buffer, File, Remote)     │                │
│         └─────────────────────────────┘                │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

---

## 2. Checkpoint/Restore State Machine

The C/R FSM operates in 5 states, enabling concurrent checkpoint while kernels execute:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckpointState {
    /// Idle, no checkpoint in progress
    Idle,
    /// Preparing snapshot metadata and tracking structures
    PreparingMetadata,
    /// Streaming device memory asynchronously
    StreamingMemory,
    /// Finalizing checkpoint, verifying integrity
    Finalizing,
    /// Checkpoint complete, ready for restore
    Complete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestoreState {
    Idle,
    /// Verifying checkpoint integrity and compatibility
    Verifying,
    /// Restoring kernel context and device state
    RestoringKernels,
    /// Streaming memory back to device
    RestoringMemory,
    /// Resuming execution from checkpointed state
    Resuming,
}

pub struct CheckpointRestoreStateMachine {
    checkpoint_state: CheckpointState,
    restore_state: RestoreState,
    active_checkpoint_id: Option<u64>,
    mutation_tracker: Arc<Mutex<MutationTracker>>,
    cuda_interceptor: Arc<CudaApiInterceptor>,
}

impl CheckpointRestoreStateMachine {
    /// Initiate non-blocking checkpoint while kernels execute
    pub async fn begin_concurrent_checkpoint(&mut self) -> Result<u64, CheckpointError> {
        if self.checkpoint_state != CheckpointState::Idle {
            return Err(CheckpointError::AlreadyInProgress);
        }

        self.checkpoint_state = CheckpointState::PreparingMetadata;
        let checkpoint_id = self.allocate_checkpoint_id();
        self.active_checkpoint_id = Some(checkpoint_id);

        // Enable mutation tracking on CUDA API calls
        self.mutation_tracker.lock().await.enable_tracking(checkpoint_id);
        self.cuda_interceptor.enable_interception(checkpoint_id);

        self.checkpoint_state = CheckpointState::StreamingMemory;

        // Spawn async streaming task—does not block kernel execution
        let streaming_handle = tokio::spawn(self.stream_device_memory_async(checkpoint_id));

        Ok(checkpoint_id)
    }

    /// Poll checkpoint progress without blocking
    pub async fn poll_checkpoint_progress(&self, checkpoint_id: u64) -> Result<f32, CheckpointError> {
        match self.checkpoint_state {
            CheckpointState::Idle => Err(CheckpointError::NoCheckpointActive),
            CheckpointState::PreparingMetadata => Ok(0.05),
            CheckpointState::StreamingMemory => {
                // Query Soft CoW manager for streaming progress
                let progress = self.soft_cow_manager.get_streaming_progress(checkpoint_id)?;
                Ok(0.1 + progress * 0.8)
            }
            CheckpointState::Finalizing => Ok(0.95),
            CheckpointState::Complete => Ok(1.0),
        }
    }

    /// Transition to finalization after streaming completes
    async fn finalize_checkpoint(&mut self, checkpoint_id: u64) -> Result<(), CheckpointError> {
        self.checkpoint_state = CheckpointState::Finalizing;

        let mutations = self.mutation_tracker.lock().await.finalize(checkpoint_id)?;
        self.cuda_interceptor.finalize_interception(checkpoint_id);

        self.checkpoint_format_engine.finalize_checkpoint(checkpoint_id, mutations)?;

        self.checkpoint_state = CheckpointState::Complete;
        Ok(())
    }
}
```

---

## 3. CUDA API Interception Mechanism

Speculative detection of GPU memory read/write operations without explicit instrumentation:

```rust
pub struct CudaApiInterceptor {
    /// Hooked function pointers for CUDA Driver API
    intercepted_functions: Arc<Mutex<HashMap<String, *const ()>>>,
    /// Per-checkpoint mutation log
    mutation_logs: Arc<Mutex<HashMap<u64, Vec<MemoryMutation>>>>,
    /// Current active checkpoints
    active_checkpoints: Arc<Mutex<HashSet<u64>>>,
}

#[derive(Debug, Clone)]
pub struct MemoryMutation {
    pub timestamp_ns: u64,
    pub device_ptr: u64,
    pub size: u64,
    pub mutation_type: MutationType,
    pub kernel_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutationType {
    MemcpyHostToDevice,
    MemcpyDeviceToHost,
    MemcpyDeviceToDevice,
    KernelWrite,      // Detected via register pressure analysis
    MemsetAsync,
    MemallocAsync,
}

impl CudaApiInterceptor {
    /// Hook CUDA Driver API functions for mutation tracking
    pub fn enable_interception(&self, checkpoint_id: u64) -> Result<(), InterceptionError> {
        self.active_checkpoints.blocking_lock().insert(checkpoint_id);

        // Hook cuMemcpyHtoD_v2 and variants
        self.install_hook("cuMemcpyHtoD_v2", Self::hooked_memcpy_htod)?;
        self.install_hook("cuMemcpyDtoH_v2", Self::hooked_memcpy_dtoh)?;
        self.install_hook("cuMemcpyDtoD_v2", Self::hooked_memcpy_dtod)?;
        self.install_hook("cuMemsetD32_v2", Self::hooked_memset)?;
        self.install_hook("cuLaunchKernel_v2", Self::hooked_launch_kernel)?;

        Ok(())
    }

    /// Intercepted cuMemcpyHtoD—logs host-to-device transfers
    extern "C" fn hooked_memcpy_htod(
        dstDevice: *mut u8,
        srcHost: *const u8,
        ByteCount: usize,
    ) -> i32 {
        let checkpoint_id = CURRENT_CHECKPOINT_ID.load(Ordering::Relaxed);
        if checkpoint_id == 0 {
            // No active checkpoint, call original
            return unsafe { ORIGINAL_MEMCPY_HTOD(dstDevice, srcHost, ByteCount) };
        }

        let mutation = MemoryMutation {
            timestamp_ns: get_timestamp_ns(),
            device_ptr: dstDevice as u64,
            size: ByteCount as u64,
            mutation_type: MutationType::MemcpyHostToDevice,
            kernel_id: get_current_kernel_id(),
        };

        // Log mutation without blocking
        let _ = MUTATION_LOGS.try_lock().map(|mut logs| {
            logs.entry(checkpoint_id)
                .or_insert_with(Vec::new)
                .push(mutation);
        });

        unsafe { ORIGINAL_MEMCPY_HTOD(dstDevice, srcHost, ByteCount) }
    }

    /// Intercepted cuLaunchKernel—tracks kernel-side mutations
    extern "C" fn hooked_launch_kernel(
        f: *mut ::std::os::raw::c_void,
        gridDimX: u32,
        gridDimY: u32,
        gridDimZ: u32,
        blockDimX: u32,
        blockDimY: u32,
        blockDimZ: u32,
        sharedMemBytes: u32,
        hStream: *mut ::std::os::raw::c_void,
        kernelParams: *mut *mut ::std::os::raw::c_void,
        extra: *mut *mut ::std::os::raw::c_void,
    ) -> i32 {
        let checkpoint_id = CURRENT_CHECKPOINT_ID.load(Ordering::Relaxed);
        let kernel_id = allocate_kernel_id();

        if checkpoint_id != 0 {
            // Analyze kernel parameters for writable GPU pointers
            if let Ok(mut params) = KERNEL_PARAMETER_ANALYZER.try_lock() {
                params.analyze_write_regions(kernelParams, kernel_id, checkpoint_id);
            }
        }

        unsafe {
            ORIGINAL_LAUNCH_KERNEL(
                f, gridDimX, gridDimY, gridDimZ,
                blockDimX, blockDimY, blockDimZ,
                sharedMemBytes, hStream, kernelParams, extra,
            )
        }
    }

    /// Finalize interception, prepare mutation log for checkpoint
    pub fn finalize_interception(&self, checkpoint_id: u64) {
        self.active_checkpoints.blocking_lock().remove(&checkpoint_id);

        if self.active_checkpoints.blocking_lock().is_empty() {
            // Uninstall hooks when no checkpoints active
            let _ = self.uninstall_all_hooks();
        }
    }
}
```

---

## 4. Soft Copy-on-Write for GPU Memory

Deferred GPU memory copying reduces checkpoint footprint and DMA bandwidth:

```rust
pub struct SoftCowManager {
    /// Pages marked for CoW, mapped device_ptr → page_state
    cow_pages: Arc<Mutex<HashMap<u64, CowPageState>>>,
    /// Pending CoW materialization queue
    materialization_queue: Arc<Mutex<VecDeque<CowPage>>>,
    /// Checkpoint buffer pool
    checkpoint_buffers: Arc<CheckpointBufferPool>,
}

#[derive(Debug, Clone)]
struct CowPageState {
    original_ptr: u64,
    snapshot_ptr: u64,
    page_size: u64,
    is_materialized: bool,
    access_count: u64,
}

#[derive(Debug)]
struct CowPage {
    original_ptr: u64,
    snapshot_ptr: u64,
    size: u64,
    priority: u8,
}

impl SoftCowManager {
    /// Mark GPU memory range for Soft CoW during checkpoint
    pub async fn mark_cow_region(
        &self,
        checkpoint_id: u64,
        device_ptr: u64,
        size: u64,
    ) -> Result<(), CowError> {
        let page_aligned_addr = align_down_to_page(device_ptr);
        let page_aligned_size = align_up_to_page(size);

        // Allocate snapshot buffer without immediate copy
        let snapshot_ptr = self.checkpoint_buffers.allocate(page_aligned_size).await?;

        let mut pages = self.cow_pages.lock().await;
        for offset in (0..page_aligned_size).step_by(PAGE_SIZE) {
            pages.insert(
                page_aligned_addr + offset as u64,
                CowPageState {
                    original_ptr: page_aligned_addr + offset as u64,
                    snapshot_ptr: snapshot_ptr + offset as u64,
                    page_size: PAGE_SIZE as u64,
                    is_materialized: false,
                    access_count: 0,
                },
            );
        }

        Ok(())
    }

    /// Materialize CoW page when accessed post-checkpoint
    pub async fn handle_cow_page_access(&self, device_ptr: u64) -> Result<(), CowError> {
        let page_addr = align_down_to_page(device_ptr);

        let mut pages = self.cow_pages.lock().await;
        if let Some(mut page_state) = pages.get_mut(&page_addr) {
            if !page_state.is_materialized {
                page_state.access_count += 1;

                // Async memcpy original → snapshot on first post-checkpoint access
                if page_state.access_count == 1 {
                    self.materialization_queue.lock().await.push_back(CowPage {
                        original_ptr: page_state.original_ptr,
                        snapshot_ptr: page_state.snapshot_ptr,
                        size: page_state.page_size,
                        priority: 1,
                    });
                }

                page_state.is_materialized = true;
            }
        }

        Ok(())
    }

    /// Batch materialize queued CoW pages on dedicated DMA stream
    pub async fn materialize_cow_pages(&self) -> Result<u64, CowError> {
        let mut queue = self.materialization_queue.lock().await;
        let mut bytes_materialized = 0u64;

        while let Some(page) = queue.pop_front() {
            // Copy original snapshot data to device memory via dedicated stream
            unsafe {
                cuda_driver::cuMemcpyDtoD_v2(
                    page.snapshot_ptr as *mut ::std::os::raw::c_void,
                    page.original_ptr as *mut ::std::os::raw::c_void,
                    page.size as usize,
                )
            }?;
            bytes_materialized += page.size;
        }

        Ok(bytes_materialized)
    }

    /// Query streaming progress for non-blocking checkpoint polling
    pub fn get_streaming_progress(&self, checkpoint_id: u64) -> Result<f32, CowError> {
        let pages = self.cow_pages.blocking_lock();
        let total_pages = pages.len() as u32;
        let materialized = pages.values()
            .filter(|p| p.is_materialized)
            .count() as u32;

        if total_pages == 0 {
            Ok(0.0)
        } else {
            Ok(materialized as f32 / total_pages as f32)
        }
    }
}
```

---

## 5. Checkpoint Format Specification

Binary checkpoint format supporting streaming serialization:

```
CHECKPOINT_HEADER (512 bytes)
├─ magic: [u8; 8]                  // "XKCKPT01"
├─ version: u32                     // Format version (1)
├─ checkpoint_id: u64               // Unique checkpoint identifier
├─ timestamp: u64                   // Unix nanoseconds
├─ cuda_compute_capability: u32     // Target GPU compute level
├─ total_memory_bytes: u64          // Total VRAM size checkpointed
├─ num_kernels: u32                 // Active kernel contexts
├─ num_memory_regions: u32          // Distinct memory allocations
└─ integrity_hash: [u8; 32]         // SHA256 of all following data

KERNEL_CONTEXT_BLOCK[num_kernels] (variable)
├─ kernel_id: u32
├─ compute_capability: u16
├─ registers_per_thread: u16
├─ shared_memory_size: u32
├─ kernel_binary_size: u32
├─ kernel_binary: [u8; kernel_binary_size]
└─ cta_state_size: u32

MEMORY_REGION_BLOCK[num_memory_regions] (variable)
├─ region_id: u32
├─ device_ptr: u64
├─ size: u64
├─ allocation_flags: u32
├─ compression_type: u8             // 0=none, 1=zstd, 2=lz4
├─ compressed_size: u64
├─ decompression_context: [u8; 256] // For streaming decompression
└─ data: [u8; compressed_size]

MUTATION_LOG (variable)
├─ num_mutations: u32
└─ mutations[num_mutations]: MemoryMutation serialized

CHECKPOINT_FOOTER
├─ data_checksum: [u8; 32]          // BLAKE3 of all blocks
└─ end_marker: [u8; 8]              // "XKCKPTXX"
```

```rust
pub struct CheckpointFormatEngine {
    /// Encoder for streaming checkpoint serialization
    encoder: Arc<CheckpointEncoder>,
    /// Decoder for restore deserialization
    decoder: Arc<CheckpointDecoder>,
    /// Compression codec selection
    compression: CompressionCodec,
}

#[derive(Debug)]
pub struct CheckpointHeader {
    pub magic: [u8; 8],
    pub version: u32,
    pub checkpoint_id: u64,
    pub timestamp: u64,
    pub cuda_compute_capability: u32,
    pub total_memory_bytes: u64,
    pub num_kernels: u32,
    pub num_memory_regions: u32,
    pub integrity_hash: [u8; 32],
}

impl CheckpointFormatEngine {
    /// Stream checkpoint to file/buffer with compression
    pub async fn stream_checkpoint(
        &self,
        checkpoint_id: u64,
        mutations: Vec<MemoryMutation>,
        output: &mut (impl AsyncWrite + Unpin),
    ) -> Result<u64, FormatError> {
        let mut bytes_written = 0u64;

        // Write header
        let header = self.build_header(checkpoint_id, &mutations)?;
        let header_bytes = bincode::serialize(&header)?;
        output.write_all(&header_bytes).await?;
        bytes_written += header_bytes.len() as u64;

        // Stream kernel contexts
        for kernel in self.get_kernel_contexts(checkpoint_id)? {
            let kernel_bytes = bincode::serialize(&kernel)?;
            output.write_all(&kernel_bytes).await?;
            bytes_written += kernel_bytes.len() as u64;
        }

        // Stream memory regions with adaptive compression
        for region in self.get_memory_regions(checkpoint_id)? {
            let compressed = self.compress_region(&region)?;
            output.write_all(&compressed).await?;
            bytes_written += compressed.len() as u64;
        }

        // Stream mutation log
        let mutation_bytes = self.encode_mutations(&mutations)?;
        output.write_all(&mutation_bytes).await?;
        bytes_written += mutation_bytes.len() as u64;

        Ok(bytes_written)
    }

    fn compress_region(&self, region: &MemoryRegion) -> Result<Vec<u8>, FormatError> {
        // Use zstd for VRAM (high compression ratio) vs lz4 for compute-heavy workloads
        let profile = self.analyze_memory_profile(region)?;

        match self.compression {
            CompressionCodec::Zstd => {
                let mut encoder = zstd::stream::Encoder::new(Vec::new(), 4)?;
                encoder.write_all(&region.data)?;
                Ok(encoder.finish()?)
            }
            CompressionCodec::Lz4 => {
                lz4_flex::compress_prepended(&region.data).map_err(|e| FormatError::CompressionFailed)
            }
            CompressionCodec::None => Ok(region.data.clone()),
        }
    }
}
```

---

## 6. Scheduler Integration

GPU C/R integrates with the task scheduler to avoid checkpoint during critical latency windows:

```rust
pub struct GpuCheckpointScheduler {
    /// Checkpoint requests from upper layers
    checkpoint_queue: Arc<tokio::sync::mpsc::UnboundedReceiver<CheckpointRequest>>,
    /// Current GPU workload profiler
    workload_profiler: Arc<GpuWorkloadProfiler>,
    /// C/R state machine
    cr_fsm: Arc<Mutex<CheckpointRestoreStateMachine>>,
    /// Scheduler priority levels
    priority_policy: CheckpointPriorityPolicy,
}

#[derive(Debug)]
pub struct CheckpointRequest {
    pub checkpoint_id: u64,
    pub requested_at: u64,
    pub target_latency_slo_ms: Option<u64>,
    pub callback: Option<Box<dyn Fn(CheckpointResult) + Send>>,
}

impl GpuCheckpointScheduler {
    /// Main scheduler loop—determines optimal checkpoint timing
    pub async fn run_scheduler(&mut self) {
        while let Some(req) = self.checkpoint_queue.recv().await {
            let workload = self.workload_profiler.current_workload();

            // Avoid checkpointing during:
            // - Real-time inference with strict SLOs
            // - Memory-intensive operations
            // - High frequency kernel launches

            if self.is_safe_to_checkpoint(&workload) {
                let mut fsm = self.cr_fsm.lock().await;
                match fsm.begin_concurrent_checkpoint().await {
                    Ok(ckpt_id) => {
                        if let Some(callback) = req.callback {
                            callback(CheckpointResult::Started(ckpt_id));
                        }
                    }
                    Err(e) => {
                        println!("Checkpoint failed: {:?}", e);
                    }
                }
            } else {
                // Enqueue for retry
                self.checkpoint_queue.send(req).ok();
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }
    }

    fn is_safe_to_checkpoint(&self, workload: &GpuWorkloadProfile) -> bool {
        // Checkpoint safe if:
        // - < 500 kernels/sec launch rate
        // - < 80% peak register utilization
        // - > 10ms between high-priority tasks

        workload.kernel_launch_rate < 500.0
            && workload.register_utilization_pct < 80.0
            && workload.time_since_last_critical_task_ms > 10.0
    }
}
```

---

## 7. Validation Framework

Testing strategy for concurrent C/R correctness:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_concurrent_checkpoint_no_latency_spike() {
        let fsm = CheckpointRestoreStateMachine::new();
        let start = Instant::now();

        // Launch reference workload
        let kernel_task = tokio::spawn(simulate_inference_kernel());

        // Concurrent checkpoint
        tokio::time::sleep(Duration::from_millis(50)).await;
        let ckpt_result = fsm.begin_concurrent_checkpoint().await;

        assert!(ckpt_result.is_ok());

        // Monitor kernel execution latency during checkpoint
        let kernel_latency = measure_kernel_latency(&fsm).await;
        assert!(kernel_latency < 2.0, "Kernel latency spike detected");

        kernel_task.await.unwrap();
        let total_duration = start.elapsed();
        assert!(total_duration < Duration::from_millis(150));
    }

    #[tokio::test]
    async fn test_soft_cow_reduces_checkpoint_size() {
        let cow_manager = SoftCowManager::new();
        let region_size = 1024 * 1024 * 1024u64; // 1GB

        cow_manager.mark_cow_region(1, 0x1000_0000u64, region_size).await.unwrap();

        let cow_size = cow_manager.get_checkpoint_size().await.unwrap();
        let uncompressed_size = region_size;

        // Soft CoW should reduce to near-zero until first access
        assert!(cow_size < 100_000, "CoW not reducing checkpoint size");
    }

    #[tokio::test]
    async fn test_checkpoint_restore_correctness() {
        let mut fsm = CheckpointRestoreStateMachine::new();

        // Checkpoint current state
        let ckpt_id = fsm.begin_concurrent_checkpoint().await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        fsm.finalize_checkpoint(ckpt_id).await.unwrap();

        // Mutate GPU memory
        mutate_gpu_memory();

        // Restore to checkpoint
        fsm.restore_from_checkpoint(ckpt_id).await.unwrap();

        // Verify memory state matches checkpoint
        assert!(verify_gpu_memory_state(&fsm).await);
    }
}
```

---

## 8. Performance Targets & Metrics

| Metric | Target | Measurement Method |
|--------|--------|-------------------|
| Checkpoint Latency (20GB) | <100ms | Wall-clock time checkpoint_begin → finalize |
| Kernel Execution Jitter | <2% | Compare baseline vs. concurrent latency |
| Checkpoint Memory Overhead | <5% of VRAM | Peak allocated memory during C/R |
| CoW Materialization Rate | >500MB/s | DMA throughput measurements |
| Mutation Detection Overhead | <3% | CUDA API hook cost via microbenchmarks |
| Restore Latency | <150ms | Memory stream + kernel restore time |

---

## 9. Implementation Roadmap

**Week 15 Deliverables:**
1. C/R state machine FSM (complete)
2. CUDA API interception hooks (complete)
3. Soft CoW manager implementation (complete)
4. Checkpoint format encoder/decoder (complete)
5. Scheduler integration (complete)
6. Functional testing suite (complete)
7. Performance microbenchmarks (in-progress)

**Week 16+ Integration:**
- Profiler-driven checkpoint scheduling
- Multi-GPU orchestration
- Distributed checkpoint persistence
- Recovery failure handling & rollback

---

## 10. Risk Mitigation

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|-----------|
| CUDA API hook incompatibility | Medium | High | Test on multiple CUDA versions; fallback to explicit tracking |
| Soft CoW page fault overhead | Low | Medium | Pre-fault pages during idle windows |
| Checkpoint buffer fragmentation | Low | Medium | Slab allocator with defrag strategy |
| Corruption on incomplete restore | Low | Critical | Integrity hashing + verify before finalize |

---

**Document Version:** 1.0
**Last Updated:** 2026-03-02
**Next Review:** Week 16
