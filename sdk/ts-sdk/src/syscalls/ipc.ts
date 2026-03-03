/**
 * Cognitive Substrate SDK - Channel (IPC) Family Syscalls
 * 
 * Syscalls for inter-process communication:
 * - ch_create (0x0300): Create a communication channel
 * - ch_send (0x0301): Send a message on a channel
 * - ch_receive (0x0302): Receive a message from a channel
 * 
 * Total: 3 syscalls
 */

import {
  ChannelId,
  ChannelConfig,
  MessagePayload,
  SendFlags,
  CsciError,
  CsciErrorCode,
} from '../index.js';

/**
 * Create a communication channel (ch_create).
 * 
 * Creates a new bidirectional communication channel for exchanging messages
 * between agents or tasks. The channel is initially empty with no messages.
 * 
 * Syscall number: 0x0300
 * 
 * @param config - Channel configuration (max message size, buffer size, protocol)
 * @returns Promise resolving to the new channel ID
 * @throws {CsciError} with code EPERM if caller lacks capability
 * @throws {CsciError} with code ENOMEM if memory allocation fails
 * @throws {CsciError} with code EINVAL if configuration is invalid
 * 
 * @example
 * ```typescript
 * const channelId = await ch_create({
 *   max_message_size: 65536,
 *   buffer_size: 1024 * 1024,
 *   protocol: ChannelProtocol.MessageBased
 * });
 * ```
 */
export async function ch_create(
  config: ChannelConfig,
): Promise<ChannelId> {
  throw new CsciError(
    CsciErrorCode.Unimplemented,
    'ch_create is not yet implemented',
  );
}

/**
 * Send a message on a channel (ch_send).
 * 
 * Sends a message through the specified channel to waiting receivers.
 * The message is copied into the channel buffer. If the buffer is full,
 * the send operation may block or fail depending on flags.
 * 
 * Syscall number: 0x0301
 * 
 * @param channel_id - Channel ID to send on
 * @param message - Message payload to send
 * @param flags - Send flags (default: wait for capacity)
 * @param timeout_ms - Optional timeout in milliseconds
 * @returns Promise resolving to the number of bytes sent
 * @throws {CsciError} with code EPERM if caller lacks capability
 * @throws {CsciError} with code ECLOSED if channel is closed
 * @throws {CsciError} with code EMSGSIZE if message exceeds maximum size
 * @throws {CsciError} with code ETIMEDOUT if operation times out
 * 
 * @example
 * ```typescript
 * const bytesSent = await ch_send(
 *   channelId,
 *   { type: 'request', data: { query: 'hello' } },
 *   SendFlags.Default,
 *   5000
 * );
 * ```
 */
export async function ch_send(
  channel_id: ChannelId,
  message: MessagePayload,
  flags?: SendFlags,
  timeout_ms?: number,
): Promise<number> {
  throw new CsciError(
    CsciErrorCode.Unimplemented,
    'ch_send is not yet implemented',
  );
}

/**
 * Receive a message from a channel (ch_receive).
 * 
 * Receives the next available message from the specified channel.
 * If no messages are available, the operation may block or fail depending
 * on the timeout parameter.
 * 
 * Syscall number: 0x0302
 * 
 * @param channel_id - Channel ID to receive from
 * @param timeout_ms - Optional timeout in milliseconds. If 0, non-blocking.
 * @returns Promise resolving to the received message payload and number of bytes
 * @throws {CsciError} with code EPERM if caller lacks capability
 * @throws {CsciError} with code ECLOSED if channel is closed
 * @throws {CsciError} with code ENOMSG if no message available (non-blocking mode)
 * @throws {CsciError} with code ETIMEDOUT if operation times out
 * 
 * @example
 * ```typescript
 * const message = await ch_receive(channelId, 5000);
 * console.log('Received:', message);
 * ```
 */
export async function ch_receive(
  channel_id: ChannelId,
  timeout_ms?: number,
): Promise<{ message: MessagePayload; bytes: number }> {
  throw new CsciError(
    CsciErrorCode.Unimplemented,
    'ch_receive is not yet implemented',
  );
}
