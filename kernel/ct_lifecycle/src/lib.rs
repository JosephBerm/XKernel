// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 XKernal Contributors
//!
//! # ct-lifecycle - Cognitive Task Lifecycle Management
//!
//! This crate implements lifecycle management for Cognitive Tasks (CTs) within the XKernal
//! microkernel. It provides state machines, scheduling, memory management, and task prioritization.
//!
//! ## Modules
//!
//! - **lifecycle**: Task state machine and phase transitions
//! - **scheduler**: Priority queue-based fair scheduler
//! - **memory**: Arena allocator and memory pool management
//! - **types**: Core domain types and error handling

#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

extern crate alloc;

pub mod lifecycle;
pub mod scheduler;
pub mod memory;
pub mod types;
pub mod error;
pub mod ids;
pub mod dependency_dag;

// Re-export commonly used types
pub use lifecycle::{TaskStateMachine, TaskTransition, TaskState};
pub use scheduler::{PriorityScheduler, SchedulingError};
pub use memory::{ArenaAllocator, MemoryPool};
pub use types::{CognitiveTask, TaskPhase, Priority, TaskId};
pub use error::CsError;
pub use ids::CTID;
pub use dependency_dag::DependencyDag;
