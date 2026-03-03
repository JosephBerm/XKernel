// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.

#nullable enable

namespace CognitiveSubstrate.SDK;

using System;

/// <summary>
/// CSCI Error Code enumeration.
/// 
/// Numeric codes match POSIX where applicable, with custom codes for CSCI-specific errors.
/// </summary>
public enum CsciErrorCode : uint
{
    /// <summary>
    /// Success - operation completed without error.
    /// </summary>
    Success = 0,

    /// <summary>
    /// Permission denied - caller lacks required capability.
    /// </summary>
    PermissionDenied = 1,

    /// <summary>
    /// Not found - referenced resource does not exist.
    /// </summary>
    NotFound = 2,

    /// <summary>
    /// Out of memory - insufficient memory available.
    /// </summary>
    OutOfMemory = 12,

    /// <summary>
    /// Resource busy - resource is in use and cannot be modified.
    /// </summary>
    ResourceBusy = 16,

    /// <summary>
    /// Already exists - resource with this name/ID already exists.
    /// </summary>
    AlreadyExists = 17,

    /// <summary>
    /// Invalid argument - syscall arguments do not satisfy preconditions.
    /// </summary>
    InvalidArgument = 22,

    /// <summary>
    /// Operation timed out - operation exceeded deadline.
    /// </summary>
    TimedOut = 110,

    /// <summary>
    /// Budget exhausted - operation would exceed resource budget.
    /// </summary>
    BudgetExhausted = 200,

    /// <summary>
    /// Dependency cycle - cyclic dependency would be created.
    /// </summary>
    CyclicDependency = 201,

    /// <summary>
    /// Not implemented - feature not yet implemented.
    /// </summary>
    Unimplemented = 202,

    /// <summary>
    /// Channel closed - channel endpoint has been closed.
    /// </summary>
    ChannelClosed = 203,

    /// <summary>
    /// Message too large - message exceeds channel capacity.
    /// </summary>
    MessageTooLarge = 204,

    /// <summary>
    /// No message - no message available on channel.
    /// </summary>
    NoMessage = 205,

    /// <summary>
    /// Sandbox error - sandbox configuration or execution failed.
    /// </summary>
    SandboxError = 206,

    /// <summary>
    /// Tool error - tool execution failed.
    /// </summary>
    ToolError = 207,

    /// <summary>
    /// Invalid attenuation - attenuation spec is invalid.
    /// </summary>
    InvalidAttenuation = 208,

    /// <summary>
    /// Policy violation - operation violates security policy.
    /// </summary>
    PolicyViolation = 209,

    /// <summary>
    /// Resource full - resource at capacity cannot accept more.
    /// </summary>
    ResourceFull = 210,

    /// <summary>
    /// Buffer overflow - write would exceed buffer capacity.
    /// </summary>
    BufferOverflow = 211,
}

/// <summary>
/// CSCI Exception for syscall errors.
/// 
/// Provides type-safe error handling with code and context information.
/// </summary>
public class CsciException : Exception
{
    /// <summary>
    /// Get the error code.
    /// </summary>
    public CsciErrorCode Code { get; }

    /// <summary>
    /// Get optional context information.
    /// </summary>
    public Dictionary<string, object>? Context { get; }

    /// <summary>
    /// Create a new CSCI exception.
    /// </summary>
    /// <param name="code">Error code.</param>
    /// <param name="message">Human-readable error message.</param>
    /// <param name="context">Optional context information.</param>
    public CsciException(
        CsciErrorCode code,
        string message,
        Dictionary<string, object>? context = null)
        : base(message)
    {
        Code = code;
        Context = context;
    }

    /// <summary>
    /// Create a new CSCI exception with inner exception.
    /// </summary>
    public CsciException(
        CsciErrorCode code,
        string message,
        Exception innerException,
        Dictionary<string, object>? context = null)
        : base(message, innerException)
    {
        Code = code;
        Context = context;
    }

    /// <summary>
    /// Get the numeric error code value.
    /// </summary>
    public uint GetCodeValue() => (uint)Code;

    /// <summary>
    /// Get a string representation of the error.
    /// </summary>
    public override string ToString()
        => $"CsciException({Code}): {Message}";
}
