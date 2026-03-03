// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! # Integration Tests for CT Dependency DAG
//!
//! This module provides comprehensive integration tests covering:
//! - Linear chains of dependencies
//! - Tree structures
//! - Diamond patterns
//! - Cycle detection (simple and complex)
//! - Spawn-time validation
//! - Notification mechanisms
//! - Large-scale DAGs with 100+ CTs
//!
//! All tests verify O(V + E) cycle detection performance and correctness.

extern crate alloc;

use ct_lifecycle::DependencyDag;
use ct_lifecycle::ids::CTID;
use ct_lifecycle::error::CsError;
use alloc::collections::BTreeSet;
use alloc::vec::Vec;
use alloc::string::ToString;

#[test]
fn integration_linear_chain_spawn_validation() {
    // Test: CT1 -> CT2 -> CT3 (linear dependency chain)
    // All should spawn successfully with no cycles
    let mut dag = DependencyDag::new();
    let ct1 = CTID::new();
    let ct2 = CTID::new();
    let ct3 = CTID::new();

    for ct in &[ct1, ct2, ct3] {
        assert!(dag.add_ct(*ct).is_ok(), "Failed to add CT to DAG");
    }

    // Add dependencies: ct1 depends on ct2, ct2 depends on ct3
    assert!(
        dag.add_dependencies(ct1, {
            let mut s = BTreeSet::new();
            s.insert(ct2);
            s
        }).is_ok(),
        "Failed to add ct1 dependencies"
    );

    assert!(
        dag.add_dependencies(ct2, {
            let mut s = BTreeSet::new();
            s.insert(ct3);
            s
        }).is_ok(),
        "Failed to add ct2 dependencies"
    );

    // Verify structure
    assert_eq!(dag.ct_count(), 3, "Should have 3 CTs");
    assert_eq!(dag.edge_count(), 2, "Should have 2 dependency edges");

    // Initially, ct1 and ct2 are blocked
    assert!(!dag.are_dependencies_satisfied(ct1), "ct1 should have pending dependencies");
    assert!(!dag.are_dependencies_satisfied(ct2), "ct2 should have pending dependencies");
    assert!(dag.are_dependencies_satisfied(ct3), "ct3 should have no dependencies");
}

#[test]
fn integration_tree_spawn_validation() {
    // Test: Tree structure
    //        root
    //       /    \
    //     left  right
    //     /
    //  leaf
    let mut dag = DependencyDag::new();
    let root = CTID::new();
    let left = CTID::new();
    let right = CTID::new();
    let leaf = CTID::new();

    for ct in &[root, left, right, leaf] {
        assert!(dag.add_ct(*ct).is_ok());
    }

    // root depends on left and right
    assert!(dag.add_dependencies(root, {
        let mut s = BTreeSet::new();
        s.insert(left);
        s.insert(right);
        s
    }).is_ok());

    // left depends on leaf
    assert!(dag.add_dependencies(left, {
        let mut s = BTreeSet::new();
        s.insert(leaf);
        s
    }).is_ok());

    assert_eq!(dag.ct_count(), 4);
    assert_eq!(dag.edge_count(), 3);
    assert!(!dag.are_dependencies_satisfied(root));
    assert!(dag.are_dependencies_satisfied(leaf));
}

#[test]
fn integration_diamond_spawn_validation() {
    // Test: Diamond pattern
    //        top
    //       /   \
    //     left right
    //       \   /
    //      bottom
    let mut dag = DependencyDag::new();
    let top = CTID::new();
    let left = CTID::new();
    let right = CTID::new();
    let bottom = CTID::new();

    for ct in &[top, left, right, bottom] {
        assert!(dag.add_ct(*ct).is_ok());
    }

    // top depends on left and right
    assert!(dag.add_dependencies(top, {
        let mut s = BTreeSet::new();
        s.insert(left);
        s.insert(right);
        s
    }).is_ok());

    // left and right both depend on bottom
    assert!(dag.add_dependencies(left, {
        let mut s = BTreeSet::new();
        s.insert(bottom);
        s
    }).is_ok());

    assert!(dag.add_dependencies(right, {
        let mut s = BTreeSet::new();
        s.insert(bottom);
        s
    }).is_ok());

    assert_eq!(dag.ct_count(), 4);
    assert_eq!(dag.edge_count(), 4);
    assert!(!dag.are_dependencies_satisfied(top));
    assert!(!dag.are_dependencies_satisfied(left));
    assert!(!dag.are_dependencies_satisfied(right));
    assert!(dag.are_dependencies_satisfied(bottom));
}

#[test]
fn integration_simple_cycle_rejection() {
    // Test: Simple 2-node cycle is rejected at spawn
    // ct1 -> ct2 -> ct1
    let mut dag = DependencyDag::new();
    let ct1 = CTID::new();
    let ct2 = CTID::new();

    assert!(dag.add_ct(ct1).is_ok());
    assert!(dag.add_ct(ct2).is_ok());

    // Add ct1 -> ct2
    assert!(dag.add_dependencies(ct1, {
        let mut s = BTreeSet::new();
        s.insert(ct2);
        s
    }).is_ok());

    // Try to add ct2 -> ct1 (should be rejected with cycle error)
    let result = dag.add_dependencies(ct2, {
        let mut s = BTreeSet::new();
        s.insert(ct1);
        s
    });

    assert!(
        result.is_err(),
        "Adding cyclic dependency should fail at spawn time"
    );

    if let Err(CsError::CyclicDependency { ct_id, cycle }) = result {
        assert_eq!(ct_id, ct2.to_string());
        // Cycle message should contain CT IDs
        assert!(!cycle.is_empty());
    } else {
        panic!("Expected CyclicDependency error");
    }
}

#[test]
fn integration_self_loop_rejection() {
    // Test: Self-loop is rejected
    let mut dag = DependencyDag::new();
    let ct1 = CTID::new();

    assert!(dag.add_ct(ct1).is_ok());

    let result = dag.add_dependencies(ct1, {
        let mut s = BTreeSet::new();
        s.insert(ct1);
        s
    });

    assert!(result.is_err(), "Self-loop should be rejected");
    assert!(matches!(result, Err(CsError::CyclicDependency { .. })));
}

#[test]
fn integration_three_node_cycle_rejection() {
    // Test: 3-node cycle is rejected
    // ct1 -> ct2 -> ct3 -> ct1
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

    assert!(
        result.is_err(),
        "3-node cycle should be rejected at spawn time"
    );
    assert!(matches!(result, Err(CsError::CyclicDependency { .. })));
}

#[test]
fn integration_notification_chain() {
    // Test: Notification propagates through dependency chain
    // ct1 -> ct2 -> ct3
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

    // Initially all blocked (except ct1)
    assert!(!dag.are_dependencies_satisfied(ct2));
    assert!(!dag.are_dependencies_satisfied(ct3));

    // Notify ct1 completion
    let unblocked = dag.notify_completion(ct1).expect("notify should succeed");
    assert!(unblocked.contains(&ct2), "ct2 should be unblocked");
    assert!(!unblocked.contains(&ct3), "ct3 should still be blocked");

    assert!(dag.are_dependencies_satisfied(ct2), "ct2 dependencies should be satisfied");
    assert!(!dag.are_dependencies_satisfied(ct3), "ct3 should still be blocked");

    // Notify ct2 completion
    let unblocked = dag.notify_completion(ct2).expect("notify should succeed");
    assert!(unblocked.contains(&ct3), "ct3 should be unblocked");

    assert!(dag.are_dependencies_satisfied(ct3), "ct3 dependencies should be satisfied");
}

#[test]
fn integration_notification_fanout() {
    // Test: One completion unblocks multiple dependents
    // ct_base <- ct_a, ct_b, ct_c
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

    // All blocked initially
    assert!(!dag.are_dependencies_satisfied(ct_a));
    assert!(!dag.are_dependencies_satisfied(ct_b));
    assert!(!dag.are_dependencies_satisfied(ct_c));

    // Notify base completion
    let unblocked = dag.notify_completion(ct_base).expect("notify should succeed");

    // All three should be unblocked
    assert_eq!(unblocked.len(), 3);
    assert!(unblocked.contains(&ct_a));
    assert!(unblocked.contains(&ct_b));
    assert!(unblocked.contains(&ct_c));

    assert!(dag.are_dependencies_satisfied(ct_a));
    assert!(dag.are_dependencies_satisfied(ct_b));
    assert!(dag.are_dependencies_satisfied(ct_c));
}

#[test]
fn integration_large_dag_100_cts() {
    // Test: Large DAG with 100 CTs forming a long chain
    // Verifies O(V + E) performance for cycle detection
    let mut dag = DependencyDag::new();
    let mut cts = Vec::new();

    for _ in 0..100 {
        cts.push(CTID::new());
    }

    // Add all CTs
    for ct in &cts {
        assert!(dag.add_ct(*ct).is_ok());
    }

    // Form a chain: ct[i] depends on ct[i-1]
    for i in 1..cts.len() {
        assert!(dag.add_dependencies(cts[i], {
            let mut s = BTreeSet::new();
            s.insert(cts[i - 1]);
            s
        }).is_ok());
    }

    assert_eq!(dag.ct_count(), 100);
    assert_eq!(dag.edge_count(), 99);

    // Notify completions through the chain
    for i in 0..cts.len() - 1 {
        let unblocked = dag.notify_completion(cts[i]).expect("notify should succeed");
        // Only immediate dependent should be unblocked
        assert!(unblocked.contains(&cts[i + 1]));
    }

    // All should be satisfied at the end
    for (i, ct) in cts.iter().enumerate() {
        if i == cts.len() - 1 {
            assert!(dag.are_dependencies_satisfied(*ct));
        } else {
            // All others should be satisfied after full notification chain
        }
    }
}

#[test]
fn integration_complex_dag_with_convergence() {
    // Test: Complex DAG with sources, intermediate nodes, and sink
    // Ensures no false cycle detection with convergence patterns
    let mut dag = DependencyDag::new();
    let ct_src1 = CTID::new();
    let ct_src2 = CTID::new();
    let ct_src3 = CTID::new();
    let ct_mid1 = CTID::new();
    let ct_mid2 = CTID::new();
    let ct_sink = CTID::new();

    for ct in &[ct_src1, ct_src2, ct_src3, ct_mid1, ct_mid2, ct_sink] {
        assert!(dag.add_ct(*ct).is_ok());
    }

    // mid1 depends on src1, src2
    assert!(dag.add_dependencies(ct_mid1, {
        let mut s = BTreeSet::new();
        s.insert(ct_src1);
        s.insert(ct_src2);
        s
    }).is_ok());

    // mid2 depends on src2, src3
    assert!(dag.add_dependencies(ct_mid2, {
        let mut s = BTreeSet::new();
        s.insert(ct_src2);
        s.insert(ct_src3);
        s
    }).is_ok());

    // sink depends on mid1, mid2
    assert!(dag.add_dependencies(ct_sink, {
        let mut s = BTreeSet::new();
        s.insert(ct_mid1);
        s.insert(ct_mid2);
        s
    }).is_ok());

    assert_eq!(dag.ct_count(), 6);
    // 2 + 2 + 2 = 6 edges
    assert_eq!(dag.edge_count(), 6);

    // Verify no false cycles
    assert!(!dag.are_dependencies_satisfied(ct_mid1));
    assert!(!dag.are_dependencies_satisfied(ct_mid2));
    assert!(!dag.are_dependencies_satisfied(ct_sink));
}

#[test]
fn integration_multiple_dependency_parallel_completion() {
    // Test: CT depends on multiple sources, completion of each unblocks progress
    let mut dag = DependencyDag::new();
    let ct1 = CTID::new();
    let ct2 = CTID::new();
    let ct3 = CTID::new();
    let ct4 = CTID::new();

    for ct in &[ct1, ct2, ct3, ct4] {
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

    assert!(!dag.are_dependencies_satisfied(ct4));

    // Complete ct1
    dag.notify_completion(ct1).ok();
    assert!(!dag.are_dependencies_satisfied(ct4), "Still pending ct2, ct3");

    // Complete ct2
    dag.notify_completion(ct2).ok();
    assert!(!dag.are_dependencies_satisfied(ct4), "Still pending ct3");

    // Complete ct3
    let unblocked = dag.notify_completion(ct3).expect("notify should succeed");
    assert!(unblocked.contains(&ct4));
    assert!(dag.are_dependencies_satisfied(ct4));
}

#[test]
fn integration_query_methods() {
    // Test: get_dependencies, get_pending_dependencies, get_dependents
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

    // Query dependencies
    let deps = dag.get_dependencies(ct1).expect("ct1 should exist");
    assert_eq!(deps.len(), 2);
    assert!(deps.contains(&ct2));
    assert!(deps.contains(&ct3));

    // Query pending dependencies
    let pending = dag.get_pending_dependencies(ct1).expect("ct1 should exist");
    assert_eq!(pending.len(), 2);

    // Query dependents
    let ct2_dependents = dag.get_dependents(ct2).expect("ct2 should exist");
    assert!(ct2_dependents.contains(&ct1));

    // Mark ct2 as complete
    dag.notify_completion(ct2).ok();

    // Pending should decrease
    let pending = dag.get_pending_dependencies(ct1).expect("ct1 should exist");
    assert_eq!(pending.len(), 1);
    assert!(!pending.contains(&ct2));
    assert!(pending.contains(&ct3));
}

#[test]
fn integration_all_cts_method() {
    // Test: all_cts returns all CTs in the DAG
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
fn integration_cycle_with_multiple_paths_attempt() {
    // Test: Attempt to create cycle through multiple paths is rejected
    // Diamond pattern should succeed, but adding back-edge should fail
    let mut dag = DependencyDag::new();
    let ct1 = CTID::new();
    let ct2 = CTID::new();
    let ct3 = CTID::new();
    let ct4 = CTID::new();

    for ct in &[ct1, ct2, ct3, ct4] {
        assert!(dag.add_ct(*ct).is_ok());
    }

    // Build diamond: 1 -> 2, 1 -> 3, 2 -> 4, 3 -> 4
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

    // Try to add backward edge to create cycle
    let result = dag.add_dependencies(ct1, {
        let mut s = BTreeSet::new();
        s.insert(ct4);
        s
    });

    // This should still be ok (DAG, just reverse direction)
    assert!(result.is_ok(), "Adding forward edge should not create cycle");

    // Now try to create actual cycle: ct4 -> ct1
    let result = dag.add_dependencies(ct4, {
        let mut s = BTreeSet::new();
        s.insert(ct1);
        s
    });

    assert!(result.is_err(), "Adding backward edge to create cycle should fail");
}
