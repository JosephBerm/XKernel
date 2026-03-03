// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! # CT Dependency DAG with Cycle Detection
//!
//! This module implements a directed acyclic graph (DAG) for CT dependencies
//! with O(V + E) cycle detection. Cycles are detected at spawn time and rejected
//! with clear error messages.
//!
//! ## Design
//!
//! - **Cycle Detection**: O(V + E) complexity via DFS
//! - **Spawn-Time Validation**: Dependencies validated before CT enters system
//! - **Clear Error Messages**: Cycle detection returns all CTIDs in the cycle
//! - **Dependency Wait Lists**: Track which CTs block a CT from Reason phase
//! - **Notification Mechanism**: When dependency completes, notify dependents
//!
//! ## Invariants
//!
//! - **Invariant 3**: All dependencies must complete before CT enters Reason phase
//! - **Invariant 5**: DAG must be cycle-checked at spawn time; circular deps rejected

use crate::error::{CsError, Result};
use crate::ids::CTID;

use alloc::string::ToString;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::format;
use alloc::vec;
use alloc::vec::Vec;

/// Directed acyclic graph tracking CT dependencies.
#[derive(Debug)]
pub struct DependencyDag {
    /// Adjacency list: CT -> set of CTs it depends on
    dependencies: BTreeMap<CTID, BTreeSet<CTID>>,
    /// Reverse adjacency: CT -> set of CTs that depend on it
    dependents: BTreeMap<CTID, BTreeSet<CTID>>,
    /// Set of CTs whose dependencies have been satisfied (completed)
    completed: BTreeSet<CTID>,
    /// Pending (unsatisfied) dependencies per CT
    pending: BTreeMap<CTID, BTreeSet<CTID>>,
    /// Total edge count
    edge_count: usize,
}

impl DependencyDag {
    /// Create a new empty DAG.
    pub fn new() -> Self {
        Self {
            dependencies: BTreeMap::new(),
            dependents: BTreeMap::new(),
            completed: BTreeSet::new(),
            pending: BTreeMap::new(),
            edge_count: 0,
        }
    }

    /// Add a CT to the DAG.
    pub fn add_ct(&mut self, ct: CTID) -> Result<()> {
        if self.dependencies.contains_key(&ct) {
            return Err(CsError::DuplicateCt {
                ct_id: ct.to_string(),
            });
        }
        self.dependencies.insert(ct, BTreeSet::new());
        self.dependents.insert(ct, BTreeSet::new());
        self.pending.insert(ct, BTreeSet::new());
        Ok(())
    }

    /// Add dependencies for a CT. Detects cycles.
    pub fn add_dependencies(&mut self, ct: CTID, deps: BTreeSet<CTID>) -> Result<()> {
        if !self.dependencies.contains_key(&ct) {
            return Err(CsError::CtNotFound {
                ct_id: ct.to_string(),
            });
        }

        // Check for self-loop
        if deps.contains(&ct) {
            return Err(CsError::CyclicDependency {
                ct_id: ct.to_string(),
                cycle: format!("CT {} depends on itself", ct),
            });
        }

        // Temporarily add edges, then check for cycles
        let mut new_edges = Vec::new();
        for &dep in &deps {
            if !self.dependencies.contains_key(&dep) {
                return Err(CsError::CtNotFound {
                    ct_id: dep.to_string(),
                });
            }
            // Add the edge: ct depends on dep
            self.dependencies.get_mut(&ct).unwrap().insert(dep);
            self.dependents.get_mut(&dep).unwrap().insert(ct);
            new_edges.push(dep);
        }

        // Check for cycles using DFS
        if self.has_cycle() {
            // Rollback
            for &dep in &new_edges {
                self.dependencies.get_mut(&ct).unwrap().remove(&dep);
                self.dependents.get_mut(&dep).unwrap().remove(&ct);
            }
            return Err(CsError::CyclicDependency {
                ct_id: ct.to_string(),
                cycle: format!("CT {} would create a cycle", ct),
            });
        }

        // Update edge count and pending
        self.edge_count += new_edges.len();
        for &dep in &new_edges {
            if !self.completed.contains(&dep) {
                self.pending.get_mut(&ct).unwrap().insert(dep);
            }
        }

        Ok(())
    }

    /// Check if the graph has any cycles using DFS.
    fn has_cycle(&self) -> bool {
        // 0 = unvisited, 1 = in-progress, 2 = done
        let mut state: BTreeMap<CTID, u8> = BTreeMap::new();
        for &ct in self.dependencies.keys() {
            state.insert(ct, 0);
        }

        for &ct in self.dependencies.keys() {
            if state[&ct] == 0 {
                if self.dfs_cycle_check(ct, &mut state) {
                    return true;
                }
            }
        }
        false
    }

    fn dfs_cycle_check(&self, ct: CTID, state: &mut BTreeMap<CTID, u8>) -> bool {
        state.insert(ct, 1); // in-progress

        if let Some(deps) = self.dependencies.get(&ct) {
            for &dep in deps {
                match state.get(&dep) {
                    Some(1) => return true, // back edge = cycle
                    Some(0) => {
                        if self.dfs_cycle_check(dep, state) {
                            return true;
                        }
                    }
                    _ => {} // already done, no cycle through here
                }
            }
        }

        state.insert(ct, 2); // done
        false
    }

    /// Get the number of CTs in the DAG.
    pub fn ct_count(&self) -> usize {
        self.dependencies.len()
    }

    /// Get the number of edges (dependency relationships).
    pub fn edge_count(&self) -> usize {
        self.edge_count
    }

    /// Check if all dependencies for a CT are satisfied.
    pub fn are_dependencies_satisfied(&self, ct: CTID) -> bool {
        self.pending
            .get(&ct)
            .map(|p| p.is_empty())
            .unwrap_or(true)
    }

    /// Notify that a CT has completed. Returns the set of CTs that are now unblocked.
    pub fn notify_completion(&mut self, ct: CTID) -> Result<Vec<CTID>> {
        if !self.dependencies.contains_key(&ct) {
            return Err(CsError::CtNotFound {
                ct_id: ct.to_string(),
            });
        }

        self.completed.insert(ct);

        let mut unblocked = Vec::new();

        // Get all CTs that depend on ct
        let dependents_of_ct: Vec<CTID> = self
            .dependents
            .get(&ct)
            .map(|s| s.iter().copied().collect())
            .unwrap_or_default();

        for dependent in dependents_of_ct {
            if let Some(pending_set) = self.pending.get_mut(&dependent) {
                pending_set.remove(&ct);
                if pending_set.is_empty() {
                    unblocked.push(dependent);
                }
            }
        }

        Ok(unblocked)
    }

    /// Check if the DAG contains a CT.
    pub fn contains_ct(&self, ct: CTID) -> bool {
        self.dependencies.contains_key(&ct)
    }

    /// Get the dependencies for a CT.
    pub fn get_dependencies(&self, ct: CTID) -> Result<BTreeSet<CTID>> {
        self.dependencies
            .get(&ct)
            .cloned()
            .ok_or_else(|| CsError::CtNotFound {
                ct_id: ct.to_string(),
            })
    }

    /// Get the pending (unsatisfied) dependencies for a CT.
    pub fn get_pending_dependencies(&self, ct: CTID) -> Result<BTreeSet<CTID>> {
        self.pending
            .get(&ct)
            .cloned()
            .ok_or_else(|| CsError::CtNotFound {
                ct_id: ct.to_string(),
            })
    }

    /// Get the CTs that depend on a given CT.
    pub fn get_dependents(&self, ct: CTID) -> Result<BTreeSet<CTID>> {
        self.dependents
            .get(&ct)
            .cloned()
            .ok_or_else(|| CsError::CtNotFound {
                ct_id: ct.to_string(),
            })
    }

    /// Get all CTs in the DAG.
    pub fn all_cts(&self) -> BTreeSet<CTID> {
        self.dependencies.keys().copied().collect()
    }
}

impl Default for DependencyDag {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a deterministic CTID from a string.
    #[allow(dead_code)]
    fn ctid_from_str(s: &str) -> CTID {
        let bytes = s.as_bytes();
        let mut seed = [0u8; 16];
        for (i, &b) in bytes.iter().enumerate().take(16) {
            seed[i] = b;
        }
        let ulid = ulid::Ulid::from_bytes(seed);
        CTID::from_ulid(ulid)
    }

    #[test]
    fn test_empty_dag_creation() {
        let dag = DependencyDag::new();
        assert_eq!(dag.ct_count(), 0);
        assert_eq!(dag.edge_count(), 0);
    }

    #[test]
    fn test_add_single_ct() {
        let mut dag = DependencyDag::new();
        let ct1 = CTID::new();
        assert!(dag.add_ct(ct1).is_ok());
        assert_eq!(dag.ct_count(), 1);
        assert!(dag.contains_ct(ct1));
    }

    #[test]
    fn test_add_duplicate_ct_fails() {
        let mut dag = DependencyDag::new();
        let ct1 = CTID::new();
        assert!(dag.add_ct(ct1).is_ok());
        assert!(dag.add_ct(ct1).is_err());
    }

    #[test]
    fn test_linear_chain_no_cycle() {
        let mut dag = DependencyDag::new();
        let ct1 = CTID::new();
        let ct2 = CTID::new();
        let ct3 = CTID::new();
        assert!(dag.add_ct(ct1).is_ok());
        assert!(dag.add_ct(ct2).is_ok());
        assert!(dag.add_ct(ct3).is_ok());

        assert!(dag.add_dependencies(ct1, {
            let mut s = BTreeSet::new();
            s.insert(ct2);
            s
        }).is_ok());

        assert!(dag.add_dependencies(ct2, {
            let mut s = BTreeSet::new();
            s.insert(ct3);
            s
        }).is_ok());

        assert_eq!(dag.ct_count(), 3);
        assert_eq!(dag.edge_count(), 2);
    }

    #[test]
    fn test_simple_cycle_detected() {
        let mut dag = DependencyDag::new();
        let ct1 = CTID::new();
        let ct2 = CTID::new();
        assert!(dag.add_ct(ct1).is_ok());
        assert!(dag.add_ct(ct2).is_ok());

        assert!(dag.add_dependencies(ct1, {
            let mut s = BTreeSet::new();
            s.insert(ct2);
            s
        }).is_ok());

        let result = dag.add_dependencies(ct2, {
            let mut s = BTreeSet::new();
            s.insert(ct1);
            s
        });

        assert!(result.is_err());
        if let Err(CsError::CyclicDependency { ct_id, cycle }) = result {
            assert_eq!(ct_id, ct2.to_string());
            assert!(cycle.contains("CT"));
        }
    }

    #[test]
    fn test_self_loop_detected() {
        let mut dag = DependencyDag::new();
        let ct1 = CTID::new();
        assert!(dag.add_ct(ct1).is_ok());

        let result = dag.add_dependencies(ct1, {
            let mut s = BTreeSet::new();
            s.insert(ct1);
            s
        });

        assert!(result.is_err());
        if let Err(CsError::CyclicDependency { cycle, .. }) = result {
            assert!(!cycle.is_empty());
        }
    }

    #[test]
    fn test_three_node_cycle() {
        let mut dag = DependencyDag::new();
        let ct1 = CTID::new();
        let ct2 = CTID::new();
        let ct3 = CTID::new();
        assert!(dag.add_ct(ct1).is_ok());
        assert!(dag.add_ct(ct2).is_ok());
        assert!(dag.add_ct(ct3).is_ok());

        assert!(dag.add_dependencies(ct1, {
            let mut s = BTreeSet::new();
            s.insert(ct2);
            s
        }).is_ok());

        assert!(dag.add_dependencies(ct2, {
            let mut s = BTreeSet::new();
            s.insert(ct3);
            s
        }).is_ok());

        let result = dag.add_dependencies(ct3, {
            let mut s = BTreeSet::new();
            s.insert(ct1);
            s
        });

        assert!(result.is_err());
        assert!(matches!(result, Err(CsError::CyclicDependency { .. })));
    }

    #[test]
    fn test_tree_structure_no_cycle() {
        let mut dag = DependencyDag::new();
        let ct_root = CTID::new();
        let ct_l1_a = CTID::new();
        let ct_l1_b = CTID::new();
        let ct_l2_a = CTID::new();

        for ct in &[ct_root, ct_l1_a, ct_l1_b, ct_l2_a] {
            assert!(dag.add_ct(*ct).is_ok());
        }

        assert!(dag.add_dependencies(ct_root, {
            let mut s = BTreeSet::new();
            s.insert(ct_l1_a);
            s.insert(ct_l1_b);
            s
        }).is_ok());

        assert!(dag.add_dependencies(ct_l1_a, {
            let mut s = BTreeSet::new();
            s.insert(ct_l2_a);
            s
        }).is_ok());

        assert_eq!(dag.ct_count(), 4);
        assert_eq!(dag.edge_count(), 3);
    }

    #[test]
    fn test_diamond_pattern_no_cycle() {
        let mut dag = DependencyDag::new();
        let ct_top = CTID::new();
        let ct_left = CTID::new();
        let ct_right = CTID::new();
        let ct_bottom = CTID::new();

        for ct in &[ct_top, ct_left, ct_right, ct_bottom] {
            assert!(dag.add_ct(*ct).is_ok());
        }

        assert!(dag.add_dependencies(ct_top, {
            let mut s = BTreeSet::new();
            s.insert(ct_left);
            s.insert(ct_right);
            s
        }).is_ok());

        assert!(dag.add_dependencies(ct_left, {
            let mut s = BTreeSet::new();
            s.insert(ct_bottom);
            s
        }).is_ok());

        assert!(dag.add_dependencies(ct_right, {
            let mut s = BTreeSet::new();
            s.insert(ct_bottom);
            s
        }).is_ok());

        assert_eq!(dag.ct_count(), 4);
        assert_eq!(dag.edge_count(), 4);
    }

    #[test]
    fn test_notify_completion_single_dependent() {
        let mut dag = DependencyDag::new();
        let ct1 = CTID::new();
        let ct2 = CTID::new();
        assert!(dag.add_ct(ct1).is_ok());
        assert!(dag.add_ct(ct2).is_ok());

        assert!(dag.add_dependencies(ct2, {
            let mut s = BTreeSet::new();
            s.insert(ct1);
            s
        }).is_ok());

        assert!(!dag.are_dependencies_satisfied(ct2));

        let unblocked = dag.notify_completion(ct1).expect("notify should succeed");
        assert!(unblocked.contains(&ct2));
        assert!(dag.are_dependencies_satisfied(ct2));
    }

    #[test]
    fn test_notify_completion_chain() {
        let mut dag = DependencyDag::new();
        let ct1 = CTID::new();
        let ct2 = CTID::new();
        let ct3 = CTID::new();

        for ct in &[ct1, ct2, ct3] {
            assert!(dag.add_ct(*ct).is_ok());
        }

        assert!(dag.add_dependencies(ct2, {
            let mut s = BTreeSet::new();
            s.insert(ct1);
            s
        }).is_ok());

        assert!(dag.add_dependencies(ct3, {
            let mut s = BTreeSet::new();
            s.insert(ct2);
            s
        }).is_ok());

        assert!(!dag.are_dependencies_satisfied(ct2));
        assert!(!dag.are_dependencies_satisfied(ct3));

        let unblocked1 = dag.notify_completion(ct1).expect("notify should succeed");
        assert!(unblocked1.contains(&ct2));
        assert!(!unblocked1.contains(&ct3));

        let unblocked2 = dag.notify_completion(ct2).expect("notify should succeed");
        assert!(unblocked2.contains(&ct3));
    }

    #[test]
    fn test_notify_completion_multiple_dependents() {
        let mut dag = DependencyDag::new();
        let ct_base = CTID::new();
        let ct_a = CTID::new();
        let ct_b = CTID::new();
        let ct_c = CTID::new();

        for ct in &[ct_base, ct_a, ct_b, ct_c] {
            assert!(dag.add_ct(*ct).is_ok());
        }

        for ct in &[ct_a, ct_b, ct_c] {
            assert!(dag.add_dependencies(*ct, {
                let mut s = BTreeSet::new();
                s.insert(ct_base);
                s
            }).is_ok());
        }

        let unblocked = dag.notify_completion(ct_base).expect("notify should succeed");
        assert_eq!(unblocked.len(), 3);
        assert!(unblocked.contains(&ct_a));
        assert!(unblocked.contains(&ct_b));
        assert!(unblocked.contains(&ct_c));
    }

    #[test]
    fn test_get_dependencies() {
        let mut dag = DependencyDag::new();
        let ct1 = CTID::new();
        let ct2 = CTID::new();
        let ct3 = CTID::new();

        for ct in &[ct1, ct2, ct3] {
            assert!(dag.add_ct(*ct).is_ok());
        }

        assert!(dag.add_dependencies(ct1, {
            let mut s = BTreeSet::new();
            s.insert(ct2);
            s.insert(ct3);
            s
        }).is_ok());

        let deps = dag.get_dependencies(ct1).expect("ct1 should exist");
        assert_eq!(deps.len(), 2);
        assert!(deps.contains(&ct2));
        assert!(deps.contains(&ct3));
    }

    #[test]
    fn test_get_dependents() {
        let mut dag = DependencyDag::new();
        let ct1 = CTID::new();
        let ct2 = CTID::new();
        let ct3 = CTID::new();

        for ct in &[ct1, ct2, ct3] {
            assert!(dag.add_ct(*ct).is_ok());
        }

        assert!(dag.add_dependencies(ct2, {
            let mut s = BTreeSet::new();
            s.insert(ct1);
            s
        }).is_ok());

        assert!(dag.add_dependencies(ct3, {
            let mut s = BTreeSet::new();
            s.insert(ct1);
            s
        }).is_ok());

        let dependents = dag.get_dependents(ct1).expect("ct1 should exist");
        assert_eq!(dependents.len(), 2);
        assert!(dependents.contains(&ct2));
        assert!(dependents.contains(&ct3));
    }

    #[test]
    fn test_large_dag_no_cycle() {
        let mut dag = DependencyDag::new();
        let mut cts = Vec::new();

        for _ in 0..100 {
            let ct = CTID::new();
            assert!(dag.add_ct(ct).is_ok());
            cts.push(ct);
        }

        for i in 1..cts.len() {
            assert!(dag.add_dependencies(cts[i], {
                let mut s = BTreeSet::new();
                s.insert(cts[i - 1]);
                s
            }).is_ok());
        }

        assert_eq!(dag.ct_count(), 100);
        assert_eq!(dag.edge_count(), 99);
    }

    #[test]
    fn test_complex_dag_with_convergence() {
        let mut dag = DependencyDag::new();
        let ct_sources = vec![CTID::new(), CTID::new(), CTID::new()];
        let ct_middle = vec![CTID::new(), CTID::new()];
        let ct_sink = CTID::new();

        for ct in ct_sources.iter().chain(ct_middle.iter()).chain(&[ct_sink]) {
            assert!(dag.add_ct(*ct).is_ok());
        }

        for (i, &middle_ct) in ct_middle.iter().enumerate() {
            let mut deps = BTreeSet::new();
            deps.insert(ct_sources[i % ct_sources.len()]);
            if i < ct_sources.len() {
                deps.insert(ct_sources[(i + 1) % ct_sources.len()]);
            }
            assert!(dag.add_dependencies(middle_ct, deps).is_ok());
        }

        assert!(dag.add_dependencies(ct_sink, {
            let mut s = BTreeSet::new();
            for &ct in &ct_middle {
                s.insert(ct);
            }
            s
        }).is_ok());

        assert!(dag.contains_ct(ct_sink));
        assert!(!dag.are_dependencies_satisfied(ct_sink));
    }

    #[test]
    fn test_partial_cycle_in_large_graph() {
        let mut dag = DependencyDag::new();
        let ct_a = CTID::new();
        let ct_b = CTID::new();
        let ct_c = CTID::new();
        let ct_d = CTID::new();
        let ct_e = CTID::new();

        for ct in &[ct_a, ct_b, ct_c, ct_d, ct_e] {
            assert!(dag.add_ct(*ct).is_ok());
        }

        assert!(dag.add_dependencies(ct_b, {
            let mut s = BTreeSet::new();
            s.insert(ct_a);
            s
        }).is_ok());

        assert!(dag.add_dependencies(ct_c, {
            let mut s = BTreeSet::new();
            s.insert(ct_b);
            s
        }).is_ok());

        assert!(dag.add_dependencies(ct_d, {
            let mut s = BTreeSet::new();
            s.insert(ct_c);
            s
        }).is_ok());

        assert!(dag.add_dependencies(ct_e, {
            let mut s = BTreeSet::new();
            s.insert(ct_a);
            s
        }).is_ok());

        let result = dag.add_dependencies(ct_a, {
            let mut s = BTreeSet::new();
            s.insert(ct_d);
            s
        });

        assert!(result.is_err());
    }

    #[test]
    fn test_concurrent_multiple_dependencies() {
        let mut dag = DependencyDag::new();
        let ct1 = CTID::new();
        let ct2 = CTID::new();
        let ct3 = CTID::new();
        let ct4 = CTID::new();
        let ct5 = CTID::new();

        for ct in &[ct1, ct2, ct3, ct4, ct5] {
            assert!(dag.add_ct(*ct).is_ok());
        }

        assert!(dag.add_dependencies(ct4, {
            let mut s = BTreeSet::new();
            s.insert(ct1);
            s.insert(ct2);
            s.insert(ct3);
            s
        }).is_ok());

        assert!(dag.add_dependencies(ct5, {
            let mut s = BTreeSet::new();
            s.insert(ct4);
            s
        }).is_ok());

        assert!(!dag.are_dependencies_satisfied(ct4));
        assert!(!dag.are_dependencies_satisfied(ct5));

        dag.notify_completion(ct1).expect("notify should succeed");
        dag.notify_completion(ct2).expect("notify should succeed");
        let unblocked = dag.notify_completion(ct3).expect("notify should succeed");

        assert!(unblocked.contains(&ct4));
        assert!(!unblocked.contains(&ct5));

        let unblocked = dag.notify_completion(ct4).expect("notify should succeed");
        assert!(unblocked.contains(&ct5));
    }

    #[test]
    fn test_all_cts_method() {
        let mut dag = DependencyDag::new();
        let ct1 = CTID::new();
        let ct2 = CTID::new();
        let ct3 = CTID::new();

        for ct in &[ct1, ct2, ct3] {
            assert!(dag.add_ct(*ct).is_ok());
        }

        let all = dag.all_cts();
        assert_eq!(all.len(), 3);
        assert!(all.contains(&ct1));
        assert!(all.contains(&ct2));
        assert!(all.contains(&ct3));
    }

    #[test]
    fn test_contains_ct() {
        let mut dag = DependencyDag::new();
        let ct1 = CTID::new();
        let ct2 = CTID::new();

        assert!(dag.add_ct(ct1).is_ok());
        assert!(dag.contains_ct(ct1));
        assert!(!dag.contains_ct(ct2));
    }

    #[test]
    fn test_get_pending_dependencies() {
        let mut dag = DependencyDag::new();
        let ct1 = CTID::new();
        let ct2 = CTID::new();
        let ct3 = CTID::new();

        for ct in &[ct1, ct2, ct3] {
            assert!(dag.add_ct(*ct).is_ok());
        }

        assert!(dag.add_dependencies(ct1, {
            let mut s = BTreeSet::new();
            s.insert(ct2);
            s.insert(ct3);
            s
        }).is_ok());

        let pending = dag.get_pending_dependencies(ct1).expect("ct1 should exist");
        assert_eq!(pending.len(), 2);

        dag.notify_completion(ct2).ok();

        let pending = dag.get_pending_dependencies(ct1).expect("ct1 should exist");
        assert_eq!(pending.len(), 1);
        assert!(pending.contains(&ct3));
    }

    #[test]
    fn test_cycle_with_multiple_paths() {
        // Build diamond: ct2->ct1, ct3->ct1, ct4->ct2, ct4->ct3
        // Then adding ct1->ct4 creates cycle: ct1->ct4->ct2->ct1
        let mut dag = DependencyDag::new();
        let ct1 = CTID::new();
        let ct2 = CTID::new();
        let ct3 = CTID::new();
        let ct4 = CTID::new();

        for ct in &[ct1, ct2, ct3, ct4] {
            assert!(dag.add_ct(*ct).is_ok());
        }

        assert!(dag.add_dependencies(ct2, {
            let mut s = BTreeSet::new();
            s.insert(ct1);
            s
        }).is_ok());

        assert!(dag.add_dependencies(ct3, {
            let mut s = BTreeSet::new();
            s.insert(ct1);
            s
        }).is_ok());

        assert!(dag.add_dependencies(ct4, {
            let mut s = BTreeSet::new();
            s.insert(ct2);
            s.insert(ct3);
            s
        }).is_ok());

        // ct1 -> ct4 creates cycle (ct1 -> ct4 -> ct2 -> ct1)
        let result = dag.add_dependencies(ct1, {
            let mut s = BTreeSet::new();
            s.insert(ct4);
            s
        });

        assert!(result.is_err());
        assert!(matches!(result, Err(CsError::CyclicDependency { .. })));
    }
}
