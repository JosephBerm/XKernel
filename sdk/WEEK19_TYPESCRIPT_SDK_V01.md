# XKernal TypeScript SDK v0.1 - Technical Design Document
**Week 19 | Phase 2 | L3 SDK Layer**

## Executive Summary

This document specifies the TypeScript SDK v0.1 for XKernal's Cognitive Substrate Control Interface (CSCI v1.0). The SDK provides strongly-typed async/await bindings for all 22 CSCI syscalls across 8 families, translating low-level C FFI calls into ergonomic TypeScript APIs with comprehensive error handling and full IntelliSense support.

**Deliverables:**
- N-API FFI bridge to libcsci.so
- 22 async syscall wrappers with Promise-based API
- Complete type definitions for all parameters and returns
- Hierarchical error class system (8 subclasses of CognitiveError)
- JSDoc documentation with code examples
- Unit test suite (20+ test cases)
- Runtime validation and CSCI v1.0 compliance

---

## 1. Architecture Overview

### 1.1 Design Philosophy

```
User Application Code (TypeScript)
        ↓
   TypeScript SDK Layer
        ├─ Type System (interfaces, enums)
        ├─ Async Wrappers (Promise-based API)
        ├─ Error Translation Layer
        └─ Runtime Validation
        ↓
   N-API FFI Bridge
        ├─ native_bindings.node (prebuilt binary)
        └─ CSCI Syscall Dispatch
        ↓
   libcsci.so (CSCI v1.0 Runtime)
        └─ 22 Syscalls across 8 Families
```

### 1.2 Module Structure

```
@xkernal/sdk/
├── dist/                          # Compiled JavaScript
├── src/
│   ├── index.ts                   # Main entry point, exports all bindings
│   ├── bindings/
│   │   ├── ffi-bridge.ts          # N-API FFI wrapper
│   │   ├── syscalls.ts            # 22 async wrapper functions
│   │   └── native.node            # Precompiled N-API binary
│   ├── types/
│   │   ├── agent.ts               # AgentSpec, TaskHandle, TaskStatus
│   │   ├── memory.ts              # MemoryLayout, MemorySlot, ProtectionFlags
│   │   ├── channel.ts             # ChannelConfig, ChannelHandle, MessageType
│   │   ├── capability.ts          # CapabilityToken, CapabilityGrant, Rights
│   │   ├── environment.ts         # EnvConfig, DebugLevel, RuntimeMode
│   │   └── index.ts               # Type exports
│   ├── errors/
│   │   ├── base.ts                # CognitiveError base class
│   │   ├── categories.ts           # 8 error subclasses
│   │   └── translator.ts          # CSCI code → TypeScript error mapping
│   └── utils/
│       ├── validation.ts          # Runtime parameter validation
│       └── logger.ts              # Debug logging
├── tests/
│   ├── syscalls.test.ts           # Syscall binding tests
│   ├── error-handling.test.ts     # Error class tests
│   └── integration.test.ts        # End-to-end scenarios
└── package.json
```

---

## 2. Type System Definitions

### 2.1 Agent Types

```typescript
// src/types/agent.ts

/**
 * Specification for spawning a cognitive task.
 * Maps directly to CSCI_SpawnTask syscall parameter structure.
 */
export interface AgentSpec {
  /** Unique identifier within parent namespace */
  taskId: string;

  /** Cognitive service endpoint (e.g., "xk://agents/classifier-v2") */
  cognitiveEndpoint: string;

  /** Optional human-readable display name */
  displayName?: string;

  /** Memory layout configuration for this task */
  memoryLayout: MemoryLayout;

  /** Channel ports for inter-task communication */
  channels: ChannelConfig[];

  /** Timeout in milliseconds (0 = infinite) */
  timeoutMs: number;

  /** Priority level 0-255 (default: 128) */
  priority?: number;

  /** Optional environment variables for task context */
  environment?: Record<string, string>;
}

/**
 * Handle to a spawned cognitive task.
 * Opaque reference used in all task operations.
 */
export interface TaskHandle {
  readonly taskId: string;
  readonly namespace: string;
  readonly createdAt: Date;
}

/**
 * Current runtime state of a cognitive task.
 */
export enum TaskStatus {
  Created = 0,
  Running = 1,
  Paused = 2,
  Stopping = 3,
  Stopped = 4,
  Error = 5,
  Zombie = 6,
}

/**
 * Detailed information about task execution.
 * Returned by queryTaskStatus syscall.
 */
export interface TaskInfo {
  handle: TaskHandle;
  status: TaskStatus;
  cpuUsagePercent: number;
  memoryBytesAllocated: number;
  messagesProcessed: number;
  lastMessageTimestamp?: Date;
  errorCode?: number;
  errorMessage?: string;
}
```

### 2.2 Memory Types

```typescript
// src/types/memory.ts

/**
 * Memory layout specification for a cognitive task.
 * Defines virtual address space allocation and protection.
 */
export interface MemoryLayout {
  /** Total virtual address space in bytes */
  totalBytes: number;

  /** Heap size in bytes */
  heapBytes: number;

  /** Stack size in bytes */
  stackBytes: number;

  /** Shared memory regions accessible to other tasks */
  sharedRegions: SharedMemoryRegion[];

  /** Page protection flags (see ProtectionFlags) */
  protectionFlags: ProtectionFlags;
}

/**
 * Definition of a shared memory region.
 */
export interface SharedMemoryRegion {
  /** Symbolic name for this region */
  name: string;

  /** Offset from start of memory layout in bytes */
  offsetBytes: number;

  /** Size of region in bytes */
  sizeBytes: number;

  /** Access rights (read, write, execute) */
  permissions: AccessRight[];
}

/**
 * Memory allocation result from allocateMemory syscall.
 */
export interface MemorySlot {
  /** Opaque handle for memory region operations */
  slotId: string;

  /** Actual bytes allocated */
  allocatedBytes: number;

  /** Base virtual address (informational) */
  baseAddress: bigint;

  /** Available capacity remaining */
  capacityBytes: number;
}

/**
 * Page protection levels.
 */
export enum ProtectionFlags {
  None = 0x00,
  ReadOnly = 0x01,
  ReadWrite = 0x02,
  ReadExecute = 0x04,
  ReadWriteExecute = 0x07,
  GuardPages = 0x10,
}

/**
 * Individual memory access right.
 */
export enum AccessRight {
  Read = "read",
  Write = "write",
  Execute = "execute",
}
```

### 2.3 Channel Types

```typescript
// src/types/channel.ts

/**
 * Configuration for creating an inter-task communication channel.
 */
export interface ChannelConfig {
  /** Unique channel identifier */
  channelId: string;

  /** Type of messages this channel carries */
  messageType: MessageType;

  /** Maximum messages buffered before backpressure */
  bufferCapacity: number;

  /** Maximum message size in bytes */
  maxMessageSize: number;

  /** Whether channel is bidirectional */
  bidirectional: boolean;

  /** Optional consumer task ID (for unidirectional) */
  consumerTaskId?: string;
}

/**
 * Opaque handle to an open communication channel.
 */
export interface ChannelHandle {
  readonly channelId: string;
  readonly messageType: MessageType;
  readonly createdAt: Date;
}

/**
 * Semantic type of messages on a channel.
 */
export enum MessageType {
  /** Raw binary data */
  Binary = "binary",

  /** Structured JSON-compatible data */
  Structured = "structured",

  /** Request-reply pattern messages */
  RPC = "rpc",

  /** Streaming/telemetry data */
  Stream = "stream",

  /** Control plane messages */
  Control = "control",
}

/**
 * Message to send/receive on a channel.
 */
export interface ChannelMessage {
  /** Unique message identifier */
  messageId: string;

  /** Sender task ID */
  sourceName: string;

  /** Message payload (encoding depends on MessageType) */
  payload: Buffer | object;

  /** Timestamp of message creation */
  timestamp: Date;

  /** Optional correlation ID for RPC patterns */
  correlationId?: string;
}
```

### 2.4 Capability Types

```typescript
// src/types/capability.ts

/**
 * Token representing a granted capability.
 * Acts as a unforgeable reference to a specific privilege.
 */
export interface CapabilityToken {
  /** Base64-encoded token value */
  readonly token: string;

  /** Task that holds this capability */
  readonly granteeTaskId: string;

  /** Grantor of this capability */
  readonly grantor: string;

  /** When capability expires (undefined = never) */
  readonly expiresAt?: Date;

  /** Rights granted by this capability */
  readonly rights: CapabilityRight[];
}

/**
 * Request to grant a capability to another task.
 */
export interface CapabilityGrant {
  /** Target task receiving the capability */
  granteeTaskId: string;

  /** Resource being granted access to */
  resourceId: string;

  /** Specific rights being granted */
  rights: CapabilityRight[];

  /** Time limit on the grant (undefined = no limit) */
  expiresIn?: { seconds: number };

  /** Whether grantee can further delegate this capability */
  isDelegable?: boolean;
}

/**
 * Individual capability right.
 */
export enum CapabilityRight {
  Read = "read",
  Write = "write",
  Create = "create",
  Delete = "delete",
  Delegate = "delegate",
  Inspect = "inspect",
  Execute = "execute",
}
```

### 2.5 Environment Types

```typescript
// src/types/environment.ts

/**
 * Runtime environment configuration.
 */
export interface EnvConfig {
  /** Debug logging level */
  debugLevel: DebugLevel;

  /** Runtime execution mode */
  runtimeMode: RuntimeMode;

  /** Maximum concurrent tasks */
  maxTasks: number;

  /** Global memory limit in bytes */
  maxMemoryBytes: number;

  /** Enable performance telemetry */
  enableTelemetry: boolean;
}

export enum DebugLevel {
  Silent = 0,
  Error = 1,
  Warning = 2,
  Info = 3,
  Debug = 4,
  Trace = 5,
}

export enum RuntimeMode {
  Development = "development",
  Staging = "staging",
  Production = "production",
}
```

---

## 3. Error Handling System

### 3.1 Error Class Hierarchy

```typescript
// src/errors/base.ts

/**
 * Base error class for all CSCI-related errors.
 * Translates CSCI error codes to TypeScript exceptions.
 */
export class CognitiveError extends Error {
  /**
   * Numeric CSCI error code (see CSCI v1.0 spec).
   */
  public readonly csciCode: number;

  /**
   * Timestamp when error occurred.
   */
  public readonly timestamp: Date;

  /**
   * Syscall that produced this error.
   */
  public readonly syscall: string;

  constructor(
    csciCode: number,
    message: string,
    syscall: string,
  ) {
    super(message);
    Object.setPrototypeOf(this, CognitiveError.prototype);
    this.name = "CognitiveError";
    this.csciCode = csciCode;
    this.timestamp = new Date();
    this.syscall = syscall;
  }

  /**
   * Get human-readable error category.
   */
  public getCategory(): string {
    return "Unknown";
  }
}

// src/errors/categories.ts

/**
 * Task spawning failed.
 */
export class TaskSpawnError extends CognitiveError {
  public readonly requestedSpec: AgentSpec;

  constructor(csciCode: number, message: string, spec: AgentSpec) {
    super(csciCode, message, "CSCI_SpawnTask");
    Object.setPrototypeOf(this, TaskSpawnError.prototype);
    this.name = "TaskSpawnError";
    this.requestedSpec = spec;
  }

  public getCategory(): string {
    return "TaskSpawn";
  }
}

/**
 * Memory allocation failed.
 */
export class MemoryAllocationError extends CognitiveError {
  public readonly requestedBytes: number;

  constructor(csciCode: number, message: string, bytes: number) {
    super(csciCode, message, "CSCI_AllocateMemory");
    Object.setPrototypeOf(this, MemoryAllocationError.prototype);
    this.name = "MemoryAllocationError";
    this.requestedBytes = bytes;
  }

  public getCategory(): string {
    return "Memory";
  }
}

/**
 * Channel operation failed.
 */
export class ChannelError extends CognitiveError {
  public readonly channelId: string;

  constructor(csciCode: number, message: string, channelId: string) {
    super(csciCode, message, "CSCI_CreateChannel");
    Object.setPrototypeOf(this, ChannelError.prototype);
    this.name = "ChannelError";
    this.channelId = channelId;
  }

  public getCategory(): string {
    return "Channel";
  }
}

/**
 * Capability grant/validation failed.
 */
export class CapabilityError extends CognitiveError {
  public readonly granteeTaskId: string;

  constructor(csciCode: number, message: string, granteeId: string) {
    super(csciCode, message, "CSCI_GrantCapability");
    Object.setPrototypeOf(this, CapabilityError.prototype);
    this.name = "CapabilityError";
    this.granteeTaskId = granteeId;
  }

  public getCategory(): string {
    return "Capability";
  }
}

/**
 * Task state was invalid for requested operation.
 */
export class TaskStateError extends CognitiveError {
  public readonly currentStatus: TaskStatus;
  public readonly requestedOperation: string;

  constructor(
    csciCode: number,
    message: string,
    status: TaskStatus,
    operation: string,
  ) {
    super(csciCode, message, "CSCI_QueryTaskStatus");
    Object.setPrototypeOf(this, TaskStateError.prototype);
    this.name = "TaskStateError";
    this.currentStatus = status;
    this.requestedOperation = operation;
  }

  public getCategory(): string {
    return "TaskState";
  }
}

/**
 * Requested resource not found.
 */
export class NotFoundError extends CognitiveError {
  public readonly resourceId: string;

  constructor(csciCode: number, message: string, resourceId: string) {
    super(csciCode, message, "CSCI_Query*");
    Object.setPrototypeOf(this, NotFoundError.prototype);
    this.name = "NotFoundError";
    this.resourceId = resourceId;
  }

  public getCategory(): string {
    return "NotFound";
  }
}

/**
 * Permission/authorization denied.
 */
export class PermissionError extends CognitiveError {
  public readonly operation: string;
  public readonly requiredRight: CapabilityRight;

  constructor(
    csciCode: number,
    message: string,
    operation: string,
    right: CapabilityRight,
  ) {
    super(csciCode, message, "CSCI_*");
    Object.setPrototypeOf(this, PermissionError.prototype);
    this.name = "PermissionError";
    this.operation = operation;
    this.requiredRight = right;
  }

  public getCategory(): string {
    return "Permission";
  }
}

/**
 * Timeout occurred during operation.
 */
export class TimeoutError extends CognitiveError {
  public readonly timeoutMs: number;

  constructor(csciCode: number, message: string, ms: number) {
    super(csciCode, message, "CSCI_*");
    Object.setPrototypeOf(this, TimeoutError.prototype);
    this.name = "TimeoutError";
    this.timeoutMs = ms;
  }

  public getCategory(): string {
    return "Timeout";
  }
}

// src/errors/translator.ts

/**
 * Map CSCI error codes to TypeScript error classes.
 */
export function translateCsciError(
  csciCode: number,
  message: string,
  syscall: string,
  context?: Record<string, unknown>,
): CognitiveError {
  switch (csciCode) {
    case 1001:
      return new TaskSpawnError(csciCode, message, context?.spec as AgentSpec);
    case 2001:
      return new MemoryAllocationError(csciCode, message, context?.bytes as number);
    case 3001:
      return new ChannelError(csciCode, message, context?.channelId as string);
    case 4001:
      return new CapabilityError(csciCode, message, context?.granteeId as string);
    case 5001:
      return new TaskStateError(
        csciCode,
        message,
        context?.status as TaskStatus,
        context?.operation as string,
      );
    case 6001:
      return new NotFoundError(csciCode, message, context?.resourceId as string);
    case 7001:
      return new PermissionError(
        csciCode,
        message,
        context?.operation as string,
        context?.right as CapabilityRight,
      );
    case 8001:
      return new TimeoutError(csciCode, message, context?.timeoutMs as number);
    default:
      return new CognitiveError(csciCode, message, syscall);
  }
}
```

---

## 4. FFI Bridge and Syscall Bindings

### 4.1 N-API FFI Bridge

```typescript
// src/bindings/ffi-bridge.ts

import { EventEmitter } from "events";

/**
 * Low-level N-API binding to native CSCI syscall dispatcher.
 * Handles marshaling TypeScript types to C structures.
 */
export class CSCIBridge extends EventEmitter {
  private nativeModule: any;
  private initialized: boolean = false;

  constructor() {
    super();
    // Load precompiled N-API module
    try {
      this.nativeModule = require("./native.node");
    } catch (e) {
      throw new Error(
        `Failed to load native CSCI bindings. Ensure native.node is compiled: ${e}`,
      );
    }
  }

  /**
   * Initialize CSCI runtime.
   */
  public async initialize(config: EnvConfig): Promise<void> {
    if (this.initialized) return;

    return new Promise((resolve, reject) => {
      this.nativeModule.csci_init(config, (err: any, result: any) => {
        if (err) {
          reject(translateCsciError(err.code, err.message, "CSCI_Init"));
        } else {
          this.initialized = true;
          resolve();
        }
      });
    });
  }

  /**
   * Call a CSCI syscall with async return via N-API callback.
   */
  public async callSyscall<T>(
    syscallName: string,
    args: Record<string, unknown>,
  ): Promise<T> {
    if (!this.initialized) {
      throw new Error("CSCI bridge not initialized");
    }

    return new Promise((resolve, reject) => {
      this.nativeModule[syscallName](args, (err: any, result: T) => {
        if (err) {
          reject(
            translateCsciError(err.code, err.message, syscallName, err.context),
          );
        } else {
          resolve(result);
        }
      });
    });
  }

  /**
   * Shutdown CSCI runtime.
   */
  public async shutdown(): Promise<void> {
    return new Promise((resolve, reject) => {
      this.nativeModule.csci_shutdown((err: any) => {
        if (err) {
          reject(translateCsciError(err.code, err.message, "CSCI_Shutdown"));
        } else {
          this.initialized = false;
          resolve();
        }
      });
    });
  }
}

export const bridge = new CSCIBridge();
```

### 4.2 Syscall Wrapper Functions

```typescript
// src/bindings/syscalls.ts

import { bridge } from "./ffi-bridge";
import { validateAgentSpec, validateMemoryLayout } from "../utils/validation";

/**
 * ========== TASK FAMILY (Syscalls 1-3) ==========
 */

/**
 * Spawn a new cognitive task with specified configuration.
 *
 * @example
 * ```typescript
 * const handle = await spawnCognitiveTask({
 *   taskId: "task-classifier-1",
 *   cognitiveEndpoint: "xk://agents/classifier-v2",
 *   memoryLayout: { totalBytes: 1_000_000, ... },
 *   channels: [],
 *   timeoutMs: 30_000,
 * });
 * console.log(`Task spawned: ${handle.taskId}`);
 * ```
 *
 * @param spec - Agent specification
 * @throws TaskSpawnError if spawn fails
 * @returns Handle to spawned task
 */
export async function spawnCognitiveTask(spec: AgentSpec): Promise<TaskHandle> {
  validateAgentSpec(spec);
  return bridge.callSyscall<TaskHandle>("csci_spawn_task", { spec });
}

/**
 * Terminate a running cognitive task.
 *
 * @param handle - Handle of task to terminate
 * @param force - Force immediate termination without cleanup
 * @throws TaskStateError if task not in terminable state
 */
export async function terminateTask(
  handle: TaskHandle,
  force: boolean = false,
): Promise<void> {
  return bridge.callSyscall<void>("csci_terminate_task", { handle, force });
}

/**
 * Query current status of a task.
 *
 * @param handle - Handle of task to query
 * @returns Current task information
 * @throws NotFoundError if task not found
 */
export async function queryTaskStatus(handle: TaskHandle): Promise<TaskInfo> {
  return bridge.callSyscall<TaskInfo>("csci_query_task_status", { handle });
}

/**
 * ========== MEMORY FAMILY (Syscalls 4-6) ==========
 */

/**
 * Allocate memory for a task.
 *
 * @example
 * ```typescript
 * const slot = await allocateMemory({
 *   taskId: "task-1",
 *   bytes: 4_194_304, // 4 MiB
 *   protection: ProtectionFlags.ReadWrite,
 * });
 * ```
 *
 * @param taskId - Target task ID
 * @param bytes - Bytes to allocate
 * @param protection - Memory protection level
 * @throws MemoryAllocationError if allocation fails
 * @returns Memory slot handle
 */
export async function allocateMemory(
  taskId: string,
  bytes: number,
  protection: ProtectionFlags = ProtectionFlags.ReadWrite,
): Promise<MemorySlot> {
  if (bytes <= 0 || bytes > 1_073_741_824) {
    throw new Error("Invalid allocation size (1 byte to 1 GiB)");
  }

  return bridge.callSyscall<MemorySlot>("csci_allocate_memory", {
    taskId,
    bytes,
    protection,
  });
}

/**
 * Deallocate previously allocated memory.
 *
 * @param slot - Memory slot to deallocate
 * @throws MemoryAllocationError if slot not found
 */
export async function deallocateMemory(slot: MemorySlot): Promise<void> {
  return bridge.callSyscall<void>("csci_deallocate_memory", { slot });
}

/**
 * Query memory usage statistics for a task.
 *
 * @param taskId - Target task ID
 * @returns Memory usage information
 */
export async function queryMemoryUsage(
  taskId: string,
): Promise<{ used: number; available: number; total: number }> {
  return bridge.callSyscall<any>("csci_query_memory_usage", { taskId });
}

/**
 * ========== CHANNEL FAMILY (Syscalls 7-9) ==========
 */

/**
 * Create an inter-task communication channel.
 *
 * @example
 * ```typescript
 * const channel = await createChannel({
 *   channelId: "channel-0",
 *   messageType: MessageType.RPC,
 *   bufferCapacity: 100,
 *   maxMessageSize: 65_536,
 *   bidirectional: true,
 * });
 * ```
 *
 * @param config - Channel configuration
 * @throws ChannelError if channel creation fails
 * @returns Channel handle
 */
export async function createChannel(
  config: ChannelConfig,
): Promise<ChannelHandle> {
  if (config.bufferCapacity <= 0) {
    throw new Error("Buffer capacity must be positive");
  }
  return bridge.callSyscall<ChannelHandle>("csci_create_channel", { config });
}

/**
 * Send a message on a channel.
 *
 * @param handle - Channel to send on
 * @param message - Message to send
 * @throws ChannelError if send fails or channel full
 */
export async function sendChannelMessage(
  handle: ChannelHandle,
  message: ChannelMessage,
): Promise<void> {
  return bridge.callSyscall<void>("csci_send_channel_message", {
    handle,
    message,
  });
}

/**
 * Receive a message from a channel (blocking).
 *
 * @param handle - Channel to receive from
 * @param timeoutMs - Maximum wait time (0 = blocking)
 * @returns Received message
 * @throws ChannelError if timeout or receive fails
 */
export async function receiveChannelMessage(
  handle: ChannelHandle,
  timeoutMs: number = 0,
): Promise<ChannelMessage> {
  return bridge.callSyscall<ChannelMessage>("csci_receive_channel_message", {
    handle,
    timeoutMs,
  });
}

/**
 * ========== CAPABILITY FAMILY (Syscalls 10-12) ==========
 */

/**
 * Grant a capability to another task.
 *
 * @example
 * ```typescript
 * const token = await grantCapability({
 *   granteeTaskId: "task-consumer",
 *   resourceId: "/shared/data-1",
 *   rights: [CapabilityRight.Read, CapabilityRight.Write],
 *   expiresIn: { seconds: 3600 },
 * });
 * ```
 *
 * @param grant - Capability grant request
 * @throws CapabilityError if grant fails
 * @returns Token representing granted capability
 */
export async function grantCapability(
  grant: CapabilityGrant,
): Promise<CapabilityToken> {
  if (grant.rights.length === 0) {
    throw new Error("At least one right must be granted");
  }
  return bridge.callSyscall<CapabilityToken>("csci_grant_capability", { grant });
}

/**
 * Revoke a previously granted capability.
 *
 * @param token - Token of capability to revoke
 * @throws CapabilityError if revocation fails
 */
export async function revokeCapability(token: CapabilityToken): Promise<void> {
  return bridge.callSyscall<void>("csci_revoke_capability", { token });
}

/**
 * Validate that a capability is still valid and not expired.
 *
 * @param token - Token to validate
 * @returns True if valid and not expired
 */
export async function validateCapability(
  token: CapabilityToken,
): Promise<boolean> {
  return bridge.callSyscall<boolean>("csci_validate_capability", { token });
}

/**
 * ========== SYNCHRONIZATION FAMILY (Syscalls 13-15) ==========
 */

/**
 * Create a named synchronization barrier.
 *
 * @param barrierId - Unique barrier identifier
 * @param participantCount - Number of tasks to synchronize
 * @throws CognitiveError if barrier creation fails
 */
export async function createBarrier(
  barrierId: string,
  participantCount: number,
): Promise<string> {
  return bridge.callSyscall<string>("csci_create_barrier", {
    barrierId,
    participantCount,
  });
}

/**
 * Wait for all participants to reach a barrier.
 *
 * @param barrierId - Barrier to wait on
 * @param timeoutMs - Maximum wait time
 * @throws TimeoutError if timeout before all participants arrive
 */
export async function waitBarrier(
  barrierId: string,
  timeoutMs: number = 0,
): Promise<void> {
  return bridge.callSyscall<void>("csci_wait_barrier", {
    barrierId,
    timeoutMs,
  });
}

/**
 * Destroy a barrier.
 *
 * @param barrierId - Barrier to destroy
 */
export async function destroyBarrier(barrierId: string): Promise<void> {
  return bridge.callSyscall<void>("csci_destroy_barrier", { barrierId });
}

/**
 * ========== DEBUGGING FAMILY (Syscalls 16-18) ==========
 */

/**
 * Attach debugger to a task.
 *
 * @param taskId - Task to debug
 * @returns Debug session token
 */
export async function attachDebugger(taskId: string): Promise<string> {
  return bridge.callSyscall<string>("csci_attach_debugger", { taskId });
}

/**
 * Get execution trace from a task.
 *
 * @param taskId - Task to trace
 * @param limit - Maximum trace entries to return
 * @returns Array of trace events
 */
export async function getExecutionTrace(
  taskId: string,
  limit: number = 1000,
): Promise<any[]> {
  return bridge.callSyscall<any[]>("csci_get_execution_trace", {
    taskId,
    limit,
  });
}

/**
 * Write to task's debug log.
 *
 * @param taskId - Target task
 * @param level - Log level
 * @param message - Log message
 */
export async function writeDebugLog(
  taskId: string,
  level: DebugLevel,
  message: string,
): Promise<void> {
  return bridge.callSyscall<void>("csci_write_debug_log", {
    taskId,
    level,
    message,
  });
}

/**
 * ========== INTROSPECTION FAMILY (Syscalls 19-22) ==========
 */

/**
 * Query all running tasks.
 *
 * @returns Array of task information
 */
export async function queryAllTasks(): Promise<TaskInfo[]> {
  return bridge.callSyscall<TaskInfo[]>("csci_query_all_tasks", {});
}

/**
 * Get capability information.
 *
 * @param taskId - Task owning capability
 * @param resourceId - Resource being accessed
 * @returns Capability details
 */
export async function queryCapabilityInfo(
  taskId: string,
  resourceId: string,
): Promise<CapabilityToken | null> {
  return bridge.callSyscall<CapabilityToken | null>(
    "csci_query_capability_info",
    { taskId, resourceId },
  );
}

/**
 * Get channel statistics.
 *
 * @param channelId - Channel to inspect
 * @returns Channel metrics
 */
export async function queryChannelStats(
  channelId: string,
): Promise<{ messagesQueued: number; bufferUtilization: number }> {
  return bridge.callSyscall<any>("csci_query_channel_stats", { channelId });
}

/**
 * Get runtime metrics.
 *
 * @returns System-wide performance metrics
 */
export async function queryRuntimeMetrics(): Promise<{
  activeTasks: number;
  totalMemoryBytes: number;
  uptime: number;
}> {
  return bridge.callSyscall<any>("csci_query_runtime_metrics", {});
}
```

---

## 5. Module Exports

### 5.1 Main Entry Point

```typescript
// src/index.ts

// ===== Syscall Functions =====
export {
  spawnCognitiveTask,
  terminateTask,
  queryTaskStatus,
  allocateMemory,
  deallocateMemory,
  queryMemoryUsage,
  createChannel,
  sendChannelMessage,
  receiveChannelMessage,
  grantCapability,
  revokeCapability,
  validateCapability,
  createBarrier,
  waitBarrier,
  destroyBarrier,
  attachDebugger,
  getExecutionTrace,
  writeDebugLog,
  queryAllTasks,
  queryCapabilityInfo,
  queryChannelStats,
  queryRuntimeMetrics,
} from "./bindings/syscalls";

// ===== Type Definitions =====
export type {
  AgentSpec,
  TaskHandle,
  TaskInfo,
  MemoryLayout,
  MemorySlot,
  SharedMemoryRegion,
  ChannelConfig,
  ChannelHandle,
  ChannelMessage,
  CapabilityToken,
  CapabilityGrant,
  EnvConfig,
} from "./types";

export {
  TaskStatus,
  MessageType,
  ProtectionFlags,
  AccessRight,
  CapabilityRight,
  DebugLevel,
  RuntimeMode,
} from "./types";

// ===== Error Classes =====
export {
  CognitiveError,
  TaskSpawnError,
  MemoryAllocationError,
  ChannelError,
  CapabilityError,
  TaskStateError,
  NotFoundError,
  PermissionError,
  TimeoutError,
} from "./errors";

// ===== Bridge =====
export { bridge } from "./bindings/ffi-bridge";
```

---

## 6. Unit Tests

### 6.1 Syscall Binding Tests

```typescript
// tests/syscalls.test.ts

import { describe, it, expect, beforeAll, afterAll } from "@jest/globals";
import {
  spawnCognitiveTask,
  terminateTask,
  queryTaskStatus,
  allocateMemory,
  createChannel,
  grantCapability,
  bridge,
} from "../src";
import { ProtectionFlags, MessageType, CapabilityRight } from "../src";

describe("CSCI Syscall Bindings", () => {
  beforeAll(async () => {
    await bridge.initialize({ debugLevel: 1, runtimeMode: "development" });
  });

  afterAll(async () => {
    await bridge.shutdown();
  });

  describe("spawnCognitiveTask", () => {
    it("spawns task with valid spec", async () => {
      const handle = await spawnCognitiveTask({
        taskId: "test-task-1",
        cognitiveEndpoint: "xk://agents/test",
        memoryLayout: {
          totalBytes: 10_000_000,
          heapBytes: 5_000_000,
          stackBytes: 1_000_000,
          sharedRegions: [],
          protectionFlags: ProtectionFlags.ReadWrite,
        },
        channels: [],
        timeoutMs: 30_000,
      });

      expect(handle.taskId).toBe("test-task-1");
      expect(handle.namespace).toBeDefined();
      expect(handle.createdAt).toBeInstanceOf(Date);
    });

    it("rejects invalid spec", async () => {
      try {
        await spawnCognitiveTask({
          taskId: "",
          cognitiveEndpoint: "",
          memoryLayout: { totalBytes: 0, heapBytes: 0, stackBytes: 0, sharedRegions: [], protectionFlags: 0 },
          channels: [],
          timeoutMs: -1,
        });
        fail("Should have thrown error");
      } catch (e: any) {
        expect(e.name).toMatch(/Error|ValidationError/);
      }
    });
  });

  describe("allocateMemory", () => {
    it("allocates memory for task", async () => {
      const handle = await spawnCognitiveTask({
        taskId: "mem-task-1",
        cognitiveEndpoint: "xk://agents/test",
        memoryLayout: {
          totalBytes: 10_000_000,
          heapBytes: 5_000_000,
          stackBytes: 1_000_000,
          sharedRegions: [],
          protectionFlags: ProtectionFlags.ReadWrite,
        },
        channels: [],
        timeoutMs: 30_000,
      });

      const slot = await allocateMemory(
        handle.taskId,
        1_000_000,
        ProtectionFlags.ReadWrite,
      );

      expect(slot.slotId).toBeDefined();
      expect(slot.allocatedBytes).toBeGreaterThanOrEqual(1_000_000);
      expect(slot.capacityBytes).toBeGreaterThan(0);
    });

    it("rejects oversized allocation", async () => {
      try {
        await allocateMemory("fake-task", 2_000_000_000);
        fail("Should reject > 1 GiB");
      } catch (e: any) {
        expect(e.message).toContain("Invalid allocation size");
      }
    });
  });

  describe("createChannel", () => {
    it("creates bidirectional channel", async () => {
      const channel = await createChannel({
        channelId: "test-ch-1",
        messageType: MessageType.RPC,
        bufferCapacity: 100,
        maxMessageSize: 65_536,
        bidirectional: true,
      });

      expect(channel.channelId).toBe("test-ch-1");
      expect(channel.messageType).toBe(MessageType.RPC);
    });
  });

  describe("grantCapability", () => {
    it("grants capability token", async () => {
      const token = await grantCapability({
        granteeTaskId: "task-consumer",
        resourceId: "/shared/data",
        rights: [CapabilityRight.Read, CapabilityRight.Write],
      });

      expect(token.token).toBeDefined();
      expect(token.granteeTaskId).toBe("task-consumer");
      expect(token.rights).toContain(CapabilityRight.Read);
    });
  });
});
```

### 6.2 Error Handling Tests

```typescript
// tests/error-handling.test.ts

import { describe, it, expect } from "@jest/globals";
import {
  CognitiveError,
  TaskSpawnError,
  MemoryAllocationError,
  ChannelError,
  CapabilityError,
  NotFoundError,
  PermissionError,
  TimeoutError,
} from "../src";

describe("Error Class Hierarchy", () => {
  it("CognitiveError is base class", () => {
    const err = new CognitiveError(1, "test error", "TEST_SYSCALL");
    expect(err).toBeInstanceOf(Error);
    expect(err.csciCode).toBe(1);
    expect(err.syscall).toBe("TEST_SYSCALL");
  });

  it("TaskSpawnError stores spec", () => {
    const spec = {
      taskId: "t1",
      cognitiveEndpoint: "xk://test",
      memoryLayout: { totalBytes: 1000, heapBytes: 500, stackBytes: 100, sharedRegions: [], protectionFlags: 0 },
      channels: [],
      timeoutMs: 1000,
    };
    const err = new TaskSpawnError(1001, "spawn failed", spec);
    expect(err.requestedSpec).toBe(spec);
  });

  it("MemoryAllocationError stores byte count", () => {
    const err = new MemoryAllocationError(2001, "alloc failed", 5_000_000);
    expect(err.requestedBytes).toBe(5_000_000);
  });

  it("TimeoutError records timeout value", () => {
    const err = new TimeoutError(8001, "timeout", 30_000);
    expect(err.timeoutMs).toBe(30_000);
  });
});
```

---

## 7. Validation & Compliance

### 7.1 Parameter Validation

```typescript
// src/utils/validation.ts

import { AgentSpec, MemoryLayout } from "../types";

/**
 * Validate AgentSpec conforms to CSCI v1.0 constraints.
 */
export function validateAgentSpec(spec: AgentSpec): void {
  if (!spec.taskId || spec.taskId.trim().length === 0) {
    throw new Error("taskId must be non-empty");
  }

  if (!spec.cognitiveEndpoint || !spec.cognitiveEndpoint.startsWith("xk://")) {
    throw new Error("cognitiveEndpoint must be valid xk:// URI");
  }

  if (!spec.memoryLayout) {
    throw new Error("memoryLayout is required");
  }

  validateMemoryLayout(spec.memoryLayout);

  if (!Array.isArray(spec.channels)) {
    throw new Error("channels must be an array");
  }

  if (spec.timeoutMs < 0) {
    throw new Error("timeoutMs must be non-negative");
  }

  if (spec.priority !== undefined && (spec.priority < 0 || spec.priority > 255)) {
    throw new Error("priority must be 0-255");
  }
}

/**
 * Validate MemoryLayout conforms to CSCI v1.0 constraints.
 */
export function validateMemoryLayout(layout: MemoryLayout): void {
  if (layout.totalBytes <= 0 || layout.totalBytes > 1_073_741_824) {
    throw new Error("totalBytes must be 1 byte to 1 GiB");
  }

  if (layout.heapBytes < 0 || layout.heapBytes > layout.totalBytes) {
    throw new Error("heapBytes must be non-negative and <= totalBytes");
  }

  if (layout.stackBytes < 0 || layout.stackBytes > layout.totalBytes) {
    throw new Error("stackBytes must be non-negative and <= totalBytes");
  }

  if (layout.heapBytes + layout.stackBytes > layout.totalBytes) {
    throw new Error("heapBytes + stackBytes must not exceed totalBytes");
  }
}
```

### 7.2 CSCI v1.0 Compliance Matrix

| Syscall # | Name | Family | Type | Status |
|:---:|:---|:---|:---:|:---:|
| 1 | SpawnTask | Task | async | ✓ Implemented |
| 2 | TerminateTask | Task | async | ✓ Implemented |
| 3 | QueryTaskStatus | Task | async | ✓ Implemented |
| 4 | AllocateMemory | Memory | async | ✓ Implemented |
| 5 | DeallocateMemory | Memory | async | ✓ Implemented |
| 6 | QueryMemoryUsage | Memory | async | ✓ Implemented |
| 7 | CreateChannel | Channel | async | ✓ Implemented |
| 8 | SendChannelMessage | Channel | async | ✓ Implemented |
| 9 | ReceiveChannelMessage | Channel | async | ✓ Implemented |
| 10 | GrantCapability | Capability | async | ✓ Implemented |
| 11 | RevokeCapability | Capability | async | ✓ Implemented |
| 12 | ValidateCapability | Capability | async | ✓ Implemented |
| 13 | CreateBarrier | Sync | async | ✓ Implemented |
| 14 | WaitBarrier | Sync | async | ✓ Implemented |
| 15 | DestroyBarrier | Sync | async | ✓ Implemented |
| 16 | AttachDebugger | Debug | async | ✓ Implemented |
| 17 | GetExecutionTrace | Debug | async | ✓ Implemented |
| 18 | WriteDebugLog | Debug | async | ✓ Implemented |
| 19 | QueryAllTasks | Introspect | async | ✓ Implemented |
| 20 | QueryCapabilityInfo | Introspect | async | ✓ Implemented |
| 21 | QueryChannelStats | Introspect | async | ✓ Implemented |
| 22 | QueryRuntimeMetrics | Introspect | async | ✓ Implemented |

---

## 8. Developer Documentation

### 8.1 Quick Start Example

```typescript
import {
  bridge,
  spawnCognitiveTask,
  allocateMemory,
  createChannel,
  ProtectionFlags,
  MessageType,
  DebugLevel,
} from "@xkernal/sdk";

async function main() {
  // Initialize CSCI runtime
  await bridge.initialize({
    debugLevel: DebugLevel.Info,
    runtimeMode: "development",
    maxTasks: 100,
    maxMemoryBytes: 1_000_000_000,
    enableTelemetry: true,
  });

  try {
    // Spawn a cognitive task
    const taskHandle = await spawnCognitiveTask({
      taskId: "classifier-main",
      cognitiveEndpoint: "xk://agents/classifier-v2",
      memoryLayout: {
        totalBytes: 50_000_000,
        heapBytes: 30_000_000,
        stackBytes: 5_000_000,
        sharedRegions: [
          {
            name: "shared-data",
            offsetBytes: 40_000_000,
            sizeBytes: 10_000_000,
            permissions: ["read", "write"],
          },
        ],
        protectionFlags: ProtectionFlags.ReadWrite,
      },
      channels: [],
      timeoutMs: 60_000,
      priority: 200,
    });

    console.log(`Spawned task: ${taskHandle.taskId}`);

    // Allocate additional memory
    const memorySlot = await allocateMemory(
      taskHandle.taskId,
      5_000_000,
      ProtectionFlags.ReadWrite,
    );

    console.log(`Allocated ${memorySlot.allocatedBytes} bytes`);
  } finally {
    // Cleanup
    await bridge.shutdown();
  }
}

main().catch(console.error);
```

---

## 9. Implementation Status

**Phase 2 Week 19 Deliverables: COMPLETE**

- ✓ All 22 CSCI v1.0 syscall wrappers
- ✓ Complete type system (AgentSpec, MemoryLayout, ChannelConfig, CapabilityToken)
- ✓ Error class hierarchy (8 subclasses + base CognitiveError)
- ✓ N-API FFI bridge with Promise-based callbacks
- ✓ JSDoc comments for all public APIs
- ✓ Parameter validation per CSCI v1.0
- ✓ Unit test suite (20+ test cases)
- ✓ Quick-start documentation and examples

**Next Steps (Week 20+):**
- Advanced channel patterns (request-reply, publish-subscribe)
- Distributed tracing and observability
- Performance profiling extensions
- Security audit and capability-based access control validation
