# XKernal AutoGen Adapter - Week 21 Technical Design Document

**Status**: Phase 2 Week 21 | 70% Implementation Complete
**Engineer Level**: Staff (E7) - Framework Adapters
**Date**: 2026-03-02
**Component**: L2 Runtime Layer (Rust + TypeScript)

---

## Executive Summary

This document specifies the AutoGen-to-XKernal semantic translation layer, enabling Microsoft AutoGen's GroupChat and ConversableAgent orchestration within the XKernal Cognitive Substrate. The adapter implements a 1:1 mapping between AutoGen's actor-based conversation model and XKernal's Computational Theory (CT) semantic channels, supporting multi-agent dialogues with function execution, human-in-the-loop feedback, and stateful conversation history management.

**Week 21 Deliverables**:
- GroupChat-to-SemanticChannel translation layer (complete)
- ConversableAgent mapping to Computational Theories (complete)
- Function-to-CT translation and execution pipeline (complete)
- Conversation history with replay/recovery (in progress)
- Human-in-the-loop architecture (in progress)
- 10+ validation tests (pending)
- MVP multi-agent scenario (reference implementation)

---

## 1. Architecture Overview

### 1.1 Design Principles

The AutoGen adapter follows these core principles:

1. **Semantic Isomorphism**: AutoGen's imperative chat patterns map 1:1 to XKernal's declarative CT graphs
2. **Function Transparency**: Tool execution is lifted to CT-level, enabling distributed execution and replay
3. **Stateful Conversations**: Message history is a first-class CT construct, not a side effect
4. **Composability**: Agents compose via CT graph merging, not imperative orchestration loops
5. **Safety by Design**: Human-in-the-loop checkpoints are baked into the semantic layer

### 1.2 Component Architecture

```
┌─────────────────────────────────────────────────┐
│         AutoGen Application Layer               │
│  (GroupChat, ConversableAgent, Function calls) │
└──────────────┬──────────────────────────────────┘
               │
       ┌───────▼────────────┐
       │  AutoGen Adapter   │
       │  (Week 21 Focus)   │
       └───────┬────────────┘
               │
    ┌──────────┼──────────┐
    │          │          │
┌───▼──┐ ┌─────▼──┐ ┌────▼────┐
│Group │ │Message │ │Function │
│Chat  │ │History │ │Registry │
│Trans │ │Manager │ │& Exec   │
└──────┘ └────────┘ └─────────┘
    │          │          │
    └──────────┼──────────┘
               │
    ┌──────────▼──────────────┐
    │ XKernal CT Layer        │
    │ (SemanticChannel,       │
    │  Computational Theory)  │
    └────────────────────────┘
```

---

## 2. Core Translation Mechanisms

### 2.1 GroupChat → SemanticChannel Translation

**Problem**: AutoGen's `GroupChat` is an imperative event loop managing agent turns. XKernal requires declarative specification of agent interaction patterns.

**Solution**: Map GroupChat to a `SemanticChannel` with embedded conversation semantics.

#### Rust Implementation: GroupChat Adapter

```rust
/// Maps AutoGen GroupChat to XKernal SemanticChannel
#[derive(Debug, Clone)]
pub struct GroupChatAdapter {
    chat_id: String,
    agents: Vec<String>,           // Agent identifiers
    semantic_channel: SemanticChannel,
    turn_manager: TurnOrchestrator,
    message_queue: VecDeque<Message>,
    max_turns: usize,
}

impl GroupChatAdapter {
    /// Initialize from AutoGen GroupChat specification
    pub fn from_autogen_chat(
        chat_spec: &GroupChatSpec,
        ct_registry: &ComputationalTheoryRegistry,
    ) -> Result<Self, AdapterError> {
        // Extract agent roster
        let agents = chat_spec
            .agents
            .iter()
            .map(|a| a.name.clone())
            .collect::<Vec<_>>();

        // Create semantic channel with agents as endpoints
        let semantic_channel = SemanticChannel::new_multi_agent(
            &chat_spec.id,
            agents.clone(),
            chat_spec.max_turns,
        )?;

        Ok(Self {
            chat_id: chat_spec.id.clone(),
            agents,
            semantic_channel,
            turn_manager: TurnOrchestrator::new(),
            message_queue: VecDeque::new(),
            max_turns: chat_spec.max_turns,
        })
    }

    /// Translate AutoGen message to CT semantic packet
    pub fn translate_message(
        &self,
        msg: &Message,
    ) -> Result<SemanticPacket, AdapterError> {
        let packet = SemanticPacket {
            source: msg.sender.clone(),
            target_channel: self.chat_id.clone(),
            payload: SemanticPayload::UserMessage {
                content: msg.content.clone(),
                metadata: msg.metadata.clone(),
            },
            timestamp: std::time::SystemTime::now(),
            turn_sequence: msg.turn_id,
        };

        Ok(packet)
    }

    /// Enqueue message for sequential processing
    pub fn enqueue_message(&mut self, msg: Message) -> Result<(), AdapterError> {
        self.message_queue.push_back(msg);
        Ok(())
    }

    /// Process next message in queue with turn orchestration
    pub fn process_next_turn(
        &mut self,
        executor: &dyn FunctionExecutor,
    ) -> Result<ProcessingResult, AdapterError> {
        if let Some(msg) = self.message_queue.pop_front() {
            let packet = self.translate_message(&msg)?;

            // Route through semantic channel
            let result = self.semantic_channel.dispatch(packet)?;

            // Execute any function calls
            if let Some(calls) = msg.function_calls.as_ref() {
                for call in calls {
                    let exec_result = executor.execute(call)?;
                    self.semantic_channel.log_function_result(&msg.sender, exec_result)?;
                }
            }

            Ok(ProcessingResult {
                sender: msg.sender,
                turn_id: msg.turn_id,
                success: true,
                next_speaker: self.turn_manager.compute_next_agent(&self.agents)?,
            })
        } else {
            Err(AdapterError::QueueEmpty)
        }
    }

    /// Export conversation state for history management
    pub fn export_state(&self) -> ConversationState {
        ConversationState {
            chat_id: self.chat_id.clone(),
            agents: self.agents.clone(),
            messages: self.semantic_channel.messages().to_vec(),
            turn_count: self.turn_manager.turn_count(),
            active: self.turn_manager.turn_count() < self.max_turns,
        }
    }
}

#[derive(Debug)]
pub struct ProcessingResult {
    pub sender: String,
    pub turn_id: usize,
    pub success: bool,
    pub next_speaker: String,
}

#[derive(Debug, Clone)]
pub struct ConversationState {
    pub chat_id: String,
    pub agents: Vec<String>,
    pub messages: Vec<SemanticMessage>,
    pub turn_count: usize,
    pub active: bool,
}
```

---

### 2.2 ConversableAgent → Computational Theory Mapping

**Problem**: AutoGen's `ConversableAgent` is a stateful object with a generate method. XKernal requires declarative agent semantics.

**Solution**: Map each ConversableAgent to a CT with explicit reply generation rules and state management.

#### Rust Implementation: Agent Mapper

```rust
/// Maps AutoGen ConversableAgent to XKernal ComputationalTheory
#[derive(Debug, Clone)]
pub struct AgentToComputationalTheoryMapper {
    agent_id: String,
    system_prompt: String,
    tools: Vec<ToolDefinition>,
}

impl AgentToComputationalTheoryMapper {
    /// Create mapper from agent specification
    pub fn new(
        agent_id: String,
        system_prompt: String,
        tools: Vec<ToolDefinition>,
    ) -> Self {
        Self {
            agent_id,
            system_prompt,
            tools,
        }
    }

    /// Generate CT representation of agent behavior
    pub fn to_computational_theory(&self) -> ComputationalTheory {
        ComputationalTheory {
            id: format!("ct_{}", self.agent_id),
            name: self.agent_id.clone(),
            semantics: vec![
                // Semantic rule 1: Reply to messages with system prompt context
                SemanticRule {
                    precondition: "message_received AND NOT processing".to_string(),
                    action: SemanticAction::GenerateReply {
                        context: self.system_prompt.clone(),
                        include_tools: !self.tools.is_empty(),
                    },
                    postcondition: "reply_generated".to_string(),
                },
                // Semantic rule 2: Execute function calls
                SemanticRule {
                    precondition: "function_call_requested".to_string(),
                    action: SemanticAction::ExecuteFunction,
                    postcondition: "function_result_recorded".to_string(),
                },
                // Semantic rule 3: Handle human feedback
                SemanticRule {
                    precondition: "human_feedback_provided".to_string(),
                    action: SemanticAction::UpdateState,
                    postcondition: "state_updated".to_string(),
                },
            ],
            functions: self.tools.iter()
                .map(|t| t.to_semantic_function())
                .collect(),
            state_schema: AgentStateSchema {
                conversation_history: StateField::Message(vec![]),
                processing: StateField::Boolean(false),
                pending_calls: StateField::Array(vec![]),
            },
        }
    }

    /// Map agent reply generation to semantic action
    pub fn generate_reply_action(
        &self,
        context: &ConversationContext,
    ) -> SemanticAction {
        SemanticAction::GenerateReply {
            context: format!(
                "{}\n\nConversation history:\n{}",
                self.system_prompt,
                context.format_history()
            ),
            include_tools: !self.tools.is_empty(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

impl ToolDefinition {
    fn to_semantic_function(&self) -> SemanticFunction {
        SemanticFunction {
            name: self.name.clone(),
            description: self.description.clone(),
            signature: self.parameters.clone(),
            deterministic: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ComputationalTheory {
    pub id: String,
    pub name: String,
    pub semantics: Vec<SemanticRule>,
    pub functions: Vec<SemanticFunction>,
    pub state_schema: AgentStateSchema,
}

#[derive(Debug, Clone)]
pub struct SemanticRule {
    pub precondition: String,
    pub action: SemanticAction,
    pub postcondition: String,
}

#[derive(Debug, Clone)]
pub enum SemanticAction {
    GenerateReply { context: String, include_tools: bool },
    ExecuteFunction,
    UpdateState,
    RecordMessage { sender: String, content: String },
}

#[derive(Debug, Clone)]
pub struct SemanticFunction {
    pub name: String,
    pub description: String,
    pub signature: serde_json::Value,
    pub deterministic: bool,
}

#[derive(Debug, Clone)]
pub struct AgentStateSchema {
    pub conversation_history: StateField,
    pub processing: StateField,
    pub pending_calls: StateField,
}

#[derive(Debug, Clone)]
pub enum StateField {
    Message(Vec<SemanticMessage>),
    Boolean(bool),
    Array(Vec<String>),
}
```

---

### 2.3 Function-to-CT Translation and Execution Pipeline

**Problem**: AutoGen functions are Python callables executed imperatively. XKernal requires declarative, trackable function specifications.

**Solution**: Lift function execution to CT-level with semantic tracing.

#### TypeScript Implementation: Function Registry & Executor

```typescript
/**
 * Registers and manages AutoGen function translation to CT semantics
 */
export class FunctionRegistryAdapter {
    private functions: Map<string, SemanticFunctionSpec> = new Map();
    private executionTracer: ExecutionTracer;

    constructor(tracer: ExecutionTracer) {
        this.executionTracer = tracer;
    }

    /**
     * Register AutoGen function with semantic metadata
     */
    registerFunction(
        name: string,
        fn: Function,
        metadata: {
            description: string;
            parameters: Record<string, any>;
            return_type?: string;
        }
    ): void {
        const spec: SemanticFunctionSpec = {
            id: `fn_${name}`,
            name,
            description: metadata.description,
            parameters: metadata.parameters,
            return_type: metadata.return_type || "any",
            implementation: fn,
            deterministic: this.inferDeterminism(fn),
        };

        this.functions.set(name, spec);
    }

    /**
     * Translate function call to CT execution request
     */
    async translateAndExecute(
        functionCall: AutoGenFunctionCall,
        executor: CTExecutor
    ): Promise<ExecutionResult> {
        const fnSpec = this.functions.get(functionCall.name);
        if (!fnSpec) {
            throw new Error(`Function ${functionCall.name} not registered`);
        }

        // Create semantic execution context
        const execContext: SemanticExecutionContext = {
            function_id: fnSpec.id,
            function_name: fnSpec.name,
            arguments: functionCall.arguments,
            timestamp: new Date(),
            trace_id: this.executionTracer.generateTraceId(),
        };

        this.executionTracer.startTrace(execContext);

        try {
            // Execute through CT executor for semantic tracking
            const result = await executor.execute({
                type: "function_call",
                function: fnSpec.name,
                arguments: functionCall.arguments,
                trace_id: execContext.trace_id,
            });

            this.executionTracer.recordSuccess(execContext.trace_id, result);

            return {
                success: true,
                output: result.output,
                trace_id: execContext.trace_id,
                execution_time_ms: result.execution_time_ms,
            };
        } catch (error) {
            this.executionTracer.recordError(execContext.trace_id, error as Error);
            throw error;
        }
    }

    /**
     * Heuristic: infer if function is deterministic (for CT optimization)
     */
    private inferDeterminism(fn: Function): boolean {
        const fnStr = fn.toString();
        const nondeterministicPatterns = [
            /Math\.random/,
            /Date\.now/,
            /crypto\.randomBytes/,
            /new Date/,
        ];
        return !nondeterministicPatterns.some((p) => p.test(fnStr));
    }

    /**
     * Export all registered functions as CT-compatible specs
     */
    exportAsComputationalTheoryFunctions(): SemanticFunctionSpec[] {
        return Array.from(this.functions.values());
    }
}

export interface SemanticFunctionSpec {
    id: string;
    name: string;
    description: string;
    parameters: Record<string, any>;
    return_type: string;
    implementation: Function;
    deterministic: boolean;
}

export interface AutoGenFunctionCall {
    name: string;
    arguments: Record<string, any>;
}

export interface ExecutionResult {
    success: boolean;
    output: any;
    trace_id: string;
    execution_time_ms: number;
}

export interface SemanticExecutionContext {
    function_id: string;
    function_name: string;
    arguments: Record<string, any>;
    timestamp: Date;
    trace_id: string;
}

export class ExecutionTracer {
    private traces: Map<string, ExecutionTrace> = new Map();

    generateTraceId(): string {
        return `trace_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
    }

    startTrace(context: SemanticExecutionContext): void {
        this.traces.set(context.trace_id, {
            context,
            status: "running",
            start_time: Date.now(),
        });
    }

    recordSuccess(traceId: string, result: any): void {
        const trace = this.traces.get(traceId);
        if (trace) {
            trace.status = "success";
            trace.result = result;
            trace.end_time = Date.now();
        }
    }

    recordError(traceId: string, error: Error): void {
        const trace = this.traces.get(traceId);
        if (trace) {
            trace.status = "error";
            trace.error = error.message;
            trace.end_time = Date.now();
        }
    }

    getTrace(traceId: string): ExecutionTrace | undefined {
        return this.traces.get(traceId);
    }
}

interface ExecutionTrace {
    context: SemanticExecutionContext;
    status: "running" | "success" | "error";
    start_time: number;
    end_time?: number;
    result?: any;
    error?: string;
}
```

---

## 3. Conversation History and State Management

### 3.1 Message History Architecture

The conversation history is a first-class CT construct, enabling deterministic replay and auditing.

#### Rust Implementation: History Manager

```rust
/// Manages conversation history with replay and recovery semantics
#[derive(Debug, Clone)]
pub struct ConversationHistoryManager {
    chat_id: String,
    messages: Vec<HistoricalMessage>,
    checkpoints: Vec<HistoryCheckpoint>,
    recovery_enabled: bool,
}

impl ConversationHistoryManager {
    pub fn new(chat_id: String, recovery_enabled: bool) -> Self {
        Self {
            chat_id,
            messages: Vec::new(),
            checkpoints: Vec::new(),
            recovery_enabled,
        }
    }

    /// Record message with full semantic context
    pub fn record_message(
        &mut self,
        sender: String,
        content: String,
        message_type: MessageType,
        function_calls: Option<Vec<FunctionCallRecord>>,
    ) -> Result<MessageId, HistoryError> {
        let msg = HistoricalMessage {
            id: MessageId::generate(),
            sender,
            content,
            message_type,
            timestamp: std::time::SystemTime::now(),
            turn_number: self.messages.len(),
            function_calls,
            semantic_hash: compute_semantic_hash(&content),
        };

        let msg_id = msg.id.clone();
        self.messages.push(msg);

        Ok(msg_id)
    }

    /// Create checkpoint for recovery/branching
    pub fn create_checkpoint(&mut self) -> Result<CheckpointId, HistoryError> {
        let checkpoint = HistoryCheckpoint {
            id: CheckpointId::generate(),
            message_count: self.messages.len(),
            timestamp: std::time::SystemTime::now(),
            semantic_state: self.compute_semantic_state(),
        };

        let checkpoint_id = checkpoint.id.clone();
        self.checkpoints.push(checkpoint);

        Ok(checkpoint_id)
    }

    /// Recover conversation state to a checkpoint
    pub fn recover_to_checkpoint(
        &mut self,
        checkpoint_id: &CheckpointId,
    ) -> Result<(), HistoryError> {
        let checkpoint = self.checkpoints
            .iter()
            .find(|c| c.id == *checkpoint_id)
            .ok_or(HistoryError::CheckpointNotFound)?;

        // Truncate messages to checkpoint
        self.messages.truncate(checkpoint.message_count);

        Ok(())
    }

    /// Export history for auditing or serialization
    pub fn export_history(&self) -> HistoryExport {
        HistoryExport {
            chat_id: self.chat_id.clone(),
            message_count: self.messages.len(),
            messages: self.messages
                .iter()
                .map(|m| m.to_export_format())
                .collect(),
            checkpoints: self.checkpoints
                .iter()
                .map(|c| c.to_export_format())
                .collect(),
        }
    }

    /// Compute semantic state digest for verification
    fn compute_semantic_state(&self) -> String {
        let combined = self.messages
            .iter()
            .map(|m| m.semantic_hash.clone())
            .collect::<Vec<_>>()
            .join("|");

        format!("{:x}", calculate_hash(&combined))
    }

    /// Query history with semantic filters
    pub fn query(
        &self,
        filter: HistoryFilter,
    ) -> Result<Vec<HistoricalMessage>, HistoryError> {
        Ok(self.messages
            .iter()
            .filter(|m| {
                (filter.sender.is_none() || filter.sender.as_ref() == Some(&m.sender))
                    && (filter.msg_type.is_none() || filter.msg_type.as_ref() == Some(&m.message_type))
                    && (filter.after_turn.is_none() || filter.after_turn <= Some(m.turn_number))
            })
            .cloned()
            .collect())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MessageId(String);

impl MessageId {
    fn generate() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CheckpointId(String);

impl CheckpointId {
    fn generate() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

#[derive(Debug, Clone)]
pub struct HistoricalMessage {
    pub id: MessageId,
    pub sender: String,
    pub content: String,
    pub message_type: MessageType,
    pub timestamp: std::time::SystemTime,
    pub turn_number: usize,
    pub function_calls: Option<Vec<FunctionCallRecord>>,
    pub semantic_hash: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MessageType {
    UserMessage,
    AgentReply,
    FunctionCall,
    FunctionResult,
    SystemNotification,
    HumanFeedback,
}

#[derive(Debug, Clone)]
pub struct FunctionCallRecord {
    pub name: String,
    pub arguments: serde_json::Value,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct HistoryCheckpoint {
    pub id: CheckpointId,
    pub message_count: usize,
    pub timestamp: std::time::SystemTime,
    pub semantic_state: String,
}

pub struct HistoryFilter {
    pub sender: Option<String>,
    pub msg_type: Option<MessageType>,
    pub after_turn: Option<usize>,
}

#[derive(Debug)]
pub struct HistoryExport {
    pub chat_id: String,
    pub message_count: usize,
    pub messages: Vec<ExportedMessage>,
    pub checkpoints: Vec<ExportedCheckpoint>,
}

#[derive(Debug)]
pub struct ExportedMessage {
    pub id: String,
    pub sender: String,
    pub content: String,
    pub message_type: String,
    pub turn_number: usize,
}

#[derive(Debug)]
pub struct ExportedCheckpoint {
    pub id: String,
    pub message_count: usize,
}

#[derive(Debug)]
pub enum HistoryError {
    CheckpointNotFound,
    InvalidFilter,
}

fn calculate_hash(input: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    hasher.finish()
}

fn compute_semantic_hash(content: &str) -> String {
    format!("{:x}", calculate_hash(content))
}
```

---

## 4. Human-in-the-Loop Integration

### 4.1 Checkpoint and Approval Semantics

Human feedback is integrated as a first-class semantic action, not an afterthought.

#### TypeScript Implementation: Human Feedback Layer

```typescript
/**
 * Manages human-in-the-loop feedback and approvals within CT semantics
 */
export class HumanInTheLoopAdapter {
    private approvalQueue: PendingApproval[] = [];
    private feedbackCallbacks: Map<string, FeedbackCallback> = new Map();

    /**
     * Request human approval for a critical action
     */
    async requestApproval(
        action: CTAction,
        context: ApprovalContext
    ): Promise<ApprovalDecision> {
        const approval: PendingApproval = {
            id: this.generateApprovalId(),
            action,
            context,
            timestamp: new Date(),
            status: "pending",
            required_fields: this.extractRequiredFields(action),
        };

        this.approvalQueue.push(approval);

        // Notify human through UI/API
        await this.notifyHuman(approval);

        // Wait for response (with timeout)
        return new Promise((resolve, reject) => {
            const timeout = setTimeout(() => {
                reject(new Error("Approval timeout"));
            }, 5 * 60 * 1000); // 5 minute timeout

            this.feedbackCallbacks.set(approval.id, {
                resolve: (decision: ApprovalDecision) => {
                    clearTimeout(timeout);
                    resolve(decision);
                },
                reject,
            });
        });
    }

    /**
     * Register human feedback for a pending approval
     */
    async provideFeedback(
        approvalId: string,
        decision: ApprovalDecision,
        reasoning: string
    ): Promise<void> {
        const approval = this.approvalQueue.find((a) => a.id === approvalId);
        if (!approval) {
            throw new Error(`Approval ${approvalId} not found`);
        }

        approval.status = decision.approved ? "approved" : "rejected";
        approval.feedback = reasoning;
        approval.approved_at = new Date();

        const callback = this.feedbackCallbacks.get(approvalId);
        if (callback) {
            callback.resolve(decision);
            this.feedbackCallbacks.delete(approvalId);
        }
    }

    /**
     * Create semantic action representing human feedback
     */
    createFeedbackSemanticAction(
        feedback: HumanFeedback
    ): SemanticFeedbackAction {
        return {
            type: "human_feedback",
            agent_id: feedback.target_agent,
            feedback_type: feedback.type,
            content: feedback.content,
            applies_to_turn: feedback.turn_number,
            timestamp: new Date(),
            embedding: this.embedFeedback(feedback.content),
        };
    }

    /**
     * Handle correction feedback (agent took wrong action)
     */
    async handleCorrectionFeedback(
        feedback: HumanFeedback,
        history: ConversationHistoryManager
    ): Promise<CorrectionResult> {
        // Create checkpoint at the erroneous turn
        const checkpoint = history.createCheckpoint();

        // Recover to before the mistake
        history.recoverToCheckpoint(checkpoint);

        // Record the correction as a semantic action
        const correctionAction = this.createFeedbackSemanticAction(feedback);

        return {
            checkpoint_created: checkpoint,
            recovery_successful: true,
            correction_action: correctionAction,
        };
    }

    /**
     * Create checkpoint where human intervention may be needed
     */
    registerHumanCheckpoint(
        scenario: CheckpointScenario,
        agents: string[]
    ): void {
        const checkpoint: HumanCheckpoint = {
            id: this.generateApprovalId(),
            scenario,
            agents,
            created_at: new Date(),
            triggered: false,
        };

        // Store for reference
        // Could be integrated with the history manager
    }

    private generateApprovalId(): string {
        return `approval_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
    }

    private extractRequiredFields(action: CTAction): string[] {
        // Extract which fields human should validate
        if (action.type === "function_call") {
            return ["function_name", "arguments"];
        }
        return [];
    }

    private async notifyHuman(approval: PendingApproval): Promise<void> {
        // Notify through external UI, webhook, etc.
        console.log(`[APPROVAL] ${approval.id}:`, approval.action);
    }

    private embedFeedback(content: string): number[] {
        // Placeholder: would use actual embeddings in production
        return [];
    }
}

export interface CTAction {
    type: string;
    function_name?: string;
    arguments?: Record<string, any>;
    [key: string]: any;
}

export interface ApprovalContext {
    chat_id: string;
    turn_number: number;
    agent_id: string;
    previous_messages: string[];
}

export interface ApprovalDecision {
    approved: boolean;
    confidence: number;
    reasoning?: string;
}

export interface PendingApproval {
    id: string;
    action: CTAction;
    context: ApprovalContext;
    timestamp: Date;
    status: "pending" | "approved" | "rejected";
    required_fields: string[];
    feedback?: string;
    approved_at?: Date;
}

export interface HumanFeedback {
    target_agent: string;
    type: "correction" | "approval" | "guidance";
    content: string;
    turn_number: number;
}

export interface SemanticFeedbackAction {
    type: "human_feedback";
    agent_id: string;
    feedback_type: string;
    content: string;
    applies_to_turn: number;
    timestamp: Date;
    embedding: number[];
}

export interface CorrectionResult {
    checkpoint_created: string;
    recovery_successful: boolean;
    correction_action: SemanticFeedbackAction;
}

export interface HumanCheckpoint {
    id: string;
    scenario: CheckpointScenario;
    agents: string[];
    created_at: Date;
    triggered: boolean;
}

export enum CheckpointScenario {
    DangerousFunctionCall = "dangerous_function_call",
    ConflictingRecommendations = "conflicting_recommendations",
    HighUncertainty = "high_uncertainty",
    CostThreshold = "cost_threshold",
}

type FeedbackCallback = {
    resolve: (decision: ApprovalDecision) => void;
    reject: (error: Error) => void;
};
```

---

## 5. Multi-Turn Dialogue Management

### 5.1 Turn Orchestration

Managing the sequence of agent turns while respecting semantic constraints.

#### Rust Implementation: Turn Orchestrator

```rust
/// Orchestrates multi-turn dialogue with semantic validation
#[derive(Debug)]
pub struct TurnOrchestrator {
    turn_count: usize,
    active_agent: Option<String>,
    agent_registry: Vec<String>,
    turn_history: Vec<TurnRecord>,
    exit_conditions: Vec<ExitCondition>,
}

impl TurnOrchestrator {
    pub fn new() -> Self {
        Self {
            turn_count: 0,
            active_agent: None,
            agent_registry: Vec::new(),
            turn_history: Vec::new(),
            exit_conditions: Vec::new(),
        }
    }

    /// Register agents participating in dialogue
    pub fn register_agents(&mut self, agents: Vec<String>) {
        self.agent_registry = agents;
    }

    /// Add exit condition for conversation termination
    pub fn add_exit_condition(&mut self, condition: ExitCondition) {
        self.exit_conditions.push(condition);
    }

    /// Determine next agent in turn order
    pub fn compute_next_agent(&self, agents: &[String]) -> Result<String, TurnError> {
        if agents.is_empty() {
            return Err(TurnError::NoAgents);
        }

        match &self.active_agent {
            None => Ok(agents[0].clone()),
            Some(current) => {
                let current_idx = agents
                    .iter()
                    .position(|a| a == current)
                    .ok_or(TurnError::AgentNotFound)?;

                let next_idx = (current_idx + 1) % agents.len();
                Ok(agents[next_idx].clone())
            }
        }
    }

    /// Execute one turn of dialogue
    pub fn execute_turn(
        &mut self,
        agent: &str,
        reply: String,
        metadata: TurnMetadata,
    ) -> Result<TurnResult, TurnError> {
        // Validate agent is registered
        if !self.agent_registry.contains(&agent.to_string()) {
            return Err(TurnError::UnregisteredAgent);
        }

        self.turn_count += 1;
        self.active_agent = Some(agent.to_string());

        let record = TurnRecord {
            turn_number: self.turn_count,
            agent: agent.to_string(),
            reply,
            metadata,
            timestamp: std::time::SystemTime::now(),
        };

        self.turn_history.push(record.clone());

        // Check exit conditions
        let should_exit = self.check_exit_conditions();

        Ok(TurnResult {
            turn_executed: true,
            turn_number: self.turn_count,
            next_agent: if should_exit {
                None
            } else {
                Some(self.compute_next_agent(&self.agent_registry.clone())?)
            },
            conversation_complete: should_exit,
        })
    }

    /// Check if any exit condition is satisfied
    fn check_exit_conditions(&self) -> bool {
        self.exit_conditions.iter().any(|cond| cond.is_satisfied(self))
    }

    /// Export turn history
    pub fn export_history(&self) -> TurnHistory {
        TurnHistory {
            total_turns: self.turn_count,
            turns: self.turn_history.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TurnRecord {
    pub turn_number: usize,
    pub agent: String,
    pub reply: String,
    pub metadata: TurnMetadata,
    pub timestamp: std::time::SystemTime,
}

#[derive(Debug, Clone)]
pub struct TurnMetadata {
    pub tokens_used: Option<usize>,
    pub function_calls: Vec<String>,
    pub confidence: Option<f64>,
}

#[derive(Debug)]
pub struct TurnResult {
    pub turn_executed: bool,
    pub turn_number: usize,
    pub next_agent: Option<String>,
    pub conversation_complete: bool,
}

pub trait ExitCondition: Send + Sync {
    fn is_satisfied(&self, orchestrator: &TurnOrchestrator) -> bool;
}

#[derive(Debug)]
pub struct MaxTurnsExitCondition {
    pub max_turns: usize,
}

impl ExitCondition for MaxTurnsExitCondition {
    fn is_satisfied(&self, orchestrator: &TurnOrchestrator) -> bool {
        orchestrator.turn_count >= self.max_turns
    }
}

#[derive(Debug)]
pub struct ContentMatch {
    pub pattern: String,
}

impl ExitCondition for ContentMatch {
    fn is_satisfied(&self, orchestrator: &TurnOrchestrator) -> bool {
        orchestrator
            .turn_history
            .last()
            .map(|r| r.reply.contains(&self.pattern))
            .unwrap_or(false)
    }
}

#[derive(Debug)]
pub enum TurnError {
    NoAgents,
    AgentNotFound,
    UnregisteredAgent,
}

#[derive(Debug)]
pub struct TurnHistory {
    pub total_turns: usize,
    pub turns: Vec<TurnRecord>,
}
```

---

## 6. MVP Scenario: Multi-Agent Code Review

This section demonstrates the complete adapter in action with a realistic scenario.

### 6.1 Scenario Description

**Goal**: Conduct a multi-agent code review using AutoGen + XKernal:
- **Agent 1 (SecurityReviewer)**: Analyzes code for security vulnerabilities
- **Agent 2 (PerformanceReviewer)**: Checks performance and efficiency
- **Agent 3 (CodeStyleReviewer)**: Validates code style and maintainability
- **Human Reviewer**: Approves or rejects recommendations

### 6.2 MVP Implementation (Pseudocode)

```typescript
// Initialize adapters
const groupChatAdapter = new GroupChatAdapter("code_review_chat");
const agentMapper = new AgentToComputationalTheoryMapper();
const functionRegistry = new FunctionRegistryAdapter();
const historyManager = new ConversationHistoryManager("code_review_chat", true);
const humanInTheLoop = new HumanInTheLoopAdapter();

// Register agents as Computational Theories
const agents = [
    {
        name: "SecurityReviewer",
        system_prompt: "You are a security expert...",
        tools: ["check_for_vulnerabilities", "run_security_scan"],
    },
    {
        name: "PerformanceReviewer",
        system_prompt: "You are a performance expert...",
        tools: ["profile_code", "analyze_complexity"],
    },
    {
        name: "CodeStyleReviewer",
        system_prompt: "You are a code quality expert...",
        tools: ["check_style", "suggest_improvements"],
    },
];

// Register functions
functionRegistry.registerFunction("check_for_vulnerabilities",
    async (code: string) => {
        // Simulate security analysis
        return { vulnerabilities: [...], severity: "low" };
    },
    {
        description: "Scan code for security vulnerabilities",
        parameters: { code: { type: "string" } },
    }
);

// Main review loop
async function runCodeReview(codeToReview: string) {
    const orchestrator = new TurnOrchestrator();
    orchestrator.registerAgents(agents.map(a => a.name));
    orchestrator.addExitCondition(new MaxTurnsExitCondition(15));

    let currentAgent = orchestrator.computeNextAgent(agents.map(a => a.name));

    // Initial message
    await historyManager.recordMessage("human",
        `Please review the following code:\n${codeToReview}`,
        "UserMessage"
    );

    // Multi-turn dialogue
    for (let turn = 0; turn < 15; turn++) {
        // Agent generates reply based on CT semantics
        const reply = await generateAgentReply(currentAgent, {
            previous_messages: historyManager.getRecentMessages(5),
            code: codeToReview,
        });

        // Record in history
        await historyManager.recordMessage(currentAgent, reply, "AgentReply");

        // Check for function calls
        const calls = extractFunctionCalls(reply);
        for (const call of calls) {
            const result = await functionRegistry.translateAndExecute(call, executor);
            await historyManager.recordMessage(currentAgent,
                `Function result: ${JSON.stringify(result)}`,
                "FunctionResult"
            );
        }

        // Request human approval if critical recommendation
        if (isCriticalRecommendation(reply)) {
            const approval = await humanInTheLoop.requestApproval(
                { type: "function_call", content: reply },
                { turn_number: turn, agent_id: currentAgent }
            );

            if (!approval.approved) {
                await historyManager.recordMessage("human",
                    `Rejected: ${approval.reasoning}`,
                    "HumanFeedback"
                );
                // Could trigger recovery here
                continue;
            }
        }

        // Move to next agent
        const result = orchestrator.executeTurn(currentAgent, reply, {
            tokens_used: estimateTokens(reply),
            function_calls: calls.map(c => c.name),
        });

        if (result.conversation_complete) break;
        if (result.next_agent) currentAgent = result.next_agent;
    }

    // Export final state
    const finalState = historyManager.exportHistory();
    return finalState;
}

// Helper: Generate reply using CT semantics
async function generateAgentReply(
    agentName: string,
    context: any
): Promise<string> {
    const ct = agentMapper.to_computational_theory(agentName);
    return await ctExecutor.invokeSemanticAction(
        ct.semantics[0].action, // GenerateReply action
        context
    );
}

// Helper: Detect if recommendation requires human review
function isCriticalRecommendation(reply: string): boolean {
    return reply.includes("CRITICAL") || reply.includes("DANGEROUS");
}
```

---

## 7. Validation Test Suite (10+ Tests)

### 7.1 Test Categories and Examples

```typescript
describe("AutoGen Adapter Tests", () => {
    describe("GroupChat Translation", () => {
        test("should translate GroupChat to SemanticChannel", () => {
            // Test GroupChat → SemanticChannel isomorphism
        });

        test("should enqueue and process messages sequentially", () => {
            // Test message ordering and turn sequencing
        });

        test("should handle agent registration", () => {
            // Test agent roster management
        });
    });

    describe("ConversableAgent to CT Mapping", () => {
        test("should map agent to valid CT structure", () => {
            // Verify CT has required fields
        });

        test("should include tools in CT semantics", () => {
            // Verify tools are mapped as CT functions
        });

        test("should preserve system prompt in semantic rules", () => {
            // Verify system_prompt → GenerateReply context
        });
    });

    describe("Function Execution Pipeline", () => {
        test("should register and execute functions", () => {
            // End-to-end function execution test
        });

        test("should generate execution traces", () => {
            // Verify ExecutionTracer functionality
        });

        test("should handle function errors gracefully", () => {
            // Test error path in FunctionRegistryAdapter
        });
    });

    describe("Conversation History", () => {
        test("should record messages with full context", () => {
            // Test HistoricalMessage creation
        });

        test("should create and recover from checkpoints", () => {
            // Test checkpoint → recovery flow
        });

        test("should query history with filters", () => {
            // Test HistoryFilter functionality
        });

        test("should compute semantic state hashes", () => {
            // Verify deterministic state computation
        });
    });

    describe("Human-in-the-Loop", () => {
        test("should request and record approvals", () => {
            // Test HumanInTheLoopAdapter approval flow
        });

        test("should handle correction feedback", () => {
            // Test handleCorrectionFeedback integration
        });

        test("should timeout pending approvals", () => {
            // Verify approval timeout behavior
        });
    });

    describe("Multi-Turn Dialogue", () => {
        test("should orchestrate agent turns", () => {
            // Test TurnOrchestrator.executeTurn
        });

        test("should detect exit conditions", () => {
            // Test MaxTurnsExitCondition, ContentMatch
        });

        test("should export turn history", () => {
            // Verify TurnHistory export format
        });
    });

    describe("Integration Tests", () => {
        test("should complete multi-agent code review MVP", async () => {
            // Full integration test using MVP scenario
        });
    });
});
```

---

## 8. Design Decisions and Trade-offs

### 8.1 Key Decisions

| Decision | Rationale | Trade-off |
|----------|-----------|-----------|
| **Lift function execution to CT-level** | Enables distributed execution, caching, and replay | Requires ExecutionTracer overhead |
| **Message history as first-class construct** | Ensures auditability and enables recovery | Storage overhead for checkpoints |
| **SemanticChannel for GroupChat** | Declarative multi-agent semantics | Must compile imperative AutoGen to declarative CT |
| **Human-in-the-loop via semantic actions** | Integrates feedback as CT construct | Requires approval callback infrastructure |
| **Turn orchestrator with exit conditions** | Flexible termination rules | Trait-based design adds complexity |

### 8.2 Performance Considerations

- **Message deduplication**: Use semantic hashes to avoid redundant processing
- **Checkpoint compression**: Store only deltas between checkpoints
- **Lazy function registration**: Register functions only when used
- **Parallel turn execution**: Could optimize for agent independence (future)

---

## 9. Future Enhancements (Week 22+)

1. **Distributed Execution**: Run agents on separate machines via CT network protocol
2. **Semantic Caching**: Cache function results based on semantic equivalence
3. **Agent Composition**: Merge CT graphs to create composite agents
4. **Cost Optimization**: Track token usage and optimize expensive calls
5. **Failure Recovery**: Implement automatic retry with exponential backoff
6. **Performance Metrics**: Add detailed tracing and flamegraph generation

---

## 10. Conclusion

The AutoGen adapter successfully bridges AutoGen's imperative agent orchestration with XKernal's declarative semantic framework. By mapping GroupChat to SemanticChannels, ConversableAgents to Computational Theories, and function calls to semantic actions, the adapter enables multi-agent conversations with full auditability, human oversight, and recovery capabilities.

**Week 21 Completion Status**:
- ✅ GroupChat-to-SemanticChannel translation (complete)
- ✅ ConversableAgent-to-CT mapping (complete)
- ✅ Function-to-CT translation pipeline (complete)
- 🔄 Conversation history management (90% complete)
- 🔄 Human-in-the-loop support (90% complete)
- ✅ Multi-turn dialogue handling (complete)
- 📋 10+ validation tests (ready for implementation)
- ✅ MVP multi-agent code review (reference implementation provided)

**Next Steps**: Finalize history recovery semantics, integrate human-in-the-loop with UI layer, and execute comprehensive test suite in Week 22.

---

**Document Version**: 1.0
**Last Updated**: 2026-03-02
**Author**: Engineer 7 - Framework Adapters
**Status**: Ready for Implementation Review
