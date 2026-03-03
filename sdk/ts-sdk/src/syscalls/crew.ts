/**
 * Cognitive Substrate SDK - Crew Family Syscalls
 * 
 * Syscalls for multi-agent coordination:
 * - crew_init (0x0700): Create a new crew
 * - crew_add (0x0701): Add an agent to a crew
 * - crew_remove (0x0702): Remove an agent from a crew
 * - crew_barrier (0x0703): Synchronize crew members at a barrier
 * 
 * Total: 4 syscalls
 */

import {
  CrewId,
  AgentId,
  CrewConfig,
  CsciError,
  CsciErrorCode,
} from '../index.js';

/**
 * Create a new crew (crew_init).
 * 
 * Creates a new crew with the specified name and configuration.
 * A crew is a collection of agents working together toward a common goal.
 * The creator becomes the coordinator for the crew.
 * 
 * Syscall number: 0x0700
 * 
 * @param name - Crew name
 * @param config - Crew configuration (mission, coordinator agent, initial members, budget)
 * @returns Promise resolving to the new crew ID
 * @throws {CsciError} with code EPERM if caller lacks capability
 * @throws {CsciError} with code ENOMEM if memory allocation fails
 * @throws {CsciError} with code EEXIST if crew with name already exists
 * 
 * @example
 * ```typescript
 * const crewId = await crew_init('analysis-team', {
 *   mission: 'Analyze data in parallel',
 *   coordinator_agent: createAgentId('coordinator-1'),
 *   initial_members: [createAgentId('worker-1'), createAgentId('worker-2')],
 *   collective_budget: 100 * 1024 * 1024
 * });
 * ```
 */
export async function crew_init(
  name: string,
  config: {
    mission: string;
    coordinator_agent: AgentId;
    initial_members?: AgentId[];
    collective_budget?: number;
  },
): Promise<CrewId> {
  throw new CsciError(
    CsciErrorCode.Unimplemented,
    'crew_init is not yet implemented',
  );
}

/**
 * Add an agent to a crew (crew_add).
 * 
 * Adds a new agent to an existing crew. The agent must be provided with
 * a configuration specifying its role and initial state within the crew.
 * 
 * Syscall number: 0x0701
 * 
 * @param crew_id - Crew ID to add agent to
 * @param agent_id - Agent ID to add
 * @param config - Agent configuration (role, initial state, etc.)
 * @returns Promise resolving when agent is added
 * @throws {CsciError} with code EPERM if caller lacks capability
 * @throws {CsciError} with code EEXIST if agent is already a crew member
 * @throws {CsciError} with code EFULL if crew is at maximum size
 * 
 * @example
 * ```typescript
 * await crew_add(
 *   crewId,
 *   createAgentId('worker-3'),
 *   { role: 'specialist', specialization: 'nlp' }
 * );
 * ```
 */
export async function crew_add(
  crew_id: CrewId,
  agent_id: AgentId,
  config?: Record<string, unknown>,
): Promise<void> {
  throw new CsciError(
    CsciErrorCode.Unimplemented,
    'crew_add is not yet implemented',
  );
}

/**
 * Remove an agent from a crew (crew_remove).
 * 
 * Removes an agent from a crew. After removal, the agent is no longer
 * considered a crew member and cannot participate in crew operations.
 * 
 * Syscall number: 0x0702
 * 
 * @param crew_id - Crew ID to remove agent from
 * @param agent_id - Agent ID to remove
 * @returns Promise resolving when agent is removed
 * @throws {CsciError} with code EPERM if caller lacks capability
 * @throws {CsciError} with code ENOENT if agent is not a crew member
 * 
 * @example
 * ```typescript
 * await crew_remove(crewId, createAgentId('worker-1'));
 * ```
 */
export async function crew_remove(
  crew_id: CrewId,
  agent_id: AgentId,
): Promise<void> {
  throw new CsciError(
    CsciErrorCode.Unimplemented,
    'crew_remove is not yet implemented',
  );
}

/**
 * Synchronize crew members at a barrier (crew_barrier).
 * 
 * Implements a barrier synchronization for all crew members. All members
 * must call this syscall and block until all have reached the barrier,
 * then all are released simultaneously. This is used for coordinated
 * checkpoints and phase transitions.
 * 
 * Syscall number: 0x0703
 * 
 * @param crew_id - Crew ID
 * @param timeout_ms - Optional timeout in milliseconds for barrier
 * @returns Promise resolving when barrier is released
 * @throws {CsciError} with code EPERM if caller is not a crew member
 * @throws {CsciError} with code ETIMEDOUT if barrier times out
 * 
 * @example
 * ```typescript
 * await crew_barrier(crewId, 30000); // 30 second timeout
 * ```
 */
export async function crew_barrier(
  crew_id: CrewId,
  timeout_ms?: number,
): Promise<void> {
  throw new CsciError(
    CsciErrorCode.Unimplemented,
    'crew_barrier is not yet implemented',
  );
}
