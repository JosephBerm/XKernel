# Engineer 3 — Kernel: IPC, Signals, Exceptions & Checkpointing — Week 07

## Phase: PHASE 1 — Advanced IPC & Distributed Communication

## Weekly Objective

Implement Pub/Sub IPC with topic-based subscriptions, capability-gated access, kernel-managed fan-out, and backpressure signaling (SIG_BUDGET_WARN then drop).

## Document References
- **Primary:** Section 3.2.4 (Publish-Subscribe IPC)
- **Supporting:** Section 7 (IPC Latency), Section 6.2 (Exit Criteria)

## Deliverables
- [ ] PubSubChannel struct with topic management and subscriber list
- [ ] Capability-gated subscription: only CT with topic_subscribe capability can subscribe
- [ ] Kernel-managed fan-out: kernel maps publisher output buffer to subscriber address spaces (read-only)
- [ ] Subscriber buffer management: per-subscriber ring buffer with depth tracking
- [ ] Backpressure implementation: SIG_BUDGET_WARN at 80%, drop messages at 100%
- [ ] pub_subscribe syscall: register subscriber for topic
- [ ] pub_unsubscribe syscall: deregister subscriber
- [ ] pub_publish syscall: publish message to topic
- [ ] Unit tests for subscription, publication, fan-out, backpressure
- [ ] Benchmark: measure fan-out latency to N subscribers (N=1,10,100)

## Technical Specifications

### PubSubChannel Structure
```
pub struct PubSubChannel {
    pub topic_id: TopicId,
    pub publisher: ContextThreadRef,
    pub subscribers: Vec<SubscriberState>,
    pub output_buffer: CircularBuffer,         // Publisher's output buffer
    pub backpressure_policy: BackpressurePolicy,
}

pub struct SubscriberState {
    pub subscriber_id: ContextThreadId,
    pub buffer: CircularBuffer,                // Subscriber's receive buffer
    pub capacity: usize,
    pub current_depth: usize,
    pub warning_sent: bool,                    // SIG_BUDGET_WARN sent
}
```

### Subscription Management
- **pub_subscribe syscall:** Register CT to receive messages on topic
  - Verify caller has topic_subscribe capability
  - Allocate per-subscriber ring buffer
  - Add SubscriberState to subscribers list
  - Return subscription ID
- **pub_unsubscribe syscall:** Remove subscriber from topic
  - Verify subscription belongs to caller
  - Free per-subscriber buffer
  - Remove SubscriberState

### Kernel-Managed Fan-Out
```
fn publish_to_topic(topic: &PubSubChannel, message: &[u8]) -> Result<(), PublishError> {
    for subscriber in &topic.subscribers {
        // Check subscriber buffer capacity
        if subscriber.current_depth >= subscriber.capacity {
            // Backpressure: buffer full
            if !subscriber.warning_sent {
                // Send SIG_BUDGET_WARN
                send_signal(subscriber.subscriber_id, CognitiveSignal::SigBudgetWarn);
                subscriber.warning_sent = true;
            } else {
                // Warning already sent, drop message
                continue;
            }
        }

        // Map publisher output buffer to subscriber's address space (read-only)
        // Copy message header + pointer to shared buffer
        let descriptor = MessageDescriptor {
            topic_id: topic.topic_id,
            sequence: next_sequence(),
            timestamp: now(),
            data_ptr: &message[0] as *const u8,
            data_len: message.len(),
        };

        subscriber.buffer.enqueue(&descriptor)?;
        subscriber.current_depth += 1;
        subscriber.warning_sent = false;
    }
    Ok(())
}
```

### Backpressure Policy
```
pub enum BackpressurePolicy {
    Drop,        // Silently drop messages if subscriber buffer full
    Suspend,     // Block publisher until subscriber buffer has space (not recommended for pub/sub)
    SignalWarn,  // Send SIG_BUDGET_WARN at 80%, drop at 100% (default)
}
```

### Message Format for Pub/Sub
```
pub struct MessageDescriptor {
    pub topic_id: TopicId,
    pub sequence: u64,              // Message sequence number per topic
    pub timestamp: Timestamp,
    pub data_ptr: *const u8,        // Points to publisher's buffer
    pub data_len: usize,
    pub publisher_id: ContextThreadId,
}
```

### pub_publish Syscall
```
syscall fn pub_publish(topic_id: TopicId, message: &[u8]) -> Result<usize, PublishError> {
    // 1. Verify caller is publisher for this topic
    // 2. Validate message size
    // 3. Write to publisher's output buffer
    // 4. Call publish_to_topic() for fan-out
    // 5. Return number of subscribers that received message
}
```

### Subscriber Receive
```
fn pub_receive(topic_id: TopicId, timeout_ms: u64) -> Result<MessageDescriptor, ReceiveError> {
    // 1. Find PubSubChannel for topic
    // 2. Find SubscriberState for caller
    // 3. Wait for message in buffer (with timeout)
    // 4. Return MessageDescriptor (read-only access to message data)
}
```

## Dependencies
- **Blocked by:** Week 1-6 (Phase 0 foundations)
- **Blocking:** Week 9-10 Shared Context IPC, Week 10-11 Protocol Negotiation

## Acceptance Criteria
1. Subscription registration and unregistration work correctly
2. Fan-out delivers messages to all subscribers
3. Backpressure signals (SIG_BUDGET_WARN) sent at 80% capacity
4. Messages dropped when subscriber buffer full and warning sent
5. Zero-copy mapping ensures subscribers see publisher's actual data
6. Capability-based access control prevents unauthorized subscriptions
7. Message sequence numbers prevent duplicates/losses
8. Unit tests cover: subscription, publication, fan-out, backpressure, drop behavior
9. Benchmark: single publisher to 100 subscribers with < 100 microsecond per-subscriber latency

## Design Principles Alignment
- **Scalability:** Kernel fan-out avoids N copies of message
- **Capability-Based:** Subscription requires explicit topic_subscribe capability
- **Backpressure:** Signals warn before drop, preventing silent data loss
- **Performance:** Zero-copy mapping minimizes latency and memory usage
