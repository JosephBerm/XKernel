# Engineer 6 — Services: Tool Registry, Telemetry & Compliance — Week 3

## Phase: Phase 0 (Weeks 1-6)

## Weekly Objective
Design the complete telemetry CEF format with encoding strategy, streaming infrastructure, and cost attribution framework. Bridge Week 2 event definitions with Week 5-6 telemetry engine implementation.

## Document References
- **Primary:** Section 3.3.4 (Cognitive Telemetry Engine, CEF events, cost attribution, real-time streaming)
- **Supporting:** Section 2.11 (ToolBinding cost fields), Section 3.3.5 (Compliance audit trails)

## Deliverables
- [ ] Complete telemetry format design document
  - CEF encoding format specification (JSON, protobuf, or binary)
  - Event serialization with compression strategy
  - Versioning scheme for event schema evolution
- [ ] Cost attribution framework design
  - Token counting methodology (input vs output vs context)
  - GPU-millisecond calculation from execution traces
  - Wall-clock time measurement points
  - TPC (Token Processing Cost) derivation formula
- [ ] Real-time streaming infrastructure design
  - Event buffering strategy (in-memory vs persistent)
  - Subscription API specification (consumers can filter by event type)
  - Message ordering guarantees
  - Backpressure handling
- [ ] Cost metrics validation methodology
  - Accuracy targets (>99% for Phase 3 Week 25-28)
  - Sampling strategy if full emission not feasible
  - Reconciliation against ground truth
- [ ] Event retention policy framework (detailed in Phase 2 Week 19-20)
  - Operational tier (7 days)
  - Compliance tier (≥6 months)
  - Long-term archive (10 years for technical docs)

## Technical Specifications

### CEF Format Specification (JSON)
```json
{
  "cef_version": "1.0",
  "event": {
    "id": "uuid-v4",
    "type": "ThoughtStep|ToolCallRequested|...",
    "ts_utc": 1709251200000000,
    "wall_clock_ms": 15342,
    "actor": "agent-42",
    "resource": "memory:0x7fff0000",
    "action": "READ|WRITE|INVOKE",
    "result": "COMPLETED|FAILED|DENIED",
    "trace_id": "trace-abc123",
    "parent_event_id": "event-xyz",
    "cost": {
      "in_tokens": 1024,
      "out_tokens": 512,
      "gpu_ms": 234.5,
      "wall_ms": 450.0,
      "tpc_hours": 0.0042
    },
    "context": { /* event-type-specific fields */ }
  }
}
```

### Cost Attribution Framework
```
Cost Components:
  - Input Tokens: Count all tokens in request context (prompt + history)
  - Output Tokens: Count generated response tokens
  - GPU-ms: ∑ GPU utilization * execution duration across all GPU tasks
  - Wall-clock: Elapsed time from request start to response complete
  - TPC-hours: (input_tokens + output_tokens) * gpu_utilization * duration_hours / 1M

Accuracy Validation:
  - Compare attributed cost vs actual metering from hardware counters (>99%)
  - Daily reconciliation reports
  - Per-tool accuracy tracking
```

### Real-Time Streaming API (gRPC/WebSocket)
```
service TelemetryStream {
  rpc Subscribe(SubscriptionFilter) returns (stream CEFEvent);
  rpc SubscribeWithAck(SubscriptionFilter) returns (stream AckedEvent);
  rpc GetMetrics(MetricsQuery) returns (MetricsSnapshot);
}

message SubscriptionFilter {
  repeated string event_types;      // Empty = all types
  repeated string actor_filters;    // Filter by agent ID
  repeated string resource_filters; // Filter by resource pattern
  string trace_id_prefix;           // Trace-based filtering
  bool include_cost_metrics;        // Include CostMetrics in stream
}

message AckedEvent {
  CEFEvent event;
  string ack_token;
}
```

### Cost Metrics Validation Plan
- Phase 0 Week 3-4: Design validation framework
- Phase 1 Week 7-10: Instrument tools with hardware counters
- Phase 3 Week 25-28: Run accuracy benchmarks (target >99%)
- Weekly accuracy reports during Phase 1-2

## Dependencies
- **Blocked by:** Week 2 (CEF event types finalized)
- **Blocking:** Week 4-5 (Stub Tool Registry with cost attribution), Week 5-6 (telemetry engine baseline)

## Acceptance Criteria
- [ ] CEF format spec complete with examples for all 10 event types
- [ ] Cost attribution formula documented and reviewable by finance/product
- [ ] Real-time streaming API spec ready for implementation
- [ ] Cost validation methodology includes ground-truth measurement points
- [ ] Retention policy framework (tiers and durations) approved
- [ ] Design review completed; ready for Week 5-6 implementation

## Design Principles Alignment
- **Cost transparency:** Every operation has attributed cost; visible to agents and auditors
- **Real-time observability:** Events flow to subscribers immediately (buffering <1s)
- **Accuracy first:** Cost metrics validated against hardware ground truth
- **Regulatory audit trail:** Retention tiers support compliance requirements (Articles 12, 18, 19)
- **Scalability:** Streaming design supports high-frequency event emission without bottleneck
