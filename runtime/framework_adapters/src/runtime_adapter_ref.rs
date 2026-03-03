// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.

//! # Runtime Adapter Reference Interface
//!
//! Defines the core adapter interface contract between framework adapters and the cognitive kernel.
//! This is the bidirectional interface through which frameworks communicate task requirements,
//! memory constraints, tool bindings, and exception handling to the kernel runtime.
//!
//! Sec 4.2: RuntimeAdapterRef Interface Contract
//! Sec 4.2: Framework-Kernel Communication Protocol
//! Sec 5.1: Translation Fidelity Tracking

use crate::framework_type::FrameworkType;
use crate::AdapterResult;

/// Cognitive Task phase enumeration for phase transition notifications.
/// Sec 4.2: CognitiveTask Execution Phases
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CTPhase {
    /// Task created but not yet started
    Created,
    /// Task queued for execution
    Queued,
    /// Task actively executing
    Executing,
    /// Task execution paused
    Paused,
    /// Task completed
    Completed,
    /// Task failed with error
    Failed,
}

impl CTPhase {
    /// Returns string representation of the phase.
    pub fn as_str(&self) -> &'static str {
        match self {
            CTPhase::Created => "created",
            CTPhase::Queued => "queued",
            CTPhase::Executing => "executing",
            CTPhase::Paused => "paused",
            CTPhase::Completed => "completed",
            CTPhase::Failed => "failed",
        }
    }
}

/// Unique identifier for a CognitiveTask
pub type CTID = u64;

/// Exception action directive from the adapter
/// Sec 4.2: Exception Handling Actions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExceptionAction {
    /// Retry the operation
    Retry,
    /// Escalate to upper handler
    Escalate,
    /// Abort the task
    Abort,
    /// Continue execution despite the error
    Continue,
}

impl ExceptionAction {
    /// Returns string representation of the action.
    pub fn as_str(&self) -> &'static str {
        match self {
            ExceptionAction::Retry => "retry",
            ExceptionAction::Escalate => "escalate",
            ExceptionAction::Abort => "abort",
            ExceptionAction::Continue => "continue",
        }
    }
}

/// Represents a cognitive exception from the kernel
/// Sec 4.2: CognitiveException Structure
#[derive(Debug, Clone)]
pub struct CognitiveException {
    /// Exception identifier
    pub exception_id: String,
    /// Exception type or class
    pub exception_type: String,
    /// Human-readable error message
    pub message: String,
    /// Stack trace or diagnostic information
    pub diagnostic_info: String,
}

/// Framework task trait defining what the framework provides
/// Sec 4.2: FrameworkTask Trait
pub trait FrameworkTask {
    /// Returns the framework-specific task identifier
    fn task_id(&self) -> &str;

    /// Returns the task name or description
    fn name(&self) -> &str;

    /// Returns the task objective
    fn objective(&self) -> &str;

    /// Returns timeout in milliseconds
    fn timeout_ms(&self) -> u64;

    /// Returns true if task execution is mandatory
    fn is_mandatory(&self) -> bool;
}

/// Framework memory trait defining what the framework provides
/// Sec 4.2: FrameworkMemory Trait
pub trait FrameworkMemory {
    /// Returns the framework-specific memory identifier
    fn memory_id(&self) -> &str;

    /// Returns the memory type name
    fn memory_type(&self) -> &str;

    /// Returns estimated capacity in tokens
    fn capacity_tokens(&self) -> u64;

    /// Returns the serialization format
    fn serialization_format(&self) -> &str;
}

/// Framework tool trait defining what the framework provides
/// Sec 4.2: FrameworkTool Trait
pub trait FrameworkTool {
    /// Returns the framework-specific tool identifier
    fn tool_id(&self) -> &str;

    /// Returns the tool name
    fn name(&self) -> &str;

    /// Returns the tool description
    fn description(&self) -> &str;

    /// Returns the input schema as a string
    fn input_schema(&self) -> &str;

    /// Returns the output schema as a string
    fn output_schema(&self) -> &str;

    /// Returns true if tool invocation requires explicit authorization
    fn requires_authorization(&self) -> bool;
}

/// Framework message trait defining what the framework provides
/// Sec 4.2: FrameworkMessage Trait
pub trait FrameworkMessage {
    /// Returns the message identifier
    fn message_id(&self) -> &str;

    /// Returns the source entity identifier
    fn source(&self) -> &str;

    /// Returns the target entity identifier
    fn target(&self) -> &str;

    /// Returns the message content
    fn content(&self) -> &str;

    /// Returns the communication pattern (e.g., "request-reply", "pub-sub")
    fn pattern(&self) -> &str;
}

/// Configuration for a cognitive task in kernel-compatible format
/// Sec 4.2: CTConfig Structure
#[derive(Debug, Clone)]
pub struct CTConfig {
    /// Kernel-assigned task identifier
    pub task_id: CTID,
    /// Task name
    pub name: String,
    /// Task objective
    pub objective: String,
    /// Timeout in milliseconds
    pub timeout_ms: u64,
    /// Whether task is mandatory
    pub is_mandatory: bool,
    /// Source framework type
    pub source_framework: FrameworkType,
}

/// Configuration for memory in kernel-compatible format
/// Sec 4.2: MemoryConfig Structure
#[derive(Debug, Clone)]
pub struct MemoryConfig {
    /// Kernel memory identifier
    pub memory_id: String,
    /// Memory type
    pub memory_type: String,
    /// Capacity in tokens
    pub capacity_tokens: u64,
    /// Serialization format
    pub serialization_format: String,
    /// Target memory tier (L1, L2, L3)
    pub target_tier: String,
}

/// Configuration for tool binding in kernel-compatible format
/// Sec 4.2: ToolBindingConfig Structure
#[derive(Debug, Clone)]
pub struct ToolBindingConfig {
    /// Tool identifier
    pub tool_id: String,
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: String,
    /// Input schema
    pub input_schema: String,
    /// Output schema
    pub output_schema: String,
    /// Authorization required flag
    pub requires_authorization: bool,
}

/// Configuration for IPC in kernel-compatible format
/// Sec 4.2: IpcConfig Structure
#[derive(Debug, Clone)]
pub struct IpcConfig {
    /// Channel identifier
    pub channel_id: String,
    /// Source entity
    pub source: String,
    /// Target entity
    pub target: String,
    /// Communication pattern
    pub pattern: String,
    /// Whether messages are persisted
    pub is_persistent: bool,
}

/// Translation metrics tracking adapter performance
/// Sec 5.1: Translation Metrics
#[derive(Debug, Clone)]
pub struct TranslationMetrics {
    /// Translation overhead in nanoseconds
    pub overhead_ns: u64,
    /// Accuracy score (0-100)
    pub accuracy: u8,
    /// Source framework type
    pub source_framework: FrameworkType,
    /// Timestamp of translation in milliseconds
    pub timestamp_ms: u64,
}

impl TranslationMetrics {
    /// Creates new translation metrics
    pub fn new(overhead_ns: u64, accuracy: u8, framework: FrameworkType, timestamp_ms: u64) -> Self {
        TranslationMetrics {
            overhead_ns,
            accuracy,
            source_framework: framework,
            timestamp_ms,
        }
    }

    /// Returns true if translation was high fidelity (accuracy >= 90)
    pub fn is_high_fidelity(&self) -> bool {
        self.accuracy >= 90
    }
}

/// Core runtime adapter interface
/// Sec 4.2: RuntimeAdapterRef Interface Specification
pub trait RuntimeAdapterRef {
    /// Returns the framework type this adapter handles
    /// Sec 4.2: Adapter Type Declaration
    fn adapter_type(&self) -> FrameworkType;

    /// Translates framework task to kernel configuration
    /// Sec 4.2: Task Translation Method
    fn translate_task(&self, framework_task: &dyn FrameworkTask) -> AdapterResult<CTConfig>;

    /// Translates framework memory to kernel configuration
    /// Sec 4.2: Memory Translation Method
    fn translate_memory(&self, framework_mem: &dyn FrameworkMemory) -> AdapterResult<MemoryConfig>;

    /// Translates framework tool to kernel configuration
    /// Sec 4.2: Tool Translation Method
    fn translate_tool(&self, framework_tool: &dyn FrameworkTool)
        -> AdapterResult<ToolBindingConfig>;

    /// Translates framework message to kernel IPC configuration
    /// Sec 4.2: IPC Translation Method
    fn translate_ipc(&self, framework_msg: &dyn FrameworkMessage) -> AdapterResult<IpcConfig>;

    /// Notifies adapter of phase transition in a CognitiveTask
    /// Sec 4.2: Phase Transition Notification
    ///
    /// # Arguments
    /// * `ct_id` - The CognitiveTask identifier
    /// * `old` - Previous phase
    /// * `new` - New phase
    fn on_phase_transition(&self, ct_id: CTID, old: CTPhase, new: CTPhase) -> AdapterResult<()>;

    /// Notifies adapter of exception and requests handling action
    /// Sec 4.2: Exception Notification and Action
    ///
    /// # Arguments
    /// * `ct_id` - The CognitiveTask identifier
    /// * `exception` - The exception details
    ///
    /// # Returns
    /// The action the kernel should take in response
    fn on_exception(
        &self,
        ct_id: CTID,
        exception: &CognitiveException,
    ) -> AdapterResult<ExceptionAction>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockFrameworkTask {
        task_id: String,
        name: String,
        objective: String,
        timeout_ms: u64,
        is_mandatory: bool,
    }

    impl FrameworkTask for MockFrameworkTask {
        fn task_id(&self) -> &str {
            &self.task_id
        }
        fn name(&self) -> &str {
            &self.name
        }
        fn objective(&self) -> &str {
            &self.objective
        }
        fn timeout_ms(&self) -> u64 {
            self.timeout_ms
        }
        fn is_mandatory(&self) -> bool {
            self.is_mandatory
        }
    }

    struct MockFrameworkMemory {
        memory_id: String,
        memory_type: String,
        capacity_tokens: u64,
        serialization_format: String,
    }

    impl FrameworkMemory for MockFrameworkMemory {
        fn memory_id(&self) -> &str {
            &self.memory_id
        }
        fn memory_type(&self) -> &str {
            &self.memory_type
        }
        fn capacity_tokens(&self) -> u64 {
            self.capacity_tokens
        }
        fn serialization_format(&self) -> &str {
            &self.serialization_format
        }
    }

    struct MockFrameworkTool {
        tool_id: String,
        name: String,
        description: String,
        input_schema: String,
        output_schema: String,
        requires_authorization: bool,
    }

    impl FrameworkTool for MockFrameworkTool {
        fn tool_id(&self) -> &str {
            &self.tool_id
        }
        fn name(&self) -> &str {
            &self.name
        }
        fn description(&self) -> &str {
            &self.description
        }
        fn input_schema(&self) -> &str {
            &self.input_schema
        }
        fn output_schema(&self) -> &str {
            &self.output_schema
        }
        fn requires_authorization(&self) -> bool {
            self.requires_authorization
        }
    }

    struct MockFrameworkMessage {
        message_id: String,
        source: String,
        target: String,
        content: String,
        pattern: String,
    }

    impl FrameworkMessage for MockFrameworkMessage {
        fn message_id(&self) -> &str {
            &self.message_id
        }
        fn source(&self) -> &str {
            &self.source
        }
        fn target(&self) -> &str {
            &self.target
        }
        fn content(&self) -> &str {
            &self.content
        }
        fn pattern(&self) -> &str {
            &self.pattern
        }
    }

    #[test]
    fn test_ctphase_as_str() {
        assert_eq!(CTPhase::Created.as_str(), "created");
        assert_eq!(CTPhase::Queued.as_str(), "queued");
        assert_eq!(CTPhase::Executing.as_str(), "executing");
        assert_eq!(CTPhase::Completed.as_str(), "completed");
    }

    #[test]
    fn test_exception_action_as_str() {
        assert_eq!(ExceptionAction::Retry.as_str(), "retry");
        assert_eq!(ExceptionAction::Escalate.as_str(), "escalate");
        assert_eq!(ExceptionAction::Abort.as_str(), "abort");
        assert_eq!(ExceptionAction::Continue.as_str(), "continue");
    }

    #[test]
    fn test_cognitive_exception_creation() {
        let exc = CognitiveException {
            exception_id: "exc-001".into(),
            exception_type: "TimeoutError".into(),
            message: "Task timed out".into(),
            diagnostic_info: "Task exceeded 5000ms limit".into(),
        };
        assert_eq!(exc.exception_id, "exc-001");
        assert_eq!(exc.exception_type, "TimeoutError");
    }

    #[test]
    fn test_ct_config_creation() {
        let config = CTConfig {
            task_id: 42,
            name: "TestTask".into(),
            objective: "Test objective".into(),
            timeout_ms: 5000,
            is_mandatory: true,
            source_framework: FrameworkType::LangChain,
        };
        assert_eq!(config.task_id, 42);
        assert_eq!(config.name, "TestTask");
        assert!(config.is_mandatory);
    }

    #[test]
    fn test_memory_config_creation() {
        let config = MemoryConfig {
            memory_id: "mem-001".into(),
            memory_type: "long-term".into(),
            capacity_tokens: 100000,
            serialization_format: "json".into(),
            target_tier: "L2".into(),
        };
        assert_eq!(config.memory_id, "mem-001");
        assert_eq!(config.target_tier, "L2");
    }

    #[test]
    fn test_tool_binding_config_creation() {
        let config = ToolBindingConfig {
            tool_id: "tool-001".into(),
            name: "SearchTool".into(),
            description: "Searches the web".into(),
            input_schema: "{\"query\": \"string\"}".into(),
            output_schema: "{\"results\": \"array\"}".into(),
            requires_authorization: true,
        };
        assert_eq!(config.tool_id, "tool-001");
        assert!(config.requires_authorization);
    }

    #[test]
    fn test_ipc_config_creation() {
        let config = IpcConfig {
            channel_id: "ch-001".into(),
            source: "agent-1".into(),
            target: "agent-2".into(),
            pattern: "request-reply".into(),
            is_persistent: true,
        };
        assert_eq!(config.channel_id, "ch-001");
        assert_eq!(config.pattern, "request-reply");
        assert!(config.is_persistent);
    }

    #[test]
    fn test_translation_metrics_creation() {
        let metrics =
            TranslationMetrics::new(12500, 95, FrameworkType::LangChain, 1234567890);
        assert_eq!(metrics.overhead_ns, 12500);
        assert_eq!(metrics.accuracy, 95);
        assert!(metrics.is_high_fidelity());
    }

    #[test]
    fn test_translation_metrics_fidelity_threshold() {
        let high_fidelity = TranslationMetrics::new(1000, 90, FrameworkType::LangChain, 0);
        let low_fidelity = TranslationMetrics::new(1000, 75, FrameworkType::LangChain, 0);
        assert!(high_fidelity.is_high_fidelity());
        assert!(!low_fidelity.is_high_fidelity());
    }

    #[test]
    fn test_mock_framework_task() {
        let task = MockFrameworkTask {
            task_id: "task-001".into(),
            name: "MyTask".into(),
            objective: "Do something".into(),
            timeout_ms: 5000,
            is_mandatory: true,
        };
        assert_eq!(task.task_id(), "task-001");
        assert_eq!(task.timeout_ms(), 5000);
    }

    #[test]
    fn test_mock_framework_memory() {
        let mem = MockFrameworkMemory {
            memory_id: "mem-001".into(),
            memory_type: "conversation".into(),
            capacity_tokens: 50000,
            serialization_format: "json".into(),
        };
        assert_eq!(mem.memory_id(), "mem-001");
        assert_eq!(mem.capacity_tokens(), 50000);
    }

    #[test]
    fn test_mock_framework_tool() {
        let tool = MockFrameworkTool {
            tool_id: "tool-001".into(),
            name: "WebSearch".into(),
            description: "Search the web".into(),
            input_schema: "{}".into(),
            output_schema: "{}".into(),
            requires_authorization: false,
        };
        assert_eq!(tool.tool_id(), "tool-001");
        assert_eq!(tool.name(), "WebSearch");
        assert!(!tool.requires_authorization());
    }

    #[test]
    fn test_mock_framework_message() {
        let msg = MockFrameworkMessage {
            message_id: "msg-001".into(),
            source: "agent-1".into(),
            target: "agent-2".into(),
            content: "Hello".into(),
            pattern: "request-reply".into(),
        };
        assert_eq!(msg.message_id(), "msg-001");
        assert_eq!(msg.source(), "agent-1");
        assert_eq!(msg.pattern(), "request-reply");
    }
}
