// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.

#nullable enable

namespace CognitiveSubstrate.SDK.Syscalls;

using System;
using System.Collections.Generic;
using System.Threading.Tasks;
using Types;

/// <summary>
/// Channel (IPC) family syscalls for inter-process communication.
/// 
/// Syscalls:
/// - ChCreateAsync (0x0300): Create a communication channel
/// - ChSendAsync (0x0301): Send a message on a channel
/// - ChReceiveAsync (0x0302): Receive a message from a channel
/// 
/// Total: 3 syscalls
/// </summary>
public static class IpcSyscalls
{
    /// <summary>
    /// Create a communication channel (ch_create).
    /// Syscall number: 0x0300
    /// </summary>
    public static Task<ChannelId> ChCreateAsync(ChannelConfig config)
    {
        throw new CsciException(
            CsciErrorCode.Unimplemented,
            "ChCreateAsync is not yet implemented");
    }

    /// <summary>
    /// Send a message on a channel (ch_send).
    /// Syscall number: 0x0301
    /// </summary>
    public static Task<ulong> ChSendAsync(
        ChannelId channelId,
        MessagePayload message,
        SendFlags flags = SendFlags.Default,
        int? timeoutMs = null)
    {
        throw new CsciException(
            CsciErrorCode.Unimplemented,
            "ChSendAsync is not yet implemented");
    }

    /// <summary>
    /// Receive a message from a channel (ch_receive).
    /// Syscall number: 0x0302
    /// </summary>
    public static Task<(MessagePayload Message, ulong Bytes)> ChReceiveAsync(
        ChannelId channelId,
        int? timeoutMs = null)
    {
        throw new CsciException(
            CsciErrorCode.Unimplemented,
            "ChReceiveAsync is not yet implemented");
    }
}
