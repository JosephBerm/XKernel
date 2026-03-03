# Engineer 3 — Kernel: IPC, Signals, Exceptions & Checkpointing — Week 13

## Phase: PHASE 1 — Advanced IPC & Distributed Communication

## Weekly Objective

Implement GPU checkpointing integration (PhoenixOS-inspired) enabling concurrent GPU checkpoint/restore without stopping CT execution. Add speculative GPU memory read/write detection for conflict avoidance.

## Document References
- **Primary:** Section 3.2.7 (Checkpointing Engine — GPU State)
- **Supporting:** Section 2.9 (Cognitive Checkpointing Engine), Section 6.2 (Exit Criteria)

## Deliverables
- [ ] GpuCheckpoint struct: GPU memory contents, register state, texture cache
- [ ] Concurrent GPU checkpoint/restore: offload to separate GPU compute kernel
- [ ] GPU memory snapshot: use GPU memcpy operations, no CPU blocking
- [ ] Speculative GPU memory access tracking: detect reads/writes without halting
- [ ] GPU register state capture: context switch GPU state (per-SM registers)
- [ ] GPU texture cache snapshot: volatile cache state preservation
- [ ] Background checkpoint thread: manages GPU checkpoint queue
- [ ] GPU restore operations: atomically swap GPU page tables
- [ ] Unit tests for concurrent checkpoint, speculative access, restore
- [ ] Benchmark: GPU checkpoint overhead, restore latency

## Technical Specifications

### GPU Checkpoint Structure
```
pub struct GpuCheckpoint {
    pub checkpoint_id: CheckpointId,
    pub gpu_id: GpuId,
    pub sm_registers: Vec<SmRegisterState>,        // Per-SM register state
    pub global_memory: Vec<GpuMemoryRegion>,       // Global memory contents
    pub shared_memory: Vec<SharedMemorySnapshot>,  // Per-block shared memory
    pub texture_cache: TextureCacheSnapshot,
    pub constant_cache: ConstantCacheSnapshot,
    pub timestamp: Timestamp,
}

pub struct SmRegisterState {
    pub sm_id: u32,
    pub registers: Vec<u32>,                // Per-thread registers in SM
    pub execution_state: SmExecutionState,
}

pub enum SmExecutionState {
    Active,
    Suspended,
    WaitingForMemory,
}

pub struct GpuMemoryRegion {
    pub base_address: u64,
    pub size: usize,
    pub data: Vec<u8>,
}

pub struct TextureCacheSnapshot {
    pub cache_lines: Vec<CacheLine>,
    pub validity_bits: Vec<bool>,
}

pub struct CacheLine {
    pub tag: u64,
    pub data: Vec<u8>,
    pub dirty: bool,
}
```

### Concurrent GPU Checkpoint via Kernel
```
pub struct GpuCheckpointKernel {
    pub kernel_name: &'static str,
    pub sm_per_block: u32,
    pub threads_per_block: u32,
}

impl GpuCheckpointKernel {
    pub fn launch_checkpoint(
        gpu: &GpuDevice,
        ct: &ContextThread,
        output_buffer: &mut GpuMemoryRegion,
    ) -> Result<KernelLaunchId, CheckpointError> {
        // 1. Allocate GPU memory for checkpoint output
        let checkpoint_buffer = gpu.allocate(std::mem::size_of::<GpuCheckpoint>())?;

        // 2. Launch checkpoint kernel on GPU
        //    Kernel runs on GPU without CPU involvement
        //    Captures SM registers, global memory, cache state
        let kernel = gpu.load_kernel(self)?;
        let launch_id = gpu.launch_kernel(
            kernel,
            1,  // blocks
            32, // threads
            vec![
                ct.gpu_state_ptr,
                checkpoint_buffer.ptr,
            ],
        )?;

        // 3. Return launch ID for polling
        Ok(launch_id)
    }

    pub fn poll_checkpoint_complete(
        gpu: &GpuDevice,
        launch_id: KernelLaunchId,
    ) -> Result<Option<GpuCheckpoint>, CheckpointError> {
        // 1. Check if kernel completed
        match gpu.poll_kernel(launch_id)? {
            KernelStatus::Completed => {
                // 2. Transfer checkpoint from GPU to CPU memory
                let cp = gpu.transfer_from_device()?;
                Ok(Some(cp))
            }
            KernelStatus::Running => Ok(None),
            KernelStatus::Failed(e) => Err(CheckpointError::KernelFailed(e)),
        }
    }
}

// GPU kernel pseudocode for checkpoint
/*
__global__ void gpu_checkpoint_kernel(GpuState* gpu_state, GpuCheckpoint* output) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;

    // Capture SM state (all SMs do this)
    capture_sm_registers(&output->sm_registers[idx]);

    // Cooperatively capture global memory
    for (int addr = idx; addr < global_mem_size; addr += blockDim.x * gridDim.x) {
        output->global_memory[addr] = global_mem[addr];
    }

    // Capture texture cache
    if (idx < CACHE_LINES) {
        output->texture_cache[idx] = read_texture_cache_line(idx);
    }

    __syncthreads();
}
*/
```

### Speculative GPU Memory Access Tracking
```
pub struct SpeculativeAccessTracker {
    pub reads: Bitmap,         // Which memory addresses were read (speculative)
    pub writes: Bitmap,        // Which memory addresses were written (speculative)
    pub read_timestamps: Vec<Timestamp>,
    pub write_timestamps: Vec<Timestamp>,
}

impl SpeculativeAccessTracker {
    pub fn record_read(
        &mut self,
        base_addr: u64,
        size: usize,
        timestamp: Timestamp,
    ) {
        // Mark memory region as speculatively read
        for addr in (base_addr..(base_addr + size as u64)).step_by(CACHE_LINE_SIZE) {
            let bit_index = (addr - GPU_MEM_BASE) / CACHE_LINE_SIZE;
            self.reads.set(bit_index as usize);
            self.read_timestamps[bit_index as usize] = timestamp;
        }
    }

    pub fn record_write(
        &mut self,
        base_addr: u64,
        size: usize,
        timestamp: Timestamp,
    ) {
        // Mark memory region as speculatively written
        for addr in (base_addr..(base_addr + size as u64)).step_by(CACHE_LINE_SIZE) {
            let bit_index = (addr - GPU_MEM_BASE) / CACHE_LINE_SIZE;
            self.writes.set(bit_index as usize);
            self.write_timestamps[bit_index as usize] = timestamp;
        }
    }

    pub fn detect_conflict(&self, other: &SpeculativeAccessTracker) -> Vec<u64> {
        // Find write-write or read-write conflicts
        let mut conflicts = Vec::new();

        // Write-write conflicts
        let ww_conflicts = &self.writes & &other.writes;
        for i in 0..ww_conflicts.len() {
            if ww_conflicts[i] {
                conflicts.push((i as u64) * CACHE_LINE_SIZE as u64 + GPU_MEM_BASE);
            }
        }

        // Read-write conflicts (our write after their read)
        let rw_conflicts = &other.reads & &self.writes;
        for i in 0..rw_conflicts.len() {
            if rw_conflicts[i] && self.write_timestamps[i] > other.read_timestamps[i] {
                conflicts.push((i as u64) * CACHE_LINE_SIZE as u64 + GPU_MEM_BASE);
            }
        }

        conflicts
    }
}
```

### Background Checkpoint Thread
```
pub struct GpuCheckpointManager {
    pub checkpoint_queue: VecDeque<PendingCheckpoint>,
    pub active_kernels: HashMap<KernelLaunchId, PendingCheckpoint>,
    pub completed_checkpoints: HashMap<CheckpointId, GpuCheckpoint>,
}

pub struct PendingCheckpoint {
    pub checkpoint_id: CheckpointId,
    pub ct_id: ContextThreadId,
    pub launch_id: Option<KernelLaunchId>,
    pub started_at: Timestamp,
    pub deadline: Timestamp,
}

impl GpuCheckpointManager {
    pub fn checkpoint_thread_main(&mut self) {
        loop {
            // 1. Poll for completed checkpoints
            for (launch_id, pending) in self.active_kernels.iter() {
                if let Ok(Some(gpu_cp)) = GpuCheckpointKernel::poll_checkpoint_complete(launch_id) {
                    self.completed_checkpoints.insert(pending.checkpoint_id, gpu_cp);
                    self.active_kernels.remove(launch_id);
                }
            }

            // 2. Launch new checkpoints from queue (if GPU available)
            if !self.checkpoint_queue.is_empty() && self.active_kernels.len() < MAX_CONCURRENT_KERNELS {
                if let Some(pending) = self.checkpoint_queue.pop_front() {
                    match GpuCheckpointKernel::launch_checkpoint(&pending) {
                        Ok(launch_id) => {
                            self.active_kernels.insert(launch_id, pending);
                        }
                        Err(e) => {
                            // Requeue on failure
                            self.checkpoint_queue.push_back(pending);
                        }
                    }
                }
            }

            // 3. Sleep briefly to avoid busy-waiting
            thread::sleep(Duration::from_micros(100));
        }
    }

    pub fn request_checkpoint(
        &mut self,
        checkpoint_id: CheckpointId,
        ct_id: ContextThreadId,
    ) -> Result<(), CheckpointError> {
        let deadline = Timestamp::now() + Duration::from_secs(5);
        self.checkpoint_queue.push_back(PendingCheckpoint {
            checkpoint_id,
            ct_id,
            launch_id: None,
            started_at: Timestamp::now(),
            deadline,
        });
        Ok(())
    }
}
```

### GPU Restore Operations
```
fn gpu_restore_checkpoint(
    gpu: &GpuDevice,
    checkpoint: &GpuCheckpoint,
) -> Result<(), RestoreError> {
    // 1. Atomically swap GPU page tables to point to checkpoint data
    gpu.atomic_swap_page_tables(&checkpoint.global_memory)?;

    // 2. Restore SM registers
    for (sm_id, sm_state) in checkpoint.sm_registers.iter().enumerate() {
        gpu.restore_sm_registers(sm_id, sm_state)?;
    }

    // 3. Invalidate caches (texture, constant, L2)
    gpu.invalidate_all_caches()?;

    // 4. Restore texture/constant cache state
    gpu.restore_texture_cache(&checkpoint.texture_cache)?;

    // 5. Signal GPU kernels to resume
    gpu.resume_execution()?;

    Ok(())
}
```

## Dependencies
- **Blocked by:** Week 6 (Checkpointing Engine), Week 12 (Distributed IPC)
- **Blocking:** Week 14 (Full Fault Tolerance Demo)

## Acceptance Criteria
1. Concurrent GPU checkpoint launched without blocking CT execution
2. GPU kernel successfully captures SM registers, global memory, caches
3. Speculative access tracking detects all conflicts correctly
4. Background thread manages checkpoint queue without stalling
5. GPU restore atomically switches page tables
6. No data corruption during concurrent checkpoint/restore
7. Unit tests cover: concurrent checkpoint, speculative access, restore
8. Benchmark: checkpoint latency < 100ms for 4GB GPU memory
9. No CPU busy-wait; background thread sleeps between polls
10. Checkpoints properly integrated with CPU checkpoints via CognitiveCheckpoint

## Design Principles Alignment
- **Concurrency:** GPU checkpoint kernel runs in parallel with CT execution
- **Non-blocking:** No CPU stalls during checkpoint; async polling
- **Conflict Detection:** Speculative tracking prevents data races
- **Transparency:** GPU checkpoints integrated seamlessly with CPU checkpoints
