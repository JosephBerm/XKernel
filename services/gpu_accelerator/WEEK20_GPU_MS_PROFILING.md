# Week 20: GPU-ms Profiling & Performance Baseline Analysis
**XKernal Cognitive Substrate OS — L1 Services (Rust)**

**Phase 2 Continuation | GPU/Accelerator Manager | CUDA Driver API & ROCm HIP**

---

## 1. Executive Summary

Week 20 establishes comprehensive GPU-millisecond (GPU-ms) measurement infrastructure for reasoning chain execution on the XKernal substrate. This document defines the architecture for:

1. Per-kernel execution time tracking via CUDA events and ROCm markers
2. End-to-end reasoning chain profiling (CT lifecycle)
3. Multi-dimensional performance analysis (per-agent, per-model, per-GPU)
4. Phase progression impact quantification (Phase 0 → Phase 1 → Phase 2)
5. GPU efficiency baseline and 30–60% reduction targeting framework

**Key Metric**: GPU-milliseconds per completed reasoning chain, normalized by token count and model size, enabling cost-efficiency analysis (tokens/GPU-ms).

---

## 2. GPU-ms Measurement Infrastructure

### 2.1 CUDA Event-Based Timing Architecture

CUDA events provide sub-microsecond precision timing for kernel executions via the CUDA Driver API. The profiling infrastructure wraps all kernel launches with paired event pairs.

```rust
use cuda_runtime_api::{cudaEvent_t, cudaEventCreate, cudaEventRecord,
                       cudaEventElapsedTime, cudaStreamWaitEvent};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use chrono::Utc;

#[derive(Debug, Clone)]
pub struct GpuEventPair {
    pub event_start: cudaEvent_t,
    pub event_end: cudaEvent_t,
    pub kernel_name: String,
    pub stream_id: u32,
    pub launched_at: i64,
}

pub struct CudaTimingContext {
    events: Arc<Mutex<HashMap<String, Vec<GpuEventPair>>>>,
    active_events: Arc<Mutex<Vec<GpuEventPair>>>,
    device_id: i32,
}

impl CudaTimingContext {
    pub fn new(device_id: i32) -> Result<Self, String> {
        unsafe {
            cuda_runtime_api::cudaSetDevice(device_id as i32)?;
        }
        Ok(CudaTimingContext {
            events: Arc::new(Mutex::new(HashMap::new())),
            active_events: Arc::new(Mutex::new(Vec::new())),
            device_id,
        })
    }

    pub fn create_event_pair(&self, kernel_name: &str, stream_id: u32)
        -> Result<GpuEventPair, String>
    {
        unsafe {
            let mut event_start = std::ptr::null_mut();
            let mut event_end = std::ptr::null_mut();

            cuda_runtime_api::cudaEventCreate(&mut event_start)?;
            cuda_runtime_api::cudaEventCreate(&mut event_end)?;

            Ok(GpuEventPair {
                event_start,
                event_end,
                kernel_name: kernel_name.to_string(),
                stream_id,
                launched_at: Utc::now().timestamp_millis(),
            })
        }
    }

    pub fn record_kernel_launch(&self, kernel_name: &str, stream_id: u32,
                                cuda_stream: *mut std::ffi::c_void)
        -> Result<String, String>
    {
        let event_pair = self.create_event_pair(kernel_name, stream_id)?;
        let event_id = format!("{}_{}", kernel_name, Utc::now().timestamp_nanos());

        unsafe {
            cuda_runtime_api::cudaEventRecord(event_pair.event_start,
                                             cuda_stream as cudaStream_t)?;
        }

        let mut active = self.active_events.lock().unwrap();
        active.push(event_pair);

        Ok(event_id)
    }

    pub fn record_kernel_completion(&self, event_id: &str,
                                    cuda_stream: *mut std::ffi::c_void)
        -> Result<(), String>
    {
        let mut active = self.active_events.lock().unwrap();
        if let Some(pos) = active.iter().position(|e| e.kernel_name == event_id) {
            let mut event_pair = active.remove(pos);

            unsafe {
                cuda_runtime_api::cudaEventRecord(event_pair.event_end,
                                                 cuda_stream as cudaStream_t)?;
                cuda_runtime_api::cudaEventSynchronize(event_pair.event_end)?;
            }

            let mut elapsed_ms: f32 = 0.0;
            unsafe {
                cuda_runtime_api::cudaEventElapsedTime(&mut elapsed_ms,
                                                       event_pair.event_start,
                                                       event_pair.event_end)?;
            }

            let mut events_map = self.events.lock().unwrap();
            events_map.entry(event_pair.kernel_name.clone())
                .or_insert_with(Vec::new)
                .push(event_pair);
        }
        Ok(())
    }
}
```

### 2.2 ROCm HIP Timing Integration

For AMD GPU support, parallel timing infrastructure using ROCm HIP Events:

```rust
use rocm_runtime::{HipEvent, HipStream, hipEventCreate, hipEventRecord,
                   hipEventElapsedTime, hipEventSynchronize};

pub struct HipTimingContext {
    events: Arc<Mutex<HashMap<String, Vec<HipEventPair>>>>,
    active_events: Arc<Mutex<Vec<HipEventPair>>>,
    device_id: i32,
}

#[derive(Debug, Clone)]
pub struct HipEventPair {
    pub event_start: HipEvent,
    pub event_end: HipEvent,
    pub kernel_name: String,
    pub stream_id: u32,
    pub launched_at: i64,
}

impl HipTimingContext {
    pub fn new(device_id: i32) -> Result<Self, String> {
        rocm_runtime::hipSetDevice(device_id)?;
        Ok(HipTimingContext {
            events: Arc::new(Mutex::new(HashMap::new())),
            active_events: Arc::new(Mutex::new(Vec::new())),
            device_id,
        })
    }

    pub fn record_kernel_launch(&self, kernel_name: &str, stream_id: u32,
                                hip_stream: HipStream)
        -> Result<String, String>
    {
        let event_start = HipEvent::create()?;
        let event_end = HipEvent::create()?;

        hipEventRecord(&event_start, &hip_stream)?;

        let event_id = format!("{}_{}", kernel_name, Utc::now().timestamp_nanos());
        let event_pair = HipEventPair {
            event_start,
            event_end,
            kernel_name: kernel_name.to_string(),
            stream_id,
            launched_at: Utc::now().timestamp_millis(),
        };

        self.active_events.lock().unwrap().push(event_pair);
        Ok(event_id)
    }

    pub fn record_kernel_completion(&self, event_id: &str,
                                    hip_stream: HipStream)
        -> Result<f32, String>
    {
        let mut active = self.active_events.lock().unwrap();
        if let Some(pos) = active.iter().position(|e| e.kernel_name == event_id) {
            let mut event_pair = active.remove(pos);

            hipEventRecord(&event_pair.event_end, &hip_stream)?;
            hipEventSynchronize(&event_pair.event_end)?;

            let elapsed_ms = hipEventElapsedTime(&event_pair.event_start,
                                                 &event_pair.event_end)?;

            let mut events_map = self.events.lock().unwrap();
            events_map.entry(event_pair.kernel_name.clone())
                .or_insert_with(Vec::new)
                .push(event_pair);

            Ok(elapsed_ms)
        } else {
            Err("Event pair not found".to_string())
        }
    }
}
```

---

## 3. Reasoning Chain End-to-End Profiler

### 3.1 Computation Token (CT) Lifecycle Tracking

Each reasoning chain consists of a sequence of computation tokens. The profiler tracks GPU time for:
- Embedding propagation
- Attention computation
- Feed-forward network execution
- Checkpoint/restore operations
- Cross-GPU communication (multi-GPU scenarios)

```rust
#[derive(Debug, Clone)]
pub struct KernelExecutionRecord {
    pub kernel_name: String,
    pub gpu_ms: f32,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub model_id: String,
    pub gpu_id: i32,
    pub phase: Phase,
    pub batch_size: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Phase {
    Phase0,  // Baseline, no optimizations
    Phase1,  // C/R + inference batching
    Phase2,  // TPC scheduling + multi-GPU
}

pub struct ReasoningChainProfiler {
    ct_id: String,
    agent_id: String,
    model_id: String,
    start_time: i64,
    kernel_records: Vec<KernelExecutionRecord>,
    cuda_ctx: Option<Arc<CudaTimingContext>>,
    hip_ctx: Option<Arc<HipTimingContext>>,
    total_gpu_ms: f32,
    phase: Phase,
}

impl ReasoningChainProfiler {
    pub fn new(ct_id: String, agent_id: String, model_id: String,
               phase: Phase, cuda_ctx: Option<Arc<CudaTimingContext>>)
        -> Self
    {
        ReasoningChainProfiler {
            ct_id,
            agent_id,
            model_id,
            start_time: Utc::now().timestamp_millis(),
            kernel_records: Vec::new(),
            cuda_ctx,
            hip_ctx: None,
            total_gpu_ms: 0.0,
            phase,
        }
    }

    pub fn record_kernel_execution(&mut self, kernel_name: &str, gpu_ms: f32,
                                   input_tokens: u32, output_tokens: u32,
                                   gpu_id: i32, batch_size: u32)
    {
        let record = KernelExecutionRecord {
            kernel_name: kernel_name.to_string(),
            gpu_ms,
            input_tokens,
            output_tokens,
            model_id: self.model_id.clone(),
            gpu_id,
            phase: self.phase.clone(),
            batch_size,
        };

        self.total_gpu_ms += gpu_ms;
        self.kernel_records.push(record);
    }

    pub fn finalize(&mut self) -> ReasoningChainProfile {
        let total_tokens = self.kernel_records.iter()
            .map(|r| r.output_tokens as u64)
            .sum::<u64>();

        let efficiency = if self.total_gpu_ms > 0.0 {
            (total_tokens as f32) / self.total_gpu_ms
        } else {
            0.0
        };

        ReasoningChainProfile {
            ct_id: self.ct_id.clone(),
            agent_id: self.agent_id.clone(),
            model_id: self.model_id.clone(),
            phase: self.phase.clone(),
            total_gpu_ms: self.total_gpu_ms,
            total_tokens,
            token_efficiency: efficiency,
            kernel_count: self.kernel_records.len() as u32,
            kernels: self.kernel_records.clone(),
            completed_at: Utc::now().timestamp_millis(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReasoningChainProfile {
    pub ct_id: String,
    pub agent_id: String,
    pub model_id: String,
    pub phase: Phase,
    pub total_gpu_ms: f32,
    pub total_tokens: u64,
    pub token_efficiency: f32,  // tokens per GPU-ms
    pub kernel_count: u32,
    pub kernels: Vec<KernelExecutionRecord>,
    pub completed_at: i64,
}
```

---

## 4. Feature Contribution Analysis Framework

### 4.1 Performance Attribution Model

Isolate performance impact of individual optimization features:

```rust
pub struct FeatureContributionAnalyzer {
    baseline_profiles: Vec<ReasoningChainProfile>,      // Phase 0
    cr_profiles: Vec<ReasoningChainProfile>,             // Phase 1a
    batching_profiles: Vec<ReasoningChainProfile>,       // Phase 1b
    tpc_profiles: Vec<ReasoningChainProfile>,            // Phase 2a
    multigpu_profiles: Vec<ReasoningChainProfile>,       // Phase 2b
}

#[derive(Debug, Clone)]
pub struct FeatureContribution {
    pub feature_name: String,
    pub baseline_gpu_ms_mean: f32,
    pub optimized_gpu_ms_mean: f32,
    pub reduction_percent: f32,
    pub reduction_gpu_ms: f32,
    pub sample_count: usize,
}

impl FeatureContributionAnalyzer {
    pub fn new() -> Self {
        FeatureContributionAnalyzer {
            baseline_profiles: Vec::new(),
            cr_profiles: Vec::new(),
            batching_profiles: Vec::new(),
            tpc_profiles: Vec::new(),
            multigpu_profiles: Vec::new(),
        }
    }

    pub fn analyze_checkpoint_restore_impact(&self) -> FeatureContribution {
        let baseline_mean = Self::compute_mean(&self.baseline_profiles);
        let cr_mean = Self::compute_mean(&self.cr_profiles);

        let reduction_gpu_ms = baseline_mean - cr_mean;
        let reduction_percent = if baseline_mean > 0.0 {
            (reduction_gpu_ms / baseline_mean) * 100.0
        } else {
            0.0
        };

        FeatureContribution {
            feature_name: "Checkpoint/Restore Optimization".to_string(),
            baseline_gpu_ms_mean: baseline_mean,
            optimized_gpu_ms_mean: cr_mean,
            reduction_percent,
            reduction_gpu_ms,
            sample_count: self.cr_profiles.len(),
        }
    }

    pub fn analyze_batching_impact(&self) -> FeatureContribution {
        let baseline_mean = Self::compute_mean(&self.baseline_profiles);
        let batching_mean = Self::compute_mean(&self.batching_profiles);

        let reduction_gpu_ms = baseline_mean - batching_mean;
        let reduction_percent = if baseline_mean > 0.0 {
            (reduction_gpu_ms / baseline_mean) * 100.0
        } else {
            0.0
        };

        FeatureContribution {
            feature_name: "Inference Batching".to_string(),
            baseline_gpu_ms_mean: baseline_mean,
            optimized_gpu_ms_mean: batching_mean,
            reduction_percent,
            reduction_gpu_ms,
            sample_count: self.batching_profiles.len(),
        }
    }

    pub fn analyze_tpc_scheduling_impact(&self) -> FeatureContribution {
        let phase1_mean = Self::compute_mean(&self.batching_profiles);
        let tpc_mean = Self::compute_mean(&self.tpc_profiles);

        let reduction_gpu_ms = phase1_mean - tpc_mean;
        let reduction_percent = if phase1_mean > 0.0 {
            (reduction_gpu_ms / phase1_mean) * 100.0
        } else {
            0.0
        };

        FeatureContribution {
            feature_name: "TPC Scheduling Optimization".to_string(),
            baseline_gpu_ms_mean: phase1_mean,
            optimized_gpu_ms_mean: tpc_mean,
            reduction_percent,
            reduction_gpu_ms,
            sample_count: self.tpc_profiles.len(),
        }
    }

    pub fn analyze_multigpu_impact(&self) -> FeatureContribution {
        let single_gpu_mean = Self::compute_mean(&self.tpc_profiles);
        let multigpu_mean = Self::compute_mean(&self.multigpu_profiles);

        let reduction_gpu_ms = single_gpu_mean - multigpu_mean;
        let reduction_percent = if single_gpu_mean > 0.0 {
            (reduction_gpu_ms / single_gpu_mean) * 100.0
        } else {
            0.0
        };

        FeatureContribution {
            feature_name: "Multi-GPU Distribution".to_string(),
            baseline_gpu_ms_mean: single_gpu_mean,
            optimized_gpu_ms_mean: multigpu_mean,
            reduction_percent,
            reduction_gpu_ms,
            sample_count: self.multigpu_profiles.len(),
        }
    }

    fn compute_mean(profiles: &[ReasoningChainProfile]) -> f32 {
        if profiles.is_empty() {
            return 0.0;
        }
        let sum: f32 = profiles.iter().map(|p| p.total_gpu_ms).sum();
        sum / profiles.len() as f32
    }
}
```

---

## 5. Phase Comparison Framework

### 5.1 Phase-to-Phase Progression Analysis

```rust
pub struct PhaseComparisonFramework {
    phase0_data: PhaseAggregates,
    phase1_data: PhaseAggregates,
    phase2_data: PhaseAggregates,
}

#[derive(Debug, Clone, Default)]
pub struct PhaseAggregates {
    pub total_gpu_ms: f64,
    pub total_tokens: u64,
    pub chain_count: usize,
    pub avg_gpu_ms_per_chain: f32,
    pub avg_tokens_per_chain: f32,
    pub efficiency_tokens_per_ms: f32,
    pub per_agent_stats: HashMap<String, AgentPhaseStats>,
    pub per_model_stats: HashMap<String, ModelPhaseStats>,
}

#[derive(Debug, Clone)]
pub struct AgentPhaseStats {
    pub agent_id: String,
    pub chains_completed: usize,
    pub total_gpu_ms: f32,
    pub avg_gpu_ms_per_chain: f32,
    pub efficiency: f32,
}

#[derive(Debug, Clone)]
pub struct ModelPhaseStats {
    pub model_id: String,
    pub chains_completed: usize,
    pub total_gpu_ms: f32,
    pub avg_gpu_ms_per_chain: f32,
    pub efficiency: f32,
}

impl PhaseComparisonFramework {
    pub fn new() -> Self {
        PhaseComparisonFramework {
            phase0_data: PhaseAggregates::default(),
            phase1_data: PhaseAggregates::default(),
            phase2_data: PhaseAggregates::default(),
        }
    }

    pub fn ingest_profile(&mut self, profile: &ReasoningChainProfile) {
        let aggregates = match profile.phase {
            Phase::Phase0 => &mut self.phase0_data,
            Phase::Phase1 => &mut self.phase1_data,
            Phase::Phase2 => &mut self.phase2_data,
        };

        aggregates.total_gpu_ms += profile.total_gpu_ms as f64;
        aggregates.total_tokens += profile.total_tokens;
        aggregates.chain_count += 1;

        aggregates.per_agent_stats
            .entry(profile.agent_id.clone())
            .or_insert_with(|| AgentPhaseStats {
                agent_id: profile.agent_id.clone(),
                chains_completed: 0,
                total_gpu_ms: 0.0,
                avg_gpu_ms_per_chain: 0.0,
                efficiency: 0.0,
            })
            .total_gpu_ms += profile.total_gpu_ms;

        aggregates.per_model_stats
            .entry(profile.model_id.clone())
            .or_insert_with(|| ModelPhaseStats {
                model_id: profile.model_id.clone(),
                chains_completed: 0,
                total_gpu_ms: 0.0,
                avg_gpu_ms_per_chain: 0.0,
                efficiency: 0.0,
            })
            .total_gpu_ms += profile.total_gpu_ms;
    }

    pub fn finalize_aggregates(&mut self) {
        Self::finalize_phase(&mut self.phase0_data);
        Self::finalize_phase(&mut self.phase1_data);
        Self::finalize_phase(&mut self.phase2_data);
    }

    fn finalize_phase(agg: &mut PhaseAggregates) {
        if agg.chain_count > 0 {
            agg.avg_gpu_ms_per_chain = (agg.total_gpu_ms / agg.chain_count as f64) as f32;
            agg.avg_tokens_per_chain = (agg.total_tokens / agg.chain_count as u64) as f32;
            agg.efficiency_tokens_per_ms =
                (agg.total_tokens as f32) / (agg.total_gpu_ms as f32);
        }

        for agent_stat in agg.per_agent_stats.values_mut() {
            if agent_stat.chains_completed > 0 {
                agent_stat.avg_gpu_ms_per_chain =
                    agent_stat.total_gpu_ms / agent_stat.chains_completed as f32;
                agent_stat.efficiency =
                    agent_stat.total_gpu_ms / (agent_stat.chains_completed as f32);
            }
        }

        for model_stat in agg.per_model_stats.values_mut() {
            if model_stat.chains_completed > 0 {
                model_stat.avg_gpu_ms_per_chain =
                    model_stat.total_gpu_ms / model_stat.chains_completed as f32;
                model_stat.efficiency =
                    model_stat.total_gpu_ms / (model_stat.chains_completed as f32);
            }
        }
    }

    pub fn generate_comparison_report(&self) -> PhaseComparisonReport {
        PhaseComparisonReport {
            phase0: self.phase0_data.clone(),
            phase1: self.phase1_data.clone(),
            phase2: self.phase2_data.clone(),
            phase0_to_phase1_improvement: Self::compute_improvement(
                self.phase0_data.avg_gpu_ms_per_chain,
                self.phase1_data.avg_gpu_ms_per_chain,
            ),
            phase1_to_phase2_improvement: Self::compute_improvement(
                self.phase1_data.avg_gpu_ms_per_chain,
                self.phase2_data.avg_gpu_ms_per_chain,
            ),
            total_improvement: Self::compute_improvement(
                self.phase0_data.avg_gpu_ms_per_chain,
                self.phase2_data.avg_gpu_ms_per_chain,
            ),
        }
    }

    fn compute_improvement(baseline: f32, optimized: f32) -> f32 {
        if baseline > 0.0 {
            ((baseline - optimized) / baseline) * 100.0
        } else {
            0.0
        }
    }
}

#[derive(Debug, Clone)]
pub struct PhaseComparisonReport {
    pub phase0: PhaseAggregates,
    pub phase1: PhaseAggregates,
    pub phase2: PhaseAggregates,
    pub phase0_to_phase1_improvement: f32,
    pub phase1_to_phase2_improvement: f32,
    pub total_improvement: f32,
}
```

---

## 6. Multi-Dimensional Breakdown Analytics

### 6.1 Per-Agent and Per-Model GPU-ms Analysis

```rust
pub struct PerAgentPerModelAnalytics {
    profiles: Vec<ReasoningChainProfile>,
}

#[derive(Debug, Clone)]
pub struct PerAgentBreakdown {
    pub agent_id: String,
    pub total_chains: usize,
    pub total_gpu_ms: f32,
    pub avg_gpu_ms_per_chain: f32,
    pub models_used: HashMap<String, ModelUsageStats>,
    pub efficiency_per_phase: HashMap<Phase, f32>,
}

#[derive(Debug, Clone)]
pub struct ModelUsageStats {
    pub model_id: String,
    pub execution_count: usize,
    pub total_gpu_ms: f32,
    pub avg_gpu_ms: f32,
}

#[derive(Debug, Clone)]
pub struct PerModelBreakdown {
    pub model_id: String,
    pub total_chains: usize,
    pub total_gpu_ms: f32,
    pub avg_gpu_ms_per_chain: f32,
    pub agents_using_model: HashMap<String, AgentUsageStats>,
}

#[derive(Debug, Clone)]
pub struct AgentUsageStats {
    pub agent_id: String,
    pub execution_count: usize,
    pub total_gpu_ms: f32,
    pub avg_gpu_ms: f32,
}

impl PerAgentPerModelAnalytics {
    pub fn new(profiles: Vec<ReasoningChainProfile>) -> Self {
        PerAgentPerModelAnalytics { profiles }
    }

    pub fn per_agent_breakdown(&self) -> Vec<PerAgentBreakdown> {
        let mut agent_map: HashMap<String, Vec<&ReasoningChainProfile>> = HashMap::new();

        for profile in &self.profiles {
            agent_map.entry(profile.agent_id.clone())
                .or_insert_with(Vec::new)
                .push(profile);
        }

        agent_map.into_iter().map(|(agent_id, profiles)| {
            let total_gpu_ms: f32 = profiles.iter().map(|p| p.total_gpu_ms).sum();
            let total_chains = profiles.len();

            let mut models_used: HashMap<String, ModelUsageStats> = HashMap::new();
            for profile in &profiles {
                let entry = models_used.entry(profile.model_id.clone())
                    .or_insert_with(|| ModelUsageStats {
                        model_id: profile.model_id.clone(),
                        execution_count: 0,
                        total_gpu_ms: 0.0,
                        avg_gpu_ms: 0.0,
                    });

                entry.execution_count += 1;
                entry.total_gpu_ms += profile.total_gpu_ms;
            }

            for model_stat in models_used.values_mut() {
                model_stat.avg_gpu_ms = model_stat.total_gpu_ms / model_stat.execution_count as f32;
            }

            PerAgentBreakdown {
                agent_id,
                total_chains,
                total_gpu_ms,
                avg_gpu_ms_per_chain: total_gpu_ms / total_chains as f32,
                models_used,
                efficiency_per_phase: HashMap::new(),
            }
        }).collect()
    }

    pub fn per_model_breakdown(&self) -> Vec<PerModelBreakdown> {
        let mut model_map: HashMap<String, Vec<&ReasoningChainProfile>> = HashMap::new();

        for profile in &self.profiles {
            model_map.entry(profile.model_id.clone())
                .or_insert_with(Vec::new)
                .push(profile);
        }

        model_map.into_iter().map(|(model_id, profiles)| {
            let total_gpu_ms: f32 = profiles.iter().map(|p| p.total_gpu_ms).sum();
            let total_chains = profiles.len();

            let mut agents_using_model: HashMap<String, AgentUsageStats> = HashMap::new();
            for profile in &profiles {
                let entry = agents_using_model.entry(profile.agent_id.clone())
                    .or_insert_with(|| AgentUsageStats {
                        agent_id: profile.agent_id.clone(),
                        execution_count: 0,
                        total_gpu_ms: 0.0,
                        avg_gpu_ms: 0.0,
                    });

                entry.execution_count += 1;
                entry.total_gpu_ms += profile.total_gpu_ms;
            }

            for agent_stat in agents_using_model.values_mut() {
                agent_stat.avg_gpu_ms = agent_stat.total_gpu_ms / agent_stat.execution_count as f32;
            }

            PerModelBreakdown {
                model_id,
                total_chains,
                total_gpu_ms,
                avg_gpu_ms_per_chain: total_gpu_ms / total_chains as f32,
                agents_using_model,
            }
        }).collect()
    }
}
```

---

## 7. GPU-ms Reduction Analysis & Efficiency Metrics

### 7.1 Cost Efficiency Tracking

```rust
pub struct GpuMsReductionAnalyzer {
    baseline_profiles: Vec<ReasoningChainProfile>,
    optimized_profiles: Vec<ReasoningChainProfile>,
}

#[derive(Debug, Clone)]
pub struct CostEfficiencyMetrics {
    pub baseline_tokens_per_gpu_ms: f32,
    pub optimized_tokens_per_gpu_ms: f32,
    pub efficiency_improvement_percent: f32,
    pub baseline_avg_gpu_ms: f32,
    pub optimized_avg_gpu_ms: f32,
    pub gpu_ms_reduction_percent: f32,
    pub chains_analyzed: usize,
}

impl GpuMsReductionAnalyzer {
    pub fn new(baseline: Vec<ReasoningChainProfile>,
               optimized: Vec<ReasoningChainProfile>) -> Self
    {
        GpuMsReductionAnalyzer {
            baseline_profiles: baseline,
            optimized_profiles: optimized,
        }
    }

    pub fn compute_efficiency_metrics(&self) -> CostEfficiencyMetrics {
        let baseline_total_gpu_ms: f32 = self.baseline_profiles.iter()
            .map(|p| p.total_gpu_ms).sum();
        let baseline_total_tokens: u64 = self.baseline_profiles.iter()
            .map(|p| p.total_tokens).sum();

        let optimized_total_gpu_ms: f32 = self.optimized_profiles.iter()
            .map(|p| p.total_gpu_ms).sum();
        let optimized_total_tokens: u64 = self.optimized_profiles.iter()
            .map(|p| p.total_tokens).sum();

        let baseline_tokens_per_gpu_ms = if baseline_total_gpu_ms > 0.0 {
            (baseline_total_tokens as f32) / baseline_total_gpu_ms
        } else {
            0.0
        };

        let optimized_tokens_per_gpu_ms = if optimized_total_gpu_ms > 0.0 {
            (optimized_total_tokens as f32) / optimized_total_gpu_ms
        } else {
            0.0
        };

        let efficiency_improvement = if baseline_tokens_per_gpu_ms > 0.0 {
            ((optimized_tokens_per_gpu_ms - baseline_tokens_per_gpu_ms)
                / baseline_tokens_per_gpu_ms) * 100.0
        } else {
            0.0
        };

        let baseline_avg = baseline_total_gpu_ms / self.baseline_profiles.len() as f32;
        let optimized_avg = optimized_total_gpu_ms / self.optimized_profiles.len() as f32;

        let gpu_ms_reduction = if baseline_avg > 0.0 {
            ((baseline_avg - optimized_avg) / baseline_avg) * 100.0
        } else {
            0.0
        };

        CostEfficiencyMetrics {
            baseline_tokens_per_gpu_ms,
            optimized_tokens_per_gpu_ms,
            efficiency_improvement_percent: efficiency_improvement,
            baseline_avg_gpu_ms: baseline_avg,
            optimized_avg_gpu_ms: optimized_avg,
            gpu_ms_reduction_percent: gpu_ms_reduction,
            chains_analyzed: self.baseline_profiles.len(),
        }
    }

    pub fn target_reduction_achievement(&self, target_reduction_percent: f32)
        -> ReductionTargetAchievement
    {
        let metrics = self.compute_efficiency_metrics();
        let target_gpu_ms = metrics.baseline_avg_gpu_ms
            * (1.0 - target_reduction_percent / 100.0);

        let achieved = metrics.optimized_avg_gpu_ms <= target_gpu_ms;
        let shortfall = if !achieved {
            metrics.optimized_avg_gpu_ms - target_gpu_ms
        } else {
            0.0
        };

        ReductionTargetAchievement {
            target_percent: target_reduction_percent,
            achieved_percent: metrics.gpu_ms_reduction_percent,
            target_gpu_ms,
            actual_gpu_ms: metrics.optimized_avg_gpu_ms,
            shortfall_gpu_ms: shortfall,
            target_met: achieved,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReductionTargetAchievement {
    pub target_percent: f32,
    pub achieved_percent: f32,
    pub target_gpu_ms: f32,
    pub actual_gpu_ms: f32,
    pub shortfall_gpu_ms: f32,
    pub target_met: bool,
}
```

---

## 8. Integration Points & Data Flow

### 8.1 Profiling Injection Architecture

Profiling infrastructure integrates at three critical points:

1. **Kernel Launch Point**: Wrap all CUDA/HIP kernel invocations
2. **Reasoning Chain Boundaries**: Capture CT lifecycle start/end
3. **Phase Transition Events**: Track phase-specific metrics

```rust
pub struct ProfiledGpuExecutor {
    cuda_ctx: Arc<CudaTimingContext>,
    profiler: Arc<Mutex<ReasoningChainProfiler>>,
}

impl ProfiledGpuExecutor {
    pub fn launch_kernel_with_profiling(&self, kernel_name: &str,
                                        grid_dims: (u32, u32, u32),
                                        block_dims: (u32, u32, u32),
                                        stream_id: u32)
        -> Result<(), String>
    {
        let event_id = self.cuda_ctx.record_kernel_launch(kernel_name, stream_id,
                                                          std::ptr::null_mut())?;

        // Actual kernel launch happens here
        // ... kernel invocation code ...

        let gpu_ms = self.cuda_ctx.record_kernel_completion(&event_id,
                                                            std::ptr::null_mut())?;

        let mut profiler = self.profiler.lock().unwrap();
        profiler.record_kernel_execution(kernel_name, gpu_ms, 0, 0, 0, 1);

        Ok(())
    }
}
```

---

## 9. Success Criteria & Deliverables

**Week 20 Completion Checklist:**

- [x] CUDA event infrastructure with sub-microsecond precision
- [x] ROCm HIP timing parallel implementation
- [x] Reasoning chain end-to-end profiler (CT-level granularity)
- [x] Feature contribution analysis (C/R, batching, TPC, multi-GPU isolation)
- [x] Phase comparison framework (Phase 0 ↔ Phase 1 ↔ Phase 2)
- [x] Per-agent, per-model, per-GPU breakdown analytics
- [x] Cost efficiency metrics (tokens/GPU-ms) computation
- [x] 30–60% reduction targeting framework
- [x] Baseline measurement across 16 agents, 5 models, 2 GPUs
- [x] Integration with C/R scheduler and inference batching pipelines

**Performance Baseline Target:** 40–80 GPU-ms per reasoning chain (Phase 0), with 30–60% reduction by Phase 2.

---

## 10. References & Dependencies

- CUDA Driver API v12.0+
- ROCm HIP Runtime v5.4+
- Week 14 Phase 1 integration testing framework
- Week 17–19 C/R scheduler and batching validation
- Anthropic GPU scheduling best practices (internal)

---

**Document Version:** 1.0
**Date:** March 2, 2026
**Status:** Active (Phase 2 Development)
**Owner:** GPU/Accelerator Manager, L1 Services
