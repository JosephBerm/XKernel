# Week 15: Pinecone Vector Database Mounting
## XKernal L2 Runtime (Semantic FS & Agent Lifecycle) — Phase 2, Week 1

**Document**: Technical Design
**Phase**: Phase 2 (Knowledge Source Integration)
**Layer**: L2 Runtime (Rust)
**Status**: Implementation Plan

---

## 1. Objective & Scope

Week 15 initiates Phase 2 with **Pinecone vector database mounting** as the first concrete knowledge source semantic volume. This work extends the Semantic FS architecture completed in Phase 1 to support external knowledge integrations with full lifecycle management, authentication, and intelligent query translation.

### Deliverables
- Pinecone mount driver in CSCI `mem_mount` interface
- Complete mount lifecycle state machine (register → validate → enable → disable → unregister)
- API key authentication with credential rotation
- Natural language → vector query translation layer
- Capability-gating enforcement
- Integration test suite

---

## 2. Architectural Overview

### 2.1 Mount Integration Points

```
Agent Request
    ↓
Semantic FS Parser (Phase 1)
    ↓
Capability Check (mount access gates)
    ↓
Query Translator (NL → Vector format)
    ↓
Pinecone Mount Driver
    ↓
Pinecone API
```

The Pinecone mount operates as a **stateful capability-gated semantic volume** within the existing Semantic FS architecture. The mount lifecycle integrates with the agent lifecycle checkpoint system to ensure consistency.

### 2.2 Design Principles Applied

- **Composability**: Mount interface reusable for future sources (Weaviate, Milvus, etc.)
- **Security**: Credential isolation, capability-based access control, audit logging
- **Reliability**: State persistence, graceful degradation, health monitoring

---

## 3. Mount Lifecycle State Machine

```
┌─────────────┐
│  Unregistered │
└────────┬────┘
         │ register()
         ↓
┌─────────────────┐
│  Registered     │
└────────┬────────┘
         │ validate()
         ↓
┌──────────────────┐
│  Validated       │
└────────┬─────────┘
         │ enable()
         ↓
┌──────────────────┐
│  Enabled         │◄─────────┐
└────────┬─────────┘          │
         │ disable()          │ health_check()
         ↓                    │ (periodic)
┌──────────────────┐          │
│  Disabled        │──────────┘
└────────┬─────────┘
         │ unregister()
         ↓
┌─────────────────┐
│  Unregistered   │
└─────────────────┘
```

### 3.1 State Transitions

```rust
pub enum PineconeMountState {
    Unregistered,
    Registered { api_key_ref: String, index_name: String },
    Validated { metadata: IndexMetadata },
    Enabled { active_since: SystemTime },
    Disabled { reason: String },
}

impl PineconeMount {
    /// Transition: Unregistered -> Registered
    pub async fn register(
        &mut self,
        api_key_ref: &str,
        index_name: &str,
        config: MountConfig,
    ) -> Result<(), MountError> {
        if !matches!(self.state, PineconeMountState::Unregistered) {
            return Err(MountError::InvalidStateTransition);
        }

        self.state = PineconeMountState::Registered {
            api_key_ref: api_key_ref.to_string(),
            index_name: index_name.to_string(),
        };
        self.persist_state().await?;
        Ok(())
    }

    /// Transition: Registered -> Validated
    pub async fn validate(&mut self) -> Result<IndexMetadata, MountError> {
        let (api_key_ref, index_name) = match &self.state {
            PineconeMountState::Registered { api_key_ref, index_name } => {
                (api_key_ref.clone(), index_name.clone())
            }
            _ => return Err(MountError::InvalidStateTransition),
        };

        let metadata = self
            .pinecone_client
            .describe_index(&index_name, &api_key_ref)
            .await?;

        self.state = PineconeMountState::Validated { metadata: metadata.clone() };
        self.persist_state().await?;
        Ok(metadata)
    }

    /// Transition: Validated -> Enabled
    pub async fn enable(&mut self) -> Result<(), MountError> {
        if !matches!(self.state, PineconeMountState::Validated { .. }) {
            return Err(MountError::InvalidStateTransition);
        }

        self.state = PineconeMountState::Enabled {
            active_since: SystemTime::now(),
        };
        self.health_check_interval = Some(Duration::from_secs(30));
        self.persist_state().await?;
        Ok(())
    }

    /// Transition: Enabled -> Disabled
    pub async fn disable(&mut self, reason: String) -> Result<(), MountError> {
        if !matches!(self.state, PineconeMountState::Enabled { .. }) {
            return Err(MountError::InvalidStateTransition);
        }

        self.state = PineconeMountState::Disabled { reason };
        self.health_check_interval = None;
        self.persist_state().await?;
        Ok(())
    }

    /// Transition: Any -> Unregistered
    pub async fn unregister(&mut self) -> Result<(), MountError> {
        self.state = PineconeMountState::Unregistered;
        self.persist_state().await?;
        Ok(())
    }
}
```

---

## 4. Authentication & Credential Management

### 4.1 Credential Rotation

API keys are managed through a **credential store** with rotation support and audit logging. Keys never appear in agent memory; only references are stored.

```rust
pub struct CredentialManager {
    store: Arc<RwLock<HashMap<String, StoredCredential>>>,
    audit_log: Arc<AuditLog>,
}

pub struct StoredCredential {
    key_id: String,
    encrypted_value: Vec<u8>,
    rotation_interval: Duration,
    last_rotated: SystemTime,
    status: CredentialStatus,
}

pub enum CredentialStatus {
    Active,
    RotationPending,
    Revoked { reason: String },
}

impl CredentialManager {
    pub async fn register_credential(
        &self,
        key_id: &str,
        api_key: &str,
        rotation_interval: Duration,
    ) -> Result<(), CredentialError> {
        let encrypted = self.encrypt(api_key)?;
        let cred = StoredCredential {
            key_id: key_id.to_string(),
            encrypted_value: encrypted,
            rotation_interval,
            last_rotated: SystemTime::now(),
            status: CredentialStatus::Active,
        };
        self.store.write().await.insert(key_id.to_string(), cred);
        self.audit_log.log_credential_registered(key_id).await;
        Ok(())
    }

    pub async fn get_credential(&self, key_id: &str) -> Result<String, CredentialError> {
        let store = self.store.read().await;
        let cred = store.get(key_id).ok_or(CredentialError::NotFound)?;

        if matches!(cred.status, CredentialStatus::Revoked { .. }) {
            return Err(CredentialError::Revoked);
        }

        self.audit_log.log_credential_access(key_id).await;
        let decrypted = self.decrypt(&cred.encrypted_value)?;
        Ok(decrypted)
    }

    pub async fn rotate_credential(
        &self,
        key_id: &str,
        new_api_key: &str,
    ) -> Result<(), CredentialError> {
        let mut store = self.store.write().await;
        let cred = store.get_mut(key_id).ok_or(CredentialError::NotFound)?;

        cred.encrypted_value = self.encrypt(new_api_key)?;
        cred.last_rotated = SystemTime::now();
        cred.status = CredentialStatus::Active;

        self.audit_log.log_credential_rotated(key_id).await;
        Ok(())
    }

    fn encrypt(&self, plaintext: &str) -> Result<Vec<u8>, CredentialError> {
        // Use ChaCha20Poly1305 with key-rotation envelope
        Ok(self.cipher.encrypt_authenticated(plaintext.as_bytes())?)
    }

    fn decrypt(&self, ciphertext: &[u8]) -> Result<String, CredentialError> {
        let plaintext = self.cipher.decrypt_authenticated(ciphertext)?;
        Ok(String::from_utf8(plaintext)?)
    }
}
```

---

## 5. Natural Language → Vector Query Translation

### 5.1 Query Translation Pipeline

Agent-issued NL queries are translated through a **composable translation layer** that handles:
- Entity extraction (concepts, keywords)
- Semantic intent classification
- Vector embedding generation
- Metadata filter construction

```rust
pub struct QueryTranslator {
    embedding_model: Arc<EmbeddingModel>,
    intent_classifier: Arc<IntentClassifier>,
}

pub struct TranslatedQuery {
    vector: Vec<f32>,
    metadata_filter: Option<MetadataFilter>,
    top_k: usize,
    include_metadata: bool,
}

impl QueryTranslator {
    pub async fn translate(
        &self,
        nl_query: &str,
        context: &QueryContext,
    ) -> Result<TranslatedQuery, TranslationError> {
        // Step 1: Extract intent and metadata constraints
        let intent = self.intent_classifier.classify(nl_query).await?;
        let (entities, filters) = self.extract_entities_and_filters(nl_query)?;

        // Step 2: Generate embedding for semantic search
        let embedding = self.embedding_model.embed(nl_query).await?;

        // Step 3: Construct metadata filters if applicable
        let metadata_filter = if !filters.is_empty() {
            Some(self.build_metadata_filter(filters, intent)?)
        } else {
            None
        };

        // Step 4: Determine result cardinality from context
        let top_k = context.max_results.unwrap_or(10);

        Ok(TranslatedQuery {
            vector: embedding,
            metadata_filter,
            top_k,
            include_metadata: true,
        })
    }

    fn extract_entities_and_filters(
        &self,
        query: &str,
    ) -> Result<(Vec<String>, Vec<MetadataConstraint>), TranslationError> {
        // NER + regex patterns for structured extraction
        let entities = self.ner_model.extract_entities(query)?;
        let filters = self.extract_temporal_spatial_constraints(query)?;
        Ok((entities, filters))
    }

    fn build_metadata_filter(
        &self,
        constraints: Vec<MetadataConstraint>,
        intent: Intent,
    ) -> Result<MetadataFilter, TranslationError> {
        let mut filter = MetadataFilter::default();

        for constraint in constraints {
            match constraint {
                MetadataConstraint::Temporal { from, to } => {
                    filter.add_range_filter("timestamp", from, to)?;
                }
                MetadataConstraint::Category { value } => {
                    filter.add_equality_filter("category", &value)?;
                }
                MetadataConstraint::Source { value } => {
                    filter.add_equality_filter("source", &value)?;
                }
            }
        }

        Ok(filter)
    }
}
```

### 5.2 Pinecone-Specific Query Execution

```rust
pub struct PineconeQueryExecutor {
    client: Arc<PineconeClient>,
    credential_manager: Arc<CredentialManager>,
}

impl PineconeQueryExecutor {
    pub async fn execute(
        &self,
        index_name: &str,
        api_key_ref: &str,
        query: TranslatedQuery,
    ) -> Result<SearchResults, QueryExecutionError> {
        let api_key = self.credential_manager.get_credential(api_key_ref).await?;

        let pinecone_query = PineconeQuery {
            vector: query.vector,
            top_k: query.top_k,
            filter: query.metadata_filter,
            include_metadata: query.include_metadata,
        };

        let results = self
            .client
            .query(index_name, &api_key, pinecone_query)
            .await?;

        Ok(self.convert_results(results))
    }

    fn convert_results(&self, pinecone_results: Vec<Match>) -> SearchResults {
        SearchResults {
            matches: pinecone_results
                .into_iter()
                .map(|m| SearchMatch {
                    id: m.id,
                    score: m.score,
                    metadata: m.metadata,
                })
                .collect(),
            stats: QueryStats { execution_time_ms: 0 },
        }
    }
}
```

---

## 6. Capability-Gating Integration

Mount access is controlled through the **capability-based security model** integrated with the agent lifecycle system. Agents must possess explicit capabilities to query specific mounts.

```rust
pub struct MountCapability {
    mount_id: String,
    agent_id: String,
    permissions: CapabilityPermissions,
    granted_at: SystemTime,
    expires_at: Option<SystemTime>,
}

pub struct CapabilityPermissions {
    can_query: bool,
    can_list_metadata: bool,
    rate_limit_qps: Option<f64>,
}

pub struct CapabilityGate {
    store: Arc<RwLock<HashMap<(String, String), MountCapability>>>,
}

impl CapabilityGate {
    pub async fn check_access(
        &self,
        agent_id: &str,
        mount_id: &str,
        action: &CapabilityAction,
    ) -> Result<bool, CapabilityError> {
        let key = (agent_id.to_string(), mount_id.to_string());
        let store = self.store.read().await;

        let capability = store.get(&key).ok_or(CapabilityError::NotGranted)?;

        // Check expiration
        if let Some(expires_at) = capability.expires_at {
            if SystemTime::now() > expires_at {
                return Err(CapabilityError::Expired);
            }
        }

        // Check specific permission
        let allowed = match action {
            CapabilityAction::Query => capability.permissions.can_query,
            CapabilityAction::ListMetadata => capability.permissions.can_list_metadata,
        };

        Ok(allowed)
    }

    pub async fn grant_capability(
        &self,
        capability: MountCapability,
    ) -> Result<(), CapabilityError> {
        let key = (capability.agent_id.clone(), capability.mount_id.clone());
        self.store.write().await.insert(key, capability);
        Ok(())
    }

    pub async fn revoke_capability(
        &self,
        agent_id: &str,
        mount_id: &str,
    ) -> Result<(), CapabilityError> {
        let key = (agent_id.to_string(), mount_id.to_string());
        self.store.write().await.remove(&key);
        Ok(())
    }
}

pub enum CapabilityAction {
    Query,
    ListMetadata,
}
```

---

## 7. CSCI mem_mount Interface Implementation

The Pinecone driver implements the standard `mem_mount` trait for composability with future knowledge sources.

```rust
#[async_trait]
pub trait SemanticMount: Send + Sync {
    async fn register(&mut self, config: MountConfig) -> Result<(), MountError>;
    async fn validate(&mut self) -> Result<MountMetadata, MountError>;
    async fn enable(&mut self) -> Result<(), MountError>;
    async fn disable(&mut self, reason: String) -> Result<(), MountError>;
    async fn unregister(&mut self) -> Result<(), MountError>;
    async fn query(&self, query: MountQuery) -> Result<QueryResults, MountError>;
    async fn health_check(&self) -> Result<HealthStatus, MountError>;
}

pub struct PineconeMount {
    mount_id: String,
    state: PineconeMountState,
    client: Arc<PineconeClient>,
    credential_manager: Arc<CredentialManager>,
    capability_gate: Arc<CapabilityGate>,
    translator: Arc<QueryTranslator>,
    state_store: Arc<StateStore>,
}

#[async_trait]
impl SemanticMount for PineconeMount {
    async fn register(&mut self, config: MountConfig) -> Result<(), MountError> {
        let api_key_ref = config.get("api_key_ref")?;
        let index_name = config.get("index_name")?;
        self.register(api_key_ref, index_name, config).await
    }

    async fn validate(&mut self) -> Result<MountMetadata, MountError> {
        self.validate()
            .await
            .map(|metadata| MountMetadata {
                index_name: metadata.name,
                dimension: metadata.dimension,
                metric: metadata.metric,
            })
            .map_err(Into::into)
    }

    async fn enable(&mut self) -> Result<(), MountError> {
        self.enable().await
    }

    async fn disable(&mut self, reason: String) -> Result<(), MountError> {
        self.disable(reason).await
    }

    async fn unregister(&mut self) -> Result<(), MountError> {
        self.unregister().await
    }

    async fn query(&self, query: MountQuery) -> Result<QueryResults, MountError> {
        // Step 1: Check agent capability
        self.capability_gate
            .check_access(&query.agent_id, &self.mount_id, &CapabilityAction::Query)
            .await?;

        // Step 2: Translate NL query to vector query
        let translated = self.translator.translate(&query.text, &query.context).await?;

        // Step 3: Execute query against Pinecone
        let results = self.execute_query(translated).await?;

        Ok(results)
    }

    async fn health_check(&self) -> Result<HealthStatus, MountError> {
        match &self.state {
            PineconeMountState::Enabled { .. } => {
                match self.client.describe_index_stats().await {
                    Ok(_) => Ok(HealthStatus::Healthy),
                    Err(_) => Ok(HealthStatus::Degraded),
                }
            }
            PineconeMountState::Disabled { reason } => {
                Ok(HealthStatus::Unhealthy(reason.clone()))
            }
            _ => Ok(HealthStatus::Unhealthy("Not enabled".to_string())),
        }
    }
}
```

---

## 8. Integration with Agent Lifecycle Checkpoints

Pinecone mount state is persisted alongside agent checkpoints to ensure consistency across restarts and migrations.

```rust
pub struct MountCheckpoint {
    mount_id: String,
    state: SerializedPineconeMountState,
    last_health_check: SystemTime,
    query_stats: QueryStatistics,
}

impl PineconeMount {
    pub async fn create_checkpoint(&self) -> Result<MountCheckpoint, CheckpointError> {
        Ok(MountCheckpoint {
            mount_id: self.mount_id.clone(),
            state: self.serialize_state()?,
            last_health_check: SystemTime::now(),
            query_stats: self.collect_stats(),
        })
    }

    pub async fn restore_from_checkpoint(
        &mut self,
        checkpoint: MountCheckpoint,
    ) -> Result<(), CheckpointError> {
        self.state = checkpoint.state.deserialize()?;
        Ok(())
    }
}
```

---

## 9. Test Suite

### 9.1 Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mount_state_transitions() {
        let mut mount = PineconeMount::new();
        assert!(matches!(mount.state, PineconeMountState::Unregistered));

        mount.register("test_key", "test_index", Default::default()).await.unwrap();
        assert!(matches!(mount.state, PineconeMountState::Registered { .. }));
    }

    #[tokio::test]
    async fn test_credential_rotation() {
        let cred_mgr = CredentialManager::new();
        cred_mgr
            .register_credential("key1", "secret123", Duration::from_secs(3600))
            .await
            .unwrap();

        let secret = cred_mgr.get_credential("key1").await.unwrap();
        assert_eq!(secret, "secret123");

        cred_mgr.rotate_credential("key1", "newsecret456").await.unwrap();
        let new_secret = cred_mgr.get_credential("key1").await.unwrap();
        assert_eq!(new_secret, "newsecret456");
    }

    #[tokio::test]
    async fn test_query_translation() {
        let translator = QueryTranslator::new();
        let query = translator
            .translate("Find articles about machine learning", &Default::default())
            .await
            .unwrap();

        assert!(!query.vector.is_empty());
        assert_eq!(query.top_k, 10);
    }

    #[tokio::test]
    async fn test_capability_gating() {
        let gate = CapabilityGate::new();

        // Deny by default
        let result = gate.check_access("agent1", "mount1", &CapabilityAction::Query).await;
        assert!(result.is_err());

        // Grant capability
        let cap = MountCapability {
            mount_id: "mount1".to_string(),
            agent_id: "agent1".to_string(),
            permissions: CapabilityPermissions {
                can_query: true,
                can_list_metadata: false,
                rate_limit_qps: Some(10.0),
            },
            granted_at: SystemTime::now(),
            expires_at: None,
        };
        gate.grant_capability(cap).await.unwrap();

        // Allow after grant
        assert!(gate.check_access("agent1", "mount1", &CapabilityAction::Query).await.unwrap());
    }
}
```

### 9.2 Integration Tests

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_end_to_end_mount_lifecycle() {
        // Setup
        let mut mount = PineconeMount::new_with_test_client();
        let cred_mgr = Arc::new(CredentialManager::new());

        // Register
        mount.register("test_key", "test_index", Default::default()).await.unwrap();

        // Validate
        let metadata = mount.validate().await.unwrap();
        assert!(!metadata.name.is_empty());

        // Enable
        mount.enable().await.unwrap();

        // Health check
        let health = mount.health_check().await.unwrap();
        assert!(matches!(health, HealthStatus::Healthy));

        // Disable
        mount.disable("Testing".to_string()).await.unwrap();

        // Unregister
        mount.unregister().await.unwrap();
        assert!(matches!(mount.state, PineconeMountState::Unregistered));
    }

    #[tokio::test]
    async fn test_query_with_capability_enforcement() {
        let mut mount = setup_enabled_mount().await;
        let gate = Arc::new(CapabilityGate::new());

        // Query without capability should fail
        let query = MountQuery {
            agent_id: "unauthorized_agent".to_string(),
            text: "find data".to_string(),
            context: Default::default(),
        };
        assert!(mount.query(query).await.is_err());

        // Grant capability
        let cap = MountCapability {
            mount_id: mount.mount_id.clone(),
            agent_id: "authorized_agent".to_string(),
            permissions: CapabilityPermissions {
                can_query: true,
                can_list_metadata: true,
                rate_limit_qps: None,
            },
            granted_at: SystemTime::now(),
            expires_at: None,
        };
        gate.grant_capability(cap).await.unwrap();

        // Query with capability should succeed
        let query = MountQuery {
            agent_id: "authorized_agent".to_string(),
            text: "find data".to_string(),
            context: Default::default(),
        };
        assert!(mount.query(query).await.is_ok());
    }
}
```

---

## 10. Future Extensions

- **Query result caching** with TTL-based invalidation
- **Multi-index federation** for cross-index queries
- **Semantic deduplication** in result sets
- **Observability dashboard** for mount health and query performance
- **Support for additional sources**: Weaviate, Milvus, Qdrant

---

## 11. References

- CSCI mem_mount interface specification (Phase 1)
- Pinecone API documentation: https://docs.pinecone.io
- Credential rotation RFC: XKernal-SEC-2025-001
- Capability-based security model: XKernal-ARCH-2024-012
