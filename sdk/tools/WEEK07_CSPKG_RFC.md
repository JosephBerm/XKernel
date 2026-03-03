# Week 7 Deliverable: cs-pkg Package Manager RFC (Phase 1)

**XKernal Cognitive Substrate — Engineer 10: Tooling, Packaging & Documentation**

---

## Executive Summary

This RFC defines Phase 1 of the **cs-pkg** package manager for XKernal's Cognitive Substrate OS. cs-pkg introduces a package ecosystem enabling agents to discover, install, and manage cognitive tools, framework adapters, agent templates, and policy packages. The design prioritizes cognitive-native lifecycle support, capability-based isolation, and cost transparency through metadata declarations.

**Key Deliverables:**
- Package format specification with cs-manifest.toml schema
- CSCI version compatibility resolution
- Capability requirements and escalation policies
- Cost metadata for resource accounting
- Registry backend architecture design
- CLI interface definition

---

## Problem Statement

The XKernal cognitive substrate currently lacks a standardized package ecosystem. This creates friction for agents attempting to:

1. **Discover and Install Tools** — No centralized registry for cognitive tools (summarizers, code generators, analysis engines)
2. **Manage Framework Adapters** — Ad-hoc integration of LangChain, Semantic Kernel, CrewAI, and other agent frameworks
3. **Share Agent Templates** — No mechanism for sharing pre-configured agent definitions with capability constraints
4. **Enforce Governance** — Absence of capability-based access control and cost metadata in tool distribution
5. **Handle Dependencies** — No version resolution or CSCI compatibility checking across packages

cs-pkg solves these problems by introducing a cognitive-native packaging system with capability isolation, version management, and cost transparency.

---

## Architecture Overview

### 1. Package Format Specification

The standard XKernal package directory structure:

```
my-cognitive-tool/
├── cs-manifest.toml          # Package metadata and CSCI requirements
├── src/
│   ├── lib.rs                # Rust implementation
│   ├── wasm_bindings.rs       # WebAssembly FFI bindings
│   └── ...
├── tests/
│   ├── unit_tests.rs
│   └── integration_tests.rs
├── docs/
│   ├── API.md
│   └── examples/
├── README.md
└── CHANGELOG.md
```

All packages are built as Rust crates targeting WebAssembly (wasm32-unknown-unknown) or native compilation with CSCI ABI compatibility.

---

### 2. cs-manifest.toml Schema

Complete manifest specification:

```toml
[package]
name = "my-cognitive-tool"
version = "1.0.0"
authors = ["Alice <alice@example.com>", "Bob <bob@example.com>"]
description = "High-performance summarization cognitive tool"
license = "Apache-2.0"
repository = "https://github.com/xkernal/my-cognitive-tool"
documentation = "https://docs.xkernal.io/my-cognitive-tool"
keywords = ["summarization", "nlp", "cognitive"]
package_type = "tool"  # tool | adapter | template | policy

[csci]
min_version = "1.0.0"
max_version = "2.0.0"
target_profiles = ["cpu", "gpu_cuda", "gpu_metal"]

[capabilities]
required = ["tool_invoke", "memory_allocate", "disk_read"]
optional = ["network_access", "capability_grant"]
max_concurrent_invocations = 10

[cost]
avg_inference_ms = 50
peak_memory_mb = 256
tool_latency_ms = 100
tpc_utilization_percent = 75
monthly_estimated_cost_usd = 0.15

[dependencies]
cognitive-stdlib = "0.5.0"
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.0", features = ["full"] }

[build]
build_target = "wasm"
optimization_level = 3
include_debuginfo = false
```

---

### 3. CSCI Version Compatibility

**Version Range Resolution:**

Packages declare minimum and maximum CSCI versions they support using semantic versioning:

```
Package A: min = 1.0.0, max = 2.5.0  → Supports 1.0.0, 1.1.x, 2.0.x, 2.5.0
Package B: min = 2.1.0, max = 2.9.0  → Supports 2.1.0, 2.5.0, 2.9.0
```

**Resolution Algorithm:**
1. Collect all package version constraints
2. Find intersection of compatible CSCI versions
3. Select highest available CSCI version in intersection
4. Validate capability matrix for selected version

**Compatibility Matrix Example:**

| CSCI Version | memory_allocate | tool_invoke | disk_read | network_access |
|--------------|-----------------|-------------|-----------|-----------------|
| 1.0.0        | ✓               | ✓           | ✓         | ✗              |
| 1.5.0        | ✓               | ✓           | ✓         | ✓ (sandboxed)  |
| 2.0.0        | ✓ (async)       | ✓ (async)   | ✓ (quota) | ✓ (mTLS)       |

---

### 4. Capability Requirements and Escalation

**Capability Types:**

```rust
pub enum Capability {
    ToolInvoke,              // Invoke other cognitive tools
    MemoryAllocate,          // Request heap allocation
    DiskRead,                // Read from persistent storage
    DiskWrite,               // Write to persistent storage
    NetworkAccess,           // Outbound network connections
    CapabilityGrant,         // Grant capabilities to sub-agents
    TimeExceed,              // Run beyond default timeout (10s)
    CPUIntensive,            // Access >= 4 CPU cores
    GPUAccess,               // GPU compute access
}
```

**Escalation Policy:**
- Required capabilities are enforced at tool invocation
- Optional capabilities are granted if available; tool operates in degraded mode if unavailable
- Capability grants create ACLs preventing unauthorized tool access
- Cost multipliers apply for premium capabilities (GPU 3x, NetworkAccess 1.5x)

---

### 5. Cost Metadata Format

Cost declarations enable agent resource accounting and budgeting:

```toml
[cost]
# Inference cost
avg_inference_ms = 50
peak_inference_ms = 200

# Memory footprint
peak_memory_mb = 256
persistent_storage_mb = 10

# Latency
tool_latency_ms = 100                    # Time to initialize tool
startup_overhead_ms = 25

# Resource utilization
tpc_utilization_percent = 75             # Tensor Processing Cluster %
cpu_cores_required = 2

# Monthly cost estimate (for billing/budgeting)
monthly_estimated_cost_usd = 0.15
cost_unit = "per_1m_invocations"
```

Agents use these metrics to:
- Estimate execution budgets before running tools
- Optimize tool selection based on cost constraints
- Generate cost reports for governance auditing

---

## Package Types

### Type 1: Tool Packages

Self-contained cognitive tools with well-defined invocation interfaces.

**Example: "text-summarizer" package**
- Input: Long document (text)
- Output: Summary (text) + confidence (float)
- Cost: 50ms inference, 128MB peak memory

### Type 2: Framework Adapters

Integrations bridging XKernal agents with external frameworks.

**Example: "langchain-adapter" package**
- Provides Rust FFI bindings to LangChain APIs
- Translates XKernal tool formats to LangChain AgentExecutor
- Manages lifecycle: init, configure, run, cleanup
- CSCI requirements: network_access, memory_allocate

### Type 3: Agent Templates

Pre-configured agent definitions with tool compositions and routing rules.

**Example: "research-agent-template" package**
- Includes: web-search tool, document-analyzer tool, summarizer tool
- Defines: tool invocation DAG, fallback strategies
- Declares: capability requirements (network_access for search)

### Type 4: Policy Packages

Governance and capability policies for multi-agent environments.

**Example: "enterprise-audit-policy" package**
- Enforces: tool capability audit logging
- Restricts: network_access to approved domains
- Implements: cost-per-agent quotas

---

## Registry Backend Architecture

### Storage Layer

**Content-Addressed Storage:**
- Each package version stored by SHA-256(package_bytes)
- Immutable: published packages cannot be modified
- Deduplication: identical package binaries share storage

```
registry/
├── packages/
│   ├── <sha256_hash_1>/
│   │   ├── metadata.json
│   │   ├── manifest.toml
│   │   └── package.tar.gz
│   └── <sha256_hash_2>/
├── index/
│   └── name-to-sha256.json
└── search/
    └── tags-index.json
```

### Indexing Layer

**Primary Index:** Package name → [sha256_hash, version, published_at]

**Search Indices:**
- By tag: "summarization" → [pkg1, pkg2, pkg3]
- By capability: "network_access" → [pkg_a, pkg_c]
- By cost range: "< 100ms_latency" → [pkg_x, pkg_y]
- By CSCI compatibility: "1.5.0" → [pkg_compatible_list]

**Scale Target:** 1000+ packages with <100ms search latency (Redis-backed)

### Version Resolution

**Algorithm: Constraint Satisfaction**

```
Input: Package dependency tree with version constraints
Output: Resolved dependency set or conflict error

Steps:
1. Build constraint graph (min/max CSCI versions)
2. Check satisfiability (timeout: 5s)
3. For conflicts: suggest compatible versions to user
4. Validate capability matrix for resolution
5. Return resolved manifest with pinned versions
```

### Authentication & Authorization

**Publish Permissions:**
- GPG-signed manifests required for package publication
- Author identity verified against registry
- Capability-based publish ACLs: only tool authors can publish tool packages

**Verification:**
- Signature validation on every registry read
- SHA-256 hash validation on package download
- Attestation: timestamp and author details

---

## CLI Interface

### Command Definitions

**cs-pkg init** — Create new package skeleton
```bash
$ cs-pkg init --name my-tool --type tool --authors "Alice <alice@example.com>"
Created package structure:
├── cs-manifest.toml
├── src/lib.rs
├── tests/
├── docs/
└── README.md
```

**cs-pkg build** — Compile package to WASM
```bash
$ cs-pkg build --target wasm32 --release
Compiling my-cognitive-tool v1.0.0
Finished release [wasm32] in 2.3s
Output: ./target/my-cognitive-tool-1.0.0.wasm
```

**cs-pkg publish** — Submit package to registry
```bash
$ cs-pkg publish --registry https://registry.xkernal.io
Signing manifest with GPG key...
Uploading my-cognitive-tool-1.0.0...
Published: https://registry.xkernal.io/packages/my-cognitive-tool/1.0.0
```

**cs-pkg install** — Download and integrate package
```bash
$ cs-pkg install text-summarizer@^1.0.0 --csci-version 1.5.0
Resolving dependencies...
Downloaded: text-summarizer 1.2.0
Verified: SHA-256 checksum
Installed to: ~/.xkernal/packages/text-summarizer/
```

**cs-pkg search** — Query registry
```bash
$ cs-pkg search --tag "summarization" --max-latency 100ms
1. text-summarizer (1.2.0) - avg_latency: 52ms
2. fast-abstractive-summarizer (0.8.0) - avg_latency: 78ms
$ cs-pkg search --capability "network_access" --min-csci 1.5.0
1. web-search-tool (2.1.0)
2. news-aggregator (1.0.5)
```

**cs-pkg info** — Display package details
```bash
$ cs-pkg info text-summarizer@1.2.0
Name: text-summarizer
Version: 1.2.0
Type: tool
CSCI: 1.0.0 - 2.5.0
Capabilities: tool_invoke, memory_allocate
Cost: 52ms inference, 128MB peak memory
Dependencies: cognitive-stdlib@0.5.0, serde@1.0
```

---

## Manifest Parsing (Rust Implementation)

Core data structures for manifest deserialization:

```rust
use serde::{Deserialize, Serialize};
use semver::VersionReq;

#[derive(Debug, Deserialize, Serialize)]
pub struct CognitiveManifest {
    pub package: PackageMetadata,
    pub csci: CSCIRequirements,
    pub capabilities: CapabilityDeclaration,
    pub cost: CostMetadata,
    pub dependencies: std::collections::HashMap<String, DependencySpec>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PackageMetadata {
    pub name: String,
    pub version: String,
    pub authors: Vec<String>,
    pub description: String,
    pub license: String,
    pub package_type: PackageType,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum PackageType {
    #[serde(rename = "tool")]
    Tool,
    #[serde(rename = "adapter")]
    Adapter,
    #[serde(rename = "template")]
    Template,
    #[serde(rename = "policy")]
    Policy,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CSCIRequirements {
    pub min_version: String,
    pub max_version: String,
    pub target_profiles: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CapabilityDeclaration {
    pub required: Vec<String>,
    pub optional: Vec<String>,
    pub max_concurrent_invocations: u32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CostMetadata {
    pub avg_inference_ms: u32,
    pub peak_memory_mb: u32,
    pub tool_latency_ms: u32,
    pub tpc_utilization_percent: u8,
    pub monthly_estimated_cost_usd: f64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DependencySpec {
    pub version: String,
    #[serde(default)]
    pub features: Vec<String>,
}

impl CognitiveManifest {
    pub fn from_toml_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let manifest = toml::from_str(&content)?;
        Ok(manifest)
    }

    pub fn validate_csci_compatibility(&self, target_csci: &str) -> Result<(), String> {
        let min = semver::Version::parse(&self.csci.min_version)
            .map_err(|e| format!("Invalid min_version: {}", e))?;
        let max = semver::Version::parse(&self.csci.max_version)
            .map_err(|e| format!("Invalid max_version: {}", e))?;
        let target = semver::Version::parse(target_csci)
            .map_err(|e| format!("Invalid target CSCI: {}", e))?;

        if target < min || target > max {
            return Err(format!(
                "CSCI {} incompatible with package (requires {}-{})",
                target_csci, self.csci.min_version, self.csci.max_version
            ));
        }
        Ok(())
    }
}
```

---

## Example Packages

### Example 1: Tool Package — "json-validator"

```toml
[package]
name = "json-validator"
version = "2.0.0"
authors = ["XKernal Team <tools@xkernal.io>"]
description = "High-performance JSON schema validation cognitive tool"
license = "Apache-2.0"
package_type = "tool"

[csci]
min_version = "1.2.0"
max_version = "2.5.0"
target_profiles = ["cpu"]

[capabilities]
required = ["tool_invoke"]
optional = []
max_concurrent_invocations = 50

[cost]
avg_inference_ms = 15
peak_memory_mb = 64
tool_latency_ms = 5
tpc_utilization_percent = 20
monthly_estimated_cost_usd = 0.05

[dependencies]
jsonschema = "0.17.0"
serde_json = "1.0"
```

**Invocation Interface:**
- Input: `{ "document": string, "schema": string }`
- Output: `{ "valid": bool, "errors": [{ "path": string, "message": string }] }`
- Latency: ~15ms per validation
- Use case: Agent validates API responses before processing

### Example 2: Framework Adapter — "langchain-adapter"

```toml
[package]
name = "langchain-adapter"
version = "1.5.0"
authors = ["Adapter Team <adapters@xkernal.io>"]
description = "XKernal ↔ LangChain integration adapter"
license = "Apache-2.0"
package_type = "adapter"

[csci]
min_version = "1.0.0"
max_version = "2.5.0"
target_profiles = ["cpu", "gpu_cuda"]

[capabilities]
required = ["tool_invoke", "memory_allocate", "network_access"]
optional = ["gpu_access"]
max_concurrent_invocations = 20

[cost]
avg_inference_ms = 100
peak_memory_mb = 512
tool_latency_ms = 250
tpc_utilization_percent = 60
monthly_estimated_cost_usd = 0.50

[dependencies]
langchain-rust = "0.4.0"
tokio = { version = "1.0", features = ["full"] }
pyo3 = { version = "0.19", features = ["extension-module"] }
```

**Integration Points:**
- Wraps LangChain AgentExecutor
- Translates XKernal `ToolInvoke` capability to LangChain tool calls
- Provides bidirectional memory coupling (XKernal ↔ LangChain)
- Handles lifecycle: init Python interpreter, configure agents, run, cleanup

---

## Registry API Design

**Endpoints (RESTful):**

```
POST /api/v1/packages/publish
  Body: { manifest: toml, signature: string, package_bytes: bytes }
  Auth: GPG signature verification
  Response: { package_id: sha256, url: string }

GET /api/v1/packages/search?query=summarization&tag=nlp&max_latency_ms=100
  Response: { results: [{ name, version, latency, cost }] }

GET /api/v1/packages/{name}/{version}/manifest
  Response: cs-manifest.toml

GET /api/v1/packages/{name}/{version}/download
  Response: package.tar.gz (binary)

POST /api/v1/resolve-dependencies
  Body: { constraints: [{ name, version_req, csci_version }] }
  Response: { resolved: [{ name, version, sha256 }], conflicts: [] }

GET /api/v1/compatibility-matrix?csci_version=1.5.0
  Response: { capability_availability: { tool_invoke: true, ... } }
```

---

## Next Steps (Week 8+)

- **CLI Implementation:** Full Rust CLI using clap; integration testing
- **Registry Deployment:** PostgreSQL backend; Redis caching; S3 package storage
- **Package Signing:** Integration with GPG; certificate authority for trusted packages
- **Dependency Solver:** SAT-based constraint resolver; performance optimization
- **Cost Accounting:** Agent budget enforcement; usage reporting dashboard

---

## Design Principles Alignment

**Cognitive-Native:** Package format reflects CT lifecycle (initialization, invocation, resource cleanup); cost metadata tied to cognitive operations (inference time, memory, TPC utilization).

**Isolation by Default:** Capability requirements enforced at package level; network/disk access explicit and grantable; sandboxed execution by default.

**Packaging Simplicity:** Single manifest file (cs-manifest.toml); straightforward directory structure; automatic build target detection (WASM vs native).

**Cost Transparency:** All computational costs declared upfront; enables agent budgeting and governance; facilitates multi-tenant cost allocation.

---

**Document Version:** 0.1 (Phase 1 RFC)
**Author:** Engineer 10 (Tooling, Packaging & Documentation)
**Date:** Week 7 Deliverable
**Status:** Ready for Technical Review
