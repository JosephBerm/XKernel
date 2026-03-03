// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! # CT Dependency DAG with Cycle Detection
//!
//! This module implements a directed acyclic graph (DAG) for CT dependencies
//! with O(V + E) cycle detection using Tarjan's strongly connected components (SCC)
//! algorithm. Cycles are detected at spawn time and rejected with clear error messages.
//!
//! ## Design
//!
//! - **Tarjan's SCC Algorithm**: O(V + E) complexity for cycle detection
//! - **Spawn-Time Validation**: Dependencies validated before CT enters system
//! - **Clear Error Messages**: Cycle detection returns all CTIDs in the cycle
//! - **Dependency Wait Lists**: Track which CTs block a CT from Reason phase
//! - **Notification Mechanism**: When dependency completes, notify dependents
//!
//! ## Invariants
//!
//! - **Invariant 3**: All dependencies must complete before CT enters Reason phase
//! - **Invariant 5**: DAG must be cycle-checked at spawn time; circular deps rejected
//!
//! ## References
//!
//! - Engineering Plan § 4.1.9 (Dependencies)
//! - Engineering Plan § 5.2 (CT Invariants & Type-Safety)
//! - Week 4 Objective: CT dependency DAG with cycle detection
use core::cmp::Ordering;
use crate::error::{CsError, Result};
use crate::ids::CTID;
use super::*;

use alloc::string::ToString;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::format;
use alloc::vec;
use alloc::vec::Vec;
use ulid::Ulid;


        let bytes = s.as_bytes();

        let mut seed = [0u8; 16];

        for (i, &b) in bytes.iter().enumerate().take(16) {

            seed[i] = b;

        }

        // Create deterministic CTID based on string

        let ulid = Ulid::from_bytes(seed);

        CTID::from_ulid(ulid)

    

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

        // ct1 -> ct2 (ct1 depends on ct2)

        assert!(dag.add_dependencies(ct1, {

            let mut s = BTreeSet::new();

            s.insert(ct2);

            s

        }).is_ok());

        // ct2 -> ct3

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

        // ct1 -> ct2

        assert!(dag.add_dependencies(ct1, {

            let mut s = BTreeSet::new();

            s.insert(ct2);

            s

        }).is_ok());

        // ct2 -> ct1 (creates cycle)

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

        // ct1 depends on itself

        let result = dag.add_dependencies(ct1, {

            let mut s = BTreeSet::new();

            s.insert(ct1);

            s

        });

        assert!(result.is_err());

        if let Err(CsError::CyclicDependency { cycle, .. }) = result {

            assert!(cycle.len() > 0);

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

        // ct1 -> ct2

        assert!(dag.add_dependencies(ct1, {

            let mut s = BTreeSet::new();

            s.insert(ct2);

            s

        }).is_ok());

        // ct2 -> ct3

        assert!(dag.add_dependencies(ct2, {

            let mut s = BTreeSet::new();

            s.insert(ct3);

            s

        }).is_ok());

        // ct3 -> ct1 (creates 3-node cycle)

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

        // Root depends on two children

        assert!(dag.add_dependencies(ct_root, {

            let mut s = BTreeSet::new();

            s.insert(ct_l1_a);

            s.insert(ct_l1_b);

            s

        }).is_ok());

        // One child depends on a grandchild

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

        // Top depends on left and right

        assert!(dag.add_dependencies(ct_top, {

            let mut s = BTreeSet::new();

            s.insert(ct_left);

            s.insert(ct_right);

            s

        }).is_ok());

        // Both left and right depend on bottom

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

        // ct2 depends on ct1

        assert!(dag.add_dependencies(ct2, {

            let mut s = BTreeSet::new();

            s.insert(ct1);

            s

        }).is_ok());

        // Before completion, ct2 has pending dependencies

        assert!(!dag.are_dependencies_satisfied(ct2));

        // Notify ct1 completion

        let unblocked = dag.notify_completion(ct1).expect("notify should succeed");
        
        // ct2 should now be unblocked

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

        // ct1 -> ct2 -> ct3

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

        // Initially, ct2 and ct3 are blocked

        assert!(!dag.are_dependencies_satisfied(ct2));

        assert!(!dag.are_dependencies_satisfied(ct3));

        // Notify ct1 completion

        let unblocked1 = dag.notify_completion(ct1).expect("notify should succeed");

        assert!(unblocked1.contains(&ct2));

        assert!(!unblocked1.contains(&ct3)); // ct3 still waiting on ct2

        // Notify ct2 completion

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

        // All three depend on base

        for ct in &[ct_a, ct_b, ct_c] {

            assert!(dag.add_dependencies(*ct, {

                let mut s = BTreeSet::new();

                s.insert(ct_base);

                s

            }).is_ok());

        }

        // Notify base completion

        let unblocked = dag.notify_completion(ct_base).expect("notify should succeed");
        
        // All three should be unblocked

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

        // ct2 and ct3 both depend on ct1

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
        
        // Create 100 CTs

        for _ in 0..100 {

            let ct = CTID::new();

            assert!(dag.add_ct(ct).is_ok());

            cts.push(ct);

        }

        // Add dependencies forming a long chain

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

        // Add all CTs

        for ct in ct_sources.iter().chain(ct_middle.iter()).chain(&[ct_sink]) {

            assert!(dag.add_ct(*ct).is_ok());

        }

        // Sources -> Middle

        for (i, &middle_ct) in ct_middle.iter().enumerate() {

            let mut deps = BTreeSet::new();

            deps.insert(ct_sources[i % ct_sources.len()]);

            if i < ct_sources.len() {

                deps.insert(ct_sources[(i + 1) % ct_sources.len()]);

            }

            assert!(dag.add_dependencies(middle_ct, deps).is_ok());

        }

        // Middle -> Sink

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

        // Build: a -> b -> c -> d (no cycle)

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

        // e depends on a (still no cycle)

        assert!(dag.add_dependencies(ct_e, {

            let mut s = BTreeSet::new();

            s.insert(ct_a);

            s

        }).is_ok());

        // Try to add d -> a (creates cycle)

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

        // ct4 depends on ct1, ct2, ct3

        assert!(dag.add_dependencies(ct4, {

            let mut s = BTreeSet::new();

            s.insert(ct1);

            s.insert(ct2);

            s.insert(ct3);

            s

        }).is_ok());

        // ct5 depends on ct4

        assert!(dag.add_dependencies(ct5, {

            let mut s = BTreeSet::new();

            s.insert(ct4);

            s

        }).is_ok());

        assert!(!dag.are_dependencies_satisfied(ct4));

        assert!(!dag.are_dependencies_satisfied(ct5));

        // Notify ct1, ct2, ct3 completion

        dag.notify_completion(ct1).expect("notify should succeed");

        dag.notify_completion(ct2).expect("notify should succeed");

        let unblocked = dag.notify_completion(ct3).expect("notify should succeed");

        // ct4 should be unblocked

        assert!(unblocked.contains(&ct4));

        assert!(!unblocked.contains(&ct5)); // ct5 still waiting on ct4

        // Notify ct4

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

        let mut dag = DependencyDag::new();

        let ct1 = CTID::new();

        let ct2 = CTID::new();

        let ct3 = CTID::new();

        let ct4 = CTID::new();

        for ct in &[ct1, ct2, ct3, ct4] {

            assert!(dag.add_ct(*ct).is_ok());

        }

        // Build: 1 -> 2, 1 -> 3, 2 -> 4, 3 -> 4 (diamond, no cycle)

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

        // Try to add 1 -> 4 (still no cycle since it's a DAG)

        assert!(dag.add_dependencies(ct1, {

            let mut s = BTreeSet::new();

            s.insert(ct4);

            s

        }).is_ok());

        // Try to add 4 -> 1 (NOW creates a cycle)

        let result = dag.add_dependencies(ct4, {

            let mut s = BTreeSet::new();

            s.insert(ct1);

            s

        });

        assert!(result.is_err());

        assert!(matches!(result, Err(CsError::CyclicDependency { .. })));

    }


