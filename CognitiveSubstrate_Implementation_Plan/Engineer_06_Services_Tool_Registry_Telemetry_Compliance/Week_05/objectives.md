# Engineer 6 — Services: Tool Registry, Telemetry & Compliance — Week 5

## Phase: Phase 0 (Weeks 1-6)

## Weekly Objective
Begin telemetry engine implementation with basic CEF event emission and cost attribution metadata. Integrate with Stub Tool Registry from Week 4 to emit events on tool registration and invocation.

## Document References
- **Primary:** Section 6.1 (Phase 0, Week 5-6: Begin telemetry engine, cost attribution), Section 3.3.4 (Cognitive Telemetry Engine, cost attribution metadata, real-time streaming)
- **Supporting:** Section 2.11 (ToolBinding cost fields), Week 2 (CEF event types), Week 3 (cost attribution framework), Week 4 (Stub Tool Registry)

## Deliverables
- [ ] Basic CEF event emitter implementation
  - Event creation and serialization (JSON format)
  - Event buffer (in-memory, up to 10k events)
  - Flush mechanism (periodic or on size threshold)
- [ ] Cost attribution metadata structure
  - Input token counter (from request context)
  - Output token counter (from response)
  - GPU-ms tracker (started in this week, detailed instrumentation in Phase 1)
  - Wall-clock timer
  - TPC-hours calculator
- [ ] Integration with Stub Tool Registry
  - Emit ToolRegistered event on register_tool()
  - Emit ToolCallRequested event before tool invocation
  - Emit ToolCallCompleted event after tool invocation (with actual cost)
  - Attach cost metrics to all events
- [ ] Basic event subscriber interface (stub)
  - Subscribe(filter) -> channel of CEFEvent
  - In-memory channel (no persistence)
  - Support filtering by event_type
- [ ] Local event logging (debug)
  - All events logged to stdout/structured log in JSON format
  - Include timestamp, event_type, cost_metrics
- [ ] Unit tests
  - Event emission and serialization
  - Cost metric calculation (basic)
  - Integration with Tool Registry (mock events)

## Technical Specifications

### Telemetry Engine Core (Pseudo-code)
```rust
pub struct TelemetryEngine {
    event_buffer: Arc<Mutex<VecDeque<CEFEvent>>>,
    subscribers: Arc<RwLock<Vec<EventSubscriber>>>,
    token_counter: Arc<TokenCounter>,
    cost_calculator: Arc<CostCalculator>,
}

impl TelemetryEngine {
    pub fn new(max_buffer_size: usize) -> Self {
        Self {
            event_buffer: Arc::new(Mutex::new(VecDeque::with_capacity(max_buffer_size))),
            subscribers: Arc::new(RwLock::new(Vec::new())),
            token_counter: Arc::new(TokenCounter::new()),
            cost_calculator: Arc::new(CostCalculator::new()),
        }
    }

    pub async fn emit_event(&self, event: CEFEvent) -> Result<(), EmitError> {
        // Serialize to JSON
        let json_str = serde_json::to_string(&event)?;

        // Add to in-memory buffer
        let mut buffer = self.event_buffer.lock().await;
        if buffer.len() >= 10_000 {
            buffer.pop_front(); // Evict oldest
        }
        buffer.push_back(event.clone());

        // Log to structured output
        eprintln!("{}", json_str);

        // Notify subscribers
        for subscriber in self.subscribers.read().await.iter() {
            subscriber.send(event.clone()).await.ok();
        }

        Ok(())
    }

    pub async fn subscribe(&self, filter: EventFilter) -> Receiver<CEFEvent> {
        let (tx, rx) = tokio::sync::mpsc::channel(1000);
        let subscriber = EventSubscriber { tx };
        self.subscribers.write().await.push(subscriber);
        rx
    }

    pub fn calculate_cost(&self, input_tokens: u64, output_tokens: u64,
                         gpu_ms: f64, wall_ms: f64) -> CostMetrics {
        CostMetrics {
            input_tokens,
            output_tokens,
            gpu_milliseconds: gpu_ms,
            wall_clock_milliseconds: wall_ms,
            tpc_hours: self.cost_calculator.calculate_tpc_hours(
                input_tokens, output_tokens, gpu_ms
            ),
        }
    }
}

pub struct CostCalculator;

impl CostCalculator {
    pub fn calculate_tpc_hours(&self, input_tokens: u64, output_tokens: u64,
                               gpu_ms: f64) -> f64 {
        let total_tokens = (input_tokens + output_tokens) as f64;
        let gpu_hours = gpu_ms / (1000.0 * 3600.0);
        (total_tokens * gpu_hours) / 1_000_000.0
    }
}

pub struct TokenCounter {
    input_total: AtomicU64,
    output_total: AtomicU64,
}

impl TokenCounter {
    pub fn count_input_tokens(&self, text: &str) -> u64 {
        // Simple tokenization: split on whitespace (refined in Phase 1)
        text.split_whitespace().count() as u64
    }

    pub fn count_output_tokens(&self, text: &str) -> u64 {
        text.split_whitespace().count() as u64
    }
}
```

### Integration with Tool Registry
```rust
impl ToolRegistry {
    pub async fn invoke_tool(&self, tool_id: &str, input: String,
                            telemetry: &TelemetryEngine) -> Result<String, InvokeError> {
        let binding = self.get_binding(tool_id).await?;

        // Emit ToolCallRequested event
        let input_tokens = telemetry.token_counter.count_input_tokens(&input);
        let request_event = CEFEvent {
            event_type: EventType::ToolCallRequested,
            actor: "tool_registry",
            resource: tool_id.to_string(),
            action: "INVOKE",
            cost: CostMetrics {
                input_tokens,
                output_tokens: 0,
                gpu_milliseconds: 0.0,
                wall_clock_milliseconds: 0.0,
                tpc_hours: 0.0,
            },
            context: {
                "tool_binding_id": tool_id.to_string(),
                "capability_required": binding.capability.clone(),
            }.into(),
            ..Default::default()
        };
        telemetry.emit_event(request_event).await?;

        // Execute tool (mock behavior)
        let wall_start = Instant::now();
        let output = format!("Mock output for {}", tool_id);
        let wall_elapsed = wall_start.elapsed();

        // Emit ToolCallCompleted event
        let output_tokens = telemetry.token_counter.count_output_tokens(&output);
        let wall_ms = wall_elapsed.as_secs_f64() * 1000.0;
        let cost = telemetry.calculate_cost(input_tokens, output_tokens, 0.0, wall_ms);

        let completed_event = CEFEvent {
            event_type: EventType::ToolCallCompleted,
            actor: "tool_registry",
            resource: tool_id.to_string(),
            action: "INVOKE",
            result: EventResult::COMPLETED,
            cost,
            context: {
                "tool_binding_id": tool_id.to_string(),
                "execution_time_ms": format!("{}", wall_ms),
            }.into(),
            ..Default::default()
        };
        telemetry.emit_event(completed_event).await?;

        Ok(output)
    }
}
```

### Event Subscriber Interface
```rust
pub struct EventFilter {
    pub event_types: Vec<EventType>,
    pub actor_filter: Option<String>,
    pub resource_filter: Option<String>,
}

pub struct EventSubscriber {
    tx: mpsc::Sender<CEFEvent>,
}

impl EventSubscriber {
    pub async fn send(&self, event: CEFEvent) -> Result<(), SendError<CEFEvent>> {
        self.tx.send(event).await
    }
}
```

## Dependencies
- **Blocked by:** Weeks 1-4 (ToolBinding, CEF events, Stub Tool Registry)
- **Blocking:** Week 6 (complete telemetry engine baseline), Phase 1 Week 7-10 (full implementation)

## Acceptance Criteria
- [ ] CEF event emitter functional; all events serializable to JSON
- [ ] Cost attribution metadata calculated and included in all events
- [ ] ToolCallRequested and ToolCallCompleted events emitted correctly
- [ ] In-memory event buffer holds up to 10k events
- [ ] Token counter functional (basic whitespace splitting)
- [ ] Cost calculator computes TPC-hours from tokens and GPU-ms
- [ ] Event subscriber channel works; supports basic filtering
- [ ] All events logged to structured output
- [ ] Unit tests cover emission, cost calculation, buffer management
- [ ] Ready for Phase 1 full telemetry implementation

## Design Principles Alignment
- **Cost transparency:** Every tool invocation attributed with token and time costs
- **Observability:** All events emitted and visible via subscriber channels
- **Performance:** In-memory buffer prevents unbounded growth; events flushed periodically
- **Extensibility:** Cost calculation framework supports future refinement (GPU instrumentation, etc.)
- **Audit trail:** All tool invocations logged with timing and cost; basis for compliance records
