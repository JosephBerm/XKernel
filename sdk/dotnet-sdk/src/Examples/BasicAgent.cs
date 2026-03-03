/*
 * # Basic Agent Example (C#)
 *
 * Demonstrates the core Agent API in C# idiom with:
 * - Creating a CognitiveTask
 * - Allocating SemanticMemory
 * - Spawning an agent
 * - Using channels for IPC
 * - Invoking a tool
 * - Proper async/await with Task patterns
 * - IDisposable for resource cleanup
 * - Nullable reference type support
 *
 * @module Examples.BasicAgent
 */

#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Serialization;
using System.Threading;
using System.Threading.Tasks;

namespace CognitiveSubstrate.Examples
{
    /// <summary>
    /// Result type for operations that may fail
    /// </summary>
    public abstract record Result<T>
    {
        public sealed record Success(T Value) : Result<T>;
        public sealed record Failure(Exception Error) : Result<T>;

        public TResult Match<TResult>(
            Func<T, TResult> onSuccess,
            Func<Exception, TResult> onFailure) =>
            this switch
            {
                Success s => onSuccess(s.Value),
                Failure f => onFailure(f.Error),
                _ => throw new InvalidOperationException("Unknown Result type"),
            };

        public async Task<TResult> MatchAsync<TResult>(
            Func<T, Task<TResult>> onSuccess,
            Func<Exception, Task<TResult>> onFailure) =>
            this switch
            {
                Success s => await onSuccess(s.Value),
                Failure f => await onFailure(f.Error),
                _ => throw new InvalidOperationException("Unknown Result type"),
            };
    }

    /// <summary>
    /// Represents a cognitive task with metadata
    /// </summary>
    public record CognitiveTask(
        string Id,
        string Name,
        string Description,
        TaskPriority Priority,
        int TimeoutMs);

    /// <summary>
    /// Task priority enumeration
    /// </summary>
    public enum TaskPriority
    {
        Low,
        Normal,
        High,
    }

    /// <summary>
    /// Semantic memory allocation
    /// </summary>
    public sealed class SemanticMemory : IDisposable
    {
        private readonly MemoryAddress _address;
        private readonly int _capacityBytes;
        private int _usedBytes;
        private readonly Dictionary<string, object?> _entries;
        private bool _disposed;

        public MemoryAddress Address => _address;
        public int CapacityBytes => _capacityBytes;
        public int UsedBytes => _usedBytes;

        public SemanticMemory(int capacityBytes)
        {
            _address = new MemoryAddress(1000);
            _capacityBytes = capacityBytes;
            _usedBytes = 0;
            _entries = new Dictionary<string, object?>();
        }

        /// <summary>
        /// Allocate memory for an entry
        /// </summary>
        public async Task<Result<MemoryAddress>> AllocateAsync(
            string key,
            object? value,
            CancellationToken cancellationToken = default)
        {
            ThrowIfDisposed();

            await Task.Yield();

            var estimatedSize = System.Text.Json.JsonSerializer.Serialize(value).Length;

            if (_usedBytes + estimatedSize > _capacityBytes)
            {
                return new Result<MemoryAddress>.Failure(
                    new InvalidOperationException("Insufficient memory"));
            }

            _entries[key] = value;
            _usedBytes += estimatedSize;

            var addr = new MemoryAddress(1000 + _entries.Count);
            return new Result<MemoryAddress>.Success(addr);
        }

        /// <summary>
        /// Retrieve value from memory
        /// </summary>
        public async Task<Result<object?>> GetAsync(
            string key,
            CancellationToken cancellationToken = default)
        {
            ThrowIfDisposed();
            await Task.Yield();

            if (_entries.TryGetValue(key, out var value))
            {
                return new Result<object?>.Success(value);
            }

            return new Result<object?>.Failure(
                new KeyNotFoundException($"Key not found: {key}"));
        }

        /// <summary>
        /// Get memory statistics
        /// </summary>
        public MemoryStats GetStats() => new(
            Address: _address,
            CapacityBytes: _capacityBytes,
            UsedBytes: _usedBytes,
            EntryCount: _entries.Count);

        public void Dispose()
        {
            if (_disposed) return;
            _entries.Clear();
            _usedBytes = 0;
            _disposed = true;
        }

        private void ThrowIfDisposed()
        {
            if (_disposed)
                throw new ObjectDisposedException(nameof(SemanticMemory));
        }
    }

    /// <summary>
    /// Memory statistics
    /// </summary>
    public record MemoryStats(
        MemoryAddress Address,
        int CapacityBytes,
        int UsedBytes,
        int EntryCount);

    /// <summary>
    /// Strongly-typed memory address
    /// </summary>
    public readonly record struct MemoryAddress(int Value)
    {
        public override string ToString() => $"0x{Value:X}";
    }

    /// <summary>
    /// Message for inter-process communication
    /// </summary>
    public record Message(
        string FromAgent,
        string ToAgent,
        long TimestampUtc,
        string Content,
        string CorrelationId);

    /// <summary>
    /// Channel for bidirectional communication
    /// </summary>
    public interface IChannel : IAsyncDisposable
    {
        string Id { get; }
        Task<Result<Unit>> SendAsync(Message message, CancellationToken cancellationToken = default);
        Task<Result<Message>> ReceiveAsync(CancellationToken cancellationToken = default);
        Task<Result<Unit>> CloseAsync(CancellationToken cancellationToken = default);
    }

    /// <summary>
    /// Unit type for void operations
    /// </summary>
    public readonly record struct Unit;

    /// <summary>
    /// Simple channel implementation
    /// </summary>
    public sealed class SimpleChannel : IChannel, IAsyncDisposable
    {
        private readonly Queue<Message> _messageQueue;
        private bool _closed;
        private bool _disposed;

        public string Id { get; }

        public SimpleChannel(string channelId)
        {
            Id = channelId;
            _messageQueue = new Queue<Message>();
            _closed = false;
        }

        public async Task<Result<Unit>> SendAsync(
            Message message,
            CancellationToken cancellationToken = default)
        {
            await Task.Yield();

            if (_closed)
            {
                return new Result<Unit>.Failure(
                    new InvalidOperationException("Channel is closed"));
            }

            lock (_messageQueue)
            {
                _messageQueue.Enqueue(message);
            }

            return new Result<Unit>.Success(new Unit());
        }

        public async Task<Result<Message>> ReceiveAsync(CancellationToken cancellationToken = default)
        {
            await Task.Yield();

            if (_closed && _messageQueue.Count == 0)
            {
                return new Result<Message>.Failure(
                    new InvalidOperationException("Channel is closed"));
            }

            lock (_messageQueue)
            {
                if (_messageQueue.TryDequeue(out var message))
                {
                    return new Result<Message>.Success(message);
                }
            }

            return new Result<Message>.Failure(
                new InvalidOperationException("No messages available"));
        }

        public async Task<Result<Unit>> CloseAsync(CancellationToken cancellationToken = default)
        {
            await Task.Yield();
            _closed = true;
            return new Result<Unit>.Success(new Unit());
        }

        async ValueTask IAsyncDisposable.DisposeAsync()
        {
            if (_disposed) return;
            await CloseAsync();
            _messageQueue.Clear();
            _disposed = true;
        }
    }

    /// <summary>
    /// Tool definition and invocation
    /// </summary>
    public interface ITool
    {
        string Name { get; }
        string Description { get; }
        Task<Result<string>> InvokeAsync(string input, CancellationToken cancellationToken = default);
    }

    /// <summary>
    /// Tool registry and executor
    /// </summary>
    public sealed class ToolRegistry
    {
        private readonly Dictionary<string, ITool> _tools = new();

        /// <summary>
        /// Register a tool
        /// </summary>
        public Result<Unit> Register(ITool tool)
        {
            if (_tools.ContainsKey(tool.Name))
            {
                return new Result<Unit>.Failure(
                    new InvalidOperationException($"Tool already registered: {tool.Name}"));
            }

            _tools[tool.Name] = tool;
            return new Result<Unit>.Success(new Unit());
        }

        /// <summary>
        /// Invoke a tool by name
        /// </summary>
        public async Task<Result<string>> InvokeAsync(
            string toolName,
            string input,
            CancellationToken cancellationToken = default)
        {
            if (!_tools.TryGetValue(toolName, out var tool))
            {
                return new Result<string>.Failure(
                    new KeyNotFoundException($"Tool not found: {toolName}"));
            }

            return await tool.InvokeAsync(input, cancellationToken);
        }

        /// <summary>
        /// List all registered tools
        /// </summary>
        public IEnumerable<string> ListTools() => _tools.Keys;
    }

    /// <summary>
    /// Agent configuration
    /// </summary>
    public record AgentConfig(
        string Id,
        string TaskId,
        MemoryAddress MemoryAddress,
        int MaxIterations);

    /// <summary>
    /// Basic Agent implementation
    /// </summary>
    public sealed class BasicAgent : IAsyncDisposable
    {
        private readonly AgentConfig _config;
        private readonly SemanticMemory _memory;
        private readonly ToolRegistry _tools;
        private readonly Dictionary<string, IChannel> _channels;
        private int _iterationCount;
        private AgentStatus _taskStatus;
        private bool _disposed;

        public enum AgentStatus
        {
            Idle,
            Running,
            Completed,
            Failed,
        }

        public BasicAgent(
            AgentConfig config,
            SemanticMemory memory,
            ToolRegistry tools)
        {
            _config = config;
            _memory = memory ?? throw new ArgumentNullException(nameof(memory));
            _tools = tools ?? throw new ArgumentNullException(nameof(tools));
            _channels = new Dictionary<string, IChannel>();
            _iterationCount = 0;
            _taskStatus = AgentStatus.Idle;
        }

        /// <summary>
        /// Register a channel for communication
        /// </summary>
        public Result<Unit> RegisterChannel(IChannel channel)
        {
            if (_channels.ContainsKey(channel.Id))
            {
                return new Result<Unit>.Failure(
                    new InvalidOperationException($"Channel already registered: {channel.Id}"));
            }

            _channels[channel.Id] = channel;
            return new Result<Unit>.Success(new Unit());
        }

        /// <summary>
        /// Execute the cognitive task
        /// </summary>
        public async Task<Result<string>> ExecuteAsync(
            CognitiveTask task,
            CancellationToken cancellationToken = default)
        {
            ThrowIfDisposed();

            try
            {
                _taskStatus = AgentStatus.Running;

                var initMemResult = await _memory.AllocateAsync(
                    "task_state",
                    new
                    {
                        task.Id,
                        StartTime = DateTime.UtcNow,
                        Iterations = 0,
                    },
                    cancellationToken);

                if (initMemResult is not Result<MemoryAddress>.Success)
                {
                    _taskStatus = AgentStatus.Failed;
                    return new Result<string>.Failure(
                        new InvalidOperationException("Failed to initialize task state"));
                }

                while (_iterationCount < _config.MaxIterations)
                {
                    var iterResult = await ExecuteIterationAsync(task, cancellationToken);
                    if (iterResult is Result<Unit>.Failure f)
                    {
                        _taskStatus = AgentStatus.Failed;
                        return new Result<string>.Failure(f.Error);
                    }

                    _iterationCount++;

                    await _memory.AllocateAsync("iteration_count", _iterationCount, cancellationToken);

                    if (_iterationCount >= 2)
                        break;
                }

                _taskStatus = AgentStatus.Completed;
                return new Result<string>.Success(
                    $"Task {task.Id} completed with {_iterationCount} iterations");
            }
            catch (Exception ex)
            {
                _taskStatus = AgentStatus.Failed;
                return new Result<string>.Failure(ex);
            }
        }

        /// <summary>
        /// Execute a single iteration
        /// </summary>
        private async Task<Result<Unit>> ExecuteIterationAsync(
            CognitiveTask task,
            CancellationToken cancellationToken = default)
        {
            var searchResult = await _tools.InvokeAsync(
                "search_documents",
                $"Query for task {task.Id}",
                cancellationToken);

            if (searchResult is Result<string>.Failure f)
            {
                return new Result<Unit>.Failure(f.Error);
            }

            var success = searchResult as Result<string>.Success;
            var storeResult = await _memory.AllocateAsync(
                $"iteration_{_iterationCount}_result",
                success?.Value,
                cancellationToken);

            if (storeResult is Result<MemoryAddress>.Failure sf)
            {
                return new Result<Unit>.Failure(sf.Error);
            }

            if (_channels.Count > 0)
            {
                var msg = new Message(
                    FromAgent: _config.Id,
                    ToAgent: "observer",
                    TimestampUtc: DateTimeOffset.UtcNow.ToUnixTimeMilliseconds(),
                    Content: success?.Value ?? string.Empty,
                    CorrelationId: $"iter_{_iterationCount}");

                foreach (var channel in _channels.Values)
                {
                    var sendResult = await channel.SendAsync(msg, cancellationToken);
                    if (sendResult is Result<Unit>.Failure sf2)
                    {
                        Console.WriteLine($"Warning: Failed to send message: {sf2.Error}");
                    }
                }
            }

            return new Result<Unit>.Success(new Unit());
        }

        /// <summary>
        /// Get current task status
        /// </summary>
        public string GetStatus() =>
            $"Agent {_config.Id}: {_taskStatus} ({_iterationCount} iterations)";

        /// <summary>
        /// Get memory statistics
        /// </summary>
        public MemoryStats GetMemoryStats() => _memory.GetStats();

        async ValueTask IAsyncDisposable.DisposeAsync()
        {
            if (_disposed) return;

            foreach (var channel in _channels.Values)
            {
                await channel.DisposeAsync();
            }

            _channels.Clear();
            _memory.Dispose();
            _disposed = true;
        }

        private void ThrowIfDisposed()
        {
            if (_disposed)
                throw new ObjectDisposedException(nameof(BasicAgent));
        }
    }

    /// <summary>
    /// Example execution entry point
    /// </summary>
    public static class BasicAgentExample
    {
        public static async Task<int> MainAsync(string[] args)
        {
            try
            {
                Console.WriteLine("=== Basic Agent Example (C#) ===\n");

                // 1. Create a cognitive task
                var task = new CognitiveTask(
                    Id: "search-documents-001",
                    Name: "Document Search",
                    Description: "Search and analyze documents",
                    Priority: TaskPriority.High,
                    TimeoutMs: 10000);

                Console.WriteLine($"Created task: {task.Name} ({task.Id})");

                // 2. Allocate semantic memory
                using var memory = new SemanticMemory(1024 * 1024); // 1MB
                Console.WriteLine("Allocated semantic memory (1MB)\n");

                // 3. Register tools
                var tools = new ToolRegistry();

                var searchTool = new SearchDocumentsTool();
                var analyzeTool = new AnalyzeTextTool();

                var registerSearch = tools.Register(searchTool);
                if (registerSearch is not Result<Unit>.Success)
                {
                    return 1;
                }

                var registerAnalyze = tools.Register(analyzeTool);
                if (registerAnalyze is not Result<Unit>.Success)
                {
                    return 1;
                }

                var toolNames = string.Join(", ", tools.ListTools());
                Console.WriteLine($"Registered tools: {toolNames}\n");

                // 4. Create channels for IPC
                using var channel = new SimpleChannel("agent-channel-001");
                Console.WriteLine("Created communication channel\n");

                // 5. Spawn agent
                var config = new AgentConfig(
                    Id: "agent-001",
                    TaskId: "search-documents-001",
                    MemoryAddress: new MemoryAddress(1000),
                    MaxIterations: 3);

                using var agent = new BasicAgent(config, memory, tools);

                var registerChannel = agent.RegisterChannel(channel);
                if (registerChannel is not Result<Unit>.Success)
                {
                    return 1;
                }

                Console.WriteLine($"Spawned agent: {config.Id}\n");

                // 6. Execute task
                Console.WriteLine("Starting task execution...");
                var execResult = await agent.ExecuteAsync(task);

                if (execResult is Result<string>.Success success)
                {
                    Console.WriteLine($"✓ {success.Value}\n");
                }
                else if (execResult is Result<string>.Failure failure)
                {
                    Console.WriteLine($"✗ Error: {failure.Error.Message}");
                    return 1;
                }

                // 7. Display results
                Console.WriteLine("Agent Status: " + agent.GetStatus());
                var stats = agent.GetMemoryStats();
                Console.WriteLine(
                    $"Memory Usage: {stats.UsedBytes}/{stats.CapacityBytes} bytes\n");

                Console.WriteLine("✓ Example completed successfully");
                return 0;
            }
            catch (Exception ex)
            {
                Console.WriteLine($"Fatal error: {ex.Message}");
                return 1;
            }
        }

        // Tool implementations
        private sealed class SearchDocumentsTool : ITool
        {
            public string Name => "search_documents";
            public string Description => "Search documents by query";

            public async Task<Result<string>> InvokeAsync(
                string input,
                CancellationToken cancellationToken = default)
            {
                await Task.Delay(100, cancellationToken);
                return new Result<string>.Success($"Found documents matching: {input}");
            }
        }

        private sealed class AnalyzeTextTool : ITool
        {
            public string Name => "analyze_text";
            public string Description => "Analyze text content";

            public async Task<Result<string>> InvokeAsync(
                string input,
                CancellationToken cancellationToken = default)
            {
                await Task.Delay(50, cancellationToken);
                return new Result<string>.Success($"Analysis: {input.Length} chars");
            }
        }
    }
}
