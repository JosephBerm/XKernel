// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 XKernal Contributors
//! Integration tests for capability-engine crate

use capability_engine::*;

#[test]
fn test_capability_creation_and_verification() {
    let perms = vec![Permission::new(1, PermissionFlags::read())];
    let cap = Capability::new(1, 100, perms, None);

    assert_eq!(cap.id, 1);
    assert_eq!(cap.owner, 100);
    assert!(cap.is_active);
    assert!(cap.has_permission(1, PermissionFlags::read()));
}

#[test]
fn test_capability_delegation() {
    let mut engine = PolicyEngine::new();
    let policy = MandatoryPolicy::new("allow_read".into(), PolicyExpression::Allow, 100);
    engine.add_policy(policy);

    let perms = vec![Permission::new(1, PermissionFlags::read() | PermissionFlags::write())];
    let cap = Capability::new(1, 100, perms, None);

    assert!(engine.enforce(&cap, 1).unwrap());
}

#[test]
fn test_permission_attenuation() {
    let perms = vec![Permission::new(1, PermissionFlags::read() | PermissionFlags::write())];
    let cap = Capability::new(1, 100, perms.clone(), None);

    let attenuated = cap.attenuate(&Permission::new(1, PermissionFlags::read())).unwrap();
    assert!(attenuated.has_permission(1, PermissionFlags::read()));
    assert!(!attenuated.has_permission(1, PermissionFlags::write()));
}

#[test]
fn test_delegation_chain() {
    let mut chain = DelegationChain::new(10);
    let perm = Permission::new(1, PermissionFlags::read());
    let rule = AttenuationRule::new(perm, false);

    chain
        .add_entry(DelegationEntry::new(
            1,
            2,
            rule.clone(),
            RevocationPolicy::Never,
        ))
        .unwrap();

    chain
        .add_entry(DelegationEntry::new(
            2,
            3,
            rule,
            RevocationPolicy::Never,
        ))
        .unwrap();

    assert_eq!(chain.depth(), 2);
    assert_eq!(chain.root_cap(), Some(1));
    assert_eq!(chain.leaf_cap(), Some(3));
}

#[test]
fn test_capability_revocation() {
    let perms = vec![Permission::new(1, PermissionFlags::all())];
    let mut cap = Capability::new(1, 100, perms, None);

    assert!(cap.is_active);
    cap.revoke();
    assert!(!cap.is_active);
    assert!(!cap.has_permission(1, PermissionFlags::read()));
}

#[test]
fn test_policy_enforcement() {
    let mut engine = PolicyEngine::new();

    let policy1 = MandatoryPolicy::new(
        "policy1".into(),
        PolicyExpression::Allow,
        100,
    );
    engine.add_policy(policy1);

    let policy2 = MandatoryPolicy::new(
        "policy2".into(),
        PolicyExpression::Allow,
        50,
    );
    engine.add_policy(policy2);

    let perms = vec![Permission::new(1, PermissionFlags::all())];
    let cap = Capability::new(1, 100, perms, None);

    assert!(engine.enforce(&cap, 1).unwrap());
    assert_eq!(engine.policy_count(), 2);
}

#[test]
fn test_proof_verification() {
    let mut engine = VerificationEngine::new();
    let policy = VerificationPolicy::new("hmac_check".into(), ProofType::Hmac);
    engine.add_policy(policy);

    let cap = Capability::new(1, 100, vec![], None);
    let proof = Proof::new(vec![1, 2, 3, 4], ProofType::Hmac);

    let result = engine.verify(&cap, Some(&proof)).unwrap();
    assert_eq!(result, VerificationResult::Valid);
}

#[test]
fn test_capability_token() {
    let token = CapabilityToken::new(42, 1);
    assert_eq!(token.id(), 42);
    assert_eq!(token.generation(), 1);
}

#[test]
fn test_invalid_policy_expression() {
    let policy = PolicyExpression::Deny;
    let cap = Capability::new(1, 100, vec![], None);

    let result = policy.evaluate(&cap, 1).unwrap();
    assert!(!result);
}
