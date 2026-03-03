/**
 * Cognitive Substrate SDK - Task Family Syscalls
 * 
 * Syscalls for task lifecycle management:
 * - ct_spawn (0x0000): Create a new task
 * - ct_yield (0x0001): Voluntarily yield task execution
 * - ct_checkpoint (0x0002): Create a state checkpoint
 * - ct_resume (0x0003): Resume task from checkpoint
 * 
 * Total: 4 syscalls
 */

import {
  CognitiveTaskId,
  CheckpointId,
  AgentId,
  TaskConfig,
  ResourceBudget,
  CheckpointType,
  CheckpointConfig,
  YieldHint,
  CsciError,
  CsciErrorCode,
} from '../index.js';

/**
 * Spawn a new cognitive task (ct_spawn).
 * 
 * Creates a new cognitive task with the specified configuration, capabilities, and budget.
 * The task is created in the Spawn phase and is ready to begin execution.
 * 
 * Syscall number: 0x0000
 * 
 * @param parent_agent - Agent creating this task
 * @param config - Task configuration (name, timeout, priority)
 * @param capabilities - Capability set for the task (array of capability names)
 * @param budget - Resource budget constraints (memory, CPU, max children)
 * @returns Promise resolving to the new task ID
 * @throws {CsciError} with code EPERM if caller lacks capability
 * @throws {CsciError} with code ENOMEM if memory allocation fails
 * @throws {CsciError} with code EBUDGET if budget would be exceeded
 * 
 * @example
 * ```typescript
 * const taskId = await ct_spawn(
 *   createAgentId('agent-1'),
 *   { name: 'compute-task', timeout_ms: 30000, priority: 100 },
 *   ['memory', 'ipc'],
 *   { memory_bytes: 10 * 1024 * 1024, cpu_ms: 60000 }
 * );
 * ```
 */
export async function ct_spawn(
  parent_agent: AgentId,
  config: TaskConfig,
  capabilities: string[],
  budget: ResourceBudget,
): Promise<CognitiveTaskId> {
  throw new CsciError(
    CsciErrorCode.Unimplemented,
    'ct_spawn is not yet implemented',
  );
}

/**
 * Voluntarily yield task execution (ct_yield).
 * 
 * Allows a task to voluntarily suspend execution and return control to the scheduler.
 * The task provides a hint about why it's yielding to help the scheduler make
 * scheduling decisions.
 * 
 * Syscall number: 0x0001
 * 
 * @param ct_id - Task ID to yield
 * @param hint - Hint about why task is yielding (Thinking, WaitingForInput, ResourceLimited)
 * @param timeout_ms - Optional timeout in milliseconds. If provided, the task will be automatically resumed after this time.
 * @returns Promise resolving when yield completes
 * @throws {CsciError} with code EPERM if caller is not the task itself
 * @throws {CsciError} with code ETIMEDOUT if timeout expires
 * 
 * @example
 * ```typescript
 * import { YieldHint } from '@cognitive-substrate/sdk';
 * 
 * await ct_yield(taskId, YieldHint.Thinking, 1000);
 * ```
 */
export async function ct_yield(
  ct_id: CognitiveTaskId,
  hint: YieldHint,
  timeout_ms?: number,
): Promise<void> {
  throw new CsciError(
    CsciErrorCode.Unimplemented,
    'ct_yield is not yet implemented',
  );
}

/**
 * Create a state checkpoint (ct_checkpoint).
 * 
 * Creates a checkpoint of the current task state for later resumption.
 * The checkpoint captures the complete task context and state, enabling
 * resumption at a later time or on a different system.
 * 
 * Syscall number: 0x0002
 * 
 * @param ct_id - Task ID to checkpoint
 * @param checkpoint_config - Checkpoint configuration (type: Full or Incremental, label)
 * @returns Promise resolving to the checkpoint ID
 * @throws {CsciError} with code EPERM if caller lacks capability
 * @throws {CsciError} with code ENOMEM if memory allocation fails
 * @throws {CsciError} with code EBUDGET if checkpoint would exceed budget
 * 
 * @example
 * ```typescript
 * import { CheckpointType } from '@cognitive-substrate/sdk';
 * 
 * const checkpointId = await ct_checkpoint(taskId, {
 *   type: CheckpointType.Full,
 *   label: 'before-decision-point'
 * });
 * ```
 */
export async function ct_checkpoint(
  ct_id: CognitiveTaskId,
  checkpoint_config: CheckpointConfig,
): Promise<CheckpointId> {
  throw new CsciError(
    CsciErrorCode.Unimplemented,
    'ct_checkpoint is not yet implemented',
  );
}

/**
 * Resume task from checkpoint (ct_resume).
 * 
 * Resumes a task's execution from a previously created checkpoint.
 * The task is restored to the exact state captured in the checkpoint,
 * including memory, task state, and execution context.
 * 
 * Syscall number: 0x0003
 * 
 * @param checkpoint_id - Checkpoint ID to resume from
 * @returns Promise resolving to the resumed task ID
 * @throws {CsciError} with code EPERM if caller lacks capability
 * @throws {CsciError} with code ENOENT if checkpoint does not exist
 * 
 * @example
 * ```typescript
 * const resumedTaskId = await ct_resume(checkpointId);
 * ```
 */
export async function ct_resume(
  checkpoint_id: CheckpointId,
): Promise<CognitiveTaskId> {
  throw new CsciError(
    CsciErrorCode.Unimplemented,
    'ct_resume is not yet implemented',
  );
}
