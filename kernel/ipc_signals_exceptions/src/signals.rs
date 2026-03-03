// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 XKernal Contributors
//! Signal delivery and handler registration

use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Signal errors
#[derive(Debug, Clone, Error)]
pub enum SignalError {
    /// Signal not found
    #[error("signal {0} not found")]
    SignalNotFound(u32),
    /// Handler not registered
    #[error("no handler for signal {0}")]
    NoHandler(u32),
    /// Delivery failed
    #[error("signal delivery failed: {0}")]
    DeliveryFailed(alloc::string::String),
}

pub type Result<T> = core::result::Result<T, SignalError>;

/// Signal types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Signal {
    /// Terminate signal
    Terminate = 1,
    /// Interrupt signal
    Interrupt = 2,
    /// Kill signal (unblockable)
    Kill = 3,
    /// Checkpoint signal
    Checkpoint = 4,
    /// Suspend signal
    Suspend = 5,
    /// Resume signal
    Resume = 6,
    /// User-defined signal 1
    User1 = 7,
    /// User-defined signal 2
    User2 = 8,
}

impl Signal {
    /// Get the signal number
    pub fn number(&self) -> u32 {
        *self as u32
    }

    /// Check if signal is blockable
    pub fn is_blockable(&self) -> bool {
        !matches!(self, Signal::Kill)
    }

    /// Get signal description
    pub fn description(&self) -> &'static str {
        match self {
            Signal::Terminate => "Terminate",
            Signal::Interrupt => "Interrupt",
            Signal::Kill => "Kill",
            Signal::Checkpoint => "Checkpoint",
            Signal::Suspend => "Suspend",
            Signal::Resume => "Resume",
            Signal::User1 => "User1",
            Signal::User2 => "User2",
        }
    }
}

/// Signal handler trait
pub trait SignalHandler {
    /// Handle a signal
    fn handle(&mut self, signal: Signal) -> Result<()>;

    /// Check if handler is still active
    fn is_active(&self) -> bool;
}

/// Simple signal handler implementation
#[derive(Debug)]
pub struct SimpleSignalHandler {
    signal: Signal,
    active: bool,
    invocation_count: u32,
}

impl SimpleSignalHandler {
    /// Create a new signal handler
    pub fn new(signal: Signal) -> Self {
        Self {
            signal,
            active: true,
            invocation_count: 0,
        }
    }

    /// Get the number of times this handler was invoked
    pub fn invocation_count(&self) -> u32 {
        self.invocation_count
    }
}

impl SignalHandler for SimpleSignalHandler {
    fn handle(&mut self, signal: Signal) -> Result<()> {
        if !self.active {
            return Err(SignalError::NoHandler(signal.number()));
        }

        if signal != self.signal {
            return Err(SignalError::SignalNotFound(signal.number()));
        }

        self.invocation_count += 1;
        Ok(())
    }

    fn is_active(&self) -> bool {
        self.active
    }
}

/// Signal dispatcher for managing signal handlers
#[derive(Debug)]
pub struct SignalDispatcher {
    handlers: Vec<(Signal, u64)>, // (signal, handler_id)
    blocked_signals: u32,
}

impl SignalDispatcher {
    /// Create a new signal dispatcher
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
            blocked_signals: 0,
        }
    }

    /// Register a handler for a signal
    pub fn register_handler(&mut self, signal: Signal, handler_id: u64) {
        self.handlers.push((signal, handler_id));
    }

    /// Unregister a handler
    pub fn unregister_handler(&mut self, handler_id: u64) -> Result<()> {
        let initial_len = self.handlers.len();
        self.handlers.retain(|(_, id)| *id != handler_id);

        if self.handlers.len() < initial_len {
            Ok(())
        } else {
            Err(SignalError::NoHandler(0))
        }
    }

    /// Dispatch a signal
    pub fn dispatch(&self, signal: Signal) -> Result<u64> {
        if self.is_blocked(signal) {
            return Err(SignalError::DeliveryFailed("signal blocked".into()));
        }

        let handler = self
            .handlers
            .iter()
            .find(|(s, _)| *s == signal)
            .map(|(_, h)| *h);

        handler.ok_or(SignalError::NoHandler(signal.number()))
    }

    /// Block a signal
    pub fn block_signal(&mut self, signal: Signal) {
        self.blocked_signals |= 1 << signal.number();
    }

    /// Unblock a signal
    pub fn unblock_signal(&mut self, signal: Signal) {
        self.blocked_signals &= !(1 << signal.number());
    }

    /// Check if a signal is blocked
    pub fn is_blocked(&self, signal: Signal) -> bool {
        (self.blocked_signals & (1 << signal.number())) != 0
    }

    /// Get all registered signal handlers
    pub fn handlers(&self) -> &[(Signal, u64)] {
        &self.handlers
    }
}

impl Default for SignalDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_numbers() {
        assert_eq!(Signal::Terminate.number(), 1);
        assert_eq!(Signal::Kill.number(), 3);
    }

    #[test]
    fn test_signal_blockable() {
        assert!(Signal::Terminate.is_blockable());
        assert!(!Signal::Kill.is_blockable());
    }

    #[test]
    fn test_signal_handler() {
        let mut handler = SimpleSignalHandler::new(Signal::Terminate);
        assert!(handler.handle(Signal::Terminate).is_ok());
        assert_eq!(handler.invocation_count(), 1);
    }

    #[test]
    fn test_signal_dispatcher() {
        let mut dispatcher = SignalDispatcher::new();
        dispatcher.register_handler(Signal::Terminate, 1);

        let handler_id = dispatcher.dispatch(Signal::Terminate).unwrap();
        assert_eq!(handler_id, 1);
    }

    #[test]
    fn test_signal_blocking() {
        let mut dispatcher = SignalDispatcher::new();
        dispatcher.register_handler(Signal::Interrupt, 1);

        dispatcher.block_signal(Signal::Interrupt);
        assert!(dispatcher.dispatch(Signal::Interrupt).is_err());

        dispatcher.unblock_signal(Signal::Interrupt);
        assert!(dispatcher.dispatch(Signal::Interrupt).is_ok());
    }
}
