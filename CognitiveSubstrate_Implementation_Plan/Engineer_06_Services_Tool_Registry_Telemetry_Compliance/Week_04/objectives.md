# Engineer 6 — Services: Tool Registry, Telemetry & Compliance — Week 4

## Phase: Phase 0 (Weeks 1-6)

## Weekly Objective
Stub the Tool Registry with mock registration, effect class enforcement, and default handling. Build the foundational registry structure that will be replaced with MCP-native implementation in Phase 1 Week 7-8.

## Document References
- **Primary:** Section 6.1 (Phase 0, Week 4-6: Stub Tool Registry with effect class defaults), Section 3.3.3 (Tool Registry, effect classes, response caching)
- **Supporting:** Section 2.11 (ToolBinding), Section 3.3.4 (Telemetry basics)

## Deliverables
- [ ] Stub Tool Registry implementation
  - In-memory registration store (no persistence)
  - Register, lookup, list operations
  - Effect class validation and default assignment (WRITE_IRREVERSIBLE for undeclared)
- [ ] Tool registration API
  - Function signature: register_tool(ToolBinding) -> Result<string, Error>
  - Validation: all required fields present, effect_class valid, schema compilable
  - Return registered tool binding ID
- [ ] Effect class enforcement layer
  - At tool invocation: validate effect_class against execution context
  - Runtime constraint: irreversible steps must be last in chain unless PREPARE/COMMIT supported
  - Log violations as audit events
- [ ] Default effect class handler
  - Undeclared tools assigned WRITE_IRREVERSIBLE
  - Log default assignment with tool ID and justification
- [ ] Basic lookup and introspection
  - get_binding(tool_id) -> ToolBinding
  - list_tools_by_effect_class(EffectClass) -> Vec<ToolBinding>
  - list_tools_by_capability(capability) -> Vec<ToolBinding>
- [ ] Mock tool implementations (at least 3 for testing)
  - A READ_ONLY tool (e.g., web search mock)
  - A WRITE_REVERSIBLE tool (e.g., database transaction mock with undo)
  - A WRITE_IRREVERSIBLE tool (e.g., email send mock)
- [ ] Unit tests for registration, lookup, effect class enforcement
- [ ] Documentation: Tool Registry stub architecture and API

## Technical Specifications

### Stub Tool Registry API (Pseudo-code)
```rust
pub struct ToolRegistry {
    bindings: Arc<RwLock<HashMap<String, ToolBinding>>>,
    id_counter: AtomicU64,
}

impl ToolRegistry {
    pub fn new() -> Self { /* ... */ }

    pub async fn register_tool(&self, mut binding: ToolBinding) -> Result<String, RegistryError> {
        // Validate all required fields
        if binding.schema.is_empty() {
            return Err(RegistryError::InvalidSchema);
        }

        // Assign default effect_class if not specified
        if binding.effect_class == None {
            binding.effect_class = EffectClass::WRITE_IRREVERSIBLE;
            // Log: "Tool {} registered with default effect_class WRITE_IRREVERSIBLE"
        }

        let tool_id = format!("tool-{}", self.id_counter.fetch_add(1, Ordering::SeqCst));
        binding.id = tool_id.clone();

        // Validate commit_protocol if present
        if let Some(protocol) = &binding.commit_protocol {
            self.validate_commit_protocol(protocol)?;
        }

        self.bindings.write().await.insert(tool_id.clone(), binding);
        Ok(tool_id)
    }

    pub async fn get_binding(&self, tool_id: &str) -> Result<ToolBinding, RegistryError> {
        self.bindings.read().await
            .get(tool_id)
            .cloned()
            .ok_or(RegistryError::NotFound)
    }

    pub async fn list_by_effect_class(&self, effect_class: EffectClass) -> Vec<ToolBinding> {
        self.bindings.read().await
            .values()
            .filter(|b| b.effect_class == effect_class)
            .cloned()
            .collect()
    }

    pub async fn validate_execution_chain(&self, tool_ids: &[String]) -> Result<(), ChainError> {
        // Runtime constraint: irreversible steps must be last in chain
        let mut last_reversible_idx = None;

        for (idx, tool_id) in tool_ids.iter().enumerate() {
            let binding = self.get_binding(tool_id).await?;
            match binding.effect_class {
                EffectClass::READ_ONLY | EffectClass::WRITE_REVERSIBLE | EffectClass::WRITE_COMPENSABLE => {
                    last_reversible_idx = Some(idx);
                }
                EffectClass::WRITE_IRREVERSIBLE => {
                    if let Some(last_rev_idx) = last_reversible_idx {
                        if last_rev_idx > idx {
                            return Err(ChainError::IrreversibleNotLast);
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
```

### Mock Tools for Testing
```
MockWebSearchTool:
  - ToolBinding ID: "mock-web-search"
  - Effect Class: READ_ONLY
  - Sandbox: Network access to Google only
  - Behavior: Return canned search results

MockDatabaseTool:
  - ToolBinding ID: "mock-database"
  - Effect Class: WRITE_REVERSIBLE
  - Sandbox: Local file system only
  - Behavior: Execute SQL with transaction log; support rollback
  - Commit Protocol: PREPARE/COMMIT

MockEmailTool:
  - ToolBinding ID: "mock-email"
  - Effect Class: WRITE_IRREVERSIBLE
  - Sandbox: Network to smtp.example.com only
  - Behavior: Log email delivery; no actual sending
```

### Effect Class Validation Logic
```
On Tool Invocation:
  1. Look up ToolBinding
  2. Check effect_class against runtime constraints
  3. For WRITE_IRREVERSIBLE: verify no reversible steps follow in execution chain
  4. If constraint violated: emit ConstraintViolation CEF event, return DENIED
  5. If constraint satisfied: proceed with tool execution, emit ToolCallRequested event
```

## Dependencies
- **Blocked by:** Weeks 1-3 (ToolBinding and CEF formalization)
- **Blocking:** Week 5-6 (telemetry engine implementation), Week 7-8 (MCP-native Tool Registry)

## Acceptance Criteria
- [ ] Tool Registry stub implementation compiles and passes unit tests
- [ ] register_tool() assigns WRITE_IRREVERSIBLE default; logged with audit event
- [ ] Effect class enforcement prevents irreversible-followed-by-reversible chains
- [ ] Mock tools registered and callable
- [ ] Lookup API (get_binding, list_by_effect_class, list_by_capability) functional
- [ ] At least 3 mock tools (one per major effect class) implemented
- [ ] Design review; ready for telemetry integration in Week 5-6

## Design Principles Alignment
- **Conservative defaults:** Undeclared tools default to safest effect class
- **Runtime safety:** Execution chains validated before execution
- **Auditability:** All registrations and constraint violations logged
- **Extensibility:** Stub design supports replacement with MCP-native implementation
- **Fail-safe:** Invalid registrations rejected; no silent defaults except effect_class
