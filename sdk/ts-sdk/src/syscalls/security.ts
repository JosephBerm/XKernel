/**
 * Cognitive Substrate SDK - Capability (Security) Family Syscalls
 * 
 * Syscalls for capability-based security:
 * - cap_delegate (0x0500): Permanently transfer capabilities
 * - cap_grant (0x0501): Temporarily grant capabilities
 * - cap_revoke (0x0502): Revoke granted capabilities
 * 
 * Total: 3 syscalls
 */

import {
  AgentId,
  GrantHandle,
  CapabilitySet,
  CapabilityDelegateConfig,
  CapabilityGrantConfig,
  CsciError,
  CsciErrorCode,
} from '../index.js';

/**
 * Permanently delegate capabilities (cap_delegate).
 * 
 * Permanently transfers a set of capabilities to a recipient agent.
 * Once delegated, the capabilities belong to the recipient and cannot be
 * revoked by the grantor (except through separate revocation mechanisms).
 * 
 * Syscall number: 0x0500
 * 
 * @param recipient_id - Agent ID to receive the capabilities
 * @param capability_set - Set of capabilities to delegate
 * @param config - Delegation configuration (optional additional parameters)
 * @returns Promise resolving when delegation completes
 * @throws {CsciError} with code EPERM if caller lacks capability
 * @throws {CsciError} with code ENOENT if recipient does not exist
 * @throws {CsciError} with code ECYCLE if delegation would create a cycle
 * 
 * @example
 * ```typescript
 * await cap_delegate(
 *   createAgentId('agent-2'),
 *   { capabilities: ['memory', 'ipc'] }
 * );
 * ```
 */
export async function cap_delegate(
  recipient_id: AgentId,
  capability_set: CapabilitySet,
  config?: Record<string, unknown>,
): Promise<void> {
  throw new CsciError(
    CsciErrorCode.Unimplemented,
    'cap_delegate is not yet implemented',
  );
}

/**
 * Temporarily grant capabilities (cap_grant).
 * 
 * Temporarily grants a set of capabilities to a recipient agent for a
 * specified duration. The grant can be revoked at any time before expiration.
 * This is the primary mechanism for short-term capability sharing.
 * 
 * Syscall number: 0x0501
 * 
 * @param recipient_id - Agent ID to receive the capabilities
 * @param capability_set - Set of capabilities to grant
 * @param duration_ms - Duration of the grant in milliseconds
 * @param config - Grant configuration (optional additional parameters)
 * @returns Promise resolving to the grant handle (for revocation)
 * @throws {CsciError} with code EPERM if caller lacks capability
 * @throws {CsciError} with code ENOENT if recipient does not exist
 * @throws {CsciError} with code EINVAL if duration is invalid
 * 
 * @example
 * ```typescript
 * const grantHandle = await cap_grant(
 *   createAgentId('agent-3'),
 *   { capabilities: ['ipc'] },
 *   60000 // 60 seconds
 * );
 * ```
 */
export async function cap_grant(
  recipient_id: AgentId,
  capability_set: CapabilitySet,
  duration_ms: number,
  config?: Record<string, unknown>,
): Promise<GrantHandle> {
  throw new CsciError(
    CsciErrorCode.Unimplemented,
    'cap_grant is not yet implemented',
  );
}

/**
 * Revoke granted capabilities (cap_revoke).
 * 
 * Revokes a previously granted set of capabilities. This syscall only affects
 * grants created with cap_grant; delegated capabilities cannot be revoked
 * (except through other means such as capability revocation hierarchies).
 * 
 * Syscall number: 0x0502
 * 
 * @param grant_handle - Grant handle from cap_grant syscall
 * @param reason - Optional reason for revocation
 * @returns Promise resolving when revocation completes
 * @throws {CsciError} with code EPERM if caller lacks capability
 * @throws {CsciError} with code ENOENT if grant handle does not exist
 * 
 * @example
 * ```typescript
 * await cap_revoke(grantHandle, 'task completed');
 * ```
 */
export async function cap_revoke(
  grant_handle: GrantHandle,
  reason?: string,
): Promise<void> {
  throw new CsciError(
    CsciErrorCode.Unimplemented,
    'cap_revoke is not yet implemented',
  );
}
