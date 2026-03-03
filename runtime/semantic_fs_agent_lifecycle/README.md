# Semantic Filesystem & Agent Lifecycle

> **Crate:** [`semantic_fs_agent_lifecycle`](Cargo.toml)
> **Stream:** 3 — Runtime & Orchestration
> **Layer:** L2 (Runtime)
> **Owner:** Engineer 03
> **Status:** Active

---

## 1. Purpose & Scope

Manages agent lifecycle (creation, configuration, termination) and provides semantic filesystem (semantic_fs) — a content-addressed, capability-based filesystem replacing traditional POSIX filesystem. Semantic_fs is typed (not byte-oriented) and integrates with semantic_memory tiers and tool bindings.

**Key Responsibilities:**
- Agent initialization and startup
- Agent configuration management
- Agent event loop orchestration
- Semantic filesystem mount and operation
- Content-addressed storage (IPFS-style)
- Capability-based filesystem access control
- Agent state persistence and recovery

**In Scope:**
- Agent lifecycle (spawn → running → shutdown)
- Semantic filesystem API (open, read, write)
- Agent watchdog and health monitoring
- Agent state serialization (for checkpointing)

**Out of Scope:**
- CT scheduling (handled by ct_lifecycle scheduler)
- GPU execution (handled by gpu_accelerator)
- Tool invocation (handled by tool_registry)

---

## 2. Engineering Plan Reference

**Relevant Sections:**
- Section 2.3: Agent domain entity
- Section 4.9: Agent Lifecycle Management
- Section 4.10: Semantic Filesystem Architecture

**Domain Model Entities Involved:**
- **Agent** — Entity managed by this module
- **AgentCrew** — Crew membership and shared filesystem
- **SemanticMemory** — Agent memory tier configuration
- **Capability** — Filesystem access control

---

## 3. Architecture & Design

### 3.1 High-Level Architecture

```
┌────────────────────────────────┐
│  Agent Lifecycle API           │
│  agent_init(), agent_run()     │
└────────────────────────────────┘
             ↓
┌────────────────────────────────┐
│  Agent State Machine           │
│  (INIT → IDLE → ACTIVE → EXIT) │
└────────────────────────────────┘
             ↓         ↓
    ┌────────────┐  ┌──────────────┐
    │ Semantic   │  │ Agent Config │
    │ Filesystem │  │ Management   │
    └────────────┘  └──────────────┘
             ↓         ↓
    ┌────────────────────────────┐
    │ Capability-Based Access    │
    │ Control                    │
    └────────────────────────────┘
```

### 3.2 Key Invariants

1. **Agent State Consistency**: Agent state always consistent across tiers
   - Enforced: L1 cache invalidation on writes, periodic syncs
   - Impact: Agent can tolerate failures and recover

2. **Filesystem Immutability by Default**: Files are immutable unless explicitly mutable
   - Enforced: File mode flag at creation
   - Impact: Prevents accidental file corruption; enables deduplication

3. **Capability-Based Access**: Only agents with FSRead/FSWrite capabilities can access
   - Enforced: Capability check on every filesystem operation
   - Impact: Untrusted agents cannot read other agents' files

---

## 4. Dependencies

| Crate | Type | Layer | Why |
|-------|------|-------|-----|
| `ct_lifecycle` | Internal | L0 | Query agent CTs for lifecycle |
| `capability_engine` | Internal | L0 | Enforce filesystem access control |
| `semantic_memory` | Internal | L1 | Allocate memory for agent state |
| `tool_registry_telemetry` | Internal | L1 | Emit agent lifecycle events |
| `framework_adapters` | Internal | L2 | Integrate with framework agents |

---

## 5. Public API Surface

```rust
/// Initialize a new agent
pub fn agent_init(
    config: AgentConfig,
) -> CsResult<Agent>;

/// Run agent's main loop (spawns CTs)
pub async fn agent_run(
    agent: &mut Agent,
) -> CsResult<()>;

/// Shut down agent gracefully
pub fn agent_shutdown(
    agent: &mut Agent,
) -> CsResult<()>;

/// Semantic filesystem file handle
pub struct SemanticFile {
    pub path: String,
    pub content_hash: ContentHash,  // IPFS-style content addressing
    pub mode: FileMode,
    pub capability_required: Capability,
}

pub enum FileMode {
    ReadOnly,
    ReadWrite,
    ReadWriteExecute,
}

/// Open file from semantic filesystem
pub fn fs_open(
    agent: &Agent,
    path: &str,
) -> CsResult<SemanticFile>;

/// Read file contents
pub fn fs_read(
    file: &SemanticFile,
) -> CsResult<Vec<u8>>;

/// Write file (if mutable)
pub fn fs_write(
    file: &mut SemanticFile,
    contents: Vec<u8>,
) -> CsResult<()>;

/// Agent lifecycle event
pub enum AgentEvent {
    Initialized { agent_id: AgentId },
    SpawnedCT { task_id: TaskId },
    ShutdownRequested,
    Crashed { reason: String },
}
```

---

## 6. Building & Testing

```bash
cargo build -p semantic_fs_agent_lifecycle
cargo test -p semantic_fs_agent_lifecycle
```

**Key Test Scenarios:**
1. Agent initialization — Agent creates with correct config
2. Agent event loop — Main loop spawns CTs correctly
3. Filesystem operations — Read/write files successfully
4. Capability enforcement — Unauthorized access fails
5. Agent checkpointing — State survives restart
6. Multi-agent isolation — Agents cannot access each other's files

---

## 7. Design Decisions Log

### 7.1 "Content-Addressed Filesystem vs. Path-Based?"

**Decision:** Content-addressed (like IPFS) instead of traditional path-based filesystem.

**Alternatives:**
1. POSIX filesystem — Traditional inode-based (Linux ext4)
2. Path-based without content addressing — Simple but no deduplication

**Rationale:**
- Content addressing enables automatic deduplication (same file = same hash)
- Immutable files prevent corruption and enable efficient snapshots
- Better integration with distributed systems (agents on multiple nodes)
- Simpler concurrent access (no inode locks needed)

**Date:** 2026-03-01
**Author:** Engineer 03

### 7.2 "Eager vs. Lazy Agent Initialization?"

**Decision:** Lazy initialization — Agent resources allocated on first use.

**Alternatives:**
1. Eager initialization — Allocate all resources immediately
2. Manual initialization — User explicitly initializes each component

**Rationale:**
- Lazy initialization reduces startup latency
- Scales to thousands of agents (not all active simultaneously)
- Graceful degradation if resources exhausted
- Automatic cleanup when agent exits

**Date:** 2026-03-01
**Author:** Engineer 03

---

## 8. Performance Characteristics

| Operation | Complexity | Notes |
|-----------|-----------|-------|
| `agent_init` | O(log n) | n = agents in system |
| `fs_open` | O(log n) | Content hash lookup |
| `fs_read` | O(s + log n) | s = file size, log n = semantic_mem lookup |
| `fs_write` | O(s) | Serialize + store |
| Agent main loop | O(t) | t = time between CT spawns |

---

## 9. Common Pitfalls & Troubleshooting

**Mistake 1: Trying to write to read-only file**
```rust
// ✗ WRONG: File mode is ReadOnly
let file = fs_open(&agent, "/config.yaml")?;
let mut file = file;
fs_write(&mut file, new_config)?;  // ERROR: read-only

// ✓ RIGHT: Check mode or open mutable
if file.mode == FileMode::ReadOnly {
    return Err(CsError::PermissionDenied);
}
let mut file = fs_open_mut(&agent, "/data.dat")?;
fs_write(&mut file, data)?;
```

**Mistake 2: Not handling agent shutdown gracefully**
```rust
// ✗ WRONG: Agent killed abruptly
drop(agent);  // Kills agent without cleanup

// ✓ RIGHT: Graceful shutdown
agent_shutdown(&mut agent)?;  // Cleanup, flush state
// Now safe to drop
```

---

## 10. Integration Points

| Module | Integration | Protocol |
|--------|-----------|----------|
| `ct_lifecycle` | Agent spawns CTs | Direct call |
| `framework_adapters` | Manage framework agent state | Direct call |
| `semantic_memory` | Allocate memory for agent config | Direct call |
| `tool_registry_telemetry` | Emit agent lifecycle events | Telemetry API |

---

## 11. Future Roadmap

**Planned Improvements:**
- Agent migration — Move agent state across nodes
- Agent marketplace — Publish/subscribe to agent services
- Agent versioning — Multiple agent versions with rollback

**Technical Debt:**
- Content addressing overhead (IPFS integration) is slow
- Agent lifecycle state machine could be simplified
- Multi-agent filesystem merging not implemented (manual for now)

---

## 12. References

- **IPFS White Paper:** https://ipfs.io/ipfs/QmR7GSQM93Cx5eAg6a6ZL86v2F34AZ3J6W1o3JtQ1K3HQ/ipfs.pdf
- **Agent Lifecycle:** https://en.wikipedia.org/wiki/Software_agent#Lifecycle

---

**README Version:** 1.0
**Last Updated:** 2026-03-01
**Owner:** Engineer 03
