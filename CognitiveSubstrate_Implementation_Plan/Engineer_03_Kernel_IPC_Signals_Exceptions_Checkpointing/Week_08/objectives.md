# Engineer 3 — Kernel: IPC, Signals, Exceptions & Checkpointing — Week 08

## Phase: PHASE 1 — Advanced IPC & Distributed Communication

## Weekly Objective

Extend Pub/Sub implementation from Week 7 with performance optimization, topology validation, and multi-topic support. Prepare infrastructure for protocol negotiation in Week 10-11.

## Document References
- **Primary:** Section 3.2.4 (Publish-Subscribe IPC)
- **Supporting:** Section 7 (IPC Latency), Section 6.2 (Exit Criteria)

## Deliverables
- [ ] Multi-topic support: single CT can publish/subscribe to multiple topics
- [ ] Topic registry: kernel maintains map of topic_id -> PubSubChannel
- [ ] Topology validation: prevent creating topics with conflicting publishers
- [ ] Subscription deduplication: prevent duplicate subscriptions by same CT
- [ ] Per-subscriber metrics: message count, drop count, backpressure events
- [ ] pub_create_topic syscall: create new pub/sub topic with publisher designation
- [ ] pub_delete_topic syscall: destroy topic and notify subscribers
- [ ] Benchmark: 100 topics with 10 publishers each, measure aggregate throughput
- [ ] Integration tests: multiple publishers/subscribers, topic isolation
- [ ] Performance profiling: identify bottlenecks in fan-out

## Technical Specifications

### Topic Registry
```
pub struct TopicRegistry {
    pub topics: HashMap<TopicId, PubSubChannel>,
    pub ct_subscriptions: HashMap<ContextThreadId, Vec<TopicId>>, // Per-CT subscriptions
    pub publisher_map: HashMap<TopicId, ContextThreadRef>,        // Topic -> publisher
}

impl TopicRegistry {
    pub fn create_topic(&mut self, topic_id: TopicId, publisher: ContextThreadRef) -> Result<(), TopicError> {
        if self.topics.contains_key(&topic_id) {
            return Err(TopicError::AlreadyExists);
        }
        if self.publisher_map.contains_key(&topic_id) {
            return Err(TopicError::PublisherAlreadyExists);
        }

        let channel = PubSubChannel {
            topic_id,
            publisher,
            subscribers: Vec::new(),
            output_buffer: CircularBuffer::new(DEFAULT_TOPIC_BUFFER_SIZE),
            backpressure_policy: BackpressurePolicy::SignalWarn,
        };
        self.topics.insert(topic_id, channel);
        self.publisher_map.insert(topic_id, publisher);
        Ok(())
    }
}
```

### Multi-Topic Support
```
pub struct SubscriberTopics {
    pub ct_id: ContextThreadId,
    pub topics: HashMap<TopicId, SubscriptionState>,
}

pub struct SubscriptionState {
    pub subscription_id: SubscriptionId,
    pub topic_id: TopicId,
    pub messages_received: u64,
    pub messages_dropped: u64,
    pub backpressure_events: u64,
}
```

### pub_create_topic Syscall
```
syscall fn pub_create_topic(topic_id: TopicId) -> Result<(), CreateTopicError> {
    // 1. Verify topic_id is unique
    // 2. Get caller CT as publisher
    // 3. Create PubSubChannel with publisher as owner
    // 4. Register in TopicRegistry
    // 5. Grant publisher with topic_publish capability for this topic
}
```

### pub_delete_topic Syscall
```
syscall fn pub_delete_topic(topic_id: TopicId) -> Result<(), DeleteTopicError> {
    // 1. Verify caller is publisher for this topic
    // 2. Get list of all subscribers
    // 3. Send SIG_IPC_FAILED to all subscribers
    // 4. Remove topic from registry
    // 5. Clean up all subscriber buffers
}
```

### Subscription Deduplication
```
fn pub_subscribe_internal(ct_id: ContextThreadId, topic_id: TopicId) -> Result<SubscriptionId, SubscribeError> {
    // Check if CT already subscribed to this topic
    if let Some(subs) = ct_subscriptions.get(&ct_id) {
        if subs.contains(&topic_id) {
            return Err(SubscribeError::AlreadySubscribed);
        }
    }

    // Check if topic exists
    let topic = topics.get_mut(&topic_id)?;

    // Check for duplicate in subscribers list
    if topic.subscribers.iter().any(|s| s.subscriber_id == ct_id) {
        return Err(SubscribeError::AlreadySubscribed);
    }

    // Add subscription
    let sub_id = SubscriptionId::new();
    topic.subscribers.push(SubscriberState {
        subscriber_id: ct_id,
        subscription_id: sub_id,
        buffer: CircularBuffer::new(DEFAULT_SUB_BUFFER_SIZE),
        capacity: DEFAULT_SUB_BUFFER_SIZE,
        current_depth: 0,
        warning_sent: false,
    });

    ct_subscriptions.entry(ct_id).or_insert_with(Vec::new).push(topic_id);
    Ok(sub_id)
}
```

### Per-Subscriber Metrics
```
pub struct SubscriberMetrics {
    pub subscription_id: SubscriptionId,
    pub messages_received: u64,
    pub messages_dropped: u64,
    pub backpressure_warnings: u64,
    pub last_message_timestamp: Option<Timestamp>,
    pub buffer_peak_depth: usize,
}

fn update_subscriber_metrics(topic_id: TopicId, subscriber_id: ContextThreadId, success: bool) {
    if let Some(topic) = topics.get_mut(&topic_id) {
        if let Some(sub) = topic.subscribers.iter_mut().find(|s| s.subscriber_id == subscriber_id) {
            if success {
                sub.metrics.messages_received += 1;
                sub.metrics.last_message_timestamp = Some(now());
                sub.metrics.buffer_peak_depth = sub.metrics.buffer_peak_depth.max(sub.current_depth);
            } else {
                sub.metrics.messages_dropped += 1;
            }
        }
    }
}
```

### Topic Isolation Validation
```
fn validate_topic_topology() -> Result<(), TopologyError> {
    // Verify:
    // 1. No topic has multiple publishers
    // 2. Each subscriber is subscribed exactly once per topic
    // 3. No circular subscription patterns
    // 4. Subscriber buffers not exceeding system limits
    Ok(())
}
```

## Dependencies
- **Blocked by:** Week 7 (Pub/Sub baseline)
- **Blocking:** Week 9-10 Shared Context IPC, Week 10-11 Protocol Negotiation

## Acceptance Criteria
1. Multi-topic support allows single CT to manage 100+ topics
2. Topic registry correctly tracks publishers and subscribers
3. Subscription deduplication prevents duplicate entries
4. pub_create_topic works for new topics
5. pub_delete_topic properly notifies all subscribers
6. Per-subscriber metrics accurately track message flow
7. No memory leaks when topics/subscriptions created/destroyed
8. Unit tests cover: multi-topic, topology validation, deduplication, metrics
9. Benchmark: 100 topics, 10 publishers each, aggregate throughput > 1M messages/second

## Design Principles Alignment
- **Scalability:** Multi-topic support enables complex pub/sub topologies
- **Isolation:** Topics are isolated; failure in one doesn't affect others
- **Observability:** Per-subscriber metrics enable debugging and monitoring
- **Reliability:** Topology validation prevents inconsistent state
