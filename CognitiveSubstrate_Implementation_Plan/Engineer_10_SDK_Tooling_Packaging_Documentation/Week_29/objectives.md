# Engineer 10 — SDK: Tooling, Packaging & Documentation — Week 29

## Phase: 3 (Cloud Deployment, Documentation & Launch)

## Weekly Objective
Launch Documentation Portal at docs.cognitivesubstrate.dev. Implement CSCI Reference documentation with complete syscall documentation. Create Getting Started guide (Hello World in 15 min). Begin Migration Guides for LangChain, SK, CrewAI.

## Document References
- **Primary:** Section 3.5.6 — Documentation Portal: CSCI Reference, Getting Started, Migration Guides, Policy Cookbook, ADRs, API Playground
- **Supporting:** Section 6.4 — Phase 3, Week 29-30

## Deliverables
- [ ] Documentation Portal infrastructure (docs.cognitivesubstrate.dev)
- [ ] CSCI Reference documentation (complete syscall docs)
- [ ] Getting Started guide (Hello World in 15 minutes)
- [ ] Migration Guides: LangChain to Cognitive Substrate
- [ ] Migration Guides: Semantic Kernel to Cognitive Substrate
- [ ] Migration Guides: CrewAI to Cognitive Substrate
- [ ] Policy Cookbook (enforcement patterns, examples)
- [ ] ADRs (Architecture Decision Records) from Phase 1-3
- [ ] Documentation portal styling and navigation

## Technical Specifications
### Documentation Portal Structure
```
docs.cognitivesubstrate.dev/
├── /csci-reference/               # CSCI syscall documentation
│   ├── index.md                   # Overview
│   ├── syscalls/                  # Individual syscall pages
│   │   ├── capability-query.md
│   │   ├── capability-grant.md
│   │   ├── memory-allocate.md
│   │   └── ... (20+ syscalls)
│   └── examples/
├── /getting-started/              # Hello World in 15 min
│   ├── index.md
│   ├── setup.md
│   ├── first-ct.md
│   ├── first-agent.md
│   └── hello-world-example/
├── /migration-guides/             # Migration from other frameworks
│   ├── langchain.md
│   ├── semantic-kernel.md
│   ├── crewai.md
│   └── custom-framework.md
├── /policy-cookbook/              # Common policy patterns
│   ├── index.md
│   ├── cost-limits.md
│   ├── audit-logging.md
│   ├── capability-templates.md
│   └── examples/
├── /adrs/                         # Architecture Decision Records
│   ├── adr-001-monorepo.md
│   ├── adr-002-csci-design.md
│   └── ... (comprehensive ADRs)
└── /api-playground/               # Interactive API explorer (Week 31-32)
```

### CSCI Reference: Syscall Documentation Template
```markdown
# SYSCALL_CAPABILITY_QUERY

## Description
Query the capability graph to determine if an agent has a specific capability.

## Signature
```rust
pub fn syscall_capability_query(
    capability: Capability,
    target_agent: AgentId,
) -> Result<bool, CapabilityError>
```

## Parameters
- `capability: Capability` - Capability to query (e.g., tool_invoke, memory_allocate)
- `target_agent: AgentId` - Agent to check capability for

## Return Value
- `Ok(true)` - Agent has capability
- `Ok(false)` - Agent does not have capability
- `Err(CapabilityError)` - Permission denied or query failed

## Cost
- **Inference cost:** $0.0001 per query
- **Latency:** <1ms (cached)

## Examples
```rust
// Check if assistant has tool_invoke capability
match syscall_capability_query(Capability::ToolInvoke, assistant_id) {
    Ok(true) => println!("Assistant can invoke tools"),
    Ok(false) => println!("Assistant cannot invoke tools"),
    Err(e) => eprintln!("Error: {}", e),
}
```

## Related Syscalls
- SYSCALL_CAPABILITY_GRANT
- SYSCALL_CAPABILITY_REVOKE
```

### Getting Started: Hello World in 15 Minutes
```markdown
# Getting Started: Hello World in 15 Minutes

## Prerequisites
- 5 minutes: Install Cognitive Substrate SDK
- 5 minutes: Create first agent and CT
- 5 minutes: Deploy and run

### Step 1: Install SDK (5 min)
```bash
curl -sSL https://install.cognitivesubstrate.dev | sh
cs-sdk init my-first-app
cd my-first-app
```

### Step 2: Create Hello World Agent (5 min)
```rust
use cs_sdk::prelude::*;

#[derive(Agent)]
struct HelloWorldAgent {
    name: String,
}

impl HelloWorldAgent {
    pub async fn greet(&self, name: &str) -> String {
        format!("Hello, {}! I'm {}.", name, self.name)
    }
}

#[tokio::main]
async fn main() {
    let agent = HelloWorldAgent {
        name: "CS Assistant".to_string(),
    };

    let greeting = agent.greet("World").await;
    println!("{}", greeting);
}
```

### Step 3: Run Your First CT (5 min)
```bash
cargo build
cargo run
```

Expected output:
```
Hello, World! I'm CS Assistant.
```

Congratulations! You've created your first Cognitive Substrate application.
```

### Migration Guide: LangChain Example
```markdown
# Migrating from LangChain to Cognitive Substrate

## Side-by-Side Comparison

### LangChain Code
```python
from langchain import OpenAI, PromptTemplate, LLMChain

llm = OpenAI(temperature=0)
prompt = PromptTemplate(
    input_variables=["topic"],
    template="Write an essay about {topic}",
)
chain = LLMChain(llm=llm, prompt=prompt)

result = chain.run(topic="AI")
```

### Cognitive Substrate Code
```rust
use cs_sdk::prelude::*;

#[derive(Agent)]
struct EssayWriter;

impl EssayWriter {
    pub async fn write_essay(&self, topic: &str) -> String {
        let ct = CT::new()
            .with_capability(Capability::ToolInvoke)
            .with_model("gpt-4")
            .with_prompt(&format!("Write an essay about {}", topic));

        ct.execute().await.unwrap()
    }
}
```

## Key Differences
1. **Type Safety:** Cognitive Substrate uses Rust's type system
2. **Cost Transparency:** Every operation has explicit cost tracking
3. **Capability-Based Security:** Agents have explicit capabilities
4. **Built-in Debugging:** cs-trace, cs-profile, cs-replay out of box
```

## Dependencies
- **Blocked by:** Week 26-28 cloud deployment complete
- **Blocking:** Week 30 policy cookbook completion, Week 31-32 API playground

## Acceptance Criteria
- [ ] Documentation portal launches with <2 second load time
- [ ] CSCI Reference has 20+ complete syscall documentations
- [ ] Getting Started guide enables users to build first app in 15 minutes
- [ ] Migration guides provide side-by-side code comparisons
- [ ] Policy Cookbook has 5+ real-world policy examples
- [ ] ADRs document all major architectural decisions
- [ ] Documentation is searchable and well-organized

## Design Principles Alignment
- **Cognitive-Native:** Documentation reflects cognitive execution model
- **Debuggability:** CSCI Reference enables developers to write correct code
- **Cost Transparency:** Cost information documented for all syscalls
- **Accessibility:** Getting Started and migration guides lower barrier to entry
