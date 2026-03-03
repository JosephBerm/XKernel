# IPC, Signals & Exceptions

> **Crate:** [`ipc_signals_exceptions`](Cargo.toml)
> **Stream:** 1 — Kernel & Scheduler
> **Layer:** L0 (Microkernel)
> **Owner:** Engineer 01
> **Status:** Active

---

## 1. Purpose & Scope

Implements inter-process communication (IPC) through SemanticChannels, asynchronous signal delivery, and the exception hierarchy for cognitive failures. SemanticChannels are strongly-typed, intent-based IPC primitives replacing traditional pipes/sockets. Signals notify agents of kernel events (CT completion, capability revocation, etc.). CognitiveExceptions model failures in reasoning/acting phases.

**Key Responsibilities:**
- SemanticChannel creation, message passing, and lifecycle
- Signal delivery and queueing
- CognitiveException hierarchy and propagation
- Bidirectional communication patterns (request-reply, pub-sub)
- Signal ordering guarantees and delivery semantics

**In Scope:**
- SemanticChannel protocol and implementation
- Signal types and delivery mechanisms
- Exception definitions and handling
- Message serialization (Cap'n Proto)

**Out of Scope:**
- Resource limiting (handled by kernel scheduler)
- Signal policy enforcement (handled by capability engine)
- Exception recovery strategies (handled by runtime)

---

## 2. Engineering Plan Reference

**Relevant Sections:**
- Section 2.3: SemanticChannel domain entity
- Section 2.4: CognitiveException hierarchy
- Section 2.5: CognitiveSignal types
- Section 4.4: IPC & Signal Delivery Architecture

**Domain Model Entities Involved:**
- **SemanticChannel** — Core IPC primitive
- **CognitiveException** — Failure types
- **CognitiveSignal** — Kernel notifications

See `docs/domain_model_deep_dive.md` for definitions.

---

## 3. Architecture & Design

### 3.1 High-Level Architecture

```
┌──────────────────────────────┐
│  SemanticChannel API         │
│  create(), send(), recv()    │
└──────────────────────────────┘
             ↓
┌──────────────────────────────┐
│  Channel Implementation      │
│  (Ring buffer, serialization)│
└──────────────────────────────┘
             ↓         ↓
    ┌────────────┐  ┌────────────┐
    │ Signal     │  │ Exception  │
    │ Dispatcher │  │ Handler    │
    └────────────┘  └────────────┘
             ↓         ↓
    ┌────────────────────────────┐
    │  Cap'n Proto Serializer    │
    │  (Schema-driven messages)  │
    └────────────────────────────┘
```

### 3.2 Key Invariants

1. **Message Ordering**: Messages on a channel are delivered FIFO
   - Enforced: Ring buffer with sequence numbers
   - Impact: Agents can rely on causal ordering

2. **Signal Delivery Guarantee**: All signals eventually delivered (unless capability revoked)
   - Enforced: Kernel retries + signal queue overflow handling
   - Impact: No silent failures; kernel ensures delivery

3. **Exception Propagation**: Exceptions bubble up the task hierarchy
   - Enforced: Signal propagation on exception catch
   - Impact: Parent can react to child failures

---

## 4. Dependencies

| Crate | Type | Layer | Why |
|-------|------|-------|-----|
| `ct_lifecycle` | Internal | L0 | Signal CT phase transitions |
| `capability_engine` | Internal | L0 | Verify channel send capability |

---

## 5. Public API Surface

```rust
/// Strongly-typed IPC channel
pub struct SemanticChannel<T> {
    pub id: ChannelId,
    pub sender: Capability,
    pub receiver: Capability,
}

/// Send a message on channel (capability required)
pub fn channel_send<T: Serialize>(
    chan: &SemanticChannel<T>,
    msg: T,
) -> CsResult<()>;

/// Receive from channel (blocks if empty)
pub fn channel_recv<T: Deserialize>(
    chan: &SemanticChannel<T>,
) -> CsResult<T>;

/// Cognitive exception types
pub enum CognitiveException {
    ReasoningTimeout,
    ReasoningDivergence,
    ActingFailure { reason: String },
    DependencyNotMet { task_id: TaskId },
    CapabilityDenied { task_id: TaskId },
}

/// Signal types
pub enum CognitiveSignal {
    TaskCompleted { task_id: TaskId },
    TaskFailed { task_id: TaskId, reason: CognitiveException },
    CapabilityRevoked { capability_id: CapabilityId },
    PhaseTransition { task_id: TaskId, phase: CTPhase },
}
```

---

## 6. Building & Testing

```bash
cargo build -p ipc_signals_exceptions
cargo test -p ipc_signals_exceptions
```

**Key Test Scenarios:**
1. Channel message ordering — FIFO guarantee
2. Signal delivery — Eventual delivery semantics
3. Exception propagation — Bubbling up task hierarchy
4. Capability checks — Send/recv require appropriate capability

---

## 7. Design Decisions Log

### 7.1 "SemanticChannels vs. Raw Message Passing?"

**Decision:** Strongly-typed SemanticChannels instead of untyped byte buffers.

**Alternatives:**
1. Raw bytes — Channels carry opaque byte sequences
2. JSON-based — Human-readable but less efficient

**Rationale:**
- Type safety prevents message format errors at compile time
- Cap'n Proto encoding is efficient and schema-driven
- Intent is explicit (not just "message on wire")
- Better integration with Rust type system

**Date:** 2026-03-01
**Author:** Engineer 01

### 7.2 "Synchronous recv() vs. Async?"

**Decision:** Synchronous (blocking) `recv()` in L0. Async wrappers in L2/L3.

**Alternatives:**
1. Async channels — Non-blocking recv with futures
2. Polling API — Check for messages without blocking

**Rationale:**
- L0 must be simple — no async executor overhead
- L2/L3 can provide async wrappers via SemanticChannel adapters
- Blocking simplifies reasoning about control flow in kernel

**Date:** 2026-03-01
**Author:** Engineer 01

---

## 8. Performance Characteristics

| Operation | Complexity | Notes |
|-----------|-----------|-------|
| `channel_send` | O(m) | m = message size (serialization) |
| `channel_recv` | O(m) | m = message size (deserialization) |
| Signal dispatch | O(n) | n = receivers |
| Exception propagation | O(h) | h = task hierarchy height |

---

## 9. Common Pitfalls & Troubleshooting

**Mistake 1: Assuming recv() is non-blocking**
```rust
// ✗ WRONG: Code blocks until message arrives
let msg = channel_recv(&chan)?;  // Blocks forever if no senders

// ✓ RIGHT: Use timeouts or async wrappers
let msg = channel_recv_timeout(&chan, Duration::from_secs(10))?;
```

**Mistake 2: Sending without capability**
```rust
// ✗ WRONG: No send capability
let ch = SemanticChannel::new()?;
// If you don't own the sender capability, this fails:
channel_send(&ch, message)?;  // ERROR: EPERM
```

---

## 10. Integration Points

| Module | Integration | Protocol |
|--------|-----------|----------|
| `ct_lifecycle` | Send phase-transition signals | SemanticChannel |
| `capability_engine` | Check send/recv capability | Direct call |
| All L1+ services | Signal-based notifications | SemanticChannel |

---

## 11. Future Roadmap

**Planned Improvements:**
- Channel broadcast mode — One sender, multiple receivers
- Channel persistence — Messages survive channel destruction
- Exception recovery hooks — User-defined exception handlers

---

## 12. References

- **Cap'n Proto Spec:** https://capnproto.org/
- **Capability Semantics:** https://en.wikipedia.org/wiki/Capability-based_security

---

**README Version:** 1.0
**Last Updated:** 2026-03-01
**Owner:** Engineer 01
