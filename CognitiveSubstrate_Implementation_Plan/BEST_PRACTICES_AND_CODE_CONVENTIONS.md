# Cognitive Substrate — Best Practices & Code Conventions

> **Classification:** Internal — Engineering Team
> **Version:** 1.1 (Updated per Addendum v2.5.1)
> **Date:** March 2026
> **Audience:** All 10 Staff Engineers
> **Reference Document:** Cognitive Substrate Engineering Plan v2.5 + Addendum v2.5.1

---

## 1. Language Standards

### 1.1 Rust (L0 Microkernel + L1 Kernel Services)

All kernel and kernel-service code MUST be written in Rust following these conventions:

- **Edition:** Rust 2024 (latest stable)
- **`#![no_std]`** for all L0 kernel code — no standard library, no heap allocator unless explicitly provided by the kernel's own allocator
- **`#![forbid(unsafe_code)]`** at the crate level for all code EXCEPT:
  - MMU/page table manipulation
  - Interrupt handlers
  - MMIO register access (L0 kernel only)
  - GPU device interface (CUDA Driver API / ROCm HIP FFI calls in L1 GPU Manager)
  - FFI boundaries (CSCI syscall entry, SDK FFI layer)
  - All `unsafe` blocks require a `// SAFETY:` comment explaining the invariant
- **Error handling:** Use `Result<T, CsError>` everywhere. Never `unwrap()` or `expect()` in kernel code. Use typed error enums per subsystem.
- **Naming:**
  - Types: `PascalCase` (e.g., `CognitiveTask`, `CapabilitySet`)
  - Functions: `snake_case` (e.g., `ct_spawn`, `cap_delegate`)
  - Constants: `SCREAMING_SNAKE_CASE` (e.g., `MAX_PHASE_ITERATIONS`)
  - Modules: `snake_case` matching the subsystem (e.g., `scheduler`, `capability_engine`)
- **Documentation:** Every public function, struct, and enum variant MUST have `///` doc comments. Include invariants, panics (if any), and examples.
- **Testing:** Every module has `#[cfg(test)] mod tests` with unit tests. Integration tests in `/tests/` directory.

> **Doc Reference:** Section 5 — Technology Decisions: "Kernel Language: Rust. Memory safety without GC. Ownership maps to capability semantics."

### 1.2 TypeScript (L2 Runtime + L3 SDK)

- **Strict mode:** `"strict": true` in `tsconfig.json`
- **No `any`:** Use `unknown` and type guards instead. ESLint rule: `@typescript-eslint/no-explicit-any: "error"`
- **Immutability by default:** Use `readonly` on all properties unless mutation is explicitly required
- **Naming:**
  - Interfaces: `PascalCase` prefixed with `I` only for service interfaces (e.g., `IFrameworkAdapter`)
  - Types: `PascalCase` (e.g., `CognitiveTaskConfig`, `AgentDefinition`)
  - Functions: `camelCase`
  - Constants: `SCREAMING_SNAKE_CASE`
- **Async/Await:** All I/O operations MUST use async/await. Never raw Promises with `.then()` chains
- **SDK Package:** `@cognitive-substrate/sdk` on npm

> **Doc Reference:** Section 3.5.5 — "TypeScript SDK: strongly-typed CSCI bindings with async/await, published as @cognitive-substrate/sdk on npm."

### 1.3 C# (L3 SDK)

- **Target:** .NET 8+ (LTS)
- **Nullable reference types:** Enabled project-wide
- **Naming:** Standard .NET conventions (PascalCase for public members, camelCase for private)
- **Async:** All I/O uses `async Task<T>` pattern
- **NuGet Package:** `CognitiveSubstrate.SDK`
- **Semantic Kernel Integration:** Follow SK plugin patterns for adapter compatibility

> **Doc Reference:** Section 3.5.5 — "C# SDK: enterprise SDK for .NET with Semantic Kernel integration, published on NuGet."

---

## 2. Domain Model Enforcement

### 2.1 The 12 Core Entities Are Sacred

Every engineer MUST be able to sketch the domain model from memory. The 12 entities are:

1. **CognitiveTask (CT)** — The fundamental schedulable unit (replaces POSIX process)
2. **Agent** — Persistent entity that spawns CognitiveTasks
3. **AgentCrew** — Group of agents with shared objective and resources
4. **Capability** — Unforgeable token of authority (seL4-inspired OCap)
5. **SemanticMemory** — Three-tier kernel-managed memory (L1/L2/L3)
6. **SemanticChannel** — Typed, intent-based IPC channel
7. **CognitiveException** — Typed exception hierarchy for cognitive failures
8. **CognitiveSignal** — Async kernel-to-agent notifications
9. **CognitiveCheckpoint** — Snapshot of CT's full cognitive state
10. **MandatoryCapabilityPolicy** — System-wide unoverridable capability rules
11. **ToolBinding** — Bridge between agents and external tools
12. **WatchdogConfig** — Per-CT reasoning watchdog configuration

> **Doc Reference:** Section 2 — Domain Model: "The type system the entire OS is built on. Every engineer should be able to sketch this model from memory."

### 2.2 Invariant Enforcement

All domain entity invariants MUST be enforced at compile time where possible, runtime where not:

- **CognitiveTask Invariants:**
  1. Capabilities always subset of parent Agent
  2. Budget cannot exceed parent quota
  3. Dependencies must complete before reason phase
  4. All phase transitions logged
  5. Dependency DAG is cycle-checked at spawn
  6. Watchdog enforces deadline and loop detection

> **Doc Reference:** Section 2.1 — CognitiveTask Invariants

- **Capability Invariants:**
  - Unforgeable — kernel-space handles only
  - Cryptographic signing ONLY at distributed trust boundaries
  - Local checks are O(1) handle lookups — target < 100ns

> **Doc Reference:** Section 2.4 — Capability: "Unforgeable kernel-space handle; cryptographic signing at trust boundaries only"

### 2.3 Type-Driven Design

- Use Rust's type system to make illegal states unrepresentable
- `CTPhase` is an enum: `Spawn | Plan | Reason | Act | Reflect | Yield | Complete | Failed`
- Phase transitions are validated at compile time via state machine patterns
- All IDs are strongly typed (no raw `String` or `u64` for IDs): `ULID` for CT, `AgentID` for agents, `CapID` for capabilities

---

## 3. Architecture Principles (Non-Negotiable)

Every PR review MUST trace design decisions back to these 8 principles:

| Principle | ID | Description |
|---|---|---|
| Agent-First, Human-Accessible | P1 | OS designed for agents. Human access via dedicated management interfaces. |
| Cognitive Primitives as Kernel Abstractions | P2 | Reasoning, memory, tool execution, IPC are kernel-level, not userland libraries. |
| Capability-Based Security from Day Zero | P3 | seL4/OCap inspired. Zero ambient authority. Fine-grained, revocable, auditable. |
| Semantic Over Syntactic | P4 | Structured, typed, semantic messages — not raw byte streams. |
| Observable by Default | P5 | Every cognitive operation traceable. Replay any reasoning chain from Cognitive Event Log. |
| Framework-Agnostic Agent Runtime | P6 | LangChain, AutoGen, CrewAI, SK all run as native CognitiveTasks. |
| Production-Grade from Phase 1 | P7 | Every component targets production workloads. No toy demos. |
| Fault-Tolerant by Design | P8 | Every operation checkpointable, every failure recoverable, every error typed. |

> **Doc Reference:** Section 1.2 — Design Principles: "Eight non-negotiable principles. Every architectural decision must trace to one or more."

---

## 4. Security Conventions

### 4.1 Capability-Based Access Control

- **Zero ambient authority:** Every agent starts with ZERO permissions
- **MMU-backed enforcement:** Memory regions mapped via page tables only if capability held
- **Six kernel operations:** Grant, Delegate, Revoke, Audit, Membrane, Policy Check
- **No ACLs, no RBAC, no POSIX permissions** — Capabilities ONLY

> **Doc Reference:** Section 3.2.3 — Capability Enforcement Engine

### 4.2 Mandatory Access Control

- All `MandatoryCapabilityPolicy` rules checked BEFORE page table mappings are created
- Policies are loaded at boot, hot-reloadable only by `policy_admin` capability holders
- Three enforcement modes: `deny`, `audit`, `warn`
- Exceptions require human approval to add

> **Doc Reference:** Section 2.10 — MandatoryCapabilityPolicy and Section 3.3.6 — Mandatory Policy Engine

### 4.3 KV-Cache Isolation

- Three modes: `STRICT` (separate physical pages per crew), `SELECTIVE` (isolation-by-default), `OPEN` (single-tenant only)
- Default: `STRICT` for regulated crews
- Performance SLO: SELECTIVE ≤ 10% p95 TTFT overhead for 13B-30B models

> **Doc Reference:** Section 3.3.2 — KV-Cache Isolation via Page Tables

---

## 5. IPC and Serialization Standards

- **Serialization:** Cap'n Proto for all IPC messages
- **Zero-copy for co-located agents:** Same physical pages mapped into both address spaces
- **Three patterns:** Request-Response, Publish-Subscribe, Shared Context (CRDT)
- **Distributed IPC:** Capability re-verification at network ingress/egress. Downgrade `exactly_once_local` to `at_least_once` with idempotency keys
- **Effect classes for tools:** `READ_ONLY | WRITE_REVERSIBLE | WRITE_COMPENSABLE | WRITE_IRREVERSIBLE`

> **Doc Reference:** Section 3.2.4 — Semantic IPC Subsystem

---

## 6. Testing Standards

### 6.1 Unit Tests

- Every Rust module: `#[cfg(test)] mod tests` block
- Every TypeScript module: co-located `.test.ts` file
- Every C# class: corresponding `*Tests.cs` in test project
- Minimum coverage target: 80% line coverage for all code

### 6.2 Integration Tests

- Cross-subsystem tests in `/tests/integration/`
- Test CT lifecycle end-to-end: spawn → plan → reason → act → reflect → complete
- Test fault scenarios: ContextOverflow, ToolCallFailed, BudgetExhausted, DeadlineExceeded
- Test checkpoint/restore cycle
- Test capability delegation chain

### 6.3 Benchmark Tests

- All benchmarks in `/benches/` using Rust `criterion` crate
- Track against 8 measurement dimensions from Section 7:
  - Multi-Agent Throughput, Inference Efficiency, Memory Efficiency, IPC Latency
  - Security Overhead, Cost Attribution, Cold Start, Fault Recovery

> **Doc Reference:** Section 7 — Benchmark Strategy

### 6.4 Adversarial Tests (Phase 3)

- Capability escalation attempts
- Memory leak detection under sustained load
- IPC flooding resilience
- Checkpoint corruption recovery
- Watchdog bypass attempts
- KV-cache side-channel testing

> **Doc Reference:** Section 6.4 — Phase 3, Weeks 28-32

---

## 7. Build System and CI/CD

- **Build:** Bazel (hermetic, multi-language, reproducible)
- **License:** Apache 2.0 — every source file MUST have license header
- **Monorepo structure:**
  ```
  /kernel/          — L0 Microkernel (Rust, #![no_std])
  /services/        — L1 Kernel Services (Rust)
  /runtime/         — L2 Agent Runtime (Rust + TypeScript)
  /sdk/             — L3 SDK Layer
    /csci/           — CSCI Specification
    /libcognitive/   — Standard Library
    /ts-sdk/         — TypeScript SDK
    /cs-sdk/         — C# SDK
    /cs-pkg/         — Package Manager
    /tools/          — Debugging Tools (cs-trace, cs-replay, etc.)
  /docs/            — Documentation Portal
  /tests/           — Cross-cutting integration tests
  /benches/         — Benchmark suite
  ```
- **CI pipeline:** Build → Lint → Unit Test → Integration Test → Benchmark (on merge)
- **PR requirements:** 2 approvals minimum, at least 1 from the owning stream, all tests pass

> **Doc Reference:** Section 5 — "Build System: Bazel. Hermetic, multi-language, reproducible. MAANG monorepo standard."

---

## 8. Telemetry and Observability

- **Every** cognitive operation emits a CEF (Cognitive Event Format) event
- CEF event types: `ThoughtStep`, `ToolCallRequested`, `ToolCallCompleted`, `PolicyDecision`, `MemoryAccess`, `IPCMessage`, `PhaseTransition`, `CheckpointCreated`, `SignalDispatched`, `ExceptionRaised`
- Cost attribution metadata on every event: tokens, GPU-ms, wall-clock, TPC-hours
- Framework adapters translate native formats to CEF at the adapter boundary
- **Cognitive Core Dumps** on CT failure: full checkpoint, reasoning chain, context window, tool history, exception context

> **Doc Reference:** Section 3.3.4 — Cognitive Telemetry Engine

---

## 9. Exception Handling Standards

- **Every exception is typed** — never use generic errors
- **8 exception types** with defined severity and default handlers:
  - Recoverable: `ContextOverflow`, `HallucinationDetected`, `ToolCallFailed`, `ReasoningDiverged`
  - Non-recoverable: `CapabilityExpired`, `BudgetExhausted`, `DeadlineExceeded`
  - Fatal: `DependencyCycleDetected`
- **Custom handlers** return one of: `Retry`, `Rollback(checkpoint_id)`, `Escalate(supervisor_ref)`, `Terminate(partial_results)`
- **Never swallow exceptions** — every exception MUST be logged via telemetry

> **Doc Reference:** Section 2.7 — CognitiveException and Section 3.2.6 — Cognitive Exception Engine

---

## 10. Git Workflow

- **Branching:** `main` (production) → `develop` (integration) → `feature/<stream>/<description>`
- **Commit messages:** Conventional Commits format: `feat(kernel): add CT phase transition validation`
- **Stream prefixes:** `kernel:`, `services:`, `runtime:`, `sdk:`
- **Weekly tags:** `phase-N-week-M` at each weekly milestone
- **ADR process:** Any architectural disagreement → written ADR in `/docs/adrs/` with rationale

> **Doc Reference:** Section 11 — Immediate Next Steps: "Resolve disagreements through ADR process."

---

## 11. Cross-Stream Collaboration

- **Daily standups** per stream
- **Weekly cross-stream sync** (all 10 engineers)
- **Cross-cutting ownership:**
  - Security: Kernel-owned, all-reviewed
  - Telemetry: Services-owned, all-consumed
  - Domain Model: Shared language, all-maintained
  - CSCI: SDK-owned, all-implemented

> **Doc Reference:** Section 4 — Engineering Team Structure

---

## 12. Performance Targets

| Metric | p50 Target | p95 Target | p99 Target |
|---|---|---|---|
| Multi-Agent Throughput (100+ agents) | 3-5× improvement vs Linux+Docker | — | — |
| Inference Efficiency (GPU-ms per chain) | 30-60% reduction | — | — |
| Memory Efficiency (per agent) | 40-60% reduction | — | — |
| IPC Latency (co-located agents) | < 500ns | < 1µs | < 5µs |
| Security Overhead (per syscall) | < 50ns | < 100ns | < 200ns |
| Cost Attribution Accuracy | > 99% | — | — |
| Cold Start (agent → first CT) | < 30ms | < 50ms | < 100ms |
| Fault Recovery (exception → resume) | < 50ms | < 100ms | < 250ms |

**Benchmark Statistical Rigor (Addendum v2.5.1):**

- Minimum 100 runs per measurement point
- Report: mean, median (p50), p95, p99, standard deviation
- Confidence interval: 95%
- Warmup: 10 runs discarded before measurement
- Linux+Docker baseline: Ubuntu 24.04 LTS, Docker 27.x, same GPU hardware

> **Doc Reference:** Section 7 — Benchmark Strategy + Addendum v2.5.1 Correction 2

---

## 13. Compliance Requirements

- **EU AI Act:** Articles 12, 18, 19, 26(6) compliance built-in
- **Two-tier retention:** Operational (7 days, verbatim) + Compliance (≥ 6 months, metadata)
- **Technical documentation:** 10-year retention (Article 18)
- **GDPR:** PII redacted/encrypted after processing purpose ends
- **Audit logs:** Merkle-tree-based, tamper-evident, append-only
- **Taint tracking:** PII tags propagate through capability system

> **Doc Reference:** Section 3.3.5 — Compliance Engine

---

## 14. GPU Strategy — Two-Phase Approach (Addendum v2.5.1)

**CRITICAL:** The GPU Manager uses a two-phase approach validated by research.

**Phase A (v1.0):** The GPU Manager is an L1 kernel service that uses **CUDA Driver API / ROCm HIP** — not custom MMIO drivers. This is validated by LithOS and PhoenixOS (both SOSP 2025), which achieve their results on top of existing driver stacks.

- TPC-level scheduling via CUDA context control and kernel launch queuing
- Kernel atomization via API-level kernel launch interception (not PTX modification)
- KV-cache isolation via GPU memory allocation pools per crew
- GPU checkpoint/restore via CUDA API interception
- **v1 Targets:** NVIDIA H100/H200/B200 (CUDA 12.x) as P0, AMD MI300X (ROCm HIP) as P1

**Phase B (v2.0, post-GA):** Native GPU driver with direct MMIO register access — long-term ambition, not blocking v1.

> **Doc Reference:** Addendum v2.5.1 — Correction 1: GPU Driver Strategy

---

## 15. Cognitive Policy Language (CPL) (Addendum v2.5.1)

MandatoryCapabilityPolicies are written in **CPL**, a declarative DSL inspired by seL4's capDL:

- Declarative, not imperative — formally verifiable
- Hot-reloadable without system restart
- Compiled to fast-path decision tables for O(1) repeated grant lookups

> **Doc Reference:** Addendum v2.5.1 — Correction 4: Policy DSL

---

## 16. Observability — OpenTelemetry Alignment (Addendum v2.5.1)

CEF events are designed as a **superset of OpenTelemetry GenAI Semantic Conventions (v1.37+)**:

- Every event includes `trace_id` (128-bit) and `span_id` (64-bit) for correlation
- CEF events translatable to OpenTelemetry spans for Datadog/Grafana/Jaeger integration
- Export via `/api/v1/events/export` supports JSON, Parquet, and OTLP formats

> **Doc Reference:** Addendum v2.5.1 — Correction 5: Observability

---

## 17. Document Reference Quick-Lookup

When implementing any feature, ALWAYS refer back to the Engineering Plan v2.5:

| Topic | Document Section |
|---|---|
| Design Principles (P1-P8) | Section 1.2 |
| Domain Model (12 entities) | Section 2 |
| Core Architecture (4 layers) | Section 3 |
| Microkernel (L0) | Section 3.2 |
| Boot Sequence | Section 3.2.1 |
| Cognitive Priority Scheduler | Section 3.2.2 |
| Capability Enforcement | Section 3.2.3 |
| Semantic IPC | Section 3.2.4 |
| Signal Dispatch | Section 3.2.5 |
| Exception Engine | Section 3.2.6 |
| Checkpointing Engine | Section 3.2.7 |
| Reasoning Watchdog | Section 3.2.8 |
| Kernel Services (L1) | Section 3.3 |
| Semantic Memory Manager | Section 3.3.1 |
| GPU/Accelerator Manager | Section 3.3.2 |
| Tool Registry | Section 3.3.3 |
| Telemetry Engine | Section 3.3.4 |
| Compliance Engine | Section 3.3.5 |
| Mandatory Policy Engine | Section 3.3.6 |
| Agent Runtime (L2) | Section 3.4 |
| Framework Adapters | Section 3.4.1 |
| Semantic File System | Section 3.4.2 |
| Agent Lifecycle Manager | Section 3.4.3 |
| SDK Layer (L3) | Section 3.5 |
| CSCI Specification | Section 3.5.1 |
| libcognitive | Section 3.5.2 |
| cs-pkg | Section 3.5.3 |
| Debugging Tools | Section 3.5.4 |
| TypeScript & C# SDKs | Section 3.5.5 |
| Documentation Portal | Section 3.5.6 |
| Team Structure | Section 4 |
| Technology Decisions | Section 5 |
| Phased Roadmap | Section 6 |
| Benchmark Strategy | Section 7 |
| Risks & Mitigations | Section 8 |

---

*This document is a living guide. Update it via PR as conventions evolve. Every PR reviewer should check compliance with these standards.*
