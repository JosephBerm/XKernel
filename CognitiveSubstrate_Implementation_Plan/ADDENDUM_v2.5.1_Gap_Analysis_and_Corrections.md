# Cognitive Substrate — Addendum v2.5.1

## External Review Gap Analysis, Research Validation, and Corrections

> **Date:** March 2026
> **Applies to:** Engineering Plan v2.5
> **Status:** APPROVED — Incorporate into all engineer weekly plans
> **Classification:** Internal — Research Division

---

## Executive Summary

An external AI model review identified 6 potential gaps in the Engineering Plan v2.5. We validated each claim against the document itself and against current research (SOSP 2025 proceedings, OpenTelemetry standards, seL4 documentation, GPU driver architecture literature). Our findings:

| Gap | Reviewer Claim | Our Verdict | Action |
|-----|---------------|-------------|--------|
| **1. Benchmark methodology** | No baselines, no metric definitions | **PARTIALLY VALID** — Metrics defined, methodology missing | Add benchmark methodology spec |
| **2. GPU reality plan** | No GPU targets, no driver plan | **PARTIALLY VALID + CRITICAL CORRECTION** — LithOS/PhoenixOS run on existing drivers, not bare metal | Revise GPU strategy to two-phase approach |
| **3. CSCI ABI spec** | No frozen spec | **VALID but EXPECTED** — ABI specs are separate documents (seL4, POSIX model) | Already planned as Phase 2 deliverable; add explicit ABI doc milestone |
| **4. Policy language** | No DSL defined | **PARTIALLY VALID** — seL4 uses capDL; we should too | Add capDL-inspired policy DSL to roadmap |
| **5. Observability schema** | No event schema | **PARTIALLY VALID** — OpenTelemetry GenAI v1.37+ exists as standard | Align CEF with OpenTelemetry GenAI Semantic Conventions |
| **6. Developer UX** | No concrete specs | **OVERSTATED** — Tools, SDKs, migration described; specs are Phase 2-3 deliverables | No architectural change; add format spec milestones |

---

## Correction 1: GPU Driver Strategy (CRITICAL)

### The Problem

Section 3.3.2 states: *"The kernel communicates with the GPU via MMIO registers and command submission queues, bypassing the CUDA/ROCm userspace driver stack."*

The document cites LithOS (SOSP 2025) and PhoenixOS (SOSP 2025) as validation. However, **research confirms both systems run on top of existing CUDA/ROCm driver stacks**:

- **LithOS** (CMU/Meta, SOSP 2025) implements TPC-granularity spatial scheduling and kernel atomization as an **OS-level scheduling layer on top of the existing GPU compute stack** — not a bare-metal GPU driver replacement. It achieves 13× lower tail latency by managing GPU resource allocation *above* the driver, not by replacing MMIO access.

- **PhoenixOS** (SJTU IPADS, SOSP 2025) implements concurrent GPU checkpoint/restore as an **OS service that intercepts CUDA API calls** — it relies on CUDA currently and plans ROCm/Ascend support. It does not perform direct MMIO register access.

### The Correction

The GPU strategy must be revised to a **two-phase approach**:

**Phase A (v1.0 — Weeks 1-36): Driver-Stack Abstraction Layer**

Instead of custom MMIO drivers, v1 builds an OS-level GPU management layer that:

- Runs as an L1 kernel service with privileged access to GPU driver APIs (CUDA Runtime/Driver API, ROCm HIP)
- Implements TPC-level spatial scheduling by controlling GPU context creation and kernel launch queuing (LithOS approach — validated at SOSP 2025)
- Implements kernel atomization by intercepting and splitting kernel launch calls at the API level (LithOS approach)
- Implements KV-cache isolation via GPU memory allocation control (allocating separate GPU memory pools per crew, enforced by the GPU Manager service)
- Implements concurrent checkpoint/restore via CUDA API interception (PhoenixOS approach)
- **v1 targets NVIDIA CUDA GPUs (H100/H200/B200 series)** as primary, with AMD ROCm (MI300 series) as stretch goal

This approach preserves ALL architectural benefits described in v2.5 (TPC scheduling, kernel atomization, KV-cache isolation, concurrent C/R) while being implementable within the 36-week timeline.

**Phase B (v2.0 — Post-GA): Native GPU Driver Interface**

The long-term vision of direct MMIO register access and custom command submission queues remains valid as a v2.0 goal. This requires:

- Custom GPU driver development (12-18 months additional)
- GPU vendor partnership or open-source driver leveraging (AMD's open ISA is more feasible than NVIDIA's proprietary stack)
- Formal verification of GPU driver isolation properties

### Impact on ADR-001

ADR-001 (Section 5.1) already identifies GPU driver feasibility as a Month 18 risk gate. This correction **strengthens** ADR-001 by providing a concrete v1 path that doesn't depend on custom driver development, while preserving the bare-metal driver as a v2 ambition.

### v1 GPU Architecture Targets

| GPU | API | Priority | Rationale |
|-----|-----|----------|-----------|
| NVIDIA H100/H200 | CUDA 12.x Driver API | P0 (Required) | Dominant AI training/inference GPU. Largest ecosystem. |
| NVIDIA B200 | CUDA 12.x Driver API | P0 (Required) | Next-gen data center GPU. Forward compatibility. |
| AMD MI300X | ROCm HIP | P1 (Stretch) | Open ISA. Growing adoption. Better long-term driver story. |

> **Doc Reference Update:** Section 3.3.2 should be amended to describe the two-phase GPU strategy. Section 5 should add ADR-002: GPU Driver Strategy — v1 Abstraction Layer vs v2 Native Driver.

---

## Correction 2: Benchmark Methodology Specification

### What v2.5 Already Has (Correctly)

The document defines 8 measurement dimensions with clear definitions and targets (Section 7), 4 reference workloads with concrete agent counts, and "vs Linux+Docker baseline" as the comparison point. The reviewer understated the existing coverage.

### What Must Be Added

**Percentile Definitions:**

All latency metrics must specify percentile targets:

| Metric | p50 | p95 | p99 | p99.9 |
|--------|-----|-----|-----|-------|
| IPC Latency (co-located) | < 500ns | < 1µs | < 5µs | < 50µs |
| Capability Check | < 50ns | < 100ns | < 200ns | < 1µs |
| Cold Start | < 30ms | < 50ms | < 100ms | < 500ms |
| Fault Recovery | < 50ms | < 100ms | < 250ms | < 1s |

**Baseline Deployment Specification:**

The Linux+Docker baseline must be precisely defined:

- **OS:** Ubuntu 24.04 LTS, kernel 6.8+
- **Container Runtime:** Docker 27.x with default seccomp profile
- **Agent Framework:** LangChain 0.3.x / Semantic Kernel 1.x (matching adapter versions)
- **GPU:** Same hardware as Cognitive Substrate test (NVIDIA H100 80GB)
- **Orchestration:** Docker Compose for single-node, Kubernetes 1.31 for multi-node
- **Measurement:** Linux `perf`, NVIDIA Nsight Systems, custom instrumentation via OpenTelemetry

**Statistical Rigor:**

- Minimum 100 runs per measurement point
- Report: mean, median (p50), p95, p99, standard deviation
- Confidence interval: 95%
- Warmup: 10 runs discarded before measurement
- Environment: Dedicated hardware, no contention, CPU governor set to performance

**Benchmark Harness:**

Engineer 10 will build a reproducible benchmark harness in `/benches/` that:

- Generates all 4 reference workloads deterministically from seed
- Deploys both Cognitive Substrate and Linux+Docker baseline configurations
- Runs measurement suite with statistical validation
- Produces comparison reports in Markdown and JSON

> **Doc Reference Update:** Section 7 should be amended with percentile definitions, baseline spec, and statistical methodology.

---

## Correction 3: CSCI ABI — Separate Specification Document

### Research Finding

seL4 maintains **three separate documents**: an abstract formal specification (generated from Isabelle/HOL), a reference manual, and architecture-specific ABI documentation. POSIX similarly separates the API standard from architecture-specific ABIs (System V ABI supplements). Redox OS uses structured TOML for syscall definitions.

### What This Means

The reviewer is correct that a frozen ABI spec is needed, but **wrong** that it should be in the architecture document. The v2.5 document is an **architectural design document**, not a syscall specification. The CSCI ABI specification is correctly planned as a Phase 2 deliverable (Week 18-22).

### Action: Add Explicit CSCI Spec Milestones

| Milestone | Week | Content |
|-----------|------|---------|
| CSCI v0.1 Draft | Week 2 | Syscall names, parameter intent, return type categories |
| CSCI v0.5 Internal | Week 15 | Full signatures in Rust, error code enumeration, capability requirements per syscall |
| CSCI v1.0 Published | Week 22 | Frozen ABI: calling conventions (x86-64 `syscall` / ARM64 `svc`), register allocation, struct layouts, error codes, versioning guarantees |
| CSCI Reference Manual | Week 30 | seL4-style reference with examples, error handling guide, security model per syscall |

The CSCI spec will follow the **Redox OS model** (structured definition file) with **seL4-style** documentation rigor.

> **Doc Reference Update:** Section 3.5.1 should reference the separate CSCI ABI specification document and its milestone timeline.

---

## Correction 4: Policy DSL — capDL-Inspired Design

### Research Finding

seL4 uses **capDL (Capability Distribution Language)**, a declarative DSL for specifying capability distributions. capDL enables formal verification of security properties ("Can component X ever access resource Y?"). This is established best practice for capability-based systems.

Capsicum (FreeBSD) takes a code-based approach but lacks centralized policy management — not suitable for an OS with mandatory access control.

### Action: Add Cognitive Policy Language (CPL) to Roadmap

The MandatoryCapabilityPolicy system (Section 2.10, 3.3.6) should be backed by a declarative DSL inspired by seL4's capDL:

**Cognitive Policy Language (CPL) — Design Principles:**

- Declarative, not imperative (like capDL, not like iptables)
- Formally verifiable — policy properties can be statically checked
- Hot-reloadable — policy changes without system restart
- Human-readable — auditors can review without code expertise

**Example CPL Policy (conceptual):**

```
policy production_db_access {
  scope: all_agents
  enforcement: deny
  rule: capability.target.type == "database"
        AND capability.target.tags contains "production"
        AND NOT agent.has_approval("human_admin")
  audit: always
  exception_requires: human_approval
}

policy pii_audit_mode {
  scope: all_agents
  enforcement: audit
  rule: capability.target.data_classification == "PII"
  audit: always
}

policy token_budget_limit {
  scope: all_agents
  enforcement: deny
  rule: ct.resource_budget.max_tokens > 10000
        AND NOT ct.crew.coordinator.has_approval("budget_override")
}
```

**CPL Milestones:**

| Milestone | Week | Owner |
|-----------|------|-------|
| CPL v0.1 Grammar + Parser | Week 8 | Engineer 2 |
| CPL Integration with Policy Engine | Week 13 | Engineer 2 + Engineer 6 |
| CPL Formal Verification (basic properties) | Week 20 | Engineer 2 |
| CPL Reference Documentation | Week 30 | Engineer 10 |

> **Doc Reference Update:** Section 2.10 and 3.3.6 should reference CPL and capDL as the policy language model.

---

## Correction 5: Observability — OpenTelemetry GenAI Alignment

### Research Finding

**OpenTelemetry GenAI Semantic Conventions (v1.37+)** is the emerging industry standard for AI agent observability. LangSmith, LangFuse, Arize Phoenix, and Datadog all support or converge on this standard. The convention defines standardized attributes for prompts, completions, tool calls, agent workflows, and token usage.

### Action: Align CEF with OpenTelemetry GenAI

The Cognitive Event Format (CEF) should be designed as a **superset** of OpenTelemetry GenAI Semantic Conventions:

**CEF Field Schema (Addendum):**

Every CEF event MUST include these base fields:

| Field | Type | Description |
|-------|------|-------------|
| `event_id` | ULID | Globally unique event identifier |
| `trace_id` | TraceID (128-bit) | **Correlation ID** — links all events in a reasoning chain (OpenTelemetry compatible) |
| `span_id` | SpanID (64-bit) | Parent span for nested operations |
| `ct_id` | ULID | CognitiveTask that generated this event |
| `agent_id` | AgentID | Agent that owns the CT |
| `crew_id` | Option<CrewID> | Crew context if applicable |
| `timestamp` | Timestamp (ns) | Nanosecond-precision event time |
| `event_type` | CEFEventType | One of 10 defined types |
| `phase` | CTPhase | CT phase when event occurred |
| `cost` | CostAttribution | { tokens, gpu_ms, wall_clock_ms, tpc_hours } |
| `data_classification` | Set<Tag> | PII tags, sensitivity labels (for taint tracking) |

**OpenTelemetry Compatibility:**

- CEF `trace_id` maps directly to OpenTelemetry `trace_id`
- CEF `span_id` maps to OpenTelemetry `span_id`
- CEF events can be exported as OpenTelemetry spans via a translation layer
- This enables integration with existing observability tools (Datadog, Grafana, Jaeger)

**Export API Contract:**

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/v1/events/stream` | WebSocket | Real-time event streaming with filter parameters |
| `/api/v1/events/query` | POST | Historical event query with time range, agent, crew, type filters |
| `/api/v1/events/export` | POST | Bulk export in JSON, Parquet, or OTLP (OpenTelemetry Protocol) format |
| `/api/v1/audit/verify` | POST | Verify Merkle-tree integrity for a time range |

> **Doc Reference Update:** Section 3.3.4 should reference OpenTelemetry GenAI alignment, add correlation ID design, and specify the export API contract.

---

## Correction 6: Developer UX — Format Specifications

### Assessment

The reviewer overstated this gap. The document describes 5 debugging tools with clear purposes (Section 3.5.4), detailed framework adapter mappings (Section 3.4.1), agent unit files with listed properties (Section 3.4.3), and SDKs with feature lists (Section 3.5.5). These are Phase 2-3 implementation deliverables, not architectural gaps.

### Action: Add Format Spec Milestones (No Architectural Change)

| Spec | Week | Owner | Format |
|------|------|-------|--------|
| Agent Unit File Schema | Week 11 | Engineer 8 | TOML with JSON Schema validation |
| cs-pkg Package Manifest | Week 15 | Engineer 10 | TOML (Cargo.toml-inspired) |
| CEF Event Schema | Week 12 | Engineer 6 | Protocol Buffers + JSON Schema |
| CPL Grammar | Week 8 | Engineer 2 | PEG grammar + ANTLR |
| CSCI ABI Spec | Week 22 | Engineer 9 | Structured Rust types + TOML (Redox OS model) |

> **Doc Reference Update:** Section 6 phase milestones should include these format spec deliverables.

---

## What the Reviewer Got WRONG

For completeness, here are claims we reject:

1. **"The doc lists ambitious performance goals but doesn't define throughput"** — FALSE. Section 7 explicitly defines "Completed CTs/sec at 10, 50, 100, 500 concurrent agents." This is a precise throughput definition.

2. **"Agent unit files exist conceptually but need full spec"** — OVERSTATED. Section 3.4.3 lists 8 specific properties. The format spec is a Phase 1 implementation deliverable, not an architectural gap.

3. **"Debugging tools need spec"** — OVERSTATED. Section 3.5.4 describes 5 tools with clear functional purposes and analogies (cs-trace = strace, cs-profile = perf). UI specifications are implementation details, not architecture.

4. **"CSCI needs to be in this document"** — WRONG. seL4 and POSIX both maintain separate ABI specification documents. An architectural design document is not an ABI spec.

5. **"You need a 'Spec Pack' of 10 documents"** — PARTIALLY WRONG. Several items in their "Spec Pack" (scheduler spec, memory service spec, threat model) are already covered in Sections 3.2.2, 3.3.1, and 8 respectively. The legitimate additions (benchmark harness, CSCI ABI, policy DSL, GPU integration spec, event schema) are incorporated in this addendum.

---

## Summary of Changes to Implementation Plans

| Engineer | Changes Required |
|----------|-----------------|
| **Engineer 1** (Scheduler) | Add benchmark percentile definitions to Week 25-28 plans |
| **Engineer 2** (Security) | Add CPL (Cognitive Policy Language) design in Week 8, capDL reference |
| **Engineer 3** (IPC/Checkpointing) | Add correlation ID (trace_id/span_id) to IPC events |
| **Engineer 4** (Memory) | Minor — add memory tier benchmark percentiles |
| **Engineer 5** (GPU) | **MAJOR** — Revise from bare-metal MMIO to v1 driver-stack abstraction layer |
| **Engineer 6** (Telemetry) | Add OpenTelemetry GenAI alignment, CEF field schema, export API spec |
| **Engineer 7** (Adapters) | Minor — add OpenTelemetry span translation at adapter boundary |
| **Engineer 8** (Lifecycle) | Add Agent Unit File TOML schema spec in Week 11 |
| **Engineer 9** (CSCI/SDKs) | Add explicit CSCI ABI doc milestones (v0.1/v0.5/v1.0) |
| **Engineer 10** (Tooling) | Add benchmark harness spec, CPL docs, export API docs |

---

*This addendum becomes part of the official Engineering Plan. All engineers should read this before their next weekly plan review.*
