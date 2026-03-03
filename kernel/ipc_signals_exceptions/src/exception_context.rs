// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! # Exception Context Capture
//!
//! This module defines the comprehensive context capture for exceptions.
//! When an exception occurs, the kernel captures the full execution context
//! (registers, memory, tool state, IPC state) for analysis and recovery.
//!
//! ## Context Snapshot
//!
//! The ExceptionContext includes:
//! - **RegisterSnapshot**: Full x86_64 register state (18 registers)
//! - **WorkingMemorySnapshot**: Current memory state
//! - **ToolCallContext**: Current tool invocation details
//! - **IpcState**: Active channels and pending messages
//! - **CheckpointReference**: Associated checkpoint for rollback
//! - **Timestamp**: When the exception occurred
//!
//! ## References
//!
//! - Engineering Plan § 6.2 (Exception System)
//! - Engineering Plan § 6.5 (Context Capture)

use crate::ids::{CheckpointID, ExceptionID};
use crate::exception::CognitiveException;
use cs_ct_lifecycle::CTID;
use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

/// x86_64 register snapshot.
///
/// Captures the 18 major x86_64 registers for debugging and recovery.
/// Helps analyze execution state at the point of exception.
///
/// See Engineering Plan § 6.5 (Context Capture)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegisterSnapshot {
    /// RAX general purpose register
    pub rax: u64,
    /// RBX general purpose register
    pub rbx: u64,
    /// RCX general purpose register
    pub rcx: u64,
    /// RDX general purpose register
    pub rdx: u64,
    /// RSI source index register
    pub rsi: u64,
    /// RDI destination index register
    pub rdi: u64,
    /// RBP base pointer register
    pub rbp: u64,
    /// RSP stack pointer register
    pub rsp: u64,
    /// R8 general purpose register
    pub r8: u64,
    /// R9 general purpose register
    pub r9: u64,
    /// R10 general purpose register
    pub r10: u64,
    /// R11 general purpose register
    pub r11: u64,
    /// R12 general purpose register
    pub r12: u64,
    /// R13 general purpose register
    pub r13: u64,
    /// R14 general purpose register
    pub r14: u64,
    /// R15 general purpose register
    pub r15: u64,
    /// RIP instruction pointer register
    pub rip: u64,
    /// RFLAGS flags register
    pub rflags: u64,
}

impl RegisterSnapshot {
    /// Create a new register snapshot.
    pub fn new(
        rax: u64, rbx: u64, rcx: u64, rdx: u64, rsi: u64, rdi: u64, rbp: u64, rsp: u64,
        r8: u64, r9: u64, r10: u64, r11: u64, r12: u64, r13: u64, r14: u64, r15: u64,
        rip: u64, rflags: u64,
    ) -> Self {
        Self {
            rax, rbx, rcx, rdx, rsi, rdi, rbp, rsp,
            r8, r9, r10, r11, r12, r13, r14, r15,
            rip, rflags,
        }
    }

    /// Create a zero-initialized register snapshot (for testing).
    pub fn zeroed() -> Self {
        Self {
            rax: 0, rbx: 0, rcx: 0, rdx: 0, rsi: 0, rdi: 0, rbp: 0, rsp: 0,
            r8: 0, r9: 0, r10: 0, r11: 0, r12: 0, r13: 0, r14: 0, r15: 0,
            rip: 0, rflags: 0,
        }
    }
}

/// Working memory snapshot.
///
/// Captures the current state of the CT's working memory and buffers.
/// Used to understand memory pressure and potential exhaustion causes.
///
/// See Engineering Plan § 6.5 (Context Capture)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkingMemorySnapshot {
    /// Current memory usage in bytes
    pub current_usage_bytes: u64,
    /// Maximum memory capacity in bytes
    pub max_capacity_bytes: u64,
    /// Memory pressure level (0-100%)
    pub pressure_percent: u32,
    /// Heap allocation count
    pub heap_allocations: u32,
    /// Stack depth estimate
    pub stack_depth: u32,
    /// Optional detailed memory breakdown
    pub breakdown: Vec<(String, u64)>,
}

impl WorkingMemorySnapshot {
    /// Create a new working memory snapshot.
    pub fn new(
        current_usage_bytes: u64,
        max_capacity_bytes: u64,
        heap_allocations: u32,
        stack_depth: u32,
    ) -> Self {
        let pressure_percent = if max_capacity_bytes > 0 {
            ((current_usage_bytes as f64 / max_capacity_bytes as f64) * 100.0) as u32
        } else {
            100
        };

        Self {
            current_usage_bytes,
            max_capacity_bytes,
            pressure_percent,
            heap_allocations,
            stack_depth,
            breakdown: Vec::new(),
        }
    }
}

/// Tool call context snapshot.
///
/// Captures information about the tool call that was executing when
/// the exception occurred.
///
/// See Engineering Plan § 6.5 (Context Capture)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolCallContext {
    /// Tool identifier
    pub tool_id: String,
    /// Tool name (human-readable)
    pub tool_name: String,
    /// Parameters passed to the tool
    pub parameters: String,
    /// Start time of tool invocation (Unix epoch milliseconds)
    pub start_time_ms: u64,
    /// Duration so far in milliseconds
    pub duration_ms: u64,
    /// Whether the tool was waiting for I/O
    pub waiting_for_io: bool,
    /// Optional timeout for the tool
    pub timeout_ms: Option<u64>,
}

impl ToolCallContext {
    /// Create a new tool call context.
    pub fn new(
        tool_id: String,
        tool_name: String,
        parameters: String,
        start_time_ms: u64,
        waiting_for_io: bool,
    ) -> Self {
        Self {
            tool_id,
            tool_name,
            parameters,
            start_time_ms,
            duration_ms: 0,
            waiting_for_io,
            timeout_ms: None,
        }
    }

    /// Update the duration based on current time.
    pub fn update_duration(&mut self, current_time_ms: u64) {
        if current_time_ms >= self.start_time_ms {
            self.duration_ms = current_time_ms - self.start_time_ms;
        }
    }

    /// Check if tool has exceeded its timeout.
    pub fn is_timeout(&self) -> bool {
        if let Some(timeout_ms) = self.timeout_ms {
            self.duration_ms >= timeout_ms
        } else {
            false
        }
    }
}

/// IPC state snapshot.
///
/// Captures the state of Inter-Process Communication at the time of exception.
/// Includes active channels, pending messages, and send/receive buffers.
///
/// See Engineering Plan § 6.5 (Context Capture)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IpcState {
    /// Active channel count
    pub active_channels: u32,
    /// Pending messages in all channels
    pub pending_messages: u32,
    /// Send buffer usage in bytes
    pub send_buffer_bytes: u64,
    /// Receive buffer usage in bytes
    pub recv_buffer_bytes: u64,
    /// List of blocked channel operations
    pub blocked_operations: Vec<String>,
    /// Last message sent/received
    pub last_message_info: Option<String>,
}

impl IpcState {
    /// Create a new IPC state snapshot.
    pub fn new(
        active_channels: u32,
        pending_messages: u32,
        send_buffer_bytes: u64,
        recv_buffer_bytes: u64,
    ) -> Self {
        Self {
            active_channels,
            pending_messages,
            send_buffer_bytes,
            recv_buffer_bytes,
            blocked_operations: Vec::new(),
            last_message_info: None,
        }
    }

    /// Add a blocked operation to the list.
    pub fn add_blocked_operation(&mut self, operation: String) {
        self.blocked_operations.push(operation);
    }

    /// Set the last message info.
    pub fn set_last_message(&mut self, info: String) {
        self.last_message_info = Some(info);
    }
}

/// Complete exception context captured at the point of exception.
///
/// Provides comprehensive information for debugging and recovery decisions.
/// Includes full register state, memory snapshot, tool context, and IPC state.
///
/// See Engineering Plan § 6.2 (Exception System) & § 6.5 (Context Capture)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExceptionContext {
    /// Unique exception identifier
    pub exception_id: ExceptionID,

    /// CT that raised this exception
    pub ct_id: CTID,

    /// The exception itself
    pub exception: CognitiveException,

    /// Timestamp when exception occurred (Unix epoch milliseconds)
    pub timestamp_ms: u64,

    /// Register state at exception point
    pub registers: RegisterSnapshot,

    /// Working memory state
    pub memory: WorkingMemorySnapshot,

    /// Tool call context (if exception occurred during tool execution)
    pub tool_context: Option<ToolCallContext>,

    /// IPC subsystem state
    pub ipc_state: IpcState,

    /// Associated checkpoint for rollback (if available)
    pub checkpoint_id: Option<CheckpointID>,

    /// Stack trace or execution context breadcrumbs
    pub stack_trace: Vec<String>,

    /// Number of times this exception has been handled
    pub retry_count: u32,

    /// Whether the handler is currently processing this exception
    pub in_handler: bool,
}

impl ExceptionContext {
    /// Create a new exception context.
    pub fn new(
        exception_id: ExceptionID,
        ct_id: CTID,
        exception: CognitiveException,
        timestamp_ms: u64,
        registers: RegisterSnapshot,
        memory: WorkingMemorySnapshot,
        ipc_state: IpcState,
    ) -> Self {
        Self {
            exception_id,
            ct_id,
            exception,
            timestamp_ms,
            registers,
            memory,
            tool_context: None,
            ipc_state,
            checkpoint_id: None,
            stack_trace: Vec::new(),
            retry_count: 0,
            in_handler: false,
        }
    }

    /// Set the tool context for this exception.
    pub fn set_tool_context(&mut self, context: ToolCallContext) {
        self.tool_context = Some(context);
    }

    /// Set the checkpoint reference.
    pub fn set_checkpoint(&mut self, checkpoint_id: CheckpointID) {
        self.checkpoint_id = Some(checkpoint_id);
    }

    /// Add a stack trace entry.
    pub fn add_stack_entry(&mut self, entry: String) {
        self.stack_trace.push(entry);
    }

    /// Increment retry count.
    pub fn increment_retry(&mut self) {
        self.retry_count = self.retry_count.saturating_add(1);
    }

    /// Mark as being handled.
    pub fn mark_in_handler(&mut self) {
        self.in_handler = true;
    }

    /// Mark as no longer being handled.
    pub fn mark_handler_done(&mut self) {
        self.in_handler = false;
    }

    /// Get the exception severity.
    pub fn severity(&self) -> crate::exception::ExceptionSeverity {
        self.exception.severity()
    }

    /// Check if exception is recoverable.
    pub fn is_recoverable(&self) -> bool {
        self.exception.is_recoverable()
    }

    /// Get size estimate of context in bytes (for memory tracking).
    pub fn size_estimate_bytes(&self) -> u64 {
        let stack_trace_size: u64 = self.stack_trace.iter()
            .map(|s| s.len() as u64)
            .sum();
        let breakdown_size: u64 = self.memory.breakdown.iter()
            .map(|(k, v)| k.len() as u64 + 8)
            .sum();
        let blocked_ops_size: u64 = self.ipc_state.blocked_operations.iter()
            .map(|s| s.len() as u64)
            .sum();

        128 + // Structure overhead
        stack_trace_size +
        breakdown_size +
        blocked_ops_size +
        self.tool_context.as_ref()
            .map(|tc| tc.tool_id.len() as u64 + tc.tool_name.len() as u64 + tc.parameters.len() as u64)
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::string::ToString;

    #[test]
    fn test_register_snapshot_new() {
        let regs = RegisterSnapshot::new(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18);
        assert_eq!(regs.rax, 1);
        assert_eq!(regs.rbx, 2);
        assert_eq!(regs.rip, 17);
        assert_eq!(regs.rflags, 18);
    }

    #[test]
    fn test_register_snapshot_zeroed() {
        let regs = RegisterSnapshot::zeroed();
        assert_eq!(regs.rax, 0);
        assert_eq!(regs.rip, 0);
    }

    #[test]
    fn test_working_memory_snapshot_pressure() {
        let mem = WorkingMemorySnapshot::new(512, 1024, 10, 5);
        assert_eq!(mem.current_usage_bytes, 512);
        assert_eq!(mem.max_capacity_bytes, 1024);
        assert_eq!(mem.pressure_percent, 50);
    }

    #[test]
    fn test_working_memory_snapshot_full() {
        let mem = WorkingMemorySnapshot::new(1024, 1024, 10, 5);
        assert_eq!(mem.pressure_percent, 100);
    }

    #[test]
    fn test_tool_call_context_new() {
        let tool = ToolCallContext::new(
            "tool1".to_string(),
            "My Tool".to_string(),
            "params".to_string(),
            5000,
            false,
        );
        assert_eq!(tool.tool_id, "tool1");
        assert_eq!(tool.start_time_ms, 5000);
        assert_eq!(tool.duration_ms, 0);
        assert!(!tool.waiting_for_io);
    }

    #[test]
    fn test_tool_call_context_update_duration() {
        let mut tool = ToolCallContext::new(
            "tool1".to_string(),
            "My Tool".to_string(),
            "params".to_string(),
            5000,
            false,
        );
        tool.update_duration(5500);
        assert_eq!(tool.duration_ms, 500);
    }

    #[test]
    fn test_tool_call_context_timeout() {
        let mut tool = ToolCallContext::new(
            "tool1".to_string(),
            "My Tool".to_string(),
            "params".to_string(),
            5000,
            false,
        );
        tool.timeout_ms = Some(1000);
        tool.duration_ms = 1500;
        assert!(tool.is_timeout());
    }

    #[test]
    fn test_ipc_state_new() {
        let ipc = IpcState::new(5, 10, 1024, 2048);
        assert_eq!(ipc.active_channels, 5);
        assert_eq!(ipc.pending_messages, 10);
    }

    #[test]
    fn test_ipc_state_add_blocked_operation() {
        let mut ipc = IpcState::new(5, 10, 1024, 2048);
        ipc.add_blocked_operation("send_timeout".to_string());
        assert_eq!(ipc.blocked_operations.len(), 1);
    }

    #[test]
    fn test_exception_context_new() {
        let exc = CognitiveException::ToolCallFailed(
            crate::exception::ToolFailureContext::new(
                "tool".to_string(),
                "error".to_string(),
                true,
                5000,
            ),
        );
        let regs = RegisterSnapshot::zeroed();
        let mem = WorkingMemorySnapshot::new(512, 1024, 10, 5);
        let ipc = IpcState::new(5, 10, 1024, 2048);

        let ctx = ExceptionContext::new(
            ExceptionID::new(),
            CTID::new(),
            exc,
            5000,
            regs,
            mem,
            ipc,
        );

        assert_eq!(ctx.timestamp_ms, 5000);
        assert!(!ctx.in_handler);
        assert_eq!(ctx.retry_count, 0);
    }

    #[test]
    fn test_exception_context_set_checkpoint() {
        let exc = CognitiveException::ToolCallFailed(
            crate::exception::ToolFailureContext::new(
                "tool".to_string(),
                "error".to_string(),
                true,
                5000,
            ),
        );
        let regs = RegisterSnapshot::zeroed();
        let mem = WorkingMemorySnapshot::new(512, 1024, 10, 5);
        let ipc = IpcState::new(5, 10, 1024, 2048);

        let mut ctx = ExceptionContext::new(
            ExceptionID::new(),
            CTID::new(),
            exc,
            5000,
            regs,
            mem,
            ipc,
        );

        let ckpt_id = CheckpointID::new();
        ctx.set_checkpoint(ckpt_id);
        assert!(ctx.checkpoint_id.is_some());
    }

    #[test]
    fn test_exception_context_mark_handler() {
        let exc = CognitiveException::ToolCallFailed(
            crate::exception::ToolFailureContext::new(
                "tool".to_string(),
                "error".to_string(),
                true,
                5000,
            ),
        );
        let regs = RegisterSnapshot::zeroed();
        let mem = WorkingMemorySnapshot::new(512, 1024, 10, 5);
        let ipc = IpcState::new(5, 10, 1024, 2048);

        let mut ctx = ExceptionContext::new(
            ExceptionID::new(),
            CTID::new(),
            exc,
            5000,
            regs,
            mem,
            ipc,
        );

        assert!(!ctx.in_handler);
        ctx.mark_in_handler();
        assert!(ctx.in_handler);
        ctx.mark_handler_done();
        assert!(!ctx.in_handler);
    }

    #[test]
    fn test_exception_context_increment_retry() {
        let exc = CognitiveException::ToolCallFailed(
            crate::exception::ToolFailureContext::new(
                "tool".to_string(),
                "error".to_string(),
                true,
                5000,
            ),
        );
        let regs = RegisterSnapshot::zeroed();
        let mem = WorkingMemorySnapshot::new(512, 1024, 10, 5);
        let ipc = IpcState::new(5, 10, 1024, 2048);

        let mut ctx = ExceptionContext::new(
            ExceptionID::new(),
            CTID::new(),
            exc,
            5000,
            regs,
            mem,
            ipc,
        );

        assert_eq!(ctx.retry_count, 0);
        ctx.increment_retry();
        assert_eq!(ctx.retry_count, 1);
    }
}
