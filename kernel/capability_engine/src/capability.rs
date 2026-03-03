// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 XKernal Contributors
//! Core capability tokens and permission management

use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Capability engine errors
#[derive(Debug, Clone, Error)]
pub enum CapabilityError {
    /// Permission denied
    #[error("permission denied for {0}")]
    PermissionDenied(alloc::string::String),
    /// Invalid capability
    #[error("invalid capability: {0}")]
    InvalidCapability(alloc::string::String),
    /// Capability revoked
    #[error("capability {0} has been revoked")]
    CapabilityRevoked(u64),
    /// Attenuation violation
    #[error("attenuation violation: {0}")]
    AttenuationViolation(alloc::string::String),
}

pub type Result<T> = core::result::Result<T, CapabilityError>;

/// Permission flags for capability-based access control
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PermissionFlags(u32);

impl PermissionFlags {
    /// Create empty permissions
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Create full permissions
    pub const fn all() -> Self {
        Self(0xFFFF_FFFF)
    }

    /// Read permission
    pub const fn read() -> Self {
        Self(0x0001)
    }

    /// Write permission
    pub const fn write() -> Self {
        Self(0x0002)
    }

    /// Execute permission
    pub const fn execute() -> Self {
        Self(0x0004)
    }

    /// Delegate permission
    pub const fn delegate() -> Self {
        Self(0x0008)
    }

    /// Revoke permission
    pub const fn revoke() -> Self {
        Self(0x0010)
    }

    /// Check if a permission is granted
    pub fn contains(&self, other: PermissionFlags) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Get raw permission bits
    pub fn bits(&self) -> u32 {
        self.0
    }
}

impl core::ops::BitOr for PermissionFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl core::ops::BitAnd for PermissionFlags {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

/// Permission specification with scope
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission {
    /// Scope/resource identifier
    pub scope: u64,
    /// Permission flags
    pub flags: PermissionFlags,
}

impl Permission {
    /// Create a new permission
    pub fn new(scope: u64, flags: PermissionFlags) -> Self {
        Self { scope, flags }
    }

    /// Attenuate this permission with another
    pub fn attenuate(&self, other: &Permission) -> Self {
        if self.scope != other.scope {
            return Self::new(self.scope, PermissionFlags::empty());
        }

        Self::new(self.scope, self.flags & other.flags)
    }
}

/// Unforgeable capability token wrapping a capability identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CapabilityToken {
    id: u64,
    generation: u32,
}

impl CapabilityToken {
    /// Create a new capability token
    pub fn new(id: u64, generation: u32) -> Self {
        Self { id, generation }
    }

    /// Get the capability ID
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Get the generation number
    pub fn generation(&self) -> u32 {
        self.generation
    }
}

/// Core capability data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    /// Unique capability identifier
    pub id: u64,
    /// Owner of this capability
    pub owner: u64,
    /// Permissions granted by this capability
    pub permissions: Vec<Permission>,
    /// Parent capability (for delegation chains)
    pub parent_cap: Option<u64>,
    /// Generation for revocation tracking
    pub generation: u32,
    /// Is this capability currently active
    pub is_active: bool,
}

impl Capability {
    /// Create a new capability
    pub fn new(
        id: u64,
        owner: u64,
        permissions: Vec<Permission>,
        parent_cap: Option<u64>,
    ) -> Self {
        Self {
            id,
            owner,
            permissions,
            parent_cap,
            generation: 0,
            is_active: true,
        }
    }

    /// Check if a specific permission is granted
    pub fn has_permission(&self, scope: u64, flags: PermissionFlags) -> bool {
        if !self.is_active {
            return false;
        }

        self.permissions
            .iter()
            .any(|p| p.scope == scope && p.flags.contains(flags))
    }

    /// Attenuate this capability with a permission set
    pub fn attenuate(&self, perm: &Permission) -> Result<Capability> {
        if !self.is_active {
            return Err(CapabilityError::CapabilityRevoked(self.id));
        }

        let mut new_perms = Vec::new();
        for p in &self.permissions {
            let attenuated = p.attenuate(perm);
            if attenuated.flags.bits() != 0 {
                new_perms.push(attenuated);
            }
        }

        if new_perms.is_empty() {
            return Err(CapabilityError::AttenuationViolation(
                "no permissions remain after attenuation".into(),
            ));
        }

        Ok(Capability::new(
            self.id + 1, // New ID for attenuated capability
            self.owner,
            new_perms,
            Some(self.id),
        ))
    }

    /// Revoke this capability
    pub fn revoke(&mut self) {
        self.is_active = false;
    }

    /// Get a token for this capability
    pub fn token(&self) -> CapabilityToken {
        CapabilityToken::new(self.id, self.generation)
    }
}

/// Capability provider trait
pub trait CapabilityProvider {
    /// Request a capability
    fn request_capability(&self, scope: u64, flags: PermissionFlags) -> Result<Capability>;

    /// Verify a capability
    fn verify_capability(&self, cap: &Capability) -> Result<()>;

    /// Revoke a capability
    fn revoke_capability(&mut self, cap_id: u64) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_flags() {
        let read = PermissionFlags::read();
        let write = PermissionFlags::write();
        let combined = read | write;

        assert!(combined.contains(read));
        assert!(combined.contains(write));
    }

    #[test]
    fn test_capability_creation() {
        let perms = vec![Permission::new(1, PermissionFlags::read())];
        let cap = Capability::new(1, 100, perms, None);

        assert!(cap.has_permission(1, PermissionFlags::read()));
        assert!(!cap.has_permission(1, PermissionFlags::write()));
    }

    #[test]
    fn test_capability_attenuation() {
        let perms = vec![Permission::new(1, PermissionFlags::read() | PermissionFlags::write())];
        let cap = Capability::new(1, 100, perms, None);

        let attenuated = cap.attenuate(&Permission::new(1, PermissionFlags::read())).unwrap();
        assert!(attenuated.has_permission(1, PermissionFlags::read()));
        assert!(!attenuated.has_permission(1, PermissionFlags::write()));
    }

    #[test]
    fn test_capability_revocation() {
        let perms = vec![Permission::new(1, PermissionFlags::all())];
        let mut cap = Capability::new(1, 100, perms, None);

        cap.revoke();
        assert!(!cap.has_permission(1, PermissionFlags::read()));
    }
}
