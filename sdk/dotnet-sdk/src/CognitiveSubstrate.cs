// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.

#nullable enable

namespace CognitiveSubstrate.SDK;

using System;

/// <summary>
/// Cognitive Substrate SDK - Main Entry Point
/// 
/// This SDK provides type-safe async/await interfaces for all 22 CSCI v0.1 syscalls
/// organized into 8 syscall families:
/// 
/// - Task (4): CtSpawnAsync, CtYieldAsync, CtCheckpointAsync, CtResumeAsync
/// - Memory (4): MemAllocAsync, MemFreeAsync, MemMountAsync, MemUnmountAsync
/// - Tool (2): ToolInvokeAsync, ToolBindAsync
/// - Channel (3): ChCreateAsync, ChSendAsync, ChReceiveAsync
/// - Capability (3): CapDelegateAsync, CapGrantAsync, CapRevokeAsync
/// - Signals (2): SigSendAsync, SigHandlerInstallAsync
/// - Crew (4): CrewInitAsync, CrewAddAsync, CrewRemoveAsync, CrewBarrierAsync
/// - Telemetry (2): TelemetryTraceAsync, TelemetrySnapshotAsync
/// 
/// Total: 22 syscalls across 8 families
/// 
/// Usage:
/// <code>
/// using CognitiveSubstrate.SDK;
/// using CognitiveSubstrate.SDK.Syscalls;
/// using CognitiveSubstrate.SDK.Types;
/// 
/// var taskId = await TaskSyscalls.CtSpawnAsync(
///     AgentId.Create("agent-1"),
///     new TaskConfig { Name = "my-task", TimeoutMs = 5000 },
///     new[] { "memory", "ipc" },
///     new ResourceBudget { MemoryBytes = 1024 * 1024 }
/// );
/// </code>
/// </summary>
public static class CognitiveSubstrateSDK
{
    /// <summary>
    /// SDK Version matching CSCI v0.1.0
    /// </summary>
    public const string Version = "0.1.0";

    /// <summary>
    /// CSCI Specification Version
    /// </summary>
    public const string CsciVersion = "0.1.0";

    /// <summary>
    /// Get the SDK version information.
    /// </summary>
    public static string GetVersionInfo()
        => $"Cognitive Substrate SDK v{Version} (CSCI v{CsciVersion})";

    /// <summary>
    /// Total number of syscalls implemented.
    /// </summary>
    public const int TotalSyscalls = 22;

    /// <summary>
    /// Number of syscall families.
    /// </summary>
    public const int FamilyCount = 8;
}
