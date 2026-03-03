/**
 * @cognitive-substrate/sdk
 * 
 * TypeScript SDK for Cognitive Substrate CSCI v0.1
 * 
 * This SDK provides type-safe async/await interfaces for all 22 CSCI v0.1 syscalls
 * organized into 8 syscall families:
 * 
 * - Task (4): ct_spawn, ct_yield, ct_checkpoint, ct_resume
 * - Memory (4): mem_alloc, mem_free, mem_mount, mem_unmount
 * - Tool (2): tool_invoke, tool_bind
 * - Channel (3): ch_create, ch_send, ch_receive
 * - Capability (3): cap_delegate, cap_grant, cap_revoke
 * - Signals (2): sig_send, sig_handler_install
 * - Crew (4): crew_init, crew_add, crew_remove, crew_barrier
 * - Telemetry (2): telemetry_trace, telemetry_snapshot
 * 
 * @example
 * ```typescript
 * import { ct_spawn, createAgentId, CsciError } from '@cognitive-substrate/sdk';
 * 
 * const taskId = await ct_spawn(
 *   createAgentId('agent-1'),
 *   { name: 'my-task', timeout_ms: 5000 },
 *   ['memory', 'ipc'],
 *   { memory_bytes: 1024 * 1024 }
 * );
 * ```
 * 
 * @license Apache-2.0
 */

// Re-export all error types
export * from './errors.js';

// Re-export all types
export * from './types.js';

// Re-export all syscall families
export * as TaskSyscalls from './syscalls/task.js';
export * as MemorySyscalls from './syscalls/memory.js';
export * as ToolSyscalls from './syscalls/tool.js';
export * as ChannelSyscalls from './syscalls/ipc.js';
export * as CapabilitySyscalls from './syscalls/security.js';
export * as SignalSyscalls from './syscalls/signals.js';
export * as CrewSyscalls from './syscalls/crew.js';
export * as TelemetrySyscalls from './syscalls/telemetry.js';

// Convenience re-exports of all syscalls

// Task family (4 syscalls)
export {
  ct_spawn,
  ct_yield,
  ct_checkpoint,
  ct_resume,
} from './syscalls/task.js';

// Memory family (4 syscalls)
export {
  mem_alloc,
  mem_free,
  mem_mount,
  mem_unmount,
} from './syscalls/memory.js';

// Tool family (2 syscalls)
export {
  tool_invoke,
  tool_bind,
} from './syscalls/tool.js';

// Channel family (3 syscalls)
export {
  ch_create,
  ch_send,
  ch_receive,
} from './syscalls/ipc.js';

// Capability family (3 syscalls)
export {
  cap_delegate,
  cap_grant,
  cap_revoke,
} from './syscalls/security.js';

// Signals family (2 syscalls)
export {
  sig_send,
  sig_handler_install,
} from './syscalls/signals.js';

// Crew family (4 syscalls)
export {
  crew_init,
  crew_add,
  crew_remove,
  crew_barrier,
} from './syscalls/crew.js';

// Telemetry family (2 syscalls)
export {
  telemetry_trace,
  telemetry_snapshot,
} from './syscalls/telemetry.js';
