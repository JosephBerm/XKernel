// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.

#nullable enable

namespace CognitiveSubstrate.SDK.Syscalls;

using System;
using System.Collections.Generic;
using System.Threading.Tasks;
using Types;

/// <summary>
/// Tool family syscalls for external tool integration.
/// 
/// Syscalls:
/// - ToolInvokeAsync (0x0200): Invoke an external tool
/// - ToolBindAsync (0x0201): Bind a tool to the namespace
/// 
/// Total: 2 syscalls
/// </summary>
public static class ToolSyscalls
{
    /// <summary>
    /// Invoke an external tool (tool_invoke).
    /// Syscall number: 0x0200
    /// </summary>
    public static Task<ToolResult> ToolInvokeAsync(
        string toolName,
        Dictionary<string, object>? args = null,
        SandboxConfig? sandboxConfig = null)
    {
        throw new CsciException(
            CsciErrorCode.Unimplemented,
            "ToolInvokeAsync is not yet implemented");
    }

    /// <summary>
    /// Bind a tool to the namespace (tool_bind).
    /// Syscall number: 0x0201
    /// </summary>
    public static Task ToolBindAsync(
        string toolName,
        string namespacePath,
        IEnumerable<string> capabilities)
    {
        throw new CsciException(
            CsciErrorCode.Unimplemented,
            "ToolBindAsync is not yet implemented");
    }
}
