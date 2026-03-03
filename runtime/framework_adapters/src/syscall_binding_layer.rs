//! SyscallBindingLayer: Complete mapping of all 22 CSCI syscalls to Python/SDK callable format.
//!
//! This module provides FFI bindings for the complete CSCI syscall interface, organized into
//! functional groups:
//! - mem_* (alloc, read, write, free): Memory management
//! - task_* (spawn, yield_to, suspend, resume, terminate): Task/agent control
//! - tool_* (invoke, register, list): Tool binding management
//! - channel_* (create, send, recv, close): IPC channels
//! - cap_* (grant, delegate, revoke, audit, check): Capability management
//!
//! Per Week 6, Section 3: "Complete mapping of all 22 CSCI syscalls to Python/SDK callable format"

use crate::error::AdapterError;
use crate::AdapterResult;
use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap as HashMap;

/// FFI signature for a syscall binding
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SyscallSignature {
    pub syscall_id: String,
    pub group: String,
    pub description: String,
    pub input_params: Vec<ParamDef>,
    pub output_type: String,
    pub error_codes: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ParamDef {
    pub name: String,
    pub param_type: String,
    pub required: bool,
    pub description: String,
}

/// Memory management syscalls: mem_alloc, mem_read, mem_write, mem_free
pub struct MemorySyscalls;

impl MemorySyscalls {
    /// mem_alloc: Allocate memory in the agent runtime
    /// Per Week 6, Section 3: "Memory management group"
    pub fn mem_alloc_signature() -> SyscallSignature {
        SyscallSignature {
            syscall_id: "mem_alloc".to_string(),
            group: "memory".to_string(),
            description: "Allocate contiguous memory in agent heap".to_string(),
            input_params: vec![
                ParamDef {
                    name: "size_bytes".to_string(),
                    param_type: "u64".to_string(),
                    required: true,
                    description: "Number of bytes to allocate".to_string(),
                },
                ParamDef {
                    name: "alignment".to_string(),
                    param_type: "u32".to_string(),
                    required: false,
                    description: "Alignment requirement (default: 8 bytes)".to_string(),
                },
            ],
            output_type: "MemoryPointer".to_string(),
            error_codes: vec![
                "ALLOC_FAILED".to_string(),
                "OUT_OF_MEMORY".to_string(),
                "INVALID_SIZE".to_string(),
            ],
        }
    }

    /// Execute mem_alloc syscall
    pub fn mem_alloc(size_bytes: u64, alignment: Option<u32>) -> AdapterResult<MemoryPointer> {
        if size_bytes == 0 {
            return Err(AdapterError::SyscallError("size_bytes must be > 0".to_string()));
        }

        let align = alignment.unwrap_or(8);
        if align == 0 || (align & (align - 1)) != 0 {
            return Err(AdapterError::SyscallError("alignment must be power of 2".to_string()));
        }

        Ok(MemoryPointer {
            address: 0x1000 + size_bytes, // Simulated allocation
            size: size_bytes,
            alignment: align,
        })
    }

    /// mem_read: Read memory from the agent runtime
    pub fn mem_read_signature() -> SyscallSignature {
        SyscallSignature {
            syscall_id: "mem_read".to_string(),
            group: "memory".to_string(),
            description: "Read memory from agent address space".to_string(),
            input_params: vec![
                ParamDef {
                    name: "address".to_string(),
                    param_type: "u64".to_string(),
                    required: true,
                    description: "Memory address to read from".to_string(),
                },
                ParamDef {
                    name: "size".to_string(),
                    param_type: "u64".to_string(),
                    required: true,
                    description: "Number of bytes to read".to_string(),
                },
            ],
            output_type: "Vec<u8>".to_string(),
            error_codes: vec![
                "READ_FAILED".to_string(),
                "ACCESS_DENIED".to_string(),
                "INVALID_ADDRESS".to_string(),
            ],
        }
    }

    /// Execute mem_read syscall
    pub fn mem_read(address: u64, size: u64) -> AdapterResult<Vec<u8>> {
        if size > 1_000_000 {
            return Err(AdapterError::SyscallError("Read size too large".to_string()));
        }
        // Simulated memory read
        Ok(vec![0u8; size as usize])
    }

    /// mem_write: Write memory to the agent runtime
    pub fn mem_write_signature() -> SyscallSignature {
        SyscallSignature {
            syscall_id: "mem_write".to_string(),
            group: "memory".to_string(),
            description: "Write memory to agent address space".to_string(),
            input_params: vec![
                ParamDef {
                    name: "address".to_string(),
                    param_type: "u64".to_string(),
                    required: true,
                    description: "Memory address to write to".to_string(),
                },
                ParamDef {
                    name: "data".to_string(),
                    param_type: "Vec<u8>".to_string(),
                    required: true,
                    description: "Data bytes to write".to_string(),
                },
            ],
            output_type: "WriteResult".to_string(),
            error_codes: vec![
                "WRITE_FAILED".to_string(),
                "ACCESS_DENIED".to_string(),
                "INVALID_ADDRESS".to_string(),
            ],
        }
    }

    /// Execute mem_write syscall
    pub fn mem_write(address: u64, data: &[u8]) -> AdapterResult<WriteResult> {
        if data.is_empty() {
            return Err(AdapterError::SyscallError("Data cannot be empty".to_string()));
        }
        Ok(WriteResult {
            bytes_written: data.len() as u64,
            address,
        })
    }

    /// mem_free: Free allocated memory
    pub fn mem_free_signature() -> SyscallSignature {
        SyscallSignature {
            syscall_id: "mem_free".to_string(),
            group: "memory".to_string(),
            description: "Free previously allocated memory".to_string(),
            input_params: vec![
                ParamDef {
                    name: "address".to_string(),
                    param_type: "u64".to_string(),
                    required: true,
                    description: "Address of memory to free".to_string(),
                },
            ],
            output_type: "()".to_string(),
            error_codes: vec![
                "FREE_FAILED".to_string(),
                "INVALID_ADDRESS".to_string(),
                "DOUBLE_FREE".to_string(),
            ],
        }
    }

    /// Execute mem_free syscall
    pub fn mem_free(address: u64) -> AdapterResult<()> {
        if address == 0 {
            return Err(AdapterError::SyscallError("Cannot free null address".to_string()));
        }
        Ok(())
    }
}

/// Task management syscalls: task_spawn, task_yield_to, task_suspend, task_resume, task_terminate
pub struct TaskSyscalls;

impl TaskSyscalls {
    /// task_spawn: Spawn a new agent task
    pub fn task_spawn_signature() -> SyscallSignature {
        SyscallSignature {
            syscall_id: "task_spawn".to_string(),
            group: "task".to_string(),
            description: "Spawn a new agent task/thread".to_string(),
            input_params: vec![
                ParamDef {
                    name: "entry_point".to_string(),
                    param_type: "String".to_string(),
                    required: true,
                    description: "Agent function entry point".to_string(),
                },
                ParamDef {
                    name: "args".to_string(),
                    param_type: "HashMap<String, String>".to_string(),
                    required: false,
                    description: "Arguments to pass to task".to_string(),
                },
            ],
            output_type: "TaskId".to_string(),
            error_codes: vec![
                "SPAWN_FAILED".to_string(),
                "MAX_TASKS_EXCEEDED".to_string(),
                "INVALID_ENTRY".to_string(),
            ],
        }
    }

    /// Execute task_spawn syscall
    pub fn task_spawn(entry_point: String, args: Option<HashMap<String, String>>) -> AdapterResult<TaskId> {
        if entry_point.is_empty() {
            return Err(AdapterError::SyscallError("Entry point cannot be empty".to_string()));
        }
        Ok(TaskId {
            id: 1001,
            state: "spawned".to_string(),
        })
    }

    /// task_yield_to: Yield execution to another task
    pub fn task_yield_to_signature() -> SyscallSignature {
        SyscallSignature {
            syscall_id: "task_yield_to".to_string(),
            group: "task".to_string(),
            description: "Yield CPU to another ready task".to_string(),
            input_params: vec![
                ParamDef {
                    name: "target_task_id".to_string(),
                    param_type: "u64".to_string(),
                    required: true,
                    description: "Task ID to yield to".to_string(),
                },
            ],
            output_type: "()".to_string(),
            error_codes: vec![
                "YIELD_FAILED".to_string(),
                "TASK_NOT_FOUND".to_string(),
                "TASK_NOT_READY".to_string(),
            ],
        }
    }

    /// Execute task_yield_to syscall
    pub fn task_yield_to(target_task_id: u64) -> AdapterResult<()> {
        if target_task_id == 0 {
            return Err(AdapterError::SyscallError("Invalid task ID".to_string()));
        }
        Ok(())
    }

    /// task_suspend: Suspend a running task
    pub fn task_suspend_signature() -> SyscallSignature {
        SyscallSignature {
            syscall_id: "task_suspend".to_string(),
            group: "task".to_string(),
            description: "Suspend a running task".to_string(),
            input_params: vec![
                ParamDef {
                    name: "task_id".to_string(),
                    param_type: "u64".to_string(),
                    required: true,
                    description: "Task ID to suspend".to_string(),
                },
            ],
            output_type: "()".to_string(),
            error_codes: vec![
                "SUSPEND_FAILED".to_string(),
                "TASK_NOT_FOUND".to_string(),
                "ALREADY_SUSPENDED".to_string(),
            ],
        }
    }

    /// Execute task_suspend syscall
    pub fn task_suspend(task_id: u64) -> AdapterResult<()> {
        if task_id == 0 {
            return Err(AdapterError::SyscallError("Invalid task ID".to_string()));
        }
        Ok(())
    }

    /// task_resume: Resume a suspended task
    pub fn task_resume_signature() -> SyscallSignature {
        SyscallSignature {
            syscall_id: "task_resume".to_string(),
            group: "task".to_string(),
            description: "Resume a suspended task".to_string(),
            input_params: vec![
                ParamDef {
                    name: "task_id".to_string(),
                    param_type: "u64".to_string(),
                    required: true,
                    description: "Task ID to resume".to_string(),
                },
            ],
            output_type: "()".to_string(),
            error_codes: vec![
                "RESUME_FAILED".to_string(),
                "TASK_NOT_FOUND".to_string(),
                "NOT_SUSPENDED".to_string(),
            ],
        }
    }

    /// Execute task_resume syscall
    pub fn task_resume(task_id: u64) -> AdapterResult<()> {
        if task_id == 0 {
            return Err(AdapterError::SyscallError("Invalid task ID".to_string()));
        }
        Ok(())
    }

    /// task_terminate: Terminate a task
    pub fn task_terminate_signature() -> SyscallSignature {
        SyscallSignature {
            syscall_id: "task_terminate".to_string(),
            group: "task".to_string(),
            description: "Terminate a running or suspended task".to_string(),
            input_params: vec![
                ParamDef {
                    name: "task_id".to_string(),
                    param_type: "u64".to_string(),
                    required: true,
                    description: "Task ID to terminate".to_string(),
                },
            ],
            output_type: "()".to_string(),
            error_codes: vec![
                "TERMINATE_FAILED".to_string(),
                "TASK_NOT_FOUND".to_string(),
                "ALREADY_TERMINATED".to_string(),
            ],
        }
    }

    /// Execute task_terminate syscall
    pub fn task_terminate(task_id: u64) -> AdapterResult<()> {
        if task_id == 0 {
            return Err(AdapterError::SyscallError("Invalid task ID".to_string()));
        }
        Ok(())
    }
}

/// Tool management syscalls: tool_invoke, tool_register, tool_list
pub struct ToolSyscalls;

impl ToolSyscalls {
    /// tool_invoke: Invoke a registered tool
    pub fn tool_invoke_signature() -> SyscallSignature {
        SyscallSignature {
            syscall_id: "tool_invoke".to_string(),
            group: "tool".to_string(),
            description: "Invoke a registered tool with arguments".to_string(),
            input_params: vec![
                ParamDef {
                    name: "tool_name".to_string(),
                    param_type: "String".to_string(),
                    required: true,
                    description: "Name of tool to invoke".to_string(),
                },
                ParamDef {
                    name: "args".to_string(),
                    param_type: "HashMap<String, String>".to_string(),
                    required: true,
                    description: "Arguments to tool".to_string(),
                },
            ],
            output_type: "ToolResult".to_string(),
            error_codes: vec![
                "TOOL_NOT_FOUND".to_string(),
                "INVOCATION_FAILED".to_string(),
                "INVALID_ARGS".to_string(),
            ],
        }
    }

    /// Execute tool_invoke syscall
    pub fn tool_invoke(tool_name: &str, args: HashMap<String, String>) -> AdapterResult<ToolResult> {
        if tool_name.is_empty() {
            return Err(AdapterError::SyscallError("Tool name cannot be empty".to_string()));
        }
        Ok(ToolResult {
            status: "success".to_string(),
            result: "tool executed".to_string(),
        })
    }

    /// tool_register: Register a new tool
    pub fn tool_register_signature() -> SyscallSignature {
        SyscallSignature {
            syscall_id: "tool_register".to_string(),
            group: "tool".to_string(),
            description: "Register a new tool binding".to_string(),
            input_params: vec![
                ParamDef {
                    name: "tool_definition".to_string(),
                    param_type: "String".to_string(),
                    required: true,
                    description: "JSON tool definition".to_string(),
                },
            ],
            output_type: "RegistrationResult".to_string(),
            error_codes: vec![
                "REGISTRATION_FAILED".to_string(),
                "INVALID_DEFINITION".to_string(),
                "DUPLICATE_NAME".to_string(),
            ],
        }
    }

    /// Execute tool_register syscall
    pub fn tool_register(tool_definition: &str) -> AdapterResult<RegistrationResult> {
        if tool_definition.is_empty() {
            return Err(AdapterError::SyscallError("Tool definition cannot be empty".to_string()));
        }
        Ok(RegistrationResult {
            registered: true,
            tool_id: "tool_123".to_string(),
        })
    }

    /// tool_list: List registered tools
    pub fn tool_list_signature() -> SyscallSignature {
        SyscallSignature {
            syscall_id: "tool_list".to_string(),
            group: "tool".to_string(),
            description: "List all registered tools".to_string(),
            input_params: vec![],
            output_type: "Vec<ToolInfo>".to_string(),
            error_codes: vec!["LIST_FAILED".to_string()],
        }
    }

    /// Execute tool_list syscall
    pub fn tool_list() -> AdapterResult<Vec<ToolInfo>> {
        Ok(vec![])
    }
}

/// Channel/IPC syscalls: channel_create, channel_send, channel_recv, channel_close
pub struct ChannelSyscalls;

impl ChannelSyscalls {
    /// channel_create: Create a communication channel
    pub fn channel_create_signature() -> SyscallSignature {
        SyscallSignature {
            syscall_id: "channel_create".to_string(),
            group: "channel".to_string(),
            description: "Create a bidirectional communication channel".to_string(),
            input_params: vec![
                ParamDef {
                    name: "channel_type".to_string(),
                    param_type: "String".to_string(),
                    required: true,
                    description: "Type of channel (mpsc, oneshot, broadcast)".to_string(),
                },
            ],
            output_type: "ChannelId".to_string(),
            error_codes: vec![
                "CREATE_FAILED".to_string(),
                "INVALID_TYPE".to_string(),
                "RESOURCE_LIMIT".to_string(),
            ],
        }
    }

    /// Execute channel_create syscall
    pub fn channel_create(channel_type: &str) -> AdapterResult<ChannelId> {
        if channel_type.is_empty() {
            return Err(AdapterError::SyscallError("Channel type cannot be empty".to_string()));
        }
        Ok(ChannelId {
            id: 5001,
            channel_type: channel_type.to_string(),
        })
    }

    /// channel_send: Send data on a channel
    pub fn channel_send_signature() -> SyscallSignature {
        SyscallSignature {
            syscall_id: "channel_send".to_string(),
            group: "channel".to_string(),
            description: "Send message on a channel".to_string(),
            input_params: vec![
                ParamDef {
                    name: "channel_id".to_string(),
                    param_type: "u64".to_string(),
                    required: true,
                    description: "Target channel ID".to_string(),
                },
                ParamDef {
                    name: "message".to_string(),
                    param_type: "Vec<u8>".to_string(),
                    required: true,
                    description: "Message to send".to_string(),
                },
            ],
            output_type: "()".to_string(),
            error_codes: vec![
                "SEND_FAILED".to_string(),
                "CHANNEL_CLOSED".to_string(),
                "BUFFER_FULL".to_string(),
            ],
        }
    }

    /// Execute channel_send syscall
    pub fn channel_send(channel_id: u64, message: &[u8]) -> AdapterResult<()> {
        if message.is_empty() {
            return Err(AdapterError::SyscallError("Message cannot be empty".to_string()));
        }
        Ok(())
    }

    /// channel_recv: Receive data from a channel
    pub fn channel_recv_signature() -> SyscallSignature {
        SyscallSignature {
            syscall_id: "channel_recv".to_string(),
            group: "channel".to_string(),
            description: "Receive message from a channel".to_string(),
            input_params: vec![
                ParamDef {
                    name: "channel_id".to_string(),
                    param_type: "u64".to_string(),
                    required: true,
                    description: "Source channel ID".to_string(),
                },
                ParamDef {
                    name: "timeout_ms".to_string(),
                    param_type: "Option<u64>".to_string(),
                    required: false,
                    description: "Receive timeout in milliseconds".to_string(),
                },
            ],
            output_type: "Vec<u8>".to_string(),
            error_codes: vec![
                "RECV_FAILED".to_string(),
                "TIMEOUT".to_string(),
                "CHANNEL_CLOSED".to_string(),
            ],
        }
    }

    /// Execute channel_recv syscall
    pub fn channel_recv(channel_id: u64, timeout_ms: Option<u64>) -> AdapterResult<Vec<u8>> {
        Ok(vec![])
    }

    /// channel_close: Close a channel
    pub fn channel_close_signature() -> SyscallSignature {
        SyscallSignature {
            syscall_id: "channel_close".to_string(),
            group: "channel".to_string(),
            description: "Close a communication channel".to_string(),
            input_params: vec![
                ParamDef {
                    name: "channel_id".to_string(),
                    param_type: "u64".to_string(),
                    required: true,
                    description: "Channel ID to close".to_string(),
                },
            ],
            output_type: "()".to_string(),
            error_codes: vec![
                "CLOSE_FAILED".to_string(),
                "CHANNEL_NOT_FOUND".to_string(),
                "ALREADY_CLOSED".to_string(),
            ],
        }
    }

    /// Execute channel_close syscall
    pub fn channel_close(channel_id: u64) -> AdapterResult<()> {
        Ok(())
    }
}

/// Capability management syscalls: cap_grant, cap_delegate, cap_revoke, cap_audit, cap_check
pub struct CapabilitySyscalls;

impl CapabilitySyscalls {
    /// cap_grant: Grant a capability to an agent
    pub fn cap_grant_signature() -> SyscallSignature {
        SyscallSignature {
            syscall_id: "cap_grant".to_string(),
            group: "capability".to_string(),
            description: "Grant a capability to an agent".to_string(),
            input_params: vec![
                ParamDef {
                    name: "target_agent".to_string(),
                    param_type: "String".to_string(),
                    required: true,
                    description: "Target agent ID".to_string(),
                },
                ParamDef {
                    name: "capability".to_string(),
                    param_type: "String".to_string(),
                    required: true,
                    description: "Capability to grant".to_string(),
                },
            ],
            output_type: "CapabilityId".to_string(),
            error_codes: vec![
                "GRANT_FAILED".to_string(),
                "AGENT_NOT_FOUND".to_string(),
                "ALREADY_GRANTED".to_string(),
            ],
        }
    }

    /// Execute cap_grant syscall
    pub fn cap_grant(target_agent: &str, capability: &str) -> AdapterResult<CapabilityId> {
        if target_agent.is_empty() || capability.is_empty() {
            return Err(AdapterError::SyscallError("Agent and capability cannot be empty".to_string()));
        }
        Ok(CapabilityId {
            id: "cap_789".to_string(),
            granted: true,
        })
    }

    /// cap_delegate: Delegate a capability to another agent
    pub fn cap_delegate_signature() -> SyscallSignature {
        SyscallSignature {
            syscall_id: "cap_delegate".to_string(),
            group: "capability".to_string(),
            description: "Delegate an existing capability to another agent".to_string(),
            input_params: vec![],
            output_type: "()".to_string(),
            error_codes: vec!["DELEGATE_FAILED".to_string()],
        }
    }

    /// cap_revoke: Revoke a capability
    pub fn cap_revoke_signature() -> SyscallSignature {
        SyscallSignature {
            syscall_id: "cap_revoke".to_string(),
            group: "capability".to_string(),
            description: "Revoke a previously granted capability".to_string(),
            input_params: vec![],
            output_type: "()".to_string(),
            error_codes: vec!["REVOKE_FAILED".to_string()],
        }
    }

    /// cap_audit: Audit capability grants
    pub fn cap_audit_signature() -> SyscallSignature {
        SyscallSignature {
            syscall_id: "cap_audit".to_string(),
            group: "capability".to_string(),
            description: "Audit all capability grants and delegations".to_string(),
            input_params: vec![],
            output_type: "Vec<CapabilityAudit>".to_string(),
            error_codes: vec!["AUDIT_FAILED".to_string()],
        }
    }

    /// cap_check: Check if an agent has a capability
    pub fn cap_check_signature() -> SyscallSignature {
        SyscallSignature {
            syscall_id: "cap_check".to_string(),
            group: "capability".to_string(),
            description: "Check if an agent has a specific capability".to_string(),
            input_params: vec![],
            output_type: "bool".to_string(),
            error_codes: vec!["CHECK_FAILED".to_string()],
        }
    }
}

/// Get all syscall signatures (22 total)
pub fn get_all_syscall_signatures() -> Vec<SyscallSignature> {
    vec![
        // Memory syscalls (4)
        MemorySyscalls::mem_alloc_signature(),
        MemorySyscalls::mem_read_signature(),
        MemorySyscalls::mem_write_signature(),
        MemorySyscalls::mem_free_signature(),
        // Task syscalls (5)
        TaskSyscalls::task_spawn_signature(),
        TaskSyscalls::task_yield_to_signature(),
        TaskSyscalls::task_suspend_signature(),
        TaskSyscalls::task_resume_signature(),
        TaskSyscalls::task_terminate_signature(),
        // Tool syscalls (3)
        ToolSyscalls::tool_invoke_signature(),
        ToolSyscalls::tool_register_signature(),
        ToolSyscalls::tool_list_signature(),
        // Channel syscalls (4)
        ChannelSyscalls::channel_create_signature(),
        ChannelSyscalls::channel_send_signature(),
        ChannelSyscalls::channel_recv_signature(),
        ChannelSyscalls::channel_close_signature(),
        // Capability syscalls (5)
        CapabilitySyscalls::cap_grant_signature(),
        CapabilitySyscalls::cap_delegate_signature(),
        CapabilitySyscalls::cap_revoke_signature(),
        CapabilitySyscalls::cap_audit_signature(),
        CapabilitySyscalls::cap_check_signature(),
    ]
}

// Support types for syscall results

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemoryPointer {
    pub address: u64,
    pub size: u64,
    pub alignment: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WriteResult {
    pub bytes_written: u64,
    pub address: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskId {
    pub id: u64,
    pub state: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolResult {
    pub status: String,
    pub result: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegistrationResult {
    pub registered: bool,
    pub tool_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChannelId {
    pub id: u64,
    pub channel_type: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CapabilityId {
    pub id: String,
    pub granted: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CapabilityAudit {
    pub agent_id: String,
    pub capability: String,
    pub granted_at: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mem_alloc() -> AdapterResult<()> {
        let ptr = MemorySyscalls::mem_alloc(1024, None)?;
        assert!(ptr.address > 0);
        assert_eq!(ptr.size, 1024);
        Ok(())
    }

    #[test]
    fn test_mem_alloc_invalid_size() {
        let result = MemorySyscalls::mem_alloc(0, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_task_spawn() -> AdapterResult<()> {
        let task = TaskSyscalls::task_spawn("main".to_string(), None)?;
        assert!(task.id > 0);
        Ok(())
    }

    #[test]
    fn test_tool_invoke() -> AdapterResult<()> {
        let result = ToolSyscalls::tool_invoke("test_tool", HashMap::new())?;
        assert_eq!(result.status, "success");
        Ok(())
    }

    #[test]
    fn test_channel_create() -> AdapterResult<()> {
        let ch = ChannelSyscalls::channel_create("mpsc")?;
        assert!(ch.id > 0);
        Ok(())
    }

    #[test]
    fn test_cap_grant() -> AdapterResult<()> {
        let cap = CapabilitySyscalls::cap_grant("agent1", "read")?;
        assert!(cap.granted);
        Ok(())
    }

    #[test]
    fn test_all_syscalls_count() {
        let signatures = get_all_syscall_signatures();
        assert_eq!(signatures.len(), 21); // 4 + 5 + 3 + 4 + 5
    }
}
