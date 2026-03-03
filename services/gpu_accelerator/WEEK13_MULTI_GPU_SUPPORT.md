# Week 13 — Multi-GPU Support: Model Parallelism, Data Parallelism & Failover

**Document Version:** 1.0
**Status:** Design Review
**Author:** Principal Engineer - GPU Acceleration Team
**Date:** 2026-03-02
**Project:** XKernal Cognitive Substrate OS

---

## Executive Summary

This document specifies the implementation of comprehensive multi-GPU support for the XKernal GPU Accelerator service, enabling horizontal scaling of cognitive workloads across multiple GPUs through model parallelism and data parallelism strategies. The design ensures fault tolerance, automatic load balancing, and efficient inter-GPU communication while maintaining sub-millisecond P2P transfer latencies and achieving 1.8× throughput improvement with data parallelism.

---

## Problem Statement

### Current Limitations
- Single GPU architecture limits cognitive model size (16GB VRAM max typical device)
- No horizontal scaling capability for batch inference workloads
- No failover mechanism for GPU failures
- Agents and models lack GPU affinity awareness
- Cognitive substrate cannot efficiently distribute large transformer models

### Requirements
1. Support 2, 4, and 8 GPU configurations
2. Enable inference on models requiring >16GB VRAM through model parallelism
3. Achieve >1.8× throughput scaling with data parallelism on 2 GPUs
4. Maintain <1ms P2P transfer latency between GPUs
5. Automatically rebalance workloads when GPU utilization diverges >10%
6. Gracefully degrade and failover on GPU failures
7. Provide transparent load balancing from agent perspective

---

## Architecture

### 3.1 Multi-GPU Device Management

```
┌─────────────────────────────────────────────────────────┐
│         Multi-GPU Device Management Layer               │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐ │
│  │ GPU Device 0 │  │ GPU Device 1 │  │ GPU Device N │ │
│  │ (16GB VRAM)  │  │ (16GB VRAM)  │  │ (16GB VRAM)  │ │
│  └──────────────┘  └──────────────┘  └──────────────┘ │
│         ▲                 ▲                 ▲          │
│         └─────────────────┼─────────────────┘          │
│               Health Monitor / Affinity                │
│                                                         │
│  ┌──────────────────────────────────────────────────┐  │
│  │  GpuDeviceRegistry: enumerate, register, query   │  │
│  │  - Device capability detection                   │  │
│  │  - VRAM allocation tracking                      │  │
│  │  - Health check & telemetry                      │  │
│  └──────────────────────────────────────────────────┘  │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

### 3.2 Model & Data Parallelism Framework

**Model Parallelism (Vertical Scaling):**
- Split large models by layer across GPUs
- GPU 0 executes layers 0-15, GPU 1 executes layers 16-31
- Forward pass: GPU 0 → GPU 1 → GPU 0 (sequential)
- Reduces per-GPU memory footprint

**Data Parallelism (Horizontal Scaling):**
- Replicate model across all GPUs
- Distribute batch across GPUs: batch[0:N/2] → GPU 0, batch[N/2:N] → GPU 1
- Compute gradients in parallel, synchronize via AllReduce
- Increases throughput for inference

### 3.3 Inter-GPU Communication

- **P2P Transfers:** Direct GPU-to-GPU DMA (PCIe Gen 4 capable)
- **Collective Operations:** AllReduce, Broadcast, Scatter for synchronization
- **Host Staging:** Fallback for non-P2P capable devices
- **Bandwidth Target:** 32GB/s+ on PCIe Gen 4

### 3.4 Load Balancing & Failover

- Real-time GPU utilization monitoring (queue depth, compute time)
- Target: All GPUs within 10% utilization of each other
- Failover: Remaining GPUs absorb failed GPU's workload
- Health checks every 100ms

---

## Implementation

### 4.1 Core Components

```rust
// Multi-GPU Manager - Master orchestrator
pub struct MultiGpuManager {
    devices: Vec<GpuDevice>,
    registry: Arc<GpuDeviceRegistry>,
    model_parallelizer: Arc<ModelParallelizer>,
    data_parallelizer: Arc<DataParallelizer>,
    p2p_engine: Arc<P2PTransferEngine>,
    load_balancer: Arc<GpuLoadBalancer>,
    failover_handler: Arc<GpuFailoverHandler>,
}

impl MultiGpuManager {
    pub fn new(config: MultiGpuConfig) -> Result<Self> {
        let registry = Arc::new(GpuDeviceRegistry::scan()?);
        let devices = registry.enumerate_devices()?;

        assert!(devices.len() >= 2, "Multi-GPU requires 2+ devices");

        Ok(Self {
            devices: devices.clone(),
            registry,
            model_parallelizer: Arc::new(ModelParallelizer::new(&devices)?),
            data_parallelizer: Arc::new(DataParallelizer::new(&devices)?),
            p2p_engine: Arc::new(P2PTransferEngine::new(&devices)?),
            load_balancer: Arc::new(GpuLoadBalancer::new(&devices)),
            failover_handler: Arc::new(GpuFailoverHandler::new(&devices)?),
        })
    }

    pub async fn allocate_model(
        &self,
        model: &CognitiveModel,
        strategy: ParallelismStrategy,
    ) -> Result<AllocatedModel> {
        match strategy {
            ParallelismStrategy::Model => {
                self.model_parallelizer.partition(model, &self.devices).await
            }
            ParallelismStrategy::Data => {
                self.data_parallelizer.replicate(model, &self.devices).await
            }
            ParallelismStrategy::Hybrid => {
                // Combine model parallelism for model layers,
                // data parallelism for batch distribution
                self.model_parallelizer
                    .partition(model, &self.devices[0..2])
                    .await
                    .and_then(|m| self.data_parallelizer.replicate(&m, &self.devices).await)
            }
        }
    }
}

// GPU Device Registry - Enumerate and manage available GPUs
pub struct GpuDeviceRegistry {
    devices: Vec<GpuDeviceInfo>,
    capabilities: HashMap<u32, GpuCapabilities>,
}

impl GpuDeviceRegistry {
    pub fn scan() -> Result<Self> {
        let device_count = unsafe { cuDeviceGetCount(&mut count) }?;
        let mut devices = Vec::new();

        for i in 0..device_count {
            let device = GpuDevice::from_ordinal(i)?;
            let info = GpuDeviceInfo {
                ordinal: i,
                name: device.name()?,
                vram_total: device.total_memory()?,
                compute_capability: device.compute_capability()?,
            };
            devices.push(info);
        }

        Ok(Self {
            devices,
            capabilities: Self::probe_capabilities(&devices)?,
        })
    }

    pub fn health_check(&self, device_id: u32) -> Result<DeviceHealth> {
        // Execute lightweight GPU kernel, verify memory access patterns
        let device = self.get_device(device_id)?;
        device.test_memory_bandwidth()?;
        device.test_compute_capability()?;

        Ok(DeviceHealth {
            device_id,
            status: HealthStatus::Healthy,
            timestamp: Instant::now(),
        })
    }
}

// Model Parallelizer - Split models across GPUs by layer
pub struct ModelParallelizer;

impl ModelParallelizer {
    pub fn new(devices: &[GpuDevice]) -> Result<Self> {
        Ok(Self)
    }

    pub async fn partition(
        &self,
        model: &CognitiveModel,
        devices: &[GpuDevice],
    ) -> Result<AllocatedModel> {
        let layer_count = model.layer_count();
        let layers_per_gpu = (layer_count + devices.len() - 1) / devices.len();

        let mut partitions = Vec::new();
        for (gpu_idx, device) in devices.iter().enumerate() {
            let start_layer = gpu_idx * layers_per_gpu;
            let end_layer = std::cmp::min((gpu_idx + 1) * layers_per_gpu, layer_count);

            let partition = model.extract_layers(start_layer..end_layer)?;
            device.allocate(partition).await?;
            partitions.push(AllocatedPartition { device_id: gpu_idx, layers: start_layer..end_layer });
        }

        Ok(AllocatedModel::ModelParallel(partitions))
    }
}

// Data Parallelizer - Replicate model, distribute batch across GPUs
pub struct DataParallelizer;

impl DataParallelizer {
    pub fn new(devices: &[GpuDevice]) -> Result<Self> {
        Ok(Self)
    }

    pub async fn replicate(
        &self,
        model: &CognitiveModel,
        devices: &[GpuDevice],
    ) -> Result<AllocatedModel> {
        let mut replicas = Vec::new();

        for (idx, device) in devices.iter().enumerate() {
            device.allocate(model.clone()).await?;
            replicas.push(AllocatedReplica { device_id: idx });
        }

        Ok(AllocatedModel::DataParallel(replicas))
    }

    pub async fn distribute_batch(
        &self,
        batch: &InferenceBatch,
        replicas: &[AllocatedReplica],
    ) -> Result<Vec<BatchPartition>> {
        let batch_size = batch.size();
        let partition_size = (batch_size + replicas.len() - 1) / replicas.len();

        let partitions = replicas
            .iter()
            .enumerate()
            .map(|(idx, replica)| {
                let start = idx * partition_size;
                let end = std::cmp::min((idx + 1) * partition_size, batch_size);
                BatchPartition {
                    device_id: replica.device_id,
                    samples: start..end,
                }
            })
            .collect();

        Ok(partitions)
    }
}

// P2P Transfer Engine - Direct GPU-to-GPU communication
pub struct P2PTransferEngine {
    p2p_capable: HashMap<(u32, u32), bool>,
    transfer_bandwidth: f64, // GB/s
}

impl P2PTransferEngine {
    pub fn new(devices: &[GpuDevice]) -> Result<Self> {
        let mut p2p_capable = HashMap::new();

        for i in 0..devices.len() {
            for j in 0..devices.len() {
                if i != j {
                    let can_access = devices[i].can_access_peer(&devices[j])?;
                    p2p_capable.insert((i as u32, j as u32), can_access);
                }
            }
        }

        Ok(Self {
            p2p_capable,
            transfer_bandwidth: 32.0, // PCIe Gen 4
        })
    }

    pub async fn transfer(
        &self,
        src_device_id: u32,
        dst_device_id: u32,
        src_ptr: *const u8,
        size_bytes: usize,
    ) -> Result<TransferMetrics> {
        let start = Instant::now();

        if self.p2p_capable.get(&(src_device_id, dst_device_id)).unwrap_or(&false) {
            // Direct P2P transfer via PCIe
            unsafe {
                cuMemcpyPeerAsync(
                    dst_ptr,
                    dst_device_id as i32,
                    src_ptr,
                    src_device_id as i32,
                    size_bytes,
                )?;
            }
        } else {
            // Fallback: Host staging memory
            let mut staging = vec![0u8; size_bytes];
            self.copy_device_to_host(src_device_id, src_ptr, &mut staging).await?;
            self.copy_host_to_device(dst_device_id, &staging, dst_ptr).await?;
        }

        let elapsed = start.elapsed();
        let latency_ms = elapsed.as_secs_f64() * 1000.0;

        assert!(latency_ms < 1.0, "P2P latency SLA violated: {:.3}ms", latency_ms);

        Ok(TransferMetrics {
            bytes_transferred: size_bytes,
            latency_ms,
            bandwidth_gbps: (size_bytes as f64 / (1024.0 * 1024.0 * 1024.0)) / elapsed.as_secs_f64(),
        })
    }

    pub async fn collective_allreduce(
        &self,
        device_ids: &[u32],
        data_ptr: *mut f32,
        element_count: usize,
    ) -> Result<()> {
        // Ring AllReduce algorithm: minimize bottlenecks
        for step in 0..device_ids.len() - 1 {
            for (idx, _device_id) in device_ids.iter().enumerate() {
                let src_idx = (idx + device_ids.len() - 1) % device_ids.len();
                let dst_idx = idx;

                self.transfer(
                    device_ids[src_idx],
                    device_ids[dst_idx],
                    data_ptr as *const u8,
                    element_count * std::mem::size_of::<f32>(),
                ).await?;
            }
        }

        Ok(())
    }
}

// GPU Load Balancer - Monitor and rebalance workloads
pub struct GpuLoadBalancer {
    device_count: usize,
    utilization_history: Arc<Mutex<Vec<Vec<f32>>>>,
    rebalance_threshold: f32, // 10% target
}

impl GpuLoadBalancer {
    pub fn new(devices: &[GpuDevice]) -> Self {
        Self {
            device_count: devices.len(),
            utilization_history: Arc::new(Mutex::new(vec![Vec::new(); devices.len()])),
            rebalance_threshold: 0.1,
        }
    }

    pub async fn monitor(&self) -> Result<LoadBalanceDecision> {
        let mut history = self.utilization_history.lock().await;
        let current_utilization = self.sample_utilization().await?;

        for (idx, util) in current_utilization.iter().enumerate() {
            history[idx].push(*util);
        }

        let mean_util = current_utilization.iter().sum::<f32>() / current_utilization.len() as f32;
        let max_deviation = current_utilization
            .iter()
            .map(|u| (u - mean_util).abs() / mean_util)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0);

        if max_deviation > self.rebalance_threshold {
            Ok(LoadBalanceDecision::Rebalance {
                source_device: self.find_hottest_device(&current_utilization),
                target_device: self.find_coolest_device(&current_utilization),
            })
        } else {
            Ok(LoadBalanceDecision::Balanced)
        }
    }

    async fn sample_utilization(&self) -> Result<Vec<f32>> {
        // Query GPU utilization metrics (queue depth, compute time)
        Ok(vec![0.5, 0.48, 0.52]) // Placeholder
    }

    fn find_hottest_device(&self, utilization: &[f32]) -> u32 {
        utilization
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(idx, _)| idx as u32)
            .unwrap_or(0)
    }

    fn find_coolest_device(&self, utilization: &[f32]) -> u32 {
        utilization
            .iter()
            .enumerate()
            .min_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(idx, _)| idx as u32)
            .unwrap_or(0)
    }
}

// GPU Failover Handler - Manage failures and graceful degradation
pub struct GpuFailoverHandler {
    healthy_devices: Arc<Mutex<Vec<bool>>>,
    health_check_interval: Duration,
}

impl GpuFailoverHandler {
    pub fn new(devices: &[GpuDevice]) -> Result<Self> {
        Ok(Self {
            healthy_devices: Arc::new(Mutex::new(vec![true; devices.len()])),
            health_check_interval: Duration::from_millis(100),
        })
    }

    pub async fn health_monitor(&self, registry: Arc<GpuDeviceRegistry>) {
        loop {
            for device_id in 0..registry.device_count() {
                match registry.health_check(device_id) {
                    Ok(health) if health.status == HealthStatus::Healthy => {
                        self.healthy_devices.lock().await[device_id as usize] = true;
                    }
                    _ => {
                        self.healthy_devices.lock().await[device_id as usize] = false;
                        eprintln!("GPU {} failed, initiating failover", device_id);
                    }
                }
            }
            tokio::time::sleep(self.health_check_interval).await;
        }
    }

    pub async fn get_healthy_devices(&self) -> Result<Vec<u32>> {
        let healthy = self.healthy_devices.lock().await;
        Ok(healthy
            .iter()
            .enumerate()
            .filter_map(|(idx, &is_healthy)| {
                if is_healthy {
                    Some(idx as u32)
                } else {
                    None
                }
            })
            .collect())
    }

    pub async fn failover(&self, failed_device_id: u32) -> Result<()> {
        self.healthy_devices.lock().await[failed_device_id as usize] = false;

        // Remaining GPUs absorb workload
        let healthy = self.get_healthy_devices().await?;
        eprintln!("Failover complete: {} GPUs remaining", healthy.len());

        Ok(())
    }
}
```

### 4.2 Integration with Agent Scheduler

```rust
pub struct AgentScheduler {
    gpu_manager: Arc<MultiGpuManager>,
}

impl AgentScheduler {
    pub async fn infer(
        &self,
        agent: &CognitiveAgent,
        batch: &InferenceBatch,
    ) -> Result<InferenceOutput> {
        // Query GPU affinity for agent
        let preferred_gpu = agent.gpu_affinity();

        // Allocate model if not cached
        let allocated = self.gpu_manager
            .allocate_model(&agent.model, ParallelismStrategy::Data)
            .await?;

        // Distribute batch using data parallelism
        let partitions = match &allocated {
            AllocatedModel::DataParallel(replicas) => {
                self.gpu_manager.data_parallelizer
                    .distribute_batch(batch, replicas)
                    .await?
            }
            _ => vec![],
        };

        // Execute inference on each partition
        let mut outputs = Vec::new();
        for partition in partitions {
            let output = self.execute_inference_partition(partition).await?;
            outputs.push(output);
        }

        // Synchronize results via AllReduce
        self.gpu_manager.p2p_engine.collective_allreduce(
            &self.get_device_ids(&outputs),
            outputs[0].data_ptr() as *mut f32,
            outputs[0].element_count(),
        ).await?;

        Ok(self.merge_outputs(outputs))
    }
}
```

---

## Testing Strategy

### 5.1 Test Configurations

- **2-GPU Setup:** Model parallelism (16GB model split), data parallelism (batch 32)
- **4-GPU Setup:** Hybrid parallelism (model across 2 GPUs, data across 4)
- **8-GPU Setup:** Maximum data parallelism, collective operations stress test

### 5.2 Test Cases

```rust
#[tokio::test]
async fn test_model_parallelism_16gb_model_2gpu() {
    let manager = MultiGpuManager::new(MultiGpuConfig::two_gpu()).await.unwrap();
    let model = CognitiveModel::load_16gb().await.unwrap();

    let allocated = manager
        .allocate_model(&model, ParallelismStrategy::Model)
        .await
        .unwrap();

    let batch = InferenceBatch::new(vec![test_input()]);
    let output = manager.infer(&batch, &allocated).await.unwrap();

    assert!(output.is_correct(&expected_output()));
}

#[tokio::test]
async fn test_data_parallelism_throughput_2gpu() {
    let manager = MultiGpuManager::new(MultiGpuConfig::two_gpu()).await.unwrap();
    let model = CognitiveModel::load_8gb().await.unwrap();

    let allocated = manager
        .allocate_model(&model, ParallelismStrategy::Data)
        .await
        .unwrap();

    let batch = InferenceBatch::new(create_batch_32());
    let start = Instant::now();
    let output = manager.infer(&batch, &allocated).await.unwrap();
    let elapsed = start.elapsed();

    let throughput = (batch.size() as f64 / elapsed.as_secs_f64()) as u32;
    assert!(throughput > 1800, "Throughput {}/sec < 1.8x target", throughput);
}

#[tokio::test]
async fn test_p2p_transfer_latency() {
    let manager = MultiGpuManager::new(MultiGpuConfig::two_gpu()).await.unwrap();
    let test_size = 1024 * 1024; // 1MB

    let metrics = manager.p2p_engine
        .transfer(0, 1, test_ptr, test_size)
        .await
        .unwrap();

    assert!(metrics.latency_ms < 1.0, "P2P latency {:.3}ms > 1ms SLA", metrics.latency_ms);
}

#[tokio::test]
async fn test_load_balancing_within_10_percent() {
    let manager = MultiGpuManager::new(MultiGpuConfig::four_gpu()).await.unwrap();

    for _ in 0..1000 {
        let decision = manager.load_balancer.monitor().await.unwrap();
        match decision {
            LoadBalanceDecision::Balanced => { /* Expected */ }
            LoadBalanceDecision::Rebalance { .. } => {
                panic!("Load imbalance detected, rebalancing needed");
            }
        }
    }
}

#[tokio::test]
async fn test_gpu_failover_graceful_degradation() {
    let manager = MultiGpuManager::new(MultiGpuConfig::four_gpu()).await.unwrap();

    manager.failover_handler.failover(2).await.unwrap();

    let healthy = manager.failover_handler.get_healthy_devices().await.unwrap();
    assert_eq!(healthy.len(), 3);
    assert!(!healthy.contains(&2));

    // Remaining GPUs absorb load
    let batch = InferenceBatch::new(create_batch_32());
    let output = manager.infer(&batch).await.unwrap();
    assert!(output.is_valid());
}
```

---

## Acceptance Criteria

1. **Multi-GPU enumeration:** Correctly identify 2, 4, 8 GPU systems
2. **Model parallelism:** 16GB model split across 2 GPUs, inference results match single-GPU baseline
3. **Data parallelism:** 32-element batch distributed across 2 GPUs, achieve >1.8× throughput
4. **P2P transfers:** <1ms latency for 1MB transfers between GPUs
5. **Load balancing:** Maintain <10% utilization deviation across all GPUs over 10-minute window
6. **Failover:** GPU failure detected within 200ms, remaining GPUs absorb workload with <5% latency degradation
7. **Stress test:** 8-GPU configuration sustains 10,000 inference requests with <2% failure rate

---

## Design Principles

1. **Transparent Scaling:** Agents unaware of GPU topology; scheduler handles allocation
2. **Fault Isolation:** GPU failures do not cascade; system degrades gracefully
3. **Minimal Latency:** P2P transfers and synchronization keep inference latency <10% overhead
4. **Automatic Optimization:** Load balancer continuously monitors and rebalances
5. **Heterogeneous Support:** Handle mixed GPU generations and P2P capability differences

---

## Rollout Plan

- **Phase 1:** 2-GPU model parallelism in staging (Week 13)
- **Phase 2:** 4-GPU data parallelism in staging (Week 14)
- **Phase 3:** Production rollout with 8-GPU canary (Week 15)

---

## Appendix: Performance Targets

| Metric | Target | SLA |
|--------|--------|-----|
| P2P Transfer Latency (1MB) | <1ms | Hard |
| Data Parallelism Throughput (2 GPU) | >1.8× | Hard |
| Load Balancing Deviation | <10% | Soft |
| GPU Failover Detection | <200ms | Soft |
| Failover Latency Overhead | <5% | Soft |

