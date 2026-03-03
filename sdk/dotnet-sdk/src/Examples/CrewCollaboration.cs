/*
 * # Crew Collaboration Example (C#)
 *
 * Demonstrates multi-agent coordination using AgentCrew with:
 * - Creating an AgentCrew
 * - Coordinating agents via shared channels
 * - Capability delegation between agents
 * - Checkpoint and restore functionality
 *
 * ## Scenario
 * A crew of specialized agents working together:
 * - Researcher: Searches for information
 * - Analyzer: Processes and analyzes findings
 * - Reporter: Generates final reports
 *
 * @module Examples.CrewCollaboration
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
    /// Agent capability enumeration
    /// </summary>
    public enum Capability
    {
        Research,
        Analysis,
        Reporting,
        Coordination,
    }

    /// <summary>
    /// Agent role specification
    /// </summary>
    public record AgentRole(
        string Name,
        string Description,
        IList<Capability> Capabilities,
        AgentPriority Priority);

    /// <summary>
    /// Agent priority levels
    /// </summary>
    public enum AgentPriority
    {
        Low,
        Normal,
        High,
    }

    /// <summary>
    /// Message for inter-agent communication
    /// </summary>
    public record CrewMessage(
        string From,
        string To,
        long TimestampUtc,
        CrewMessageType MessageType,
        Dictionary<string, object?> Content,
        string CorrelationId);

    /// <summary>
    /// Message type enumeration
    /// </summary>
    public enum CrewMessageType
    {
        Task,
        Result,
        Query,
        Status,
    }

    /// <summary>
    /// Checkpoint for state persistence
    /// </summary>
    public record Checkpoint(
        string Id,
        string CrewId,
        long TimestampUtc,
        Dictionary<string, Dictionary<string, object?>> AgentStates,
        Dictionary<string, object?> SharedContext);

    /// <summary>
    /// Crew member definition
    /// </summary>
    public record CrewMember(
        string Id,
        AgentRole Role,
        bool Active);

    /// <summary>
    /// Result type
    /// </summary>
    public abstract record Result<T>
    {
        public sealed record Success(T Value) : Result<T>;
        public sealed record Failure(Exception Error) : Result<T>;
    }

    /// <summary>
    /// Unit type
    /// </summary>
    public readonly record struct Unit;

    /// <summary>
    /// Manages inter-crew communication
    /// </summary>
    public sealed class CrewChannelManager
    {
        private readonly Dictionary<string, Queue<CrewMessage>> _channels = new();
        private readonly object _lockObj = new();

        /// <summary>
        /// Create or get a channel
        /// </summary>
        public string GetOrCreate(string channelId)
        {
            lock (_lockObj)
            {
                if (!_channels.ContainsKey(channelId))
                {
                    _channels[channelId] = new Queue<CrewMessage>();
                }
                return channelId;
            }
        }

        /// <summary>
        /// Broadcast message to all agents
        /// </summary>
        public async Task<Result<Unit>> BroadcastAsync(
            CrewMessage message,
            CancellationToken cancellationToken = default)
        {
            await Task.Yield();

            if (message.To != "all")
            {
                return new Result<Unit>.Failure(
                    new InvalidOperationException("Broadcast requires To: 'all'"));
            }

            lock (_lockObj)
            {
                foreach (var queue in _channels.Values)
                {
                    queue.Enqueue(message);
                }
            }

            return new Result<Unit>.Success(new Unit());
        }

        /// <summary>
        /// Send direct message
        /// </summary>
        public async Task<Result<Unit>> SendAsync(
            CrewMessage message,
            string channelId,
            CancellationToken cancellationToken = default)
        {
            await Task.Yield();

            lock (_lockObj)
            {
                if (!_channels.TryGetValue(channelId, out var queue))
                {
                    return new Result<Unit>.Failure(
                        new KeyNotFoundException($"Channel not found: {channelId}"));
                }

                queue.Enqueue(message);
            }

            return new Result<Unit>.Success(new Unit());
        }

        /// <summary>
        /// Receive messages for an agent
        /// </summary>
        public async Task<Result<List<CrewMessage>>> ReceiveAsync(
            string channelId,
            string agentId,
            CancellationToken cancellationToken = default)
        {
            await Task.Yield();

            lock (_lockObj)
            {
                if (!_channels.TryGetValue(channelId, out var queue))
                {
                    return new Result<List<CrewMessage>>.Failure(
                        new KeyNotFoundException($"Channel not found: {channelId}"));
                }

                var messages = queue
                    .Where(m => m.To == agentId || m.To == "all")
                    .ToList();

                return new Result<List<CrewMessage>>.Success(messages);
            }
        }

        /// <summary>
        /// Clear messages for an agent
        /// </summary>
        public async Task<Result<int>> ClearMessagesAsync(
            string channelId,
            string agentId,
            CancellationToken cancellationToken = default)
        {
            await Task.Yield();

            lock (_lockObj)
            {
                if (!_channels.TryGetValue(channelId, out var queue))
                {
                    return new Result<int>.Failure(
                        new KeyNotFoundException($"Channel not found: {channelId}"));
                }

                var initial_count = queue.Count;
                var remaining = queue
                    .Where(m => m.To != agentId && m.To != "all")
                    .ToList();

                var newQueue = new Queue<CrewMessage>(remaining);
                _channels[channelId] = newQueue;

                return new Result<int>.Success(initial_count - remaining.Count);
            }
        }
    }

    /// <summary>
    /// Checkpoint manager for state persistence
    /// </summary>
    public sealed class CheckpointManager
    {
        private readonly Dictionary<string, Checkpoint> _checkpoints = new();
        private readonly object _lockObj = new();

        /// <summary>
        /// Create a checkpoint
        /// </summary>
        public async Task<Result<string>> CreateAsync(
            string crewId,
            Dictionary<string, Dictionary<string, object?>> agentStates,
            Dictionary<string, object?> context,
            CancellationToken cancellationToken = default)
        {
            await Task.Yield();

            var checkpointId = $"cp_{crewId}_{DateTimeOffset.UtcNow.ToUnixTimeMilliseconds()}";

            var checkpoint = new Checkpoint(
                Id: checkpointId,
                CrewId: crewId,
                TimestampUtc: DateTimeOffset.UtcNow.ToUnixTimeMilliseconds(),
                AgentStates: new Dictionary<string, Dictionary<string, object?>>(agentStates),
                SharedContext: new Dictionary<string, object?>(context));

            lock (_lockObj)
            {
                _checkpoints[checkpointId] = checkpoint;
            }

            return new Result<string>.Success(checkpointId);
        }

        /// <summary>
        /// Restore from checkpoint
        /// </summary>
        public async Task<Result<Checkpoint>> RestoreAsync(
            string checkpointId,
            CancellationToken cancellationToken = default)
        {
            await Task.Yield();

            lock (_lockObj)
            {
                if (!_checkpoints.TryGetValue(checkpointId, out var checkpoint))
                {
                    return new Result<Checkpoint>.Failure(
                        new KeyNotFoundException($"Checkpoint not found: {checkpointId}"));
                }

                return new Result<Checkpoint>.Success(checkpoint);
            }
        }

        /// <summary>
        /// List checkpoints for a crew
        /// </summary>
        public async Task<Result<List<string>>> ListCheckpointsAsync(
            string crewId,
            CancellationToken cancellationToken = default)
        {
            await Task.Yield();

            lock (_lockObj)
            {
                var ids = _checkpoints
                    .Where(kvp => kvp.Value.CrewId == crewId)
                    .Select(kvp => kvp.Key)
                    .ToList();

                return new Result<List<string>>.Success(ids);
            }
        }

        /// <summary>
        /// Delete a checkpoint
        /// </summary>
        public async Task<Result<Unit>> DeleteAsync(
            string checkpointId,
            CancellationToken cancellationToken = default)
        {
            await Task.Yield();

            lock (_lockObj)
            {
                if (!_checkpoints.Remove(checkpointId))
                {
                    return new Result<Unit>.Failure(
                        new KeyNotFoundException($"Checkpoint not found: {checkpointId}"));
                }

                return new Result<Unit>.Success(new Unit());
            }
        }
    }

    /// <summary>
    /// Specialized crew member agent
    /// </summary>
    public sealed class CrewAgent
    {
        private readonly string _id;
        private AgentRole _role;
        private readonly Dictionary<string, object?> _state = new();
        private readonly List<string> _delegatedTasks = new();

        public string Id => _id;
        public AgentRole Role => _role;

        public CrewAgent(string id, AgentRole role)
        {
            _id = id;
            _role = role;
        }

        /// <summary>
        /// Check if agent has a capability
        /// </summary>
        public bool HasCapability(Capability capability) => _role.Capabilities.Contains(capability);

        /// <summary>
        /// Execute a task
        /// </summary>
        public async Task<Result<string>> ExecuteAsync(
            string task,
            CancellationToken cancellationToken = default)
        {
            await Task.Delay(100, cancellationToken);

            _state["last_task"] = task;
            _delegatedTasks.Add(task);

            var result = $"[{_role.Name}] Completed: {task}";
            return new Result<string>.Success(result);
        }

        /// <summary>
        /// Accept delegated capability
        /// </summary>
        public Result<Unit> DelegateCapability(Capability capability)
        {
            if (!_role.Capabilities.Contains(capability))
            {
                _role.Capabilities.Add(capability);
            }
            return new Result<Unit>.Success(new Unit());
        }

        /// <summary>
        /// Get agent state
        /// </summary>
        public Dictionary<string, object?> GetState() => new(_state);

        /// <summary>
        /// Set agent state
        /// </summary>
        public void SetState(Dictionary<string, object?> state)
        {
            _state.Clear();
            foreach (var kvp in state)
            {
                _state[kvp.Key] = kvp.Value;
            }
        }

        /// <summary>
        /// Get statistics
        /// </summary>
        public object GetStats() => new
        {
            Id = _id,
            Role = _role.Name,
            TasksCompleted = _delegatedTasks.Count,
            Capabilities = _role.Capabilities.Count,
        };
    }

    /// <summary>
    /// AgentCrew coordinator
    /// </summary>
    public sealed class AgentCrew
    {
        private readonly string _crewId;
        private readonly string _name;
        private readonly string _description;
        private readonly Dictionary<string, CrewAgent> _agents = new();
        private readonly List<CrewMember> _members = new();
        private readonly CrewChannelManager _channels;
        private readonly CheckpointManager _checkpoints;
        private readonly List<string> _executionLog = new();
        private readonly Dictionary<string, object?> _context = new();

        public AgentCrew(string crewId, string name, string description)
        {
            _crewId = crewId;
            _name = name;
            _description = description;
            _channels = new CrewChannelManager();
            _checkpoints = new CheckpointManager();
        }

        /// <summary>
        /// Add member to crew
        /// </summary>
        public Result<Unit> AddMember(string id, AgentRole role)
        {
            if (_agents.ContainsKey(id))
            {
                return new Result<Unit>.Failure(
                    new InvalidOperationException($"Agent already exists: {id}"));
            }

            var agent = new CrewAgent(id, role);
            _agents[id] = agent;

            var member = new CrewMember(
                Id: id,
                Role: role,
                Active: true);

            _members.Add(member);
            Log($"Added member: {role.Name} ({id})");

            return new Result<Unit>.Success(new Unit());
        }

        /// <summary>
        /// Get agent by ID
        /// </summary>
        public Result<CrewAgent> GetAgent(string id)
        {
            if (!_agents.TryGetValue(id, out var agent))
            {
                return new Result<CrewAgent>.Failure(
                    new KeyNotFoundException($"Agent not found: {id}"));
            }

            return new Result<CrewAgent>.Success(agent);
        }

        /// <summary>
        /// Execute task with agent
        /// </summary>
        public async Task<Result<string>> ExecuteTaskAsync(
            string agentId,
            string task,
            CancellationToken cancellationToken = default)
        {
            var agentResult = GetAgent(agentId);

            if (agentResult is Result<CrewAgent>.Failure failure)
            {
                return new Result<string>.Failure(failure.Error);
            }

            var success = agentResult as Result<CrewAgent>.Success;
            var result = await success!.Value.ExecuteAsync(task, cancellationToken);

            if (result is Result<string>.Success s)
            {
                Log(s.Value);
            }

            return result;
        }

        /// <summary>
        /// Delegate capability to agent
        /// </summary>
        public Result<Unit> DelegateCapability(string agentId, Capability capability)
        {
            var agentResult = GetAgent(agentId);
            if (agentResult is Result<CrewAgent>.Failure failure)
            {
                return new Result<Unit>.Failure(failure.Error);
            }

            var success = agentResult as Result<CrewAgent>.Success;
            success!.Value.DelegateCapability(capability);

            Log($"Delegated capability: {capability} to {agentId}");
            return new Result<Unit>.Success(new Unit());
        }

        /// <summary>
        /// Broadcast message to all agents
        /// </summary>
        public async Task<Result<Unit>> BroadcastAsync(
            string from,
            Dictionary<string, object?> content,
            CancellationToken cancellationToken = default)
        {
            var channelId = _channels.GetOrCreate("crew-broadcast");

            var message = new CrewMessage(
                From: from,
                To: "all",
                TimestampUtc: DateTimeOffset.UtcNow.ToUnixTimeMilliseconds(),
                MessageType: CrewMessageType.Status,
                Content: content,
                CorrelationId: $"msg_{DateTimeOffset.UtcNow.ToUnixTimeMilliseconds()}");

            var result = await _channels.BroadcastAsync(message, cancellationToken);

            if (result is Result<Unit>.Success)
            {
                Log($"Broadcast from {from}: {string.Join(", ", content.Keys)}");
            }

            return result;
        }

        /// <summary>
        /// Create checkpoint
        /// </summary>
        public async Task<Result<string>> CheckpointAsync(
            CancellationToken cancellationToken = default)
        {
            var agentStates = new Dictionary<string, Dictionary<string, object?>>();

            foreach (var (id, agent) in _agents)
            {
                agentStates[id] = agent.GetState();
            }

            var result = await _checkpoints.CreateAsync(
                _crewId,
                agentStates,
                _context,
                cancellationToken);

            if (result is Result<string>.Success s)
            {
                Log($"Created checkpoint: {s.Value}");
            }

            return result;
        }

        /// <summary>
        /// Restore from checkpoint
        /// </summary>
        public async Task<Result<Unit>> RestoreAsync(
            string checkpointId,
            CancellationToken cancellationToken = default)
        {
            var result = await _checkpoints.RestoreAsync(checkpointId, cancellationToken);

            if (result is not Result<Checkpoint>.Success success)
            {
                return new Result<Unit>.Failure(
                    (result as Result<Checkpoint>.Failure)!.Error);
            }

            var checkpoint = success.Value;

            foreach (var (agentId, state) in checkpoint.AgentStates)
            {
                if (_agents.TryGetValue(agentId, out var agent))
                {
                    agent.SetState(state);
                }
            }

            foreach (var kvp in checkpoint.SharedContext)
            {
                _context[kvp.Key] = kvp.Value;
            }

            Log($"Restored from checkpoint: {checkpointId}");
            return new Result<Unit>.Success(new Unit());
        }

        /// <summary>
        /// Get crew summary
        /// </summary>
        public object GetSummary() => new
        {
            Id = _crewId,
            Name = _name,
            Description = _description,
            Members = _members.Count,
            ActiveAgents = _agents.Values.Select(a => a.GetStats()).ToList(),
        };

        /// <summary>
        /// Get execution log
        /// </summary>
        public IEnumerable<string> GetLog() => _executionLog.AsReadOnly();

        /// <summary>
        /// Internal logging
        /// </summary>
        private void Log(string message)
        {
            var timestamp = DateTime.UtcNow.ToString("O");
            _executionLog.Add($"[{timestamp}] {message}");
        }
    }

    /// <summary>
    /// Crew collaboration example
    /// </summary>
    public static class CrewCollaborationExample
    {
        public static async Task<int> MainAsync(string[] args)
        {
            try
            {
                Console.WriteLine("=== Crew Collaboration Example (C#) ===\n");

                // 1. Create crew
                var crew = new AgentCrew(
                    "research-crew-001",
                    "Research Crew",
                    "Coordinated research and analysis team");

                Console.WriteLine("✓ Created AgentCrew: Research Crew\n");

                // 2. Define roles and add members
                var researcherRole = new AgentRole(
                    Name: "Researcher",
                    Description: "Searches and gathers information",
                    Capabilities: new List<Capability> { Capability.Research },
                    Priority: AgentPriority.High);

                var analyzerRole = new AgentRole(
                    Name: "Analyzer",
                    Description: "Processes and analyzes findings",
                    Capabilities: new List<Capability> { Capability.Analysis },
                    Priority: AgentPriority.Normal);

                var reporterRole = new AgentRole(
                    Name: "Reporter",
                    Description: "Generates final reports",
                    Capabilities: new List<Capability> { Capability.Reporting },
                    Priority: AgentPriority.Normal);

                crew.AddMember("agent-researcher-001", researcherRole);
                crew.AddMember("agent-analyzer-001", analyzerRole);
                crew.AddMember("agent-reporter-001", reporterRole);

                Console.WriteLine("✓ Added crew members: Researcher, Analyzer, Reporter\n");

                // 3. Execute coordinated tasks
                Console.WriteLine("--- Executing Tasks ---\n");

                var searchResult = await crew.ExecuteTaskAsync(
                    "agent-researcher-001",
                    "Search for relevant papers and data");

                if (searchResult is not Result<string>.Success)
                {
                    return 1;
                }

                var analyzeResult = await crew.ExecuteTaskAsync(
                    "agent-analyzer-001",
                    "Analyze search results and identify patterns");

                if (analyzeResult is not Result<string>.Success)
                {
                    return 1;
                }

                var reportResult = await crew.ExecuteTaskAsync(
                    "agent-reporter-001",
                    "Generate final report from analysis");

                if (reportResult is not Result<string>.Success)
                {
                    return 1;
                }

                Console.WriteLine();

                // 4. Delegate capabilities
                Console.WriteLine("--- Delegating Capabilities ---\n");

                crew.DelegateCapability("agent-reporter-001", Capability.Analysis);

                Console.WriteLine();

                // 5. Create checkpoint
                Console.WriteLine("--- Creating Checkpoint ---\n");

                var cpResult = await crew.CheckpointAsync();

                if (cpResult is not Result<string>.Success cpSuccess)
                {
                    return 1;
                }

                var checkpointId = cpSuccess.Value;

                Console.WriteLine();

                // 6. Broadcast status message
                Console.WriteLine("--- Broadcasting Status ---\n");

                var broadcastResult = await crew.BroadcastAsync(
                    "agent-researcher-001",
                    new Dictionary<string, object?>
                    {
                        { "status", "workflow_complete" },
                        { "phase", "reporting" },
                    });

                if (broadcastResult is not Result<Unit>.Success)
                {
                    return 1;
                }

                Console.WriteLine();

                // 7. Restore from checkpoint
                Console.WriteLine("--- Restoring from Checkpoint ---\n");

                var restoreResult = await crew.RestoreAsync(checkpointId);

                if (restoreResult is not Result<Unit>.Success)
                {
                    return 1;
                }

                Console.WriteLine();

                // 8. Display final summary
                Console.WriteLine("--- Crew Summary ---\n");
                var summary = crew.GetSummary();
                Console.WriteLine(System.Text.Json.JsonSerializer.Serialize(
                    summary,
                    new System.Text.Json.JsonSerializerOptions { WriteIndented = true }));

                Console.WriteLine("\n--- Execution Log ---\n");
                foreach (var logEntry in crew.GetLog())
                {
                    Console.WriteLine(logEntry);
                }

                Console.WriteLine("\n✓ Crew collaboration example completed successfully");
                return 0;
            }
            catch (Exception ex)
            {
                Console.WriteLine($"Fatal error: {ex.Message}");
                return 1;
            }
        }
    }
}
