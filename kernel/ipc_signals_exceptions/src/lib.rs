// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 XKernal Contributors
//!
//! # ipc-signals-exceptions - Inter-Process Communication, Signal Delivery, and Fault Recovery
//!
//! This crate implements IPC channels, signal delivery mechanisms, exception handling,
//! and checkpointing infrastructure for fault tolerance in the XKernal microkernel.
//!
//! ## Modules
//!
//! - **ipc**: Channels, publish-subscribe, and shared context
//! - **signals**: Signal delivery and handler registration
//! - **exceptions**: Fault dispatch and recovery mechanisms
//! - **checkpoint**: Snapshot, restore, and CRDT-based synchronization

#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

extern crate alloc;

pub mod ipc;
pub mod signals;
pub mod exceptions;
pub mod checkpoint;

// Re-export commonly used types
pub use ipc::{Channel, Message, IpcEndpoint};
pub use signals::{Signal, SignalHandler, SignalDispatcher};
pub use exceptions::{Exception, ExceptionHandler, FaultDispatcher};
pub use checkpoint::{Checkpoint, CheckpointProvider, SnapshotFormat};
