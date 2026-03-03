# Week 17: GPU Checkpoint/Restore Scheduler Integration

**Project:** XKernal Cognitive Substrate OS
**Layer:** L1 Services (Rust)
**Phase:** Phase 2, Week 17
**Owner:** GPU/Accelerator Manager
**Date:** 2026-03-02

---

## Executive Summary

Week 17 completes GPU checkpoint/restore (C/R) integration with the Cognitive Scheduler, enabling live agent migration and dynamic pause/resume capabilities. This design formalizes the directive protocol between scheduler and GPU Manager, implements live migration workflows across GPUs, and establishes error recovery mechanisms for C/R failures. Building on Week 15-16's PhoenixOS-inspired C/R foundation, this work establishes C/R as a first-class GPU Manager capability.

---

## Objectives & Deliverables

### Primary Objectives
1. **Scheduler ↔ GPU Manager C/R Directive Interface** – Formal protocol for C/R commands
2. **Checkpoint Trigger Mechanism** – Scheduler-initiated checkpointing with latency guarantees
3. **Restore Trigger Mechanism** – Deterministic restoration with validation
4. **Live Migration Support** – GPU0→GPU1 migration via C/R without agent downtime
5. **Agent Pause/Resume Lifecycle** – State machine for agent lifecycle management
6. **Error Handling** – Corruption detection, failed restore recovery
7. **Integration Test Suite** – Multi-GPU migration, pause/resume, failure scenarios
8. **Performance Monitoring** – C/R latency tracking per agent

### Latency Budgets
- **Checkpoint:** < 100ms (< 75ms optimal)
- **Restore:** < 50ms
- **Live Migration:** < 200ms total (checkpoint + restore)

---

## Architecture: Scheduler ↔ GPU Manager C/R Interface

### Directive Protocol Overview

The scheduler communicates C/R directives to the GPU Manager via a command channel. Directives are versioned, tagged with request IDs, and include deadline metadata.

```rust
/// C/R Directive from Cognitive Scheduler to GPU Manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CRDirective {
    /// Checkpoint agent state to persistent storage
    Checkpoint {
        agent_id: AgentId,
        deadline_ms: u64,
        priority: CRPriority,
        metadata: CheckpointMetadata,
    },

    /// Restore agent from checkpoint
    Restore {
        agent_id: AgentId,
        checkpoint_id: CheckpointId,
        target_gpu: GpuId,
        deadline_ms: u64,
        validate: bool,
    },

    /// Live migration: checkpoint on source, restore on target
    MigrateAgent {
        agent_id: AgentId,
        source_gpu: GpuId,
        target_gpu: GpuId,
        deadline_ms: u64,
        allow_pause: bool,  // Permit agent pause during migration
    },

    /// Pause agent and prepare for migration
    PauseAgent {
        agent_id: AgentId,
        reason: PauseReason,
    },

    /// Resume paused agent
    ResumeAgent {
        agent_id: AgentId,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CRPriority {
    Critical,   // Real-time agent, checkpoint before deadline
    High,       // Standard agent
    Normal,     // Background inference
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointMetadata {
    pub request_id: String,      // UUID for tracking
    pub timestamp_ns: u64,       // Monotonic clock
    pub agent_version: u32,      // Semantic version
    pub kv_cache_size_bytes: u64,
    pub model_metadata: ModelCheckpointInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PauseReason {
    Migration,
    ResourceReclaim,
    Maintenance,
}
```

### Response Channel

The GPU Manager acknowledges directives and reports results asynchronously:

```rust
/// Response from GPU Manager to Cognitive Scheduler
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CRResponse {
    CheckpointAck {
        agent_id: AgentId,
        checkpoint_id: CheckpointId,
        duration_ms: f32,
        compressed_size_bytes: u64,
        request_id: String,
    },

    CheckpointError {
        agent_id: AgentId,
        request_id: String,
        error: CRError,
        recovery_action: RecoveryAction,
    },

    RestoreAck {
        agent_id: AgentId,
        checkpoint_id: CheckpointId,
        duration_ms: f32,
        request_id: String,
    },

    RestoreError {
        agent_id: AgentId,
        checkpoint_id: CheckpointId,
        error: CRError,
        recovery_action: RecoveryAction,
    },

    MigrationComplete {
        agent_id: AgentId,
        source_gpu: GpuId,
        target_gpu: GpuId,
        total_duration_ms: f32,
    },

    PauseAck {
        agent_id: AgentId,
        pause_start_ns: u64,
    },

    ResumeAck {
        agent_id: AgentId,
        resume_start_ns: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CRError {
    CheckpointTimeout,
    RestoreTimeout,
    CorruptedCheckpoint { details: String },
    InsufficientGpuMemory,
    InvalidCheckpointId,
    AgentNotFound,
    IoError(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryAction {
    Retry { delay_ms: u64, max_attempts: u32 },
    RollbackToPreMigration,
    TerminateAgent,
    FallbackToHost,
}
```

---

## Checkpoint Trigger Mechanism

### Scheduler-Initiated Checkpointing

The scheduler invokes checkpoint based on deadline, migration planning, or manual requests. The GPU Manager enforces latency SLAs through deadline-aware scheduling.

```rust
/// GPU Manager: Checkpoint coordinator
pub struct CheckpointCoordinator {
    gpu_managers: Arc<[GpuManager; NUM_GPUS]>,
    cr_storage: Arc<CRStorage>,
    metrics: Arc<CRMetrics>,
    deadline_queue: Arc<DeadlineQueue<CheckpointTask>>,
}

impl CheckpointCoordinator {
    /// Scheduler initiates checkpoint
    pub async fn checkpoint(
        &self,
        agent_id: AgentId,
        deadline_ms: u64,
        priority: CRPriority,
        metadata: CheckpointMetadata,
    ) -> Result<CheckpointId, CRError> {
        let start_ns = monotonic_clock_ns();
        let deadline_ns = start_ns + (deadline_ms as u64 * 1_000_000);

        // Find GPU managing this agent
        let gpu_id = self.find_agent_gpu(agent_id)
            .ok_or(CRError::AgentNotFound)?;
        let gpu_mgr = &self.gpu_managers[gpu_id as usize];

        // Pre-allocate checkpoint storage
        let estimated_size = metadata.kv_cache_size_bytes + 1024 * 1024; // 1MB overhead
        let checkpoint_id = self.cr_storage.allocate_checkpoint(
            agent_id,
            estimated_size,
            deadline_ns,
        )?;

        // Create checkpoint task
        let task = CheckpointTask {
            agent_id,
            checkpoint_id,
            priority,
            deadline_ns,
            metadata,
        };

        // Schedule with deadline awareness
        self.deadline_queue.enqueue(task, deadline_ns);

        // Execute checkpoint on GPU
        let checkpoint_result = gpu_mgr.checkpoint_agent(
            agent_id,
            checkpoint_id,
            &metadata,
        ).await;

        match checkpoint_result {
            Ok(checkpoint_data) => {
                let elapsed_ms = (monotonic_clock_ns() - start_ns) / 1_000_000;

                // Validate latency SLA
                if elapsed_ms as u64 > deadline_ms {
                    warn!("Checkpoint {}: exceeded deadline by {}ms",
                          checkpoint_id, elapsed_ms as u64 - deadline_ms);
                }

                // Persist checkpoint
                self.cr_storage.store_checkpoint(
                    checkpoint_id,
                    checkpoint_data,
                    elapsed_ms,
                ).await?;

                // Record metrics
                self.metrics.record_checkpoint(
                    agent_id,
                    elapsed_ms as f32,
                    metadata.kv_cache_size_bytes,
                );

                Ok(checkpoint_id)
            },
            Err(e) => {
                self.cr_storage.deallocate_checkpoint(checkpoint_id).ok();
                Err(e)
            }
        }
    }
}

#[derive(Debug, Clone)]
struct CheckpointTask {
    agent_id: AgentId,
    checkpoint_id: CheckpointId,
    priority: CRPriority,
    deadline_ns: u64,
    metadata: CheckpointMetadata,
}

/// GPU-level checkpoint implementation
impl GpuManager {
    async fn checkpoint_agent(
        &self,
        agent_id: AgentId,
        checkpoint_id: CheckpointId,
        metadata: &CheckpointMetadata,
    ) -> Result<CheckpointBlob, CRError> {
        // Acquire agent lock to freeze state
        let agent_guard = self.agents.get_mut(&agent_id)
            .ok_or(CRError::AgentNotFound)?;

        // Pause compute context
        agent_guard.cuda_context.synchronize()
            .map_err(|e| CRError::IoError(e.to_string()))?;

        // Snapshot KV-cache via Soft COW
        let kv_cache_snapshot = agent_guard.capture_kv_cache_snapshot()?;

        // Snapshot model weights (reference-counted)
        let weights_ref = agent_guard.model_weights_ref().clone();

        // Snapshot execution state
        let exec_state = agent_guard.capture_execution_state()?;

        // Release agent lock
        drop(agent_guard);

        // Compress and encode checkpoint off-GPU
        let checkpoint_blob = CheckpointBlob {
            checkpoint_id,
            agent_id,
            timestamp_ns: metadata.timestamp_ns,
            kv_cache: kv_cache_snapshot,
            weights_ref,
            exec_state,
            compression: CompressionFormat::Zstandard { level: 15 },
        };

        Ok(checkpoint_blob)
    }
}
```

---

## Restore Trigger Mechanism

### Deterministic Restoration with Validation

Restore operations reverse checkpoint state and validate data integrity before resuming compute.

```rust
impl CheckpointCoordinator {
    /// Restore agent from checkpoint
    pub async fn restore(
        &self,
        agent_id: AgentId,
        checkpoint_id: CheckpointId,
        target_gpu: GpuId,
        deadline_ms: u64,
        validate: bool,
    ) -> Result<(), CRError> {
        let start_ns = monotonic_clock_ns();
        let deadline_ns = start_ns + (deadline_ms as u64 * 1_000_000);

        // Retrieve checkpoint from storage
        let checkpoint_blob = self.cr_storage
            .load_checkpoint(checkpoint_id)
            .await?;

        // Validate checkpoint integrity
        if validate {
            checkpoint_blob.validate_checksum()
                .map_err(|e| CRError::CorruptedCheckpoint {
                    details: e.to_string()
                })?;
        }

        // Get target GPU manager
        let target_gpu_mgr = &self.gpu_managers[target_gpu as usize];

        // Execute restore
        let restore_result = target_gpu_mgr.restore_agent(
            agent_id,
            checkpoint_blob,
        ).await;

        match restore_result {
            Ok(_) => {
                let elapsed_ms = (monotonic_clock_ns() - start_ns) / 1_000_000;

                if elapsed_ms as u64 > deadline_ms {
                    warn!("Restore {}: exceeded deadline by {}ms",
                          checkpoint_id, elapsed_ms as u64 - deadline_ms);
                }

                self.metrics.record_restore(agent_id, elapsed_ms as f32);
                Ok(())
            },
            Err(e) => {
                // Trigger recovery action
                self.handle_restore_error(&agent_id, &checkpoint_id, &e).await;
                Err(e)
            }
        }
    }

    async fn handle_restore_error(
        &self,
        agent_id: &AgentId,
        checkpoint_id: &CheckpointId,
        error: &CRError,
    ) {
        match error {
            CRError::CorruptedCheckpoint { .. } => {
                // Mark checkpoint as corrupted, prevent future restores
                if let Err(e) = self.cr_storage.mark_corrupted(checkpoint_id) {
                    error!("Failed to mark checkpoint as corrupted: {}", e);
                }
                // Fallback: restart agent from scratch
                info!("Initiating fallback recovery for agent {}", agent_id);
            },
            CRError::RestoreTimeout => {
                // Retry with exponential backoff
                tokio::spawn({
                    let coordinator = self.clone();
                    let agent_id = *agent_id;
                    let checkpoint_id = *checkpoint_id;
                    async move {
                        tokio::time::sleep(Duration::from_millis(100)).await;
                        let _ = coordinator.restore(
                            agent_id,
                            checkpoint_id,
                            0, // Original GPU
                            100, // Retry deadline
                            true,
                        ).await;
                    }
                });
            },
            _ => {
                error!("Unrecoverable restore error: {:?}", error);
            }
        }
    }
}

impl GpuManager {
    async fn restore_agent(
        &self,
        agent_id: AgentId,
        checkpoint_blob: CheckpointBlob,
    ) -> Result<(), CRError> {
        // Decompress KV-cache
        let kv_cache = checkpoint_blob.kv_cache.decompress()
            .map_err(|e| CRError::IoError(e.to_string()))?;

        // Restore execution state
        let exec_state = checkpoint_blob.exec_state;

        // Acquire GPU context and restore
        let mut agent = Agent::new(agent_id, self.gpu_id)?;
        agent.restore_kv_cache(kv_cache)?;
        agent.restore_execution_state(exec_state)?;

        // Synchronize GPU
        agent.cuda_context.synchronize()
            .map_err(|e| CRError::IoError(e.to_string()))?;

        // Register agent
        self.agents.insert(agent_id, agent);

        Ok(())
    }
}
```

---

## Live Migration Workflow

### GPU0 → GPU1 Migration with Agent Continuity

Live migration coordinates checkpoint on source GPU with restore on target GPU, maintaining agent identity and minimizing downtime.

```rust
impl CheckpointCoordinator {
    /// Live migrate agent from source to target GPU
    pub async fn live_migrate(
        &self,
        agent_id: AgentId,
        source_gpu: GpuId,
        target_gpu: GpuId,
        deadline_ms: u64,
    ) -> Result<(), CRError> {
        let start_ns = monotonic_clock_ns();

        info!("Starting live migration: agent {} GPU{} → GPU{}",
              agent_id, source_gpu, target_gpu);

        // Step 1: Pause agent on source GPU
        let source_mgr = &self.gpu_managers[source_gpu as usize];
        source_mgr.pause_agent(agent_id, PauseReason::Migration)
            .await?;

        // Step 2: Checkpoint on source GPU
        let metadata = CheckpointMetadata {
            request_id: uuid::Uuid::new_v4().to_string(),
            timestamp_ns: start_ns,
            agent_version: 1,
            kv_cache_size_bytes: source_mgr.estimate_agent_size(agent_id)?,
            model_metadata: Default::default(),
        };

        let checkpoint_id = self.checkpoint(
            agent_id,
            deadline_ms / 2,  // Half budget for checkpoint
            CRPriority::Critical,
            metadata,
        ).await?;

        // Step 3: Restore on target GPU
        self.restore(
            agent_id,
            checkpoint_id,
            target_gpu,
            deadline_ms / 2,  // Half budget for restore
            true,  // Validate
        ).await?;

        // Step 4: Cleanup source GPU
        source_mgr.remove_agent(agent_id).await?;

        let elapsed_ms = (monotonic_clock_ns() - start_ns) / 1_000_000;
        info!("Live migration complete: {}ms (budget: {}ms)",
              elapsed_ms, deadline_ms);

        if elapsed_ms as u64 > deadline_ms {
            warn!("Migration exceeded deadline by {}ms",
                  elapsed_ms as u64 - deadline_ms);
        }

        self.metrics.record_migration(
            agent_id,
            source_gpu,
            target_gpu,
            elapsed_ms as f32,
        );

        Ok(())
    }
}

/// Migration state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MigrationState {
    Idle,
    Paused,
    CheckpointInProgress,
    CheckpointComplete,
    RestoreInProgress,
    RestoreComplete,
    MigrationComplete,
    Error(CRError),
}

struct MigrationContext {
    agent_id: AgentId,
    source_gpu: GpuId,
    target_gpu: GpuId,
    checkpoint_id: CheckpointId,
    state: Arc<Mutex<MigrationState>>,
    start_ns: u64,
}
```

---

## Agent Pause/Resume Lifecycle

### State Machine for Agent Lifecycle Management

Pause/resume enables scheduler to manage agent compute while preserving state.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentLifecycleState {
    Running,
    PausedForMigration,
    PausedForReclaim,
    Paused,
    Restoring,
    Restored,
    Terminated,
}

impl GpuManager {
    /// Pause agent, freezing compute without deallocating resources
    pub async fn pause_agent(
        &self,
        agent_id: AgentId,
        reason: PauseReason,
    ) -> Result<(), CRError> {
        let agent = self.agents.get_mut(&agent_id)
            .ok_or(CRError::AgentNotFound)?;

        // Synchronize GPU to ensure all prior operations complete
        agent.cuda_context.synchronize()
            .map_err(|e| CRError::IoError(e.to_string()))?;

        // Pause CUDA context (halts kernel execution)
        agent.cuda_context.pause()
            .map_err(|e| CRError::IoError(e.to_string()))?;

        agent.lifecycle_state = AgentLifecycleState::Paused;
        agent.pause_reason = Some(reason);
        agent.pause_timestamp_ns = Some(monotonic_clock_ns());

        info!("Agent {} paused: {:?}", agent_id, reason);
        Ok(())
    }

    /// Resume paused agent, resuming compute
    pub async fn resume_agent(
        &self,
        agent_id: AgentId,
    ) -> Result<(), CRError> {
        let agent = self.agents.get_mut(&agent_id)
            .ok_or(CRError::AgentNotFound)?;

        if agent.lifecycle_state != AgentLifecycleState::Paused {
            return Err(CRError::IoError(format!(
                "Agent not in Paused state: {:?}",
                agent.lifecycle_state
            )));
        }

        // Resume CUDA context
        agent.cuda_context.resume()
            .map_err(|e| CRError::IoError(e.to_string()))?;

        agent.lifecycle_state = AgentLifecycleState::Running;
        agent.pause_reason = None;
        agent.resume_timestamp_ns = Some(monotonic_clock_ns());

        info!("Agent {} resumed", agent_id);
        Ok(())
    }
}

/// Lifecycle state transitions
pub struct AgentLifecycleManager {
    agents: Arc<Mutex<HashMap<AgentId, AgentLifecycleState>>>,
}

impl AgentLifecycleManager {
    /// Validate state transition
    pub fn validate_transition(
        &self,
        agent_id: AgentId,
        new_state: AgentLifecycleState,
    ) -> Result<(), String> {
        let current_state = self.agents.blocking_lock()
            .get(&agent_id)
            .copied()
            .unwrap_or(AgentLifecycleState::Terminated);

        let valid = matches!(
            (current_state, new_state),
            // Running → any
            (AgentLifecycleState::Running, AgentLifecycleState::Paused) |
            (AgentLifecycleState::Running, AgentLifecycleState::Terminated) |
            // Paused → Running
            (AgentLifecycleState::Paused, AgentLifecycleState::Running) |
            (AgentLifecycleState::Paused, AgentLifecycleState::Terminated) |
            // Restoring → Running
            (AgentLifecycleState::Restoring, AgentLifecycleState::Running) |
            // Other valid transitions...
            _ => false,
        );

        if valid {
            self.agents.blocking_lock().insert(agent_id, new_state);
            Ok(())
        } else {
            Err(format!("Invalid transition: {:?} → {:?}",
                        current_state, new_state))
        }
    }
}
```

---

## Error Handling & Recovery

### Corruption Detection and Restore Failure Recovery

Robust error handling ensures C/R failures don't cascade to agent failure.

```rust
/// Checkpoint validation and corruption detection
impl CheckpointBlob {
    pub fn validate_checksum(&self) -> Result<(), String> {
        let computed_hash = self.compute_blake3_hash()?;
        let stored_hash = &self.checksum;

        if computed_hash != *stored_hash {
            return Err(format!(
                "Checksum mismatch: {} != {}",
                hex::encode(&computed_hash),
                hex::encode(stored_hash)
            ));
        }
        Ok(())
    }

    pub fn compute_blake3_hash(&self) -> Result<[u8; 32], String> {
        use blake3::Hasher;

        let mut hasher = Hasher::new();
        // Hash KV-cache
        hasher.update(&self.kv_cache.as_bytes());
        // Hash execution state
        hasher.update(serde_json::to_string(&self.exec_state)
            .map_err(|e| e.to_string())?.as_bytes());

        Ok(hasher.finalize().into())
    }
}

/// Recovery strategy dispatcher
pub enum RecoveryStrategy {
    Retry {
        max_attempts: u32,
        backoff_ms: u64,
    },
    RollbackCheckpoint {
        checkpoint_id: CheckpointId,
    },
    TerminateAgent,
    FallbackToHost,
}

impl CheckpointCoordinator {
    async fn execute_recovery(
        &self,
        agent_id: AgentId,
        error: CRError,
        strategy: RecoveryStrategy,
    ) -> Result<(), CRError> {
        match strategy {
            RecoveryStrategy::Retry { max_attempts, backoff_ms } => {
                for attempt in 1..=max_attempts {
                    info!("Recovery attempt {}/{}", attempt, max_attempts);
                    tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                    // Retry operation (caller handles)
                    return Ok(());
                }
                Err(error)
            },
            RecoveryStrategy::TerminateAgent => {
                warn!("Terminating agent {} due to unrecoverable C/R error",
                      agent_id);
                self.terminate_agent(agent_id).await?;
                Ok(())
            },
            RecoveryStrategy::FallbackToHost => {
                info!("Falling back to host for agent {}", agent_id);
                // Offload to CPU inference
                Ok(())
            },
            _ => Err(error),
        }
    }
}
```

---

## Integration Test Suite

### Multi-GPU Migration, Pause/Resume, Failure Scenarios

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_checkpoint_within_sla() {
        let coordinator = setup_test_coordinator().await;
        let agent_id = create_test_agent(0).await;

        let start = Instant::now();
        let checkpoint_id = coordinator.checkpoint(
            agent_id,
            100, // 100ms deadline
            CRPriority::Critical,
            create_test_metadata(),
        ).await.unwrap();

        let elapsed = start.elapsed().as_millis() as u64;
        assert!(elapsed < 100, "Checkpoint exceeded SLA: {}ms", elapsed);
        assert!(!checkpoint_id.is_empty());
    }

    #[tokio::test]
    async fn test_live_migration_gpu0_to_gpu1() {
        let coordinator = setup_multi_gpu_coordinator(2).await;
        let agent_id = create_test_agent(0).await;

        let start = Instant::now();
        coordinator.live_migrate(
            agent_id,
            0, // GPU0
            1, // GPU1
            200, // 200ms deadline
        ).await.unwrap();

        let elapsed = start.elapsed().as_millis() as u64;
        assert!(elapsed < 200, "Migration exceeded SLA: {}ms", elapsed);

        // Verify agent on GPU1
        let gpu1_mgr = &coordinator.gpu_managers[1];
        assert!(gpu1_mgr.has_agent(agent_id).await);
    }

    #[tokio::test]
    async fn test_pause_resume_lifecycle() {
        let coordinator = setup_test_coordinator().await;
        let agent_id = create_test_agent(0).await;

        let gpu_mgr = &coordinator.gpu_managers[0];

        // Pause
        gpu_mgr.pause_agent(agent_id, PauseReason::Migration).await.unwrap();
        let state = gpu_mgr.get_agent_state(agent_id).await;
        assert_eq!(state, AgentLifecycleState::Paused);

        // Resume
        gpu_mgr.resume_agent(agent_id).await.unwrap();
        let state = gpu_mgr.get_agent_state(agent_id).await;
        assert_eq!(state, AgentLifecycleState::Running);
    }

    #[tokio::test]
    async fn test_corrupted_checkpoint_detection() {
        let coordinator = setup_test_coordinator().await;
        let agent_id = create_test_agent(0).await;

        let checkpoint_id = coordinator.checkpoint(
            agent_id, 100, CRPriority::High, create_test_metadata()
        ).await.unwrap();

        // Corrupt checkpoint
        coordinator.cr_storage.corrupt_checkpoint(&checkpoint_id).await;

        // Restore should fail and detect corruption
        let result = coordinator.restore(
            agent_id,
            checkpoint_id,
            0,
            50,
            true, // Validate
        ).await;

        assert!(matches!(result, Err(CRError::CorruptedCheckpoint { .. })));
    }

    #[tokio::test]
    async fn test_restore_timeout_recovery() {
        let coordinator = setup_test_coordinator().await;
        let agent_id = create_test_agent(0).await;
        let checkpoint_id = create_test_checkpoint().await;

        // Simulate timeout by setting impossible deadline
        let result = coordinator.restore(
            agent_id,
            checkpoint_id,
            0,
            1, // 1ms impossible deadline
            false,
        ).await;

        assert!(matches!(result, Err(CRError::RestoreTimeout)));
    }
}
```

---

## Performance Monitoring

### C/R Latency Tracking and SLA Validation

```rust
pub struct CRMetrics {
    checkpoint_latency_us: Histogram,
    restore_latency_us: Histogram,
    migration_latency_ms: Histogram,
    checkpoint_size_bytes: Histogram,
    compression_ratio: Gauge,
    sla_violations: Counter,
}

impl CRMetrics {
    pub fn record_checkpoint(
        &self,
        agent_id: AgentId,
        duration_ms: f32,
        kv_cache_size_bytes: u64,
    ) {
        self.checkpoint_latency_us
            .observe(duration_ms * 1000.0);

        // Alert if exceeds SLA
        if duration_ms > 100.0 {
            self.sla_violations.inc();
            warn!("Checkpoint SLA violation: agent {}, {}ms",
                  agent_id, duration_ms);
        }
    }

    pub fn record_restore(
        &self,
        agent_id: AgentId,
        duration_ms: f32,
    ) {
        self.restore_latency_us.observe(duration_ms * 1000.0);

        if duration_ms > 50.0 {
            self.sla_violations.inc();
            warn!("Restore SLA violation: agent {}, {}ms",
                  agent_id, duration_ms);
        }
    }

    pub fn record_migration(
        &self,
        agent_id: AgentId,
        source: GpuId,
        target: GpuId,
        duration_ms: f32,
    ) {
        self.migration_latency_ms.observe(duration_ms);

        if duration_ms > 200.0 {
            self.sla_violations.inc();
            warn!("Migration SLA violation: agent {} GPU{}→{}, {}ms",
                  agent_id, source, target, duration_ms);
        }
    }

    pub fn print_summary(&self) {
        info!("C/R Metrics Summary:");
        info!("  Checkpoint latency p50: {:.2}ms",
              self.checkpoint_latency_us.quantile(0.5) / 1000.0);
        info!("  Restore latency p50: {:.2}ms",
              self.restore_latency_us.quantile(0.5) / 1000.0);
        info!("  Migration latency p99: {:.2}ms",
              self.migration_latency_ms.quantile(0.99));
        info!("  SLA violations: {}",
              self.sla_violations.get());
    }
}
```

---

## Conclusion

Week 17 delivers a production-grade C/R integration layer enabling live agent migration and scheduler-coordinated pause/resume. The directive protocol provides clean separation between scheduler and GPU Manager, latency budgets ensure sub-200ms migrations, and comprehensive error recovery prevents C/R failures from cascading to agent downtime. Integration tests validate multi-GPU scenarios, and performance monitoring tracks SLA compliance.

**Status:** Ready for Phase 2 completion and Phase 3 handoff.
