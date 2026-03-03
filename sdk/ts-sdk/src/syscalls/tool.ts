/**
 * Cognitive Substrate SDK - Tool Family Syscalls
 * 
 * Syscalls for external tool integration:
 * - tool_invoke (0x0200): Invoke an external tool
 * - tool_bind (0x0201): Bind a tool to the namespace
 * 
 * Total: 2 syscalls
 */

import {
  ToolResult,
  ToolInvokeConfig,
  ToolBindConfig,
  CsciError,
  CsciErrorCode,
} from '../index.js';

/**
 * Invoke an external tool (tool_invoke).
 * 
 * Invokes an external tool with the specified arguments in a sandboxed
 * execution environment. The tool runs to completion and returns a result.
 * 
 * Syscall number: 0x0200
 * 
 * @param tool_name - Name of the tool to invoke
 * @param args - Tool arguments (optional)
 * @param sandbox_config - Sandbox configuration (optional, memory/timeout limits)
 * @returns Promise resolving to the tool execution result
 * @throws {CsciError} with code EPERM if caller lacks tool capability
 * @throws {CsciError} with code ENOENT if tool does not exist
 * @throws {CsciError} with code EPOLICY if invocation violates policy
 * @throws {CsciError} with code ETOOLERR if tool execution fails
 * 
 * @example
 * ```typescript
 * const result = await tool_invoke(
 *   'python-interpreter',
 *   { script: 'print("hello")' },
 *   { timeout_ms: 5000, memory_limit: 100 * 1024 * 1024 }
 * );
 * ```
 */
export async function tool_invoke(
  tool_name: string,
  args?: Record<string, unknown>,
  sandbox_config?: Record<string, unknown>,
): Promise<ToolResult> {
  throw new CsciError(
    CsciErrorCode.Unimplemented,
    'tool_invoke is not yet implemented',
  );
}

/**
 * Bind a tool to the namespace (tool_bind).
 * 
 * Binds an external tool to a path in the agent's namespace, making it
 * accessible for invocation. The tool must have the required capabilities
 * granted to it.
 * 
 * Syscall number: 0x0201
 * 
 * @param tool_name - Name of the tool to bind
 * @param namespace_path - Path in namespace to bind tool to
 * @param capabilities - Capabilities required by the tool
 * @returns Promise resolving when binding completes
 * @throws {CsciError} with code EPERM if caller lacks capability
 * @throws {CsciError} with code ENOENT if tool does not exist
 * @throws {CsciError} with code EEXIST if namespace path is already occupied
 * 
 * @example
 * ```typescript
 * await tool_bind(
 *   'python-interpreter',
 *   '/tools/python',
 *   ['memory', 'ipc']
 * );
 * ```
 */
export async function tool_bind(
  tool_name: string,
  namespace_path: string,
  capabilities: string[],
): Promise<void> {
  throw new CsciError(
    CsciErrorCode.Unimplemented,
    'tool_bind is not yet implemented',
  );
}
