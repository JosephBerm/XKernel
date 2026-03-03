// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.

//! # Framework Syscall Binding Layer
//!
//! Defines how adapters invoke the 22 CSCI syscalls for kernel operations.
//! This layer abstracts syscall invocation and provides typed interfaces to the kernel.
//!
//! Sec 3.5: CSCI Syscall Interface
//! Sec 5.2: Framework Syscall Binding

use alloc::{string::String, vec::Vec};
use crate::error::AdapterError;
use crate::AdapterResult;

/// Syscall identifiers for CSCI operations.
/// Sec 3.5: CSCI Syscall Enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CsciSyscallId {
    /// mem_write - Write to semantic memory
    MemWrite = 1,
    /// mem_read - Read from semantic memory
    MemRead = 2,
    /// task_spawn - Spawn a cognitive task
    TaskSpawn = 3,
    /// task_wait - Wait for task completion
    TaskWait = 4,
    /// task_kill - Terminate a task
    TaskKill = 5,
    /// tool_bind - Bind a tool to agent
    ToolBind = 6,
    /// tool_invoke - Invoke a tool
    ToolInvoke = 7,
    /// channel_create - Create IPC channel
    ChannelCreate = 8,
    /// channel_send - Send message on channel
    ChannelSend = 9,
    /// channel_recv - Receive message from channel
    ChannelRecv = 10,
    /// cap_grant - Grant capability to entity
    CapGrant = 11,
    /// cap_revoke - Revoke capability from entity
    CapRevoke = 12,
    /// signal_install - Install signal handler
    SignalInstall = 13,
    /// signal_raise - Raise a signal
    SignalRaise = 14,
    /// exception_throw - Throw exception
    ExceptionThrow = 15,
    /// exception_catch - Register exception handler
    ExceptionCatch = 16,
    /// timer_set - Set a timer
    TimerSet = 17,
    /// timer_cancel - Cancel a timer
    TimerCancel = 18,
    /// debug_trace - Write debug trace
    DebugTrace = 19,
    /// profiler_sample - Record profiler sample
    ProfilerSample = 20,
    /// agent_create - Create agent entity
    AgentCreate = 21,
    /// agent_destroy - Destroy agent entity
    AgentDestroy = 22,
}

impl CsciSyscallId {
    /// Returns syscall ID as u32.
    pub fn as_u32(&self) -> u32 {
        *self as u32
    }

    /// Returns string name of syscall.
    pub fn name(&self) -> &'static str {
        match self {
            CsciSyscallId::MemWrite => "mem_write",
            CsciSyscallId::MemRead => "mem_read",
            CsciSyscallId::TaskSpawn => "task_spawn",
            CsciSyscallId::TaskWait => "task_wait",
            CsciSyscallId::TaskKill => "task_kill",
            CsciSyscallId::ToolBind => "tool_bind",
            CsciSyscallId::ToolInvoke => "tool_invoke",
            CsciSyscallId::ChannelCreate => "channel_create",
            CsciSyscallId::ChannelSend => "channel_send",
            CsciSyscallId::ChannelRecv => "channel_recv",
            CsciSyscallId::CapGrant => "cap_grant",
            CsciSyscallId::CapRevoke => "cap_revoke",
            CsciSyscallId::SignalInstall => "signal_install",
            CsciSyscallId::SignalRaise => "signal_raise",
            CsciSyscallId::ExceptionThrow => "exception_throw",
            CsciSyscallId::ExceptionCatch => "exception_catch",
            CsciSyscallId::TimerSet => "timer_set",
            CsciSyscallId::TimerCancel => "timer_cancel",
            CsciSyscallId::DebugTrace => "debug_trace",
            CsciSyscallId::ProfilerSample => "profiler_sample",
            CsciSyscallId::AgentCreate => "agent_create",
            CsciSyscallId::AgentDestroy => "agent_destroy",
        }
    }
}

/// Syscall request envelope.
/// Sec 5.2: Syscall Request Structure
#[derive(Debug, Clone)]
pub struct SyscallRequest {
    /// Syscall identifier
    pub syscall_id: CsciSyscallId,
    /// Request identifier for correlation
    pub request_id: String,
    /// Argument buffer (serialized)
    pub args: Vec<u8>,
    /// Timeout in milliseconds
    pub timeout_ms: u64,
}

impl SyscallRequest {
    /// Creates a new syscall request.
    pub fn new(
        syscall_id: CsciSyscallId,
        request_id: String,
        args: Vec<u8>,
        timeout_ms: u64,
    ) -> Self {
        SyscallRequest {
            syscall_id,
            request_id,
            args,
            timeout_ms,
        }
    }
}

/// Syscall response envelope.
/// Sec 5.2: Syscall Response Structure
#[derive(Debug, Clone)]
pub struct SyscallResponse {
    /// Request identifier
    pub request_id: String,
    /// Success indicator
    pub success: bool,
    /// Response data (serialized)
    pub result: Vec<u8>,
    /// Error message if failed
    pub error_message: String,
    /// Timestamp in milliseconds
    pub timestamp_ms: u64,
}

impl SyscallResponse {
    /// Creates a successful response.
    pub fn success(request_id: String, result: Vec<u8>, timestamp_ms: u64) -> Self {
        SyscallResponse {
            request_id,
            success: true,
            result,
            error_message: String::new(),
            timestamp_ms,
        }
    }

    /// Creates a failed response.
    pub fn error(request_id: String, error: String, timestamp_ms: u64) -> Self {
        SyscallResponse {
            request_id,
            success: false,
            result: Vec::new(),
            error_message: error,
            timestamp_ms,
        }
    }
}

/// Adapter syscall binding - abstracts kernel syscall invocation.
/// Sec 5.2: Framework Syscall Binding Interface
pub trait SyscallBinding {
    /// Invokes a syscall and returns response.
    /// Sec 5.2: Syscall Invocation
    fn invoke_syscall(&self, request: SyscallRequest) -> AdapterResult<SyscallResponse>;

    /// Spawns a cognitive task via task_spawn syscall.
    /// Sec 5.2: Task Spawn Binding
    fn spawn_task(&self, agent_id: &str, task_name: &str, timeout_ms: u64)
        -> AdapterResult<String>;

    /// Waits for task completion via task_wait syscall.
    /// Sec 5.2: Task Wait Binding
    fn wait_task(&self, task_id: &str, timeout_ms: u64) -> AdapterResult<String>;

    /// Binds a tool via tool_bind syscall.
    /// Sec 5.2: Tool Bind Binding
    fn bind_tool(&self, agent_id: &str, tool_id: &str) -> AdapterResult<()>;

    /// Invokes a tool via tool_invoke syscall.
    /// Sec 5.2: Tool Invoke Binding
    fn invoke_tool(&self, tool_id: &str, args: &str) -> AdapterResult<String>;

    /// Creates an IPC channel via channel_create syscall.
    /// Sec 5.2: Channel Create Binding
    fn create_channel(&self, channel_type: &str) -> AdapterResult<String>;

    /// Sends message on channel via channel_send syscall.
    /// Sec 5.2: Channel Send Binding
    fn send_channel(&self, channel_id: &str, message: &str) -> AdapterResult<()>;

    /// Grants capability via cap_grant syscall.
    /// Sec 5.2: Capability Grant Binding
    fn grant_capability(&self, entity_id: &str, cap_id: &str) -> AdapterResult<()>;

    /// Revokes capability via cap_revoke syscall.
    /// Sec 5.2: Capability Revoke Binding
    fn revoke_capability(&self, entity_id: &str, cap_id: &str) -> AdapterResult<()>;

    /// Writes to memory via mem_write syscall.
    /// Sec 5.2: Memory Write Binding
    fn write_memory(&self, memory_id: &str, data: &str) -> AdapterResult<()>;

    /// Reads from memory via mem_read syscall.
    /// Sec 5.2: Memory Read Binding
    fn read_memory(&self, memory_id: &str) -> AdapterResult<String>;
}

/// Mock syscall binding for testing.
/// Sec 5.2: Mock Syscall Binding for Tests
#[derive(Debug, Clone)]
pub struct MockSyscallBinding {
    /// Mock responses stored by request_id
    responses: alloc::collections::BTreeMap<String, SyscallResponse>,
    /// Recorded invocations
    invocations: Vec<SyscallRequest>,
}

impl MockSyscallBinding {
    /// Creates a new mock binding.
    pub fn new() -> Self {
        MockSyscallBinding {
            responses: alloc::collections::BTreeMap::new(),
            invocations: Vec::new(),
        }
    }

    /// Registers a mock response.
    pub fn register_response(&mut self, request_id: String, response: SyscallResponse) {
        self.responses.insert(request_id, response);
    }

    /// Gets recorded invocations.
    pub fn invocations(&self) -> &[SyscallRequest] {
        &self.invocations
    }

    /// Gets invocation count for specific syscall.
    pub fn invocation_count(&self, syscall_id: CsciSyscallId) -> usize {
        self.invocations
            .iter()
            .filter(|r| r.syscall_id == syscall_id)
            .count()
    }
}

impl Default for MockSyscallBinding {
    fn default() -> Self {
        Self::new()
    }
}

impl SyscallBinding for MockSyscallBinding {
    fn invoke_syscall(&self, request: SyscallRequest) -> AdapterResult<SyscallResponse> {
        if let Some(response) = self.responses.get(&request.request_id) {
            Ok(response.clone())
        } else {
            // Default success response
            Ok(SyscallResponse::success(
                request.request_id,
                Vec::new(),
                0,
            ))
        }
    }

    fn spawn_task(
        &self,
        agent_id: &str,
        task_name: &str,
        timeout_ms: u64,
    ) -> AdapterResult<String> {
        let request = SyscallRequest::new(
            CsciSyscallId::TaskSpawn,
            alloc::format!("spawn-{}", agent_id),
            alloc::format!("{},{}", agent_id, task_name).into_bytes(),
            timeout_ms,
        );
        self.invoke_syscall(request)?;
        Ok(alloc::format!("task-{}", agent_id))
    }

    fn wait_task(&self, task_id: &str, timeout_ms: u64) -> AdapterResult<String> {
        let request = SyscallRequest::new(
            CsciSyscallId::TaskWait,
            alloc::format!("wait-{}", task_id),
            task_id.as_bytes().to_vec(),
            timeout_ms,
        );
        self.invoke_syscall(request)?;
        Ok(alloc::format!("result-{}", task_id))
    }

    fn bind_tool(&self, agent_id: &str, tool_id: &str) -> AdapterResult<()> {
        let request = SyscallRequest::new(
            CsciSyscallId::ToolBind,
            alloc::format!("bind-{}-{}", agent_id, tool_id),
            alloc::format!("{},{}", agent_id, tool_id).into_bytes(),
            5000,
        );
        self.invoke_syscall(request)?;
        Ok(())
    }

    fn invoke_tool(&self, tool_id: &str, args: &str) -> AdapterResult<String> {
        let request = SyscallRequest::new(
            CsciSyscallId::ToolInvoke,
            alloc::format!("invoke-{}", tool_id),
            args.as_bytes().to_vec(),
            10000,
        );
        self.invoke_syscall(request)?;
        Ok(alloc::format!("tool-result-{}", tool_id))
    }

    fn create_channel(&self, channel_type: &str) -> AdapterResult<String> {
        let request = SyscallRequest::new(
            CsciSyscallId::ChannelCreate,
            "create-channel".into(),
            channel_type.as_bytes().to_vec(),
            1000,
        );
        self.invoke_syscall(request)?;
        Ok("channel-001".into())
    }

    fn send_channel(&self, channel_id: &str, message: &str) -> AdapterResult<()> {
        let request = SyscallRequest::new(
            CsciSyscallId::ChannelSend,
            alloc::format!("send-{}", channel_id),
            alloc::format!("{},{}", channel_id, message).into_bytes(),
            5000,
        );
        self.invoke_syscall(request)?;
        Ok(())
    }

    fn grant_capability(&self, entity_id: &str, cap_id: &str) -> AdapterResult<()> {
        let request = SyscallRequest::new(
            CsciSyscallId::CapGrant,
            alloc::format!("grant-{}-{}", entity_id, cap_id),
            alloc::format!("{},{}", entity_id, cap_id).into_bytes(),
            1000,
        );
        self.invoke_syscall(request)?;
        Ok(())
    }

    fn revoke_capability(&self, entity_id: &str, cap_id: &str) -> AdapterResult<()> {
        let request = SyscallRequest::new(
            CsciSyscallId::CapRevoke,
            alloc::format!("revoke-{}-{}", entity_id, cap_id),
            alloc::format!("{},{}", entity_id, cap_id).into_bytes(),
            1000,
        );
        self.invoke_syscall(request)?;
        Ok(())
    }

    fn write_memory(&self, memory_id: &str, data: &str) -> AdapterResult<()> {
        let request = SyscallRequest::new(
            CsciSyscallId::MemWrite,
            alloc::format!("write-{}", memory_id),
            alloc::format!("{},{}", memory_id, data).into_bytes(),
            2000,
        );
        self.invoke_syscall(request)?;
        Ok(())
    }

    fn read_memory(&self, memory_id: &str) -> AdapterResult<String> {
        let request = SyscallRequest::new(
            CsciSyscallId::MemRead,
            alloc::format!("read-{}", memory_id),
            memory_id.as_bytes().to_vec(),
            2000,
        );
        self.invoke_syscall(request)?;
        Ok(alloc::format!("data-{}", memory_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::vec;

    #[test]
    fn test_csci_syscall_id_as_u32() {
        assert_eq!(CsciSyscallId::MemWrite.as_u32(), 1);
        assert_eq!(CsciSyscallId::TaskSpawn.as_u32(), 3);
        assert_eq!(CsciSyscallId::AgentDestroy.as_u32(), 22);
    }

    #[test]
    fn test_csci_syscall_id_name() {
        assert_eq!(CsciSyscallId::MemWrite.name(), "mem_write");
        assert_eq!(CsciSyscallId::TaskSpawn.name(), "task_spawn");
        assert_eq!(CsciSyscallId::AgentDestroy.name(), "agent_destroy");
    }

    #[test]
    fn test_syscall_request_creation() {
        let req = SyscallRequest::new(
            CsciSyscallId::TaskSpawn,
            "req-001".into(),
            vec![1, 2, 3],
            5000,
        );
        assert_eq!(req.syscall_id, CsciSyscallId::TaskSpawn);
        assert_eq!(req.request_id, "req-001");
        assert_eq!(req.timeout_ms, 5000);
    }

    #[test]
    fn test_syscall_response_success() {
        let resp = SyscallResponse::success("req-001".into(), vec![1, 2, 3], 123456);
        assert!(resp.success);
        assert_eq!(resp.request_id, "req-001");
        assert_eq!(resp.result.len(), 3);
    }

    #[test]
    fn test_syscall_response_error() {
        let resp = SyscallResponse::error("req-001".into(), "Failed".into(), 123456);
        assert!(!resp.success);
        assert_eq!(resp.error_message, "Failed");
    }

    #[test]
    fn test_mock_syscall_binding_spawn_task() {
        let binding = MockSyscallBinding::new();
        let result = binding.spawn_task("agent-001", "task-name", 5000);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "task-agent-001");
    }

    #[test]
    fn test_mock_syscall_binding_wait_task() {
        let binding = MockSyscallBinding::new();
        let result = binding.wait_task("task-001", 5000);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("result"));
    }

    #[test]
    fn test_mock_syscall_binding_bind_tool() {
        let binding = MockSyscallBinding::new();
        let result = binding.bind_tool("agent-001", "tool-001");
        assert!(result.is_ok());
    }

    #[test]
    fn test_mock_syscall_binding_create_channel() {
        let binding = MockSyscallBinding::new();
        let result = binding.create_channel("request-reply");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "channel-001");
    }

    #[test]
    fn test_mock_syscall_binding_memory_ops() {
        let binding = MockSyscallBinding::new();
        assert!(binding.write_memory("mem-001", "data").is_ok());
        let read_result = binding.read_memory("mem-001");
        assert!(read_result.is_ok());
    }

    #[test]
    fn test_mock_syscall_binding_capability_ops() {
        let binding = MockSyscallBinding::new();
        assert!(binding.grant_capability("entity-001", "cap-001").is_ok());
        assert!(binding.revoke_capability("entity-001", "cap-001").is_ok());
    }
}
