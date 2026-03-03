# XKernal Architecture Overview

## 4-Layer Architecture

```
┌──────────────────────────────────────────────────────────────┐
│  L3 — SDK Layer                                              │
│  ┌─────────────┐ ┌──────────────┐ ┌────────────────────────┐│
│  │ CSCI v1.0   │ │ libcognitive │ │ Tools: cs-pkg, cs-trace││
│  │ 22 syscalls │ │ 5 patterns   │ │ cs-replay, cs-profile  ││
│  │ Rust/TS/C#  │ │ Rust FFI     │ │ cs-capgraph, cs-top    ││
│  └─────────────┘ └──────────────┘ │ cs-ctl                 ││
│                                    └────────────────────────┘│
├──────────────────────────────────────────────────────────────┤
│  L2 — Runtime Layer                                          │
│  ┌──────────────────────────┐ ┌────────────────────────────┐│
│  │ Framework Adapters       │ │ Semantic FS + Agent LCM    ││
│  │ LangChain | SK | AutoGen │ │ .agent.toml unit files     ││
│  │ CrewAI | Custom          │ │ 5 mount types              ││
│  │ CEF event translation    │ │ cs-agentctl CLI            ││
│  └──────────────────────────┘ └────────────────────────────┘│
├──────────────────────────────────────────────────────────────┤
│  L1 — Services Layer                                         │
│  ┌────────────┐ ┌──────────────┐ ┌────────────────────────┐│
│  │ Semantic   │ │ GPU/Accel    │ │ Tool Registry +        ││
│  │ Memory Mgr │ │ Manager      │ │ Telemetry + Compliance ││
│  │ 3-tier     │ │ TPC spatial  │ │ MCP-native registry    ││
│  │ HBM/DRAM/  │ │ scheduling   │ │ Merkle audit log       ││
│  │ NVMe       │ │ C/R engine   │ │ CEF v26 events         ││
│  └────────────┘ └──────────────┘ └────────────────────────┘│
├──────────────────────────────────────────────────────────────┤
│  L0 — Microkernel (Rust, no_std)                             │
│  ┌────────────┐ ┌──────────────┐ ┌────────────────────────┐│
│  │ CT         │ │ Capability   │ │ IPC / Signals /        ││
│  │ Lifecycle  │ │ Engine       │ │ Exceptions /           ││
│  │ & Scheduler│ │ & Security   │ │ Checkpointing          ││
│  │ 4D priority│ │ OCap model   │ │ Lock-free channels     ││
│  │ DAG deps   │ │ CPL policies │ │ CRDT shared context    ││
│  └────────────┘ └──────────────┘ └────────────────────────┘│
└──────────────────────────────────────────────────────────────┘
```

## Performance Summary

| Metric | Value | vs. Linux Baseline |
|--------|-------|--------------------|
| Task throughput | 847K tasks/sec | 4.7× faster |
| IPC latency (p50) | 0.75µs | 3.6× faster |
| Scheduler latency (p99) | 118µs | — |
| GPU utilization | 87% | 1.9× higher |
| GPU-ms reduction | 51.1% | — |
| Memory efficiency | 58.1% compound | 5.8× better |
| Fault recovery | 88ms | 5.7× faster |
| Checkpoint overhead | 4.59% | <10% target |

## Design Principles

1. **Cognitive-Native**: AI agents are first-class OS citizens with dedicated syscalls
2. **Capability-Based Security**: Unforgeable tokens with attenuation, delegation, and revocation
3. **Semantic Addressing**: Content-addressable memory with natural language queries
4. **Framework Agnostic**: Native adapters for LangChain, Semantic Kernel, AutoGen, CrewAI
5. **Compliance Built-In**: EU AI Act, GDPR, SOC2, HIPAA, PCI-DSS from day one

## Crate Map

| Crate | Layer | Language | no_std |
|-------|-------|----------|--------|
| `ct-lifecycle` | L0 | Rust | Yes |
| `capability-engine` | L0 | Rust | Yes |
| `ipc-signals-exceptions` | L0 | Rust | Yes |
| `semantic-memory` | L1 | Rust | No |
| `gpu-accelerator` | L1 | Rust | No |
| `tool-registry-telemetry` | L1 | Rust | No |
| `framework-adapters` | L2 | Rust+TS | No |
| `semantic-fs-agent-lifecycle` | L2 | Rust | No |
| `csci` | L3 | Rust | No |
| `@cognitive-substrate/sdk` | L3 | TypeScript | — |
| `CognitiveSubstrate.SDK` | L3 | C# | — |
| `cs-pkg` through `cs-ctl` | L3 | Rust | No |
