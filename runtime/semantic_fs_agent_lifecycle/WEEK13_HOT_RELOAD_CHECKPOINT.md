# Week 13 — Agent Hot-Reload: Checkpoint, State Preservation & Zero-Downtime Updates

**XKernal Cognitive Substrate OS** | Engineer 8 Technical Design Document

---

## Executive Summary

Week 13 implements hot-reload capability for autonomous agents within the XKernal semantic filesystem framework, enabling zero-downtime agent updates while preserving critical state. This mechanism captures agent checkpoints before updates, serializes semantic memory and conversation context, and provides atomic resumption with rollback guarantees. Integration with `cs-agentctl` CLI allows operators to safely update agent configuration and code without disrupting crew operations or losing inference context.

**Key Outcomes:**
- Checkpoint-based state capture and serialization
- Zero-downtime updates within agent crews
- Atomic rollback on update failure
- Persistent checkpoint storage with versioning
- 10+ comprehensive hot-reload test scenarios

---

## Problem Statement

Current agent lifecycle management lacks hot-reload capability:

1. **Downtime Impact**: Agent updates require full shutdown/restart, disrupting ongoing crew operations and losing inference context
2. **State Loss Risk**: Configuration and code updates may lose semantic memory, conversation history, and tool execution progress
3. **No Rollback**: Failed updates have no atomic recovery path; manual recovery is error-prone
4. **Inconsistent State**: Partial updates across multi-agent crews create divergent system states
5. **Operational Complexity**: Operators cannot safely push updates to production agent fleets without manual coordination

**Success Criteria:**
- Zero-downtime agent updates while crew operations continue
- 100% semantic memory and conversation state preservation
- Atomic rollback within 500ms on update failure
- Checkpoint storage with versioning and cleanup policies

---

## Architecture

### Core Components

#### 1. **AgentCheckpoint** — State Capture
```rust
/// Captures complete agent state for hot-reload
#[derive(Serialize, Deserialize, Clone)]
pub struct AgentCheckpoint {
    pub agent_id: String,
    pub checkpoint_id: String,
    pub timestamp: SystemTime,
    pub version: u64,

    // Core state
    pub semantic_memory: SemanticMemorySnapshot,
    pub conversation_history: Vec<ConversationTurn>,
    pub tool_state: ToolExecutionState,
    pub config_snapshot: AgentConfig,

    // Progress tracking
    pub active_tasks: Vec<TaskMarker>,
    pub pending_operations: Vec<Operation>,
    pub inference_context: InferenceContext,

    // Metadata
    pub checksum: String,
    pub retention_ttl: Duration,
}

impl AgentCheckpoint {
    /// Create checkpoint from running agent
    pub async fn capture(agent: &RunningAgent) -> Result<Self> {
        let semantic_memory = agent.memory_store.snapshot().await?;
        let conversation = agent.conversation_manager.export_history().await?;
        let tool_state = agent.tool_executor.capture_state().await?;
        let config = agent.current_config.clone();

        let checkpoint = AgentCheckpoint {
            agent_id: agent.id.clone(),
            checkpoint_id: format!("ckpt-{}-{}", agent.id, Uuid::new_v4()),
            timestamp: SystemTime::now(),
            version: agent.state_version,
            semantic_memory,
            conversation_history: conversation,
            tool_state,
            config_snapshot: config,
            active_tasks: agent.task_queue.current_markers(),
            pending_operations: agent.pending_ops.clone(),
            inference_context: agent.inference_ctx.snapshot(),
            checksum: String::new(), // Computed below
            retention_ttl: Duration::from_secs(86400 * 7), // 7 days
        };

        // Compute deterministic checksum
        let checkpoint_with_checksum = checkpoint.with_computed_checksum()?;
        Ok(checkpoint_with_checksum)
    }

    /// Validate checkpoint integrity
    pub fn validate(&self) -> Result<()> {
        let computed = self.compute_checksum()?;
        if computed != self.checksum {
            return Err(CheckpointError::IntegrityViolation.into());
        }
        Ok(())
    }
}
```

#### 2. **StateSerializer** — Persistence Layer
```rust
/// Serializes and deserializes agent state for checkpoint storage
pub struct StateSerializer {
    storage: Arc<dyn CheckpointStore>,
    compression: CompressionCodec,
}

impl StateSerializer {
    /// Persist checkpoint to storage
    pub async fn serialize_checkpoint(&self, ckpt: &AgentCheckpoint) -> Result<CheckpointRef> {
        // Validate before serialization
        ckpt.validate()?;

        let serialized = serde_json::to_vec(ckpt)?;
        let compressed = self.compression.encode(&serialized)?;

        // Write with metadata
        let ref_id = self.storage.store(
            &ckpt.agent_id,
            ckpt.version,
            compressed,
            ckpt.checksum.clone(),
        ).await?;

        Ok(CheckpointRef {
            id: ref_id,
            agent_id: ckpt.agent_id.clone(),
            version: ckpt.version,
            created_at: ckpt.timestamp,
        })
    }

    /// Deserialize checkpoint from storage
    pub async fn deserialize_checkpoint(&self, ref_id: &str) -> Result<AgentCheckpoint> {
        let compressed = self.storage.retrieve(ref_id).await?;
        let serialized = self.compression.decode(&compressed)?;
        let ckpt: AgentCheckpoint = serde_json::from_slice(&serialized)?;

        ckpt.validate()?;
        Ok(ckpt)
    }

    /// List available checkpoints for agent
    pub async fn list_checkpoints(&self, agent_id: &str, limit: usize) -> Result<Vec<CheckpointRef>> {
        self.storage.list(agent_id, limit).await
    }
}
```

#### 3. **HotReloadOrchestrator** — Update Coordination
```rust
/// Orchestrates safe hot-reload workflow within crews
pub struct HotReloadOrchestrator {
    checkpoint_store: Arc<StateSerializer>,
    agent_registry: Arc<AgentRegistry>,
    rollback_mgr: Arc<RollbackManager>,
}

impl HotReloadOrchestrator {
    /// Execute hot-reload: checkpoint → update → resume
    pub async fn hot_reload(
        &self,
        agent_id: &str,
        update_spec: AgentUpdateSpec,
    ) -> Result<HotReloadResult> {
        // Phase 1: Checkpoint
        let agent = self.agent_registry.get(agent_id)?;
        let checkpoint = AgentCheckpoint::capture(&agent).await?;
        let ckpt_ref = self.checkpoint_store.serialize_checkpoint(&checkpoint).await?;

        // Phase 2: Pause agent (synchronous)
        agent.pause_execution().await?;

        // Phase 3: Apply update
        match self.apply_update(&agent, update_spec).await {
            Ok(new_agent) => {
                // Phase 4: Resume from checkpoint
                new_agent.resume_from_checkpoint(&checkpoint).await?;

                // Phase 5: Verify state
                self.verify_state_consistency(&new_agent).await?;

                Ok(HotReloadResult {
                    agent_id: agent_id.to_string(),
                    checkpoint_ref: ckpt_ref,
                    status: HotReloadStatus::Success,
                    duration: /* elapsed */,
                })
            }
            Err(e) => {
                // Rollback on failure
                self.rollback_mgr.restore(&agent, &checkpoint).await?;

                Err(HotReloadError::UpdateFailed(e).into())
            }
        }
    }

    /// Apply configuration or code update
    async fn apply_update(
        &self,
        agent: &RunningAgent,
        spec: AgentUpdateSpec,
    ) -> Result<RunningAgent> {
        match spec {
            AgentUpdateSpec::ConfigUpdate(new_config) => {
                agent.update_config(new_config).await
            }
            AgentUpdateSpec::CodeUpdate { module, bytecode } => {
                agent.load_module(&module, bytecode).await
            }
            AgentUpdateSpec::ToolUpdate { tool_name, definition } => {
                agent.register_tool(&tool_name, definition).await
            }
        }
    }

    /// Verify state consistency post-reload
    async fn verify_state_consistency(&self, agent: &RunningAgent) -> Result<()> {
        // Validate semantic memory integrity
        agent.memory_store.validate().await?;

        // Verify conversation history continuity
        agent.conversation_manager.verify_continuity().await?;

        // Check tool state consistency
        agent.tool_executor.validate_state().await?;

        Ok(())
    }
}
```

#### 4. **RollbackManager** — Failure Recovery
```rust
/// Manages atomic rollback on update failure
pub struct RollbackManager {
    storage: Arc<StateSerializer>,
    agent_registry: Arc<AgentRegistry>,
}

impl RollbackManager {
    /// Restore agent to checkpoint state atomically
    pub async fn restore(
        &self,
        agent: &RunningAgent,
        checkpoint: &AgentCheckpoint,
    ) -> Result<()> {
        // Begin atomic transaction
        let txn = agent.begin_txn().await?;

        // Restore in order: semantic memory → conversation → tools → config
        agent.memory_store.restore(&checkpoint.semantic_memory, &txn).await?;
        agent.conversation_manager.restore(&checkpoint.conversation_history, &txn).await?;
        agent.tool_executor.restore(&checkpoint.tool_state, &txn).await?;
        agent.apply_config(&checkpoint.config_snapshot, &txn).await?;

        // Restore task and inference context
        agent.task_queue.restore(&checkpoint.active_tasks, &txn).await?;
        agent.inference_ctx.restore(&checkpoint.inference_context, &txn).await?;

        // Commit atomically
        txn.commit().await?;

        // Log rollback event
        agent.audit_log.record_rollback(&checkpoint.checkpoint_id).await?;

        Ok(())
    }

    /// Clean up expired checkpoints
    pub async fn cleanup_expired(&self, agent_id: &str) -> Result<usize> {
        let checkpoints = self.storage.list_checkpoints(agent_id, 1000).await?;
        let mut deleted = 0;

        for ckpt_ref in checkpoints {
            if ckpt_ref.created_at.elapsed()? > Duration::from_secs(86400 * 7) {
                self.storage.delete(&ckpt_ref.id).await?;
                deleted += 1;
            }
        }

        Ok(deleted)
    }
}
```

#### 5. **CheckpointStore** — Storage Interface
```rust
/// Abstract checkpoint persistence layer
#[async_trait]
pub trait CheckpointStore: Send + Sync {
    /// Store checkpoint data
    async fn store(
        &self,
        agent_id: &str,
        version: u64,
        data: Vec<u8>,
        checksum: String,
    ) -> Result<String>; // Returns checkpoint ID

    /// Retrieve checkpoint data
    async fn retrieve(&self, checkpoint_id: &str) -> Result<Vec<u8>>;

    /// List checkpoints for agent
    async fn list(&self, agent_id: &str, limit: usize) -> Result<Vec<CheckpointRef>>;

    /// Delete checkpoint
    async fn delete(&self, checkpoint_id: &str) -> Result<()>;

    /// Get checkpoint metadata
    async fn metadata(&self, checkpoint_id: &str) -> Result<CheckpointMetadata>;
}

/// Filesystem-based checkpoint store
pub struct FilesystemCheckpointStore {
    base_path: PathBuf,
}

impl FilesystemCheckpointStore {
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    fn checkpoint_path(&self, agent_id: &str, version: u64) -> PathBuf {
        self.base_path
            .join(agent_id)
            .join(format!("ckpt-v{}.bin", version))
    }
}

#[async_trait]
impl CheckpointStore for FilesystemCheckpointStore {
    async fn store(
        &self,
        agent_id: &str,
        version: u64,
        data: Vec<u8>,
        checksum: String,
    ) -> Result<String> {
        let path = self.checkpoint_path(agent_id, version);
        fs::create_dir_all(path.parent().unwrap()).await?;

        fs::write(&path, &data).await?;

        let ckpt_id = format!("{}-v{}", agent_id, version);
        Ok(ckpt_id)
    }

    async fn retrieve(&self, checkpoint_id: &str) -> Result<Vec<u8>> {
        let parts: Vec<_> = checkpoint_id.split('-').collect();
        let agent_id = parts[0];
        let version: u64 = parts[1].strip_prefix('v')?.parse()?;

        let path = self.checkpoint_path(agent_id, version);
        fs::read(&path).await.map_err(Into::into)
    }

    async fn list(&self, agent_id: &str, limit: usize) -> Result<Vec<CheckpointRef>> {
        let agent_path = self.base_path.join(agent_id);
        let mut entries = fs::read_dir(&agent_path).await?;
        let mut checkpoints = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "bin") {
                let metadata = entry.metadata().await?;
                checkpoints.push(CheckpointRef {
                    id: path.file_stem().unwrap().to_string_lossy().to_string(),
                    agent_id: agent_id.to_string(),
                    version: 0, // Parse from filename
                    created_at: metadata.modified()?.into(),
                });
            }
        }

        checkpoints.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(checkpoints.into_iter().take(limit).collect())
    }

    async fn delete(&self, checkpoint_id: &str) -> Result<()> {
        let parts: Vec<_> = checkpoint_id.split('-').collect();
        let agent_id = parts[0];
        let version: u64 = parts[1].strip_prefix('v')?.parse()?;

        let path = self.checkpoint_path(agent_id, version);
        fs::remove_file(path).await.map_err(Into::into)
    }

    async fn metadata(&self, checkpoint_id: &str) -> Result<CheckpointMetadata> {
        let parts: Vec<_> = checkpoint_id.split('-').collect();
        let agent_id = parts[0];
        let version: u64 = parts[1].strip_prefix('v')?.parse()?;

        let path = self.checkpoint_path(agent_id, version);
        let file_metadata = fs::metadata(&path).await?;

        Ok(CheckpointMetadata {
            size_bytes: file_metadata.len(),
            created_at: file_metadata.modified()?.into(),
            version,
        })
    }
}
```

#### 6. **CsAgentctlHotReload** — CLI Integration
```rust
/// cs-agentctl hot-reload command support
pub struct CsAgentctlHotReload {
    orchestrator: Arc<HotReloadOrchestrator>,
}

impl CsAgentctlHotReload {
    /// CLI: cs-agentctl hot-reload <agent-id> --config <new-config>
    pub async fn handle_config_update(
        &self,
        agent_id: &str,
        config_path: &Path,
    ) -> Result<String> {
        let config = AgentConfig::load(config_path)?;
        let update_spec = AgentUpdateSpec::ConfigUpdate(config);

        let result = self.orchestrator.hot_reload(agent_id, update_spec).await?;

        Ok(format!(
            "Hot-reload completed in {:?}\nCheckpoint: {}",
            result.duration, result.checkpoint_ref.id
        ))
    }

    /// CLI: cs-agentctl checkpoint list <agent-id>
    pub async fn list_checkpoints(&self, agent_id: &str) -> Result<String> {
        let checkpoints = self.orchestrator.checkpoint_store
            .list_checkpoints(agent_id, 20)
            .await?;

        let mut output = String::from("Agent Checkpoints:\n");
        for ckpt in checkpoints {
            output.push_str(&format!(
                "  {}: {} ({})\n",
                ckpt.id, ckpt.agent_id, ckpt.created_at
            ));
        }

        Ok(output)
    }

    /// CLI: cs-agentctl checkpoint restore <agent-id> <checkpoint-id>
    pub async fn restore_checkpoint(
        &self,
        agent_id: &str,
        checkpoint_id: &str,
    ) -> Result<String> {
        let agent = self.orchestrator.agent_registry.get(agent_id)?;
        let checkpoint = self.orchestrator.checkpoint_store
            .deserialize_checkpoint(checkpoint_id)
            .await?;

        self.orchestrator.rollback_mgr.restore(&agent, &checkpoint).await?;

        Ok(format!("Restored agent {} to checkpoint {}", agent_id, checkpoint_id))
    }
}
```

---

## Testing

### Test Scenarios (10+)

```rust
#[cfg(test)]
mod hot_reload_tests {
    use super::*;

    #[tokio::test]
    async fn test_checkpoint_capture_complete_state() {
        let agent = create_test_agent().await;
        let checkpoint = AgentCheckpoint::capture(&agent).await.unwrap();

        assert!(!checkpoint.semantic_memory.is_empty());
        assert!(!checkpoint.conversation_history.is_empty());
        assert!(checkpoint.validate().is_ok());
    }

    #[tokio::test]
    async fn test_checkpoint_checksum_validation() {
        let mut checkpoint = create_test_checkpoint();
        assert!(checkpoint.validate().is_ok());

        checkpoint.semantic_memory.mutate(); // Corrupt
        assert!(checkpoint.validate().is_err());
    }

    #[tokio::test]
    async fn test_state_serialization_roundtrip() {
        let store = FilesystemCheckpointStore::new(temp_dir().into());
        let serializer = StateSerializer::new(Arc::new(store), CompressionCodec::Gzip);

        let original = create_test_checkpoint();
        let ref_id = serializer.serialize_checkpoint(&original).await.unwrap();
        let restored = serializer.deserialize_checkpoint(&ref_id).await.unwrap();

        assert_eq!(original.agent_id, restored.agent_id);
        assert_eq!(original.version, restored.version);
    }

    #[tokio::test]
    async fn test_hot_reload_config_update_zero_downtime() {
        let orchestrator = create_test_orchestrator().await;
        let agent_id = "test-agent";

        let update = AgentUpdateSpec::ConfigUpdate(/* new config */);
        let result = orchestrator.hot_reload(agent_id, update).await.unwrap();

        assert_eq!(result.status, HotReloadStatus::Success);
        assert!(result.duration.as_millis() < 500);
    }

    #[tokio::test]
    async fn test_rollback_on_update_failure() {
        let orchestrator = create_test_orchestrator().await;
        let agent_id = "test-agent";

        let original_config = orchestrator.agent_registry.get(agent_id)
            .unwrap()
            .current_config.clone();

        let bad_update = AgentUpdateSpec::ConfigUpdate(/* invalid config */);
        let _ = orchestrator.hot_reload(agent_id, bad_update).await;

        let restored_config = orchestrator.agent_registry.get(agent_id)
            .unwrap()
            .current_config.clone();

        assert_eq!(original_config, restored_config);
    }

    #[tokio::test]
    async fn test_crew_other_agents_continue_during_reload() {
        let crew = create_test_crew_with_3_agents().await;
        let agent1_id = &crew.agents[0].id;
        let agent2_id = &crew.agents[1].id;

        let update = AgentUpdateSpec::ConfigUpdate(/* update for agent1 */);

        // Start reload on agent1 in background
        let reload_task = tokio::spawn(async {
            crew.orchestrator.hot_reload(agent1_id, update).await
        });

        // Agent2 and Agent3 should continue working
        assert!(crew.agents[1].execute_task(/* task */).await.is_ok());
        assert!(crew.agents[2].execute_task(/* task */).await.is_ok());

        let _ = reload_task.await;
    }

    #[tokio::test]
    async fn test_semantic_memory_preservation() {
        let agent = create_test_agent().await;
        let memories = vec!["fact1", "fact2", "fact3"];
        for mem in &memories {
            agent.memory_store.store(*mem, 0.9).await.unwrap();
        }

        let checkpoint = AgentCheckpoint::capture(&agent).await.unwrap();
        assert_eq!(checkpoint.semantic_memory.entries().len(), 3);

        let new_agent = create_fresh_agent(agent.id.clone()).await;
        new_agent.resume_from_checkpoint(&checkpoint).await.unwrap();

        for mem in &memories {
            assert!(new_agent.memory_store.contains(*mem).await.unwrap());
        }
    }

    #[tokio::test]
    async fn test_conversation_history_preservation() {
        let agent = create_test_agent().await;
        let original_history_len = agent.conversation_manager.history_len().await;

        let checkpoint = AgentCheckpoint::capture(&agent).await.unwrap();
        assert_eq!(checkpoint.conversation_history.len(), original_history_len);

        let new_agent = create_fresh_agent(agent.id.clone()).await;
        new_agent.resume_from_checkpoint(&checkpoint).await.unwrap();

        assert_eq!(
            new_agent.conversation_manager.history_len().await,
            original_history_len
        );
    }

    #[tokio::test]
    async fn test_tool_state_preservation() {
        let agent = create_test_agent_with_tools().await;
        let original_tool_state = agent.tool_executor.capture_state().await.unwrap();

        let checkpoint = AgentCheckpoint::capture(&agent).await.unwrap();

        let new_agent = create_fresh_agent(agent.id.clone()).await;
        new_agent.resume_from_checkpoint(&checkpoint).await.unwrap();

        let restored_tool_state = new_agent.tool_executor.capture_state().await.unwrap();
        assert_eq!(original_tool_state, restored_tool_state);
    }

    #[tokio::test]
    async fn test_checkpoint_expiration_cleanup() {
        let rollback_mgr = create_test_rollback_manager().await;
        let agent_id = "test-agent";

        // Create old checkpoint
        let old_checkpoint = create_test_checkpoint_with_timestamp(
            SystemTime::now() - Duration::from_secs(86400 * 8)
        );
        rollback_mgr.storage.serialize_checkpoint(&old_checkpoint).await.unwrap();

        // Create recent checkpoint
        let new_checkpoint = create_test_checkpoint();
        rollback_mgr.storage.serialize_checkpoint(&new_checkpoint).await.unwrap();

        let deleted = rollback_mgr.cleanup_expired(agent_id).await.unwrap();
        assert_eq!(deleted, 1);
    }
}
```

---

## Implementation Checklist

- [ ] Implement `AgentCheckpoint` with complete state capture
- [ ] Build `StateSerializer` with compression and validation
- [ ] Develop `HotReloadOrchestrator` with atomic workflow
- [ ] Implement `RollbackManager` with transactional restore
- [ ] Create `CheckpointStore` and `FilesystemCheckpointStore`
- [ ] Integrate `CsAgentctlHotReload` with CLI
- [ ] Implement checkpoint versioning and retention policies
- [ ] Add audit logging for all hot-reload operations
- [ ] Develop monitoring metrics (reload duration, rollback count)
- [ ] Write comprehensive test suite (10+ scenarios)

---

## Acceptance Criteria

| Criterion | Target | Status |
|-----------|--------|--------|
| Checkpoint captures semantic memory, conversation, tools, config | 100% | - |
| State serialization roundtrip fidelity | 100% | - |
| Hot-reload completes within 500ms | 100% | - |
| Rollback atomic, sub-500ms | 100% | - |
| Other crew agents unaffected during reload | 100% | - |
| Checkpoint persistence (7-day TTL) | Configurable | - |
| `cs-agentctl` hot-reload commands functional | All 3 | - |
| Test coverage (hot-reload paths) | 10+ scenarios | - |
| Zero data loss on update success/failure | 100% | - |

---

## Design Principles

1. **Atomicity**: Checkpoint and rollback are all-or-nothing operations; no partial state transitions
2. **Isolation**: Agent hot-reload does not affect crew peers; concurrent execution continues
3. **Durability**: Checkpoints persisted to durable storage; recoverable after crashes
4. **Transparency**: Hot-reload is opaque to inference and tool execution; no API changes needed
5. **Observability**: Comprehensive audit logging and metrics for all state transitions
6. **Resilience**: Automatic rollback on failure; operational safety guaranteed

---

**Document Version:** 1.0
**Last Updated:** 2026-03-02
**Owner:** Engineer 8 | XKernal Core Team
