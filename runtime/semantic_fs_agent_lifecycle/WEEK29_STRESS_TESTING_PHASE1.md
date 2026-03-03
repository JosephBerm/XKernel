# XKernal Cognitive Substrate OS - WEEK 29: Agent Lifecycle Stress Testing Phase 1

**Engineer:** Engineer 8 (Semantic FS & Agent Lifecycle)
**Date:** 2026-03-02
**Phase:** Stress Testing Phase 1
**Document Version:** 1.0
**Status:** In Progress

---

## 1. Executive Summary and Scope

### Objective
WEEK 29 marks the commencement of comprehensive stress testing for the Agent Lifecycle Manager (ALM) in the XKernal 4-layer architecture. This phase focuses on validating system resilience under failure conditions, establishing MTTR baselines, and identifying critical failure modes that could impact AI agent orchestration.

### Key Deliverables
- **Failure Injection Framework**: Pluggable fault injection system with deterministic and probabilistic modes
- **Health Check Stress Tests**: Degradation simulation and concurrent load validation (1000+ req/s)
- **Restart Policy Stress Tests**: Storm prevention, budget enforcement, and dependency-aware restart validation
- **Hot-Reload Stress Tests**: Config reload, schema migration, and rolling updates under full load
- **Chaos Engineering Tests**: Random failures, network partitions, clock skew, disk I/O delays, memory pressure
- **MTTR Metrics Framework**: Automated measurement with per-failure-type targets and percentile tracking
- **Comprehensive Test Report**: Test matrix with key findings and remediation recommendations

### Acceptance Criteria
- All failure modes documented with reproduction procedures
- Health check resilience verified with <2% false positive rate under degraded conditions
- Restart policies preventing restart storms with exponential backoff validation
- MTTR targets met: Agent restart <200ms, Service recovery <500ms, Cascading failure mitigation <1s
- Chaos test suite running continuously with >500 iterations without unrecovered failures

---

## 2. Failure Injection Framework Design

### 2.1 Architecture Overview

The Failure Injection Framework (FIF) is a modular system for injecting controlled faults at specific points in the Agent Lifecycle Manager. It operates in two modes:

**Deterministic Mode**: Predefined fault sequences for reproducible testing
**Probabilistic Mode**: Random fault injection with configurable probability distributions

### 2.2 Fault Type Taxonomy

```
Fault Type      | Layer   | Description                          | Severity
----------------|---------|--------------------------------------|----------
CRASH           | L2/L3   | Unexpected process termination      | Critical
HANG            | L2/L3   | Process becomes unresponsive         | Critical
SLOW            | L1/L2   | Request latency degradation (10x)   | High
CORRUPT         | L1/L3   | State corruption or data loss        | Critical
TIMEOUT         | L2      | Operation exceeds deadline          | Medium
PARTIAL_FAULT   | L1      | Subset of instances fail            | High
NETWORK_DELAY   | L1      | Inter-agent communication latency   | Medium
```

### 2.3 Injection Points

| Injection Point | Component | Trigger Event | Fault Injectability |
|-----------------|-----------|---------------|----------------------|
| Health Endpoint | Health Probe | GET /health | High |
| Restart Handler | ALM State Machine | restart() call | High |
| Reload Pipeline | Config Manager | reload() call | High |
| State Persistence | L1 Service | write_state() | Medium |
| Agent Dispatch | Task Scheduler | dispatch_task() | High |
| Health Aggregator | Health Monitor | aggregate_health() | Medium |
| Recovery Handler | Restart Logic | execute_recovery() | High |

### 2.4 Injection Scheduling

#### Deterministic Scheduling
```
sequence: [
  {
    time_ms: 0,
    fault_type: SLOW,
    duration_ms: 5000,
    injection_point: "health_endpoint",
    target_latency_ms: 500
  },
  {
    time_ms: 5000,
    fault_type: HANG,
    duration_ms: 2000,
    injection_point: "restart_handler",
    recovery_trigger: "manual"
  },
  {
    time_ms: 7000,
    fault_type: CRASH,
    injection_point: "agent_dispatch",
    restart_policy: "exponential_backoff"
  }
]
```

#### Probabilistic Scheduling
```
probability_distribution: {
  crash_rate: 0.001,              // 0.1% of dispatches
  hang_probability: 0.002,        // 0.2% of operations
  slow_probability: 0.01,         // 1% slow requests
  corrupt_probability: 0.0005,    // 0.05% data corruption
  time_scale_ms: 100              // Evaluation window
}
```

### 2.5 Rust Implementation: Failure Injection Framework

```rust
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::Mutex;
use rand::Rng;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultType {
    Crash,
    Hang,
    Slow,
    Corrupt,
    Timeout,
    PartialFault,
    NetworkDelay,
}

#[derive(Debug, Clone)]
pub struct FaultInjectionConfig {
    pub fault_type: FaultType,
    pub injection_point: String,
    pub duration_ms: u64,
    pub target_latency_ms: Option<u64>,
    pub recovery_trigger: RecoveryTrigger,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RecoveryTrigger {
    Immediate,
    Manual,
    Timeout(u64),
    HealthCheck,
}

pub struct FailureInjectionFramework {
    config: Arc<Mutex<Vec<FaultInjectionConfig>>>,
    active_faults: Arc<Mutex<Vec<(FaultType, Instant)>>>,
    injection_stats: Arc<Mutex<InjectionStats>>,
    rng: Arc<Mutex<rand::rngs::ThreadRng>>,
}

#[derive(Debug, Default, Clone)]
pub struct InjectionStats {
    pub total_injections: u64,
    pub successful_injections: u64,
    pub failed_injections: u64,
    pub recovery_count: u64,
    pub unrecovered_count: u64,
}

impl FailureInjectionFramework {
    pub fn new() -> Self {
        Self {
            config: Arc::new(Mutex::new(Vec::new())),
            active_faults: Arc::new(Mutex::new(Vec::new())),
            injection_stats: Arc::new(Mutex::new(InjectionStats::default())),
            rng: Arc::new(Mutex::new(rand::thread_rng())),
        }
    }

    pub fn inject_deterministic(&self, configs: Vec<FaultInjectionConfig>) {
        let mut cfg = self.config.lock();
        cfg.extend(configs);
    }

    pub fn should_inject_fault(
        &self,
        injection_point: &str,
        fault_type: FaultType,
    ) -> bool {
        let mut active = self.active_faults.lock();

        // Remove expired faults
        let now = Instant::now();
        active.retain(|(_, start)| {
            now.duration_since(*start) < Duration::from_millis(5000)
        });

        // Check if injection point matches config
        let config = self.config.lock();
        config.iter().any(|c| {
            c.injection_point == injection_point && c.fault_type == fault_type
        })
    }

    pub fn inject_probabilistic(&self, fault_type: FaultType, probability: f64) -> bool {
        let mut rng = self.rng.lock();
        let rand_val: f64 = rng.gen();
        rand_val < probability
    }

    pub fn record_injection(&self, fault_type: FaultType) {
        let mut stats = self.injection_stats.lock();
        stats.total_injections += 1;

        let mut active = self.active_faults.lock();
        active.push((fault_type, Instant::now()));
    }

    pub fn record_recovery(&self) {
        let mut stats = self.injection_stats.lock();
        stats.recovery_count += 1;
    }

    pub fn get_stats(&self) -> InjectionStats {
        self.injection_stats.lock().clone()
    }
}

// Injection Point Wrapper
pub async fn injection_point<F, T>(
    framework: &FailureInjectionFramework,
    point_name: &str,
    fault_type: FaultType,
    future: F,
) -> Result<T, String>
where
    F: std::future::Future<Output = Result<T, String>>,
{
    if framework.should_inject_fault(point_name, fault_type) {
        framework.record_injection(fault_type);
        match fault_type {
            FaultType::Crash => {
                return Err("Injected CRASH fault".to_string());
            }
            FaultType::Hang => {
                tokio::time::sleep(Duration::from_secs(30)).await;
                return Err("Injected HANG fault".to_string());
            }
            FaultType::Slow => {
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
            _ => {}
        }
    }

    future.await
}
```

---

## 3. Health Check Stress Testing

### 3.1 Degraded Endpoint Simulation

Health check endpoints must gracefully degrade without triggering cascading failures. Testing involves:

- **Partial Response Degradation**: Health endpoint returns with missing fields
- **Slow Response**: Responses exceed normal latency thresholds
- **Intermittent Failures**: Alternating success/failure patterns
- **Corrupted Data**: Invalid JSON or malformed health states

### 3.2 Timeout Boundary Testing

Critical timeout thresholds for health checks:

| Threshold | Scenario | Expected Behavior | Recovery |
|-----------|----------|-------------------|----------|
| 100ms | Fast health check baseline | Complete within window | Immediate |
| 500ms | Degraded but acceptable | Counted as partial health | Retry with backoff |
| 1s | Concerning degradation | Trigger warning level | Increase timeout, partial recovery |
| 5s | Critical degradation | Mark unhealthy, escalate | Immediate recovery action |

### 3.3 Cascading Health Failure Testing

```rust
pub struct HealthCheckStressTest {
    health_monitor: Arc<HealthMonitor>,
    cascade_simulator: Arc<CascadeSimulator>,
}

impl HealthCheckStressTest {
    pub async fn test_cascading_failure_propagation(&self) {
        // Phase 1: Single agent unhealthy
        let agent1_id = "agent_001";
        self.cascade_simulator
            .inject_unhealthy(agent1_id, Duration::from_secs(2))
            .await;

        // Phase 2: Verify dependent agents detect failure
        tokio::time::sleep(Duration::from_millis(100)).await;
        let health = self.health_monitor.get_aggregate_health().await;
        assert!(!health.is_healthy, "Cascade not detected");

        // Phase 3: Verify exponential backoff prevents thundering herd
        let mut retry_times = Vec::new();
        for i in 0..10 {
            let start = Instant::now();
            let _ = self.health_monitor.check_health(agent1_id).await;
            retry_times.push(start.elapsed());
        }

        // Verify exponential growth: t_n > t_{n-1} * backoff_factor
        for i in 1..retry_times.len() {
            let ratio = retry_times[i].as_secs_f64() / retry_times[i - 1].as_secs_f64();
            assert!(ratio > 1.0, "Backoff not applied");
        }
    }

    pub async fn test_concurrent_health_checks_1000plus_rps(&self) {
        const CONCURRENT_REQUESTS: usize = 1500;
        const TEST_DURATION_SECS: u64 = 60;

        let start = Instant::now();
        let mut handles = vec![];

        for i in 0..CONCURRENT_REQUESTS {
            let monitor = self.health_monitor.clone();
            let agent_id = format!("agent_{:04}", i % 100);

            let handle = tokio::spawn(async move {
                let mut request_count = 0;
                let mut errors = 0;

                while start.elapsed() < Duration::from_secs(TEST_DURATION_SECS) {
                    match monitor.check_health(&agent_id).await {
                        Ok(_) => request_count += 1,
                        Err(_) => errors += 1,
                    }
                }
                (request_count, errors)
            });

            handles.push(handle);
        }

        let mut total_requests = 0;
        let mut total_errors = 0;
        for handle in handles {
            let (reqs, errs) = handle.await.unwrap();
            total_requests += reqs;
            total_errors += errs;
        }

        let error_rate = (total_errors as f64) / (total_requests as f64);
        assert!(error_rate < 0.02, "Error rate {:.2}% exceeds 2%", error_rate * 100.0);

        let throughput = total_requests as f64 / TEST_DURATION_SECS as f64;
        println!("Health Check Throughput: {:.0} req/s", throughput);
        assert!(throughput > 1000.0, "Throughput below 1000 req/s");
    }

    pub async fn test_false_positive_false_negative_rates(&self) {
        let mut false_positives = 0;
        let mut false_negatives = 0;
        const ITERATIONS: usize = 1000;

        for _ in 0..ITERATIONS {
            // Scenario 1: Agent actually healthy, marked unhealthy (false negative)
            let health_result = self.health_monitor.check_health("healthy_agent").await;
            if health_result.is_err() {
                false_negatives += 1;
            }

            // Scenario 2: Agent unhealthy, marked healthy (false positive)
            self.cascade_simulator
                .inject_unhealthy("unhealthy_agent", Duration::from_millis(500))
                .await;
            let health_result = self.health_monitor.check_health("unhealthy_agent").await;
            if health_result.is_ok() {
                false_positives += 1;
            }
        }

        let fp_rate = (false_positives as f64) / (ITERATIONS as f64);
        let fn_rate = (false_negatives as f64) / (ITERATIONS as f64);

        assert!(fp_rate < 0.02, "False positive rate {:.2}%", fp_rate * 100.0);
        assert!(fn_rate < 0.02, "False negative rate {:.2}%", fn_rate * 100.0);
    }
}
```

---

## 4. Restart Policy Stress Testing

### 4.1 Rapid Restart Detection

The restart policy must detect and prevent rapid restart loops. Detection window: 1 second, threshold: 3 restarts.

```rust
pub struct RestartPolicy {
    restart_history: Arc<Mutex<Vec<Instant>>>,
    exponential_backoff: Arc<Mutex<ExponentialBackoff>>,
    restart_budget: Arc<Mutex<RestartBudget>>,
}

#[derive(Clone, Copy)]
pub struct ExponentialBackoff {
    initial_delay_ms: u64,
    max_delay_ms: u64,
    multiplier: f64,
    jitter_factor: f64,
}

pub struct RestartBudget {
    window_duration_ms: u64,
    max_restarts_per_window: usize,
    last_window_start: Instant,
    restarts_in_window: usize,
}

impl RestartPolicy {
    pub fn should_allow_restart(&self) -> bool {
        let mut budget = self.restart_budget.lock();
        let now = Instant::now();

        // Check if window has expired
        if now.duration_since(budget.last_window_start).as_millis() as u64
            > budget.window_duration_ms {
            budget.restarts_in_window = 0;
            budget.last_window_start = now;
        }

        if budget.restarts_in_window >= budget.max_restarts_per_window {
            return false;
        }

        budget.restarts_in_window += 1;
        true
    }

    pub fn get_backoff_delay(&self) -> Duration {
        let mut history = self.restart_history.lock();
        let now = Instant::now();

        // Count restarts in last 1 second
        history.retain(|&instant| now.duration_since(instant) < Duration::from_secs(1));

        let restart_count = history.len();
        let mut backoff = self.exponential_backoff.lock();

        let delay_ms = if restart_count == 0 {
            backoff.initial_delay_ms
        } else {
            let exponent = (restart_count - 1).min(10) as f64;
            let base_delay = backoff.initial_delay_ms as f64
                * backoff.multiplier.powf(exponent);
            base_delay.min(backoff.max_delay_ms as f64) as u64
        };

        // Add jitter
        let mut rng = rand::thread_rng();
        let jitter = (delay_ms as f64 * backoff.jitter_factor * (rng.gen::<f64>() - 0.5)) as u64;
        let final_delay = (delay_ms as i64 + jitter as i64).max(0) as u64;

        history.push(now);
        Duration::from_millis(final_delay)
    }
}
```

### 4.2 Restart Storm Prevention

```rust
pub async fn test_restart_storm_prevention() {
    let policy = RestartPolicy::new(
        ExponentialBackoff {
            initial_delay_ms: 10,
            max_delay_ms: 5000,
            multiplier: 2.0,
            jitter_factor: 0.1,
        },
        RestartBudget {
            window_duration_ms: 1000,
            max_restarts_per_window: 5,
            last_window_start: Instant::now(),
            restarts_in_window: 0,
        },
    );

    let mut restart_times = vec![];
    for i in 0..20 {
        if !policy.should_allow_restart() {
            println!("Restart {} blocked by budget", i);
            continue;
        }

        let delay = policy.get_backoff_delay();
        tokio::time::sleep(delay).await;
        restart_times.push(Instant::now());

        // Verify exponential growth in delays
        if i > 1 {
            let ratio = delay.as_millis() as f64
                / restart_times[i - 2].elapsed().as_millis() as f64;
            assert!(ratio > 1.5, "Exponential backoff not working");
        }
    }

    // Verify that total time exceeds expected exponential growth
    let total_time = restart_times.last().unwrap()
        .elapsed_since(restart_times.first().unwrap());
    println!("Total restart time: {:?}", total_time);
}
```

### 4.3 Dependency-Aware Restart Ordering

```rust
pub struct DependencyAwareRestartOrchestrator {
    dependency_graph: Arc<DependencyGraph>,
    restart_order: Arc<Mutex<Vec<String>>>,
}

impl DependencyAwareRestartOrchestrator {
    pub async fn restart_with_dependencies(&self, agent_id: &str) {
        let dependencies = self.dependency_graph.get_dependencies(agent_id);

        // Topological sort: restart dependencies first
        let mut restart_queue = vec![agent_id.to_string()];
        let mut visited = std::collections::HashSet::new();

        while let Some(current) = restart_queue.pop() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());

            // Get dependent services (not dependencies)
            let dependents = self.dependency_graph.get_dependents(&current);
            for dependent in dependents {
                restart_queue.push(dependent);
            }
        }

        // Execute restarts in dependency order
        for agent in &self.restart_order.lock().iter().cloned().collect::<Vec<_>>() {
            self.restart_agent(agent).await;
        }
    }

    pub async fn restart_during_active_request_handling(&self) {
        // Graceful restart: drain active requests first
        let grace_period = Duration::from_secs(30);
        let start = Instant::now();

        while !self.all_requests_drained().await
            && start.elapsed() < grace_period {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        // Force kill remaining requests if grace period exceeded
        if !self.all_requests_drained().await {
            self.force_terminate_requests().await;
        }

        // Now safe to restart
        self.restart_agent_safe().await;
    }

    async fn all_requests_drained(&self) -> bool {
        // Implementation: check active request count
        true
    }

    async fn force_terminate_requests(&self) {
        // Implementation: terminate with prejudice
    }

    async fn restart_agent_safe(&self) {
        // Implementation: actual restart
    }

    async fn restart_agent(&self, agent_id: &str) {
        // Implementation
    }
}
```

---

## 5. Hot-Reload Stress Testing

### 5.1 Configuration Reload Under Full Load

```rust
pub struct HotReloadStressTest {
    config_manager: Arc<ConfigManager>,
    load_simulator: Arc<LoadSimulator>,
    reload_stats: Arc<Mutex<ReloadStats>>,
}

#[derive(Debug, Clone, Default)]
pub struct ReloadStats {
    pub successful_reloads: u64,
    pub failed_reloads: u64,
    pub reloads_during_load: u64,
    pub average_reload_time_ms: f64,
    pub max_reload_time_ms: u64,
}

impl HotReloadStressTest {
    pub async fn test_config_reload_under_full_load(&self) {
        const LOAD_CONCURRENT_TASKS: usize = 500;
        const TEST_DURATION_SECS: u64 = 120;
        const RELOAD_INTERVAL_MS: u64 = 5000;

        // Start baseline load
        let mut load_handles = vec![];
        for i in 0..LOAD_CONCURRENT_TASKS {
            let simulator = self.load_simulator.clone();
            let handle = tokio::spawn(async move {
                simulator.generate_load(format!("task_{}", i)).await
            });
            load_handles.push(handle);
        }

        // Interleave config reloads
        let reload_start = Instant::now();
        while reload_start.elapsed() < Duration::from_secs(TEST_DURATION_SECS) {
            let reload_time_start = Instant::now();

            match self.config_manager.reload_config().await {
                Ok(_) => {
                    let reload_time = reload_time_start.elapsed().as_millis() as u64;
                    let mut stats = self.reload_stats.lock();
                    stats.successful_reloads += 1;
                    stats.average_reload_time_ms =
                        (stats.average_reload_time_ms * (stats.successful_reloads as f64 - 1.0)
                            + reload_time as f64) / (stats.successful_reloads as f64);
                    stats.max_reload_time_ms = stats.max_reload_time_ms.max(reload_time);
                }
                Err(_) => {
                    let mut stats = self.reload_stats.lock();
                    stats.failed_reloads += 1;
                }
            }

            tokio::time::sleep(Duration::from_millis(RELOAD_INTERVAL_MS)).await;
        }

        // Verify load continued successfully
        for handle in load_handles {
            let _ = handle.await;
        }

        let stats = self.reload_stats.lock().clone();
        assert!(stats.failed_reloads == 0, "Reloads failed during load");
        assert!(stats.average_reload_time_ms < 100.0, "Reload time too high");
    }

    pub async fn test_schema_migration_during_operation(&self) {
        // V1 -> V2 schema with backward compatibility
        let migration_strategy = MigrationStrategy::BackwardCompatible;

        let result = self.config_manager
            .migrate_schema("v1".to_string(), "v2".to_string(), migration_strategy)
            .await;

        assert!(result.is_ok(), "Schema migration failed");

        // Verify existing agents can still operate
        let agent_health = self.load_simulator.verify_agent_health().await;
        assert!(agent_health.is_healthy, "Agents unhealthy after migration");
    }

    pub async fn test_binary_rolling_update(&self) {
        const BATCH_SIZE: usize = 10;
        const TOTAL_AGENTS: usize = 100;

        for batch_num in 0..(TOTAL_AGENTS / BATCH_SIZE) {
            let batch_start = batch_num * BATCH_SIZE;
            let batch_end = (batch_num + 1) * BATCH_SIZE;

            // Update batch
            for agent_id in batch_start..batch_end {
                self.config_manager
                    .update_agent_binary(&format!("agent_{}", agent_id), "v2")
                    .await
                    .expect("Update failed");
            }

            // Verify batch health before proceeding
            tokio::time::sleep(Duration::from_millis(500)).await;
            let batch_health = self.load_simulator
                .verify_batch_health(batch_start, batch_end)
                .await;
            assert!(batch_health.is_healthy, "Batch {} unhealthy", batch_num);
        }
    }

    pub async fn test_partial_reload_failure_recovery(&self) {
        // Simulate partial reload failure: 3 of 10 services fail to reload
        let result = self.config_manager
            .reload_config_with_failures(3)
            .await;

        // Should enter recovery state, not complete failure
        assert!(result.is_partial_success(), "Should be partial success");

        // Verify rollback mechanism
        let rolled_back = self.config_manager.rollback_failed_reloads().await;
        assert!(rolled_back.is_ok(), "Rollback failed");

        // Verify system health restored
        let health = self.load_simulator.verify_agent_health().await;
        assert!(health.is_healthy, "Health not restored");
    }
}
```

### 5.2 Reload Ordering with Dependent Services

Reload must respect service dependencies to prevent broken links:

```rust
pub struct ReloadOrchestrator {
    dependency_graph: Arc<DependencyGraph>,
    reload_order: Arc<Mutex<Vec<String>>>,
    reload_validator: Arc<ReloadValidator>,
}

impl ReloadOrchestrator {
    pub async fn execute_ordered_reload(&self) -> Result<(), String> {
        let reload_sequence = self.compute_reload_order().await?;

        for service_id in reload_sequence {
            // 1. Validate current state before reload
            self.reload_validator.validate_pre_reload(&service_id).await?;

            // 2. Perform reload with dependency awareness
            self.reload_service(&service_id).await?;

            // 3. Verify dependent services still healthy
            let dependents = self.dependency_graph.get_dependents(&service_id);
            for dependent in dependents {
                self.reload_validator.validate_dependent(&dependent).await?;
            }
        }

        Ok(())
    }

    async fn compute_reload_order(&self) -> Result<Vec<String>, String> {
        // Topological sort of dependency graph
        // Services with no dependencies reload first
        Ok(vec![])
    }

    async fn reload_service(&self, service_id: &str) -> Result<(), String> {
        // Implementation
        Ok(())
    }
}
```

---

## 6. Chaos Engineering Tests

### 6.1 Random Agent Kill

```rust
pub struct ChaosEngineeringTests {
    agent_pool: Arc<AgentPool>,
    chaos_generator: Arc<Mutex<rand::rngs::ThreadRng>>,
}

impl ChaosEngineeringTests {
    pub async fn test_random_agent_kill(&self) {
        const ITERATIONS: usize = 100;
        const KILL_PROBABILITY: f64 = 0.3; // 30% kill rate per iteration

        let mut recovery_times = vec![];
        let mut failures_recovered = 0;

        for iteration in 0..ITERATIONS {
            let agent_ids = self.agent_pool.list_active_agents().await;

            for agent_id in agent_ids {
                let mut rng = self.chaos_generator.lock();
                if rng.gen::<f64>() < KILL_PROBABILITY {
                    let kill_start = Instant::now();

                    // Kill agent
                    self.agent_pool.kill_agent(&agent_id).await;

                    // Measure recovery time
                    let mut recovered = false;
                    for _ in 0..50 {
                        tokio::time::sleep(Duration::from_millis(100)).await;
                        if self.agent_pool.is_agent_healthy(&agent_id).await {
                            let recovery_time = kill_start.elapsed();
                            recovery_times.push(recovery_time);
                            failures_recovered += 1;
                            recovered = true;
                            break;
                        }
                    }

                    assert!(recovered, "Agent {} did not recover", agent_id);
                }
            }
        }

        // Analyze recovery metrics
        recovery_times.sort();
        let p50 = recovery_times[recovery_times.len() / 2];
        let p99 = recovery_times[(recovery_times.len() * 99) / 100];

        println!("Recovery Time P50: {:?}", p50);
        println!("Recovery Time P99: {:?}", p99);
        assert!(p50 < Duration::from_millis(200), "P50 recovery time too high");
        assert!(p99 < Duration::from_millis(500), "P99 recovery time too high");
    }
}
```

### 6.2 Network Partition Simulation

```rust
pub async fn test_network_partition_between_agents(&self) {
    // Partition network: agent_001 <-> agent_002 isolated
    let partition_start = Instant::now();

    self.chaos_generator
        .create_partition(&["agent_001", "agent_002"])
        .await;

    // Verify partition is effective
    let result = self.agent_pool
        .send_message("agent_001", "agent_002", "test")
        .await;
    assert!(result.is_err(), "Partition not effective");

    // Wait for partition detection
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Verify health check detects partition
    let health = self.agent_pool.get_agent_health("agent_001").await;
    assert!(!health.is_healthy, "Partition not detected by health check");

    // Heal partition
    self.chaos_generator.heal_partition().await;

    // Measure recovery time
    let mut recovered = false;
    for _ in 0..50 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        let result = self.agent_pool
            .send_message("agent_001", "agent_002", "test")
            .await;
        if result.is_ok() {
            let recovery_time = partition_start.elapsed();
            println!("Network partition recovery: {:?}", recovery_time);
            assert!(recovery_time < Duration::from_secs(5), "Recovery too slow");
            recovered = true;
            break;
        }
    }

    assert!(recovered, "Network partition not recovered");
}
```

### 6.3 Clock Skew Injection

```rust
pub async fn test_clock_skew_injection(&self) {
    const SKEW_AMOUNT_MS: i64 = 5000; // 5 second skew

    // Inject positive skew
    self.chaos_generator
        .inject_clock_skew(Duration::from_millis(SKEW_AMOUNT_MS as u64))
        .await;

    // Verify timestamp ordering issues don't break system
    let timestamps = self.collect_operation_timestamps(100).await;

    // System should still function despite clock anomalies
    let agents_healthy = self.agent_pool.check_all_agents_healthy().await;
    assert!(agents_healthy.count() > 0, "All agents failed under clock skew");

    // Verify causality enforcement still works
    let causality_preserved = self.verify_causality_constraints().await;
    assert!(causality_preserved, "Causality violated under clock skew");

    // Remove skew
    self.chaos_generator.remove_clock_skew().await;
}
```

### 6.4 Disk I/O Delay

```rust
pub async fn test_disk_io_delay_injection(&self) {
    const IO_DELAY_MS: u64 = 500;

    self.chaos_generator
        .inject_io_delay(Duration::from_millis(IO_DELAY_MS))
        .await;

    // Start state persistence operations
    let persist_start = Instant::now();
    let mut persist_times = vec![];

    for i in 0..50 {
        let op_start = Instant::now();
        let _ = self.agent_pool.persist_agent_state(i).await;
        persist_times.push(op_start.elapsed());
    }

    // Verify operations complete despite I/O delays
    assert!(!persist_times.is_empty(), "Persistence operations failed");

    // Verify no deadlocks under sustained I/O delays
    let health = self.agent_pool.check_all_agents_healthy().await;
    assert!(health.is_ok(), "Agents unhealthy under I/O delays");

    self.chaos_generator.remove_io_delay().await;
}
```

### 6.5 Memory Pressure During Lifecycle Operations

```rust
pub async fn test_memory_pressure_during_lifecycle(&self) {
    const MEMORY_LIMIT_PERCENT: u32 = 80; // Limit to 80% of available

    self.chaos_generator
        .set_memory_pressure(MEMORY_LIMIT_PERCENT)
        .await;

    // Start agent lifecycle operations under memory pressure
    let mut lifecycle_results = vec![];

    for i in 0..20 {
        let result = self.agent_pool
            .create_agent(&format!("pressure_agent_{}", i))
            .await;
        lifecycle_results.push(result);
    }

    // Some creates may fail, but system shouldn't panic or deadlock
    let success_count = lifecycle_results.iter().filter(|r| r.is_ok()).count();
    assert!(success_count > 0, "No agents created under memory pressure");

    // Verify OOM protection mechanisms
    let stats = self.chaos_generator.get_memory_stats().await;
    assert!(stats.oom_kills < 5, "Too many OOM kills");

    self.chaos_generator.release_memory_pressure().await;
}
```

---

## 7. MTTR Metrics Framework

### 7.1 Measurement Methodology

Mean Time To Recovery (MTTR) is measured from fault detection to operational recovery:

```
MTTR = Recovery Time - Detection Time
```

**Detection Time**: Time when system first detects failure (health check, error log, exception)
**Recovery Time**: Time when system returns to stable operational state (verified by health check)

### 7.2 Per-Failure-Type MTTR Targets

| Failure Type | Detection Target | Recovery Target | MTTR Target | SLA |
|--------------|-----------------|-----------------|-------------|-----|
| Agent Crash | <50ms | <200ms | 150ms | 99.5% |
| Service Hang | <100ms | <500ms | 400ms | 99% |
| Health Check Degradation | <100ms | <1000ms | 900ms | 98% |
| Network Partition | <200ms | <2000ms | 1800ms | 95% |
| Cascading Failure | <100ms | <3000ms | 2900ms | 95% |
| Config Reload Failure | <50ms | <500ms | 450ms | 99% |
| Memory Exhaustion | <500ms | <2000ms | 1500ms | 95% |

### 7.3 Rust Implementation: MTTR Measurement

```rust
use std::collections::BTreeMap;

pub struct MTTRMetricsCollector {
    failure_events: Arc<Mutex<Vec<FailureEvent>>>,
    recovery_events: Arc<Mutex<Vec<RecoveryEvent>>>,
    mttr_samples: Arc<Mutex<BTreeMap<String, Vec<Duration>>>>,
}

#[derive(Debug, Clone)]
pub struct FailureEvent {
    pub failure_id: String,
    pub failure_type: String,
    pub component_id: String,
    pub timestamp: Instant,
    pub detection_method: String,
}

#[derive(Debug, Clone)]
pub struct RecoveryEvent {
    pub failure_id: String,
    pub recovery_method: String,
    pub timestamp: Instant,
    pub success: bool,
}

#[derive(Debug, Clone)]
pub struct MTTRReport {
    pub failure_type: String,
    pub p50: Duration,
    pub p95: Duration,
    pub p99: Duration,
    pub max: Duration,
    pub mean: Duration,
    pub sample_count: usize,
    pub sla_compliance: f64,
}

impl MTTRMetricsCollector {
    pub fn new() -> Self {
        Self {
            failure_events: Arc::new(Mutex::new(Vec::new())),
            recovery_events: Arc::new(Mutex::new(Vec::new())),
            mttr_samples: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    pub fn record_failure(&self, failure: FailureEvent) {
        let mut events = self.failure_events.lock();
        events.push(failure);
    }

    pub fn record_recovery(&self, recovery: RecoveryEvent) {
        let mut events = self.recovery_events.lock();
        events.push(recovery);

        // Calculate MTTR if matching failure found
        self.calculate_mttr(&recovery);
    }

    fn calculate_mttr(&self, recovery: &RecoveryEvent) {
        let failures = self.failure_events.lock();

        if let Some(failure) = failures.iter()
            .find(|f| f.failure_id == recovery.failure_id) {

            let mttr = recovery.timestamp
                .duration_since(failure.timestamp);

            let mut samples = self.mttr_samples.lock();
            samples
                .entry(failure.failure_type.clone())
                .or_insert_with(Vec::new)
                .push(mttr);
        }
    }

    pub fn generate_report(&self, failure_type: &str) -> MTTRReport {
        let samples = self.mttr_samples.lock();
        let mut durations = samples
            .get(failure_type)
            .cloned()
            .unwrap_or_default();

        durations.sort();

        let p50_idx = durations.len() / 2;
        let p95_idx = (durations.len() * 95) / 100;
        let p99_idx = (durations.len() * 99) / 100;

        let mean = if !durations.is_empty() {
            let sum: u128 = durations.iter()
                .map(|d| d.as_millis())
                .sum();
            Duration::from_millis((sum / durations.len() as u128) as u64)
        } else {
            Duration::ZERO
        };

        MTTRReport {
            failure_type: failure_type.to_string(),
            p50: durations.get(p50_idx).copied().unwrap_or(Duration::ZERO),
            p95: durations.get(p95_idx).copied().unwrap_or(Duration::ZERO),
            p99: durations.get(p99_idx).copied().unwrap_or(Duration::ZERO),
            max: durations.last().copied().unwrap_or(Duration::ZERO),
            mean,
            sample_count: durations.len(),
            sla_compliance: self.calculate_sla_compliance(failure_type),
        }
    }

    fn calculate_sla_compliance(&self, failure_type: &str) -> f64 {
        let targets = self.get_mttr_targets();
        let samples = self.mttr_samples.lock();

        if let Some(durations) = samples.get(failure_type) {
            if let Some(target) = targets.get(failure_type) {
                let compliant = durations.iter()
                    .filter(|d| d <= target)
                    .count();
                (compliant as f64) / (durations.len() as f64)
            } else {
                0.0
            }
        } else {
            0.0
        }
    }

    fn get_mttr_targets(&self) -> BTreeMap<String, Duration> {
        vec![
            ("agent_crash".to_string(), Duration::from_millis(150)),
            ("service_hang".to_string(), Duration::from_millis(400)),
            ("health_degradation".to_string(), Duration::from_millis(900)),
            ("network_partition".to_string(), Duration::from_millis(1800)),
            ("cascading_failure".to_string(), Duration::from_millis(2900)),
        ]
        .into_iter()
        .collect()
    }
}
```

### 7.4 Automated Measurement and Percentile Tracking

```rust
pub async fn run_automated_mttr_measurement() {
    let collector = Arc::new(MTTRMetricsCollector::new());
    let test_runner = Arc::new(TestRunner::new());

    let test_scenarios = vec![
        ("agent_crash", Self::test_agent_crash),
        ("service_hang", Self::test_service_hang),
        ("health_degradation", Self::test_health_degradation),
    ];

    for (name, test_fn) in test_scenarios {
        println!("Running MTTR measurement for: {}", name);

        for iteration in 0..100 {
            let failure_id = format!("{}_iter_{}", name, iteration);
            let failure = FailureEvent {
                failure_id: failure_id.clone(),
                failure_type: name.to_string(),
                component_id: "test_component".to_string(),
                timestamp: Instant::now(),
                detection_method: "health_check".to_string(),
            };

            collector.record_failure(failure);

            // Run test scenario
            // (test_fn)(&collector, &failure_id).await;

            // Wait for recovery
            tokio::time::sleep(Duration::from_secs(1)).await;

            let recovery = RecoveryEvent {
                failure_id,
                recovery_method: "automatic".to_string(),
                timestamp: Instant::now(),
                success: true,
            };

            collector.record_recovery(recovery);
        }

        // Generate report
        let report = collector.generate_report(name);
        println!("MTTR Report for {}: {:#?}", name, report);
    }
}
```

---

## 8. Results Summary and Test Matrix

### 8.1 Comprehensive Test Coverage

| Test Category | Test Name | Status | MTTR (P99) | Pass/Fail |
|---|---|---|---|---|
| **Failure Injection** | Deterministic Scheduling | In Progress | N/A | Pending |
| | Probabilistic Scheduling | In Progress | N/A | Pending |
| | Multi-Fault Cascading | In Progress | N/A | Pending |
| **Health Checks** | Concurrent Load 1000+ RPS | Planned | <500ms | Pending |
| | Timeout Boundary Testing | Planned | <100ms | Pending |
| | False Positive Rate | Planned | <2% | Pending |
| | Cascading Propagation | Planned | <1s | Pending |
| **Restart Policies** | Rapid Restart Detection | Planned | <100ms | Pending |
| | Restart Storm Prevention | Planned | <5s | Pending |
| | Budget Enforcement | Planned | N/A | Pending |
| | Dependency-Aware Ordering | Planned | N/A | Pending |
| **Hot-Reload** | Config Reload Under Load | Planned | <100ms | Pending |
| | Schema Migration | Planned | <2s | Pending |
| | Rolling Update | Planned | <5s | Pending |
| | Partial Failure Recovery | Planned | <500ms | Pending |
| **Chaos Engineering** | Random Agent Kill | Planned | <200ms | Pending |
| | Network Partition | Planned | <2s | Pending |
| | Clock Skew Injection | Planned | N/A | Pending |
| | Disk I/O Delays | Planned | <1s | Pending |
| | Memory Pressure | Planned | <2s | Pending |

### 8.2 Key Findings and Recommendations

**Phase 1 Acceptance Status**: Awaiting test execution

**Expected Outcomes**:
- Failure injection framework operational and validated
- Health check resilience demonstrated under 1000+ RPS load
- Restart policies preventing cascading failures
- MTTR targets achieved within SLA bounds
- Chaos tests identifying edge cases for Phase 2 hardening

**Critical Metrics to Monitor**:
- MTTR compliance across all failure types
- False positive/negative rate in health detection
- Restart storm frequency and duration
- Recovery time percentiles (P50, P95, P99)
- Resource utilization during stress scenarios

**Phase 2 Recommendations**:
- Implement identified edge case handling
- Optimize hot-reload latency below 50ms
- Enhance cascading failure detection
- Add load-adaptive restart policies

---

## 9. Execution Plan and Timeline

### Week 29 Milestones
- **Days 1-2**: Failure injection framework implementation and validation
- **Days 3-4**: Health check stress testing execution
- **Days 5-6**: Restart policy stress testing execution
- **Days 7-8**: Hot-reload and chaos testing execution
- **Day 9-10**: Report generation and Phase 2 planning

### Success Criteria
- All tests execute without unrecovered failures
- MTTR metrics within 10% of targets
- Documentation complete with remediation paths
- Ready for Phase 2 hardening and optimization

---

**Document Status**: Ready for Implementation
**Next Review**: Post-execution, Day 10 of WEEK 29
**Approval**: Engineer 8 (Semantic FS & Agent Lifecycle)
