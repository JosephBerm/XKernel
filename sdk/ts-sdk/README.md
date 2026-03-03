# Cognitive Substrate TypeScript SDK

**Status:** Week 5 Complete - All 22 CSCI v0.1 Syscall Stubs  
**Version:** 0.1.0  
**License:** Apache-2.0  

## Overview

The Cognitive Substrate TypeScript SDK (`@cognitive-substrate/sdk`) provides type-safe, async-first bindings for all 22 CSCI v0.1 syscalls. Built with strict mode TypeScript, it enables cognitive agents and services to interact with the Cognitive Substrate OS.

## Features

- **Strict Mode TypeScript**: Full type safety and null checking enabled
- **Async-First Design**: All syscalls return Promises for seamless integration with async/await
- **22 Syscalls**: Complete coverage across 8 families
- **Comprehensive JSDoc**: Detailed documentation for every syscall
- **Error Handling**: Type-safe error codes matching POSIX conventions
- **Branded Types**: Compile-time guarantees for IDs (TaskID, AgentID, etc.)

## Installation

```bash
npm install @cognitive-substrate/sdk
```

or

```bash
yarn add @cognitive-substrate/sdk
```

## Quick Start

```typescript
import {
  ct_spawn,
  createAgentId,
  createMemoryRegionId,
  ResourceBudget,
  CsciError,
  CsciErrorCode,
} from '@cognitive-substrate/sdk';

async function main() {
  try {
    // Spawn a new cognitive task
    const taskId = await ct_spawn(
      createAgentId('agent-1'),
      { name: 'my-task', timeout_ms: 30000, priority: 100 },
      ['memory', 'ipc', 'tool'],
      { memory_bytes: 10 * 1024 * 1024, cpu_ms: 60000 }
    );
    
    console.log('Task spawned:', taskId);
  } catch (error) {
    if (error instanceof CsciError) {
      console.error(`CSCI Error (${error.code}): ${error.message}`);
    } else {
      throw error;
    }
  }
}

main();
```

## Architecture

### 8 Syscall Families (22 Total Syscalls)

#### Task Family (4 syscalls)
- `ct_spawn(0x0000)` - Create a new task
- `ct_yield(0x0001)` - Voluntarily yield task execution
- `ct_checkpoint(0x0002)` - Create a state checkpoint
- `ct_resume(0x0003)` - Resume task from checkpoint

#### Memory Family (4 syscalls)
- `mem_alloc(0x0100)` - Allocate a memory region
- `mem_free(0x0101)` - Free a memory region
- `mem_mount(0x0102)` - Mount a memory region at a path
- `mem_unmount(0x0103)` - Unmount a memory region

#### Tool Family (2 syscalls)
- `tool_invoke(0x0200)` - Invoke an external tool
- `tool_bind(0x0201)` - Bind a tool to the namespace

#### Channel/IPC Family (3 syscalls)
- `ch_create(0x0300)` - Create a communication channel
- `ch_send(0x0301)` - Send a message on a channel
- `ch_receive(0x0302)` - Receive a message from a channel

#### Capability/Security Family (3 syscalls)
- `cap_delegate(0x0500)` - Permanently transfer capabilities
- `cap_grant(0x0501)` - Temporarily grant capabilities
- `cap_revoke(0x0502)` - Revoke granted capabilities

#### Signals Family (2 syscalls)
- `sig_send(0x0600)` - Send a signal to an agent/task
- `sig_handler_install(0x0601)` - Install a signal handler

#### Crew Family (4 syscalls)
- `crew_init(0x0700)` - Create a new crew
- `crew_add(0x0701)` - Add an agent to a crew
- `crew_remove(0x0702)` - Remove an agent from a crew
- `crew_barrier(0x0703)` - Synchronize crew members at a barrier

#### Telemetry Family (2 syscalls)
- `telemetry_trace(0x0800)` - Emit a telemetry event
- `telemetry_snapshot(0x0801)` - Capture system snapshot

## Type System

All IDs are branded types to prevent accidental mixing:

```typescript
import {
  CognitiveTaskId,
  AgentId,
  MemoryRegionId,
  ChannelId,
  GrantHandle,
  CheckpointId,
  CrewId,
  SignalHandlerId,
  createCognitiveTaskId,
  createAgentId,
  // ... etc
} from '@cognitive-substrate/sdk';

const taskId: CognitiveTaskId = createCognitiveTaskId('task-123');
const agentId: AgentId = createAgentId('agent-1');
// Compile error: Cannot assign AgentId to CognitiveTaskId
// const wrong: CognitiveTaskId = agentId;
```

## Error Handling

All syscalls throw `CsciError` with codes matching POSIX conventions plus CSCI-specific codes:

```typescript
import { CsciError, CsciErrorCode } from '@cognitive-substrate/sdk';

try {
  await mem_alloc(1024 * 1024 * 1024 * 1024); // 1TB
} catch (error) {
  if (error instanceof CsciError) {
    switch (error.code) {
      case CsciErrorCode.OutOfMemory:
        console.log('Memory allocation failed');
        break;
      case CsciErrorCode.PermissionDenied:
        console.log('Insufficient capabilities');
        break;
      case CsciErrorCode.InvalidArgument:
        console.log('Invalid size or alignment');
        break;
    }
  }
}
```

## Module Organization

```
src/
  index.ts           # Main barrel export
  errors.ts          # CsciError and CsciErrorCode
  types.ts           # All type definitions
  syscalls/
    index.ts         # Syscalls barrel export
    task.ts          # Task family (4 syscalls)
    memory.ts        # Memory family (4 syscalls)
    ipc.ts           # Channel/IPC family (3 syscalls)
    security.ts      # Capability family (3 syscalls)
    tool.ts          # Tool family (2 syscalls)
    signals.ts       # Signals family (2 syscalls)
    crew.ts          # Crew family (4 syscalls)
    telemetry.ts     # Telemetry family (2 syscalls)
```

## Configuration

### TypeScript Configuration

The SDK is built with the following TypeScript settings:

```json
{
  "compilerOptions": {
    "target": "ES2020",
    "module": "ES2020",
    "strict": true,
    "declaration": true,
    "sourceMap": true,
    "moduleResolution": "node"
  }
}
```

### Node Version

Requires Node.js 18.0.0 or higher for full ESM support.

## Roadmap

### Week 5 (Current) ✓
- All 22 CSCI v0.1 syscall stubs
- Complete type system
- Error handling framework
- Comprehensive JSDoc

### Week 6-7 (Planned)
- Kernel integration stubs
- IPC transport implementation
- ABI marshalling layer
- Unit tests for all syscalls

### Week 8+ (Planned)
- Full kernel binding implementation
- Performance optimization
- Advanced error recovery
- Example applications

## Development

### Build

```bash
npm run build       # Compile TypeScript
npm run type-check  # Type check only
npm run dev         # Watch mode
```

### Testing

```bash
npm run test        # Run tests with Vitest
npm run test:watch  # Watch mode
```

### Formatting

```bash
npm run format      # Format with Prettier
npm run lint        # Check with ESLint
```

## CSCI Specification

This SDK implements CSCI v0.1.0 as specified in:  
`sdk/csci/docs/csci_v0.1_specification.md`

Key design principles:
- **22 syscalls** across 8 families provide complete core functionality
- **POSIX-compatible error codes** for familiar error handling
- **x86-64 System V ABI** for efficient kernel integration
- **Capability-based security** with separate delegate/grant semantics
- **Forward compatibility** with structured configs for v0.2+ evolution

## Contributing

Contributions welcome! Please see CONTRIBUTING.md for guidelines.

## License

Apache-2.0 License - See LICENSE file for details

## Support

- Issues: https://github.com/cognitive-substrate/xkernal/issues
- Discussions: https://github.com/cognitive-substrate/xkernal/discussions
- Documentation: https://github.com/cognitive-substrate/xkernal/wiki
