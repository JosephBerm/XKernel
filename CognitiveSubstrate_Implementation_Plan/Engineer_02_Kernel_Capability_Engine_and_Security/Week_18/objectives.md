# Engineer 2 — Kernel: Capability Engine & Security — Week 18

## Phase: PHASE 2 - Data Governance & Performance

## Weekly Objective
Implement output gates for data inspection and filtering. Prevent sensitive data from leaving system via tool calls, IPC, and external APIs. Support policy-based filtering and transformation.

## Document References
- **Primary:** Section 3.3.5 (Output Gates & Data Filtering), Section 3.3.5 (Data Governance Overview)
- **Supporting:** Week 15-17 (data governance), Engineer 4 (Tool Interface), Engineer 5 (IPC)

## Deliverables
- [ ] Output gate abstraction layer (intercepts data egress)
- [ ] Data inspection pipeline (classify data on output)
- [ ] Policy-based filtering rules (allow/deny/redact based on classification)
- [ ] Redaction engine (replace sensitive data with placeholders)
- [ ] Integration with tool calls (Engineer 4)
- [ ] Integration with IPC (Engineer 5)
- [ ] Integration with external API calls
- [ ] Audit logging for all output gate decisions
- [ ] Comprehensive test suite (180+ tests)
- [ ] Performance impact assessment (<10ms latency for output filtering)

## Technical Specifications
- **Output Gate Abstraction:**
  - Intercepts all data leaving agent context:
    - Tool call arguments
    - IPC messages
    - External API calls
    - Network responses
  - Gate ID: unique identifier per output channel
  - Policy: routing rules for classification-based filtering
  - State: audit logs, statistics (bytes filtered, items blocked)
  - Latency target: <5000ns overhead per output
- **Data Inspection Pipeline:**
  - Stage 1: Classification detection (scan for PII patterns)
    - Regex patterns: SSN (\d{3}-\d{2}-\d{4}), email ([a-z]+@[a-z]+\.[a-z]+)
    - ML-based: detect medical terms, financial keywords
    - Taint-based: use classification tags from data governance
  - Stage 2: Policy evaluation (check against output policies)
    - Rule 1: PII → tool_calls = deny
    - Rule 2: PII → ipc = audit (if authorized capability)
    - Rule 3: API_KEY → any_output = deny
    - Rule 4: PHI → external_api = audit (comply with HIPAA)
  - Stage 3: Action (apply filtering/redaction)
    - Action 1: deny (block output, return error to agent)
    - Action 2: audit (log + allow)
    - Action 3: redact (replace sensitive data)
- **Policy-Based Filtering Rules:**
  - Syntax: output_channel(classification_tag) → action
  - Examples:
    - tool_call(PII) → deny
    - ipc(PII) → audit_if_capable("READ_PII")
    - external_api(API_KEY) → deny
    - external_api(PII) → redact(keep_first_letter_last_n)
  - Composition: multiple rules can apply (most restrictive wins)
  - Dynamic update: policies can be updated without restart
- **Redaction Engine:**
  - Pattern-based redaction:
    - SSN: \d{3}-\d{2}-\d{4} → XXX-XX-XXXX
    - Email: \w+@\w+\.\w+ → [REDACTED]
    - Phone: \d{3}-\d{3}-\d{4} → XXX-XXX-XXXX
  - Semantic redaction:
    - Replace detected PII with safe placeholders
    - Maintain data structure (if token list, preserve count)
    - Optional: keep first/last N characters for context
  - Consistency: same PII redacted to same placeholder (in audit context)
- **Tool Call Integration (Engineer 4):**
  - Tool invocation: agent calls external tool with arguments
  - Output gate intercepts: before tool receives arguments
  - Inspection: scan arguments for sensitive data
  - Action: deny (block tool call), redact (pass cleaned args), or audit (pass + log)
  - Error handling: if blocked, return error to agent
- **IPC Integration (Engineer 5):**
  - IPC send: agent sends message to another agent
  - Output gate intercepts: before IPC dispatch
  - Inspection: scan message for sensitive data
  - Action: check if recipient has READ_DATA_TYPE capability
  - Policy: if PII and no capability → deny/redact based on policy
- **External API Integration:**
  - API call: HTTP request to external service (e.g., LLM API, database)
  - Output gate intercepts: before HTTP dispatch
  - Inspection: scan request body for sensitive data
  - Policy: strict for external APIs (deny API_KEY always, redact PII)
  - Audit: log all external API calls that touched sensitive data
- **Audit Logging:**
  - Entry: (timestamp, agent_id, output_channel, action, classification_tags, data_summary)
  - Summary: (input_size, output_size, items_blocked, items_redacted)
  - Redaction: (pattern_matched, replacement_count, sample_before, sample_after)
  - Audit retention: 90 days (configurable)
- **Performance Considerations:**
  - Fast path: no sensitive data → direct output, <100ns
  - Slow path: sensitive data → inspection + filtering, <5000ns
  - Batching: multiple outputs batched for efficiency
  - Caching: classification results cached per output

## Dependencies
- **Blocked by:** Week 17 (data governance completion), Engineer 4 (tool calls), Engineer 5 (IPC)
- **Blocking:** Week 19 (continuation and integration), Week 20-22 (KV-cache isolation)

## Acceptance Criteria
- Output gates successfully intercept all output channels
- Data inspection pipeline correctly identifies sensitive data
- Policy-based filtering prevents unauthorized egress
- Redaction engine produces safe, consistent redactions
- Tool call integration prevents sensitive args transmission
- IPC integration respects capability-based data sharing
- External API integration prevents credential leakage
- Audit logging captures all filtering decisions
- All 180+ tests pass
- <5000ns latency for typical output filtering
- Code review completed by security and data governance teams

## Design Principles Alignment
- **P1 (Security-First):** Output gates prevent data leakage
- **P2 (Transparency):** Audit logs document all filtering decisions
- **P3 (Granular Control):** Policy-based filtering enables fine-grained control
- **P4 (Performance):** <5000ns overhead for typical case
- **P6 (Compliance & Audit):** Output gates support GDPR/HIPAA compliance
