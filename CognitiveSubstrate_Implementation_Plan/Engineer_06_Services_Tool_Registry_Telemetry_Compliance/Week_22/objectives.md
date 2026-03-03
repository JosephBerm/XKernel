# Engineer 6 — Services: Tool Registry, Telemetry & Compliance — Week 22

## Phase: Phase 3 (Weeks 25-36) Prep

## Weekly Objective
Optimize Phase 2 services for production and Phase 3 load testing. Focus on critical path performance: cache lookups, policy evaluation, event emission.

## Document References
- **Primary:** Week 21 (Phase 3 planning), Phase 2 components

## Deliverables
- [ ] Performance optimization
  - Cache lookup latency <1ms (currently on track)
  - Policy evaluation <5ms p99 (currently on track)
  - Event emission <1ms (target improvement if needed)
  - Critical path profiling
- [ ] Memory optimization
  - Reduce in-memory cache footprint
  - Optimize Merkle-tree node structure
  - Reduce policy rule parsing overhead
  - Profile and document memory usage patterns
- [ ] Concurrent access optimization
  - Lock-free reads where possible (RwLock optimization)
  - Reduce contention on shared data structures
  - Batch operations to reduce lock-hold time
- [ ] Database optimization (SQLite compliance tier)
  - Index optimization for common queries
  - Query plan analysis
  - Write batching for policy decision logging
- [ ] Network optimization (gRPC telemetry streaming)
  - Connection pooling (already done)
  - Message compression
  - Backpressure handling
- [ ] Documentation
  - Performance optimization guide
  - Tuning recommendations for high-scale deployments
  - Bottleneck analysis and solutions

## Technical Specifications

### Critical Path Optimization
```rust
// Before: RwLock on every read
impl ResponseCache {
    pub async fn get(&self, key: &str) -> Option<String> {
        let cache = self.cache.read().await;  // LOCK
        cache.get(key).cloned()               // LOOKUP
                                              // UNLOCK
    }
}

// After: Arc<DashMap> for lock-free reads
pub struct ResponseCacheOptimized {
    cache: Arc<DashMap<String, CachedResponse>>,
    stats: Arc<CacheStats>,
}

impl ResponseCacheOptimized {
    pub fn get(&self, key: &str) -> Option<String> {
        self.cache.get(key).map(|v| v.value.clone())  // LOCK-FREE
    }
}

// Before: Policy evaluation iterates rules sequentially
impl MandatoryPolicyEngine {
    pub async fn evaluate_capability_request(&self, input: PolicyDecisionInput)
        -> Result<PolicyOutcome, PolicyError>
    {
        let policies = self.policies.read().await;
        for rule in &policies.rules {
            if self.evaluate_condition(&rule.condition, &input).await? {
                return Ok(rule.decision.clone());
            }
        }
        Ok(PolicyOutcome::Deny)
    }
}

// After: Add rule indexing and short-circuiting
impl MandatoryPolicyEngine {
    pub async fn evaluate_capability_request_optimized(&self, input: &PolicyDecisionInput)
        -> Result<PolicyOutcome, PolicyError>
    {
        // Check fast-path rules first (most common denies/allows)
        if let Some(outcome) = self.check_fast_path_rules(input) {
            return Ok(outcome);
        }

        // Then slower evaluation rules
        let policies = self.policies.read().await;
        for rule in policies.rules.iter().take(10) { // Limit iterations
            if self.evaluate_condition_quick(&rule.condition, input).await? {
                return Ok(rule.decision.clone());
            }
        }

        Ok(PolicyOutcome::Deny)
    }

    fn check_fast_path_rules(&self, input: &PolicyDecisionInput) -> Option<PolicyOutcome> {
        // Pre-computed index of common decisions
        if input.requested_capability.starts_with("readonly.") {
            Some(PolicyOutcome::Allow)
        } else if input.requested_capability.starts_with("admin.") {
            Some(PolicyOutcome::RequireApproval)
        } else {
            None
        }
    }
}
```

### Memory Optimization
```rust
// Before: Full policy rule stored in memory
pub struct PolicyRule {
    pub id: String,
    pub description: String,
    pub condition: PolicyCondition,
    pub decision: PolicyOutcome,
    pub explanation: Option<String>,
}

// After: Compact representation with lazy loading
pub struct PolicyRuleCompact {
    pub id: u16,  // Use ID index instead of String
    pub desc_idx: u16,  // Index into description table
    pub cond_idx: u16,  // Index into condition table
    pub decision: PolicyOutcome,
    pub expl_idx: Option<u16>,
}

pub struct PolicyRuleTable {
    rules: Vec<PolicyRuleCompact>,
    descriptions: Vec<String>,
    conditions: Vec<PolicyCondition>,
    explanations: Vec<String>,
}

// Before: Every policy decision stores full content
pub struct PolicyDecision {
    pub decision_id: String,
    pub timestamp: i64,
    pub input: PolicyDecisionInput,
    pub outcome: PolicyOutcome,
    pub explanation: String,  // Can be large
}

// After: Store commitment + metadata only
pub struct PolicyDecisionCompact {
    pub decision_id: u64,  // Numeric ID
    pub timestamp: i32,    // Relative timestamp
    pub input_hash: u64,   // Deterministic hash
    pub outcome: PolicyOutcome,
    pub expl_hash: u64,    // Hash of explanation (for dedup)
}
```

### Database Query Optimization
```rust
// Create indexes for common queries
CREATE INDEX idx_policy_decisions_timestamp ON policy_decisions(timestamp);
CREATE INDEX idx_policy_decisions_outcome ON policy_decisions(outcome);
CREATE INDEX idx_policy_decisions_rule_id ON policy_decisions(matching_rule_id);

// Batch write policy decisions
pub struct PolicyDecisionBatcher {
    batch: Arc<Mutex<Vec<PolicyDecision>>>,
    batch_size: usize,
}

impl PolicyDecisionBatcher {
    pub async fn add(&self, decision: PolicyDecision) -> Result<(), BatchError> {
        let mut batch = self.batch.lock().await;
        batch.push(decision);

        if batch.len() >= self.batch_size {
            self.flush().await?;
        }

        Ok(())
    }

    pub async fn flush(&self) -> Result<(), BatchError> {
        let batch: Vec<PolicyDecision> = self.batch.lock().await.drain(..).collect();

        // Single INSERT with multiple rows
        // INSERT INTO policy_decisions VALUES (...), (...), (...)
        // Instead of N separate INSERTs

        Ok(())
    }
}
```

## Dependencies
- **Blocked by:** Week 21 (planning complete)
- **Blocking:** Week 23-24 (deployment and final preparation)

## Acceptance Criteria
- [ ] Cache lookup latency verified <1ms
- [ ] Policy evaluation latency verified <5ms p99
- [ ] Event emission latency optimized
- [ ] Lock-free reads where applicable
- [ ] Memory footprint reduced
- [ ] Database indexes created and verified
- [ ] Batch operations implemented
- [ ] Performance tuning guide documented
- [ ] Bottleneck analysis completed
- [ ] Ready for Phase 3 load testing

## Design Principles Alignment
- **Performance:** Critical paths optimized for production scale
- **Scalability:** Lock-free design enables horizontal scaling
- **Efficiency:** Memory usage optimized for resource-constrained environments
- **Maintainability:** Optimizations documented for future tuning
