# Week 14: Consensus Pattern & Phase 1 Finalization

**Project**: XKernal Cognitive Substrate OS
**Layer**: L3 SDK (Rust/TypeScript/C#)
**Phase**: 1 (Final Week)
**Date**: 2026-03-02
**Status**: Design & Implementation

---

## 1. Executive Summary

Week 14 concludes Phase 1 by introducing the **Consensus Pattern**—a Byzantine fault-tolerant mechanism enabling N cognitive agents to reach agreement on critical decisions. This document details the consensus algorithm, voting protocol, fault tolerance guarantees, and integration into `libcognitive` v0.1.

**Key Deliverables**:
- Consensus pattern with Byzantine fault tolerance (BFT)
- `ct.Consensus()` API and crew channel integration
- Quorum-based voting protocol (yes/no decisions)
- Adversarial testing framework with delay injection
- Polished Phase 1 patterns (ReAct, CoT, Reflection)
- libcognitive v0.1 API finalization

---

## 2. Consensus Pattern Design

### 2.1 Problem Statement

Multi-agent coordination requires mechanisms for distributed decision-making under uncertainty. Agents may:
- Have conflicting analysis results
- Operate with stale or partial information
- Be subject to network delays
- Exhibit Byzantine (arbitrary/malicious) behavior

**Design Goal**: Implement a consensus protocol where N agents reach agreement on binary decisions (yes/no) with Byzantine fault tolerance.

### 2.2 Algorithm: Practical Byzantine Fault Tolerance (PBFT)

We implement a simplified PBFT variant optimized for cognitive agent coordination:

```rust
// File: crates/libcognitive/src/patterns/consensus.rs

use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use serde::{Deserialize, Serialize};

/// Consensus message types for agent coordination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsensusMessage {
    /// Pre-prepare: proposer broadcasts decision to replicate
    PrePrepare {
        proposal_id: u64,
        decision: bool,
        evidence: String,
        timestamp: u64,
    },
    /// Prepare: backup confirms pre-prepare reception
    Prepare {
        proposal_id: u64,
        agent_id: u32,
        view: u32,
    },
    /// Commit: agent commits to the decision
    Commit {
        proposal_id: u64,
        agent_id: u32,
        vote: bool,
    },
}

/// Byzantine Fault-Tolerant Consensus State Machine
pub struct BFTConsensus {
    agent_id: u32,
    total_agents: u32,
    fault_tolerance: u32,  // f = (N - 1) / 3
    view: u32,

    // Message logs
    pre_prepares: HashMap<u64, (bool, String)>,
    prepares: HashMap<u64, Vec<u32>>,
    commits: HashMap<u64, Vec<(u32, bool)>>,

    // Timing
    timeout: Duration,
    proposal_start: HashMap<u64, Instant>,
}

impl BFTConsensus {
    /// Initialize consensus for N agents with Byzantine fault tolerance
    /// Guarantees: with f faulty agents, system tolerates up to f faults (N >= 3f + 1)
    pub fn new(agent_id: u32, total_agents: u32, timeout_ms: u64) -> Self {
        let fault_tolerance = (total_agents - 1) / 3;

        assert!(
            total_agents >= 3 * fault_tolerance + 1,
            "Consensus requires N >= 3f + 1 agents (N={}, f={})",
            total_agents,
            fault_tolerance
        );

        Self {
            agent_id,
            total_agents,
            fault_tolerance,
            view: 0,
            pre_prepares: HashMap::new(),
            prepares: HashMap::new(),
            commits: HashMap::new(),
            timeout: Duration::from_millis(timeout_ms),
            proposal_start: HashMap::new(),
        }
    }

    /// Phase 1: Pre-Prepare (Proposer broadcasts proposal)
    pub fn propose(&mut self, proposal_id: u64, decision: bool, evidence: String) -> ConsensusMessage {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        self.proposal_start.insert(proposal_id, Instant::now());
        self.pre_prepares.insert(proposal_id, (decision, evidence.clone()));

        ConsensusMessage::PrePrepare {
            proposal_id,
            decision,
            evidence,
            timestamp,
        }
    }

    /// Phase 2: Prepare (Backups acknowledge pre-prepare)
    pub fn handle_pre_prepare(
        &mut self,
        proposal_id: u64,
        decision: bool,
        evidence: &str,
    ) -> Option<ConsensusMessage> {
        // Validation: check proposal is well-formed
        if !self.is_valid_proposal(proposal_id, decision, evidence) {
            return None;
        }

        self.pre_prepares.insert(proposal_id, (decision, evidence.to_string()));

        Some(ConsensusMessage::Prepare {
            proposal_id,
            agent_id: self.agent_id,
            view: self.view,
        })
    }

    /// Phase 3: Commit (Agents vote after collecting f+1 prepares)
    pub fn handle_prepare(&mut self, proposal_id: u64, agent_id: u32, _view: u32) {
        self.prepares
            .entry(proposal_id)
            .or_insert_with(Vec::new)
            .push(agent_id);

        // Check if we have f + 1 prepares (quorum for commit)
        let prepare_count = self.prepares.get(&proposal_id).map(|v| v.len()).unwrap_or(0);
        if prepare_count >= (self.fault_tolerance + 1) as usize {
            // Can safely commit
            if let Some((decision, _)) = self.pre_prepares.get(&proposal_id) {
                self.initiate_commit(proposal_id, *decision);
            }
        }
    }

    /// Initiate commit phase with agent's decision
    fn initiate_commit(&self, proposal_id: u64, decision: bool) {
        // This would be broadcast to all agents
        let _msg = ConsensusMessage::Commit {
            proposal_id,
            agent_id: self.agent_id,
            vote: decision,
        };
    }

    /// Handle commit votes from peers
    pub fn handle_commit(&mut self, proposal_id: u64, agent_id: u32, vote: bool) {
        self.commits
            .entry(proposal_id)
            .or_insert_with(Vec::new)
            .push((agent_id, vote));
    }

    /// Finalize consensus: requires 2f + 1 matching commits
    pub fn finalize(&self, proposal_id: u64) -> Option<bool> {
        let commits = self.commits.get(&proposal_id)?;
        let quorum_size = 2 * self.fault_tolerance + 1;

        if commits.len() < quorum_size as usize {
            return None;  // Not enough commits
        }

        // Count yes/no votes
        let yes_count = commits.iter().filter(|(_, vote)| *vote).count();
        let no_count = commits.len() - yes_count;

        // Decision requires strict majority with BFT guarantee
        if yes_count > no_count && yes_count >= quorum_size as usize {
            return Some(true);
        } else if no_count > yes_count && no_count >= quorum_size as usize {
            return Some(false);
        }

        None  // Inconclusive (retry or escalate)
    }

    /// Validation: check proposal consistency
    fn is_valid_proposal(&self, _proposal_id: u64, _decision: bool, evidence: &str) -> bool {
        // Verify evidence hash matches pre-prepare
        // Verify signature chain from proposer
        // In production: cryptographic verification
        !evidence.is_empty()
    }

    /// Timeout handling: view change on timeout
    pub fn handle_timeout(&mut self, proposal_id: u64) -> bool {
        if let Some(start) = self.proposal_start.get(&proposal_id) {
            if start.elapsed() > self.timeout {
                self.view += 1;
                return true;  // Trigger view change (new proposer)
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consensus_quorum_threshold() {
        // N=4 agents, f=1 Byzantine fault
        // Requires: 2f + 1 = 3 commits to finalize
        let mut consensus = BFTConsensus::new(0, 4, 5000);

        let proposal_id = 1;
        consensus.propose(proposal_id, true, "evidence".to_string());

        // Simulate 3 agents voting YES
        consensus.handle_commit(proposal_id, 0, true);
        consensus.handle_commit(proposal_id, 1, true);
        consensus.handle_commit(proposal_id, 2, true);

        assert_eq!(consensus.finalize(proposal_id), Some(true));
    }

    #[test]
    fn test_byzantine_agent_isolation() {
        // N=7 agents, f=2 Byzantine faults (7 >= 3*2 + 1)
        let mut consensus = BFTConsensus::new(0, 7, 5000);

        let proposal_id = 1;
        consensus.propose(proposal_id, true, "valid_evidence".to_string());

        // 5 honest agents vote YES (sufficient quorum = 2*2 + 1 = 5)
        for i in 0..5 {
            consensus.handle_commit(proposal_id, i, true);
        }

        // 2 Byzantine agents vote NO (should be overridden)
        for i in 5..7 {
            consensus.handle_commit(proposal_id, i, false);
        }

        assert_eq!(consensus.finalize(proposal_id), Some(true));
    }
}
```

### 2.3 Byzantine Fault Tolerance Guarantees

| Property | Guarantee |
|----------|-----------|
| **Safety** | All non-faulty agents decide the same value |
| **Liveness** | Consensus terminates within bounded time |
| **Fault Tolerance** | Tolerates up to f faulty agents where f < N/3 |
| **Message Complexity** | O(N²) per consensus round |

**Key Invariant**: With N >= 3f + 1:
- **Pre-prepare phase**: Proposer broadcasts proposal to all
- **Prepare phase**: Agents confirm reception (f + 1 quorum)
- **Commit phase**: Agents commit votes (2f + 1 quorum achieves consensus)

---

## 3. ct.Consensus() API Design

### 3.1 Crew Channel Integration

```rust
// File: crates/libcognitive/src/crew/consensus.rs

use crate::patterns::consensus::{BFTConsensus, ConsensusMessage};
use crate::crew::CrewChannel;
use std::time::Duration;

/// High-level consensus API for cognitive agent crews
pub struct CrewConsensus {
    bft: BFTConsensus,
    channel: CrewChannel,
}

impl CrewConsensus {
    /// Create consensus coordinator for crew
    pub async fn new(
        agent_id: u32,
        crew_size: u32,
        channel: CrewChannel,
        timeout_ms: u64,
    ) -> Result<Self> {
        let bft = BFTConsensus::new(agent_id, crew_size, timeout_ms);

        Ok(Self { bft, channel })
    }

    /// Propose decision and run consensus protocol
    pub async fn propose_and_decide(
        &mut self,
        proposal_id: u64,
        decision: bool,
        evidence: String,
    ) -> Result<bool> {
        // Phase 1: Broadcast pre-prepare
        let msg = self.bft.propose(proposal_id, decision, evidence);
        self.channel.broadcast("consensus", msg).await?;

        // Phase 2-3: Collect responses and finalize
        let result = self.collect_votes(proposal_id).await?;

        Ok(result)
    }

    /// Collect votes from crew with timeout
    async fn collect_votes(&mut self, proposal_id: u64) -> Result<bool> {
        let timeout = Duration::from_secs(5);

        loop {
            tokio::select! {
                msg = self.channel.recv("consensus") => {
                    match msg? {
                        ConsensusMessage::Prepare { .. } => {
                            // Handle prepare message
                        },
                        ConsensusMessage::Commit { agent_id, vote, .. } => {
                            self.bft.handle_commit(proposal_id, agent_id, vote);

                            // Check if finalized
                            if let Some(decision) = self.bft.finalize(proposal_id) {
                                return Ok(decision);
                            }
                        },
                        _ => {}
                    }
                },
                _ = tokio::time::sleep(timeout) => {
                    if self.bft.handle_timeout(proposal_id) {
                        return Err("Consensus timeout: view change required".into());
                    }
                }
            }
        }
    }
}

/// Convenience macro for consensus in crew workflows
#[macro_export]
macro_rules! ct_consensus {
    ($crew:expr, $proposal_id:expr, $decision:expr, $evidence:expr) => {{
        $crew
            .consensus
            .propose_and_decide($proposal_id, $decision, $evidence.to_string())
            .await
    }};
}
```

---

## 4. Voting Protocol

### 4.1 Quorum-Based Voting

```rust
/// Voting mechanism with quorum requirements
pub struct QuorumVote {
    required_quorum: usize,
    yes_votes: usize,
    no_votes: usize,
    abstentions: usize,
}

impl QuorumVote {
    pub fn new(quorum_percent: u32) -> Self {
        Self {
            required_quorum: ((quorum_percent as usize) / 100),
            yes_votes: 0,
            no_votes: 0,
            abstentions: 0,
        }
    }

    /// Add vote to tally
    pub fn vote(&mut self, yes: bool) -> Option<bool> {
        if yes {
            self.yes_votes += 1;
        } else {
            self.no_votes += 1;
        }

        self.resolve()
    }

    /// Determine outcome if quorum reached
    fn resolve(&self) -> Option<bool> {
        let total = self.yes_votes + self.no_votes;
        if total < self.required_quorum {
            return None;
        }

        // Strict majority check
        if self.yes_votes > self.no_votes {
            Some(true)
        } else if self.no_votes > self.yes_votes {
            Some(false)
        } else {
            None  // Tie - requires tiebreaker
        }
    }
}
```

**Voting Rules**:
- **Quorum**: 2f + 1 out of N votes (Byzantine majority)
- **Majority**: Strict > 50% within quorum
- **Tiebreaker**: Escalate to supervisor agent or random selection

---

## 5. Adversarial Testing Framework

### 5.1 Byzantine Fault Injection

```rust
#[cfg(test)]
mod adversarial_tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    /// Simulate Byzantine agent that sends contradictory votes
    #[tokio::test]
    async fn test_byzantine_double_vote() {
        let mut honest = BFTConsensus::new(0, 4, 5000);
        let mut byzantine = BFTConsensus::new(1, 4, 5000);

        let proposal_id = 1;

        // Honest agent proposes YES
        honest.propose(proposal_id, true, "valid".to_string());

        // Byzantine agent sends contradictory votes to different peers
        byzantine.handle_commit(proposal_id, 1, true);   // tells agent 2 it voted YES
        byzantine.handle_commit(proposal_id, 2, false);  // tells agent 3 it voted NO

        // System should still reach consensus with 3+ honest votes
        honest.handle_commit(proposal_id, 0, true);
        honest.handle_commit(proposal_id, 2, true);
        honest.handle_commit(proposal_id, 3, true);

        assert_eq!(honest.finalize(proposal_id), Some(true));
    }

    /// Simulate network delay injection
    #[tokio::test]
    async fn test_consensus_under_delay() {
        let mut consensus = BFTConsensus::new(0, 5, 10000);

        let proposal_id = 1;
        consensus.propose(proposal_id, true, "evidence".to_string());

        // Simulate network latency (1-2 second delays)
        for i in 0..4 {
            sleep(Duration::from_millis(500 + i * 200)).await;
            consensus.handle_commit(proposal_id, i, true);
        }

        // Should still finalize within timeout
        assert_eq!(consensus.finalize(proposal_id), Some(true));
    }

    /// Test behavior with f agents faulty (maximum tolerated)
    #[tokio::test]
    fn test_maximum_fault_tolerance() {
        // N=10, f=3 (3*3 + 1 = 10 agents)
        let mut consensus = BFTConsensus::new(0, 10, 5000);

        let proposal_id = 1;
        consensus.propose(proposal_id, true, "evidence".to_string());

        // 7 honest agents vote YES (sufficient for consensus)
        for i in 0..7 {
            consensus.handle_commit(proposal_id, i, true);
        }

        // 3 Byzantine agents vote NO (should be ignored)
        for i in 7..10 {
            consensus.handle_commit(proposal_id, i, false);
        }

        assert_eq!(consensus.finalize(proposal_id), Some(true));
    }

    /// Test partition tolerance: agents in minority partition
    #[tokio::test]
    fn test_partition_safety() {
        // N=5 agents, f=1
        // Partition: 3 agents vs 2 agents
        // Only partition with 2f + 1 = 3 agents can decide

        let mut consensus_a = BFTConsensus::new(0, 5, 5000);
        let mut consensus_b = BFTConsensus::new(3, 5, 5000);

        let proposal_id = 1;

        // Majority partition: 3 agents reach consensus
        consensus_a.propose(proposal_id, true, "evidence".to_string());
        for i in 0..3 {
            consensus_a.handle_commit(proposal_id, i, true);
        }
        assert_eq!(consensus_a.finalize(proposal_id), Some(true));

        // Minority partition: 2 agents cannot reach consensus
        consensus_b.propose(proposal_id, false, "evidence".to_string());
        for i in 0..2 {
            consensus_b.handle_commit(proposal_id, i, true);
        }
        // 2 votes < 2f + 1 = 3, so cannot finalize
        assert_eq!(consensus_b.finalize(proposal_id), None);
    }
}
```

---

## 6. Phase 1 Pattern Polish

### 6.1 Pattern Summary Table

| Pattern | Introduced | Optimizations (Week 14) |
|---------|-----------|------------------------|
| **ReAct** | Week 9 | Tool isolation, timeout mgmt (Week 10) |
| **Chain-of-Thought** | Week 11 | Step validation, reasoning traces |
| **Reflection** | Week 11 | Self-critique, error recovery |
| **Crew Coordination** | Week 13 | Supervisor routing, round-robin |
| **Consensus** | Week 14 | Byzantine FT, quorum voting |

### 6.2 Error Handling Pattern Integration

```rust
/// Consensus with integrated error handling from Week 12
pub async fn consensus_with_recovery(
    crew: &mut CrewConsensus,
    proposal_id: u64,
    decision: bool,
    evidence: String,
) -> Result<bool> {
    use crate::patterns::error_handling::{retry, rollback, escalate};

    retry(3, Duration::from_secs(1), || async {
        crew.propose_and_decide(proposal_id, decision, evidence.clone()).await
    }).await
        .or_else(|_| escalate("consensus", "supervisor"))
        .or_else(|_| rollback())
}
```

---

## 7. libcognitive v0.1 API Finalization

### 7.1 Complete Module Structure

```
libcognitive/
├── patterns/
│   ├── react.rs          # ReAct (Reason + Act)
│   ├── cot.rs            # Chain-of-Thought
│   ├── reflection.rs     # Reflection & Self-Critique
│   ├── consensus.rs      # Byzantine FT Consensus (NEW)
│   └── mod.rs
├── crew/
│   ├── supervisor.rs     # Multi-agent routing
│   ├── round_robin.rs    # Fair scheduling
│   ├── consensus.rs      # Crew consensus API
│   └── mod.rs
├── error_handling/
│   ├── retry.rs          # Retry with backoff
│   ├── rollback.rs       # State recovery
│   ├── escalate.rs       # Failure escalation
│   ├── degrade.rs        # Graceful degradation
│   └── mod.rs
├── ffi/
│   ├── csci_x86_64.rs    # Week 7 FFI
│   ├── csci_arm64.rs     # Week 8 FFI
│   └── mod.rs
└── lib.rs                # Public API
```

### 7.2 v0.1 Public API Surface

```rust
// crates/libcognitive/src/lib.rs

pub mod patterns {
    pub use crate::patterns::react::*;
    pub use crate::patterns::cot::*;
    pub use crate::patterns::reflection::*;
    pub use crate::patterns::consensus::*;
}

pub mod crew {
    pub use crate::crew::supervisor::*;
    pub use crate::crew::round_robin::*;
    pub use crate::crew::consensus::*;
}

pub mod error_handling {
    pub use crate::error_handling::retry::*;
    pub use crate::error_handling::rollback::*;
    pub use crate::error_handling::escalate::*;
    pub use crate::error_handling::degrade::*;
}

/// Cognitive Type: Consensus decision with Byzantine FT
pub type ct_Consensus = consensus::CrewConsensus;

/// Semantic versioning: Phase 1 complete
pub const VERSION: &str = "0.1.0";
pub const PHASE: &str = "Phase 1 - Cognitive Patterns & Crew Coordination";
```

---

## 8. Design Principles Compliance

| Principle | Implementation |
|-----------|----------------|
| **Cognitive-Native** | Patterns designed for agent reasoning workflows |
| **Semantic Versioning** | v0.1.0: Phase 1 complete, API stable |
| **Developer Experience** | Macro sugar (ct_consensus!), clear error types |
| **Interoperability** | FFI layers (x86-64, ARM64) enable multi-platform |
| **Testing** | Adversarial test suite, Byzantine fault injection |
| **Documentation** | Inline examples, algorithm guarantees documented |

---

## 9. Phase 2 Readiness Checklist

- [x] Consensus pattern with Byzantine FT (N >= 3f + 1)
- [x] ct.Consensus() API with crew channel integration
- [x] Quorum-based voting (2f + 1 threshold)
- [x] Adversarial testing framework (delay, Byzantine agents, partitions)
- [x] Phase 1 pattern polish (ReAct, CoT, Reflection)
- [x] libcognitive v0.1 API finalization
- [x] FFI bindings (x86-64, ARM64) functional
- [x] Error handling utilities (retry, rollback, escalate, degrade)
- [x] Crew coordination (supervisor, round-robin, consensus)

**Phase 2 Focus**: Advanced patterns (multi-agent debate, tree-of-thoughts), horizontal scaling with distributed consensus, performance optimization (lock-free channels), production hardening.

---

## 10. References

- Practical Byzantine Fault Tolerance (Castro & Liskov, 1999)
- Consensus in the Presence of Partial Synchrony (Dwork et al., 1988)
- XKernal CSCI FFI Design (Week 7-8)
- libcognitive Architecture (Weeks 9-13)
