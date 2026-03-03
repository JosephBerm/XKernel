# Engineer 3 — Kernel: IPC, Signals, Exceptions & Checkpointing — Week 28

## Phase: PHASE 3 — Benchmarking, Testing & Validation

## Weekly Objective

Complete benchmarking phase: distributed multi-machine performance, stress tests, combined workload scenarios, and final performance report comparing all results to targets.

## Document References
- **Primary:** Section 7 (Performance Targets), Section 6.2 (Exit Criteria)
- **Supporting:** All prior sections

## Deliverables
- [ ] Distributed channel latency: single machine to 3+ machines
- [ ] Network failover testing: performance during node failures
- [ ] Stress test: 1000+ concurrent distributed messages
- [ ] Combined workload: fault recovery + IPC + checkpointing together
- [ ] Scaling validation: 100-1000 agents across multiple machines
- [ ] Hotspot analysis: identify remaining performance bottlenecks
- [ ] Optimization opportunities: recommendations for future work
- [ ] Performance summary: all benchmarks vs targets
- [ ] Hardware compatibility report: results on 3+ reference platforms
- [ ] Final performance report: comprehensive analysis and conclusions

## Technical Specifications

### Distributed Benchmarking Suite
```
pub struct DistributedBenchmark {
    pub machine_count: Vec<usize>,  // 1, 2, 3 machines
    pub agents_per_machine: usize,
    pub message_rate_per_second: u32,
    pub duration_seconds: u32,
}

impl DistributedBenchmark {
    pub fn run_all(&self) -> DistributedReport {
        let mut report = DistributedReport::new("Distributed Performance");

        for machine_count in &self.machine_count {
            let machines = setup_test_machines(*machine_count);
            let result = self.benchmark_machines(&machines);
            report.add_result(*machine_count, result);
        }

        report
    }

    fn benchmark_machines(&self, machines: &[MachineHandle]) -> MachineResult {
        let mut latencies = Vec::new();
        let mut failures = 0;
        let mut recovery_time = 0;

        let start = Instant::now();

        while start.elapsed().as_secs() < self.duration_seconds as u64 {
            for (i, machine) in machines.iter().enumerate() {
                for agent_id in 0..self.agents_per_machine {
                    let target_machine = (i + 1) % machines.len();

                    let msg_start = Instant::now();
                    match machine.send_to_remote(target_machine, agent_id, false) {
                        Ok(()) => {
                            latencies.push(msg_start.elapsed().as_millis() as u64);
                        }
                        Err(_) => {
                            failures += 1;
                            let recovery_start = Instant::now();
                            let _ = machine.recover_from_failure(target_machine);
                            recovery_time += recovery_start.elapsed().as_millis() as u64;
                        }
                    }
                }
            }
        }

        MachineResult {
            machine_count: machines.len(),
            total_messages: latencies.len(),
            p50: percentile(&latencies, 50),
            p99: percentile(&latencies, 99),
            failures,
            avg_recovery_time: if failures > 0 { recovery_time / failures } else { 0 },
        }
    }
}
```

### Combined Workload Scenario
```
pub struct CombinedWorkload {
    pub duration_seconds: u32,
}

impl CombinedWorkload {
    pub fn run(&self) -> CombinedResult {
        let mut result = CombinedResult::new();

        // Setup: Multiple agents with exception handlers and checkpoints
        let mut agents = Vec::new();
        for i in 0..10 {
            agents.push(setup_agent_with_handlers(i));
        }

        let start = Instant::now();

        while start.elapsed().as_secs() < self.duration_seconds as u64 {
            for agent in &mut agents {
                // Simulate reasoning cycle
                agent.observe().ok();

                // 1/3 of time: take checkpoint
                if rand::random::<f32>() < 0.33 {
                    agent.checkpoint().ok();
                }

                // 1/3 of time: communicate with peer
                if rand::random::<f32>() < 0.33 {
                    let peer_idx = rand::random::<usize>() % agents.len();
                    agent.send_to_peer(peer_idx).ok();
                }

                // 1/3 of time: call tool (might fail)
                if rand::random::<f32>() < 0.33 {
                    let _ = agent.call_tool();
                }

                agent.act().ok();
            }
        }

        // Collect metrics
        result.total_observations = agents.iter().map(|a| a.observation_count).sum();
        result.total_checkpoints = agents.iter().map(|a| a.checkpoint_count).sum();
        result.total_messages = agents.iter().map(|a| a.message_count).sum();
        result.total_tool_calls = agents.iter().map(|a| a.tool_call_count).sum();
        result.total_exceptions = agents.iter().map(|a| a.exception_count).sum();

        result
    }
}
```

### Performance Summary vs Targets
```
pub struct PerformanceSummary {
    pub benchmarks: HashMap<String, BenchmarkTarget>,
}

pub struct BenchmarkTarget {
    pub name: String,
    pub target: u64,
    pub actual: u64,
    pub unit: String,
    pub met: bool,
}

impl PerformanceSummary {
    pub fn generate() -> Self {
        let mut benchmarks = HashMap::new();

        // IPC Targets
        benchmarks.insert("request_response_p50_us".to_string(), BenchmarkTarget {
            name: "Request-Response P50".to_string(),
            target: 1,
            actual: measure_request_response_p50(),
            unit: "microseconds".to_string(),
            met: false,
        });

        benchmarks.insert("request_response_p99_us".to_string(), BenchmarkTarget {
            name: "Request-Response P99".to_string(),
            target: 5,
            actual: measure_request_response_p99(),
            unit: "microseconds".to_string(),
            met: false,
        });

        // Fault Recovery Targets
        benchmarks.insert("fault_recovery_p99_ms".to_string(), BenchmarkTarget {
            name: "Fault Recovery P99".to_string(),
            target: 100,
            actual: measure_fault_recovery_p99(),
            unit: "milliseconds".to_string(),
            met: false,
        });

        // Checkpoint Targets
        benchmarks.insert("checkpoint_creation_p99_ms".to_string(), BenchmarkTarget {
            name: "Checkpoint Creation P99".to_string(),
            target: 100,
            actual: measure_checkpoint_p99(),
            unit: "milliseconds".to_string(),
            met: false,
        });

        // Distributed Targets
        benchmarks.insert("distributed_latency_p99_ms".to_string(), BenchmarkTarget {
            name: "Distributed Cross-Machine P99".to_string(),
            target: 100,
            actual: measure_distributed_p99(),
            unit: "milliseconds".to_string(),
            met: false,
        });

        // Check if targets met
        for target in benchmarks.values_mut() {
            target.met = target.actual <= target.target;
        }

        Self { benchmarks }
    }

    pub fn print_summary(&self) {
        println!("\n=== PERFORMANCE SUMMARY ===\n");
        let mut all_met = true;

        for (_, target) in &self.benchmarks {
            let status = if target.met { "✓ PASS" } else { "✗ FAIL" };
            println!("{}: {} {}/{} {}",
                status, target.name, target.actual, target.target, target.unit);
            if !target.met {
                all_met = false;
            }
        }

        println!("\n{}\n", if all_met { "ALL TARGETS MET ✓" } else { "SOME TARGETS MISSED ✗" });
    }
}
```

## Dependencies
- **Blocked by:** Week 27 (Checkpoint benchmarking)
- **Blocking:** None (benchmarking phase complete)

## Acceptance Criteria
1. Distributed channel latency p99 < 100ms (network dependent)
2. Stress test: 1000+ concurrent messages, > 95% success
3. Combined workload runs all subsystems together
4. Scaling validated from 10 to 1000 agents
5. All benchmark targets met or exceeded
6. Hardware compatibility confirmed on 3+ platforms
7. Performance report complete with all results
8. Final summary shows target status
9. Bottleneck analysis complete
10. Recommendations for optimization documented

## Design Principles Alignment
- **Completeness:** Final benchmarking validates entire system
- **Validation:** All targets confirmed achievable
- **Documentation:** Comprehensive report captures performance profile
- **Future Work:** Recommendations guide continued optimization
