# XKernal Agent System: Comprehensive Documentation
**WEEK 33 | Semantic FS & Agent Lifecycle Engineering**

> RFC-Style Specification for Agent Deployment, Configuration, and Operations on XKernal L0-L3 Runtime Stack

---

## Table of Contents
1. [Agent Unit File Specification (RFC)](#agent-unit-file-specification)
2. [Mount Configuration Guide](#mount-configuration-guide)
3. [cs-agentctl CLI Reference](#cs-agentctl-cli-reference)
4. [Operator's Manual](#operators-manual)
5. [Developer's Guide](#developers-guide)
6. [Architecture Documentation](#architecture-documentation)

---

## Agent Unit File Specification

### RFC: Agent Unit File Format v1.0

**Status:** Proposed Standard
**Author:** Engineer 8 (Semantic FS & Agent Lifecycle)
**Date:** 2026-03-02
**Target:** XKernal L2 Runtime (microkernel-resident agents)

#### 1.1 Overview

The Agent Unit File (`.agent.toml`) defines an autonomous agent's metadata, capabilities, memory configuration, knowledge source mounts, inter-process communication channels, health probes, and lifecycle policies. Files reside in `/mnt/XKernal/agents/{agent-id}/unit.toml`.

#### 1.2 Formal Grammar

```
agent-unit-file = section-agent
                   [ section-capabilities ]
                   [ section-memory ]
                   [ section-mounts ]
                   [ section-ipc ]
                   [ section-health ]
                   [ section-lifecycle ]

section-agent        = "[agent]" fields-agent
section-capabilities = "[capabilities]" fields-capabilities
section-memory       = "[memory]" fields-memory
section-mounts       = "[mounts]" fields-mounts
section-ipc          = "[ipc]" fields-ipc
section-health       = "[health]" fields-health
section-lifecycle    = "[lifecycle]" fields-lifecycle
```

#### 1.3 Section Definitions

**[agent]** (REQUIRED)
- `id` (string): Unique agent identifier, alphanumeric+underscore, 1-63 chars
- `name` (string): Human-readable name
- `version` (string): Semantic version (x.y.z)
- `kind` (enum): ASSISTANT | REASONER | EXECUTOR | PLANNER | OBSERVER
- `model` (string): AI model identifier (e.g., "claude-opus-4-6")
- `description` (string): Purpose and behavior documentation

**[capabilities]** (OPTIONAL, default: minimal permissions)
- `scope` (array): [LLM_INFERENCE, TOOL_EXECUTION, STATE_ACCESS, IPC_SEND, MOUNT_READ, ...]
- `permissions` (table):
  - `tool_execution` (bool): Can invoke external tools
  - `state_mutation` (bool): Can modify persistent state
  - `mount_write` (bool): Can write to mounted sources (default: false)
  - `ipc_broadcast` (bool): Can send IPC messages to multiple agents
  - `scale` (bool): Can trigger auto-scaling requests
  - `resource_limit_cpu` (string): "100m", "1000m", "2" (cores)
  - `resource_limit_memory` (string): "256Mi", "1Gi"

**[memory]** (OPTIONAL, default: single-tier transient)
- `tiers` (array of tables):
  - `name` (string): TRANSIENT | PERSISTENT | CACHE | CONTEXT
  - `capacity_bytes` (int): Maximum size
  - `ttl_seconds` (int, optional): Time-to-live for entries
  - `eviction_policy` (enum): LRU | FIFO | LFU
  - `persistence_backend` (string, optional): "sqlite" | "redis" | "s3"

**[mounts]** (OPTIONAL, default: no mounted sources)
- Subsection for each mount (name = mount-id):
  - `type` (enum): LOCAL_FS | HTTP_API | S3_OBJECT | DATABASE | PLUGIN
  - `source_uri` (string): Path/URL to knowledge source
  - `mount_path` (string): Logical path within agent (/data/kb, /api/ext, etc.)
  - `auth` (table, optional): {method: "bearer|basic|mTLS", secret_ref: "..."}
  - `cache_ttl_seconds` (int, optional, default: 300)
  - `failure_mode` (enum, default: FAIL_CLOSED): FAIL_OPEN | FAIL_CLOSED | DEGRADE
  - `read_only` (bool, default: true)

**[ipc]** (OPTIONAL, default: no inter-agent communication)
- Subsection for each channel (name = channel-id):
  - `direction` (enum): SEND | RECEIVE | BIDIRECTIONAL
  - `peer_agent_id` (string or array): Target agent(s)
  - `message_type` (string): Semantic type tag (e.g., "state_update", "query_request")
  - `queue_depth` (int, default: 100): Max pending messages
  - `timeout_seconds` (int, default: 30): Message timeout

**[health]** (OPTIONAL, default: basic liveness check)
- `check_interval_seconds` (int, default: 30): Probe frequency
- `timeout_seconds` (int, default: 10): Individual probe timeout
- `failure_threshold` (int, default: 3): Failures before unhealthy
- `probes` (array of tables):
  - `name` (string): Probe identifier
  - `type` (enum): EXEC | HTTP | TCP | CUSTOM
  - `endpoint` (string): Command/URL/socket
  - `expected_status` (int/string): Expected response code or stdout match

**[lifecycle]** (OPTIONAL, default: on-demand, no auto-restart)
- `startup_policy` (enum): ON_DEMAND | EAGER | SCHEDULED
- `restart_policy` (enum, default: NEVER): NEVER | ON_FAILURE | ALWAYS
- `restart_delay_seconds` (int, default: 5): Backoff delay
- `max_restart_attempts` (int, default: 3): -1 = infinite
- `graceful_shutdown_timeout_seconds` (int, default: 30)
- `scaling_policy` (table, optional):
  - `enable_autoscale` (bool, default: false)
  - `min_instances` (int, default: 1)
  - `max_instances` (int, default: 5)
  - `cpu_threshold_percent` (int, default: 80)
  - `memory_threshold_percent` (int, default: 85)
  - `scale_up_delay_seconds` (int, default: 60)
  - `scale_down_delay_seconds` (int, default: 300)

#### 1.4 Validation Rules

1. `[agent].id` must be globally unique
2. `[agent].model` must be registered in L2 Model Registry
3. All mount paths must be unique and absolute
4. IPC peer agents must exist and have reciprocal channels (or unidirectional setup)
5. Total memory from all tiers ≤ process resource limit
6. Health probes must not overlap in execution timing
7. Lifecycle policies must not create perpetual restart loops (detected via config analysis)

#### 1.5 Example: Comprehensive Agent Unit File

```toml
[agent]
id = "research_assistant_v1"
name = "Research Assistant"
version = "1.0.0"
kind = "REASONER"
model = "claude-opus-4-6"
description = "Autonomous research assistant with web and database access"

[capabilities]
scope = ["LLM_INFERENCE", "TOOL_EXECUTION", "STATE_ACCESS", "MOUNT_READ", "IPC_SEND"]
permissions = {
  tool_execution = true,
  state_mutation = true,
  mount_write = false,
  ipc_broadcast = false,
  resource_limit_cpu = "1000m",
  resource_limit_memory = "1Gi"
}

[[memory.tiers]]
name = "TRANSIENT"
capacity_bytes = 10485760  # 10 MiB
ttl_seconds = 3600
eviction_policy = "LRU"

[[memory.tiers]]
name = "PERSISTENT"
capacity_bytes = 104857600  # 100 MiB
persistence_backend = "sqlite"

[mounts.web_kb]
type = "HTTP_API"
source_uri = "https://api.knowledge.internal/v1"
mount_path = "/data/web"
auth = { method = "bearer", secret_ref = "sec_web_api_token" }
cache_ttl_seconds = 600
failure_mode = "FAIL_CLOSED"
read_only = true

[mounts.local_docs]
type = "LOCAL_FS"
source_uri = "/opt/documents"
mount_path = "/data/local"
read_only = true

[mounts.postgres_research]
type = "DATABASE"
source_uri = "postgresql://research-db:5432/main"
mount_path = "/data/db"
auth = { method = "basic", secret_ref = "sec_db_creds" }
failure_mode = "DEGRADE"

[[ipc.coordinator_channel]]
direction = "BIDIRECTIONAL"
peer_agent_id = "task_coordinator"
message_type = "task_request"
queue_depth = 200

[health]
check_interval_seconds = 30
timeout_seconds = 10
failure_threshold = 3

[[health.probes]]
name = "liveness"
type = "EXEC"
endpoint = "/opt/bin/health-check"
expected_status = 0

[[health.probes]]
name = "model_api"
type = "HTTP"
endpoint = "http://localhost:8000/health"
expected_status = 200

[lifecycle]
startup_policy = "EAGER"
restart_policy = "ON_FAILURE"
restart_delay_seconds = 10
max_restart_attempts = 5
graceful_shutdown_timeout_seconds = 60

[lifecycle.scaling_policy]
enable_autoscale = true
min_instances = 1
max_instances = 5
cpu_threshold_percent = 80
memory_threshold_percent = 85
scale_up_delay_seconds = 60
scale_down_delay_seconds = 300
```

---

## Mount Configuration Guide

### 2.1 Local Filesystem Mount

**Use Case:** Knowledge bases, document stores, embedded embeddings
**Performance:** Native filesystem speed (typically <10ms latency)

```toml
[mounts.local_kb]
type = "LOCAL_FS"
source_uri = "/opt/data/knowledge-base"
mount_path = "/data/kb"
read_only = true
cache_ttl_seconds = 0  # No caching; direct FS access
failure_mode = "FAIL_CLOSED"
```

**Directory Structure:**
```
/opt/data/knowledge-base/
├── metadata.json
├── vectors/
│   ├── embeddings.bin (indexed search)
│   └── metadata.json
└── documents/
    ├── doc_001.txt
    ├── doc_002.pdf
    └── ...
```

**Auth:** None (uses agent process identity and Unix permissions)
**Failure Handling:** Mount unavailable → queries fail with semantic error

---

### 2.2 HTTP/REST API Mount

**Use Case:** External APIs, microservices, SaaS integrations
**Performance:** Network latency + API response time (typically 100-2000ms)

```toml
[mounts.ext_api]
type = "HTTP_API"
source_uri = "https://api.external.com/v1/knowledge"
mount_path = "/api/external"
auth = { method = "bearer", secret_ref = "token_ext_api" }
cache_ttl_seconds = 300
failure_mode = "DEGRADE"
read_only = true

[mounts.ext_api.headers]
"Accept" = "application/json"
"X-Request-Id" = "{trace_id}"  # Dynamic substitution
```

**Supported Auth Methods:**
- `bearer`: Authorization: Bearer <token>
- `basic`: Authorization: Basic base64(user:pass)
- `mTLS`: Client certificates from secret_ref path
- `api_key`: Custom header injection (method = "api_key", header_name = "X-API-Key")

**Caching Strategy:**
- Cache hits bypass network calls
- Stale responses returned if API fails (failure_mode = DEGRADE)
- Cache eviction via TTL and LRU within capacity

**Example Query Flow:**
```
Agent → Mount Manager → Cache lookup (hit/miss)
  → HTTP client → Endpoint → Response → Parse → Return to Agent
```

---

### 2.3 S3/Object Storage Mount

**Use Case:** Large-scale data lakes, logs, ML artifacts
**Performance:** S3 API latency (typically 100-500ms); batch operations

```toml
[mounts.data_lake]
type = "S3_OBJECT"
source_uri = "s3://my-data-lake/agent-kb"
mount_path = "/data/lake"
auth = { method = "bearer", secret_ref = "s3_credentials" }
cache_ttl_seconds = 1800
failure_mode = "FAIL_CLOSED"
read_only = true

[mounts.data_lake.config]
region = "us-west-2"
prefix_filter = "kb/"  # Only expose kb/ prefix
list_max_keys = 1000
multipart_threshold_bytes = 5242880  # 5 MiB
```

**Secret Format (referenced by secret_ref):**
```json
{
  "access_key_id": "AKIA...",
  "secret_access_key": "...",
  "session_token": "optional"
}
```

**Failure Scenarios:**
- Network timeout → FAIL_CLOSED: Mount unavailable
- 403 Forbidden → FAIL_CLOSED: Permission error logged
- 404 Not Found → FAIL_CLOSED: Resource missing (recoverable)

---

### 2.4 Database Connector Mount

**Use Case:** SQL queries, relational state, real-time data access
**Performance:** Database query latency (typically 10-100ms with indices)

```toml
[mounts.postgres_data]
type = "DATABASE"
source_uri = "postgresql://host:5432/agent_db"
mount_path = "/data/db"
auth = { method = "basic", secret_ref = "db_creds" }
cache_ttl_seconds = 60
failure_mode = "DEGRADE"
read_only = false

[mounts.postgres_data.config]
pool_size = 10
query_timeout_seconds = 30
ssl_mode = "require"
prepared_statements = true
```

**Supported Backends:**
- PostgreSQL (recommended)
- MySQL/MariaDB
- SQLite (embedded; requires LOCAL_FS path)

**Query Execution:**
```sql
-- Agent can execute (via semantic mount layer):
SELECT * FROM documents WHERE agent_id = $1 AND timestamp > $2
-- Parameterized to prevent injection
```

**Transactional Semantics:**
- Read-only: Autocommit mode
- Read-write: Explicit transaction support via IPC signals

---

### 2.5 Custom Plugin Mount

**Use Case:** Proprietary data sources, legacy systems, specialized protocols
**Performance:** Plugin-dependent (typically 50-1000ms)

```toml
[mounts.legacy_system]
type = "PLUGIN"
source_uri = "plugin://legacy-integration/0.2.1"
mount_path = "/data/legacy"
auth = { method = "mTLS", secret_ref = "legacy_certs" }
cache_ttl_seconds = 0
failure_mode = "FAIL_CLOSED"
read_only = true

[mounts.legacy_system.config]
plugin_path = "/opt/plugins/legacy_adapter.so"
init_timeout_seconds = 30
heartbeat_interval_seconds = 60
```

**Plugin Interface (Rust FFI):**
```rust
#[no_mangle]
pub extern "C" fn mount_init() -> *mut MountPlugin;
pub extern "C" fn mount_query(plugin: *mut MountPlugin, query: *const u8) -> Result;
pub extern "C" fn mount_health(plugin: *mut MountPlugin) -> HealthStatus;
```

**Deployment:** Plugins distributed as `.so` (Linux), `.dylib` (macOS), loaded at agent startup

---

## cs-agentctl CLI Reference

### 3.1 Installation & Configuration

```bash
$ cs-agentctl version
cs-agentctl v1.2.0 (XKernal Agent Runtime)
Target: x86_64-unknown-linux-gnu / L2 Runtime v0.8.2

$ cs-agentctl config set runtime.endpoint unix:///mnt/XKernal/runtime.sock
$ cs-agentctl config set logs.level debug
```

---

### 3.2 Subcommand Reference

#### create — Provision new agent instance

```bash
Usage: cs-agentctl create [OPTIONS] UNIT_FILE

Options:
  --agent-id <ID>        Override unit file agent.id
  --namespace <NS>       Kubernetes-style namespace (default: default)
  --dry-run             Validate without deploying
  -q, --quiet            Minimal output

Example:
$ cs-agentctl create research_assistant.agent.toml --namespace prod --dry-run
✓ Validation passed (3 mounts, 2 health probes)
├─ Agent ID: research_assistant_v1
├─ Memory: 110 MiB (transient + persistent)
├─ Mounts: web_kb (HTTP), local_docs (LOCAL_FS), postgres_research (DATABASE)
└─ Capabilities: LLM_INFERENCE, TOOL_EXECUTION, STATE_ACCESS

$ cs-agentctl create research_assistant.agent.toml --namespace prod
✓ Agent created successfully
Agent ID: research_assistant_v1
Namespace: prod
Status: CREATED
Next: cs-agentctl start research_assistant_v1 --namespace prod
```

---

#### start — Launch agent instance

```bash
Usage: cs-agentctl start [OPTIONS] <AGENT_ID>

Options:
  --wait <TIMEOUT>      Wait for ready state (default: 30s)
  --namespace <NS>      Agent namespace

Example:
$ cs-agentctl start research_assistant_v1 --namespace prod --wait 60s
⟳ Starting agent...
├─ Load unit file: /mnt/XKernal/agents/research_assistant_v1/unit.toml
├─ Validate capabilities...
├─ Initialize memory tiers...
  ├─ TRANSIENT: 10 MiB (LRU, TTL 3600s)
  ├─ PERSISTENT: 100 MiB (SQLite)
├─ Mount sources...
  ├─ web_kb (HTTP): https://api.knowledge.internal/v1
  ├─ local_docs (LOCAL_FS): /opt/documents
  ├─ postgres_research (DATABASE): postgresql://research-db:5432/main
├─ Spawn process (PID: 12847)
├─ Wait for health probes...
  ├─ liveness (EXEC): ✓ passed
  ├─ model_api (HTTP): ✓ passed
├─ Start lifecycle watchers

✓ Agent started (elapsed: 3.2s)
Agent: research_assistant_v1
Status: READY
PID: 12847
Uptime: 3.2s
Memory: 45 MiB (transient), 2 MiB (persistent)
Next: cs-agentctl status research_assistant_v1
```

---

#### stop — Gracefully shutdown agent

```bash
Usage: cs-agentctl stop [OPTIONS] <AGENT_ID>

Options:
  --timeout <SEC>       Graceful shutdown timeout (default: 30s)
  --force               Hard kill (SIGKILL)
  --namespace <NS>      Agent namespace

Example:
$ cs-agentctl stop research_assistant_v1 --namespace prod --timeout 45s
⟳ Stopping agent...
├─ Send SIGTERM (graceful shutdown signal)
├─ Wait for cleanup (persistent state sync, mount flush)
├─ Timeout: 45s

✓ Agent stopped
Agent: research_assistant_v1
Final Status: STOPPED
Uptime: 7m 23s
State Persisted: 2.1 MiB
```

---

#### status — Display agent state

```bash
Usage: cs-agentctl status [OPTIONS] <AGENT_ID>

Options:
  --detailed, -d        Include metrics and mount health
  --watch              Continuous monitoring (1s interval)
  --namespace <NS>      Agent namespace

Example:
$ cs-agentctl status research_assistant_v1 --namespace prod --detailed
Agent: research_assistant_v1
Status: READY
Uptime: 12m 47s
PID: 12847
Restarts: 0 (max: 5)

Memory:
  Transient: 67 MiB / 10 MiB (67%)
  Persistent: 18 MiB / 100 MiB (18%)
  Resident: 85 MiB / 1 GiB (8%)

CPU: 120m / 1000m (12%)

Mounts:
  web_kb (HTTP): HEALTHY (cache hit rate: 73%, latency p99: 250ms)
  local_docs (LOCAL_FS): HEALTHY (98 documents indexed)
  postgres_research (DATABASE): HEALTHY (pool: 4/10, query latency p99: 18ms)

Health Probes:
  liveness (EXEC): ✓ PASS (2ms)
  model_api (HTTP): ✓ PASS (152ms)
  Last check: 12 seconds ago

IPC Channels:
  → task_coordinator (BIDIRECTIONAL): 3 pending messages, 0 errors
```

---

#### logs — Stream agent output

```bash
Usage: cs-agentctl logs [OPTIONS] <AGENT_ID>

Options:
  -f, --follow          Follow logs (tail mode)
  --tail <N>           Display last N lines (default: 50)
  --level <LEVEL>      Filter by severity (ERROR, WARN, INFO, DEBUG)
  --namespace <NS>      Agent namespace

Example:
$ cs-agentctl logs research_assistant_v1 --namespace prod -f --level WARN
2026-03-02T10:23:45.123Z [WARN] Mount health degraded: postgres_research
  Reason: Connection pool exhaustion (10/10 active)
  Action: Consider scale-up or connection pooling adjustment
  Trace: mount_manager::query_executor::pool_allocator

2026-03-02T10:24:12.567Z [WARN] Memory: TRANSIENT tier near capacity (89%)
  Eviction triggered (LRU): freed 3 entries, 1.2 MiB
  Trend: Avg growth 0.5 MiB/min; capacity exceeded in ~11 minutes
  Recommendation: Increase tier capacity or reduce TTL

2026-03-02T10:25:30.891Z [INFO] Health probe recovered: postgres_research
  Downtime: 1m 18s
  Recovery action: Automatic reconnect with exponential backoff
```

---

#### exec — Execute command in agent context

```bash
Usage: cs-agentctl exec [OPTIONS] <AGENT_ID> [CMD]

Options:
  --stdin               Read from stdin
  --tty                 Allocate pseudo-TTY
  --namespace <NS>      Agent namespace

Example:
$ cs-agentctl exec research_assistant_v1 -n prod \
  "SELECT COUNT(*) FROM documents WHERE processed = false;"

(executes query in agent's DATABASE mount context)
Result:
┌───────┐
│ count │
├───────┤
│ 247   │
└───────┘

$ cs-agentctl exec research_assistant_v1 -n prod --tty
(interactive shell in agent namespace; can inspect state, debug mounts)
```

---

#### mount/unmount — Manage dynamic mounts

```bash
Usage: cs-agentctl mount [OPTIONS] <AGENT_ID> <MOUNT_ID> <SOURCE_URI>
Usage: cs-agentctl unmount [OPTIONS] <AGENT_ID> <MOUNT_ID>

Options:
  --type <TYPE>         Mount type (LOCAL_FS, HTTP_API, S3_OBJECT, ...)
  --mount-path <PATH>   Logical mount path in agent namespace
  --read-only           Mark mount as read-only
  --namespace <NS>      Agent namespace

Example:
$ cs-agentctl mount research_assistant_v1 -n prod \
  new_api_source https://api.newservice.com/v1 \
  --type HTTP_API \
  --mount-path /api/new \
  --read-only

✓ Mount created (not yet attached)
├─ Mount ID: new_api_source
├─ Type: HTTP_API
├─ Source: https://api.newservice.com/v1
├─ Path: /api/new
├─ Read-only: true

$ cs-agentctl mount research_assistant_v1 -n prod new_api_source activate
✓ Mount activated
├─ Health check: PASS
├─ Cache initialized: 100 MiB
└─ Ready for queries

$ cs-agentctl unmount research_assistant_v1 -n prod new_api_source
✓ Mount deactivated and removed
```

---

#### health — Detailed health analysis

```bash
Usage: cs-agentctl health [OPTIONS] <AGENT_ID>

Options:
  --probes-only        Show only health probe results
  --namespace <NS>      Agent namespace

Example:
$ cs-agentctl health research_assistant_v1 -n prod
Agent Health: READY

Probes (3 total):
  ✓ liveness (EXEC)
    └─ Command: /opt/bin/health-check
    └─ Last run: 15s ago
    └─ Status code: 0
    └─ Latency: 2ms

  ✓ model_api (HTTP)
    └─ Endpoint: http://localhost:8000/health
    └─ Last run: 18s ago
    └─ Status code: 200
    └─ Latency: 142ms

  ✓ db_connectivity (HTTP)
    └─ Endpoint: http://localhost:5432 (PostgreSQL)
    └─ Last run: 21s ago
    └─ Status: connected, pool 6/10 active
    └─ Latency: 8ms

Mount Health:
  web_kb (HTTP): HEALTHY
    └─ Availability: 99.87% (over 24h)
    └─ P99 latency: 287ms
    └─ Cache hit rate: 71%

  local_docs (LOCAL_FS): HEALTHY
    └─ Availability: 100%
    └─ Latency: <1ms (cached)

  postgres_research (DATABASE): HEALTHY
    └─ Availability: 100%
    └─ Query latency p99: 21ms
    └─ Connection errors (24h): 0

System Health:
  Memory pressure: NORMAL (67% of transient tier)
  CPU usage: 12% of allocated (120m/1000m)
  IPC queue depth: 3/200 (healthy)

Overall: HEALTHY
Recommended actions: None
```

---

#### scale — Adjust instance count

```bash
Usage: cs-agentctl scale [OPTIONS] <AGENT_ID> --replicas <N>

Options:
  --replicas <N>       Target replica count
  --namespace <NS>      Agent namespace

Example:
$ cs-agentctl scale research_assistant_v1 -n prod --replicas 3
⟳ Scaling agent...
├─ Current replicas: 1
├─ Target replicas: 3
├─ Spawn 2 new instances...
  ├─ Instance 2: PID 14201 (ready)
  ├─ Instance 3: PID 14205 (ready)
├─ Health checks: all PASS
├─ Load balancer: updated

✓ Scaled successfully
Agent: research_assistant_v1
Replicas: 3 (all READY)
```

---

#### rollback — Revert to previous version

```bash
Usage: cs-agentctl rollback [OPTIONS] <AGENT_ID>

Options:
  --to-version <VER>   Specific version to rollback to
  --namespace <NS>      Agent namespace

Example:
$ cs-agentctl rollback research_assistant_v1 -n prod --to-version 0.9.2
✓ Rollback initiated
├─ Current version: 1.0.0
├─ Target version: 0.9.2
├─ State snapshot: created
├─ Spawn v0.9.2 instance
├─ Health checks: PASS
├─ Route traffic → v0.9.2
├─ Shutdown v1.0.0

✓ Rollback complete
Agent version: 0.9.2
Instances: 1 (READY)
```

---

#### config — Manage agent configuration

```bash
Usage: cs-agentctl config [OPTIONS] <AGENT_ID> get|set|patch

Example:
$ cs-agentctl config research_assistant_v1 -n prod get
(outputs current unit.toml)

$ cs-agentctl config research_assistant_v1 -n prod set capabilities.tool_execution false
✓ Configuration updated
├─ Restart required: yes
├─ Impact: Tool execution disabled

$ cs-agentctl config research_assistant_v1 -n prod patch lifecycle.restart_policy ALWAYS
✓ Configuration patched
├─ Field: lifecycle.restart_policy
├─ Old: ON_FAILURE
├─ New: ALWAYS
├─ Live update: yes (no restart needed)
```

---

## Operator's Manual

### 4.1 Deployment Checklist

1. **Pre-flight Validation**
   - Verify XKernal L2 Runtime version ≥ 0.8.0
   - Check resource availability: requested CPU/memory vs. available
   - Validate unit file TOML syntax: `cs-agentctl create unit.toml --dry-run`
   - Verify mount endpoints reachable (ping URLs, test DB connections)
   - Confirm secrets provisioned in secret store

2. **Progressive Rollout**
   ```bash
   # Stage 1: Single instance in staging
   cs-agentctl create agent.toml --namespace staging
   cs-agentctl start agent_v1 --namespace staging --wait 60s
   cs-agentctl health agent_v1 --namespace staging

   # Stage 2: Canary (10% traffic) in production
   cs-agentctl create agent.toml --namespace prod --replicas 1
   cs-agentctl scale agent_v1 --replicas 2 --namespace prod
   # Monitor 30 minutes

   # Stage 3: Full rollout
   cs-agentctl scale agent_v1 --replicas 5 --namespace prod
   ```

3. **Post-deployment Verification**
   - Confirm all replicas READY: `cs-agentctl status agent_v1 --namespace prod --watch`
   - Run smoke tests: invoke agent with synthetic queries
   - Monitor error rates: <0.1% for 5 minutes
   - Check mount health: `cs-agentctl health agent_v1 --namespace prod`

---

### 4.2 Health Monitoring Dashboard

**Recommended Metrics:**

| Metric | Alert Threshold | Action |
|--------|-----------------|--------|
| Agent status != READY | Immediate | Page on-call; investigate logs |
| Memory (any tier) > 90% | 2 min window | Scale up or reduce TTL |
| CPU usage > 85% | 5 min window | Scale up or optimize queries |
| Mount health degraded | 2 failures | Investigate mount endpoint; rollback if critical |
| IPC queue depth > 80% | Persistent | Reduce message volume or increase queue depth |
| Health probe failure rate > 1% | 3 min window | Review probe configuration; may indicate flaky deps |
| Error rate > 1% | 5 min window | Check logs; consider rollback |

**Dashboard Queries (Prometheus format):**
```promql
# Agent uptime
time() - agent_start_timestamp_seconds

# Memory trend
rate(agent_memory_bytes[5m])

# Mount latency
histogram_quantile(0.99, agent_mount_latency_seconds)

# IPC queue depth
agent_ipc_queue_depth

# Health check success rate
rate(agent_health_check_total{status="pass"}[5m])
```

---

### 4.3 Incident Response

**Scenario 1: Agent Stuck in STARTING State**
```bash
# 1. Check logs
cs-agentctl logs agent_v1 --namespace prod --level WARN

# 2. Validate mounts
cs-agentctl health agent_v1 --namespace prod --probes-only

# 3. Restart with extended timeout
cs-agentctl stop agent_v1 --namespace prod --force
cs-agentctl start agent_v1 --namespace prod --wait 120s

# 4. If still failing, rollback
cs-agentctl rollback agent_v1 --namespace prod --to-version <prior-version>
```

**Scenario 2: Sudden Memory Spike**
```bash
# 1. Identify culprit
cs-agentctl status agent_v1 --namespace prod --detailed

# 2. Check memory tier eviction logs
cs-agentctl logs agent_v1 --namespace prod --level INFO | grep -i eviction

# 3. Options:
#    a) Increase tier capacity in unit file
#    b) Reduce cache TTLs in mount configs
#    c) Reduce TRANSIENT tier TTL

# 4. Apply config patch (if non-breaking)
cs-agentctl config agent_v1 --namespace prod \
  patch memory.tiers[0].ttl_seconds 1800

# 5. Monitor recovery
cs-agentctl status agent_v1 --namespace prod --watch
```

**Scenario 3: Mount Endpoint Failures**
```bash
# 1. Identify failing mount
cs-agentctl health agent_v1 --namespace prod

# 2. Test mount connectivity
cs-agentctl exec agent_v1 --namespace prod \
  "curl -v https://api.knowledge.internal/health"

# 3. Check auth credentials
cs-agentctl config agent_v1 --namespace prod get | grep -A3 "[mounts.web_kb]"

# 4. Temporarily remove mount (if not critical)
cs-agentctl unmount agent_v1 --namespace prod web_kb

# 5. Wait for mount endpoint recovery or contact platform team
# (Monitor: cs-agentctl logs agent_v1 --namespace prod -f)

# 6. Re-attach when stable
cs-agentctl mount agent_v1 --namespace prod web_kb \
  "https://api.knowledge.internal/v1" \
  --type HTTP_API --mount-path /data/web
```

---

### 4.4 Capacity Planning

**Calculation Template:**

```
Per-Agent Overhead:
  Base process: ~30 MiB
  TRANSIENT memory tier: config-defined (e.g., 10 MiB)
  PERSISTENT tier: config-defined (e.g., 100 MiB)
  IPC queue buffer: queue_depth × ~100 bytes
  Health probe threads: 2 threads × ~1 MiB each

Example (research_assistant_v1):
  Base: 30 MiB
  Transient: 10 MiB
  Persistent: 100 MiB
  IPC (200 queue depth): 0.02 MiB
  Probes: 2 MiB
  ────────────────
  Total per instance: ~142 MiB

Cluster Capacity (for N=5 replicas):
  Agent overhead: 142 MiB × 5 = 710 MiB
  Mount cache (shared): 50 MiB
  Runtime overhead: 500 MiB
  ────────────────
  Total minimum: 1.26 GiB
  Recommended (with headroom): 2.0 GiB
```

**Horizontal Scaling Recommendation:**
- Agent CPU > 80% for 5+ minutes: Scale up 1-2 replicas
- Memory > 85% for 5+ minutes: Consider increasing memory limit or cache eviction
- Max safe replicas per node: 10 (subject to total memory budget)

---

## Developer's Guide

### 5.1 Agent Creation Tutorial

**Step 1: Define Unit File**
```toml
[agent]
id = "my_assistant_v1"
name = "My Custom Assistant"
version = "1.0.0"
kind = "ASSISTANT"
model = "claude-opus-4-6"

[capabilities]
scope = ["LLM_INFERENCE", "TOOL_EXECUTION"]
permissions = { tool_execution = true, resource_limit_memory = "512Mi" }

[[memory.tiers]]
name = "TRANSIENT"
capacity_bytes = 5242880  # 5 MiB
ttl_seconds = 3600
eviction_policy = "LRU"

[lifecycle]
startup_policy = "ON_DEMAND"
restart_policy = "ON_FAILURE"
max_restart_attempts = 3
```

**Step 2: Deploy Agent**
```bash
cs-agentctl create my_assistant.agent.toml --namespace dev --dry-run
cs-agentctl create my_assistant.agent.toml --namespace dev
```

**Step 3: Start & Verify**
```bash
cs-agentctl start my_assistant_v1 --namespace dev --wait 30s
cs-agentctl status my_assistant_v1 --namespace dev
```

---

### 5.2 Lifecycle Hooks

Agents implement lifecycle hooks via environment callbacks:

```rust
// Pseudo-code; actual implementation is runtime-dependent

// Hook 1: on_init (called during agent instantiation)
#[lifecycle_hook("on_init")]
pub fn initialize_state() -> Result<()> {
    // Load persistent state from storage
    // Initialize tool registry
    // Warm up mount caches
    Ok(())
}

// Hook 2: on_start (called when agent transitions to READY)
#[lifecycle_hook("on_start")]
pub fn begin_operation() -> Result<()> {
    // Start background health monitors
    // Open IPC channels
    // Begin inference loop
    Ok(())
}

// Hook 3: on_stop (called during graceful shutdown)
#[lifecycle_hook("on_stop")]
pub fn shutdown() -> Result<()> {
    // Flush persistent state to storage
    // Close mounts and cleanup connections
    // Drain IPC queues
    Ok(())
}

// Hook 4: on_health (called by health probes)
#[lifecycle_hook("on_health")]
pub fn report_health() -> HealthStatus {
    HealthStatus {
        status: Status::READY,
        checks: vec![
            ("memory", Check::PASS),
            ("inference_engine", Check::PASS),
            ("mounts", Check::PASS),
        ],
    }
}

// Hook 5: on_scale (called when replica count changes)
#[lifecycle_hook("on_scale")]
pub fn handle_scaling(event: ScalingEvent) -> Result<()> {
    match event.direction {
        ScalingDirection::UP => {
            // Notify load balancer of new instance
            // Sync state if needed
        },
        ScalingDirection::DOWN => {
            // Drain in-flight requests
            // Persist state before shutdown
        },
    }
    Ok(())
}
```

---

### 5.3 State Management

**Memory Tier Access Patterns:**

```python
# Pseudo-code (agent can use HTTP API to memory subsystem)

# Transient memory (fast, volatile)
agent.memory.transient.set("current_task", task_obj, ttl_seconds=1800)
current_task = agent.memory.transient.get("current_task")
agent.memory.transient.delete("current_task")

# Persistent memory (durable, slower)
agent.memory.persistent.set("user_history", history_list)
history = agent.memory.persistent.get("user_history")

# Cache (mount-based, read-only from agent perspective)
cached_docs = agent.mounts["web_kb"].query("SELECT * FROM documents WHERE tag=?", ["important"])
```

**Data Consistency:**
- Transient tier: Lost on shutdown (suitable for in-flight computations)
- Persistent tier: Durable (suitable for long-lived state, conversation history)
- Mounts: External source of truth (agent is consumer, not authority)

---

### 5.4 IPC Patterns

**Pattern 1: Request-Response**
```
Agent A (requester):
  1. Send message via IPC channel
  2. Wait for response (with timeout)
  3. Process response

Agent B (responder):
  1. Receive message
  2. Process and compute response
  3. Send response back
```

**Pattern 2: Pub-Sub (Broadcast)**
```toml
[ipc.broadcast_channel]
direction = "SEND"
peer_agent_id = ["observer_a", "observer_b", "observer_c"]
message_type = "event_notification"
queue_depth = 100
```

**Pattern 3: Streaming**
```
Agent A: Send stream_start message
Agent A: Send data chunks (with sequence numbers)
Agent A: Send stream_end message
Agent B: Reassemble stream, deduplicate via sequence nums
```

---

### 5.5 Tool Integration

**Tool Registration:**
```toml
# In unit file (future enhancement)
[capabilities.tools]
search_web = { endpoint = "tool://search-service", timeout_seconds = 10 }
send_email = { endpoint = "tool://email-service", timeout_seconds = 5 }
query_database = { endpoint = "mount://postgres_research", timeout_seconds = 30 }
```

**Tool Invocation:**
```
Agent → Tool Manager → Route to endpoint (remote or mount)
  → Validate permissions (scope includes TOOL_EXECUTION)
  → Execute with timeout
  → Return result or timeout error
```

---

### 5.6 Testing Strategies

**Unit Test (agent logic):**
```bash
# Mock all external mounts and IPC channels
# Test logic in isolation with synthetic data
cs-agentctl exec agent_v1 --namespace test "run_unit_tests()"
```

**Integration Test (agent + mounts):**
```bash
# Start agent with test mount endpoints
# Verify mount queries return expected results
# Check cache behavior, failover logic
cs-agentctl start agent_v1 --namespace test
cs-agentctl exec agent_v1 --namespace test \
  "SELECT * FROM test_mount WHERE id=42"
```

**Load Test (scaling + performance):**
```bash
# Generate sustained request load
# Monitor memory, CPU, latency
# Verify autoscaling triggers correctly
# Check mount degradation behavior

# Synthetic load: 100 req/s for 5 minutes
cs-agentctl exec agent_v1 --namespace test \
  "simulate_load(requests_per_sec=100, duration_seconds=300)"
```

---

## Architecture Documentation

### 6.1 Component Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                    XKernal L2 Runtime                       │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌──────────────────────────────────────────────────────┐  │
│  │              Agent Manager                            │  │
│  │  ┌────────────┐  ┌──────────────┐  ┌────────────┐   │  │
│  │  │Lifecycle   │  │Resource      │  │Config      │   │  │
│  │  │Controller  │  │Allocator     │  │Store       │   │  │
│  │  └────────────┘  └──────────────┘  └────────────┘   │  │
│  └──────────────────────────────────────────────────────┘  │
│                           │                                  │
│                           ▼                                  │
│  ┌──────────────────────────────────────────────────────┐  │
│  │          Agent Instance (per replica)                │  │
│  │  ┌────────────┐  ┌──────────────┐  ┌────────────┐   │  │
│  │  │Inference   │  │Memory Tiers  │  │Mount       │   │  │
│  │  │Engine      │  │(Transient,   │  │Manager     │   │  │
│  │  │            │  │Persistent)   │  │            │   │  │
│  │  └────────────┘  └──────────────┘  └────────────┘   │  │
│  └──────────────────────────────────────────────────────┘  │
│          │                      │               │            │
│          ▼                      ▼               ▼            │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │Health Monitor│  │IPC Router    │  │Mount         │     │
│  │              │  │              │  │Resolver      │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
│          │                                     │             │
└──────────┼──────────────────────────────────────┼───────────┘
           │                                     │
           ▼ (metrics,alerts)                   ▼ (queries)
    ┌──────────────┐                   ┌─────────────────┐
    │Observability │                   │ Knowledge       │
    │ (Prometheus) │                   │ Sources:        │
    └──────────────┘                   │ - HTTP/REST     │
                                       │ - S3/Object     │
                                       │ - Database      │
                                       │ - Local FS      │
                                       │ - Plugins       │
                                       └─────────────────┘
```

---

### 6.2 Data Flow: Deploy to Terminate

```
1. DEPLOY PHASE
   ┌─────────────┐
   │Unit File    │
   │(.agent.toml)│
   └──────┬──────┘
          │
          ▼
   ┌─────────────────────────────────┐
   │1. Parse & Validate              │
   │   - TOML syntax check           │
   │   - Field requirements          │
   │   - Cross-reference validation  │
   └──────┬──────────────────────────┘
          │
          ▼
   ┌─────────────────────────────────┐
   │2. Provision Agent               │
   │   - Create agent namespace      │
   │   - Allocate resources          │
   │   - Store config in Config Store│
   └──────┬──────────────────────────┘
          │
          ▼
   ┌─────────────────────────────────┐
   │Status: CREATED                  │
   │Next: cs-agentctl start          │
   └─────────────────────────────────┘

2. INITIALIZATION PHASE
   ┌──────────────┐
   │cs-agentctl   │
   │start         │
   └──────┬───────┘
          │
          ▼
   ┌─────────────────────────────────┐
   │1. Load Unit & Config             │
   └──────┬──────────────────────────┘
          │
          ▼
   ┌─────────────────────────────────┐
   │2. Initialize Memory Tiers        │
   │   - Allocate transient (RAM)     │
   │   - Connect persistent (disk)    │
   └──────┬──────────────────────────┘
          │
          ▼
   ┌─────────────────────────────────┐
   │3. Mount Knowledge Sources        │
   │   - Validate connectivity        │
   │   - Warm caches                  │
   └──────┬──────────────────────────┘
          │
          ▼
   ┌─────────────────────────────────┐
   │4. Spawn Process                  │
   │   - Allocate PID                 │
   │   - Inject environment variables │
   │   - Set resource limits (cgroups)│
   └──────┬──────────────────────────┘
          │
          ▼
   ┌─────────────────────────────────┐
   │Status: STARTING                  │
   │Running initialization hooks      │
   └─────────────────────────────────┘

3. READINESS PHASE
   ┌─────────────────────────────────┐
   │1. Run Health Probes              │
   │   - Liveness (EXEC/HTTP/TCP)     │
   │   - Timeout & failure threshold  │
   └──────┬──────────────────────────┘
          │
          ▼
   ┌─────────────────────────────────┐
   │2. Enable IPC Channels            │
   │   - Register peer connections    │
   │   - Activate message queues      │
   └──────┬──────────────────────────┘
          │
          ▼
   ┌─────────────────────────────────┐
   │3. Start Monitoring               │
   │   - Health check watchers        │
   │   - Metrics collectors           │
   └──────┬──────────────────────────┘
          │
          ▼
   ┌─────────────────────────────────┐
   │Status: READY                     │
   │Agent operational                 │
   └─────────────────────────────────┘

4. RUNTIME PHASE (Steady State)
   ┌─────────────────────────────────┐
   │Continuous Monitoring Loop        │
   │┌─────────────────────────────┐   │
   ││ Every 30 seconds:           │   │
   ││ - Run health probes         │   │
   ││ - Check mount connectivity  │   │
   ││ - Measure CPU/memory        │   │
   ││ - Evaluate autoscale policy │   │
   └─────────────────────────────────┘
          │
          ├─→ All healthy?
          │      │
          │      ├─YES─→ Continue
          │      │
          │      └─NO─→ Escalate alert
          │             Optionally: Restart
          │
          └─→ Autoscale threshold hit?
                 │
                 ├─UP─→ Spawn new replicas
                 │      (via Lifecycle Controller)
                 │
                 └─DOWN─→ Drain & terminate
                          (via Lifecycle Controller)

5. TERMINATION PHASE
   ┌──────────────────────┐
   │cs-agentctl stop      │
   │(or autoscale down)   │
   └──────┬───────────────┘
          │
          ▼
   ┌─────────────────────────────────┐
   │1. Send SIGTERM (graceful)       │
   │   - Timeout: 30 seconds (config)│
   └──────┬──────────────────────────┘
          │
          ▼
   ┌─────────────────────────────────┐
   │2. Drain In-Flight Requests      │
   │   - Wait for ongoing ops        │
   └──────┬──────────────────────────┘
          │
          ▼
   ┌─────────────────────────────────┐
   │3. Flush Persistent State        │
   │   - Sync memory tiers to disk    │
   │   - Close database connections  │
   │   - Unmount sources             │
   └──────┬──────────────────────────┘
          │
          ▼
   ┌─────────────────────────────────┐
   │4. Cleanup Resources             │
   │   - Release memory              │
   │   - Close file descriptors      │
   │   - Unregister from load bal.   │
   └──────┬──────────────────────────┘
          │
          ▼
   ┌─────────────────────────────────┐
   │Status: STOPPED                  │
   │Process terminated               │
   └─────────────────────────────────┘

6. FAILURE RECOVERY (if enabled)
   ┌─────────────────────────────────┐
   │Health check failed 3 times       │
   │(failure_threshold = 3)          │
   └──────┬──────────────────────────┘
          │
          ▼
   ┌─────────────────────────────────┐
   │Check restart_policy             │
   └──────┬──────────────────────────┘
          │
          ├─ON_FAILURE─────→ Restart (with backoff)
          │                  Delay: 5 seconds × attempt #
          │                  Max attempts: 3
          │
          ├─ALWAYS─────────→ Auto-restart (no threshold)
          │
          └─NEVER──────────→ Manual intervention required
                              Alert operator
```

---

### 6.3 Component Interaction Example: Mount Query

```
User Query: "Search knowledge base for documents on AI safety"
        │
        ▼
┌────────────────────────┐
│ Agent Inference Engine │
│ - Receives query       │
│ - Plans tool use       │
│ - Calls Mount Manager  │
└────┬───────────────────┘
     │
     ▼
┌────────────────────────────────────────┐
│ Mount Manager                          │
│ 1. Identify target mount: "web_kb"    │
│ 2. Check if cached locally             │
└────┬───────────────────────────────────┘
     │
     ├─→ Cache HIT (71% hit rate)
     │   │
     │   └─→ Return cached result (2ms)
     │
     └─→ Cache MISS
         │
         ▼
    ┌──────────────────────────────────┐
    │ Mount Resolver (HTTP_API)        │
    │ 1. Check mount config            │
    │ 2. Load auth (bearer token)      │
    │ 3. Build request:                │
    │    GET /v1/documents             │
    │    ?query=AI+safety              │
    │ 4. Execute HTTP call             │
    │    Timeout: 30s                  │
    └────┬─────────────────────────────┘
         │
         ▼
    ┌──────────────────────────────────┐
    │ External API: knowledge.internal │
    │ - Parse query                    │
    │ - Search vector DB               │
    │ - Return 10 docs (top-k)         │
    │ - Response time: ~200ms          │
    └────┬─────────────────────────────┘
         │
         ▼
    ┌──────────────────────────────────┐
    │ Mount Manager (on response)      │
    │ 1. Parse JSON response           │
    │ 2. Validate schema               │
    │ 3. Update cache (TTL: 600s)      │
    │ 4. Record metrics:               │
    │    - Latency: 205ms (p99: 287ms) │
    │    - Cache misses: +1            │
    │ 5. Return results to agent       │
    └────┬─────────────────────────────┘
         │
         ▼
    ┌──────────────────────────────────┐
    │ Agent Inference Engine           │
    │ - Process returned documents     │
    │ - Extract relevant excerpts      │
    │ - Generate summary answer        │
    │ Total time: 350ms                │
    └──────────────────────────────────┘
```

---

## Summary

The XKernal Agent System provides a production-grade platform for deploying autonomous agents at scale. This documentation specifies the unit file format (RFC), mount system, CLI interface, operational procedures, developer patterns, and system architecture.

**Key Design Principles:**
- **Declarative Configuration**: Unit files express intent, not implementation
- **Observable**: Comprehensive health monitoring and metrics
- **Resilient**: Automatic recovery, graceful degradation
- **Scalable**: Horizontal scaling, workload distribution
- **Secure**: Capability-based access, secret management, permission scoping

**For additional support:** Contact the Semantic FS & Agent Lifecycle team or consult the L2 Runtime documentation.

---

**Document Version:** 1.0.0
**Last Updated:** 2026-03-02
**Status:** Production Ready (RFC v1)
