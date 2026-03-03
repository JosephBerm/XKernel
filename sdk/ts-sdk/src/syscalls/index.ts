/**
 * Cognitive Substrate SDK - Syscalls Barrel Export
 * 
 * Re-exports all 22 CSCI v0.1 syscalls from their respective family modules.
 */

// Task family (4 syscalls)
export {
  ct_spawn,
  ct_yield,
  ct_checkpoint,
  ct_resume,
} from './task.js';

// Memory family (4 syscalls)
export {
  mem_alloc,
  mem_free,
  mem_mount,
  mem_unmount,
} from './memory.js';

// Tool family (2 syscalls)
export {
  tool_invoke,
  tool_bind,
} from './tool.js';

// Channel/IPC family (3 syscalls)
export {
  ch_create,
  ch_send,
  ch_receive,
} from './ipc.js';

// Capability/Security family (3 syscalls)
export {
  cap_delegate,
  cap_grant,
  cap_revoke,
} from './security.js';

// Signals family (2 syscalls)
export {
  sig_send,
  sig_handler_install,
} from './signals.js';

// Crew family (4 syscalls)
export {
  crew_init,
  crew_add,
  crew_remove,
  crew_barrier,
} from './crew.js';

// Telemetry family (2 syscalls)
export {
  telemetry_trace,
  telemetry_snapshot,
} from './telemetry.js';
