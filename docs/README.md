# XKernal Documentation

## Structure

### `/architecture/`
High-level architecture overview of the 4-layer Cognitive Substrate OS.

### `/design-documents/`
Weekly engineering deliverables organized by crate. Each document contains detailed technical specifications, code examples, audit results, and benchmarks produced during the 36-week development cycle.

Browse by layer:
- `kernel/` — L0 Microkernel (ct_lifecycle, capability_engine, ipc_signals_exceptions)
- `services/` — L1 Services (semantic_memory, gpu_accelerator, tool_registry_telemetry)
- `runtime/` — L2 Runtime (framework_adapters, semantic_fs_agent_lifecycle)
- `sdk/` — L3 SDK (csci, ts-sdk, dotnet-sdk, tools)

### `/implementation-plan/`
The complete 36-week implementation plan with per-engineer, per-week objectives across all 10 engineering streams.

## Key Documents

| Document | Description |
|----------|-------------|
| [Progress Report](../XKernal-Progress-Report.md) | Master tracking document for all 310 deliverables |
| [CSCI Specification](../sdk/csci/) | Cognitive Substrate Calling Interface v1.0 |
| [Architecture Overview](architecture/OVERVIEW.md) | 4-layer system architecture |
