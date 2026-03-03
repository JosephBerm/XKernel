// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.

#nullable enable

namespace CognitiveSubstrate.SDK.Syscalls;

using System;
using System.Threading.Tasks;
using Types;

/// <summary>
/// Signals family syscalls for signal handling.
/// 
/// Syscalls:
/// - SigSendAsync (0x0600): Send a signal to an agent/task
/// - SigHandlerInstallAsync (0x0601): Install a signal handler
/// 
/// Total: 2 syscalls
/// </summary>
public static class SignalSyscalls
{
    /// <summary>
    /// Send a signal to an agent/task (sig_send).
    /// Syscall number: 0x0600
    /// </summary>
    public static Task SigSendAsync(
        AgentId recipientId,
        int signalNumber,
        object? data = null)
    {
        throw new CsciException(
            CsciErrorCode.Unimplemented,
            "SigSendAsync is not yet implemented");
    }

    /// <summary>
    /// Install a signal handler (sig_handler_install).
    /// Syscall number: 0x0601
    /// </summary>
    public static Task<SignalHandlerId> SigHandlerInstallAsync(
        int signalNumber,
        Func<int, object?, Task> handlerFn,
        uint? flags = null)
    {
        throw new CsciException(
            CsciErrorCode.Unimplemented,
            "SigHandlerInstallAsync is not yet implemented");
    }
}
