// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 XKernal Contributors
//!
//! # capability-engine - Capability-based Security Microkernel
//!
//! This crate implements the core capability-based access control mechanism
//! for the XKernal microkernel, enabling fine-grained privilege delegation
//! and mandatory policy enforcement.
//!
//! ## Modules
//!
//! - **capability**: Core capability tokens and permission management
//! - **delegation**: Capability delegation chains and attenuation rules
//! - **policy**: Mandatory policy enforcement and Cognitive Policy Language
//! - **verification**: Cryptographic proof checking and validation

#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

extern crate alloc;

pub mod capability;
pub mod delegation;
pub mod policy;
pub mod verification;

// Re-export commonly used types
pub use capability::{Capability, CapabilityToken, Permission, PermissionFlags};
pub use delegation::{DelegationChain, AttenuationRule, RevocationPolicy};
pub use policy::{PolicyEngine, MandatoryPolicy};
pub use verification::{Proof, VerificationResult};
