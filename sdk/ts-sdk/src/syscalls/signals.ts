/**
 * Cognitive Substrate SDK - Signals Family Syscalls
 * 
 * Syscalls for signal handling:
 * - sig_send (0x0600): Send a signal to an agent/task
 * - sig_handler_install (0x0601): Install a signal handler
 * 
 * Total: 2 syscalls
 */

import {
  AgentId,
  SignalHandlerId,
  CsciError,
  CsciErrorCode,
} from '../index.js';

/**
 * Send a signal to an agent/task (sig_send).
 * 
 * Sends a signal with optional data to a specified agent or task.
 * The receiving agent/task's signal handlers will be invoked to process
 * the signal.
 * 
 * Syscall number: 0x0600
 * 
 * @param recipient_id - Agent/task ID to send signal to
 * @param signal_number - Signal number to send
 * @param data - Optional signal data (optional)
 * @returns Promise resolving when signal is sent
 * @throws {CsciError} with code EPERM if caller lacks capability
 * @throws {CsciError} with code ENOENT if recipient does not exist
 * 
 * @example
 * ```typescript
 * await sig_send(
 *   createAgentId('agent-1'),
 *   SIGTERM,
 *   { reason: 'timeout' }
 * );
 * ```
 */
export async function sig_send(
  recipient_id: AgentId,
  signal_number: number,
  data?: unknown,
): Promise<void> {
  throw new CsciError(
    CsciErrorCode.Unimplemented,
    'sig_send is not yet implemented',
  );
}

/**
 * Install a signal handler (sig_handler_install).
 * 
 * Registers a signal handler function to process signals of the specified
 * signal number. Only one handler can be registered per signal number;
 * installing a new handler replaces any previous handler.
 * 
 * Syscall number: 0x0601
 * 
 * @param signal_number - Signal number to handle
 * @param handler_fn - Handler function (async, receives signal number and data)
 * @param flags - Registration flags (optional)
 * @returns Promise resolving to the signal handler ID
 * @throws {CsciError} with code EPERM if caller lacks capability
 * @throws {CsciError} with code EINVAL if signal number is invalid
 * 
 * @example
 * ```typescript
 * const handlerId = await sig_handler_install(
 *   SIGTERM,
 *   async (signal, data) => {
 *     console.log(`Received signal ${signal}:`, data);
 *     // Perform cleanup
 *   }
 * );
 * ```
 */
export async function sig_handler_install(
  signal_number: number,
  handler_fn: (signal: number, data: unknown) => Promise<void>,
  flags?: number,
): Promise<SignalHandlerId> {
  throw new CsciError(
    CsciErrorCode.Unimplemented,
    'sig_handler_install is not yet implemented',
  );
}
