/**
 * Cognitive Substrate SDK - Telemetry Family Syscalls
 * 
 * Syscalls for system monitoring and metrics:
 * - telemetry_trace (0x0800): Emit a telemetry event
 * - telemetry_snapshot (0x0801): Capture system snapshot
 * 
 * Total: 2 syscalls
 */

import {
  TraceEventData,
  SnapshotConfig,
  SnapshotData,
  CsciError,
  CsciErrorCode,
} from '../index.js';

/**
 * Emit a telemetry trace event (telemetry_trace).
 * 
 * Emits a telemetry event to the tracing system. Events are buffered
 * and can be retrieved through the metrics/tracing infrastructure.
 * 
 * Syscall number: 0x0800
 * 
 * @param event_name - Name of the telemetry event
 * @param data - Event data (arbitrary key-value pairs)
 * @returns Promise resolving when event is recorded
 * @throws {CsciError} with code EPERM if caller lacks telemetry capability
 * @throws {CsciError} with code EBUFFER if trace buffer is full
 * 
 * @example
 * ```typescript
 * await telemetry_trace('task-checkpoint', {
 *   task_id: taskId,
 *   checkpoint_type: 'full',
 *   size_bytes: 1024 * 1024,
 *   duration_ms: 250
 * });
 * ```
 */
export async function telemetry_trace(
  event_name: string,
  data: Record<string, unknown>,
): Promise<void> {
  throw new CsciError(
    CsciErrorCode.Unimplemented,
    'telemetry_trace is not yet implemented',
  );
}

/**
 * Capture a system snapshot (telemetry_snapshot).
 * 
 * Captures a snapshot of system state including task metrics, memory
 * metrics, and channel metrics. The snapshot includes counters, gauges,
 * and other measurement data.
 * 
 * Syscall number: 0x0801
 * 
 * @param config - Snapshot configuration (which metrics to include)
 * @returns Promise resolving to the snapshot data
 * @throws {CsciError} with code EPERM if caller lacks telemetry capability
 * @throws {CsciError} with code EBUFFER if snapshot buffer is insufficient
 * 
 * @example
 * ```typescript
 * const snapshot = await telemetry_snapshot({
 *   include_tasks: true,
 *   include_memory: true,
 *   include_channels: true
 * });
 * 
 * console.log('Task metrics:', snapshot.task_metrics);
 * console.log('Memory metrics:', snapshot.memory_metrics);
 * ```
 */
export async function telemetry_snapshot(
  config: SnapshotConfig,
): Promise<SnapshotData> {
  throw new CsciError(
    CsciErrorCode.Unimplemented,
    'telemetry_snapshot is not yet implemented',
  );
}
