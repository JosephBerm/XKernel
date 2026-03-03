// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 XKernal Contributors
//! Exception handling and fault dispatch for recovery

use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Exception handling errors
#[derive(Debug, Clone, Error)]
pub enum ExceptionError {
    /// Exception not found
    #[error("exception {0} not found")]
    NotFound(u64),
    /// No recovery available
    #[error("no recovery available for exception {0}")]
    NoRecovery(u64),
    /// Recovery failed
    #[error("recovery failed: {0}")]
    RecoveryFailed(alloc::string::String),
}

pub type Result<T> = core::result::Result<T, ExceptionError>;

/// Exception severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExceptionSeverity {
    /// Recoverable exception
    Recoverable = 0,
    /// Severe exception requiring intervention
    Severe = 1,
    /// Fatal exception
    Fatal = 2,
}

impl ExceptionSeverity {
    /// Get description
    pub fn description(&self) -> &'static str {
        match self {
            ExceptionSeverity::Recoverable => "Recoverable",
            ExceptionSeverity::Severe => "Severe",
            ExceptionSeverity::Fatal => "Fatal",
        }
    }
}

/// Exception data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Exception {
    /// Unique exception ID
    pub id: u64,
    /// Task that generated the exception
    pub task_id: u64,
    /// Exception type/code
    pub code: u32,
    /// Severity level
    pub severity: ExceptionSeverity,
    /// Exception message
    pub message: alloc::string::String,
    /// Stack trace
    pub stack_trace: Vec<u64>,
    /// Timestamp
    pub timestamp: u64,
}

impl Exception {
    /// Create a new exception
    pub fn new(id: u64, task_id: u64, code: u32, severity: ExceptionSeverity, message: alloc::string::String) -> Self {
        Self {
            id,
            task_id,
            code,
            severity,
            message,
            stack_trace: Vec::new(),
            timestamp: 0,
        }
    }

    /// Add a frame to the stack trace
    pub fn add_stack_frame(&mut self, frame: u64) {
        self.stack_trace.push(frame);
    }

    /// Check if this is a fatal exception
    pub fn is_fatal(&self) -> bool {
        self.severity == ExceptionSeverity::Fatal
    }
}

/// Exception handler trait
pub trait ExceptionHandler {
    /// Handle an exception
    fn handle(&mut self, exception: &Exception) -> Result<()>;

    /// Check if handler can handle this exception type
    fn can_handle(&self, exception_code: u32) -> bool;
}

/// Simple exception handler
#[derive(Debug)]
pub struct SimpleExceptionHandler {
    exception_code: u32,
    handled_count: u32,
}

impl SimpleExceptionHandler {
    /// Create a new exception handler
    pub fn new(exception_code: u32) -> Self {
        Self {
            exception_code,
            handled_count: 0,
        }
    }

    /// Get the number of exceptions handled
    pub fn handled_count(&self) -> u32 {
        self.handled_count
    }
}

impl ExceptionHandler for SimpleExceptionHandler {
    fn handle(&mut self, exception: &Exception) -> Result<()> {
        if !self.can_handle(exception.code) {
            return Err(ExceptionError::NotFound(exception.id));
        }
        self.handled_count += 1;
        Ok(())
    }

    fn can_handle(&self, code: u32) -> bool {
        code == self.exception_code
    }
}

/// Fault dispatcher for exception management
#[derive(Debug)]
pub struct FaultDispatcher {
    exceptions: Vec<Exception>,
    max_exceptions: usize,
}

impl FaultDispatcher {
    /// Create a new fault dispatcher
    pub fn new(max_exceptions: usize) -> Self {
        Self {
            exceptions: Vec::with_capacity(max_exceptions),
            max_exceptions,
        }
    }

    /// Dispatch an exception
    pub fn dispatch(&mut self, exception: Exception) -> Result<u64> {
        if self.exceptions.len() >= self.max_exceptions {
            return Err(ExceptionError::RecoveryFailed(
                "exception buffer full".into(),
            ));
        }

        let id = exception.id;
        self.exceptions.push(exception);
        Ok(id)
    }

    /// Get the number of pending exceptions
    pub fn pending_count(&self) -> usize {
        self.exceptions.len()
    }

    /// Retrieve the next exception
    pub fn next_exception(&mut self) -> Option<Exception> {
        if self.exceptions.is_empty() {
            None
        } else {
            Some(self.exceptions.remove(0))
        }
    }

    /// Find an exception by ID
    pub fn find_exception(&self, id: u64) -> Option<&Exception> {
        self.exceptions.iter().find(|e| e.id == id)
    }

    /// Clear all exceptions
    pub fn clear(&mut self) {
        self.exceptions.clear();
    }

    /// Get fatal exception count
    pub fn fatal_count(&self) -> usize {
        self.exceptions.iter().filter(|e| e.is_fatal()).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exception_creation() {
        let exc = Exception::new(1, 1, 100, ExceptionSeverity::Recoverable, "test".into());
        assert_eq!(exc.id, 1);
        assert_eq!(exc.code, 100);
        assert!(!exc.is_fatal());
    }

    #[test]
    fn test_exception_stack_trace() {
        let mut exc = Exception::new(1, 1, 100, ExceptionSeverity::Recoverable, "test".into());
        exc.add_stack_frame(0x1000);
        exc.add_stack_frame(0x2000);

        assert_eq!(exc.stack_trace.len(), 2);
    }

    #[test]
    fn test_exception_handler() {
        let mut handler = SimpleExceptionHandler::new(100);
        let exc = Exception::new(1, 1, 100, ExceptionSeverity::Recoverable, "test".into());

        assert!(handler.handle(&exc).is_ok());
        assert_eq!(handler.handled_count(), 1);
    }

    #[test]
    fn test_fault_dispatcher() {
        let mut dispatcher = FaultDispatcher::new(10);
        let exc = Exception::new(1, 1, 100, ExceptionSeverity::Recoverable, "test".into());

        dispatcher.dispatch(exc).unwrap();
        assert_eq!(dispatcher.pending_count(), 1);

        let retrieved = dispatcher.next_exception();
        assert!(retrieved.is_some());
    }

    #[test]
    fn test_fatal_exception_count() {
        let mut dispatcher = FaultDispatcher::new(10);

        let exc1 = Exception::new(1, 1, 100, ExceptionSeverity::Fatal, "fatal".into());
        let exc2 = Exception::new(2, 2, 101, ExceptionSeverity::Recoverable, "recoverable".into());

        dispatcher.dispatch(exc1).unwrap();
        dispatcher.dispatch(exc2).unwrap();

        assert_eq!(dispatcher.fatal_count(), 1);
    }
}
