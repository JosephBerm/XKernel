// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.

#nullable enable

namespace CognitiveSubstrate.SDK.Syscalls;

using System;
using System.Collections.Generic;
using System.Threading.Tasks;
using Types;

/// <summary>
/// Task family syscalls for task lifecycle management.
/// 
/// Syscalls:
/// - CtSpawnAsync (0x0000): Create a new task
/// - CtYieldAsync (0x0001): Voluntarily yield task execution
/// - CtCheckpointAsync (0x0002): Create a state checkpoint
/// - CtResumeAsync (0x0003): Resume task from checkpoint
/// 
/// Total: 4 syscalls
/// </summary>
public static class TaskSyscalls
{
    /// <summary>
    /// Spawn a new cognitive task (ct_spawn).
    /// 
    /// Creates a new cognitive task with the specified configuration, capabilities, and budget.
    /// The task is created in the Spawn phase and is ready to begin execution.
    /// 
    /// Syscall number: 0x0000
    /// </summary>
    /// <param name="parentAgent">Agent creating this task.</param>
    /// <param name="config">Task configuration (name, timeout, priority).</param>
    /// <param name="capabilities">Capability set for the task.</param>
    /// <param name="budget">Resource budget constraints.</param>
    /// <returns>Task representing the spawned task, resolving to the new task ID.</returns>
    /// <exception cref="CsciException">If task creation fails.</exception>
    public static Task<CognitiveTaskId> CtSpawnAsync(
        AgentId parentAgent,
        TaskConfig config,
        IEnumerable<string> capabilities,
        ResourceBudget budget)
    {
        throw new CsciException(
            CsciErrorCode.Unimplemented,
            "CtSpawnAsync is not yet implemented");
    }

    /// <summary>
    /// Voluntarily yield task execution (ct_yield).
    /// 
    /// Allows a task to voluntarily suspend execution and return control to the scheduler.
    /// The task provides a hint about why it's yielding to help the scheduler make
    /// scheduling decisions.
    /// 
    /// Syscall number: 0x0001
    /// </summary>
    /// <param name="ctId">Task ID to yield.</param>
    /// <param name="hint">Hint about why yielding.</param>
    /// <param name="timeoutMs">Optional timeout in milliseconds.</param>
    /// <returns>Task representing the yield operation.</returns>
    /// <exception cref="CsciException">If yield operation fails.</exception>
    public static Task CtYieldAsync(
        CognitiveTaskId ctId,
        YieldHint hint,
        int? timeoutMs = null)
    {
        throw new CsciException(
            CsciErrorCode.Unimplemented,
            "CtYieldAsync is not yet implemented");
    }

    /// <summary>
    /// Create a state checkpoint (ct_checkpoint).
    /// 
    /// Creates a checkpoint of the current task state for later resumption.
    /// The checkpoint captures the complete task context and state.
    /// 
    /// Syscall number: 0x0002
    /// </summary>
    /// <param name="ctId">Task ID to checkpoint.</param>
    /// <param name="checkpointConfig">Checkpoint configuration.</param>
    /// <returns>Task representing the checkpoint operation, resolving to the checkpoint ID.</returns>
    /// <exception cref="CsciException">If checkpoint creation fails.</exception>
    public static Task<CheckpointId> CtCheckpointAsync(
        CognitiveTaskId ctId,
        CheckpointConfig checkpointConfig)
    {
        throw new CsciException(
            CsciErrorCode.Unimplemented,
            "CtCheckpointAsync is not yet implemented");
    }

    /// <summary>
    /// Resume task from checkpoint (ct_resume).
    /// 
    /// Resumes a task's execution from a previously created checkpoint.
    /// The task is restored to the exact state captured in the checkpoint.
    /// 
    /// Syscall number: 0x0003
    /// </summary>
    /// <param name="checkpointId">Checkpoint ID to resume from.</param>
    /// <returns>Task representing the resume operation, resolving to the resumed task ID.</returns>
    /// <exception cref="CsciException">If resume operation fails.</exception>
    public static Task<CognitiveTaskId> CtResumeAsync(
        CheckpointId checkpointId)
    {
        throw new CsciException(
            CsciErrorCode.Unimplemented,
            "CtResumeAsync is not yet implemented");
    }
}
