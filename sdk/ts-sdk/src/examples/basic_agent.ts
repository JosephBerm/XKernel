/**
 * # Basic Agent Example
 *
 * Demonstrates the core Agent API with the following features:
 * - Creating a CognitiveTask
 * - Allocating SemanticMemory
 * - Spawning an agent
 * - Using channels for inter-process communication (IPC)
 * - Invoking a tool
 *
 * ## Architecture
 * 1. Define a task (search documents)
 * 2. Allocate memory for state management
 * 3. Register and invoke tools
 * 4. Send/receive messages via channels
 * 5. Demonstrate full async/await patterns
 *
 * @module examples/basic_agent
 */

// ============================================================================
// Types & Interfaces
// ============================================================================

/**
 * Branded type for task identifiers
 */
type TaskId = string & { readonly __brand: "TaskId" };

function createTaskId(id: string): TaskId {
    return id as TaskId;
}

/**
 * Branded type for memory addresses
 */
type MemoryAddress = number & { readonly __brand: "MemoryAddress" };

function createMemoryAddress(addr: number): MemoryAddress {
    return addr as MemoryAddress;
}

/**
 * Branded type for agent identifiers
 */
type AgentId = string & { readonly __brand: "AgentId" };

function createAgentId(id: string): AgentId {
    return id as AgentId;
}

/**
 * Branded type for channel identifiers
 */
type ChannelId = string & { readonly __brand: "ChannelId" };

function createChannelId(id: string): ChannelId {
    return id as ChannelId;
}

/**
 * Result type for operations that may fail
 */
type Result<T, E = Error> = { ok: true; value: T } | { ok: false; error: E };

function Ok<T>(value: T): Result<T> {
    return { ok: true, value };
}

function Err<E>(error: E): Result<never, E> {
    return { ok: false, error };
}

/**
 * Represents a cognitive task with metadata
 */
interface CognitiveTask {
    id: TaskId;
    name: string;
    description: string;
    priority: "low" | "normal" | "high";
    timeout_ms: number;
}

/**
 * Semantic memory allocation
 */
interface SemanticMemory {
    address: MemoryAddress;
    capacity_bytes: number;
    used_bytes: number;
    entries: Map<string, unknown>;
}

/**
 * Agent configuration
 */
interface AgentConfig {
    id: AgentId;
    task_id: TaskId;
    memory_address: MemoryAddress;
    max_iterations: number;
}

/**
 * Tool definition and invocation
 */
interface Tool {
    name: string;
    description: string;
    invoke: (input: string) => Promise<Result<string>>;
}

/**
 * Message for inter-process communication
 */
interface Message {
    from: AgentId;
    to: AgentId;
    timestamp: number;
    content: string;
    correlation_id: string;
}

/**
 * Channel for bidirectional communication
 */
interface Channel {
    id: ChannelId;
    send: (msg: Message) => Promise<Result<void>>;
    receive: () => Promise<Result<Message>>;
    close: () => Promise<Result<void>>;
}

// ============================================================================
// Core Agent Implementation
// ============================================================================

/**
 * Manages semantic memory for agent state
 */
class MemoryManager {
    private memory: SemanticMemory;
    private allocation_counter: number = 0;

    constructor(capacity_bytes: number) {
        this.memory = {
            address: createMemoryAddress(1000),
            capacity_bytes,
            used_bytes: 0,
            entries: new Map(),
        };
    }

    /**
     * Allocate memory for an entry
     */
    async allocate(key: string, value: unknown): Promise<Result<MemoryAddress>> {
        const estimated_size = JSON.stringify(value).length;

        if (this.memory.used_bytes + estimated_size > this.memory.capacity_bytes) {
            return Err(new Error("Insufficient memory"));
        }

        this.memory.entries.set(key, value);
        this.memory.used_bytes += estimated_size;
        this.allocation_counter += 1;

        const addr = createMemoryAddress(1000 + this.allocation_counter);
        return Ok(addr);
    }

    /**
     * Retrieve value from memory
     */
    async get(key: string): Promise<Result<unknown>> {
        if (this.memory.entries.has(key)) {
            return Ok(this.memory.entries.get(key));
        }
        return Err(new Error(`Key not found: ${key}`));
    }

    /**
     * Get memory statistics
     */
    getStats(): SemanticMemory {
        return { ...this.memory };
    }
}

/**
 * Channel implementation for IPC
 */
class SimpleChannel implements Channel {
    id: ChannelId;
    private queue: Message[] = [];
    private closed: boolean = false;

    constructor(channel_id: string) {
        this.id = createChannelId(channel_id);
    }

    async send(msg: Message): Promise<Result<void>> {
        if (this.closed) {
            return Err(new Error("Channel is closed"));
        }

        this.queue.push(msg);
        return Ok(void 0);
    }

    async receive(): Promise<Result<Message>> {
        if (this.closed && this.queue.length === 0) {
            return Err(new Error("Channel is closed"));
        }

        // In a real implementation, this would be async
        if (this.queue.length === 0) {
            return Err(new Error("No messages available"));
        }

        const msg = this.queue.shift()!;
        return Ok(msg);
    }

    async close(): Promise<Result<void>> {
        this.closed = true;
        return Ok(void 0);
    }
}

/**
 * Tool registry and executor
 */
class ToolRegistry {
    private tools: Map<string, Tool> = new Map();

    /**
     * Register a tool
     */
    register(tool: Tool): Result<void> {
        if (this.tools.has(tool.name)) {
            return Err(new Error(`Tool already registered: ${tool.name}`));
        }
        this.tools.set(tool.name, tool);
        return Ok(void 0);
    }

    /**
     * Invoke a tool by name
     */
    async invoke(tool_name: string, input: string): Promise<Result<string>> {
        const tool = this.tools.get(tool_name);
        if (!tool) {
            return Err(new Error(`Tool not found: ${tool_name}`));
        }
        return await tool.invoke(input);
    }

    /**
     * List all registered tools
     */
    listTools(): string[] {
        return Array.from(this.tools.keys());
    }
}

/**
 * Basic Agent implementation
 */
class BasicAgent {
    private config: AgentConfig;
    private memory: MemoryManager;
    private tools: ToolRegistry;
    private channels: Map<ChannelId, Channel> = new Map();
    private iteration_count: number = 0;
    private task_status: "idle" | "running" | "completed" | "failed" = "idle";

    constructor(
        config: AgentConfig,
        memory: MemoryManager,
        tools: ToolRegistry,
    ) {
        this.config = config;
        this.memory = memory;
        this.tools = tools;
    }

    /**
     * Register a channel for communication
     */
    registerChannel(channel: Channel): Result<void> {
        if (this.channels.has(channel.id)) {
            return Err(new Error(`Channel already registered: ${channel.id}`));
        }
        this.channels.set(channel.id, channel);
        return Ok(void 0);
    }

    /**
     * Execute the cognitive task
     */
    async execute(task: CognitiveTask): Promise<Result<string>> {
        try {
            this.task_status = "running";

            // Initialize task state in memory
            const initMemResult = await this.memory.allocate("task_state", {
                task_id: task.id,
                start_time: Date.now(),
                iterations: 0,
            });

            if (!initMemResult.ok) {
                this.task_status = "failed";
                return Err(initMemResult.error);
            }

            // Main execution loop
            while (this.iteration_count < this.config.max_iterations) {
                const iteration_result = await this.executeIteration(task);

                if (!iteration_result.ok) {
                    this.task_status = "failed";
                    return Err(iteration_result.error);
                }

                this.iteration_count += 1;

                // Update task state
                await this.memory.allocate("iteration_count", this.iteration_count);

                // Check for completion (simulated)
                if (this.iteration_count >= 2) {
                    break;
                }
            }

            this.task_status = "completed";
            return Ok(`Task ${task.id} completed with ${this.iteration_count} iterations`);
        } catch (e) {
            this.task_status = "failed";
            return Err(e instanceof Error ? e : new Error(String(e)));
        }
    }

    /**
     * Execute a single iteration
     */
    private async executeIteration(task: CognitiveTask): Promise<Result<void>> {
        // Example: invoke search tool
        const search_result = await this.tools.invoke(
            "search_documents",
            `Query for task ${task.id}`,
        );

        if (!search_result.ok) {
            return Err(search_result.error);
        }

        // Store results in memory
        const store_result = await this.memory.allocate(
            `iteration_${this.iteration_count}_result`,
            search_result.value,
        );

        if (!store_result.ok) {
            return Err(store_result.error);
        }

        // Send message through channel if registered
        if (this.channels.size > 0) {
            for (const [, channel] of this.channels) {
                const msg: Message = {
                    from: this.config.id,
                    to: createAgentId("observer"),
                    timestamp: Date.now(),
                    content: search_result.value,
                    correlation_id: `iter_${this.iteration_count}`,
                };

                const send_result = await channel.send(msg);
                if (!send_result.ok) {
                    console.warn(`Failed to send message: ${send_result.error}`);
                }
            }
        }

        return Ok(void 0);
    }

    /**
     * Get current task status
     */
    getStatus(): string {
        return `Agent ${this.config.id}: ${this.task_status} (${this.iteration_count} iterations)`;
    }

    /**
     * Get memory statistics
     */
    getMemoryStats(): SemanticMemory {
        return this.memory.getStats();
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
        console.log("=== Basic Agent Example ===\n");

        // 1. Create a cognitive task
        const taskId = createTaskId("search-documents-001");
        const task: CognitiveTask = {
            id: taskId,
            name: "Document Search",
            description: "Search and analyze documents",
            priority: "high",
            timeout_ms: 10000,
        };
        console.log(`Created task: ${task.name} (${task.id})`);

        // 2. Allocate semantic memory
        const memory = new MemoryManager(1024 * 1024); // 1MB
        console.log("Allocated semantic memory (1MB)\n");

        // 3. Register tools
        const tools = new ToolRegistry();

        const searchTool: Tool = {
            name: "search_documents",
            description: "Search documents by query",
            invoke: async (input: string): Promise<Result<string>> => {
                // Simulate tool execution
                await new Promise((resolve) => setTimeout(resolve, 100));
                return Ok(`Found documents matching: ${input}`);
            },
        };

        const analyzeTool: Tool = {
            name: "analyze_text",
            description: "Analyze text content",
            invoke: async (input: string): Promise<Result<string>> => {
                await new Promise((resolve) => setTimeout(resolve, 50));
                return Ok(`Analysis: ${input.length} chars`);
            },
        };

        if (!tools.register(searchTool).ok) {
            return Err(new Error("Failed to register search tool"));
        }

        if (!tools.register(analyzeTool).ok) {
            return Err(new Error("Failed to register analyze tool"));
        }

        console.log("Registered tools: " + tools.listTools().join(", ") + "\n");

        // 4. Create channels for IPC
        const channel = new SimpleChannel("agent-channel-001");
        console.log("Created communication channel\n");

        // 5. Spawn agent
        const agentId = createAgentId("agent-001");
        const agentConfig: AgentConfig = {
            id: agentId,
            task_id: taskId,
            memory_address: createMemoryAddress(1000),
            max_iterations: 3,
        };

        const agent = new BasicAgent(agentConfig, memory, tools);

        if (!agent.registerChannel(channel).ok) {
            return Err(new Error("Failed to register channel"));
        }

        console.log(`Spawned agent: ${agentId}\n`);

        // 6. Execute task
        console.log("Starting task execution...");
        const exec_result = await agent.execute(task);

        if (!exec_result.ok) {
            return Err(exec_result.error);
        }

        console.log(`✓ ${exec_result.value}\n`);

        // 7. Display results
        console.log("Agent Status: " + agent.getStatus());
        const stats = agent.getMemoryStats();
        console.log(
            `Memory Usage: ${stats.used_bytes}/${stats.capacity_bytes} bytes\n`,
        );

        // 8. Cleanup
        const close_result = await channel.close();
        if (!close_result.ok) {
            console.warn("Warning: Channel close failed");
        }

        console.log("✓ Example completed successfully");
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
    CognitiveTask,
    SemanticMemory,
    AgentConfig,
    Tool,
    Message,
    Channel,
    BasicAgent,
    MemoryManager,
    SimpleChannel,
    ToolRegistry,
    TaskId,
    AgentId,
    ChannelId,
    MemoryAddress,
    Result,
};
