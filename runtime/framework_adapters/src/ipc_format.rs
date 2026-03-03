// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.

//! # IPC Format Specification
//!
//! Defines the message formats and protocols for inter-process communication between
//! framework adapters and the cognitive kernel. Enables bidirectional communication
//! for task submission, status updates, memory requests, tool invocations, and exception
//! handling.
//!
//! Sec 4.2: Adapter-Kernel IPC Protocol
//! Sec 4.2: Message Format Specification

use alloc::string::String;
use crate::framework_type::FrameworkType;
use crate::runtime_adapter_ref::{CTPhase, CTID};

/// Serialization format hints for message encoding
/// Sec 4.2: Serialization Format Options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SerializationHint {
    /// Cap'n Proto binary serialization (efficient)
    CapnProto,
    /// JSON text serialization (human-readable)
    Json,
    /// Custom binary format
    Binary,
}

impl SerializationHint {
    /// Returns string representation of the serialization format
    pub fn as_str(&self) -> &'static str {
        match self {
            SerializationHint::CapnProto => "capnproto",
            SerializationHint::Json => "json",
            SerializationHint::Binary => "binary",
        }
    }
}

/// Task submission message from adapter to kernel
/// Sec 4.2: TaskSubmit Message
#[derive(Debug, Clone)]
pub struct TaskSubmit {
    /// Adapter-assigned task identifier
    pub task_id: String,
    /// Human-readable task name
    pub name: String,
    /// Task objective description
    pub objective: String,
    /// Timeout in milliseconds
    pub timeout_ms: u64,
    /// Whether task execution is mandatory
    pub is_mandatory: bool,
    /// Serialized task configuration from framework
    pub task_config: String,
}

/// Task status update message from kernel to adapter
/// Sec 4.2: TaskStatus Message
#[derive(Debug, Clone)]
pub struct TaskStatus {
    /// CognitiveTask identifier assigned by kernel
    pub ct_id: CTID,
    /// Current phase of task execution
    pub phase: CTPhase,
    /// Human-readable status message
    pub status_message: String,
    /// Progress percentage (0-100)
    pub progress_percent: u8,
}

/// Memory allocation request from adapter to kernel
/// Sec 4.2: MemoryRequest Message
#[derive(Debug, Clone)]
pub struct MemoryRequest {
    /// Adapter-assigned memory identifier
    pub memory_id: String,
    /// Type of memory (e.g., "conversation", "vector-store")
    pub memory_type: String,
    /// Requested capacity in tokens
    pub capacity_tokens: u64,
    /// Preferred serialization format
    pub serialization_format: String,
    /// Target memory tier preference (L1, L2, L3)
    pub target_tier: String,
}

/// Tool invocation request from kernel to adapter
/// Sec 4.2: ToolInvocation Message
#[derive(Debug, Clone)]
pub struct ToolInvocation {
    /// Tool invocation identifier
    pub invocation_id: String,
    /// CognitiveTask that triggered tool invocation
    pub ct_id: CTID,
    /// Tool identifier
    pub tool_id: String,
    /// Serialized tool input parameters
    pub input_parameters: String,
    /// Expected output schema for validation
    pub output_schema: String,
}

/// Phase transition notification from kernel to adapter
/// Sec 4.2: PhaseNotification Message
#[derive(Debug, Clone)]
pub struct PhaseNotification {
    /// CognitiveTask identifier
    pub ct_id: CTID,
    /// Previous phase
    pub old_phase: CTPhase,
    /// New phase
    pub new_phase: CTPhase,
    /// Timestamp of transition in milliseconds
    pub timestamp_ms: u64,
}

/// Exception report from kernel to adapter
/// Sec 4.2: ExceptionReport Message
#[derive(Debug, Clone)]
pub struct ExceptionReport {
    /// CognitiveTask identifier
    pub ct_id: CTID,
    /// Exception identifier
    pub exception_id: String,
    /// Exception type/class
    pub exception_type: String,
    /// Error message
    pub message: String,
    /// Diagnostic information
    pub diagnostic_info: String,
}

/// Adapter message from adapter to kernel
/// Sec 4.2: AdapterMessage Enumeration
#[derive(Debug, Clone)]
pub enum AdapterMessage {
    /// Task submission request
    TaskSubmit(TaskSubmit),
    /// Memory allocation request
    MemoryRequest(MemoryRequest),
    /// Exception report
    ExceptionReport(ExceptionReport),
}

impl AdapterMessage {
    /// Returns string representation of the message type
    pub fn message_type(&self) -> &'static str {
        match self {
            AdapterMessage::TaskSubmit(_) => "task_submit",
            AdapterMessage::MemoryRequest(_) => "memory_request",
            AdapterMessage::ExceptionReport(_) => "exception_report",
        }
    }
}

/// Kernel response message to adapter
/// Sec 4.2: KernelResponse Enumeration
#[derive(Debug, Clone)]
pub enum KernelResponse {
    /// Task accepted and queued
    TaskAccepted {
        /// Adapter task identifier
        task_id: String,
        /// Kernel-assigned CognitiveTask identifier
        ct_id: CTID,
    },
    /// Task rejected
    TaskRejected {
        /// Adapter task identifier
        task_id: String,
        /// Reason for rejection
        reason: String,
    },
    /// Memory successfully allocated
    MemoryAllocated {
        /// Memory identifier
        memory_id: String,
        /// Allocated tier
        tier: String,
        /// Actual capacity in tokens
        actual_capacity_tokens: u64,
    },
    /// Tool execution result
    ToolResult {
        /// Invocation identifier
        invocation_id: String,
        /// Serialized tool output
        output: String,
        /// Whether execution succeeded
        success: bool,
    },
    /// Message acknowledgment
    Acknowledged {
        /// Correlation identifier
        correlation_id: String,
    },
    /// Error response
    ErrorResponse {
        /// Correlation identifier
        correlation_id: String,
        /// Error message
        error_message: String,
        /// Error code
        error_code: u32,
    },
}

impl KernelResponse {
    /// Returns string representation of the response type
    pub fn response_type(&self) -> &'static str {
        match self {
            KernelResponse::TaskAccepted { .. } => "task_accepted",
            KernelResponse::TaskRejected { .. } => "task_rejected",
            KernelResponse::MemoryAllocated { .. } => "memory_allocated",
            KernelResponse::ToolResult { .. } => "tool_result",
            KernelResponse::Acknowledged { .. } => "acknowledged",
            KernelResponse::ErrorResponse { .. } => "error_response",
        }
    }
}

/// Message envelope for all adapter-kernel communication
/// Sec 4.2: MessageEnvelope Structure
#[derive(Debug, Clone)]
pub struct MessageEnvelope {
    /// Source adapter type
    pub source_adapter: FrameworkType,
    /// Target (always kernel for adapter messages)
    pub target: String,
    /// Correlation identifier for request-response pairing
    pub correlation_id: String,
    /// Timestamp when message was created (milliseconds)
    pub timestamp_ms: u64,
    /// Preferred serialization format
    pub serialization_hint: SerializationHint,
    /// Message payload as variant
    pub payload: MessagePayload,
}

/// Message payload variant for envelope
/// Sec 4.2: MessagePayload Enumeration
#[derive(Debug, Clone)]
pub enum MessagePayload {
    /// Adapter message (adapter → kernel)
    Adapter(AdapterMessage),
    /// Kernel response (kernel → adapter)
    Response(KernelResponse),
    /// Task status update (kernel → adapter)
    TaskStatus(TaskStatus),
    /// Phase notification (kernel → adapter)
    PhaseNotification(PhaseNotification),
    /// Tool invocation request (kernel → adapter)
    ToolInvocation(ToolInvocation),
}

impl MessagePayload {
    /// Returns string representation of the payload type
    pub fn payload_type(&self) -> &'static str {
        match self {
            MessagePayload::Adapter(msg) => msg.message_type(),
            MessagePayload::Response(resp) => resp.response_type(),
            MessagePayload::TaskStatus(_) => "task_status",
            MessagePayload::PhaseNotification(_) => "phase_notification",
            MessagePayload::ToolInvocation(_) => "tool_invocation",
        }
    }
}

impl MessageEnvelope {
    /// Creates a new message envelope
    /// Sec 4.2: MessageEnvelope Construction
    pub fn new(
        source_adapter: FrameworkType,
        target: String,
        correlation_id: String,
        timestamp_ms: u64,
        serialization_hint: SerializationHint,
        payload: MessagePayload,
    ) -> Self {
        MessageEnvelope {
            source_adapter,
            target,
            correlation_id,
            timestamp_ms,
            serialization_hint,
            payload,
        }
    }

    /// Returns the message type from the payload
    pub fn message_type(&self) -> &'static str {
        self.payload.payload_type()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialization_hint_as_str() {
        assert_eq!(SerializationHint::CapnProto.as_str(), "capnproto");
        assert_eq!(SerializationHint::Json.as_str(), "json");
        assert_eq!(SerializationHint::Binary.as_str(), "binary");
    }

    #[test]
    fn test_task_submit_creation() {
        let submit = TaskSubmit {
            task_id: "task-001".into(),
            name: "MyTask".into(),
            objective: "Do something".into(),
            timeout_ms: 5000,
            is_mandatory: true,
            task_config: "{}".into(),
        };
        assert_eq!(submit.task_id, "task-001");
        assert!(submit.is_mandatory);
    }

    #[test]
    fn test_task_status_creation() {
        let status = TaskStatus {
            ct_id: 42,
            phase: CTPhase::Executing,
            status_message: "Running".into(),
            progress_percent: 50,
        };
        assert_eq!(status.ct_id, 42);
        assert_eq!(status.phase, CTPhase::Executing);
        assert_eq!(status.progress_percent, 50);
    }

    #[test]
    fn test_memory_request_creation() {
        let req = MemoryRequest {
            memory_id: "mem-001".into(),
            memory_type: "conversation".into(),
            capacity_tokens: 100000,
            serialization_format: "json".into(),
            target_tier: "L2".into(),
        };
        assert_eq!(req.memory_id, "mem-001");
        assert_eq!(req.target_tier, "L2");
    }

    #[test]
    fn test_tool_invocation_creation() {
        let invocation = ToolInvocation {
            invocation_id: "inv-001".into(),
            ct_id: 42,
            tool_id: "tool-001".into(),
            input_parameters: "{}".into(),
            output_schema: "{}".into(),
        };
        assert_eq!(invocation.invocation_id, "inv-001");
        assert_eq!(invocation.ct_id, 42);
    }

    #[test]
    fn test_phase_notification_creation() {
        let notif = PhaseNotification {
            ct_id: 42,
            old_phase: CTPhase::Queued,
            new_phase: CTPhase::Executing,
            timestamp_ms: 1234567890,
        };
        assert_eq!(notif.ct_id, 42);
        assert_eq!(notif.old_phase, CTPhase::Queued);
        assert_eq!(notif.new_phase, CTPhase::Executing);
    }

    #[test]
    fn test_exception_report_creation() {
        let report = ExceptionReport {
            ct_id: 42,
            exception_id: "exc-001".into(),
            exception_type: "TimeoutError".into(),
            message: "Task timed out".into(),
            diagnostic_info: "Exceeded 5000ms".into(),
        };
        assert_eq!(report.ct_id, 42);
        assert_eq!(report.exception_type, "TimeoutError");
    }

    #[test]
    fn test_adapter_message_task_submit() {
        let submit = TaskSubmit {
            task_id: "task-001".into(),
            name: "MyTask".into(),
            objective: "Do something".into(),
            timeout_ms: 5000,
            is_mandatory: true,
            task_config: "{}".into(),
        };
        let msg = AdapterMessage::TaskSubmit(submit);
        assert_eq!(msg.message_type(), "task_submit");
    }

    #[test]
    fn test_adapter_message_memory_request() {
        let req = MemoryRequest {
            memory_id: "mem-001".into(),
            memory_type: "conversation".into(),
            capacity_tokens: 100000,
            serialization_format: "json".into(),
            target_tier: "L2".into(),
        };
        let msg = AdapterMessage::MemoryRequest(req);
        assert_eq!(msg.message_type(), "memory_request");
    }

    #[test]
    fn test_kernel_response_task_accepted() {
        let resp = KernelResponse::TaskAccepted {
            task_id: "task-001".into(),
            ct_id: 42,
        };
        assert_eq!(resp.response_type(), "task_accepted");
    }

    #[test]
    fn test_kernel_response_task_rejected() {
        let resp = KernelResponse::TaskRejected {
            task_id: "task-001".into(),
            reason: "Invalid configuration".into(),
        };
        assert_eq!(resp.response_type(), "task_rejected");
    }

    #[test]
    fn test_kernel_response_memory_allocated() {
        let resp = KernelResponse::MemoryAllocated {
            memory_id: "mem-001".into(),
            tier: "L2".into(),
            actual_capacity_tokens: 100000,
        };
        assert_eq!(resp.response_type(), "memory_allocated");
    }

    #[test]
    fn test_kernel_response_tool_result() {
        let resp = KernelResponse::ToolResult {
            invocation_id: "inv-001".into(),
            output: "Result data".into(),
            success: true,
        };
        assert_eq!(resp.response_type(), "tool_result");
    }

    #[test]
    fn test_kernel_response_error() {
        let resp = KernelResponse::ErrorResponse {
            correlation_id: "corr-001".into(),
            error_message: "Internal error".into(),
            error_code: 500,
        };
        assert_eq!(resp.response_type(), "error_response");
    }

    #[test]
    fn test_message_payload_adapter() {
        let submit = TaskSubmit {
            task_id: "task-001".into(),
            name: "MyTask".into(),
            objective: "Do something".into(),
            timeout_ms: 5000,
            is_mandatory: true,
            task_config: "{}".into(),
        };
        let payload = MessagePayload::Adapter(AdapterMessage::TaskSubmit(submit));
        assert_eq!(payload.payload_type(), "task_submit");
    }

    #[test]
    fn test_message_payload_response() {
        let resp = KernelResponse::TaskAccepted {
            task_id: "task-001".into(),
            ct_id: 42,
        };
        let payload = MessagePayload::Response(resp);
        assert_eq!(payload.payload_type(), "task_accepted");
    }

    #[test]
    fn test_message_envelope_creation() {
        let submit = TaskSubmit {
            task_id: "task-001".into(),
            name: "MyTask".into(),
            objective: "Do something".into(),
            timeout_ms: 5000,
            is_mandatory: true,
            task_config: "{}".into(),
        };
        let payload = MessagePayload::Adapter(AdapterMessage::TaskSubmit(submit));
        let envelope = MessageEnvelope::new(
            FrameworkType::LangChain,
            "kernel".into(),
            "corr-001".into(),
            1234567890,
            SerializationHint::Json,
            payload,
        );
        assert_eq!(envelope.source_adapter, FrameworkType::LangChain);
        assert_eq!(envelope.message_type(), "task_submit");
    }

    #[test]
    fn test_message_envelope_phase_notification() {
        let notif = PhaseNotification {
            ct_id: 42,
            old_phase: CTPhase::Queued,
            new_phase: CTPhase::Executing,
            timestamp_ms: 1234567890,
        };
        let payload = MessagePayload::PhaseNotification(notif);
        let envelope = MessageEnvelope::new(
            FrameworkType::CrewAI,
            "adapter".into(),
            "corr-002".into(),
            1234567900,
            SerializationHint::CapnProto,
            payload,
        );
        assert_eq!(envelope.source_adapter, FrameworkType::CrewAI);
        assert_eq!(envelope.message_type(), "phase_notification");
    }

    #[test]
    fn test_message_envelope_tool_invocation() {
        let invocation = ToolInvocation {
            invocation_id: "inv-001".into(),
            ct_id: 42,
            tool_id: "tool-001".into(),
            input_parameters: "{}".into(),
            output_schema: "{}".into(),
        };
        let payload = MessagePayload::ToolInvocation(invocation);
        let envelope = MessageEnvelope::new(
            FrameworkType::SemanticKernel,
            "adapter".into(),
            "corr-003".into(),
            1234567910,
            SerializationHint::Binary,
            payload,
        );
        assert_eq!(envelope.message_type(), "tool_invocation");
    }
}
