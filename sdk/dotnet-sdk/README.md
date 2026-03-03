# Cognitive Substrate C# SDK

**Status:** Week 5 Complete - All 22 CSCI v0.1 Syscall Stubs  
**Version:** 0.1.0  
**License:** Apache-2.0  

## Overview

The Cognitive Substrate C# SDK (`CognitiveSubstrate.SDK`) provides type-safe, async-first bindings for all 22 CSCI v0.1 syscalls. Built with nullable reference types and async Task patterns, it enables .NET applications to interact with the Cognitive Substrate OS.

## Features

- **Nullable Reference Types**: Full type safety with null checking enabled
- **Async-First Design**: All syscalls use async Task patterns for seamless integration
- **22 Syscalls**: Complete coverage across 8 families
- **Comprehensive XMLDoc**: Detailed documentation for every syscall
- **Error Handling**: Type-safe error codes matching POSIX conventions
- **Branded Types**: Record structs for IDs (TaskID, AgentID, etc.)
- **.NET 8.0**: Modern C# language features

## Installation

Via NuGet:

```bash
dotnet add package CognitiveSubstrate.SDK
```

or in your `.csproj`:

```xml
<ItemGroup>
  <PackageReference Include="CognitiveSubstrate.SDK" Version="0.1.0" />
</ItemGroup>
```

## Quick Start

```csharp
using CognitiveSubstrate.SDK;
using CognitiveSubstrate.SDK.Syscalls;
using CognitiveSubstrate.SDK.Types;

class Program
{
    static async Task Main(string[] args)
    {
        try
        {
            // Spawn a new cognitive task
            var taskId = await TaskSyscalls.CtSpawnAsync(
                AgentId.Create("agent-1"),
                new TaskConfig { Name = "my-task", TimeoutMs = 30000, Priority = 100 },
                new[] { "memory", "ipc", "tool" },
                new ResourceBudget { MemoryBytes = 10 * 1024 * 1024, CpuMs = 60000 }
            );
            
            Console.WriteLine($"Task spawned: {taskId}");
        }
        catch (CsciException ex)
        {
            Console.Error.WriteLine($"CSCI Error ({ex.Code}): {ex.Message}");
        }
    }
}
```

## Architecture

### 8 Syscall Families (22 Total Syscalls)

#### Task Family (4 syscalls)
- `CtSpawnAsync(0x0000)` - Create a new task
- `CtYieldAsync(0x0001)` - Voluntarily yield task execution
- `CtCheckpointAsync(0x0002)` - Create a state checkpoint
- `CtResumeAsync(0x0003)` - Resume task from checkpoint

#### Memory Family (4 syscalls)
- `MemAllocAsync(0x0100)` - Allocate a memory region
- `MemFreeAsync(0x0101)` - Free a memory region
- `MemMountAsync(0x0102)` - Mount a memory region at a path
- `MemUnmountAsync(0x0103)` - Unmount a memory region

#### Tool Family (2 syscalls)
- `ToolInvokeAsync(0x0200)` - Invoke an external tool
- `ToolBindAsync(0x0201)` - Bind a tool to the namespace

#### Channel/IPC Family (3 syscalls)
- `ChCreateAsync(0x0300)` - Create a communication channel
- `ChSendAsync(0x0301)` - Send a message on a channel
- `ChReceiveAsync(0x0302)` - Receive a message from a channel

#### Capability/Security Family (3 syscalls)
- `CapDelegateAsync(0x0500)` - Permanently transfer capabilities
- `CapGrantAsync(0x0501)` - Temporarily grant capabilities
- `CapRevokeAsync(0x0502)` - Revoke granted capabilities

#### Signals Family (2 syscalls)
- `SigSendAsync(0x0600)` - Send a signal to an agent/task
- `SigHandlerInstallAsync(0x0601)` - Install a signal handler

#### Crew Family (4 syscalls)
- `CrewInitAsync(0x0700)` - Create a new crew
- `CrewAddAsync(0x0701)` - Add an agent to a crew
- `CrewRemoveAsync(0x0702)` - Remove an agent from a crew
- `CrewBarrierAsync(0x0703)` - Synchronize crew members at a barrier

#### Telemetry Family (2 syscalls)
- `TelemetryTraceAsync(0x0800)` - Emit a telemetry event
- `TelemetrySnapshotAsync(0x0801)` - Capture system snapshot

## Type System

All IDs are record structs for compile-time type safety:

```csharp
using CognitiveSubstrate.SDK.Types;

var taskId = CognitiveTaskId.Create("task-123");
var agentId = AgentId.Create("agent-1");

// Type-safe - compile-time checked
// CognitiveTaskId wrongId = agentId; // Compile error!
```

### Record Types

All configuration objects use C# records with init-only properties:

```csharp
var config = new TaskConfig 
{ 
    Name = "my-task",
    TimeoutMs = 5000,
    Priority = 100
};

var budget = new ResourceBudget
{
    MemoryBytes = 10 * 1024 * 1024,
    CpuMs = 60000,
    MaxChildren = 10
};
```

## Error Handling

All syscalls throw `CsciException` with codes matching POSIX conventions plus CSCI-specific codes:

```csharp
try
{
    var regionId = await MemorySyscalls.MemAllocAsync(1024 * 1024 * 1024 * 1024); // 1TB
}
catch (CsciException ex)
{
    switch (ex.Code)
    {
        case CsciErrorCode.OutOfMemory:
            Console.WriteLine("Memory allocation failed");
            break;
        case CsciErrorCode.PermissionDenied:
            Console.WriteLine("Insufficient capabilities");
            break;
        case CsciErrorCode.InvalidArgument:
            Console.WriteLine("Invalid size or alignment");
            break;
    }
}
```

## Namespace Organization

```
CognitiveSubstrate.SDK
├── CognitiveSubstrateSDK         # Main entry point
├── CsciErrorCode                 # Error code enumeration
├── CsciException                 # Exception class
├── Types/
│   ├── CognitiveTaskId           # Branded types
│   ├── AgentId
│   ├── MemoryRegionId
│   ├── ... (8 total ID types)
│   ├── TaskConfig                # Configuration records
│   ├── ResourceBudget
│   ├── ... (8 configuration types)
│   ├── CheckpointType            # Enumerations
│   ├── YieldHint
│   ├── ChannelProtocol
│   └── ... (4 total enumerations)
└── Syscalls/
    ├── TaskSyscalls              # Task family (4 syscalls)
    ├── MemorySyscalls            # Memory family (4 syscalls)
    ├── IpcSyscalls               # Channel family (3 syscalls)
    ├── SecuritySyscalls          # Capability family (3 syscalls)
    ├── ToolSyscalls              # Tool family (2 syscalls)
    ├── SignalSyscalls            # Signals family (2 syscalls)
    ├── CrewSyscalls              # Crew family (4 syscalls)
    └── TelemetrySyscalls         # Telemetry family (2 syscalls)
```

## Project Configuration

### Target Framework

- **.NET 8.0** or later
- C# 12 language features
- Nullable reference types enabled
- Implicit using statements

### Build Settings

```xml
<PropertyGroup>
  <TargetFramework>net8.0</TargetFramework>
  <LangVersion>latest</LangVersion>
  <Nullable>enable</Nullable>
  <ImplicitUsings>enable</ImplicitUsings>
  <GenerateDocumentationFile>true</GenerateDocumentationFile>
</PropertyGroup>
```

## Usage Examples

### Spawning a Task

```csharp
var taskId = await TaskSyscalls.CtSpawnAsync(
    AgentId.Create("coordinator-agent"),
    new TaskConfig { Name = "analysis-task", TimeoutMs = 120000 },
    new[] { "memory", "ipc", "tool" },
    new ResourceBudget { MemoryBytes = 50 * 1024 * 1024 }
);
```

### Memory Allocation and Mounting

```csharp
// Allocate a 1MB memory region
var regionId = await MemorySyscalls.MemAllocAsync(
    size: 1024 * 1024,
    alignment: 4096
);

// Mount it in the namespace
await MemorySyscalls.MemMountAsync(
    regionId,
    "/memory/workspace"
);

// Later: unmount and free
await MemorySyscalls.MemUnmountAsync(regionId);
await MemorySyscalls.MemFreeAsync(regionId);
```

### IPC Communication

```csharp
// Create a channel
var channelId = await IpcSyscalls.ChCreateAsync(
    new ChannelConfig 
    { 
        MaxMessageSize = 65536,
        BufferSize = 1024 * 1024,
        Protocol = ChannelProtocol.MessageBased
    }
);

// Send a message
var bytesSent = await IpcSyscalls.ChSendAsync(
    channelId,
    new MessagePayload 
    { 
        Type = "request",
        Data = new { query = "hello", id = 123 }
    },
    flags: SendFlags.Default,
    timeoutMs: 5000
);

// Receive a message
var (message, bytes) = await IpcSyscalls.ChReceiveAsync(
    channelId,
    timeoutMs: 10000
);
```

### Capability Management

```csharp
// Grant temporary capabilities
var grantHandle = await SecuritySyscalls.CapGrantAsync(
    AgentId.Create("worker-agent"),
    new CapabilitySet { Capabilities = new[] { "memory", "ipc" } },
    durationMs: 60000 // 60 seconds
);

// Later: revoke the grant
await SecuritySyscalls.CapRevokeAsync(
    grantHandle,
    reason: "task completed"
);

// Delegate permanent capabilities
await SecuritySyscalls.CapDelegateAsync(
    AgentId.Create("trusted-service"),
    new CapabilitySet { Capabilities = new[] { "tool", "telemetry" } }
);
```

## Roadmap

### Week 5 (Current) ✓
- All 22 CSCI v0.1 syscall stubs
- Complete type system with branded types
- Error handling framework
- Comprehensive XMLDoc documentation

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
dotnet build
```

### Test

```bash
dotnet test
```

### Package

```bash
dotnet pack --configuration Release
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
