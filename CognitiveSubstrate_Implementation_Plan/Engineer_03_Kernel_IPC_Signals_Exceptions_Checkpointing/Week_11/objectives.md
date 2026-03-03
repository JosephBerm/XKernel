# Engineer 3 — Kernel: IPC, Signals, Exceptions & Checkpointing — Week 11

## Phase: PHASE 1 — Advanced IPC & Distributed Communication

## Weekly Objective

Implement protocol negotiation framework enabling agents to declare protocols (ReAct, structured-data, event-stream) and kernel to match and translate between different protocol formats.

## Document References
- **Primary:** Section 3.2.4 (Protocol Negotiation)
- **Supporting:** Section 3.2.4 (Request-Response, Pub/Sub, Shared Context), Section 6.2 (Exit Criteria)

## Deliverables
- [ ] ProtocolNegotiation struct: tracks compatible protocols for channel
- [ ] Protocol declaration API: agents declare supported protocols
- [ ] Protocol matcher: kernel selects best-fit protocol from mutual capabilities
- [ ] Protocol translator: convert messages between ReAct, structured-data, event-stream formats
- [ ] chan_open syscall enhancement: support protocol_hint parameter
- [ ] Protocol validation: ensure translator can handle conversions safely
- [ ] Fallback protocol: default to raw binary if no translator available
- [ ] Unit tests for all protocol combinations and translation
- [ ] Benchmark: measure translation overhead for each protocol pair
- [ ] Documentation: protocol grammar and translation rules

## Technical Specifications

### Protocol Types
```
pub enum Protocol {
    ReAct,              // Reasoning/Action/Observation: {thought, action, observation}
    StructuredData,     // Schema-driven: strongly-typed with validation
    EventStream,        // Event-based: stream of timestamped events
    Raw,                // Binary: no translation, pass-through
}

pub struct ProtocolDeclaration {
    pub protocol: Protocol,
    pub version: String,
    pub capabilities: Vec<String>,
    pub required_fields: Vec<String>,
    pub optional_fields: Vec<String>,
}
```

### ProtocolNegotiation Structure
```
pub struct ProtocolNegotiation {
    pub channel_id: ChannelId,
    pub endpoint_a: ContextThreadRef,
    pub endpoint_b: ContextThreadRef,
    pub protocols_a: Vec<ProtocolDeclaration>,
    pub protocols_b: Vec<ProtocolDeclaration>,
    pub negotiated_protocol: Option<Protocol>,
    pub translator: Option<ProtocolTranslator>,
}

pub struct ProtocolTranslator {
    pub from: Protocol,
    pub to: Protocol,
    pub conversion_fn: fn(&[u8]) -> Result<Vec<u8>, TranslationError>,
    pub overhead_bytes: usize,
}
```

### Protocol Negotiation Algorithm
```
fn negotiate_protocol(negotiation: &mut ProtocolNegotiation) -> Result<Protocol, NegotiationError> {
    // Find compatible protocols
    let compatible = find_compatible_protocols(&negotiation.protocols_a, &negotiation.protocols_b);

    if compatible.is_empty() {
        return Err(NegotiationError::NoCommonProtocol);
    }

    // Select best protocol based on:
    // 1. Protocol preference order (ReAct > StructuredData > EventStream > Raw)
    // 2. Translator overhead (prefer minimal translation)
    // 3. Feature overlap (prefer protocol with most common capabilities)
    let best = select_best_protocol(&compatible);

    negotiation.negotiated_protocol = Some(best.protocol.clone());

    // If protocols differ, create translator
    if negotiation.protocols_a[0].protocol != negotiation.protocols_b[0].protocol {
        negotiation.translator = Some(create_translator(
            negotiation.protocols_a[0].protocol.clone(),
            negotiation.protocols_b[0].protocol.clone(),
        )?);
    }

    Ok(best.protocol)
}

fn find_compatible_protocols(
    a: &[ProtocolDeclaration],
    b: &[ProtocolDeclaration],
) -> Vec<ProtocolDeclaration> {
    // Find protocols supported by both
    a.iter()
        .filter(|proto_a| b.iter().any(|proto_b| proto_a.protocol == proto_b.protocol))
        .cloned()
        .collect()
}
```

### Protocol Translator
```
pub struct ProtocolTranslator {
    pub from: Protocol,
    pub to: Protocol,
}

impl ProtocolTranslator {
    pub fn translate(&self, data: &[u8]) -> Result<Vec<u8>, TranslateError> {
        match (&self.from, &self.to) {
            (Protocol::ReAct, Protocol::StructuredData) => {
                translate_react_to_structured_data(data)
            }
            (Protocol::StructuredData, Protocol::ReAct) => {
                translate_structured_data_to_react(data)
            }
            (Protocol::EventStream, Protocol::ReAct) => {
                translate_event_stream_to_react(data)
            }
            (Protocol::ReAct, Protocol::EventStream) => {
                translate_react_to_event_stream(data)
            }
            (a, b) if a == b => Ok(data.to_vec()),
            (_, Protocol::Raw) => Ok(data.to_vec()),  // Any -> Raw is identity
            _ => Err(TranslateError::NoTranslator),
        }
    }
}

// Example: ReAct to StructuredData translation
fn translate_react_to_structured_data(react_data: &[u8]) -> Result<Vec<u8>, TranslateError> {
    // 1. Parse ReAct JSON: {thought, action, observation}
    let react: serde_json::Value = serde_json::from_slice(react_data)?;

    // 2. Convert to StructuredData schema
    let thought = react["thought"].as_str().unwrap_or("");
    let action = react["action"].as_str().unwrap_or("");
    let observation = react["observation"].as_str().unwrap_or("");

    let structured = serde_json::json!({
        "type": "ReasoningCycle",
        "fields": {
            "thought": {"type": "string", "value": thought},
            "action": {"type": "string", "value": action},
            "observation": {"type": "string", "value": observation},
        }
    });

    // 3. Serialize and return
    Ok(serde_json::to_vec(&structured)?)
}
```

### Protocol Specification for ReAct
```
pub struct ReActMessage {
    pub thought: String,       // Reasoning step
    pub action: String,        // Action to perform
    pub observation: String,   // Result of action
}
```

### Protocol Specification for StructuredData
```
pub struct StructuredMessage {
    pub schema_id: String,     // Reference to schema definition
    pub fields: HashMap<String, FieldValue>,
}

pub enum FieldValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Bytes(Vec<u8>),
    Object(HashMap<String, FieldValue>),
    Array(Vec<FieldValue>),
}
```

### Protocol Specification for EventStream
```
pub struct EventStreamMessage {
    pub events: Vec<Event>,
}

pub struct Event {
    pub timestamp: Timestamp,
    pub event_type: String,
    pub payload: Vec<u8>,
}
```

### chan_open Enhancement for Protocol Negotiation
```
syscall fn chan_open(
    protocol_hint: Option<Protocol>,
    endpoint_a: ContextThreadRef,
    endpoint_b: ContextThreadRef,
) -> Result<ChannelId, OpenError> {
    // 1. Create channel between endpoints
    // 2. If protocol_hint provided, use it; otherwise auto-negotiate
    // 3. Request protocol declarations from both endpoints
    // 4. Run protocol negotiation
    // 5. Set up translator if needed
    // 6. Return channel with negotiated protocol
}
```

## Dependencies
- **Blocked by:** Week 7-10 (Pub/Sub, Shared Context)
- **Blocking:** Week 12-13 Distributed IPC & GPU Checkpointing

## Acceptance Criteria
1. Protocol negotiation finds compatible protocols correctly
2. Protocol matcher selects best protocol based on criteria
3. All translator pairs (ReAct <-> StructuredData, ReAct <-> EventStream) work
4. Translator overhead is < 5% for typical message size
5. Fallback to Raw protocol when no translator available
6. Protocol declarations properly advertise capabilities
7. Unit tests cover: negotiation, all translator pairs, fallback
8. Benchmark: measure translation overhead for 100-byte messages
9. No data corruption during translation
10. Documentation includes protocol grammar and translation examples

## Design Principles Alignment
- **Interoperability:** Protocol negotiation enables diverse agents to communicate
- **Efficiency:** Translator selected based on minimal overhead
- **Transparency:** Translation is automatic; application sees consistent interface
- **Extensibility:** New protocol types can be added with new translators
