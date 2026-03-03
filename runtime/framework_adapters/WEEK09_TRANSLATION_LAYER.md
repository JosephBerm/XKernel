# Week 9 Deliverable: Adapter Translation Layer Design (Phase 1)

## Overview

This document details the complete translation layer architecture that converts framework-agnostic agent concepts into CSCI (Composable Semantic Computation Interface) syscalls. The translation pipeline standardizes how LangChain, Semantic Kernel, AutoGen, CrewAI, and custom frameworks express computation, enabling XKernal's unified execution model.

**Scope:** Framework concept mapping, chain-to-DAG translation, memory model bridging, tool adaptation, and context propagation across all supported frameworks.

---

## 1. Translation Layer Design Document

### 1.1 Core Principles

The translation layer operates on three fundamental principles:

1. **Framework Opacity**: Preserve framework semantics while normalizing to CSCI primitives
2. **Semantic Preservation**: Maintain intent through translation (dependencies, ordering, control flow)
3. **Context Continuity**: Propagate agent_id, user_id, session_id through all translation steps

### 1.2 Framework Concept → CSCI Syscall Mapping

```
┌─────────────────────────────────────────────────────────────────┐
│                    Framework Concept Layer                       │
│  (Chains, Steps, Tasks, Functions, Messages, Tools, State)      │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         │ TranslationPipeline
                         ↓
┌─────────────────────────────────────────────────────────────────┐
│                   CSCI Syscall Layer                             │
│  (CT Spawn, Reads, Writes, SemanticChannel Ops, Barriers)      │
└─────────────────────────────────────────────────────────────────┘
```

### 1.3 Universal Mapping Table

| Framework Concept | CSCI Target | Mapping Strategy |
|---|---|---|
| Step/Task/Function | CT (Computation Token) spawn | Dependency parsing → CT spawn batch |
| Step dependency | CT dependency graph | Topological sort → spawn ordering |
| Tool invocation | ToolBinding CT | Tool capability → syscall parameters |
| Agent message | SemanticChannel write | User_id + agent_id + session_id tagging |
| Volatile state | L2 ephemeral write | Scoped to execution frame |
| Named memory | L3 semantic write | Namespace = framework_id.store_name |
| Control flow | CT barrier/merge | Conditional spawn, loop unrolling |
| Exception | CT abort + context preservation | Error propagation with full state |

---

## 2. Chain-to-DAG Algorithm

### 2.1 Algorithm Overview

Converts sequential framework chains into directed acyclic graphs (DAGs) of CSCI computation tokens.

**Input:** Framework-native chain representation (steps with optional dependencies)
**Output:** DAG specification + CT spawn batch with dependency metadata

### 2.2 Pseudocode

```
Algorithm: ChainToDAG(chain)
  Input: chain - framework chain object with steps
  Output: dag - DAG of computation nodes, spawn_batch - CT spawn commands

  steps ← ExtractSteps(chain)
  explicit_deps ← ExtractDependencies(chain)
  implicit_deps ← InferDependencies(steps)
  all_deps ← Merge(explicit_deps, implicit_deps)

  dag ← BuildDAG(steps, all_deps)
  ValidateAcyclic(dag)

  spawn_batch ← []
  for each node in TopologicalSort(dag):
    ct ← CreateCT(node.framework_id, node.handler, node.inputs)
    ct.dependencies ← GetDependentCTs(node, dag)
    ct.context ← CopyContext(chain.context)
    spawn_batch.Append(ct)

  return (dag, spawn_batch)

Procedure: ExtractDependencies(chain)
  // Framework-specific extraction
  if chain.type == "LangChain":
    return ExtractRunnable Dependencies(chain)
  else if chain.type == "SemanticKernel":
    return ExtractPlannerDependencies(chain)
  // ... per-framework logic

Procedure: InferDependencies(steps)
  // Discover implicit dependencies from variable flow
  deps ← []
  for i, step in steps:
    for j < i, prev_step in steps:
      if StepOutputUsedByStep(prev_step, step):
        deps.Append((prev_step, step))
  return deps

Procedure: CreateCT(framework_id, handler, inputs)
  return ComputationToken {
    id: GenerateUUID(),
    framework_id: framework_id,
    handler_ref: handler,
    input_args: inputs,
    spawned_at: CurrentTimestamp(),
    context: EmptyContext()
  }
```

### 2.3 Example: Sequential Chain

**Framework Input (LangChain-like):**
```
chain = Chain([
  Step("fetch_user", "user_repo.get_by_id"),
  Step("fetch_profile", "profile_repo.get_by_user_id", depends_on=["fetch_user"]),
  Step("enrich_data", "enricher.enrich", depends_on=["fetch_user", "fetch_profile"])
])
```

**DAG Output:**
```
Node 0 (fetch_user)
  │
  ├─ deps: []
  └─ outputs: [user_data]

Node 1 (fetch_profile)
  │
  ├─ deps: [Node 0]
  └─ outputs: [profile_data]

Node 2 (enrich_data)
  │
  ├─ deps: [Node 0, Node 1]
  └─ outputs: [enriched_result]
```

**Spawn Batch:**
```json
{
  "spawn_requests": [
    {
      "ct_id": "ct-001",
      "handler": "user_repo.get_by_id",
      "framework_id": "langchain",
      "dependencies": [],
      "context": {
        "agent_id": "agent-123",
        "user_id": "user-456",
        "session_id": "sess-789"
      }
    },
    {
      "ct_id": "ct-002",
      "handler": "profile_repo.get_by_user_id",
      "framework_id": "langchain",
      "dependencies": ["ct-001"],
      "context": { ... }
    },
    {
      "ct_id": "ct-003",
      "handler": "enricher.enrich",
      "framework_id": "langchain",
      "dependencies": ["ct-001", "ct-002"],
      "context": { ... }
    }
  ]
}
```

---

## 3. Framework-Specific Translations

### 3.1 LangChain: Chain.invoke → CSCI

**Concept:** Sequential or branching Runnables with input/output contracts.

**Translation:**
- `Chain.invoke()` → CT spawn batch
- `Runnable` → CT handler reference
- `RunnableParallel` → parallel CT spawns with merge barrier
- `RunnableSequence` → sequential CT dependency chain
- `RunnableBranch` → conditional CT spawning
- Runnable context (input, config) → CT input_args + syscall context

**Example:**
```
from langchain.schema import RunnableSequence

chain = (
    {"input": RunnablePassthrough()}
    | retriever
    | prompt_template
    | llm
    | output_parser
)

# Translation:
# Step 1: Retrieve (input → retriever) → CT spawn
# Step 2: Template (retrieve_output → prompt) → CT spawn (depends on Step 1)
# Step 3: LLM (prompt_output → llm) → CT spawn (depends on Step 2)
# Step 4: Parse (llm_output → parser) → CT spawn (depends on Step 3)
```

### 3.2 Semantic Kernel: Planner Output → CSCI

**Concept:** Planner produces step plan with function calls and parameter bindings.

**Translation:**
- `Plan.steps` → CT spawn list
- Function call → CT with kernel function handler
- Parameter references → CT input_args with variable substitution
- Plan context variables → L3 semantic store writes
- Subtask dependencies → CT dependency graph

**Example:**
```
plan = planner.create_plan(goal="Summarize documents")
# Planner output:
# Step 1: read_file(filepath)
# Step 2: chunk_text(text from Step 1)
# Step 3: summarize_chunk(chunk from Step 2) [parallel for each chunk]
# Step 4: merge_summaries(summaries from Step 3)

# Translation produces:
# CT: read_file → CT: chunk_text (dep: read_file) →
#   [parallel CT: summarize_chunk (dep: chunk_text)] → CT: merge_summaries (dep: all)
```

### 3.3 AutoGen: Function Calls → CSCI

**Concept:** Multi-agent conversation with function call requests/responses.

**Translation:**
- Agent message with function call → CT spawn
- Function arguments → CT input_args
- Function return value → SemanticChannel write (tagged with source agent)
- Conversation history → L3 semantic store append
- Tool registry → ToolBinding registrations

**Example:**
```
# AutoGen agent generates:
{
  "type": "function_call",
  "function": {
    "name": "search_api",
    "arguments": {"query": "XKernal architecture"}
  }
}

# Translation:
# CT spawn: handler="search_api", input_args={query: "XKernal architecture"}
# On completion: SemanticChannel write with (agent_id, user_id, session_id) tags
```

### 3.4 CrewAI: Task Dependencies → CSCI

**Concept:** Tasks with role assignments and explicit dependency DAG.

**Translation:**
- Task → CT spawn
- Task dependencies → CT dependency graph
- Role (agent) assignment → CT capability requirements
- Tool list → required ToolBindings
- Output format → CT result serialization spec

**Example:**
```
task_1 = Task(
    description="Research XKernal design",
    agent=researcher,
    tools=[search_tool, scrape_tool]
)

task_2 = Task(
    description="Summarize findings",
    agent=writer,
    tools=[summarize_tool],
    depends_on=[task_1]
)

# Translation:
# CT-1: handler=researcher.execute, capability_req=[search, scrape]
# CT-2: handler=writer.execute, depends_on=[CT-1], capability_req=[summarize]
```

### 3.5 Custom Framework: Direct API Mapping

**Concept:** Custom frameworks expose computation via direct function/class interfaces.

**Translation:**
- Custom execution function → CT handler via adapter interface
- Function signature → CT input_args schema validation
- Return values → CT output tagging
- State management → L2/L3 store mapping (framework-specified)

---

## 4. Memory Translation

### 4.1 Volatile Memory (L2 Ephemeral)

**Mapping:** Temporary computation state scoped to single execution frame.

- **Framework:** In-memory variables, local state, step outputs
- **CSCI:** L2 ephemeral writes with execution_id scope
- **Lifetime:** From CT spawn to CT completion
- **Garbage Collection:** Automatic on execution frame exit

**Example:**
```
# Framework: LangChain step output variable
step_output = chain.invoke({"input": user_query})

# Translation → L2 ephemeral write:
ephemeral_write {
  key: "step_output",
  value: step_output_data,
  scope: execution_id,
  lifespan: [ct_spawn_timestamp, ct_completion_timestamp]
}
```

### 4.2 Named Memory (L3 Semantic)

**Mapping:** Persistent semantic storage across sessions.

- **Framework:** Named stores, knowledge bases, context managers
- **CSCI:** L3 semantic writes with namespace=framework_id.store_name
- **Lifetime:** Persists across executions
- **Query:** Full semantic search + embedding similarity

**Example:**
```
# Framework: CrewAI knowledge store
crew.memory.add("user_preferences", user_prefs_dict)

# Translation → L3 semantic write:
semantic_write {
  namespace: "crewai.memory",
  key: "user_preferences",
  value: user_prefs_dict,
  embeddings: embed(user_prefs_dict),
  user_id: user_id,
  session_id: session_id
}
```

### 4.3 Memory Translation Pipeline

```
┌─────────────────────────────────────────┐
│   Framework Memory Operations            │
│  (variables, stores, state objects)      │
└────────────┬────────────────────────────┘
             │
             ├─ [volatile] → MemoryTranslator → L2 ephemeral write
             │
             └─ [named]    → MemoryTranslator → L3 semantic write
                              (with embedding)
```

---

## 5. Tool Translation

### 5.1 Tool → ToolBinding Mapping

**Concept:** Framework tools (functions, external APIs, plugins) become standardized ToolBindings.

| Tool Attribute | ToolBinding Field | Mapping |
|---|---|---|
| tool.name | binding.name | Direct copy |
| tool.description | binding.description | Direct copy |
| tool.parameters | binding.input_schema | JSON schema generation |
| tool.return_type | binding.output_schema | Type introspection + schema |
| tool.docstring | binding.documentation | Extracted and stored |
| tool.capabilities | binding.required_capabilities | Framework-specific extraction |
| tool.async | binding.is_async | Framework metadata |

### 5.2 Tool Translation Procedure

```
Procedure: TranslateTool(framework_tool) → ToolBinding
  binding = ToolBinding {
    id: GenerateUUID(),
    name: framework_tool.name,
    description: framework_tool.description,
    framework_id: current_framework_id,
    handler_ref: framework_tool,
    input_schema: GenerateJSONSchema(framework_tool.parameters),
    output_schema: GenerateJSONSchema(framework_tool.return_type),
    documentation: ExtractDocumentation(framework_tool),
    required_capabilities: ExtractCapabilities(framework_tool),
    is_async: IsAsync(framework_tool),
    timeout_ms: ExtractTimeout(framework_tool) or DEFAULT_TIMEOUT
  }
  return binding

Procedure: ExtractCapabilities(tool) → [capability]
  // Infer from tool metadata, decorators, or hints
  // Examples: "file_system", "network_http", "gpu_compute", "database"
  caps ← []
  for annotation in tool.annotations:
    if annotation.type == CapabilityMarker:
      caps.Append(annotation.value)
  return caps
```

### 5.3 Example: LangChain Tool Translation

```python
# Framework input
class CalculatorTool(BaseTool):
    name: str = "calculator"
    description: str = "Performs arithmetic operations"

    def _run(self, expression: str) -> str:
        """Evaluate math expression"""
        return str(eval(expression))

# Translation output (Rust struct instantiation):
ToolBinding {
    id: "tool-calc-001",
    name: "calculator",
    description: "Performs arithmetic operations",
    framework_id: "langchain",
    handler_ref: "<calculator_tool_ref>",
    input_schema: {
        "type": "object",
        "properties": {
            "expression": {"type": "string", "description": "Math expression"}
        },
        "required": ["expression"]
    },
    output_schema: {
        "type": "string",
        "description": "Evaluation result"
    },
    required_capabilities: ["compute"],
    is_async: false,
    timeout_ms: 5000
}
```

---

## 6. Context Propagation

### 6.1 Context Structure

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionContext {
    pub agent_id: String,
    pub user_id: String,
    pub session_id: String,
    pub request_id: String,
    pub created_at: Timestamp,
    pub parent_ct_id: Option<String>,
}
```

### 6.2 Propagation Rules

1. **CT Creation:** Inherit context from parent chain/plan
2. **CT Spawn:** Copy context to all dependent CTs
3. **SemanticChannel Write:** Tag writes with (agent_id, user_id, session_id)
4. **L2/L3 Store Access:** Filter by user_id and session_id
5. **Tool Execution:** Pass context to tool handler; tool may use for authorization
6. **Error Handling:** Preserve context in error reports

### 6.3 Context Flow Example

```
User Request (user_id=u123, session_id=s456)
    ↓
Agent (agent_id=a789) receives request
    ↓
Create ExecutionContext {
    agent_id: "a789",
    user_id: "u123",
    session_id: "s456",
    request_id: "req-001",
    created_at: <now>
}
    ↓
CT Spawn Batch: all CTs inherit this context
    ↓
SemanticChannel writes: tag with (a789, u123, s456)
    ↓
L3 semantic store: filter reads by (user_id=u123, session_id=s456)
```

---

## 7. Translation Pipeline Diagrams

### 7.1 LangChain Translation Flow

```
LangChain Chain
    ↓
[ChainInspector]
    ├─ Extract Runnables
    ├─ Extract Dependencies
    └─ Extract Context
    ↓
[DependencyResolver]
    ├─ Build DAG
    └─ Topological Sort
    ↓
[ContextInjector]
    ├─ Inject agent_id
    ├─ Inject user_id
    └─ Inject session_id
    ↓
[CTSpawner]
    ├─ Create CT per Runnable
    ├─ Assign Dependencies
    └─ Register Handler Refs
    ↓
CSCI CT Spawn Batch
```

### 7.2 Semantic Kernel Translation Flow

```
Semantic Kernel Plan
    ↓
[PlanParser]
    ├─ Extract Steps
    ├─ Extract Function Calls
    └─ Extract Parameters
    ↓
[ParameterResolver]
    ├─ Resolve Variable References
    ├─ Map to Function Signatures
    └─ Build Input Schemas
    ↓
[ContextInjector]
    └─ Inject Execution Context
    ↓
[CTSpawner]
    ├─ Create CT per Step
    ├─ Link Parameter Dependencies
    └─ Register Function Handlers
    ↓
CSCI CT Spawn Batch
```

### 7.3 AutoGen Translation Flow

```
AutoGen Conversation
    ↓
[ConversationMonitor]
    ├─ Detect function_call Messages
    ├─ Extract Function Names & Args
    └─ Map to Tool Registry
    ↓
[ToolBindingResolver]
    ├─ Look up Tool Definition
    ├─ Validate Arguments
    └─ Generate Input Schemas
    ↓
[SemanticChannelWriter]
    ├─ Tag with Agent ID
    ├─ Tag with User ID
    └─ Tag with Session ID
    ↓
[CTSpawner]
    ├─ Create Tool Invocation CT
    ├─ Register Result Callback
    └─ Inject Context
    ↓
CSCI CT Spawn Batch
```

### 7.4 CrewAI Translation Flow

```
CrewAI Task Graph
    ↓
[TaskGraphInspector]
    ├─ Extract Task Definitions
    ├─ Extract Dependencies
    └─ Extract Role Assignments
    ↓
[CapabilityMapper]
    ├─ Map Agent Roles → Capabilities
    ├─ Map Tool Sets → ToolBindings
    └─ Build Requirement Specs
    ↓
[DependencyResolver]
    ├─ Build Task DAG
    └─ Validate Acyclic
    ↓
[ContextInjector]
    ├─ Inject agent_id (from role)
    ├─ Inject user_id
    └─ Inject session_id
    ↓
[CTSpawner]
    ├─ Create CT per Task
    ├─ Assign Task Dependencies
    ├─ Register Capability Requires
    └─ Register Tool Bindings
    ↓
CSCI CT Spawn Batch
```

---

## 8. Rust Implementation

### 8.1 TranslationPipeline

```rust
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionContext {
    pub agent_id: String,
    pub user_id: String,
    pub session_id: String,
    pub request_id: String,
    pub created_at: DateTime<Utc>,
    pub parent_ct_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComputationToken {
    pub id: String,
    pub framework_id: String,
    pub handler_ref: String,
    pub input_args: serde_json::Value,
    pub dependencies: Vec<String>,
    pub context: ExecutionContext,
    pub spawned_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpawnBatch {
    pub batch_id: String,
    pub tokens: Vec<ComputationToken>,
    pub dependency_graph: HashMap<String, Vec<String>>,
}

pub struct TranslationPipeline {
    framework_id: String,
    context: ExecutionContext,
}

impl TranslationPipeline {
    pub fn new(framework_id: String, context: ExecutionContext) -> Self {
        TranslationPipeline { framework_id, context }
    }

    pub fn translate_chain(
        &self,
        chain_spec: serde_json::Value,
    ) -> Result<SpawnBatch, String> {
        let steps = self.extract_steps(&chain_spec)?;
        let dependencies = self.extract_dependencies(&chain_spec)?;
        let dag = self.build_dag(&steps, &dependencies)?;

        self.validate_acyclic(&dag)?;

        let tokens = self.create_tokens_from_dag(&dag)?;
        let batch = SpawnBatch {
            batch_id: Uuid::new_v4().to_string(),
            tokens,
            dependency_graph: dag,
        };

        Ok(batch)
    }

    fn extract_steps(
        &self,
        chain_spec: &serde_json::Value,
    ) -> Result<Vec<String>, String> {
        chain_spec
            .get("steps")
            .and_then(|s| s.as_array())
            .map(|steps| {
                steps
                    .iter()
                    .filter_map(|s| s.get("id").and_then(|id| id.as_str()))
                    .map(|s| s.to_string())
                    .collect()
            })
            .ok_or_else(|| "Failed to extract steps".to_string())
    }

    fn extract_dependencies(
        &self,
        chain_spec: &serde_json::Value,
    ) -> Result<HashMap<String, Vec<String>>, String> {
        let mut deps = HashMap::new();

        if let Some(steps) = chain_spec.get("steps").and_then(|s| s.as_array()) {
            for step in steps {
                let step_id = step
                    .get("id")
                    .and_then(|id| id.as_str())
                    .ok_or("Missing step id")?;

                let step_deps = step
                    .get("dependencies")
                    .and_then(|d| d.as_array())
                    .map(|d| {
                        d.iter()
                            .filter_map(|dep| dep.as_str())
                            .map(|s| s.to_string())
                            .collect()
                    })
                    .unwrap_or_default();

                deps.insert(step_id.to_string(), step_deps);
            }
        }

        Ok(deps)
    }

    fn build_dag(
        &self,
        steps: &[String],
        dependencies: &HashMap<String, Vec<String>>,
    ) -> Result<HashMap<String, Vec<String>>, String> {
        let mut dag = HashMap::new();

        for step in steps {
            let step_deps = dependencies.get(step).cloned().unwrap_or_default();
            dag.insert(step.clone(), step_deps);
        }

        Ok(dag)
    }

    fn validate_acyclic(
        &self,
        dag: &HashMap<String, Vec<String>>,
    ) -> Result<(), String> {
        let mut visited = std::collections::HashSet::new();
        let mut rec_stack = std::collections::HashSet::new();

        for node in dag.keys() {
            if !visited.contains(node) {
                self.has_cycle(node, dag, &mut visited, &mut rec_stack)?;
            }
        }

        Ok(())
    }

    fn has_cycle(
        &self,
        node: &str,
        dag: &HashMap<String, Vec<String>>,
        visited: &mut std::collections::HashSet<String>,
        rec_stack: &mut std::collections::HashSet<String>,
    ) -> Result<(), String> {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());

        if let Some(deps) = dag.get(node) {
            for dep in deps {
                if !visited.contains(dep) {
                    self.has_cycle(dep, dag, visited, rec_stack)?;
                } else if rec_stack.contains(dep) {
                    return Err("Cycle detected in DAG".to_string());
                }
            }
        }

        rec_stack.remove(node);
        Ok(())
    }

    fn create_tokens_from_dag(
        &self,
        dag: &HashMap<String, Vec<String>>,
    ) -> Result<Vec<ComputationToken>, String> {
        let mut tokens = Vec::new();
        let topo_order = self.topological_sort(dag)?;

        for step_id in topo_order {
            let ct = ComputationToken {
                id: format!("ct-{}", Uuid::new_v4()),
                framework_id: self.framework_id.clone(),
                handler_ref: step_id.clone(),
                input_args: serde_json::json!({}),
                dependencies: dag
                    .get(&step_id)
                    .cloned()
                    .unwrap_or_default(),
                context: self.context.clone(),
                spawned_at: Utc::now(),
            };
            tokens.push(ct);
        }

        Ok(tokens)
    }

    fn topological_sort(
        &self,
        dag: &HashMap<String, Vec<String>>,
    ) -> Result<Vec<String>, String> {
        let mut result = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut temp_mark = std::collections::HashSet::new();

        fn visit(
            node: &str,
            dag: &HashMap<String, Vec<String>>,
            visited: &mut std::collections::HashSet<String>,
            temp_mark: &mut std::collections::HashSet<String>,
            result: &mut Vec<String>,
        ) -> Result<(), String> {
            if visited.contains(node) {
                return Ok(());
            }

            if temp_mark.contains(node) {
                return Err("Not a DAG".to_string());
            }

            temp_mark.insert(node.to_string());

            if let Some(deps) = dag.get(node) {
                for dep in deps {
                    visit(dep, dag, visited, temp_mark, result)?;
                }
            }

            temp_mark.remove(node);
            visited.insert(node.to_string());
            result.push(node.to_string());
            Ok(())
        }

        for node in dag.keys() {
            visit(node, dag, &mut visited, &mut temp_mark, &mut result)?;
        }

        result.reverse();
        Ok(result)
    }
}
```

### 8.2 ChainToDagConverter

```rust
use std::collections::HashMap;

pub struct ChainToDagConverter;

impl ChainToDagConverter {
    pub fn convert(
        chain_spec: &serde_json::Value,
    ) -> Result<HashMap<String, Vec<String>>, String> {
        let mut dag = HashMap::new();

        if let Some(steps) = chain_spec.get("steps").and_then(|s| s.as_array()) {
            for (idx, step) in steps.iter().enumerate() {
                let step_id = step
                    .get("id")
                    .and_then(|id| id.as_str())
                    .unwrap_or(&idx.to_string())
                    .to_string();

                let explicit_deps = step
                    .get("depends_on")
                    .and_then(|d| d.as_array())
                    .map(|d| {
                        d.iter()
                            .filter_map(|dep| dep.as_str())
                            .map(|s| s.to_string())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();

                let inferred_deps = Self::infer_dependencies(step, steps, idx);

                let mut all_deps = explicit_deps;
                all_deps.extend(inferred_deps);
                all_deps.sort();
                all_deps.dedup();

                dag.insert(step_id, all_deps);
            }
        }

        Ok(dag)
    }

    fn infer_dependencies(
        step: &serde_json::Value,
        all_steps: &[serde_json::Value],
        current_idx: usize,
    ) -> Vec<String> {
        let mut inferred = Vec::new();

        if let Some(input) = step.get("input") {
            for prev_idx in 0..current_idx {
                if let Some(prev_step) = all_steps.get(prev_idx) {
                    if let Some(prev_outputs) = prev_step.get("outputs").and_then(|o| o.as_array()) {
                        for output in prev_outputs {
                            if let Some(output_str) = output.as_str() {
                                if input.to_string().contains(output_str) {
                                    let prev_id = prev_step
                                        .get("id")
                                        .and_then(|id| id.as_str())
                                        .unwrap_or(&prev_idx.to_string());
                                    inferred.push(prev_id.to_string());
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        inferred
    }
}
```

### 8.3 MemoryTranslator

```rust
use chrono::{DateTime, Utc};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EphemeralWrite {
    pub key: String,
    pub value: serde_json::Value,
    pub scope: String,
    pub execution_id: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SemanticWrite {
    pub namespace: String,
    pub key: String,
    pub value: serde_json::Value,
    pub embeddings: Vec<f32>,
    pub user_id: String,
    pub session_id: String,
    pub created_at: DateTime<Utc>,
}

pub struct MemoryTranslator {
    execution_id: String,
    context: ExecutionContext,
}

impl MemoryTranslator {
    pub fn new(execution_id: String, context: ExecutionContext) -> Self {
        MemoryTranslator { execution_id, context }
    }

    pub fn translate_to_ephemeral(
        &self,
        key: &str,
        value: serde_json::Value,
        lifetime_ms: Option<u64>,
    ) -> EphemeralWrite {
        let now = Utc::now();
        let expires_at = lifetime_ms.map(|ms| {
            now + chrono::Duration::milliseconds(ms as i64)
        });

        EphemeralWrite {
            key: key.to_string(),
            value,
            scope: self.execution_id.clone(),
            execution_id: self.execution_id.clone(),
            created_at: now,
            expires_at,
        }
    }

    pub fn translate_to_semantic(
        &self,
        store_name: &str,
        key: &str,
        value: serde_json::Value,
        embeddings: Vec<f32>,
    ) -> SemanticWrite {
        let namespace = format!("{}.{}", self.context.agent_id, store_name);

        SemanticWrite {
            namespace,
            key: key.to_string(),
            value,
            embeddings,
            user_id: self.context.user_id.clone(),
            session_id: self.context.session_id.clone(),
            created_at: Utc::now(),
        }
    }

    pub fn classify_memory(
        &self,
        memory_spec: &serde_json::Value,
    ) -> Result<MemoryType, String> {
        match memory_spec.get("memory_type").and_then(|t| t.as_str()) {
            Some("volatile") | Some("ephemeral") => Ok(MemoryType::Ephemeral),
            Some("named") | Some("persistent") | Some("semantic") => Ok(MemoryType::Semantic),
            Some(t) => Err(format!("Unknown memory type: {}", t)),
            None => Err("Missing memory_type field".to_string()),
        }
    }
}

#[derive(Debug)]
pub enum MemoryType {
    Ephemeral,
    Semantic,
}
```

### 8.4 ToolTranslator

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolBinding {
    pub id: String,
    pub name: String,
    pub description: String,
    pub framework_id: String,
    pub handler_ref: String,
    pub input_schema: serde_json::Value,
    pub output_schema: serde_json::Value,
    pub documentation: String,
    pub required_capabilities: Vec<String>,
    pub is_async: bool,
    pub timeout_ms: u64,
}

pub struct ToolTranslator {
    framework_id: String,
}

impl ToolTranslator {
    pub fn new(framework_id: String) -> Self {
        ToolTranslator { framework_id }
    }

    pub fn translate_tool(&self, tool_spec: &serde_json::Value) -> Result<ToolBinding, String> {
        let name = tool_spec
            .get("name")
            .and_then(|n| n.as_str())
            .ok_or("Missing tool name")?
            .to_string();

        let description = tool_spec
            .get("description")
            .and_then(|d| d.as_str())
            .unwrap_or("No description")
            .to_string();

        let handler_ref = tool_spec
            .get("handler_ref")
            .and_then(|h| h.as_str())
            .ok_or("Missing handler_ref")?
            .to_string();

        let input_schema = tool_spec
            .get("input_schema")
            .cloned()
            .unwrap_or(serde_json::json!({}));

        let output_schema = tool_spec
            .get("output_schema")
            .cloned()
            .unwrap_or(serde_json::json!({}));

        let required_capabilities = tool_spec
            .get("capabilities")
            .and_then(|c| c.as_array())
            .map(|caps| {
                caps.iter()
                    .filter_map(|cap| cap.as_str())
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default();

        let is_async = tool_spec
            .get("is_async")
            .and_then(|a| a.as_bool())
            .unwrap_or(false);

        let timeout_ms = tool_spec
            .get("timeout_ms")
            .and_then(|t| t.as_u64())
            .unwrap_or(30000);

        let documentation = tool_spec
            .get("documentation")
            .and_then(|d| d.as_str())
            .unwrap_or("")
            .to_string();

        Ok(ToolBinding {
            id: format!("tool-{}", Uuid::new_v4()),
            name,
            description,
            framework_id: self.framework_id.clone(),
            handler_ref,
            input_schema,
            output_schema,
            documentation,
            required_capabilities,
            is_async,
            timeout_ms,
        })
    }

    pub fn validate_tool_invocation(
        &self,
        binding: &ToolBinding,
        arguments: &serde_json::Value,
    ) -> Result<(), String> {
        // Validate arguments against input schema
        if let Some(required) = binding.input_schema.get("required").and_then(|r| r.as_array()) {
            for req_field in required {
                if let Some(field_name) = req_field.as_str() {
                    if !arguments.get(field_name).is_some() {
                        return Err(format!("Missing required argument: {}", field_name));
                    }
                }
            }
        }

        Ok(())
    }
}
```

---

## Summary

This Week 9 deliverable establishes the complete adapter translation layer:

1. **Universal Mapping:** Framework concepts systematically map to CSCI syscalls
2. **Chain-to-DAG:** Algorithms parse sequential workflows into parallel-safe dependency graphs
3. **Framework-Specific:** LangChain, Semantic Kernel, AutoGen, CrewAI get native translation paths
4. **Memory Bridging:** Volatile state → L2 ephemeral; named stores → L3 semantic with embeddings
5. **Tool Standardization:** Framework tools become standardized ToolBindings with schema validation
6. **Context Preservation:** agent_id, user_id, session_id flow through all translation steps
7. **Production Code:** Rust implementation provides type-safe, efficient translation pipeline

**Next Steps (Week 10):** Implement framework-specific adapters for LangChain and Semantic Kernel; add bidirectional result marshaling; performance profiling of translation overhead.
