// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.

#nullable enable

namespace CognitiveSubstrate.SDK;

using System;
using System.Collections.Generic;
using System.Threading;
using System.Threading.Tasks;
using Types;

/// <summary>
/// Connection configuration for Cognitive Substrate kernel
/// </summary>
public class ConnectionConfig
{
    /// <summary>Kernel endpoint URL or socket path</summary>
    public required string Endpoint { get; init; }

    /// <summary>Authentication token (optional)</summary>
    public string? Token { get; init; }

    /// <summary>Connection timeout in milliseconds (default: 5000)</summary>
    public int Timeout { get; init; } = 5000;

    /// <summary>Auto-reconnect on disconnect (default: true)</summary>
    public bool AutoReconnect { get; init; } = true;

    /// <summary>Number of reconnect attempts (default: 3)</summary>
    public int MaxReconnectAttempts { get; init; } = 3;
}

/// <summary>
/// Kernel event listener delegate
/// </summary>
public delegate Task KernelEventListener(KernelEvent @event);

/// <summary>
/// Kernel event data
/// </summary>
public class KernelEvent
{
    /// <summary>Event type</summary>
    public required string Type { get; init; }

    /// <summary>Event timestamp</summary>
    public DateTime Timestamp { get; init; } = DateTime.UtcNow;

    /// <summary>Event data</summary>
    public object? Data { get; init; }
}

/// <summary>
/// CognitiveSubstrateClient - Main SDK client class
///
/// Provides async/await interfaces for all CSCI syscalls and manages
/// the connection to the Cognitive Substrate kernel.
///
/// Example:
/// ```csharp
/// var client = new CognitiveSubstrateClient(new ConnectionConfig
/// {
///     Endpoint = "ws://localhost:8080"
/// });
///
/// await client.ConnectAsync();
///
/// var taskId = await client.SpawnAsync(
///     new AgentId("agent-1"),
///     new TaskConfig { Name = "my-task" },
///     new[] { "memory", "ipc" },
///     new ResourceBudget { MemoryBytes = 1024 * 1024 }
/// );
///
/// await client.DisconnectAsync();
/// ```
/// </summary>
public class CognitiveSubstrateClient : IAsyncDisposable
{
    private readonly ConnectionConfig _config;
    private bool _connected = false;
    private readonly Dictionary<string, HashSet<KernelEventListener>> _eventListeners = new();
    private readonly SemaphoreSlim _connectionLock = new(1, 1);

    /// <summary>Create a new Cognitive Substrate client</summary>
    public CognitiveSubstrateClient(ConnectionConfig config)
    {
        _config = config ?? throw new ArgumentNullException(nameof(config));
        if (string.IsNullOrWhiteSpace(_config.Endpoint))
        {
            throw new ArgumentException("Endpoint cannot be null or empty", nameof(config));
        }
    }

    /// <summary>Connect to the Cognitive Substrate kernel</summary>
    public async Task ConnectAsync(CancellationToken cancellationToken = default)
    {
        await _connectionLock.WaitAsync(cancellationToken);
        try
        {
            if (_connected)
                return;

            // Implementation would establish connection to kernel
            // via WebSocket, HTTP, or IPC based on endpoint
            _connected = true;
        }
        catch (Exception ex)
        {
            throw new InvalidOperationException($"Failed to connect to kernel: {ex.Message}", ex);
        }
        finally
        {
            _connectionLock.Release();
        }
    }

    /// <summary>Disconnect from the Cognitive Substrate kernel</summary>
    public async Task DisconnectAsync(CancellationToken cancellationToken = default)
    {
        await _connectionLock.WaitAsync(cancellationToken);
        try
        {
            _connected = false;
        }
        finally
        {
            _connectionLock.Release();
        }
    }

    /// <summary>Check if client is connected</summary>
    public bool IsConnected => _connected;

    /// <summary>Spawn a new cognitive task</summary>
    public async Task<CognitiveTaskId> SpawnAsync(
        AgentId parentAgent,
        TaskConfig config,
        IEnumerable<string> capabilities,
        ResourceBudget budget,
        CancellationToken cancellationToken = default)
    {
        EnsureConnected();
        // Implementation would send ct_spawn syscall to kernel
        throw new NotImplementedException();
    }

    /// <summary>Yield task execution</summary>
    public async Task YieldAsync(
        CognitiveTaskId taskId,
        string? hint = null,
        CancellationToken cancellationToken = default)
    {
        EnsureConnected();
        // Implementation would send ct_yield syscall to kernel
        await Task.CompletedTask;
    }

    /// <summary>Send message over a channel</summary>
    public async Task SendAsync(
        ChannelId channelId,
        MessagePayload payload,
        CancellationToken cancellationToken = default)
    {
        EnsureConnected();
        // Implementation would send ch_send syscall to kernel
        await Task.CompletedTask;
    }

    /// <summary>Receive message from a channel</summary>
    public async Task<MessagePayload> ReceiveAsync(
        ChannelId channelId,
        CancellationToken cancellationToken = default)
    {
        EnsureConnected();
        // Implementation would send ch_receive syscall to kernel
        throw new NotImplementedException();
    }

    /// <summary>Register event listener</summary>
    public void On(string eventType, KernelEventListener listener)
    {
        if (!_eventListeners.TryGetValue(eventType, out var listeners))
        {
            listeners = new HashSet<KernelEventListener>();
            _eventListeners[eventType] = listeners;
        }
        listeners.Add(listener);
    }

    /// <summary>Unregister event listener</summary>
    public void Off(string eventType, KernelEventListener listener)
    {
        if (_eventListeners.TryGetValue(eventType, out var listeners))
        {
            listeners.Remove(listener);
        }
    }

    /// <summary>Ensure client is connected</summary>
    private void EnsureConnected()
    {
        if (!_connected)
        {
            throw new InvalidOperationException(
                "Client is not connected. Call ConnectAsync() first.");
        }
    }

    /// <summary>Async disposal pattern</summary>
    async ValueTask IAsyncDisposable.DisposeAsync()
    {
        if (_connected)
        {
            await DisconnectAsync();
        }
        _connectionLock.Dispose();
    }
}
