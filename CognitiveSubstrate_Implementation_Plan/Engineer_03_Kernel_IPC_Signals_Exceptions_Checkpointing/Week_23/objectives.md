# Engineer 3 — Kernel: IPC, Signals, Exceptions & Checkpointing — Week 23

## Phase: PHASE 3 — Benchmarking, Testing & Validation

## Weekly Objective

Execute comprehensive benchmarking across 4 reference workloads: fault recovery latency, IPC throughput, checkpoint overhead, and distributed multi-machine scenarios. Document all results and compare against targets.

## Document References
- **Primary:** Section 6.2 (Exit Criteria — Benchmarking & Testing), Section 7 (Performance Targets)
- **Supporting:** All Sections 2.6-3.2.8

## Deliverables
- [ ] Workload 1 benchmark: Fault recovery (tool failure, exception handling)
- [ ] Workload 2 benchmark: IPC throughput (request-response, pub/sub, shared context)
- [ ] Workload 3 benchmark: Checkpoint overhead (creation, restoration, delta)
- [ ] Workload 4 benchmark: Distributed multi-machine (cross-machine latency, failover)
- [ ] Performance regression test suite: automated testing against targets
- [ ] Hardware compatibility: test on 3+ reference platforms
- [ ] Scaling tests: measure performance with 10, 100, 1000 agents
- [ ] Baseline comparison: before/after optimization metrics
- [ ] Performance report: detailed analysis and recommendations
- [ ] Documentation: benchmark methodology and how to reproduce results

## Technical Specifications

### Workload 1: Fault Recovery Benchmark
```
pub struct FaultRecoveryWorkload {
    pub ct_count: usize,               // Number of context threads
    pub exception_rate: f32,           // Exceptions per second
    pub tool_failure_rate: f32,        // Tool failure rate (0.0-1.0)
    pub duration_seconds: u32,         // Benchmark duration
}

impl FaultRecoveryWorkload {
    pub fn run(&self) -> BenchmarkResults {
        let mut results = BenchmarkResults::new("Fault Recovery");

        // Setup: Create multiple agents, have them call tools with failure injection
        let mut agents = Vec::new();
        for i in 0..self.ct_count {
            agents.push(setup_agent(format!("Agent_{}", i)));
        }

        let start = Instant::now();
        let mut exception_count = 0;
        let mut recovery_latencies = Vec::new();

        while start.elapsed().as_secs() < self.duration_seconds as u64 {
            for agent in &mut agents {
                // Randomly inject exception
                if rand::random::<f32>() < self.exception_rate / 1000.0 {
                    let exception_start = Instant::now();

                    // Trigger tool call that will fail
                    if rand::random::<f32>() < self.tool_failure_rate {
                        let _ = agent.call_tool_failing();
                    } else {
                        let _ = agent.call_tool_succeeding();
                    }

                    // Measure recovery latency
                    let recovery_latency = exception_start.elapsed().as_micros() as u64;
                    recovery_latencies.push(recovery_latency);
                    exception_count += 1;
                }
            }
        }

        // Analyze results
        results.total_exceptions = exception_count;
        results.p50_latency_us = percentile(&recovery_latencies, 50);
        results.p99_latency_us = percentile(&recovery_latencies, 99);
        results.p999_latency_us = percentile(&recovery_latencies, 99.9);
        results.max_latency_us = *recovery_latencies.iter().max().unwrap_or(&0);

        // Target: P99 < 100,000 microseconds (100ms)
        println!("Fault Recovery: {} exceptions, P99: {}us", exception_count, results.p99_latency_us);

        results
    }
}

// Configuration: Moderate failure rate
let workload = FaultRecoveryWorkload {
    ct_count: 10,
    exception_rate: 1.0,           // ~1 exception per second
    tool_failure_rate: 0.3,        // 30% of calls fail
    duration_seconds: 60,
};
let results = workload.run();
assert!(results.p99_latency_us < 100_000, "P99 recovery latency must be < 100ms");
```

### Workload 2: IPC Throughput Benchmark
```
pub struct IPCThroughputWorkload {
    pub ct_count: usize,
    pub ipc_type: IpcType,
    pub message_size_bytes: usize,
    pub duration_seconds: u32,
}

pub enum IpcType {
    RequestResponse,
    PubSub { subscribers: usize },
    SharedContext,
}

impl IPCThroughputWorkload {
    pub fn run(&self) -> BenchmarkResults {
        let mut results = BenchmarkResults::new("IPC Throughput");
        let start = Instant::now();
        let mut message_count = 0;

        match self.ipc_type {
            IpcType::RequestResponse => {
                // Producer-consumer pattern
                let (tx, rx) = create_channel();
                let mut producer_handles = Vec::new();
                let mut consumer_handles = Vec::new();

                // Producers
                for _ in 0..self.ct_count {
                    let tx = tx.clone();
                    let h = std::thread::spawn(move || {
                        let mut count = 0;
                        while count < 10000 {
                            let msg = vec![0u8; self.message_size_bytes];
                            if tx.send(msg).is_ok() {
                                count += 1;
                            }
                        }
                        count
                    });
                    producer_handles.push(h);
                }

                // Consumers
                for _ in 0..self.ct_count {
                    let rx = rx.clone();
                    let h = std::thread::spawn(move || {
                        let mut count = 0;
                        while count < 10000 {
                            if rx.recv().is_ok() {
                                count += 1;
                            }
                        }
                        count
                    });
                    consumer_handles.push(h);
                }

                for h in producer_handles.into_iter().chain(consumer_handles) {
                    message_count += h.join().unwrap_or(0);
                }
            }
            IpcType::PubSub { subscribers } => {
                // Publish-subscribe pattern
                let topic = create_pub_sub_topic();
                let mut sub_handles = Vec::new();

                // Subscribers
                for _ in 0..subscribers {
                    let topic = topic.clone();
                    let h = std::thread::spawn(move || {
                        let mut count = 0;
                        while count < 1000 {
                            if topic.recv().is_ok() {
                                count += 1;
                            }
                        }
                        count
                    });
                    sub_handles.push(h);
                }

                // Publisher
                for _ in 0..10000 {
                    let msg = vec![0u8; self.message_size_bytes];
                    topic.publish(&msg).ok();
                }

                for h in sub_handles {
                    message_count += h.join().unwrap_or(0);
                }
            }
            IpcType::SharedContext => {
                // Concurrent writes to shared context
                let shared_ctx = create_shared_context();
                let mut handles = Vec::new();

                for i in 0..self.ct_count {
                    let shared_ctx = shared_ctx.clone();
                    let h = std::thread::spawn(move || {
                        let mut count = 0;
                        for j in 0..1000 {
                            let offset = (i * 1000 + j) * self.message_size_bytes;
                            shared_ctx.write(offset, &vec![0u8; self.message_size_bytes]).ok();
                            count += 1;
                        }
                        count
                    });
                    handles.push(h);
                }

                for h in handles {
                    message_count += h.join().unwrap_or(0);
                }
            }
        }

        let elapsed = start.elapsed();
        results.total_messages = message_count;
        results.throughput_msg_per_sec = (message_count as f64 / elapsed.as_secs_f64()) as u64;
        results.latency_us = (elapsed.as_micros() as u64) / (message_count as u64);

        println!("IPC Throughput: {} messages, {} msg/sec", message_count, results.throughput_msg_per_sec);

        results
    }
}

// Configuration: Request-response
let workload = IPCThroughputWorkload {
    ct_count: 10,
    ipc_type: IpcType::RequestResponse,
    message_size_bytes: 256,
    duration_seconds: 30,
};
let results = workload.run();
assert!(results.throughput_msg_per_sec > 100_000, "Throughput must be > 100k msg/sec");
```

### Workload 3: Checkpoint Overhead Benchmark
```
pub struct CheckpointOverheadWorkload {
    pub memory_size_mb: usize,
    pub checkpoint_interval: CheckpointInterval,
    pub duration_seconds: u32,
}

pub enum CheckpointInterval {
    OnDemand,
    EveryNSeconds(u32),
    DeltaBased { dirty_page_threshold: f32 },
}

impl CheckpointOverheadWorkload {
    pub fn run(&self) -> BenchmarkResults {
        let mut results = BenchmarkResults::new("Checkpoint Overhead");

        // Setup: Create CT with large working memory
        let mut memory = vec![0u8; self.memory_size_mb * 1_000_000];
        let mut checkpoint_times = Vec::new();
        let mut restore_times = Vec::new();

        let start = Instant::now();
        let mut checkpoint_count = 0;

        while start.elapsed().as_secs() < self.duration_seconds as u64 {
            // Dirty some memory
            for i in 0..memory.len() {
                memory[i] = (i % 256) as u8;
            }

            // Take checkpoint
            let cp_start = Instant::now();
            let checkpoint_id = ct_checkpoint().ok();
            let cp_elapsed = cp_start.elapsed().as_micros() as u64;
            checkpoint_times.push(cp_elapsed);
            checkpoint_count += 1;

            // Simulate work
            std::thread::sleep(Duration::from_millis(100));

            // Restore checkpoint
            if let Some(cp_id) = checkpoint_id {
                let restore_start = Instant::now();
                let _ = ct_resume(cp_id);
                let restore_elapsed = restore_start.elapsed().as_micros() as u64;
                restore_times.push(restore_elapsed);
            }
        }

        results.total_checkpoints = checkpoint_count;
        results.p50_checkpoint_us = percentile(&checkpoint_times, 50);
        results.p99_checkpoint_us = percentile(&checkpoint_times, 99);
        results.p50_restore_us = percentile(&restore_times, 50);
        results.p99_restore_us = percentile(&restore_times, 99);

        println!("Checkpoint Overhead: {} checkpoints, P99 create: {}us, P99 restore: {}us",
            checkpoint_count, results.p99_checkpoint_us, results.p99_restore_us);

        results
    }
}

// Configuration: 1GB memory, periodic checkpoints
let workload = CheckpointOverheadWorkload {
    memory_size_mb: 1024,
    checkpoint_interval: CheckpointInterval::EveryNSeconds(10),
    duration_seconds: 60,
};
let results = workload.run();
assert!(results.p99_checkpoint_us < 10_000_000, "P99 checkpoint must be < 10 seconds");
```

### Workload 4: Distributed Multi-Machine Benchmark
```
pub struct DistributedMultiMachineWorkload {
    pub machine_count: usize,
    pub agents_per_machine: usize,
    pub cross_machine_message_rate: f32,
    pub failure_injection_rate: f32,
    pub duration_seconds: u32,
}

impl DistributedMultiMachineWorkload {
    pub fn run(&self, machines: &[MachineHandle]) -> BenchmarkResults {
        let mut results = BenchmarkResults::new("Distributed Multi-Machine");
        let mut latencies = Vec::new();
        let mut failures = 0;
        let mut recoveries = 0;

        let start = Instant::now();

        // Simulate cross-machine communication
        while start.elapsed().as_secs() < self.duration_seconds as u64 {
            for (i, machine) in machines.iter().enumerate() {
                for agent_id in 0..self.agents_per_machine {
                    if rand::random::<f32>() < self.cross_machine_message_rate / 1000.0 {
                        let remote_machine = (i + 1) % self.machine_count;
                        let remote_agent = rand::random::<usize>() % self.agents_per_machine;

                        let msg_start = Instant::now();

                        // Inject failure randomly
                        let injected_failure = rand::random::<f32>() < self.failure_injection_rate;

                        match machine.send_to_remote(remote_machine, remote_agent, injected_failure) {
                            Ok(()) => {
                                let latency = msg_start.elapsed().as_millis() as u64;
                                latencies.push(latency);
                            }
                            Err(_) => {
                                failures += 1;
                                recoveries += 1;
                            }
                        }
                    }
                }
            }
        }

        results.total_messages = latencies.len();
        results.p50_latency_ms = percentile(&latencies, 50) as u64;
        results.p99_latency_ms = percentile(&latencies, 99) as u64;
        results.failure_count = failures;
        results.recovery_count = recoveries;

        println!("Distributed: {} messages, P99: {}ms, {} failures, {} recoveries",
            latencies.len(), results.p99_latency_ms, failures, recoveries);

        results
    }
}

// Configuration: 3 machines, 10 agents each
let workload = DistributedMultiMachineWorkload {
    machine_count: 3,
    agents_per_machine: 10,
    cross_machine_message_rate: 1000.0,  // 1000 messages/second
    failure_injection_rate: 0.1,         // 10% failure rate
    duration_seconds: 60,
};
let results = workload.run(&machines);
assert!(results.p99_latency_ms < 100, "P99 cross-machine latency must be < 100ms");
```

### Performance Regression Test Suite
```
#[test]
fn test_performance_baseline_request_response() {
    let workload = IPCThroughputWorkload {
        ct_count: 4,
        ipc_type: IpcType::RequestResponse,
        message_size_bytes: 256,
        duration_seconds: 10,
    };
    let results = workload.run();

    // Target: > 50,000 messages/second
    assert!(results.throughput_msg_per_sec > 50_000,
        "Request-response throughput {} < target 50k msg/sec",
        results.throughput_msg_per_sec);
}

#[test]
fn test_performance_baseline_fault_recovery() {
    let workload = FaultRecoveryWorkload {
        ct_count: 5,
        exception_rate: 1.0,
        tool_failure_rate: 0.2,
        duration_seconds: 30,
    };
    let results = workload.run();

    // Target: P99 < 100ms
    assert!(results.p99_latency_us < 100_000,
        "Fault recovery P99 {}us > target 100ms",
        results.p99_latency_us);
}

#[test]
fn test_performance_baseline_checkpoint() {
    let workload = CheckpointOverheadWorkload {
        memory_size_mb: 512,
        checkpoint_interval: CheckpointInterval::EveryNSeconds(5),
        duration_seconds: 30,
    };
    let results = workload.run();

    // Target: P99 < 100ms
    assert!(results.p99_checkpoint_us < 100_000_000,
        "Checkpoint P99 {}us > target 100ms",
        results.p99_checkpoint_us);
}

#[test]
fn test_performance_baseline_distributed() {
    let machines = setup_test_machines(3);
    let workload = DistributedMultiMachineWorkload {
        machine_count: 3,
        agents_per_machine: 5,
        cross_machine_message_rate: 100.0,
        failure_injection_rate: 0.05,
        duration_seconds: 30,
    };
    let results = workload.run(&machines);

    // Target: P99 < 100ms
    assert!(results.p99_latency_ms < 100,
        "Distributed P99 {}ms > target 100ms",
        results.p99_latency_ms);
}
```

## Dependencies
- **Blocked by:** Week 1-22 (All implementation & integration)
- **Blocking:** Week 24 (Final validation & launch)

## Acceptance Criteria
1. Fault recovery P99 latency < 100ms
2. IPC throughput > 50,000 messages/second
3. Checkpoint overhead P99 < 100ms
4. Distributed cross-machine latency P99 < 100ms
5. All workloads complete successfully
6. Regression test suite passes
7. Performance meets or exceeds targets
8. Benchmark methodology documented
9. Hardware compatibility verified on 3+ platforms
10. Scaling tests show linear or better scaling to 1000 agents

## Design Principles Alignment
- **Validation:** Comprehensive benchmarks ensure system meets requirements
- **Reproducibility:** Documented workloads enable reproducing results
- **Observability:** Detailed metrics support performance analysis
- **Regression Prevention:** Automated tests catch performance regressions
