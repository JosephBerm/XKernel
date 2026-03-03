/**
 * # Crew Collaboration Example
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
 * @module examples/crew_collaboration
 */

// ============================================================================
// Types & Interfaces (extending basic_agent)
// ============================================================================

/**
 * Branded types for strong typing
 */
type AgentId = string & { readonly __brand: "AgentId" };
type TaskId = string & { readonly __brand: "TaskId" };
type CrewId = string & { readonly __brand: "CrewId" };
type ChannelId = string & { readonly __brand: "ChannelId" };
type CheckpointId = string & { readonly __brand: "CheckpointId" };

function createAgentId(id: string): AgentId {
    return id as AgentId;
}

function createTaskId(id: string): TaskId {
    return id as TaskId;
}

function createCrewId(id: string): CrewId {
    return id as CrewId;
}

function createChannelId(id: string): ChannelId {
    return id as ChannelId;
}

function createCheckpointId(id: string): CheckpointId {
    return id as CheckpointId;
}

/**
 * Result type for operations
 */
type Result<T, E = Error> = { ok: true; value: T } | { ok: false; error: E };

function Ok<T>(value: T): Result<T> {
    return { ok: true, value };
}

function Err<E>(error: E): Result<never, E> {
    return { ok: false, error };
}

/**
 * Agent capability enumeration
 */
type Capability = "research" | "analysis" | "reporting" | "coordination";

/**
 * Agent role specification
 */
interface AgentRole {
    name: string;
    description: string;
    capabilities: Capability[];
    priority: "low" | "normal" | "high";
}

/**
 * Message for inter-agent communication
 */
interface CrewMessage {
    from: AgentId;
    to: AgentId | "all";
    timestamp: number;
    message_type: "task" | "result" | "query" | "status";
    content: Record<string, unknown>;
    correlation_id: string;
}

/**
 * Checkpoint for state persistence
 */
interface Checkpoint {
    id: CheckpointId;
    crew_id: CrewId;
    timestamp: number;
    agent_states: Map<AgentId, unknown>;
    shared_context: Record<string, unknown>;
}

/**
 * Crew member definition
 */
interface CrewMember {
    id: AgentId;
    role: AgentRole;
    active: boolean;
}

/**
 * Crew configuration and state
 */
interface CrewConfig {
    id: CrewId;
    name: string;
    description: string;
    members: CrewMember[];
    shared_channels: Map<ChannelId, CrewMessage[]>;
    context: Record<string, unknown>;
}

// ============================================================================
// Core Crew Implementation
// ============================================================================

/**
 * Manages inter-crew communication
 */
class CrewChannelManager {
    private channels: Map<ChannelId, CrewMessage[]> = new Map();

    /**
     * Create or get a channel
     */
    getOrCreate(channel_id: string): ChannelId {
        const id = createChannelId(channel_id);
        if (!this.channels.has(id)) {
            this.channels.set(id, []);
        }
        return id;
    }

    /**
     * Broadcast message to all agents
     */
    async broadcast(msg: CrewMessage): Promise<Result<void>> {
        if (msg.to !== "all") {
            return Err(new Error("Broadcast requires to: 'all'"));
        }

        for (const [, queue] of this.channels) {
            queue.push(msg);
        }

        return Ok(void 0);
    }

    /**
     * Send direct message
     */
    async send(msg: CrewMessage, channel_id: ChannelId): Promise<Result<void>> {
        const queue = this.channels.get(channel_id);
        if (!queue) {
            return Err(new Error(`Channel not found: ${channel_id}`));
        }

        queue.push(msg);
        return Ok(void 0);
    }

    /**
     * Receive messages for an agent
     */
    async receive(
        channel_id: ChannelId,
        agent_id: AgentId,
    ): Promise<Result<CrewMessage[]>> {
        const queue = this.channels.get(channel_id);
        if (!queue) {
            return Err(new Error(`Channel not found: ${channel_id}`));
        }

        const messages = queue.filter((m) => m.to === agent_id || m.to === "all");
        return Ok(messages);
    }

    /**
     * Clear messages for an agent
     */
    async clearMessages(
        channel_id: ChannelId,
        agent_id: AgentId,
    ): Promise<Result<number>> {
        const queue = this.channels.get(channel_id);
        if (!queue) {
            return Err(new Error(`Channel not found: ${channel_id}`));
        }

        const initial_length = queue.length;
        const remaining = queue.filter((m) => m.to !== agent_id && m.to !== "all");
        this.channels.set(channel_id, remaining);

        return Ok(initial_length - remaining.length);
    }
}

/**
 * Checkpoint manager for state persistence
 */
class CheckpointManager {
    private checkpoints: Map<CheckpointId, Checkpoint> = new Map();

    /**
     * Create a checkpoint
     */
    async create(
        crew_id: CrewId,
        agent_states: Map<AgentId, unknown>,
        context: Record<string, unknown>,
    ): Promise<Result<CheckpointId>> {
        const checkpoint_id = createCheckpointId(
            `cp_${crew_id}_${Date.now()}`,
        );

        const checkpoint: Checkpoint = {
            id: checkpoint_id,
            crew_id,
            timestamp: Date.now(),
            agent_states: new Map(agent_states),
            shared_context: { ...context },
        };

        this.checkpoints.set(checkpoint_id, checkpoint);
        return Ok(checkpoint_id);
    }

    /**
     * Restore from checkpoint
     */
    async restore(
        checkpoint_id: CheckpointId,
    ): Promise<Result<Checkpoint>> {
        const checkpoint = this.checkpoints.get(checkpoint_id);
        if (!checkpoint) {
            return Err(new Error(`Checkpoint not found: ${checkpoint_id}`));
        }

        return Ok({
            ...checkpoint,
            agent_states: new Map(checkpoint.agent_states),
        });
    }

    /**
     * List checkpoints for a crew
     */
    async listCheckpoints(crew_id: CrewId): Promise<Result<CheckpointId[]>> {
        const ids = Array.from(this.checkpoints.entries())
            .filter(([, cp]) => cp.crew_id === crew_id)
            .map(([id]) => id);

        return Ok(ids);
    }

    /**
     * Delete a checkpoint
     */
    async delete(checkpoint_id: CheckpointId): Promise<Result<void>> {
        if (!this.checkpoints.has(checkpoint_id)) {
            return Err(new Error(`Checkpoint not found: ${checkpoint_id}`));
        }

        this.checkpoints.delete(checkpoint_id);
        return Ok(void 0);
    }
}

/**
 * Specialized crew member agent
 */
class CrewAgent {
    private id: AgentId;
    private role: AgentRole;
    private state: Record<string, unknown> = {};
    private delegated_tasks: string[] = [];

    constructor(id: AgentId, role: AgentRole) {
        this.id = id;
        this.role = role;
    }

    /**
     * Get agent ID
     */
    getId(): AgentId {
        return this.id;
    }

    /**
     * Get agent role
     */
    getRole(): AgentRole {
        return this.role;
    }

    /**
     * Check if agent has a capability
     */
    hasCapability(capability: Capability): boolean {
        return this.role.capabilities.includes(capability);
    }

    /**
     * Execute a task
     */
    async execute(task: string): Promise<Result<string>> {
        // Simulate task execution based on role
        await new Promise((resolve) => setTimeout(resolve, 100));

        this.state["last_task"] = task;
        this.delegated_tasks.push(task);

        const result = `[${this.role.name}] Completed: ${task}`;
        return Ok(result);
    }

    /**
     * Accept delegated capability
     */
    delegateCapability(capability: Capability): Result<void> {
        if (!this.role.capabilities.includes(capability)) {
            this.role.capabilities.push(capability);
        }
        return Ok(void 0);
    }

    /**
     * Get agent state
     */
    getState(): Record<string, unknown> {
        return { ...this.state };
    }

    /**
     * Set agent state
     */
    setState(state: Record<string, unknown>): void {
        this.state = state;
    }

    /**
     * Get statistics
     */
    getStats() {
        return {
            id: this.id,
            role: this.role.name,
            tasks_completed: this.delegated_tasks.length,
            capabilities: this.role.capabilities.length,
        };
    }
}

/**
 * AgentCrew coordinator
 */
class AgentCrew {
    private config: CrewConfig;
    private agents: Map<AgentId, CrewAgent> = new Map();
    private channels: CrewChannelManager;
    private checkpoints: CheckpointManager;
    private execution_log: string[] = [];

    constructor(
        crew_id: CrewId,
        name: string,
        description: string,
    ) {
        this.config = {
            id: crew_id,
            name,
            description,
            members: [],
            shared_channels: new Map(),
            context: {},
        };
        this.channels = new CrewChannelManager();
        this.checkpoints = new CheckpointManager();
    }

    /**
     * Add member to crew
     */
    addMember(
        id: AgentId,
        role: AgentRole,
    ): Result<void> {
        if (this.agents.has(id)) {
            return Err(new Error(`Agent already exists: ${id}`));
        }

        const agent = new CrewAgent(id, role);
        this.agents.set(id, agent);

        const member: CrewMember = {
            id,
            role,
            active: true,
        };

        this.config.members.push(member);
        this.log(`Added member: ${role.name} (${id})`);

        return Ok(void 0);
    }

    /**
     * Get agent by ID
     */
    getAgent(id: AgentId): Result<CrewAgent> {
        const agent = this.agents.get(id);
        if (!agent) {
            return Err(new Error(`Agent not found: ${id}`));
        }
        return Ok(agent);
    }

    /**
     * Execute task with agent
     */
    async executeTask(agent_id: AgentId, task: string): Promise<Result<string>> {
        const agent_result = this.getAgent(agent_id);
        if (!agent_result.ok) {
            return Err(agent_result.error);
        }

        const result = await agent_result.value.execute(task);
        if (result.ok) {
            this.log(result.value);
        }

        return result;
    }

    /**
     * Delegate capability to agent
     */
    delegateCapability(agent_id: AgentId, capability: Capability): Result<void> {
        const agent_result = this.getAgent(agent_id);
        if (!agent_result.ok) {
            return Err(agent_result.error);
        }

        agent_result.value.delegateCapability(capability)?;
        this.log(`Delegated capability: ${capability} to ${agent_id}`);

        return Ok(void 0);
    }

    /**
     * Broadcast message to all agents
     */
    async broadcast(
        from: AgentId,
        content: Record<string, unknown>,
    ): Promise<Result<void>> {
        const channel = this.channels.getOrCreate("crew-broadcast");

        const msg: CrewMessage = {
            from,
            to: "all",
            timestamp: Date.now(),
            message_type: "status",
            content,
            correlation_id: `msg_${Date.now()}`,
        };

        const result = await this.channels.broadcast(msg);
        if (result.ok) {
            this.log(`Broadcast from ${from}: ${JSON.stringify(content)}`);
        }

        return result;
    }

    /**
     * Create checkpoint
     */
    async checkpoint(): Promise<Result<CheckpointId>> {
        const agent_states = new Map<AgentId, unknown>();

        for (const [id, agent] of this.agents) {
            agent_states.set(id, agent.getState());
        }

        const result = await this.checkpoints.create(
            this.config.id,
            agent_states,
            this.config.context,
        );

        if (result.ok) {
            this.log(`Created checkpoint: ${result.value}`);
        }

        return result;
    }

    /**
     * Restore from checkpoint
     */
    async restore(checkpoint_id: CheckpointId): Promise<Result<void>> {
        const result = await this.checkpoints.restore(checkpoint_id);
        if (!result.ok) {
            return Err(result.error);
        }

        const cp = result.value;

        for (const [agent_id, state] of cp.agent_states) {
            const agent = this.agents.get(agent_id);
            if (agent && typeof state === "object" && state !== null) {
                agent.setState(state as Record<string, unknown>);
            }
        }

        this.config.context = cp.shared_context;
        this.log(`Restored from checkpoint: ${checkpoint_id}`);

        return Ok(void 0);
    }

    /**
     * Get crew summary
     */
    getSummary() {
        return {
            id: this.config.id,
            name: this.config.name,
            description: this.config.description,
            members: this.config.members.length,
            active_agents: Array.from(this.agents.values()).map((agent) =>
                agent.getStats(),
            ),
        };
    }

    /**
     * Get execution log
     */
    getLog(): string[] {
        return [...this.execution_log];
    }

    /**
     * Internal logging
     */
    private log(message: string): void {
        const timestamp = new Date().toISOString();
        this.execution_log.push(`[${timestamp}] ${message}`);
    }
}

// ============================================================================
// Example Execution
// ============================================================================

/**
 * Main example entry point
 */
async function main(): Promise<Result<void>> {
    try {
        console.log("=== Crew Collaboration Example ===\n");

        // 1. Create crew
        const crewId = createCrewId("research-crew-001");
        const crew = new AgentCrew(
            crewId,
            "Research Crew",
            "Coordinated research and analysis team",
        );
        console.log("✓ Created AgentCrew: Research Crew\n");

        // 2. Define roles and add members
        const researcherRole: AgentRole = {
            name: "Researcher",
            description: "Searches and gathers information",
            capabilities: ["research"],
            priority: "high",
        };

        const analyzerRole: AgentRole = {
            name: "Analyzer",
            description: "Processes and analyzes findings",
            capabilities: ["analysis"],
            priority: "normal",
        };

        const reporterRole: AgentRole = {
            name: "Reporter",
            description: "Generates final reports",
            capabilities: ["reporting"],
            priority: "normal",
        };

        const researcherId = createAgentId("agent-researcher-001");
        const analyzerId = createAgentId("agent-analyzer-001");
        const reporterId = createAgentId("agent-reporter-001");

        let result = crew.addMember(researcherId, researcherRole);
        if (!result.ok) return Err(result.error);

        result = crew.addMember(analyzerId, analyzerRole);
        if (!result.ok) return Err(result.error);

        result = crew.addMember(reporterId, reporterRole);
        if (!result.ok) return Err(result.error);

        console.log("✓ Added crew members: Researcher, Analyzer, Reporter\n");

        // 3. Execute coordinated tasks
        console.log("--- Executing Tasks ---\n");

        const search_result = await crew.executeTask(
            researcherId,
            "Search for relevant papers and data",
        );
        if (!search_result.ok) return Err(search_result.error);

        const analyze_result = await crew.executeTask(
            analyzerId,
            "Analyze search results and identify patterns",
        );
        if (!analyze_result.ok) return Err(analyze_result.error);

        const report_result = await crew.executeTask(
            reporterId,
            "Generate final report from analysis",
        );
        if (!report_result.ok) return Err(report_result.error);

        console.log("");

        // 4. Delegate capabilities
        console.log("--- Delegating Capabilities ---\n");

        const delegate_result = crew.delegateCapability(
            reporterId,
            "analysis",
        );
        if (!delegate_result.ok) return Err(delegate_result.error);

        console.log("");

        // 5. Create checkpoint
        console.log("--- Creating Checkpoint ---\n");

        const cp_result = await crew.checkpoint();
        if (!cp_result.ok) return Err(cp_result.error);
        const checkpoint_id = cp_result.value;

        console.log("");

        // 6. Broadcast status message
        console.log("--- Broadcasting Status ---\n");

        const broadcast_result = await crew.broadcast(
            researcherId,
            {
                status: "workflow_complete",
                phase: "reporting",
            },
        );
        if (!broadcast_result.ok) return Err(broadcast_result.error);

        console.log("");

        // 7. Restore from checkpoint
        console.log("--- Restoring from Checkpoint ---\n");

        const restore_result = await crew.restore(checkpoint_id);
        if (!restore_result.ok) return Err(restore_result.error);

        console.log("");

        // 8. Display final summary
        console.log("--- Crew Summary ---\n");
        const summary = crew.getSummary();
        console.log(JSON.stringify(summary, null, 2));

        console.log("\n--- Execution Log ---\n");
        const log = crew.getLog();
        log.forEach((entry) => console.log(entry));

        console.log("\n✓ Crew collaboration example completed successfully");
        return Ok(void 0);
    } catch (e) {
        return Err(e instanceof Error ? e : new Error(String(e)));
    }
}

// Run if executed directly
if (require.main === module) {
    main()
        .then((result) => {
            if (!result.ok) {
                console.error("Error: " + result.error.message);
                process.exit(1);
            }
        });
}

export {
    AgentCrew,
    CrewAgent,
    CrewChannelManager,
    CheckpointManager,
    AgentRole,
    CrewMessage,
    Checkpoint,
    CrewConfig,
    Capability,
    AgentId,
    CrewId,
    CheckpointId,
    Result,
};
