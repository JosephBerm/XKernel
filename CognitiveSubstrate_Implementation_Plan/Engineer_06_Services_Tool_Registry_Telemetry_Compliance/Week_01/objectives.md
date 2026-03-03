# Engineer 6 — Services: Tool Registry, Telemetry & Compliance — Week 1

## Phase: Phase 0 (Weeks 1-6)

## Weekly Objective
Formalize the ToolBinding entity with effect class semantics and commit protocol support. Establish the foundational data model and type definitions that will drive Tool Registry, Telemetry Engine, and Compliance Engine throughout all phases.

## Document References
- **Primary:** Section 2.11 (ToolBinding: id, tool, agent, capability, schema, sandbox_config, response_cache with effect_class and commit_protocol)
- **Supporting:** Section 3.3.3 (Tool Registry overview), Section 3.3.4 (Telemetry foundations)

## Deliverables
- [ ] Formalize ToolBinding struct/type definition with all required fields
  - id (unique identifier)
  - tool (tool name/reference)
  - agent (owning agent ID)
  - capability (required capability for use)
  - schema (input/output type specification)
  - sandbox_config (security constraints per tool)
  - response_cache (configuration with TTL and freshness policies)
  - effect_class (READ_ONLY | WRITE_REVERSIBLE | WRITE_COMPENSABLE | WRITE_IRREVERSIBLE)
  - commit_protocol (optional PREPARE/COMMIT two-phase specification)
- [ ] Define effect_class semantics document
  - READ_ONLY: No state mutations
  - WRITE_REVERSIBLE: State changes can be undone (undo stack)
  - WRITE_COMPENSABLE: Changes can be compensated (via inverse transaction)
  - WRITE_IRREVERSIBLE: Changes cannot be undone (default for undeclared tools)
- [ ] Define commit_protocol specification (PREPARE/COMMIT two-phase protocol details)
- [ ] CEF event type enumeration (draft, to be expanded in Week 3-4)
  - Basic structure: timestamp, event_type, actor, resource, action, result, context
- [ ] Design review document for ToolBinding semantics
- [ ] Create type definitions in target language (Rust/C++/Go)

## Technical Specifications

### ToolBinding Entity
```
struct ToolBinding {
    id: string,                    // Unique identifier
    tool: string,                  // Tool name/reference
    agent: string,                 // Owning agent ID
    capability: string,            // Required capability for invocation
    schema: TypeSchema,            // Input/output specification
    sandbox_config: SandboxConfig, // Per-tool security constraints
    response_cache: CacheConfig,   // TTL, freshness, strategy
    effect_class: EffectClass,     // Effect classification
    commit_protocol: Option<CommitProtocol> // Optional two-phase protocol
}

enum EffectClass {
    READ_ONLY,
    WRITE_REVERSIBLE,
    WRITE_COMPENSABLE,
    WRITE_IRREVERSIBLE
}

struct CommitProtocol {
    protocol_type: "PREPARE_COMMIT",
    prepare_timeout_ms: u64,
    commit_timeout_ms: u64,
    rollback_strategy: string
}
```

### Default Behavior Rule
- Tools without explicit effect_class declaration default to WRITE_IRREVERSIBLE
- This conservative default forces explicit declaration for safer effect classes

## Dependencies
- **Blocked by:** Architecture phase completion (Section 2.11 finalized)
- **Blocking:** Week 2 (CEF formalization), Week 4-5 (Stub Tool Registry implementation)

## Acceptance Criteria
- [ ] All ToolBinding fields documented and typed
- [ ] Effect class semantics unambiguous and testable
- [ ] Commit protocol specification complete with examples
- [ ] Type definitions compile without errors in target language
- [ ] Design review sign-off from kernel architect
- [ ] CEF event structure supports all planned event types (10 types target for Week 3-4)

## Design Principles Alignment
- **Explicit over implicit:** Effect classes must be declared; defaults are conservative
- **Composability:** ToolBinding works with response caching and sandbox layers
- **Audit-ready:** All bindings logged for compliance tracking
- **Backward compatibility:** New fields optional where possible; versioning strategy defined
