/**
 * Cognitive Substrate SDK - Type Definitions
 * 
 * Shared type definitions for all CSCI v0.1 syscalls.
 * Part of @cognitive-substrate/sdk
 */

/**
 * Cognitive Task ID - globally unique identifier for a task
 */
export type CognitiveTaskId = string & { readonly __brand: 'CognitiveTaskId' };

export function createCognitiveTaskId(value: string): CognitiveTaskId {
  return value as CognitiveTaskId;
}

/**
 * Memory Region ID - identifier for allocated memory
 */
export type MemoryRegionId = string & { readonly __brand: 'MemoryRegionId' };

export function createMemoryRegionId(value: string): MemoryRegionId {
  return value as MemoryRegionId;
}

/**
 * Channel ID - identifier for IPC channel
 */
export type ChannelId = string & { readonly __brand: 'ChannelId' };

export function createChannelId(value: string): ChannelId {
  return value as ChannelId;
}

/**
 * Capability ID - identifier for security capability
 */
export type CapabilityId = string & { readonly __brand: 'CapabilityId' };

export function createCapabilityId(value: string): CapabilityId {
  return value as CapabilityId;
}

/**
 * Agent ID - identifier for an agent
 */
export type AgentId = string & { readonly __brand: 'AgentId' };

export function createAgentId(value: string): AgentId {
  return value as AgentId;
}

/**
 * Checkpoint ID - identifier for task checkpoint
 */
export type CheckpointId = string & { readonly __brand: 'CheckpointId' };

export function createCheckpointId(value: string): CheckpointId {
  return value as CheckpointId;
}

/**
 * Tool Binding ID - identifier for bound external tool
 */
export type ToolBindingId = string & { readonly __brand: 'ToolBindingId' };

export function createToolBindingId(value: string): ToolBindingId {
  return value as ToolBindingId;
}

/**
 * Crew ID - identifier for agent crew
 */
export type CrewId = string & { readonly __brand: 'CrewId' };

export function createCrewId(value: string): CrewId {
  return value as CrewId;
}

/**
 * Task configuration for task creation
 */
export interface TaskConfig {
  /** Task name */
  name: string;
  /** Execution timeout in milliseconds */
  timeout_ms?: number;
  /** Task priority (0-255, higher = more important) */
  priority?: number;
}

/**
 * Resource quota constraints
 */
export interface ResourceQuota {
  /** Memory quota in bytes */
  memory_bytes?: number;
  /** CPU quota in milliseconds */
  cpu_ms?: number;
  /** Maximum child tasks */
  max_children?: number;
}

/**
 * Memory tier specification
 */
export enum MemoryTier {
  L1 = 'L1', // Fast, small (working memory)
  L2 = 'L2', // Medium speed and capacity
  L3 = 'L3', // Large capacity, persistent
}

/**
 * Channel protocol type
 */
export enum ChannelProtocol {
  ByteStream = 'ByteStream',
  MessageBased = 'MessageBased',
}

/**
 * Message delivery guarantee
 */
export enum DeliveryGuarantee {
  BestEffort = 'BestEffort',
  AtLeastOnce = 'AtLeastOnce',
  ExactlyOnce = 'ExactlyOnce',
}

/**
 * Checkpoint type
 */
export enum CheckpointType {
  Full = 'Full',
  Incremental = 'Incremental',
}

/**
 * Yield hint for scheduler
 */
export enum YieldHint {
  Thinking = 'Thinking',
  WaitingForInput = 'WaitingForInput',
  ResourceLimited = 'ResourceLimited',
}

/**
 * Signal type
 */
export enum SignalType {
  TaskComplete = 'TaskComplete',
  ChannelMessage = 'ChannelMessage',
  Timeout = 'Timeout',
  ResourceWarning = 'ResourceWarning',
}

/**
 * Exception type
 */
export enum ExceptionType {
  MemoryViolation = 'MemoryViolation',
  CapabilityViolation = 'CapabilityViolation',
  TimeoutException = 'TimeoutException',
  AssertionFailure = 'AssertionFailure',
  StackOverflow = 'StackOverflow',
}

/**
 * Crew role
 */
export enum CrewRole {
  Coordinator = 'Coordinator',
  Worker = 'Worker',
  Specialist = 'Specialist',
}

/**
 * Sandbox configuration for tool execution
 */
export interface SandboxConfig {
  /** Memory limit in bytes */
  memory_limit?: number;
  /** Execution timeout in milliseconds */
  timeout_ms?: number;
  /** Allow network access */
  allow_network?: boolean;
  /** Allowed file system paths (optional) */
  allowed_paths?: string[];
}

/**
 * Tool specification
 */
export interface ToolSpec {
  /** Tool name */
  name: string;
  /** Tool version */
  version?: string;
  /** Tool type */
  type?: string;
}

/**
 * Tool invocation arguments
 */
export interface ToolArguments {
  [key: string]: unknown;
}

/**
 * Tool invocation result
 */
export interface ToolResult {
  /** Success indicator */
  success: boolean;
  /** Result data */
  data?: unknown;
  /** Error message if failed */
  error?: string;
}

/**
 * Capability specification
 */
export interface CapabilitySpec {
  /** Capability name */
  name: string;
  /** Resource being granted access to */
  resource?: string;
  /** Access level */
  access_level?: 'Read' | 'Write' | 'Execute' | 'Admin';
}

/**
 * Capability constraints
 */
export interface CapabilityConstraints {
  /** Expiration time (Unix timestamp) */
  expiration_timestamp?: number;
  /** Maximum number of uses */
  max_uses?: number;
  /** Whether capability can be delegated */
  delegable?: boolean;
}

/**
 * CSCI Error Code
 */
export enum CsciErrorCode {
  Success = 'CS_SUCCESS',
  PermissionDenied = 'CS_EPERM',
  NotFound = 'CS_ENOENT',
  OutOfMemory = 'CS_ENOMEM',
  ResourceBusy = 'CS_EBUSY',
  AlreadyExists = 'CS_EEXIST',
  InvalidArgument = 'CS_EINVAL',
  Timeout = 'CS_ETIMEOUT',
  BudgetExhausted = 'CS_EBUDGET',
  DependencyCycle = 'CS_ECYCLE',
  ChannelClosed = 'CS_ECLOSED',
  MessageTooLarge = 'CS_EMSGSIZE',
  PolicyDenied = 'CS_EPOLICY',
  SandboxError = 'CS_ESANDBOX',
}

/**
 * CSCI Error
 */
export class CsciError extends Error {
  constructor(
    public readonly code: CsciErrorCode,
    message: string,
  ) {
    super(message);
    this.name = 'CsciError';
  }
}

/**
 * Message payload for channels
 */
export interface MessagePayload {
  /** Message type */
  type: string;
  /** Message data */
  data: unknown;
  /** Optional metadata */
  metadata?: Record<string, unknown>;
}

/**
 * Memory slice for read operations
 */
export interface MemorySlice {
  /** Memory region ID */
  region_id: MemoryRegionId;
  /** Offset in region */
  offset: number;
  /** Data bytes */
  data: Uint8Array;
}

/**
 * Crew configuration
 */
export interface CrewConfig {
  /** Mission description */
  mission: string;
  /** Coordinator agent ID */
  coordinator_agent: AgentId;
  /** Initial crew members */
  initial_members?: AgentId[];
  /** Collective budget */
  collective_budget?: number;
}
