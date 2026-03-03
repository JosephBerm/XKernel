// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.

#nullable enable

namespace CognitiveSubstrate.SDK.Syscalls;

using System;
using System.Collections.Generic;
using System.Threading.Tasks;
using Types;

/// <summary>
/// Telemetry family syscalls for system monitoring and metrics.
/// 
/// Syscalls:
/// - TelemetryTraceAsync (0x0800): Emit a telemetry event
/// - TelemetrySnapshotAsync (0x0801): Capture system snapshot
/// 
/// Total: 2 syscalls
/// </summary>
public static class TelemetrySyscalls
{
    /// <summary>
    /// Emit a telemetry trace event (telemetry_trace).
    /// Syscall number: 0x0800
    /// </summary>
    public static Task TelemetryTraceAsync(
        string eventName,
        Dictionary<string, object> data)
    {
        throw new CsciException(
            CsciErrorCode.Unimplemented,
            "TelemetryTraceAsync is not yet implemented");
    }

    /// <summary>
    /// Capture a system snapshot (telemetry_snapshot).
    /// Syscall number: 0x0801
    /// </summary>
    public static Task<SnapshotData> TelemetrySnapshotAsync(
        SnapshotConfig config)
    {
        throw new CsciException(
            CsciErrorCode.Unimplemented,
            "TelemetrySnapshotAsync is not yet implemented");
    }
}
