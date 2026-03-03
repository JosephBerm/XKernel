# Engineer 6 — Services: Tool Registry, Telemetry & Compliance — Week 11

## Phase: Phase 1 (Weeks 7-14)

## Weekly Objective
Implement Telemetry Engine full production version with real-time streaming, core dumps on CT failure, and comprehensive event recording. Integrate with MCP Tool Registry (Weeks 7-8) and response caching (Weeks 9-10).

## Document References
- **Primary:** Section 6.2 (Phase 1, Week 11-12: Telemetry Engine full implementation, core dumps), Section 3.3.4 (Cognitive Telemetry Engine, record every model inference output, real-time streaming, core dumps)
- **Supporting:** Week 2 (CEF event types), Week 3 (cost attribution), Week 5-6 (baseline implementation), Week 7-10 (Tool Registry and caching)

## Deliverables
- [ ] Full telemetry event recording
  - Record every model inference output verbatim (or cryptographic commitment if output too large)
  - Record every tool call input/output with full context
  - Record every IPC message (sender, receiver, payload size, priority)
  - Record checkpoint references and memory state summaries
- [ ] Real-time event streaming infrastructure
  - gRPC bidirectional streaming for event subscription
  - Multiple subscriber support with independent filters
  - Backpressure handling (buffered subscribers)
  - Low-latency delivery (<100ms end-to-end)
- [ ] Cognitive Core Dumps
  - Trigger on CT (Cognitive Substrate) failure or exception
  - Capture full checkpoint (CPU state equivalent, GPU state)
  - Capture reasoning chain (all thoughts up to failure)
  - Capture context window state (all tokens)
  - Capture tool history (all tool invocations leading to failure)
  - Capture exception context (error type, message, stack)
  - Capture exact failure point (instruction, memory location)
  - Serialized to binary format for efficient storage
- [ ] Hardware-assisted telemetry (if available)
  - GPU performance counters (if available)
  - CPU cycle counting
  - Memory access patterns (sampled)
  - Interrupt and signal handling
- [ ] Event batching and flushing
  - Batch events in-memory before writing to persistence
  - Flush on size threshold (100 events) or time threshold (5 seconds)
  - Guarantee at-least-once delivery to persistent storage
- [ ] Compression and archival
  - Compress events before archival (gzip with streaming API)
  - Archival to fast storage (local SSD initially)
  - Retention policy: 7 days operational (verbatim), ≥6 months metadata
- [ ] Integration with Tool Registry and caching
  - Emit events for cache hits/misses, sandbox violations
  - Cost attribution includes cache operation costs
  - Tool invocation events contain binding ID, effect class, sandbox config
- [ ] **OpenTelemetry GenAI Semantic Conventions alignment (v1.37+)**
  - **CEF events include OpenTelemetry-compatible trace_id (128-bit) and span_id (64-bit)**
  - **Trace correlation: events linked via trace_id across agent boundaries**
  - **CEF base fields enriched: event_id (ULID), trace_id, span_id, ct_id, agent_id, crew_id**
  - **CEF timestamps in nanosecond precision for correlation with OTEL collectors**
  - **CEF events translatable to OpenTelemetry spans for Datadog/Grafana/Jaeger integration**
- [ ] **Addendum v2.5.1 — Correction 5: Observability**
- [ ] Unit and integration tests
  - Event recording and streaming
  - Core dump capture and serialization
  - Backpressure and buffering
  - Archival and retention
  - OpenTelemetry trace correlation

## Technical Specifications

### OpenTelemetry GenAI Semantic Conventions Integration

CEF events are enriched with OpenTelemetry trace correlation fields for seamless integration with observability platforms.

**CEF Event Schema with OTEL Fields**
```rust
pub struct CEFEvent {
    // Original CEF fields
    pub event_id: String,           // ULID for deduplication
    pub event_type: EventType,
    pub timestamp_utc: i64,         // Nanoseconds since epoch (for OTEL correlation)
    pub actor: String,
    pub resource: String,
    pub action: String,
    pub result: EventResult,
    pub context: Map<String, String>,
    pub cost: CostMetrics,

    // OpenTelemetry Semantic Conventions (v1.37+)
    pub trace_id: String,           // 128-bit hex, W3C Trace Context compatible
    pub span_id: String,            // 64-bit hex, derived from trace_id + event_id
    pub parent_span_id: Option<String>, // For event ordering
    pub ct_id: String,              // Cognitive Thread ID (local to kernel)
    pub agent_id: String,           // Agent that triggered event
    pub crew_id: String,            // Crew (multi-agent group) identifier
    pub phase: String,              // "reasoning", "tool_call", "response", etc.
    pub data_classification: String, // "public", "internal", "restricted", "confidential"
}

impl CEFEvent {
    pub fn from_inference(agent_id: &str, crew_id: &str, inference: &InferenceData) -> Self {
        let trace_id = Self::generate_trace_id(); // Random 128-bit hex
        let event_id = ulid::ULID::new().to_string();
        let span_id = Self::derive_span_id(&trace_id, &event_id);

        CEFEvent {
            event_id,
            trace_id,
            span_id,
            parent_span_id: None,
            ct_id: inference.ct_id.clone(),
            agent_id: agent_id.to_string(),
            crew_id: crew_id.to_string(),
            timestamp_utc: now_ns(),
            phase: "reasoning".to_string(),
            data_classification: "restricted".to_string(),
            // ... other fields
        }
    }

    fn generate_trace_id() -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let bytes: [u8; 16] = rng.gen();
        format!("{:032x}", u128::from_le_bytes(bytes))
    }

    fn derive_span_id(trace_id: &str, event_id: &str) -> String {
        use sha2::{Sha256, Digest};
        let input = format!("{}:{}", trace_id, event_id);
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        let result = hasher.finalize();
        format!("{:016x}", u64::from_le_bytes([
            result[0], result[1], result[2], result[3],
            result[4], result[5], result[6], result[7],
        ]))
    }
}
```

**CEF to OpenTelemetry Span Translation**
```rust
pub struct OTelSpanExporter {
    exporter: opentelemetry_otlp::OtlpPipelineBuilder,
}

impl OTelSpanExporter {
    pub fn translate_cef_to_span(cef: &CEFEvent) -> opentelemetry::trace::Span {
        let tracer = global::tracer("cognitive_substrate");

        let mut span_builder = tracer
            .span_builder(format!("{}:{}", cef.event_type, cef.action))
            .with_parent_context(cef.parent_span_id.as_ref().map(|_| {
                // Reconstruct parent context from parent_span_id
                opentelemetry::Context::new()
            }));

        // Add attributes per OpenTelemetry GenAI conventions
        span_builder = span_builder
            .with_attribute("trace_id", cef.trace_id.clone())
            .with_attribute("span_id", cef.span_id.clone())
            .with_attribute("ct_id", cef.ct_id.clone())
            .with_attribute("agent_id", cef.agent_id.clone())
            .with_attribute("crew_id", cef.crew_id.clone())
            .with_attribute("phase", cef.phase.clone())
            .with_attribute("data_classification", cef.data_classification.clone())
            .with_attribute("cost.tokens", cef.cost.input_tokens as i64 + cef.cost.output_tokens as i64)
            .with_attribute("cost.gpu_ms", cef.cost.gpu_milliseconds as i64)
            .with_attribute("cost.wall_clock_ms", cef.cost.wall_clock_milliseconds as i64);

        span_builder.start(&tracer)
    }

    pub async fn export_batch_to_datadog(&self, events: &[CEFEvent]) -> Result<(), ExportError> {
        // Convert all CEF events to OpenTelemetry spans
        let spans: Vec<_> = events.iter().map(Self::translate_cef_to_span).collect();

        // Send to Datadog via OTLP gRPC
        // Datadog automatically ingests via opentelemetry_otlp exporter
        Ok(())
    }
}
```

**Trace Correlation Example**
```
Agent A issues thought: trace_id=abc123def456, span_id=1111222233334444, phase=reasoning
Agent A invokes tool: trace_id=abc123def456, span_id=2222333344445555, parent_span_id=1111222233334444
Agent B receives delegated capability: trace_id=abc123def456, span_id=3333444455556666, parent_span_id=2222333344445555
Tool returns result to Agent B: trace_id=abc123def456, span_id=4444555566667777, parent_span_id=3333444455556666
```

All events linked by `trace_id`, enabling end-to-end tracing in Datadog/Grafana/Jaeger.

### Full Event Recording System
```rust
pub struct TelemetryEngineV2 {
    event_buffer: Arc<Mutex<VecDeque<CEFEvent>>>,
    subscribers: Arc<RwLock<Vec<SubscriberChannel>>>,
    persistent_storage: Arc<PersistentEventStore>,
    core_dump_service: Arc<CoreDumpService>,
    cost_engine: Arc<CostAttributionEngine>,
}

pub struct PersistentEventStore {
    event_log_path: PathBuf,
    compressed_archive_path: PathBuf,
    rotation_size_bytes: u64,
    rotation_duration: Duration,
}

impl PersistentEventStore {
    pub async fn write_batch(&self, events: Vec<CEFEvent>) -> Result<(), StorageError> {
        let file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.event_log_path)
            .await?;

        let mut writer = tokio::io::BufWriter::new(file);
        for event in events {
            let line = serde_json::to_string(&event)?;
            writer.write_all(line.as_bytes()).await?;
            writer.write_all(b"\n").await?;
        }
        writer.flush().await?;

        Ok(())
    }

    pub async fn rotate_and_compress(&self, input_path: &Path, output_path: &Path)
        -> Result<u64, StorageError>
    {
        use flate2::write::GzEncoder;
        use flate2::Compression;

        let input_file = tokio::fs::File::open(input_path).await?;
        let reader = tokio::io::BufReader::new(input_file);

        let output_file = tokio::fs::File::create(output_path).await?;
        let encoder = GzEncoder::new(output_file.into_std(), Compression::default());

        let compressed_size = tokio::io::copy(
            &mut tokio::io::BufReader::new(reader),
            &mut std::io::BufWriter::new(encoder)
        ).await?;

        Ok(compressed_size)
    }
}

impl TelemetryEngineV2 {
    pub fn new(config: TelemetryConfig) -> Self {
        Self {
            event_buffer: Arc::new(Mutex::new(VecDeque::with_capacity(10_000))),
            subscribers: Arc::new(RwLock::new(Vec::new())),
            persistent_storage: Arc::new(PersistentEventStore {
                event_log_path: config.event_log_path,
                compressed_archive_path: config.archive_path,
                rotation_size_bytes: config.rotation_size,
                rotation_duration: config.rotation_interval,
            }),
            core_dump_service: Arc::new(CoreDumpService::new(config.core_dump_path)),
            cost_engine: Arc::new(CostAttributionEngine::new()),
        }
    }

    pub async fn emit_event(&self, event: CEFEvent) -> Result<(), EmitError> {
        // Add to in-memory buffer
        {
            let mut buffer = self.event_buffer.lock().await;
            if buffer.len() >= 10_000 {
                // Flush before adding if buffer at capacity
                self.flush_events().await.ok();
            }
            buffer.push_back(event.clone());
        }

        // Notify subscribers (non-blocking)
        for subscriber in self.subscribers.read().await.iter() {
            subscriber.send(event.clone()).await.ok();
        }

        Ok(())
    }

    pub async fn flush_events(&self) -> Result<u32, StorageError> {
        let mut buffer = self.event_buffer.lock().await;
        let events: Vec<CEFEvent> = buffer.drain(..buffer.len()).collect();
        let count = events.len() as u32;

        if !events.is_empty() {
            self.persistent_storage.write_batch(events).await?;
        }

        Ok(count)
    }

    pub async fn subscribe(&self, filter: SubscriptionFilter)
        -> tokio::sync::mpsc::Receiver<CEFEvent>
    {
        let (tx, rx) = tokio::sync::mpsc::channel(1000);
        let subscriber = SubscriberChannel {
            tx,
            filter: filter.clone(),
        };
        self.subscribers.write().await.push(subscriber);
        rx
    }

    pub async fn record_model_inference(&self, inference_data: &InferenceData)
        -> Result<(), RecordError>
    {
        let output_commitment = if inference_data.output.len() > 1_000_000 {
            // Large output: store cryptographic commitment
            self.create_commitment(&inference_data.output).await?
        } else {
            // Small output: store verbatim
            inference_data.output.clone()
        };

        let event = CEFEvent {
            event_type: EventType::ThoughtStep,
            actor: inference_data.agent_id.clone(),
            resource: "model_inference".to_string(),
            action: "GENERATE",
            result: EventResult::COMPLETED,
            context: {
                "model_id": inference_data.model_id.clone(),
                "output_commitment": output_commitment,
                "context_window_tokens": format!("{}", inference_data.context_tokens),
                "stop_reason": inference_data.stop_reason.clone(),
            }.into(),
            cost: self.cost_engine.calculate_inference_cost(inference_data),
            ..Default::default()
        };

        self.emit_event(event).await
    }

    pub async fn record_tool_invocation(&self, tool_data: &ToolInvocationData)
        -> Result<(), RecordError>
    {
        let input_event = CEFEvent {
            event_type: EventType::ToolCallRequested,
            actor: tool_data.agent_id.clone(),
            resource: tool_data.tool_binding_id.clone(),
            action: "INVOKE",
            result: EventResult::COMPLETED,
            context: {
                "input_hash": self.create_commitment(&tool_data.input).await?,
                "input_size": format!("{}", tool_data.input.len()),
                "effect_class": format!("{:?}", tool_data.effect_class),
                "sandbox_config": serde_json::to_string(&tool_data.sandbox_config)?,
            }.into(),
            cost: self.cost_engine.estimate_tool_cost(tool_data),
            ..Default::default()
        };
        self.emit_event(input_event).await?;

        // Output event (after tool completes)
        let output_event = CEFEvent {
            event_type: EventType::ToolCallCompleted,
            actor: tool_data.agent_id.clone(),
            resource: tool_data.tool_binding_id.clone(),
            action: "INVOKE",
            result: EventResult::COMPLETED,
            context: {
                "output_hash": self.create_commitment(&tool_data.output).await?,
                "output_size": format!("{}", tool_data.output.len()),
                "execution_time_ms": format!("{}", tool_data.execution_time_ms),
                "cache_hit": tool_data.cache_hit.to_string(),
            }.into(),
            cost: self.cost_engine.calculate_tool_cost(tool_data),
            ..Default::default()
        };
        self.emit_event(output_event).await
    }

    async fn create_commitment(&self, data: &str) -> Result<String, RecordError> {
        use sha2::{Sha256, Digest};

        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        let result = hasher.finalize();
        Ok(format!("sha256:{:x}", result))
    }
}

pub struct InferenceData {
    pub agent_id: String,
    pub model_id: String,
    pub output: String,
    pub context_tokens: u64,
    pub stop_reason: String,
}

pub struct ToolInvocationData {
    pub agent_id: String,
    pub tool_binding_id: String,
    pub input: String,
    pub output: String,
    pub execution_time_ms: f64,
    pub effect_class: EffectClass,
    pub sandbox_config: SandboxConfig,
    pub cache_hit: bool,
}
```

### Real-Time Streaming with gRPC
```rust
pub struct SubscriberChannel {
    tx: tokio::sync::mpsc::Sender<CEFEvent>,
    filter: SubscriptionFilter,
}

pub struct SubscriptionFilter {
    pub event_types: Vec<EventType>,
    pub actor_filter: Option<String>,
    pub resource_filter: Option<String>,
    pub min_cost_threshold: Option<f64>,
}

impl SubscriberChannel {
    pub async fn send(&self, event: CEFEvent) -> Result<(), SendError<CEFEvent>> {
        // Apply filter
        if !self.event_types.is_empty() && !self.event_types.contains(&event.event_type) {
            return Ok(()); // Skip filtered event
        }

        if let Some(ref actor) = self.filter.actor_filter {
            if !event.actor.contains(actor) {
                return Ok(());
            }
        }

        if let Some(ref resource) = self.filter.resource_filter {
            if !event.resource.contains(resource) {
                return Ok(());
            }
        }

        self.tx.send(event).await
    }
}

// gRPC service definition (protobuf)
pub mod telemetry_pb {
    pub struct TelemetryStreamRequest {
        pub event_types: Vec<String>,
        pub actor_filter: Option<String>,
        pub resource_filter: Option<String>,
    }

    pub struct TelemetryEvent {
        pub event_id: String,
        pub event_type: String,
        pub timestamp_utc: i64,
        pub actor: String,
        pub resource: String,
        pub action: String,
        pub result: String,
    }
}

pub struct TelemetryStreamService {
    telemetry: Arc<TelemetryEngineV2>,
}

// In actual implementation: impl TelemetryStream for TelemetryStreamService
```

### Cognitive Core Dumps
```rust
pub struct CoreDumpService {
    dump_dir: PathBuf,
}

pub struct CognitiveCoreData {
    pub dump_id: String,
    pub timestamp: i64,
    pub trigger_type: CoreDumpTrigger,
    pub checkpoint_id: String,
    pub cpu_state: Vec<u8>,
    pub gpu_state: Option<Vec<u8>>,
    pub reasoning_chain: Vec<String>,
    pub context_window: String,
    pub tool_history: Vec<ToolInvocationRecord>,
    pub exception_context: Option<ExceptionContext>,
    pub failure_point: String,
}

pub enum CoreDumpTrigger {
    Exception(String),
    Timeout,
    ResourceExhaustion,
    Policy Violation,
    ManualRequest,
}

pub struct ExceptionContext {
    pub exception_type: String,
    pub message: String,
    pub stack_trace: String,
    pub memory_state_at_failure: Vec<u8>,
}

impl CoreDumpService {
    pub async fn capture_core_dump(&self, core_data: CognitiveCoreData)
        -> Result<String, DumpError>
    {
        let dump_id = core_data.dump_id.clone();
        let dump_path = self.dump_dir.join(format!("{}.coredump", dump_id));

        // Serialize to binary format
        let serialized = bincode::serialize(&core_data)?;

        // Write to file
        tokio::fs::write(&dump_path, serialized).await?;

        // Emit event
        eprintln!("Core dump captured: {} ({} bytes)", dump_id, std::fs::metadata(&dump_path)?.len());

        Ok(dump_id)
    }

    pub async fn load_core_dump(&self, dump_id: &str) -> Result<CognitiveCoreData, DumpError> {
        let dump_path = self.dump_dir.join(format!("{}.coredump", dump_id));
        let data = tokio::fs::read(&dump_path).await?;
        let core_data = bincode::deserialize(&data)?;
        Ok(core_data)
    }
}
```

### Cost Attribution Engine
```rust
pub struct CostAttributionEngine;

impl CostAttributionEngine {
    pub fn calculate_inference_cost(&self, data: &InferenceData) -> CostMetrics {
        CostMetrics {
            input_tokens: data.context_tokens,
            output_tokens: self.estimate_output_tokens(&data.output),
            gpu_milliseconds: 0.0, // Populated by hardware counters
            wall_clock_milliseconds: 0.0, // Populated by caller
            tpc_hours: 0.0,
        }
    }

    pub fn estimate_tool_cost(&self, data: &ToolInvocationData) -> CostMetrics {
        CostMetrics {
            input_tokens: (data.input.len() / 4) as u64,
            output_tokens: 0,
            gpu_milliseconds: 0.0,
            wall_clock_milliseconds: data.execution_time_ms,
            tpc_hours: 0.0,
        }
    }

    pub fn calculate_tool_cost(&self, data: &ToolInvocationData) -> CostMetrics {
        let mut cost = self.estimate_tool_cost(data);
        cost.output_tokens = (data.output.len() / 4) as u64;
        cost.tpc_hours = ((cost.input_tokens + cost.output_tokens) as f64 * data.execution_time_ms / 3_600_000.0) / 1_000_000.0;
        cost
    }

    fn estimate_output_tokens(&self, text: &str) -> u64 {
        (text.split_whitespace().count() as u64).max(1)
    }
}
```

## Dependencies
- **Blocked by:** Weeks 1-10 (all prior components), Week 12 (Policy Engine)
- **Blocking:** Week 12 (complete telemetry with policy events), Phase 2 compliance work

## Acceptance Criteria
- [ ] Event recording for model inference (verbatim or commitment)
- [ ] Event recording for tool invocations (input, output, cost)
- [ ] Event recording for IPC messages (sender, receiver, payload, priority)
- [ ] Real-time gRPC streaming with subscriber channels
- [ ] Core dump capture on exception (CPU state, GPU state, reasoning chain, tool history)
- [ ] Core dump serialization to binary format; deserialization for analysis
- [ ] Event batching and flushing (100 events or 5 seconds)
- [ ] Compression and archival functional
- [ ] Cost attribution for all event types calculated
- [ ] Hardware performance counters integrated (if available)
- [ ] Backpressure handling for slow subscribers
- [ ] Unit and integration tests pass (recording, streaming, core dumps, archival)

## Design Principles Alignment
- **Complete audit trail:** Every inference, tool call, and IPC message recorded
- **Core dump capability:** On failure, full system state captured for post-mortem analysis
- **Real-time observability:** Events stream to subscribers immediately
- **Efficiency:** Large outputs hashed; small outputs stored verbatim
- **Safety:** Core dumps isolated to crash conditions; no performance overhead in normal operation
