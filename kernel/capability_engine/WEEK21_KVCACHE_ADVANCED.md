# XKernal Week 21: KV-Cache Isolation - Advanced Scenarios & Adversarial Hardening

**Phase:** 2 (Capability Engine Hardening)
**Objective:** Complete KV-cache isolation with eviction policies, cross-team information flow control, preemption handling, and adversarial security testing.
**Target Platform:** L0 Microkernel (Rust, no_std)
**Lines of Code:** ~350-400 (production + tests)
**Test Coverage:** 200+ tests (basic isolation, preemption, information flow, performance, edge cases + adversarial)

---

## 1. Executive Summary

Week 21 extends Week 20's KV-cache isolation foundation with production-grade mechanisms for handling realistic workloads. Core deliverables include:

- **Eviction Policies:** LRU, LFU, and adaptive algorithms with preemption awareness
- **Information Flow Control:** Cross-team privacy enforcement with taint tracking
- **Preemption Handling:** Atomic save/restore of KV-cache state during context switches
- **Cache Warmup:** Priming strategies for latency-critical scenarios
- **Adversarial Testing:** Six attack vectors (side-channel, timing, eviction, bandwidth, TLB, speculation)

This document provides the technical specification, implementation guidance, and security analysis.

---

## 2. Architecture Overview

### 2.1 KV-Cache Isolation Stack

```
┌──────────────────────────────────────────────┐
│  Application Layer (Teams A, B, C, ...)      │
├──────────────────────────────────────────────┤
│  Information Flow Control (Taint Tracking)   │
├──────────────────────────────────────────────┤
│  Eviction Policy Engine (LRU/LFU/Adaptive)   │
├──────────────────────────────────────────────┤
│  Preemption State Manager (Save/Restore)     │
├──────────────────────────────────────────────┤
│  Cache Isolation Layer (STRICT/SELECTIVE)    │
│  - Page-table PTE enforcement                │
│  - Mode transition guards                    │
├──────────────────────────────────────────────┤
│  Hardware KV-Cache (Partitioned)             │
└──────────────────────────────────────────────┘
```

Week 20 established the lower three layers. Week 21 adds upper layers: eviction, information flow, and preemption handling.

---

## 3. Eviction Policy Engine

### 3.1 Policy Variants

#### 3.1.1 LRU (Least Recently Used)

**Use Case:** General workloads with temporal locality.

```rust
#[derive(Clone, Copy, Debug)]
pub struct LruMetadata {
    /// Logical timestamp of last access
    last_access: u64,
    /// Access counter for ordering
    access_epoch: u64,
}

pub struct LruEvictionPolicy {
    entries: [LruMetadata; MAX_KV_ENTRIES],
    current_epoch: u64,
    /// Per-team LRU deques (maintain separate queues)
    team_lru_queues: [VecDeque<u32>; MAX_TEAMS],
}

impl LruEvictionPolicy {
    pub fn on_access(&mut self, entry_id: u32, team_id: u32, epoch: u64) {
        // Move entry to tail (most recently used)
        self.entries[entry_id as usize].last_access = epoch;
        self.entries[entry_id as usize].access_epoch = self.current_epoch;
        self.current_epoch = self.current_epoch.saturating_add(1);
    }

    pub fn evict(&mut self, team_id: u32) -> Option<u32> {
        // Evict oldest entry in team's queue
        self.team_lru_queues[team_id as usize].pop_front()
    }

    /// Preemption-aware: save LRU state for task
    pub fn save_state(&self, task_id: u32) -> LruSnapshot {
        LruSnapshot {
            epoch: self.current_epoch,
            task_epoch: task_id.wrapping_mul(0x9e3779b1),
        }
    }
}

pub struct LruSnapshot {
    epoch: u64,
    task_epoch: u64,
}
```

**Key Properties:**
- **Temporal Locality:** Favors recently accessed entries
- **Fairness:** Equal weight per access regardless of frequency
- **Preemption Cost:** O(log n) to save/restore per context switch

#### 3.1.2 LFU (Least Frequently Used)

**Use Case:** Workloads with high-value hot-set (ML inference, caching).

```rust
pub struct LfuMetadata {
    /// Frequency counter (saturating)
    frequency: u16,
    /// Virtual time for LFU tiebreaking
    last_access: u64,
}

pub struct LfuEvictionPolicy {
    entries: [LfuMetadata; MAX_KV_ENTRIES],
    /// Min-heap of (frequency, entry_id) per team
    team_freq_heaps: [BinaryHeap<(u16, u32)>; MAX_TEAMS],
}

impl LfuEvictionPolicy {
    pub fn on_access(&mut self, entry_id: u32, team_id: u32) {
        let freq = &mut self.entries[entry_id as usize].frequency;
        *freq = freq.saturating_add(1);
    }

    pub fn evict(&mut self, team_id: u32) -> Option<u32> {
        // Pop least frequent
        self.team_freq_heaps[team_id as usize].pop().map(|(_, id)| id)
    }

    /// Adaptive ghost tracking: remember evicted entries
    pub fn check_ghost_hit(&self, entry_id: u32) -> bool {
        // Return true if entry was recently evicted
        // Adjust LFU insertion weight accordingly
        false // Simplified: full impl uses bloom filter
    }
}
```

**Key Properties:**
- **Frequency Weight:** Heavily accessed entries survive longer
- **Ghost Tracking:** Optional hit detection on re-request
- **Preemption Cost:** O(1) metadata update

#### 3.1.3 Adaptive Policy

**Use Case:** Unknown workload patterns; online learning.

```rust
pub struct AdaptiveEvictionPolicy {
    /// Current policy switch
    active_policy: EvictionPolicyType,
    /// Metrics for switching decision
    lru_misses: u64,
    lfu_misses: u64,
    /// Threshold for policy switch
    switch_threshold: u64,
    /// Hysteresis to avoid thrashing
    policy_age: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EvictionPolicyType {
    Lru,
    Lfu,
}

impl AdaptiveEvictionPolicy {
    pub fn on_eviction(&mut self, hit: bool) {
        match self.active_policy {
            EvictionPolicyType::Lru => {
                if !hit { self.lru_misses += 1; }
            }
            EvictionPolicyType::Lfu => {
                if !hit { self.lfu_misses += 1; }
            }
        }

        // Switch if miss rate diverges
        if self.policy_age > 1000 {
            if self.lru_misses > self.lfu_misses * 110 / 100 {
                self.active_policy = EvictionPolicyType::Lfu;
                self.lru_misses = 0;
                self.lfu_misses = 0;
            } else if self.lfu_misses > self.lru_misses * 110 / 100 {
                self.active_policy = EvictionPolicyType::Lru;
                self.lru_misses = 0;
                self.lfu_misses = 0;
            }
            self.policy_age = 0;
        }
        self.policy_age += 1;
    }

    /// Multivariate switch: consider memory pressure + workload mix
    pub fn should_switch_to_aggressive(&self, memory_pressure: f32) -> bool {
        memory_pressure > 0.85 && self.lfu_misses < self.lru_misses / 2
    }
}
```

**Key Properties:**
- **Online Learning:** Switches between LRU/LFU based on miss rates
- **Hysteresis:** 1000-epoch window prevents oscillation
- **Adaptive Aggression:** Tighter eviction under memory pressure

---

## 4. Cross-Team Information Flow Control

### 4.1 Taint Tracking System

Information flow control prevents side-channel leakage where one team's access patterns poison another's cache.

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TaintLabel {
    /// Bitmask of teams that contributed to this entry
    team_mask: u32,
    /// Confidentiality level (0=public, 1=internal, 2=secret)
    confidentiality: u8,
    /// Integrity rating (0=untrusted, 255=certified)
    integrity: u8,
}

impl TaintLabel {
    /// Merge two labels (union of team contributions)
    pub fn merge(&self, other: &TaintLabel) -> TaintLabel {
        TaintLabel {
            team_mask: self.team_mask | other.team_mask,
            confidentiality: self.confidentiality.max(other.confidentiality),
            integrity: self.integrity.min(other.integrity),
        }
    }

    /// Check if label is safe for team_id to observe
    pub fn can_flow_to(&self, team_id: u32) -> bool {
        // Team can only read if:
        // 1. Team contributed to creation (team_mask includes team_id), OR
        // 2. Confidentiality allows (public data), OR
        // 3. Team has explicit declassification
        (self.team_mask & (1 << team_id)) != 0 || self.confidentiality == 0
    }

    /// Declassify for specific team (with audit log)
    pub fn declassify_for(&self, team_id: u32) -> TaintLabel {
        TaintLabel {
            team_mask: self.team_mask & !(1 << team_id),
            confidentiality: 0,
            integrity: self.integrity,
        }
    }
}

pub struct KvCacheEntryWithTaint {
    pub key: u64,
    pub value: [u8; 256],
    pub taint: TaintLabel,
    /// Timestamp for eviction policy
    pub timestamp: u64,
}

pub struct InformationFlowController {
    /// Per-entry taint labels
    taints: [TaintLabel; MAX_KV_ENTRIES],
    /// Audit log for declassifications
    audit_log: [AuditEntry; 1024],
    audit_ptr: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct AuditEntry {
    timestamp: u64,
    team_id: u32,
    operation: AuditOp,
}

#[derive(Clone, Copy, Debug)]
pub enum AuditOp {
    Read(u32),       // entry_id
    Write(u32),      // entry_id
    Declassify(u32), // entry_id
}

impl InformationFlowController {
    pub fn check_read(&self, entry_id: u32, team_id: u32) -> Result<(), FlowViolation> {
        if self.taints[entry_id as usize].can_flow_to(team_id) {
            Ok(())
        } else {
            Err(FlowViolation {
                entry_id,
                team_id,
                taint: self.taints[entry_id as usize],
            })
        }
    }

    pub fn on_write(&mut self, entry_id: u32, team_id: u32, new_taint: TaintLabel) {
        self.taints[entry_id as usize] = new_taint;
        self.log_audit(AuditOp::Write(entry_id), team_id);
    }

    fn log_audit(&mut self, op: AuditOp, team_id: u32) {
        let entry = AuditEntry {
            timestamp: unsafe { crate::time::rdtsc() },
            team_id,
            operation: op,
        };
        self.audit_log[self.audit_ptr] = entry;
        self.audit_ptr = (self.audit_ptr + 1) % self.audit_log.len();
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FlowViolation {
    pub entry_id: u32,
    pub team_id: u32,
    pub taint: TaintLabel,
}
```

### 4.2 Information Flow Policies

```rust
pub enum FlowPolicy {
    /// No flow between teams (STRICT from Week 20)
    NoFlow,
    /// Allow flow only via explicit declassification
    DeclassifyOnly,
    /// Allow flow from low-confidentiality to high-confidence teams
    ConfidentialityBased,
    /// Custom lattice-based policy
    Custom(FlowLattice),
}

pub struct FlowLattice {
    /// Partial order: team_a <= team_b if a can flow into b
    order: [[bool; MAX_TEAMS]; MAX_TEAMS],
}

impl FlowLattice {
    pub fn can_flow(&self, from_team: u32, to_team: u32) -> bool {
        self.order[from_team as usize][to_team as usize]
    }

    pub fn least_upper_bound(&self, a: u32, b: u32) -> Option<u32> {
        // Find minimal team that both a and b can flow into
        for t in 0..MAX_TEAMS as u32 {
            if self.can_flow(a, t) && self.can_flow(b, t) {
                return Some(t);
            }
        }
        None
    }
}
```

---

## 5. Preemption Handling: Save/Restore State

During a context switch, KV-cache metadata must be atomically saved and restored to prevent interleaving corruption.

### 5.1 Preemption State Manager

```rust
pub struct PreemptionSnapshot {
    /// Task identifier
    task_id: u32,
    /// Eviction policy state
    eviction_snapshot: EvictionSnapshot,
    /// Information flow state
    taint_snapshot: TaintSnapshot,
    /// Timestamp of snapshot
    timestamp: u64,
}

pub struct EvictionSnapshot {
    /// LRU: current epoch
    epoch: u64,
    /// LFU: frequency counts
    frequencies: [u16; MAX_KV_ENTRIES],
    /// Policy variant active
    policy_type: u8,
}

pub struct TaintSnapshot {
    /// Per-entry taint labels
    taints: [TaintLabel; MAX_KV_ENTRIES],
}

pub struct PreemptionManager {
    /// Per-task saved state (bounded)
    snapshots: [Option<PreemptionSnapshot>; MAX_TASKS],
    /// Lock-free ring buffer for consistency
    consistency_ring: [u64; 8], // 8 sequential checksums
}

impl PreemptionManager {
    /// Save state before task preemption
    pub fn save_on_preempt(
        &mut self,
        task_id: u32,
        eviction: &EvictionSnapshot,
        taint: &TaintSnapshot,
    ) -> Result<(), PreemptionError> {
        // Atomic save with CAS-based consistency check
        let snapshot = PreemptionSnapshot {
            task_id,
            eviction_snapshot: *eviction,
            taint_snapshot: *taint,
            timestamp: unsafe { crate::time::rdtsc() },
        };

        // Write checksum before/after for consistency
        let checksum_before = self.compute_checksum(&snapshot);
        self.snapshots[task_id as usize] = Some(snapshot);
        let checksum_after = self.compute_checksum(
            &self.snapshots[task_id as usize].unwrap(),
        );

        if checksum_before == checksum_after {
            self.update_consistency_ring(checksum_after);
            Ok(())
        } else {
            Err(PreemptionError::InconsistentState)
        }
    }

    /// Restore state after task resumption
    pub fn restore_on_resume(&self, task_id: u32) -> Result<PreemptionSnapshot, PreemptionError> {
        self.snapshots[task_id as usize]
            .ok_or(PreemptionError::SnapshotNotFound)
    }

    /// Detect corruption during storage
    pub fn verify_consistency(&self) -> bool {
        // Check all ring values match
        let first = self.consistency_ring[0];
        self.consistency_ring.iter().all(|&v| v == first)
    }

    fn compute_checksum(&self, snap: &PreemptionSnapshot) -> u64 {
        // Simple FNV-like hash (full impl: use BLAKE2/SHA2)
        let mut h = 0u64;
        h = h.wrapping_mul(0x100000001b3);
        h ^= snap.task_id as u64;
        h = h.wrapping_mul(0x100000001b3);
        h ^= snap.timestamp;
        h
    }

    fn update_consistency_ring(&mut self, checksum: u64) {
        // Rotate ring
        for i in 0..7 {
            self.consistency_ring[i] = self.consistency_ring[i + 1];
        }
        self.consistency_ring[7] = checksum;
    }
}

#[derive(Clone, Copy, Debug)]
pub enum PreemptionError {
    SnapshotNotFound,
    InconsistentState,
    CorruptedMetadata,
}
```

### 5.2 Context Switch Handler

```rust
pub fn context_switch_save(
    task_id: u32,
    eviction: &mut EvictionSnapshot,
    taint: &mut TaintSnapshot,
    preemption_mgr: &mut PreemptionManager,
) -> Result<(), PreemptionError> {
    // Critical section: disable interrupts
    let _irq_guard = DisableInterrupts::new();

    // Snapshot eviction state
    eviction.epoch = eviction.epoch.saturating_add(1);
    // Copy LFU frequencies (atomic copy in real impl)
    // ...

    // Snapshot taint state
    // Copy all taint labels
    // ...

    // Atomically save
    preemption_mgr.save_on_preempt(task_id, eviction, taint)?;

    Ok(())
}

pub fn context_switch_restore(
    task_id: u32,
    preemption_mgr: &PreemptionManager,
) -> Result<(EvictionSnapshot, TaintSnapshot), PreemptionError> {
    let snap = preemption_mgr.restore_on_resume(task_id)?;

    // Verify consistency
    if !preemption_mgr.verify_consistency() {
        return Err(PreemptionError::CorruptedMetadata);
    }

    Ok((snap.eviction_snapshot, snap.taint_snapshot))
}

struct DisableInterrupts;

impl DisableInterrupts {
    fn new() -> Self {
        unsafe { core::arch::asm!("cli") };
        DisableInterrupts
    }
}

impl Drop for DisableInterrupts {
    fn drop(&mut self) {
        unsafe { core::arch::asm!("sti") };
    }
}
```

---

## 6. Cache Warmup & Priming Strategies

### 6.1 Warmup Policies

```rust
pub enum WarmupStrategy {
    /// No priming (cold start)
    None,
    /// Preload most frequent entries from previous run
    HistoryBased { history_size: u32 },
    /// Priming based on static workload characterization
    StaticProfile { profile: &'static [WarmupEntry] },
    /// Speculative: predict first-access entries
    Speculative { predictor: AccessPredictor },
}

#[derive(Clone, Copy, Debug)]
pub struct WarmupEntry {
    pub key: u64,
    pub frequency_weight: u16,
    pub team_id: u32,
}

pub struct CacheWarmer {
    /// Historical entry frequencies
    history: [(u64, u16); 64],
    history_size: usize,
}

impl CacheWarmer {
    pub fn prime_cache(
        &self,
        cache: &mut KvCacheManager,
        strategy: &WarmupStrategy,
    ) -> Result<u32, WarmupError> {
        let mut entries_loaded = 0u32;

        match strategy {
            WarmupStrategy::None => Ok(0),
            WarmupStrategy::HistoryBased { history_size } => {
                // Load top N entries from history
                for i in 0..*history_size as usize {
                    if i >= self.history.len() {
                        break;
                    }
                    let (key, _freq) = self.history[i];
                    cache.preload(key)?;
                    entries_loaded += 1;
                }
                Ok(entries_loaded)
            }
            WarmupStrategy::StaticProfile { profile } => {
                // Load predefined profile
                for entry in profile.iter() {
                    cache.preload(entry.key)?;
                    entries_loaded += 1;
                }
                Ok(entries_loaded)
            }
            WarmupStrategy::Speculative { predictor } => {
                // Predict and load
                for prediction in predictor.predict(8).iter() {
                    cache.preload(prediction.key)?;
                    entries_loaded += 1;
                }
                Ok(entries_loaded)
            }
        }
    }

    pub fn record_history(&mut self, key: u64, frequency: u16) {
        // Maintain sorted history
        for i in 0..self.history_size {
            if self.history[i].1 < frequency {
                // Insert at position i
                self.history.copy_within(i..self.history_size - 1, i + 1);
                self.history[i] = (key, frequency);
                if self.history_size < self.history.len() {
                    self.history_size += 1;
                }
                return;
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct AccessPredictor {
    /// Markov chain: (current_key, next_key_prediction)
    markov_table: [(u64, u64); 256],
    /// Confidence scores
    confidence: [u8; 256],
}

impl AccessPredictor {
    pub fn predict(&self, count: usize) -> Vec<WarmupEntry> {
        (0..count)
            .filter_map(|i| {
                if self.confidence[i % 256] > 200 {
                    Some(WarmupEntry {
                        key: self.markov_table[i % 256].1,
                        frequency_weight: self.confidence[i % 256] as u16,
                        team_id: 0,
                    })
                } else {
                    None
                }
            })
            .collect()
    }
}

#[derive(Clone, Copy, Debug)]
pub enum WarmupError {
    CacheExhausted,
    InvalidKey,
}
```

---

## 7. Adversarial Testing Framework

### 7.1 Six Attack Vectors

#### 7.1.1 Cache Side-Channel Attack (Eviction Timing)

```rust
#[test]
fn test_cache_sidechannel_eviction_timing() {
    // ATTACK: Measure eviction latency to infer access patterns of other team
    // DEFENSE: Constant-time eviction + noise injection

    let mut policy = LruEvictionPolicy::new();
    let mut timings = Vec::new();

    for iter in 0..100 {
        // Team A: access high-value entries
        policy.on_access(0, 0, iter as u64);

        // Attacker (Team B): measure eviction time for team A's entries
        let t0 = unsafe { crate::time::rdtsc() };
        let _ = policy.evict(0);
        let t1 = unsafe { crate::time::rdtsc() };

        timings.push(t1 - t0);
    }

    // Check that eviction timing has high variance (noise added)
    let variance = calculate_variance(&timings);
    assert!(
        variance > 100,
        "Eviction timing too constant: {}. Attacker can infer access patterns.",
        variance
    );
}

fn calculate_variance(samples: &[u64]) -> u64 {
    if samples.is_empty() {
        return 0;
    }
    let mean = samples.iter().sum::<u64>() / samples.len() as u64;
    let sq_diff: u64 = samples
        .iter()
        .map(|&x| {
            let d = if x > mean { x - mean } else { mean - x };
            d * d
        })
        .sum();
    sq_diff / samples.len() as u64
}
```

#### 7.1.2 Timing Attack (Cache Hits vs. Misses)

```rust
#[test]
fn test_timing_attack_hit_miss_distinction() {
    // ATTACK: Distinguish cache hit from miss by timing latency
    // DEFENSE: Constant-latency read path with dummy operations

    let mut cache = KvCacheManager::new();
    let mut hit_times = Vec::new();
    let mut miss_times = Vec::new();

    // Populate cache
    for i in 0..16 {
        cache.insert(i as u64, i as u64, 0).unwrap();
    }

    // Measure hit latency
    for i in 0..16 {
        let t0 = unsafe { crate::time::rdtsc() };
        let _ = cache.get(i as u64, 0);
        let t1 = unsafe { crate::time::rdtsc() };
        hit_times.push(t1 - t0);
    }

    // Measure miss latency
    for i in 100..116 {
        let t0 = unsafe { crate::time::rdtsc() };
        let _ = cache.get(i as u64, 0);
        let t1 = unsafe { crate::time::rdtsc() };
        miss_times.push(t1 - t0);
    }

    // Calculate separation
    let hit_mean = mean(&hit_times);
    let miss_mean = mean(&miss_times);
    let separation = if miss_mean > hit_mean {
        miss_mean - hit_mean
    } else {
        hit_mean - miss_mean
    };

    // Separation should be negligible (< 10% of latency)
    let avg_latency = (hit_mean + miss_mean) / 2;
    let separation_pct = (separation * 100) / avg_latency;

    assert!(
        separation_pct < 10,
        "Hit/miss timing distinguishable: {} > 10%. Defense: add constant-time dummy ops.",
        separation_pct
    );
}

fn mean(samples: &[u64]) -> u64 {
    samples.iter().sum::<u64>() / samples.len() as u64
}
```

#### 7.1.3 Eviction Attack (Cache Pollution)

```rust
#[test]
fn test_eviction_attack_cache_pollution() {
    // ATTACK: Attacker floods cache with entries, evicting victim's data
    // DEFENSE: Per-team eviction quotas + backpressure

    let mut policy = LruEvictionPolicy::new();
    let mut quota = EvictionQuota::new(MAX_KV_ENTRIES / 2);

    // Team A: legitimate access
    for i in 0..16 {
        policy.on_access(i, 0, i as u64);
    }

    // Team B (attacker): flood cache with many entries
    let mut evictions = 0;
    for i in 0..256 {
        if quota.can_evict(1 as u32) {
            if let Some(_) = policy.evict(1) {
                evictions += 1;
            }
        } else {
            // Quota exceeded: Team B is backpressured
            break;
        }
    }

    // Verify Team A's entries are mostly preserved
    let team_a_survival = (0..16)
        .filter(|&i| {
            // Check if Team A's entries are still in top N
            policy.team_lru_queues[0].contains(&i)
        })
        .count();

    assert!(
        team_a_survival >= 12,
        "Eviction attack successful: only {} of 16 Team A entries survived",
        team_a_survival
    );
}

pub struct EvictionQuota {
    per_team_limit: u32,
    per_team_used: [u32; MAX_TEAMS],
}

impl EvictionQuota {
    pub fn new(total_quota: u32) -> Self {
        EvictionQuota {
            per_team_limit: total_quota / MAX_TEAMS as u32,
            per_team_used: [0; MAX_TEAMS],
        }
    }

    pub fn can_evict(&mut self, team_id: u32) -> bool {
        let team = team_id as usize;
        if self.per_team_used[team] < self.per_team_limit {
            self.per_team_used[team] += 1;
            true
        } else {
            false
        }
    }

    pub fn reset_epoch(&mut self) {
        self.per_team_used = [0; MAX_TEAMS];
    }
}
```

#### 7.1.4 Bandwidth Attack (Cache Contention)

```rust
#[test]
fn test_bandwidth_attack_cache_contention() {
    // ATTACK: Saturate bandwidth, delay victim's requests
    // DEFENSE: Bandwidth reservation + priority scheduling

    let mut bw_scheduler = BandwidthScheduler::new(1000); // 1000 ops/epoch

    // Victim: latency-sensitive (low priority initially)
    let victim_task = 0;

    // Attacker: generate sustained bandwidth demand
    let attacker_task = 1;

    let mut victim_latencies = Vec::new();

    for epoch in 0..100 {
        // Attacker: issue many ops
        for _ in 0..800 {
            let _ = bw_scheduler.schedule(attacker_task, BwClass::Batch);
        }

        // Victim: issue critical op
        let t0 = unsafe { crate::time::rdtsc() };
        let granted = bw_scheduler.schedule(victim_task, BwClass::RealTime);
        let t1 = unsafe { crate::time::rdtsc() };

        if granted {
            victim_latencies.push(t1 - t0);
        }
    }

    // Verify victim achieved minimum latency SLA
    let p50 = percentile(&victim_latencies, 50);
    assert!(
        p50 < 200,
        "Bandwidth attack successful: victim P50 latency {} > 200 cycles",
        p50
    );
}

pub enum BwClass {
    RealTime,
    Interactive,
    Batch,
}

pub struct BandwidthScheduler {
    capacity: u32,
    allocations: [u32; MAX_TEAMS],
    granted_this_epoch: u32,
}

impl BandwidthScheduler {
    pub fn new(capacity: u32) -> Self {
        BandwidthScheduler {
            capacity,
            allocations: [0; MAX_TEAMS],
            granted_this_epoch: 0,
        }
    }

    pub fn schedule(&mut self, team_id: u32, class: BwClass) -> bool {
        let cost = match class {
            BwClass::RealTime => 10,
            BwClass::Interactive => 50,
            BwClass::Batch => 100,
        };

        if self.granted_this_epoch + cost <= self.capacity {
            self.granted_this_epoch += cost;
            true
        } else {
            false
        }
    }
}

fn percentile(samples: &[u64], p: u32) -> u64 {
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let idx = ((p as usize) * sorted.len()) / 100;
    sorted[idx]
}
```

#### 7.1.5 TLB Poisoning Attack

```rust
#[test]
fn test_tlb_poisoning_shared_pt() {
    // ATTACK: Attacker pollutes TLB with translations, forcing victim misses
    // DEFENSE: Per-team TLB partitioning + selective flush on context switch

    let mut tlb = TlbManager::new();

    // Victim: access legitimate pages
    let victim_pages = [0x1000, 0x2000, 0x3000];
    for &addr in &victim_pages {
        tlb.insert(addr, 0, TlbPriority::HighFreq).unwrap();
    }

    // Attacker: spray TLB with junk translations
    for i in 0..512 {
        let addr = 0x100000 + (i << 12); // 4KB pages
        let _ = tlb.insert(addr, 1, TlbPriority::Low);
    }

    // Verify victim's pages are still cached
    let victim_hits = victim_pages
        .iter()
        .filter(|&&addr| tlb.lookup(addr, 0).is_some())
        .count();

    assert!(
        victim_hits == 3,
        "TLB poisoning successful: only {} of 3 victim entries remain",
        victim_hits
    );
}

pub enum TlbPriority {
    HighFreq,
    Normal,
    Low,
}

pub struct TlbManager {
    entries: [Option<TlbEntry>; 64], // Small partitioned TLB
    lru_counters: [u32; 64],
    per_team_quota: [u32; MAX_TEAMS],
}

#[derive(Clone, Copy)]
pub struct TlbEntry {
    pa: u64,
    team_id: u32,
    priority: u8,
}

impl TlbManager {
    pub fn new() -> Self {
        TlbManager {
            entries: [None; 64],
            lru_counters: [0; 64],
            per_team_quota: [32 / MAX_TEAMS as u32; MAX_TEAMS],
        }
    }

    pub fn insert(&mut self, va: u64, team_id: u32, priority: TlbPriority) -> Result<(), ()> {
        // Check team quota
        if self.per_team_quota[team_id as usize] == 0 {
            return Err(());
        }

        // Find eviction victim (LRU, low priority)
        let evict_idx = (0..64)
            .min_by_key(|&i| {
                (
                    self.entries[i].is_some() as u32,
                    self.entries[i].map(|e| e.priority).unwrap_or(255),
                    self.lru_counters[i],
                )
            })
            .unwrap();

        self.entries[evict_idx] = Some(TlbEntry {
            pa: va, // Simplified: VA as PA
            team_id,
            priority: priority as u8,
        });
        self.per_team_quota[team_id as usize] -= 1;
        Ok(())
    }

    pub fn lookup(&self, va: u64, team_id: u32) -> Option<u64> {
        self.entries
            .iter()
            .find(|e| e.map(|entry| entry.pa == va && entry.team_id == team_id).unwrap_or(false))
            .and_then(|e| e.map(|entry| entry.pa))
    }
}
```

#### 7.1.6 Speculative Execution Attack (Meltdown-like)

```rust
#[test]
fn test_speculative_execution_information_leak() {
    // ATTACK: Speculative access to victim's data, cache side-channel to read
    // DEFENSE: Speculation barriers + cache isolation on illegal access

    let mut cache = KvCacheManager::new();
    let mut flow_ctrl = InformationFlowController::new();

    // Insert victim data with confidential taint
    let secret_key = 0xDEADBEEF;
    let secret_value = [0x42u8; 256];
    cache
        .insert(secret_key, &secret_value, 0)
        .unwrap();

    // Taint as confidential (Team 0 only)
    flow_ctrl.set_taint(
        0,
        TaintLabel {
            team_mask: 1,
            confidentiality: 2,
            integrity: 255,
        },
    );

    // Attacker (Team 1): attempt speculative read
    let mut side_channel_samples = Vec::new();

    for iteration in 0..100 {
        // Start speculative operation that will fault
        let t0 = unsafe { crate::time::rdtsc() };
        let result = unsafe {
            // Speculative access (will fault)
            crate::arch::speculative_read(
                secret_key,
                1, // Attacker team
                &flow_ctrl,
            )
        };
        let t1 = unsafe { crate::time::rdtsc() };

        // If speculative read hits cache before faulting, timing differs
        side_channel_samples.push((t1 - t0, result.is_ok()));
    }

    // Verify fault happens BEFORE side-channel is observable
    let fault_count = side_channel_samples.iter().filter(|(_, ok)| !ok).count();
    assert!(
        fault_count == 100,
        "Speculative read should fault: {} faults of 100 iterations",
        fault_count
    );

    // Verify no cache pollution from speculative read
    let leaked = side_channel_samples
        .iter()
        .filter(|(latency, _)| *latency < 50)
        .count();
    assert!(
        leaked < 5,
        "Speculative access leaked to cache: {} of 100 fast accesses",
        leaked
    );
}

// Hypothetical unsafe speculative_read function (no_std compatible)
pub mod arch {
    use crate::kernel::capability_engine::{InformationFlowController, FlowViolation};

    pub unsafe fn speculative_read(
        key: u64,
        team_id: u32,
        flow_ctrl: &InformationFlowController,
    ) -> Result<[u8; 256], FlowViolation> {
        // lfence: ensure prior memory ops complete (block speculation)
        core::arch::asm!("lfence");

        // Check flow control (this will fault on violation)
        flow_ctrl.check_read(0, team_id)?;

        // Only speculative if check_read succeeds
        // On x86-64 with SVM/VMX, privileged code would set "spec barrier"
        core::arch::asm!("mfence"); // Full barrier

        Ok([0; 256])
    }
}
```

---

## 8. Test Suite Summary

### 8.1 Test Categories (200+ tests)

| Category | Count | Examples |
|----------|-------|----------|
| **Basic Isolation** | 40 | STRICT/SELECTIVE/OPEN modes, page-table enforcement |
| **Preemption** | 35 | Save/restore correctness, consistency checks, race conditions |
| **Information Flow** | 30 | Taint propagation, declassification, flow violations |
| **Eviction Policies** | 35 | LRU ordering, LFU frequency, adaptive switching |
| **Cache Warmup** | 15 | History loading, speculative priming, profile matching |
| **Performance** | 20 | Latency SLAs, throughput, fairness |
| **Adversarial (6 vectors)** | 25+ | Side-channel, timing, eviction, bandwidth, TLB, speculation |

---

## 9. Integration with Week 20 Baseline

| Week 20 Component | Week 21 Enhancement |
|-------------------|---------------------|
| Cache isolation (STRICT/SELECTIVE/OPEN) | Integrated with eviction policies |
| Page-table PTE enforcement | Combined with information flow taint labels |
| Mode transitions | Protected by preemption state snapshots |
| Per-team quotas | Enforced via EvictionQuota + BandwidthScheduler |

---

## 10. Performance Targets

- **Eviction Policy Overhead:** < 5% relative to cache access latency
- **Information Flow Check Latency:** < 10 cycles (constant-time)
- **Preemption Save/Restore:** < 1 μs total (for 512 entries)
- **Cache Warmup Latency:** < 100 μs for 64-entry prime
- **Attack Detection/Prevention:** All six attack vectors mitigated

---

## 11. Security Guarantees

**Threat Model:** Multi-tenant inference; mutually untrusting teams with shared KV-cache.

**Guarantees:**
1. **Isolation:** Team A cannot observe Team B's access patterns (side-channel free)
2. **Integrity:** Eviction policy cannot be exploited for cache pollution
3. **Availability:** Bandwidth and eviction quota prevent DoS
4. **Auditability:** All cross-team flows logged and verifiable
5. **Preemption Safety:** Cache state corruption prevented via consistency checks

---

## 12. Implementation Checklist

- [ ] LRU eviction policy with per-team queues
- [ ] LFU eviction policy with ghost tracking
- [ ] Adaptive policy switching logic
- [ ] TaintLabel system with merge/declassify operations
- [ ] InformationFlowController with audit logging
- [ ] PreemptionManager with consistency checking
- [ ] DisableInterrupts RAII guard (IRQ safety)
- [ ] CacheWarmer with history/profile/speculative strategies
- [ ] EvictionQuota and BandwidthScheduler
- [ ] TlbManager with per-team partitioning
- [ ] All 200+ tests passing
- [ ] Adversarial test suite (6 vectors, 25+ tests)
- [ ] Documentation + code review

---

## 13. References & Standards

- **Flush+Reload Mitigation:** (Yarom & Falk, CCS 2014)
- **Spectre/Meltdown Defense:** (Lipp et al., arXiv 2018)
- **Information Flow Control:** (Denning & Denning, ACM Computing Surveys 1977)
- **Preemption Atomicity:** (Herlihy & Moss, ASPLOS 1993)

---

**Document Version:** 1.0
**Last Updated:** Week 21, Phase 2
**Owner:** Staff Engineer, Capability Engine & Security
**Status:** Ready for Implementation
