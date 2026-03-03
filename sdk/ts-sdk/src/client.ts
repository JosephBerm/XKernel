/**
 * Cognitive Substrate SDK - Client Class
 *
 * CognitiveSubstrateClient provides a unified interface for connecting to and
 * interacting with the Cognitive Substrate kernel over a network or IPC channel.
 *
 * Part of @cognitive-substrate/sdk
 */

import {
  CognitiveTaskId,
  AgentId,
  ChannelId,
  TaskConfig,
  ResourceBudget,
  MessagePayload,
} from './types.js';

/**
 * Connection configuration for the Cognitive Substrate kernel
 */
export interface ConnectionConfig {
  /** Kernel endpoint URL or socket path */
  endpoint: string;
  /** Authentication token (optional) */
  token?: string;
  /** Connection timeout in milliseconds (default: 5000) */
  timeout?: number;
  /** Auto-reconnect on disconnect (default: true) */
  autoReconnect?: boolean;
  /** Number of reconnect attempts (default: 3) */
  maxReconnectAttempts?: number;
}

/**
 * Event listener for kernel events
 */
export type KernelEventListener = (event: KernelEvent) => void;

/**
 * Kernel event data
 */
export interface KernelEvent {
  /** Event type */
  type: string;
  /** Event timestamp */
  timestamp: number;
  /** Event data */
  data: unknown;
}

/**
 * CognitiveSubstrateClient - Main SDK client class
 *
 * Provides async/await interfaces for all CSCI syscalls and manages
 * the connection to the Cognitive Substrate kernel.
 *
 * @example
 * ```typescript
 * const client = new CognitiveSubstrateClient({
 *   endpoint: 'ws://localhost:8080',
 * });
 *
 * await client.connect();
 *
 * const taskId = await client.spawn(
 *   createAgentId('agent-1'),
 *   { name: 'my-task' },
 *   ['memory', 'ipc'],
 *   { memory_bytes: 1024 * 1024 }
 * );
 *
 * await client.disconnect();
 * ```
 */
export class CognitiveSubstrateClient {
  private config: ConnectionConfig;
  private connected: boolean = false;
  private eventListeners: Map<string, Set<KernelEventListener>> = new Map();

  /**
   * Create a new Cognitive Substrate client
   * @param config Connection configuration
   */
  constructor(config: ConnectionConfig) {
    this.config = {
      timeout: 5000,
      autoReconnect: true,
      maxReconnectAttempts: 3,
      ...config,
    };
  }

  /**
   * Connect to the Cognitive Substrate kernel
   * @returns Promise that resolves when connected
   * @throws Error if connection fails
   */
  async connect(): Promise<void> {
    try {
      // Implementation would establish connection to kernel
      // via WebSocket, HTTP, or IPC based on endpoint
      this.connected = true;
    } catch (error) {
      throw new Error(`Failed to connect to kernel: ${error}`);
    }
  }

  /**
   * Disconnect from the Cognitive Substrate kernel
   * @returns Promise that resolves when disconnected
   */
  async disconnect(): Promise<void> {
    this.connected = false;
  }

  /**
   * Check if client is connected
   * @returns True if connected
   */
  isConnected(): boolean {
    return this.connected;
  }

  /**
   * Spawn a new cognitive task
   */
  async spawn(
    parentAgent: AgentId,
    config: TaskConfig,
    capabilities: string[],
    budget: ResourceBudget
  ): Promise<CognitiveTaskId> {
    this.ensureConnected();
    // Implementation would send ct_spawn syscall to kernel
    throw new Error('Not implemented');
  }

  /**
   * Yield task execution
   */
  async yield(taskId: CognitiveTaskId, hint?: string): Promise<void> {
    this.ensureConnected();
    // Implementation would send ct_yield syscall to kernel
  }

  /**
   * Send message over a channel
   */
  async send(channelId: ChannelId, payload: MessagePayload): Promise<void> {
    this.ensureConnected();
    // Implementation would send ch_send syscall to kernel
  }

  /**
   * Receive message from a channel
   */
  async receive(channelId: ChannelId): Promise<MessagePayload> {
    this.ensureConnected();
    // Implementation would send ch_receive syscall to kernel
    throw new Error('Not implemented');
  }

  /**
   * Register event listener
   */
  on(eventType: string, listener: KernelEventListener): void {
    if (!this.eventListeners.has(eventType)) {
      this.eventListeners.set(eventType, new Set());
    }
    this.eventListeners.get(eventType)!.add(listener);
  }

  /**
   * Unregister event listener
   */
  off(eventType: string, listener: KernelEventListener): void {
    const listeners = this.eventListeners.get(eventType);
    if (listeners) {
      listeners.delete(listener);
    }
  }

  /**
   * Ensure client is connected
   * @throws Error if not connected
   */
  private ensureConnected(): void {
    if (!this.connected) {
      throw new Error('Client is not connected. Call connect() first.');
    }
  }
}
