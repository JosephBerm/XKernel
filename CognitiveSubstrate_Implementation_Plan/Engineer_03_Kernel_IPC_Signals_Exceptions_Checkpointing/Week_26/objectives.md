# Engineer 3 — Kernel: IPC, Signals, Exceptions & Checkpointing — Week 26

## Phase: PHASE 3 — Benchmarking, Testing & Validation

## Weekly Objective

Continue comprehensive benchmarking: measure IPC latency and throughput across all channel types (request-response, pub/sub, shared context) and protocol variants (ReAct, structured-data, event-stream).

## Document References
- **Primary:** Section 7 (IPC Latency & Throughput), Section 6.2 (Exit Criteria)
- **Supporting:** Section 3.2.4 (All IPC Types)

## Deliverables
- [ ] Request-response latency: p50, p99, p999 for varying message sizes
- [ ] Pub/Sub throughput: messages/second with varying subscriber counts
- [ ] Shared context write latency: concurrent writes with CRDT merge overhead
- [ ] Protocol negotiation overhead: ReAct vs structured-data vs event-stream
- [ ] Translation overhead: inter-protocol message translation cost
- [ ] Zero-copy verification: confirm no memcpy for co-located channels
- [ ] Distributed IPC latency: cross-machine request-response
- [ ] Batching efficiency: throughput improvement with message batching
- [ ] Comprehensive IPC report: all measurements and comparisons
- [ ] Performance breakdown: identify bottlenecks

## Technical Specifications

### IPC Benchmarking Framework
```
pub struct IPCLatencyBenchmark {
    pub channel_type: ChannelType,
    pub message_size_bytes: Vec<usize>,
    pub iterations: usize,
}

pub enum ChannelType {
    RequestResponse,
    PubSub { subscriber_count: usize },
    SharedContext,
}

impl IPCLatencyBenchmark {
    pub fn run(&self) -> LatencyReport {
        let mut report = LatencyReport::new(format!("{:?}", self.channel_type));

        for msg_size in &self.message_size_bytes {
            let result = self.benchmark_size(*msg_size);
            report.add_result(*msg_size, result);
        }

        report
    }

    fn benchmark_size(&self, size: usize) -> LatencyResult {
        let mut latencies = Vec::new();

        for _ in 0..self.iterations {
            let msg = vec![0u8; size];
            let start = Instant::now();

            match self.channel_type {
                ChannelType::RequestResponse => {
                    let _ = channel.send(&msg);
                    let _ = channel.recv(TIMEOUT);
                }
                ChannelType::PubSub { .. } => {
                    let _ = topic.publish(&msg);
                }
                ChannelType::SharedContext => {
                    let _ = shared_ctx.write(0, &msg);
                }
            }

            let elapsed = start.elapsed();
            latencies.push(elapsed.as_micros() as u64);
        }

        LatencyResult {
            message_size: size,
            p50: percentile(&latencies, 50),
            p99: percentile(&latencies, 99),
            p999: percentile(&latencies, 99.9),
            max: *latencies.iter().max().unwrap_or(&0),
            avg: latencies.iter().sum::<u64>() / latencies.len() as u64,
        }
    }
}
```

## Dependencies
- **Blocked by:** Week 25 (Fault recovery benchmarking baseline)
- **Blocking:** Week 27-28 (Continued benchmarking)

## Acceptance Criteria
1. Request-response p99 latency < 1 microsecond for 64-byte messages
2. Pub/Sub throughput > 100,000 messages/second with 10 subscribers
3. Shared context write latency < 10 microseconds (without conflict)
4. Protocol translation overhead < 5% for typical message size
5. Zero-copy confirmed for co-located channels
6. Distributed cross-machine latency < 100 milliseconds
7. Batching provides > 50% throughput improvement
8. All message sizes (64B to 1MB) tested
9. Complete report with all results
10. Bottlenecks identified and documented

## Design Principles Alignment
- **Performance:** Comprehensive measurement ensures IPC latency targets met
- **Optimization:** Bottleneck identification guides further optimization
- **Validation:** Benchmarks confirm design choices effective
