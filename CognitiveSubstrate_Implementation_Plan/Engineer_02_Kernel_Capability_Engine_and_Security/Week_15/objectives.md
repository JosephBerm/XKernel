# Engineer 2 — Kernel: Capability Engine & Security — Week 15

## Phase: PHASE 2 - Data Governance & Performance

## Weekly Objective
Implement data governance framework with information-flow controls at page-table level. Create data classification system and establish taint tracking infrastructure for PII-tagged memory regions.

## Document References
- **Primary:** Section 3.3.5 (Data Governance & Information-Flow Controls), Section 3.3.5 (Data Classification Tags)
- **Supporting:** Section 3.2.3 (MMU Integration), Section 2.4 (Capability Constraints)

## Deliverables
- [ ] Data classification tag system (PII, PHI, API_KEY, FINANCIAL, PUBLIC)
- [ ] Page table entry extension for classification metadata
- [ ] Taint propagation algorithm (PII-tagged → CT output tagged)
- [ ] Taint tracking instrumentation (compiler support or runtime)
- [ ] Data flow graph construction and analysis
- [ ] Classification enforcement at page level (read restricted by tag)
- [ ] Comprehensive test suite (150+ tests for classification and taint tracking)
- [ ] Performance impact assessment (<5% overhead for taint tracking)
- [ ] Documentation of data governance model

## Technical Specifications
- **Data Classification Tags:**
  - PII: Personally identifiable information (names, email, SSN, phone)
  - PHI: Protected health information (medical records, health conditions)
  - API_KEY: Authentication credentials (API keys, tokens, passwords)
  - FINANCIAL: Financial data (account numbers, transaction history, balance)
  - PUBLIC: Non-sensitive data (can be shared freely)
  - DYNAMIC: Computed from multiple sources (tag = union of source tags)
  - Each page/memory region assigned single tag or DYNAMIC
- **Page Table Extension:**
  - PTE.classification_tag: 8-bit tag ID (supports up to 256 classification types)
  - PTE.taint_level: 2-bit level (none, transient, persistent)
  - PTE.declassification_allowed: 1-bit flag (can tag be removed?)
  - PTE.output_restricted: 1-bit flag (output requires inspection?)
  - Total overhead: 2 bytes per PTE (cache-line aligned)
- **Taint Propagation Algorithm:**
  - Rule 1: Read PII-tagged page → output is tagged PII
  - Rule 2: Compute on PII-tagged data → result is tagged PII
  - Rule 3: Write PII to untagged page → tag the page
  - Rule 4: Combine multiple tags → union of tags (conjunctive)
  - Rule 5: Declassify → remove tag (only if declassification_allowed)
  - Propagation: forward flow analysis (read-compute-write)
- **Taint Tracking Instrumentation:**
  - Compiler pass: LLVM or Rust compiler plugin
  - Instruments all loads/stores/computes with taint propagation
  - Runtime: shadow memory per byte (stores current taint value)
  - API: tag_set(addr, tag), tag_get(addr) → tag, propagate_taint(src_tag, dst_addr)
  - Zero-cost abstraction: metadata only added at strategic points
- **Data Flow Graph:**
  - Nodes: memory regions, computation operations, output channels
  - Edges: direct flow (read X, compute, write Y), indirect flow (if-then based on X)
  - Construction: static analysis (compile-time) + dynamic analysis (runtime)
  - Visualization: tools to graph data dependencies
  - Breach detection: if PII flows to unrestricted output, alert
- **Classification Enforcement:**
  - Hardware policy: page classified as FINANCIAL can only be read by authorized agents
  - Kernel policy: agents must have explicit capability for classified data
  - Agent A grants FINANCIAL capability → Agent B can read financial data
  - Agent C without grant → cannot read (page fault + SIG_INVALID_ACCESS)
  - Revocation: revoke FINANCIAL capability → all reads rejected
- **Taint Level Semantics:**
  - none: no taint (public data, unclassified)
  - transient: temporary taint (local variable, computed value, will be cleared)
  - persistent: permanent taint (stored in persistent state, cannot be cleared)
  - Enforcement: persistent taint cannot be declassified without authorization

## Dependencies
- **Blocked by:** Week 1-14 (Phase 1 capability engine)
- **Blocking:** Week 16-17 (advanced data governance), Week 18-19 (output gates)

## Acceptance Criteria
- Data classification system supports 5+ tag types extensibly
- Page table metadata added with <2% memory overhead
- Taint propagation correctly traces PII flow
- Taint tracking instrumentation adds <5% performance overhead
- Data flow graph construction complete and accurate
- Classification enforcement prevents unauthorized access
- All 150+ tests pass
- Code review completed by security and data governance teams

## Design Principles Alignment
- **P1 (Security-First):** Classification prevents unauthorized data access
- **P2 (Transparency):** Taint tracking provides visibility into data flow
- **P3 (Granular Control):** Per-page classification enables fine-grained policies
- **P6 (Compliance & Audit):** Data governance supports GDPR, HIPAA compliance
