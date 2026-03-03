// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.

#nullable enable

namespace CognitiveSubstrate.SDK.Types;

using System;
using System.Collections.Generic;

// ============================================================================
// Branded Type Records
// ============================================================================

/// <summary>
/// Cognitive Task ID - globally unique identifier for a task.
/// </summary>
public readonly record struct CognitiveTaskId(string Value)
{
    /// <summary>
    /// Create a Cognitive Task ID from a string.
    /// </summary>
    public static CognitiveTaskId Create(string value) => new(value);

    /// <summary>
    /// Get the string representation.
    /// </summary>
    public override string ToString() => Value;
}

/// <summary>
/// Memory Region ID - identifier for allocated memory.
/// </summary>
public readonly record struct MemoryRegionId(string Value)
{
    /// <summary>
    /// Create a Memory Region ID from a string.
    /// </summary>
    public static MemoryRegionId Create(string value) => new(value);

    /// <summary>
    /// Get the string representation.
    /// </summary>
    public override string ToString() => Value;
}

/// <summary>
/// Channel ID - identifier for IPC channel.
/// </summary>
public readonly record struct ChannelId(string Value)
{
    /// <summary>
    /// Create a Channel ID from a string.
    /// </summary>
    public static ChannelId Create(string value) => new(value);

    /// <summary>
    /// Get the string representation.
    /// </summary>
    public override string ToString() => Value;
}

/// <summary>
/// Grant Handle - identifier for capability grant.
/// </summary>
public readonly record struct GrantHandle(string Value)
{
    /// <summary>
    /// Create a Grant Handle from a string.
    /// </summary>
    public static GrantHandle Create(string value) => new(value);

    /// <summary>
    /// Get the string representation.
    /// </summary>
    public override string ToString() => Value;
}

/// <summary>
/// Agent ID - identifier for an agent.
/// </summary>
public readonly record struct AgentId(string Value)
{
    /// <summary>
    /// Create an Agent ID from a string.
    /// </summary>
    public static AgentId Create(string value) => new(value);

    /// <summary>
    /// Get the string representation.
    /// </summary>
    public override string ToString() => Value;
}

/// <summary>
/// Checkpoint ID - identifier for task checkpoint.
/// </summary>
public readonly record struct CheckpointId(string Value)
{
    /// <summary>
    /// Create a Checkpoint ID from a string.
    /// </summary>
    public static CheckpointId Create(string value) => new(value);

    /// <summary>
    /// Get the string representation.
    /// </summary>
    public override string ToString() => Value;
}

/// <summary>
/// Crew ID - identifier for agent crew.
/// </summary>
public readonly record struct CrewId(string Value)
{
    /// <summary>
    /// Create a Crew ID from a string.
    /// </summary>
    public static CrewId Create(string value) => new(value);

    /// <summary>
    /// Get the string representation.
    /// </summary>
    public override string ToString() => Value;
}

/// <summary>
/// Signal Handler ID - identifier for registered signal handler.
/// </summary>
public readonly record struct SignalHandlerId(string Value)
{
    /// <summary>
    /// Create a Signal Handler ID from a string.
    /// </summary>
    public static SignalHandlerId Create(string value) => new(value);

    /// <summary>
    /// Get the string representation.
    /// </summary>
    public override string ToString() => Value;
}

// ============================================================================
// Configuration Records
// ============================================================================

/// <summary>
/// Task configuration for task creation.
/// </summary>
public record TaskConfig
{
    /// <summary>
    /// Task name.
    /// </summary>
    public required string Name { get; init; }

    /// <summary>
    /// Execution timeout in milliseconds.
    /// </summary>
    public int? TimeoutMs { get; init; }

    /// <summary>
    /// Task priority (0-255, higher = more important).
    /// </summary>
    public byte? Priority { get; init; }
}

/// <summary>
/// Resource budget constraints.
/// </summary>
public record ResourceBudget
{
    /// <summary>
    /// Memory quota in bytes.
    /// </summary>
    public ulong? MemoryBytes { get; init; }

    /// <summary>
    /// CPU quota in milliseconds.
    /// </summary>
    public int? CpuMs { get; init; }

    /// <summary>
    /// Maximum child tasks.
    /// </summary>
    public int? MaxChildren { get; init; }
}

/// <summary>
/// Memory allocation configuration.
/// </summary>
public record MemoryAllocConfig
{
    /// <summary>
    /// Size in bytes to allocate.
    /// </summary>
    public required ulong Size { get; init; }

    /// <summary>
    /// Alignment requirement in bytes.
    /// </summary>
    public ulong? Alignment { get; init; }

    /// <summary>
    /// Allocation flags.
    /// </summary>
    public uint? Flags { get; init; }
}

/// <summary>
/// Checkpoint creation configuration.
/// </summary>
public record CheckpointConfig
{
    /// <summary>
    /// Checkpoint type (Full or Incremental).
    /// </summary>
    public required CheckpointType Type { get; init; }

    /// <summary>
    /// Checkpoint label for identification.
    /// </summary>
    public required string Label { get; init; }
}

/// <summary>
/// Capability set for delegation/grant.
/// </summary>
public record CapabilitySet
{
    /// <summary>
    /// Capability names.
    /// </summary>
    public required IEnumerable<string> Capabilities { get; init; }
}

/// <summary>
/// Channel creation configuration.
/// </summary>
public record ChannelConfig
{
    /// <summary>
    /// Maximum message size in bytes.
    /// </summary>
    public ulong? MaxMessageSize { get; init; }

    /// <summary>
    /// Channel buffer size.
    /// </summary>
    public ulong? BufferSize { get; init; }

    /// <summary>
    /// Channel protocol type.
    /// </summary>
    public ChannelProtocol? Protocol { get; init; }
}

/// <summary>
/// Crew creation configuration.
/// </summary>
public record CrewConfig
{
    /// <summary>
    /// Mission description.
    /// </summary>
    public required string Mission { get; init; }

    /// <summary>
    /// Coordinator agent ID.
    /// </summary>
    public required AgentId CoordinatorAgent { get; init; }

    /// <summary>
    /// Initial crew members.
    /// </summary>
    public IEnumerable<AgentId>? InitialMembers { get; init; }

    /// <summary>
    /// Collective budget.
    /// </summary>
    public ulong? CollectiveBudget { get; init; }
}

/// <summary>
/// Sandbox configuration for tool execution.
/// </summary>
public record SandboxConfig
{
    /// <summary>
    /// Memory limit in bytes.
    /// </summary>
    public ulong? MemoryLimit { get; init; }

    /// <summary>
    /// Execution timeout in milliseconds.
    /// </summary>
    public int? TimeoutMs { get; init; }

    /// <summary>
    /// Allow network access.
    /// </summary>
    public bool? AllowNetwork { get; init; }

    /// <summary>
    /// Allowed file system paths.
    /// </summary>
    public IEnumerable<string>? AllowedPaths { get; init; }
}

/// <summary>
/// Telemetry snapshot configuration.
/// </summary>
public record SnapshotConfig
{
    /// <summary>
    /// Include task metrics.
    /// </summary>
    public bool? IncludeTasks { get; init; }

    /// <summary>
    /// Include memory metrics.
    /// </summary>
    public bool? IncludeMemory { get; init; }

    /// <summary>
    /// Include channel metrics.
    /// </summary>
    public bool? IncludeChannels { get; init; }
}

// ============================================================================
// Enumeration Types
// ============================================================================

/// <summary>
/// Checkpoint type for ct_checkpoint syscall.
/// </summary>
public enum CheckpointType
{
    /// <summary>
    /// Full checkpoint of complete task state.
    /// </summary>
    Full = 0,

    /// <summary>
    /// Incremental checkpoint (changes since last checkpoint).
    /// </summary>
    Incremental = 1,
}

/// <summary>
/// Yield hint for ct_yield syscall.
/// </summary>
public enum YieldHint
{
    /// <summary>
    /// Task is thinking/processing.
    /// </summary>
    Thinking = 0,

    /// <summary>
    /// Task is waiting for input/response.
    /// </summary>
    WaitingForInput = 1,

    /// <summary>
    /// Task is resource-limited.
    /// </summary>
    ResourceLimited = 2,
}

/// <summary>
/// Channel protocol type for ch_create syscall.
/// </summary>
public enum ChannelProtocol
{
    /// <summary>
    /// Unstructured byte stream.
    /// </summary>
    ByteStream = 0,

    /// <summary>
    /// Structured message-based protocol.
    /// </summary>
    MessageBased = 1,
}

/// <summary>
/// Channel send flags for ch_send syscall.
/// </summary>
[Flags]
public enum SendFlags : uint
{
    /// <summary>
    /// Default send behavior.
    /// </summary>
    Default = 0,

    /// <summary>
    /// Don't wait for receiver (fire and forget).
    /// </summary>
    DontWait = 1,
}

// ============================================================================
// Result Types
// ============================================================================

/// <summary>
/// Tool invocation result.
/// </summary>
public record ToolResult
{
    /// <summary>
    /// Success indicator.
    /// </summary>
    public required bool Success { get; init; }

    /// <summary>
    /// Result data if successful.
    /// </summary>
    public object? Data { get; init; }

    /// <summary>
    /// Error message if failed.
    /// </summary>
    public string? Error { get; init; }
}

/// <summary>
/// Message payload for channels.
/// </summary>
public record MessagePayload
{
    /// <summary>
    /// Message type identifier.
    /// </summary>
    public required string Type { get; init; }

    /// <summary>
    /// Message data.
    /// </summary>
    public required object Data { get; init; }

    /// <summary>
    /// Optional metadata.
    /// </summary>
    public Dictionary<string, object>? Metadata { get; init; }
}

/// <summary>
/// Snapshot data from telemetry_snapshot syscall.
/// </summary>
public record SnapshotData
{
    /// <summary>
    /// Timestamp of snapshot.
    /// </summary>
    public required long Timestamp { get; init; }

    /// <summary>
    /// Task metrics if requested.
    /// </summary>
    public Dictionary<string, object>? TaskMetrics { get; init; }

    /// <summary>
    /// Memory metrics if requested.
    /// </summary>
    public Dictionary<string, object>? MemoryMetrics { get; init; }

    /// <summary>
    /// Channel metrics if requested.
    /// </summary>
    public Dictionary<string, object>? ChannelMetrics { get; init; }
}
