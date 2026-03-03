// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.

#nullable enable

namespace CognitiveSubstrate.SDK.Syscalls;

using System;
using System.Collections.Generic;
using System.Threading.Tasks;
using Types;

/// <summary>
/// Capability (Security) family syscalls for capability-based security.
/// 
/// Syscalls:
/// - CapDelegateAsync (0x0500): Permanently transfer capabilities
/// - CapGrantAsync (0x0501): Temporarily grant capabilities
/// - CapRevokeAsync (0x0502): Revoke granted capabilities
/// 
/// Total: 3 syscalls
/// </summary>
public static class SecuritySyscalls
{
    /// <summary>
    /// Permanently delegate capabilities (cap_delegate).
    /// Syscall number: 0x0500
    /// </summary>
    public static Task CapDelegateAsync(
        AgentId recipientId,
        CapabilitySet capabilitySet,
        Dictionary<string, object>? config = null)
    {
        throw new CsciException(
            CsciErrorCode.Unimplemented,
            "CapDelegateAsync is not yet implemented");
    }

    /// <summary>
    /// Temporarily grant capabilities (cap_grant).
    /// Syscall number: 0x0501
    /// </summary>
    public static Task<GrantHandle> CapGrantAsync(
        AgentId recipientId,
        CapabilitySet capabilitySet,
        int durationMs,
        Dictionary<string, object>? config = null)
    {
        throw new CsciException(
            CsciErrorCode.Unimplemented,
            "CapGrantAsync is not yet implemented");
    }

    /// <summary>
    /// Revoke granted capabilities (cap_revoke).
    /// Syscall number: 0x0502
    /// </summary>
    public static Task CapRevokeAsync(
        GrantHandle grantHandle,
        string? reason = null)
    {
        throw new CsciException(
            CsciErrorCode.Unimplemented,
            "CapRevokeAsync is not yet implemented");
    }
}
