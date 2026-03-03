/**
 * Cognitive Substrate SDK - Type Definitions
 * 
 * Shared type definitions for all CSCI v0.1 syscalls.
 * These types are used across all 8 syscall families.
 * 
 * Part of @cognitive-substrate/sdk
 */

// ============================================================================
// Branded Type Constructors
// ============================================================================

/**
 * Cognitive Task ID - globally unique identifier for a task
 * 
 * Returned by ct_spawn syscall and used to reference tasks in other syscalls.
 */
export type CognitiveTaskId = string & { readonly __brand: 'CognitiveTaskId' };

export function createCognitiveTaskId(value: string): CognitiveTaskId {
  return value as CognitiveTaskId;
}

/**
 * Memory Region ID - identifier for allocated memory
 * 
 * Returned by mem_alloc syscall and used to reference memory regions.
 */
export type MemoryRegionId = string & { readonly __brand: 'MemoryRegionId' };

export function createMemoryRegionId(value: string): MemoryRegionId {
  return value as MemoryRegionId;
}

/**
 * Channel ID - identifier for IPC channel
 * 
 * Returned by ch_create syscall and used for ch_send and ch_receive.
 */
export type ChannelId = string & { readonly __brand: 'ChannelId' };

export function createChannelId(value: string): ChannelId {
  return value as ChannelId;
}

/**
 * Grant Handle - identifier for capability grant
 * 
 * Returned by cap_grant syscall and used for cap_revoke.
 */
export type GrantHandle = string & { readonly __brand: 'GrantHandle' };

export function createGrantHandle(value: string): GrantHandle {
  return value as GrantHandle;
}

/**
 * Agent ID - identifier for an agent
 * 
 * Agents are persistent entities that create and manage cognitive tasks.
 */
export type AgentId = string & { readonly __brand: 'AgentId' };

export function createAgentId(value: string): AgentId {
  return value as AgentId;
}

/**
 * Checkpoint ID - identifier for task checkpoint
 * 
 * Returned by ct_checkpoint syscall and used by ct_resume.
 */
export type CheckpointId = string & { readonly __brand: 'CheckpointId' };

export function createCheckpointId(value: string): CheckpointId {
  return value as CheckpointId;
}

/**
 * Crew ID - identifier for agent crew
 * 
 * Returned by crew_init syscall and used for crew operations.
 */
export type CrewId = string & { readonly __brand: 'CrewId' };

export function createCrewId(value: string): CrewId {
  return value as CrewId;
}

/**
 * Signal Handler ID - identifier for registered signal handler
 * 
 * Returned by sig_handler_install syscall.
 */
export type SignalHandlerId = string & { readonly __brand: 'SignalHandlerId' };

export function createSignalHandlerId(value: string): SignalHandlerId {
  return value as SignalHandlerId;
}

// ============================================================================
// Configuration Types
// ============================================================================

/**
 * Task spawn configuration
 */
export interface TaskSpawnConfig {
  /** Parent agent creating this task */
  parent_agent: AgentId;
  /** Task configuration */
  config: TaskConfig;
  /** Capability set (array of capability names) */
  capabilities: string[];
  /** Resource budget constraints */
  budget: ResourceBudget;
}

/**
 * Task configuration for task creation
 */
export interface TaskConfig {
  /** Task name */
  name: string;
  /** Execution timeout in milliseconds (optional) */
  timeout_ms?: number;
  /** Task priority (0-255, higher = more important) (optional) */
  priority?: number;
}

/**
 * Resource budget constraints
 */
export interface ResourceBudget {
  /** Memory quota in bytes (optional) */
  memory_bytes?: number;
  /** CPU quota in milliseconds (optional) */
  cpu_ms?: number;
  /** Maximum child tasks (optional) */
  max_children?: number;
}

/**
 * Memory allocation configuration
 */
export interface MemoryAllocConfig {
  /** Size in bytes to allocate */
  size: number;
  /** Alignment requirement in bytes (optional) */
  alignment?: number;
  /** Allocation flags (optional) */
  flags?: number;
}

/**
 * Checkpoint creation configuration
 */
export interface CheckpointConfig {
  /** Checkpoint type (Full or Incremental) */
  type: CheckpointType;
  /** Checkpoint label for identification */
  label: string;
}

/**
 * Capability set for delegation
 */
export interface CapabilitySet {
  /** Capability names */
  capabilities: string[];
}

/**
 * Capability grant configuration
 */
export interface CapabilityGrantConfig {
  /** Recipient agent ID */
  recipient_id: AgentId;
  /** Capability set to grant */
  capability_set: CapabilitySet;
  /** Grant duration in milliseconds */
  duration_ms: number;
}

/**
 * Capability delegation configuration
 */
export interface CapabilityDelegateConfig {
  /** Recipient agent ID */
  recipient_id: AgentId;
  /** Capability set to delegate */
  capability_set: CapabilitySet;
}

/**
 * Tool invocation configuration
 */
export interface ToolInvokeConfig {
  /** Tool name */
  tool_name: string;
  /** Tool arguments (optional) */
  args?: Record<string, unknown>;
  /** Sandbox configuration (optional) */
  sandbox_config?: SandboxConfig;
}

/**
 * Tool binding configuration
 */
export interface ToolBindConfig {
  /** Tool name */
  tool_name: string;
  /** Namespace path for binding */
  namespace_path: string;
  /** Capabilities required by tool */
  capabilities: string[];
}

/**
 * Channel creation configuration
 */
export interface ChannelConfig {
  /** Maximum message size in bytes (optional) */
  max_message_size?: number;
  /** Channel buffer size (optional) */
  buffer_size?: number;
  /** Channel protocol type (optional) */
  protocol?: ChannelProtocol;
}

/**
 * Crew creation configuration
 */
export interface CrewConfig {
  /** Crew name */
  name: string;
  /** Crew configuration */
  config: {
    /** Mission description */
    mission: string;
    /** Coordinator agent ID */
    coordinator_agent: AgentId;
    /** Initial crew members (optional) */
    initial_members?: AgentId[];
    /** Collective budget (optional) */
    collective_budget?: number;
  };
}

/**
 * Signal handler registration configuration
 */
export interface SignalHandlerConfig {
  /** Signal number to handle */
  signal_number: number;
  /** Handler function */
  handler_fn: (signal: number, data: unknown) => Promise<void>;
  /** Registration flags (optional) */
  flags?: number;
}

/**
 * Telemetry trace event data
 */
export interface TraceEventData {
  /** Event name */
  event_name: string;
  /** Event data */
  data: Record<string, unknown>;
}

/**
 * Telemetry snapshot configuration
 */
export interface SnapshotConfig {
  /** Include task metrics (optional) */
  include_tasks?: boolean;
  /** Include memory metrics (optional) */
  include_memory?: boolean;
  /** Include channel metrics (optional) */
  include_channels?: boolean;
}

/**
 * Sandbox configuration for tool execution
 */
export interface SandboxConfig {
  /** Memory limit in bytes (optional) */
  memory_limit?: number;
  /** Execution timeout in milliseconds (optional) */
  timeout_ms?: number;
  /** Allow network access (optional) */
  allow_network?: boolean;
  /** Allowed file system paths (optional) */
  allowed_paths?: string[];
}

// ============================================================================
// Enumeration Types
// ============================================================================

/**
 * Checkpoint type for ct_checkpoint syscall
 */
export enum CheckpointType {
  /** Full checkpoint of complete task state */
  Full = 'Full',
  /** Incremental checkpoint (changes since last checkpoint) */
  Incremental = 'Incremental',
}

/**
 * Yield hint for ct_yield syscall
 */
export enum YieldHint {
  /** Task is thinking/processing */
  Thinking = 'Thinking',
  /** Task is waiting for input/response */
  WaitingForInput = 'WaitingForInput',
  /** Task is resource-limited */
  ResourceLimited = 'ResourceLimited',
}

/**
 * Channel protocol type for ch_create syscall
 */
export enum ChannelProtocol {
  /** Unstructured byte stream */
  ByteStream = 'ByteStream',
  /** Structured message-based protocol */
  MessageBased = 'MessageBased',
}

/**
 * Channel send flags for ch_send syscall
 */
export enum SendFlags {
  /** Default send behavior */
  Default = 0,
  /** Don't wait for receiver (fire and forget) */
  DontWait = 1,
}

// ============================================================================
// Result Types
// ============================================================================

/**
 * Tool invocation result
 */
export interface ToolResult {
  /** Success indicator */
  success: boolean;
  /** Result data (if successful) */
  data?: unknown;
  /** Error message (if failed) */
  error?: string;
}

/**
 * Snapshot data from telemetry_snapshot syscall
 */
export interface SnapshotData {
  /** Timestamp of snapshot */
  timestamp: number;
  /** Task metrics (if requested) */
  task_metrics?: Record<string, unknown>;
  /** Memory metrics (if requested) */
  memory_metrics?: Record<string, unknown>;
  /** Channel metrics (if requested) */
  channel_metrics?: Record<string, unknown>;
}

// ============================================================================
// Message Types
// ============================================================================

/**
 * Message payload for channels
 */
export interface MessagePayload {
  /** Message type identifier */
  type: string;
  /** Message data */
  data: unknown;
  /** Optional metadata */
  metadata?: Record<string, unknown>;
}
