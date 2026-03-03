// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.

#nullable enable

namespace CognitiveSubstrate.SDK.Syscalls;

using System;
using System.Collections.Generic;
using System.Threading.Tasks;
using Types;

/// <summary>
/// Crew family syscalls for multi-agent coordination.
/// 
/// Syscalls:
/// - CrewInitAsync (0x0700): Create a new crew
/// - CrewAddAsync (0x0701): Add an agent to a crew
/// - CrewRemoveAsync (0x0702): Remove an agent from a crew
/// - CrewBarrierAsync (0x0703): Synchronize crew members at a barrier
/// 
/// Total: 4 syscalls
/// </summary>
public static class CrewSyscalls
{
    /// <summary>
    /// Create a new crew (crew_init).
    /// Syscall number: 0x0700
    /// </summary>
    public static Task<CrewId> CrewInitAsync(
        string name,
        CrewConfig config)
    {
        throw new CsciException(
            CsciErrorCode.Unimplemented,
            "CrewInitAsync is not yet implemented");
    }

    /// <summary>
    /// Add an agent to a crew (crew_add).
    /// Syscall number: 0x0701
    /// </summary>
    public static Task CrewAddAsync(
        CrewId crewId,
        AgentId agentId,
        Dictionary<string, object>? config = null)
    {
        throw new CsciException(
            CsciErrorCode.Unimplemented,
            "CrewAddAsync is not yet implemented");
    }

    /// <summary>
    /// Remove an agent from a crew (crew_remove).
    /// Syscall number: 0x0702
    /// </summary>
    public static Task CrewRemoveAsync(
        CrewId crewId,
        AgentId agentId)
    {
        throw new CsciException(
            CsciErrorCode.Unimplemented,
            "CrewRemoveAsync is not yet implemented");
    }

    /// <summary>
    /// Synchronize crew members at a barrier (crew_barrier).
    /// Syscall number: 0x0703
    /// </summary>
    public static Task CrewBarrierAsync(
        CrewId crewId,
        int? timeoutMs = null)
    {
        throw new CsciException(
            CsciErrorCode.Unimplemented,
            "CrewBarrierAsync is not yet implemented");
    }
}
