// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Cognitive Event Format (CEF) types for telemetry.
//!
//! Defines event structures for distributed tracing, auditing, and telemetry
//! across the Cognitive Substrate. Aligned with OpenTelemetry standards.
//!
//! See Engineering Plan § 2.12: Cognitive Event Format (CEF) & Telemetry
//! and Addendum v2.5.1: OpenTelemetry Alignment.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

/// Cost attribution for a cognitive event.
///
/// Tracks resource consumption (tokens, GPU time, wall-clock time) for accurate
/// metering and cost allocation across agents and crews.
///
/// See Engineering Plan § 2.12: CEF - Cost Attribution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CostAttribution {
    /// Number of tokens processed (input + output).
    pub tokens: u64,

    /// GPU compute time in milliseconds.
    pub gpu_ms: u64,

    /// Wall-clock time in milliseconds.
    pub wall_clock_ms: u64,

    /// Allocated TPC (Token Processing Compute) hours.
    pub tpc_hours: u64,
}

impl CostAttribution {
    /// Creates a new cost attribution.
    pub fn new(tokens: u64, gpu_ms: u64, wall_clock_ms: u64, tpc_hours: u64) -> Self {
        CostAttribution {
            tokens,
            gpu_ms,
            wall_clock_ms,
            tpc_hours,
        }
    }

    /// Creates a zero cost attribution (for read-only operations).
    pub fn zero() -> Self {
        CostAttribution {
            tokens: 0,
            gpu_ms: 0,
            wall_clock_ms: 0,
            tpc_hours: 0,
        }
    }

    /// Returns true if this attribution has any non-zero cost.
    pub fn has_cost(&self) -> bool {
        self.tokens > 0 || self.gpu_ms > 0 || self.wall_clock_ms > 0 || self.tpc_hours > 0
    }
}

/// CEF event type enumeration.
///
/// Categorizes different types of events in the Cognitive Substrate.
/// Each type represents a distinct operation phase or state transition.
///
/// See Engineering Plan § 2.12: CEF Event Types.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CefEventType {
    /// Thought step (reasoning/planning operation).
    ThoughtStep,

    /// Tool call requested by agent.
    ToolCallRequested,

    /// Tool call completed with result.
    ToolCallCompleted,

    /// Policy decision made (access control, capability check).
    PolicyDecision,

    /// Memory access (read/write to semantic memory).
    MemoryAccess,

    /// Inter-process communication message.
    IpcMessage,

    /// Phase transition (e.g., thinking -> action).
    PhaseTransition,

    /// Checkpoint created (state saved for recovery).
    CheckpointCreated,

    /// Signal dispatched (interruption, cancellation, etc).
    SignalDispatched,

    /// Exception raised.
    ExceptionRaised,
}

impl fmt::Display for CefEventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CefEventType::ThoughtStep => write!(f, "ThoughtStep"),
            CefEventType::ToolCallRequested => write!(f, "ToolCallRequested"),
            CefEventType::ToolCallCompleted => write!(f, "ToolCallCompleted"),
            CefEventType::PolicyDecision => write!(f, "PolicyDecision"),
            CefEventType::MemoryAccess => write!(f, "MemoryAccess"),
            CefEventType::IpcMessage => write!(f, "IpcMessage"),
            CefEventType::PhaseTransition => write!(f, "PhaseTransition"),
            CefEventType::CheckpointCreated => write!(f, "CheckpointCreated"),
            CefEventType::SignalDispatched => write!(f, "SignalDispatched"),
            CefEventType::ExceptionRaised => write!(f, "ExceptionRaised"),
        }
    }
}

/// Data classification level for event data.
///
/// Determines privacy/security handling of event data.
///
/// See Engineering Plan § 2.12: CEF - Data Classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DataClassification {
    /// Public data (no confidentiality concerns).
    Public,

    /// Internal data (restricted to organization).
    Internal,

    /// Confidential (restricted to authorized personnel).
    Confidential,

    /// Restricted/Sensitive (highest protection level).
    Restricted,
}

impl fmt::Display for DataClassification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataClassification::Public => write!(f, "Public"),
            DataClassification::Internal => write!(f, "Internal"),
            DataClassification::Confidential => write!(f, "Confidential"),
            DataClassification::Restricted => write!(f, "Restricted"),
        }
    }
}

/// Cognitive Thread Execution Phase.
///
/// See Engineering Plan § 2.12: CEF Event Structure - Phase.
/// Represents the current execution phase of a cognitive thread.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CTPhase {
    /// Thinking phase (planning, reasoning, decision-making).
    Thinking,

    /// Acting phase (tool invocation, state modification).
    Acting,

    /// Observing phase (receiving results, state inspection).
    Observing,

    /// Reflecting phase (learning, meta-reasoning).
    Reflecting,

    /// Idle/waiting phase.
    Idle,
}

impl fmt::Display for CTPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CTPhase::Thinking => write!(f, "thinking"),
            CTPhase::Acting => write!(f, "acting"),
            CTPhase::Observing => write!(f, "observing"),
            CTPhase::Reflecting => write!(f, "reflecting"),
            CTPhase::Idle => write!(f, "idle"),
        }
    }
}

/// Thought step event data.
///
/// See Engineering Plan § 2.12: Event Types - ThoughtStep.
/// Contains reasoning and planning information for a thought operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThoughtStepData {
    /// The reasoning text or planning content.
    pub reasoning_text: String,

    /// Model identifier used for this thought step.
    pub model_id: String,

    /// Context window tokens available for reasoning.
    pub context_window_tokens: u64,

    /// Number of decisions considered in this step.
    pub decisions_considered: u64,
}

impl ThoughtStepData {
    /// Creates new thought step data.
    pub fn new(
        reasoning_text: impl Into<String>,
        model_id: impl Into<String>,
        context_window_tokens: u64,
        decisions_considered: u64,
    ) -> Self {
        ThoughtStepData {
            reasoning_text: reasoning_text.into(),
            model_id: model_id.into(),
            context_window_tokens,
            decisions_considered,
        }
    }
}

/// Tool call requested event data.
///
/// See Engineering Plan § 2.12: Event Types - ToolCallRequested.
/// Captures details about a tool invocation request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolCallRequestedData {
    /// The tool binding ID for this invocation.
    pub tool_binding_id: String,

    /// The input schema/structure as a string (JSON representation).
    pub input_schema: String,

    /// Estimated cost of tool execution.
    pub estimated_cost: CostAttribution,

    /// Required capability level for this tool.
    pub capability_required: String,
}

impl ToolCallRequestedData {
    /// Creates new tool call requested data.
    pub fn new(
        tool_binding_id: impl Into<String>,
        input_schema: impl Into<String>,
        estimated_cost: CostAttribution,
        capability_required: impl Into<String>,
    ) -> Self {
        ToolCallRequestedData {
            tool_binding_id: tool_binding_id.into(),
            input_schema: input_schema.into(),
            estimated_cost,
            capability_required: capability_required.into(),
        }
    }
}

/// Tool call completed event data.
///
/// See Engineering Plan § 2.12: Event Types - ToolCallCompleted.
/// Captures the results and metrics of a completed tool invocation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolCallCompletedData {
    /// The tool binding ID that was invoked.
    pub tool_binding_id: String,

    /// Actual cost incurred by execution.
    pub actual_cost: CostAttribution,

    /// The output schema/structure as a string (JSON representation).
    pub output_schema: String,

    /// Execution time in milliseconds.
    pub execution_time_ms: u64,

    /// Whether the result was served from cache.
    pub response_cached: bool,
}

impl ToolCallCompletedData {
    /// Creates new tool call completed data.
    pub fn new(
        tool_binding_id: impl Into<String>,
        actual_cost: CostAttribution,
        output_schema: impl Into<String>,
        execution_time_ms: u64,
        response_cached: bool,
    ) -> Self {
        ToolCallCompletedData {
            tool_binding_id: tool_binding_id.into(),
            actual_cost,
            output_schema: output_schema.into(),
            execution_time_ms,
            response_cached,
        }
    }
}

/// Policy decision outcome.
///
/// See Engineering Plan § 2.12: Event Types - PolicyDecision.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PolicyOutcome {
    /// Operation allowed to proceed.
    Allow,

    /// Operation denied.
    Deny,

    /// User approval required before proceeding.
    RequireApproval,

    /// Operation allowed but audit required.
    Audit,

    /// Operation allowed with warning.
    Warn,
}

impl fmt::Display for PolicyOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PolicyOutcome::Allow => write!(f, "Allow"),
            PolicyOutcome::Deny => write!(f, "Deny"),
            PolicyOutcome::RequireApproval => write!(f, "RequireApproval"),
            PolicyOutcome::Audit => write!(f, "Audit"),
            PolicyOutcome::Warn => write!(f, "Warn"),
        }
    }
}

/// Policy decision event data.
///
/// See Engineering Plan § 2.12: Event Types - PolicyDecision.
/// Tracks policy decisions for access control, capability checks, and compliance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolicyDecisionData {
    /// Type of policy decision (e.g., "access_control", "capability_check").
    pub decision_type: String,

    /// ID of the policy rule applied.
    pub rule_id: String,

    /// Hash of the policy version for compliance tracking.
    pub policy_version_hash: String,

    /// The outcome of the policy decision.
    pub outcome: PolicyOutcome,

    /// Machine-readable reason code for the decision.
    pub reason_code: String,

    /// User-readable explanation (may be redacted per EU AI Act Art 12).
    pub explanation_redacted: String,
}

impl PolicyDecisionData {
    /// Creates new policy decision data.
    pub fn new(
        decision_type: impl Into<String>,
        rule_id: impl Into<String>,
        policy_version_hash: impl Into<String>,
        outcome: PolicyOutcome,
        reason_code: impl Into<String>,
        explanation_redacted: impl Into<String>,
    ) -> Self {
        PolicyDecisionData {
            decision_type: decision_type.into(),
            rule_id: rule_id.into(),
            policy_version_hash: policy_version_hash.into(),
            outcome,
            reason_code: reason_code.into(),
            explanation_redacted: explanation_redacted.into(),
        }
    }
}

/// Memory access type.
///
/// See Engineering Plan § 2.12: Event Types - MemoryAccess.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MemoryAccessType {
    /// Read-only access.
    Read,

    /// Write access.
    Write,
}

impl fmt::Display for MemoryAccessType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemoryAccessType::Read => write!(f, "Read"),
            MemoryAccessType::Write => write!(f, "Write"),
        }
    }
}

/// Memory access event data.
///
/// See Engineering Plan § 2.12: Event Types - MemoryAccess.
/// Tracks access to semantic memory and state storage.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryAccessData {
    /// Memory address range accessed.
    pub address_range: String,

    /// Type of access (read or write).
    pub access_type: MemoryAccessType,

    /// Size of access in bytes.
    pub size_bytes: u64,

    /// Memory tier accessed (e.g., "L1", "L2", "L3", "persistent").
    pub tier: String,

    /// Reference to checkpoint if this is recovery-related.
    pub checkpoint_ref: Option<String>,
}

impl MemoryAccessData {
    /// Creates new memory access data.
    pub fn new(
        address_range: impl Into<String>,
        access_type: MemoryAccessType,
        size_bytes: u64,
        tier: impl Into<String>,
    ) -> Self {
        MemoryAccessData {
            address_range: address_range.into(),
            access_type,
            size_bytes,
            tier: tier.into(),
            checkpoint_ref: None,
        }
    }

    /// Sets the checkpoint reference.
    pub fn with_checkpoint(mut self, checkpoint_ref: impl Into<String>) -> Self {
        self.checkpoint_ref = Some(checkpoint_ref.into());
        self
    }
}

/// IPC message delivery status.
///
/// See Engineering Plan § 2.12: Event Types - IpcMessage.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DeliveryStatus {
    /// Message sent.
    Sent,

    /// Message received.
    Received,

    /// Message delivery failed.
    Failed,
}

impl fmt::Display for DeliveryStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeliveryStatus::Sent => write!(f, "Sent"),
            DeliveryStatus::Received => write!(f, "Received"),
            DeliveryStatus::Failed => write!(f, "Failed"),
        }
    }
}

/// Inter-process communication event data.
///
/// See Engineering Plan § 2.12: Event Types - IpcMessage.
/// Tracks message passing between agents and processes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IpcMessageData {
    /// Sender agent identifier.
    pub sender_agent: String,

    /// Receiver agent identifier.
    pub receiver_agent: String,

    /// Channel identifier for this message.
    pub channel_id: String,

    /// Size of the message in bytes.
    pub message_size_bytes: u64,

    /// Delivery status of the message.
    pub delivery_status: DeliveryStatus,
}

impl IpcMessageData {
    /// Creates new IPC message data.
    pub fn new(
        sender_agent: impl Into<String>,
        receiver_agent: impl Into<String>,
        channel_id: impl Into<String>,
        message_size_bytes: u64,
        delivery_status: DeliveryStatus,
    ) -> Self {
        IpcMessageData {
            sender_agent: sender_agent.into(),
            receiver_agent: receiver_agent.into(),
            channel_id: channel_id.into(),
            message_size_bytes,
            delivery_status,
        }
    }
}

/// Phase transition event data.
///
/// See Engineering Plan § 2.12: Event Types - PhaseTransition.
/// Tracks transitions between cognitive thread execution phases.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhaseTransitionData {
    /// The phase being transitioned from.
    pub old_phase: String,

    /// The phase being transitioned to.
    pub new_phase: String,

    /// Reason for the phase transition.
    pub transition_reason: String,

    /// Duration spent in the old phase (milliseconds).
    pub duration_in_old_phase_ms: u64,
}

impl PhaseTransitionData {
    /// Creates new phase transition data.
    pub fn new(
        old_phase: impl Into<String>,
        new_phase: impl Into<String>,
        transition_reason: impl Into<String>,
        duration_in_old_phase_ms: u64,
    ) -> Self {
        PhaseTransitionData {
            old_phase: old_phase.into(),
            new_phase: new_phase.into(),
            transition_reason: transition_reason.into(),
            duration_in_old_phase_ms,
        }
    }
}

/// Checkpoint created event data.
///
/// See Engineering Plan § 2.12: Event Types - CheckpointCreated.
/// Records creation of recovery checkpoints for state snapshots.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckpointCreatedData {
    /// Unique identifier for this checkpoint.
    pub checkpoint_id: String,

    /// Size of checkpoint data in bytes.
    pub size_bytes: u64,

    /// Amount of memory committed in this checkpoint.
    pub memory_committed: u64,

    /// Whether GPU state was committed in this checkpoint.
    pub gpu_state_committed: bool,

    /// Whether this is an incremental checkpoint.
    pub incremental: bool,
}

impl CheckpointCreatedData {
    /// Creates new checkpoint created data.
    pub fn new(
        checkpoint_id: impl Into<String>,
        size_bytes: u64,
        memory_committed: u64,
        gpu_state_committed: bool,
        incremental: bool,
    ) -> Self {
        CheckpointCreatedData {
            checkpoint_id: checkpoint_id.into(),
            size_bytes,
            memory_committed,
            gpu_state_committed,
            incremental,
        }
    }
}

/// Signal dispatch mode.
///
/// See Engineering Plan § 2.12: Event Types - SignalDispatched.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SignalDeliveryMode {
    /// Immediate delivery (synchronous).
    Immediate,

    /// Queued delivery (asynchronous).
    Queued,
}

impl fmt::Display for SignalDeliveryMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SignalDeliveryMode::Immediate => write!(f, "Immediate"),
            SignalDeliveryMode::Queued => write!(f, "Queued"),
        }
    }
}

/// Signal dispatched event data.
///
/// See Engineering Plan § 2.12: Event Types - SignalDispatched.
/// Tracks delivery of control signals (interrupts, cancellation, etc).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignalDispatchedData {
    /// Type of signal (e.g., "interrupt", "cancel", "checkpoint").
    pub signal_type: String,

    /// Target agent for this signal.
    pub target_agent: String,

    /// Size of the signal payload in bytes.
    pub payload_size: u64,

    /// Signal delivery mode (immediate or queued).
    pub delivery_mode: SignalDeliveryMode,
}

impl SignalDispatchedData {
    /// Creates new signal dispatched data.
    pub fn new(
        signal_type: impl Into<String>,
        target_agent: impl Into<String>,
        payload_size: u64,
        delivery_mode: SignalDeliveryMode,
    ) -> Self {
        SignalDispatchedData {
            signal_type: signal_type.into(),
            target_agent: target_agent.into(),
            payload_size,
            delivery_mode,
        }
    }
}

/// Exception severity level.
///
/// See Engineering Plan § 2.12: Event Types - ExceptionRaised.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ExceptionSeverity {
    /// Low severity (informational).
    Low,

    /// Medium severity (warnings).
    Medium,

    /// High severity (errors).
    High,

    /// Critical severity (system-threatening).
    Critical,
}

impl fmt::Display for ExceptionSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExceptionSeverity::Low => write!(f, "Low"),
            ExceptionSeverity::Medium => write!(f, "Medium"),
            ExceptionSeverity::High => write!(f, "High"),
            ExceptionSeverity::Critical => write!(f, "Critical"),
        }
    }
}

/// Exception raised event data.
///
/// See Engineering Plan § 2.12: Event Types - ExceptionRaised.
/// Records exceptions, errors, and failure conditions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExceptionRaisedData {
    /// Type of exception (e.g., "RuntimeError", "TimeoutError", "PermissionDenied").
    pub exception_type: String,

    /// Severity of the exception.
    pub severity: ExceptionSeverity,

    /// Human-readable error message.
    pub error_message: String,

    /// Whether recovery was attempted.
    pub recovery_attempted: bool,

    /// Outcome of recovery attempt if made.
    pub recovery_outcome: Option<String>,
}

impl ExceptionRaisedData {
    /// Creates new exception raised data.
    pub fn new(
        exception_type: impl Into<String>,
        severity: ExceptionSeverity,
        error_message: impl Into<String>,
        recovery_attempted: bool,
    ) -> Self {
        ExceptionRaisedData {
            exception_type: exception_type.into(),
            severity,
            error_message: error_message.into(),
            recovery_attempted,
            recovery_outcome: None,
        }
    }

    /// Sets the recovery outcome.
    pub fn with_recovery_outcome(mut self, outcome: impl Into<String>) -> Self {
        self.recovery_outcome = Some(outcome.into());
        self
    }
}

/// Cognitive Event Format event structure.
///
/// Base event structure for all telemetry in the Cognitive Substrate.
/// Aligned with OpenTelemetry standards for distributed tracing.
///
/// See Engineering Plan § 2.12: Cognitive Event Format (CEF).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CefEvent {
    /// Globally unique event identifier (ULID).
    ///
    /// See Engineering Plan § 2.12: Event Identification.
    pub event_id: String,

    /// Trace ID (128-bit, hex-encoded).
    ///
    /// Links events across distributed systems.
    /// See OpenTelemetry specification.
    pub trace_id: String,

    /// Span ID (64-bit, hex-encoded).
    ///
    /// Identifies a specific operation within a trace.
    /// See OpenTelemetry specification.
    pub span_id: String,

    /// Cognitive Thread ID.
    ///
    /// Identifies the CT (cognitive thread) that generated this event.
    pub ct_id: String,

    /// Agent ID.
    ///
    /// Principal that initiated or triggered the event.
    pub agent_id: String,

    /// Crew ID.
    ///
    /// Collaborative crew context if applicable.
    pub crew_id: Option<String>,

    /// Timestamp in nanoseconds since epoch.
    pub timestamp_ns: u64,

    /// Type of event.
    pub event_type: CefEventType,

    /// Phase in execution (e.g., "thinking", "acting", "observing").
    pub phase: String,

    /// Cost attribution for this event.
    pub cost: CostAttribution,

    /// Data classification level.
    pub data_classification: DataClassification,

    /// Optional additional event metadata.
    pub metadata: Option<String>,
}

impl CefEvent {
    /// Creates a new CEF event.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        event_id: impl Into<String>,
        trace_id: impl Into<String>,
        span_id: impl Into<String>,
        ct_id: impl Into<String>,
        agent_id: impl Into<String>,
        timestamp_ns: u64,
        event_type: CefEventType,
        phase: impl Into<String>,
    ) -> Self {
        CefEvent {
            event_id: event_id.into(),
            trace_id: trace_id.into(),
            span_id: span_id.into(),
            ct_id: ct_id.into(),
            agent_id: agent_id.into(),
            crew_id: None,
            timestamp_ns,
            event_type,
            phase: phase.into(),
            cost: CostAttribution::zero(),
            data_classification: DataClassification::Internal,
            metadata: None,
        }
    }

    /// Sets the crew ID for this event.
    pub fn with_crew(mut self, crew_id: impl Into<String>) -> Self {
        self.crew_id = Some(crew_id.into());
        self
    }

    /// Sets the cost attribution for this event.
    pub fn with_cost(mut self, cost: CostAttribution) -> Self {
        self.cost = cost;
        self
    }

    /// Sets the data classification for this event.
    pub fn with_classification(mut self, classification: DataClassification) -> Self {
        self.data_classification = classification;
        self
    }

    /// Sets metadata for this event.
    pub fn with_metadata(mut self, metadata: impl Into<String>) -> Self {
        self.metadata = Some(metadata.into());
        self
    }

    /// Returns true if this event represents a tool invocation (request or completion).
    pub fn is_tool_event(&self) -> bool {
        matches!(
            self.event_type,
            CefEventType::ToolCallRequested | CefEventType::ToolCallCompleted
        )
    }

    /// Returns true if this event is a policy-related event.
    pub fn is_policy_event(&self) -> bool {
        matches!(
            self.event_type,
            CefEventType::PolicyDecision
        )
    }

    /// Returns true if this event represents an error or exception.
    pub fn is_error_event(&self) -> bool {
        matches!(
            self.event_type,
            CefEventType::ExceptionRaised
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::string::ToString;

    #[test]
    fn test_cost_attribution_creation() {
        let cost = CostAttribution::new(1000, 100, 50, 1);
        assert_eq!(cost.tokens, 1000);
        assert_eq!(cost.gpu_ms, 100);
        assert_eq!(cost.wall_clock_ms, 50);
        assert_eq!(cost.tpc_hours, 1);
    }

    #[test]
    fn test_cost_attribution_zero() {
        let cost = CostAttribution::zero();
        assert_eq!(cost.tokens, 0);
        assert_eq!(cost.gpu_ms, 0);
        assert_eq!(cost.wall_clock_ms, 0);
        assert_eq!(cost.tpc_hours, 0);
        assert!(!cost.has_cost());
    }

    #[test]
    fn test_cost_attribution_has_cost() {
        let zero = CostAttribution::zero();
        assert!(!zero.has_cost());

        let with_tokens = CostAttribution::new(1, 0, 0, 0);
        assert!(with_tokens.has_cost());

        let with_gpu = CostAttribution::new(0, 1, 0, 0);
        assert!(with_gpu.has_cost());

        let with_wall_clock = CostAttribution::new(0, 0, 1, 0);
        assert!(with_wall_clock.has_cost());

        let with_tpc = CostAttribution::new(0, 0, 0, 1);
        assert!(with_tpc.has_cost());
    }

    #[test]
    fn test_cef_event_type_display() {
        assert_eq!(CefEventType::ThoughtStep.to_string(), "ThoughtStep");
        assert_eq!(
            CefEventType::ToolCallRequested.to_string(),
            "ToolCallRequested"
        );
        assert_eq!(
            CefEventType::ToolCallCompleted.to_string(),
            "ToolCallCompleted"
        );
        assert_eq!(CefEventType::PolicyDecision.to_string(), "PolicyDecision");
        assert_eq!(CefEventType::MemoryAccess.to_string(), "MemoryAccess");
        assert_eq!(CefEventType::IpcMessage.to_string(), "IpcMessage");
        assert_eq!(
            CefEventType::PhaseTransition.to_string(),
            "PhaseTransition"
        );
        assert_eq!(
            CefEventType::CheckpointCreated.to_string(),
            "CheckpointCreated"
        );
        assert_eq!(CefEventType::SignalDispatched.to_string(), "SignalDispatched");
        assert_eq!(
            CefEventType::ExceptionRaised.to_string(),
            "ExceptionRaised"
        );
    }

    #[test]
    fn test_data_classification_display() {
        assert_eq!(DataClassification::Public.to_string(), "Public");
        assert_eq!(DataClassification::Internal.to_string(), "Internal");
        assert_eq!(DataClassification::Confidential.to_string(), "Confidential");
        assert_eq!(DataClassification::Restricted.to_string(), "Restricted");
    }

    #[test]
    fn test_cef_event_creation() {
        let event = CefEvent::new(
            "01ARZ3NDEKTSV4RRFFQ69G5FAV",
            "0af7651916cd43dd8448eb211c80319c",
            "b9c7c989f97918e1",
            "ct-001",
            "agent-001",
            1000000000,
            CefEventType::ToolCallRequested,
            "acting",
        );

        assert_eq!(event.event_id, "01ARZ3NDEKTSV4RRFFQ69G5FAV");
        assert_eq!(event.ct_id, "ct-001");
        assert_eq!(event.agent_id, "agent-001");
        assert_eq!(event.timestamp_ns, 1000000000);
        assert_eq!(event.event_type, CefEventType::ToolCallRequested);
        assert_eq!(event.phase, "acting");
        assert_eq!(event.crew_id, None);
    }

    #[test]
    fn test_cef_event_with_crew() {
        let event = CefEvent::new(
            "event-1",
            "trace-1",
            "span-1",
            "ct-1",
            "agent-1",
            1000,
            CefEventType::ThoughtStep,
            "thinking",
        )
        .with_crew("crew-1");

        assert_eq!(event.crew_id, Some("crew-1".to_string()));
    }

    #[test]
    fn test_cef_event_with_cost() {
        let cost = CostAttribution::new(1000, 100, 50, 1);
        let event = CefEvent::new(
            "event-1",
            "trace-1",
            "span-1",
            "ct-1",
            "agent-1",
            1000,
            CefEventType::ToolCallCompleted,
            "acting",
        )
        .with_cost(cost.clone());

        assert_eq!(event.cost, cost);
    }

    #[test]
    fn test_cef_event_with_classification() {
        let event = CefEvent::new(
            "event-1",
            "trace-1",
            "span-1",
            "ct-1",
            "agent-1",
            1000,
            CefEventType::ThoughtStep,
            "thinking",
        )
        .with_classification(DataClassification::Confidential);

        assert_eq!(event.data_classification, DataClassification::Confidential);
    }

    #[test]
    fn test_cef_event_with_metadata() {
        let event = CefEvent::new(
            "event-1",
            "trace-1",
            "span-1",
            "ct-1",
            "agent-1",
            1000,
            CefEventType::ThoughtStep,
            "thinking",
        )
        .with_metadata("custom metadata here");

        assert_eq!(event.metadata, Some("custom metadata here".to_string()));
    }

    #[test]
    fn test_cef_event_is_tool_event() {
        let tool_req = CefEvent::new(
            "e1",
            "t1",
            "s1",
            "ct1",
            "a1",
            100,
            CefEventType::ToolCallRequested,
            "phase",
        );
        assert!(tool_req.is_tool_event());

        let tool_comp = CefEvent::new(
            "e2",
            "t1",
            "s1",
            "ct1",
            "a1",
            200,
            CefEventType::ToolCallCompleted,
            "phase",
        );
        assert!(tool_comp.is_tool_event());

        let thought = CefEvent::new(
            "e3",
            "t1",
            "s1",
            "ct1",
            "a1",
            300,
            CefEventType::ThoughtStep,
            "phase",
        );
        assert!(!thought.is_tool_event());
    }

    #[test]
    fn test_cef_event_is_policy_event() {
        let policy = CefEvent::new(
            "e1",
            "t1",
            "s1",
            "ct1",
            "a1",
            100,
            CefEventType::PolicyDecision,
            "phase",
        );
        assert!(policy.is_policy_event());

        let tool = CefEvent::new(
            "e2",
            "t1",
            "s1",
            "ct1",
            "a1",
            200,
            CefEventType::ToolCallRequested,
            "phase",
        );
        assert!(!tool.is_policy_event());
    }

    #[test]
    fn test_cef_event_is_error_event() {
        let error = CefEvent::new(
            "e1",
            "t1",
            "s1",
            "ct1",
            "a1",
            100,
            CefEventType::ExceptionRaised,
            "phase",
        );
        assert!(error.is_error_event());

        let success = CefEvent::new(
            "e2",
            "t1",
            "s1",
            "ct1",
            "a1",
            200,
            CefEventType::ToolCallCompleted,
            "phase",
        );
        assert!(!success.is_error_event());
    }

    #[test]
    fn test_cost_attribution_equality() {
        let c1 = CostAttribution::new(1000, 100, 50, 1);
        let c2 = CostAttribution::new(1000, 100, 50, 1);
        assert_eq!(c1, c2);

        let c3 = CostAttribution::new(2000, 100, 50, 1);
        assert_ne!(c1, c3);
    }

    #[test]
    fn test_cef_event_equality() {
        let e1 = CefEvent::new(
            "event-1",
            "trace-1",
            "span-1",
            "ct-1",
            "agent-1",
            1000,
            CefEventType::ThoughtStep,
            "thinking",
        );
        let e2 = CefEvent::new(
            "event-1",
            "trace-1",
            "span-1",
            "ct-1",
            "agent-1",
            1000,
            CefEventType::ThoughtStep,
            "thinking",
        );
        assert_eq!(e1, e2);

        let e3 = CefEvent::new(
            "event-2",
            "trace-1",
            "span-1",
            "ct-1",
            "agent-1",
            1000,
            CefEventType::ThoughtStep,
            "thinking",
        );
        assert_ne!(e1, e3);
    }

    #[test]
    fn test_cef_event_builder_chain() {
        let event = CefEvent::new(
            "event-1",
            "trace-1",
            "span-1",
            "ct-1",
            "agent-1",
            1000,
            CefEventType::ToolCallCompleted,
            "acting",
        )
        .with_crew("crew-1")
        .with_cost(CostAttribution::new(500, 50, 25, 1))
        .with_classification(DataClassification::Confidential)
        .with_metadata("important tool invocation");

        assert_eq!(event.crew_id, Some("crew-1".to_string()));
        assert_eq!(event.cost.tokens, 500);
        assert_eq!(event.data_classification, DataClassification::Confidential);
        assert_eq!(event.metadata, Some("important tool invocation".to_string()));
    }

    #[test]
    fn test_cef_event_type_equality() {
        assert_eq!(CefEventType::ThoughtStep, CefEventType::ThoughtStep);
        assert_ne!(CefEventType::ThoughtStep, CefEventType::PolicyDecision);
    }

    #[test]
    fn test_data_classification_equality() {
        assert_eq!(DataClassification::Public, DataClassification::Public);
        assert_ne!(DataClassification::Public, DataClassification::Restricted);
    }

    // Tests for CTPhase enum
    #[test]
    fn test_ct_phase_display() {
        assert_eq!(CTPhase::Thinking.to_string(), "thinking");
        assert_eq!(CTPhase::Acting.to_string(), "acting");
        assert_eq!(CTPhase::Observing.to_string(), "observing");
        assert_eq!(CTPhase::Reflecting.to_string(), "reflecting");
        assert_eq!(CTPhase::Idle.to_string(), "idle");
    }

    #[test]
    fn test_ct_phase_equality() {
        assert_eq!(CTPhase::Thinking, CTPhase::Thinking);
        assert_ne!(CTPhase::Thinking, CTPhase::Acting);
    }

    // Tests for ThoughtStepData (10+ tests per event type)
    #[test]
    fn test_thought_step_data_creation() {
        let data = ThoughtStepData::new("reasoning content", "gpt-4", 8000, 5);
        assert_eq!(data.reasoning_text, "reasoning content");
        assert_eq!(data.model_id, "gpt-4");
        assert_eq!(data.context_window_tokens, 8000);
        assert_eq!(data.decisions_considered, 5);
    }

    #[test]
    fn test_thought_step_data_clone() {
        let data = ThoughtStepData::new("reasoning", "model-1", 1000, 3);
        let cloned = data.clone();
        assert_eq!(data, cloned);
    }

    #[test]
    fn test_thought_step_data_equality() {
        let data1 = ThoughtStepData::new("reasoning", "model-1", 1000, 3);
        let data2 = ThoughtStepData::new("reasoning", "model-1", 1000, 3);
        assert_eq!(data1, data2);
    }

    #[test]
    fn test_thought_step_data_inequality() {
        let data1 = ThoughtStepData::new("reasoning-1", "model-1", 1000, 3);
        let data2 = ThoughtStepData::new("reasoning-2", "model-1", 1000, 3);
        assert_ne!(data1, data2);
    }

    #[test]
    fn test_thought_step_data_zero_decisions() {
        let data = ThoughtStepData::new("reasoning", "model", 0, 0);
        assert_eq!(data.decisions_considered, 0);
    }

    #[test]
    fn test_thought_step_data_large_context() {
        let data = ThoughtStepData::new("reasoning", "gpt-4-128k", 128000, 1000);
        assert_eq!(data.context_window_tokens, 128000);
    }

    // Tests for ToolCallRequestedData
    #[test]
    fn test_tool_call_requested_data_creation() {
        let cost = CostAttribution::new(100, 50, 25, 1);
        let data = ToolCallRequestedData::new(
            "binding-001",
            r#"{"url": "string"}"#,
            cost.clone(),
            "read_files",
        );
        assert_eq!(data.tool_binding_id, "binding-001");
        assert_eq!(data.input_schema, r#"{"url": "string"}"#);
        assert_eq!(data.estimated_cost, cost);
        assert_eq!(data.capability_required, "read_files");
    }

    #[test]
    fn test_tool_call_requested_data_equality() {
        let cost = CostAttribution::new(100, 50, 25, 1);
        let data1 = ToolCallRequestedData::new("binding-001", "{}", cost.clone(), "cap");
        let data2 = ToolCallRequestedData::new("binding-001", "{}", cost.clone(), "cap");
        assert_eq!(data1, data2);
    }

    #[test]
    fn test_tool_call_requested_data_clone() {
        let cost = CostAttribution::new(100, 50, 25, 1);
        let data = ToolCallRequestedData::new("binding-001", "{}", cost, "cap");
        let cloned = data.clone();
        assert_eq!(data, cloned);
    }

    #[test]
    fn test_tool_call_requested_data_zero_cost() {
        let cost = CostAttribution::zero();
        let data = ToolCallRequestedData::new("binding", "{}", cost, "none");
        assert!(!data.estimated_cost.has_cost());
    }

    #[test]
    fn test_tool_call_requested_data_complex_schema() {
        let schema = r#"{"type":"object","properties":{"url":{"type":"string"},"method":{"type":"string"}}}"#;
        let data =
            ToolCallRequestedData::new("binding-001", schema, CostAttribution::zero(), "http");
        assert!(data.input_schema.contains("properties"));
    }

    // Tests for ToolCallCompletedData
    #[test]
    fn test_tool_call_completed_data_creation() {
        let cost = CostAttribution::new(100, 50, 25, 1);
        let data = ToolCallCompletedData::new(
            "binding-001",
            cost.clone(),
            r#"{"status": "success"}"#,
            1500,
            false,
        );
        assert_eq!(data.tool_binding_id, "binding-001");
        assert_eq!(data.actual_cost, cost);
        assert_eq!(data.output_schema, r#"{"status": "success"}"#);
        assert_eq!(data.execution_time_ms, 1500);
        assert!(!data.response_cached);
    }

    #[test]
    fn test_tool_call_completed_data_cached() {
        let cost = CostAttribution::zero();
        let data = ToolCallCompletedData::new("binding-001", cost, "{}", 10, true);
        assert!(data.response_cached);
    }

    #[test]
    fn test_tool_call_completed_data_equality() {
        let cost = CostAttribution::new(100, 50, 25, 1);
        let data1 = ToolCallCompletedData::new("binding-001", cost.clone(), "{}", 100, false);
        let data2 = ToolCallCompletedData::new("binding-001", cost.clone(), "{}", 100, false);
        assert_eq!(data1, data2);
    }

    #[test]
    fn test_tool_call_completed_data_different_times() {
        let cost = CostAttribution::zero();
        let data1 = ToolCallCompletedData::new("binding-001", cost.clone(), "{}", 100, false);
        let data2 = ToolCallCompletedData::new("binding-001", cost.clone(), "{}", 200, false);
        assert_ne!(data1, data2);
    }

    // Tests for PolicyOutcome
    #[test]
    fn test_policy_outcome_display() {
        assert_eq!(PolicyOutcome::Allow.to_string(), "Allow");
        assert_eq!(PolicyOutcome::Deny.to_string(), "Deny");
        assert_eq!(PolicyOutcome::RequireApproval.to_string(), "RequireApproval");
        assert_eq!(PolicyOutcome::Audit.to_string(), "Audit");
        assert_eq!(PolicyOutcome::Warn.to_string(), "Warn");
    }

    #[test]
    fn test_policy_outcome_equality() {
        assert_eq!(PolicyOutcome::Allow, PolicyOutcome::Allow);
        assert_ne!(PolicyOutcome::Allow, PolicyOutcome::Deny);
    }

    // Tests for PolicyDecisionData
    #[test]
    fn test_policy_decision_data_creation() {
        let data = PolicyDecisionData::new(
            "access_control",
            "rule-001",
            "hash-123",
            PolicyOutcome::Allow,
            "approved",
            "Access approved by policy",
        );
        assert_eq!(data.decision_type, "access_control");
        assert_eq!(data.rule_id, "rule-001");
        assert_eq!(data.outcome, PolicyOutcome::Allow);
    }

    #[test]
    fn test_policy_decision_data_denied() {
        let data = PolicyDecisionData::new(
            "capability_check",
            "rule-002",
            "hash-456",
            PolicyOutcome::Deny,
            "insufficient_capability",
            "Insufficient capability level",
        );
        assert_eq!(data.outcome, PolicyOutcome::Deny);
    }

    #[test]
    fn test_policy_decision_data_equality() {
        let data1 = PolicyDecisionData::new(
            "type",
            "rule",
            "hash",
            PolicyOutcome::Allow,
            "code",
            "explanation",
        );
        let data2 = PolicyDecisionData::new(
            "type",
            "rule",
            "hash",
            PolicyOutcome::Allow,
            "code",
            "explanation",
        );
        assert_eq!(data1, data2);
    }

    #[test]
    fn test_policy_decision_data_clone() {
        let data = PolicyDecisionData::new(
            "type",
            "rule",
            "hash",
            PolicyOutcome::RequireApproval,
            "code",
            "explanation",
        );
        let cloned = data.clone();
        assert_eq!(data, cloned);
    }

    // Tests for MemoryAccessType
    #[test]
    fn test_memory_access_type_display() {
        assert_eq!(MemoryAccessType::Read.to_string(), "Read");
        assert_eq!(MemoryAccessType::Write.to_string(), "Write");
    }

    #[test]
    fn test_memory_access_type_equality() {
        assert_eq!(MemoryAccessType::Read, MemoryAccessType::Read);
        assert_ne!(MemoryAccessType::Read, MemoryAccessType::Write);
    }

    // Tests for MemoryAccessData
    #[test]
    fn test_memory_access_data_creation() {
        let data = MemoryAccessData::new("0x1000-0x2000", MemoryAccessType::Read, 4096, "L1");
        assert_eq!(data.address_range, "0x1000-0x2000");
        assert_eq!(data.access_type, MemoryAccessType::Read);
        assert_eq!(data.size_bytes, 4096);
        assert_eq!(data.tier, "L1");
        assert_eq!(data.checkpoint_ref, None);
    }

    #[test]
    fn test_memory_access_data_with_checkpoint() {
        let data = MemoryAccessData::new("0x3000-0x4000", MemoryAccessType::Write, 1024, "L2")
            .with_checkpoint("checkpoint-001");
        assert_eq!(data.checkpoint_ref, Some("checkpoint-001".to_string()));
    }

    #[test]
    fn test_memory_access_data_equality() {
        let data1 = MemoryAccessData::new("0x1000", MemoryAccessType::Read, 100, "tier");
        let data2 = MemoryAccessData::new("0x1000", MemoryAccessType::Read, 100, "tier");
        assert_eq!(data1, data2);
    }

    #[test]
    fn test_memory_access_data_clone() {
        let data = MemoryAccessData::new("0x1000", MemoryAccessType::Write, 256, "L3");
        let cloned = data.clone();
        assert_eq!(data, cloned);
    }

    // Tests for DeliveryStatus
    #[test]
    fn test_delivery_status_display() {
        assert_eq!(DeliveryStatus::Sent.to_string(), "Sent");
        assert_eq!(DeliveryStatus::Received.to_string(), "Received");
        assert_eq!(DeliveryStatus::Failed.to_string(), "Failed");
    }

    #[test]
    fn test_delivery_status_equality() {
        assert_eq!(DeliveryStatus::Sent, DeliveryStatus::Sent);
        assert_ne!(DeliveryStatus::Sent, DeliveryStatus::Received);
    }

    // Tests for IpcMessageData
    #[test]
    fn test_ipc_message_data_creation() {
        let data = IpcMessageData::new(
            "agent-001",
            "agent-002",
            "channel-a",
            512,
            DeliveryStatus::Sent,
        );
        assert_eq!(data.sender_agent, "agent-001");
        assert_eq!(data.receiver_agent, "agent-002");
        assert_eq!(data.channel_id, "channel-a");
        assert_eq!(data.message_size_bytes, 512);
        assert_eq!(data.delivery_status, DeliveryStatus::Sent);
    }

    #[test]
    fn test_ipc_message_data_equality() {
        let data1 = IpcMessageData::new("a1", "a2", "ch", 100, DeliveryStatus::Received);
        let data2 = IpcMessageData::new("a1", "a2", "ch", 100, DeliveryStatus::Received);
        assert_eq!(data1, data2);
    }

    #[test]
    fn test_ipc_message_data_clone() {
        let data = IpcMessageData::new("a1", "a2", "ch", 100, DeliveryStatus::Failed);
        let cloned = data.clone();
        assert_eq!(data, cloned);
    }

    // Tests for PhaseTransitionData
    #[test]
    fn test_phase_transition_data_creation() {
        let data = PhaseTransitionData::new("thinking", "acting", "tool_invoked", 5000);
        assert_eq!(data.old_phase, "thinking");
        assert_eq!(data.new_phase, "acting");
        assert_eq!(data.transition_reason, "tool_invoked");
        assert_eq!(data.duration_in_old_phase_ms, 5000);
    }

    #[test]
    fn test_phase_transition_data_equality() {
        let data1 = PhaseTransitionData::new("old", "new", "reason", 100);
        let data2 = PhaseTransitionData::new("old", "new", "reason", 100);
        assert_eq!(data1, data2);
    }

    #[test]
    fn test_phase_transition_data_clone() {
        let data = PhaseTransitionData::new("observing", "reflecting", "result_received", 2000);
        let cloned = data.clone();
        assert_eq!(data, cloned);
    }

    #[test]
    fn test_phase_transition_data_zero_duration() {
        let data = PhaseTransitionData::new("a", "b", "reason", 0);
        assert_eq!(data.duration_in_old_phase_ms, 0);
    }

    // Tests for CheckpointCreatedData
    #[test]
    fn test_checkpoint_created_data_creation() {
        let data = CheckpointCreatedData::new("checkpoint-001", 1024, 512, true, false);
        assert_eq!(data.checkpoint_id, "checkpoint-001");
        assert_eq!(data.size_bytes, 1024);
        assert_eq!(data.memory_committed, 512);
        assert!(data.gpu_state_committed);
        assert!(!data.incremental);
    }

    #[test]
    fn test_checkpoint_created_data_incremental() {
        let data = CheckpointCreatedData::new("checkpoint-002", 256, 128, false, true);
        assert!(data.incremental);
    }

    #[test]
    fn test_checkpoint_created_data_equality() {
        let data1 = CheckpointCreatedData::new("cp-1", 100, 50, true, false);
        let data2 = CheckpointCreatedData::new("cp-1", 100, 50, true, false);
        assert_eq!(data1, data2);
    }

    #[test]
    fn test_checkpoint_created_data_clone() {
        let data = CheckpointCreatedData::new("cp-2", 2000, 1000, true, true);
        let cloned = data.clone();
        assert_eq!(data, cloned);
    }

    // Tests for SignalDeliveryMode
    #[test]
    fn test_signal_delivery_mode_display() {
        assert_eq!(SignalDeliveryMode::Immediate.to_string(), "Immediate");
        assert_eq!(SignalDeliveryMode::Queued.to_string(), "Queued");
    }

    #[test]
    fn test_signal_delivery_mode_equality() {
        assert_eq!(SignalDeliveryMode::Immediate, SignalDeliveryMode::Immediate);
        assert_ne!(SignalDeliveryMode::Immediate, SignalDeliveryMode::Queued);
    }

    // Tests for SignalDispatchedData
    #[test]
    fn test_signal_dispatched_data_creation() {
        let data = SignalDispatchedData::new(
            "interrupt",
            "agent-001",
            256,
            SignalDeliveryMode::Immediate,
        );
        assert_eq!(data.signal_type, "interrupt");
        assert_eq!(data.target_agent, "agent-001");
        assert_eq!(data.payload_size, 256);
        assert_eq!(data.delivery_mode, SignalDeliveryMode::Immediate);
    }

    #[test]
    fn test_signal_dispatched_data_queued() {
        let data = SignalDispatchedData::new("cancel", "agent-002", 128, SignalDeliveryMode::Queued);
        assert_eq!(data.delivery_mode, SignalDeliveryMode::Queued);
    }

    #[test]
    fn test_signal_dispatched_data_equality() {
        let data1 = SignalDispatchedData::new("type", "agent", 100, SignalDeliveryMode::Immediate);
        let data2 = SignalDispatchedData::new("type", "agent", 100, SignalDeliveryMode::Immediate);
        assert_eq!(data1, data2);
    }

    #[test]
    fn test_signal_dispatched_data_clone() {
        let data = SignalDispatchedData::new("checkpoint", "agent", 512, SignalDeliveryMode::Queued);
        let cloned = data.clone();
        assert_eq!(data, cloned);
    }

    // Tests for ExceptionSeverity
    #[test]
    fn test_exception_severity_display() {
        assert_eq!(ExceptionSeverity::Low.to_string(), "Low");
        assert_eq!(ExceptionSeverity::Medium.to_string(), "Medium");
        assert_eq!(ExceptionSeverity::High.to_string(), "High");
        assert_eq!(ExceptionSeverity::Critical.to_string(), "Critical");
    }

    #[test]
    fn test_exception_severity_equality() {
        assert_eq!(ExceptionSeverity::High, ExceptionSeverity::High);
        assert_ne!(ExceptionSeverity::High, ExceptionSeverity::Low);
    }

    // Tests for ExceptionRaisedData
    #[test]
    fn test_exception_raised_data_creation() {
        let data = ExceptionRaisedData::new(
            "TimeoutError",
            ExceptionSeverity::High,
            "Tool execution exceeded timeout",
            false,
        );
        assert_eq!(data.exception_type, "TimeoutError");
        assert_eq!(data.severity, ExceptionSeverity::High);
        assert_eq!(data.error_message, "Tool execution exceeded timeout");
        assert!(!data.recovery_attempted);
        assert_eq!(data.recovery_outcome, None);
    }

    #[test]
    fn test_exception_raised_data_with_recovery() {
        let data =
            ExceptionRaisedData::new("RuntimeError", ExceptionSeverity::Medium, "Error occurred", true)
                .with_recovery_outcome("Recovered successfully");
        assert!(data.recovery_attempted);
        assert_eq!(data.recovery_outcome, Some("Recovered successfully".to_string()));
    }

    #[test]
    fn test_exception_raised_data_equality() {
        let data1 =
            ExceptionRaisedData::new("Error", ExceptionSeverity::Low, "message", false);
        let data2 =
            ExceptionRaisedData::new("Error", ExceptionSeverity::Low, "message", false);
        assert_eq!(data1, data2);
    }

    #[test]
    fn test_exception_raised_data_clone() {
        let data = ExceptionRaisedData::new("Critical", ExceptionSeverity::Critical, "msg", true);
        let cloned = data.clone();
        assert_eq!(data, cloned);
    }

    #[test]
    fn test_exception_raised_data_critical() {
        let data = ExceptionRaisedData::new(
            "SystemFailure",
            ExceptionSeverity::Critical,
            "System is failing",
            true,
        );
        assert_eq!(data.severity, ExceptionSeverity::Critical);
    }
}
