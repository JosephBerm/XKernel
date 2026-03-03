// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! End-to-end data flow types for inference requests.
//!
//! Defines structures for tracking inference requests from submission through
//! completion, including batching configuration and per-request lifecycle
//! (queued -> allocated -> launched -> executing -> complete).
//!
//! Reference: Engineering Plan § Data Flow, Request Tracking

use core::fmt;

/// End-to-end inference request descriptor.
///
/// Represents a single inference request entering the GPU pipeline.
/// Includes model ID, input data size, deadline, and priority.
///
/// Reference: Engineering Plan § Request Submission
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InferenceRequest {
    /// Cognitive Tensor (crew) ID that issued this request.
    pub ct_id: [u8; 16],

    /// Model identifier (e.g., LLaMA 70B variant).
    pub model_id: [u8; 16],

    /// Input token count.
    pub input_tokens: u32,

    /// Maximum output tokens (hard limit for generation).
    pub max_output_tokens: u32,

    /// Request priority (0=lowest, 255=highest).
    ///
    /// Used for scheduling across competing requests.
    pub priority: u8,

    /// Deadline in milliseconds from submission.
    ///
    /// GPU Manager attempts to complete request within this window.
    /// 0 = no deadline (best-effort).
    pub deadline_ms: u32,

    /// Timestamp in nanoseconds since boot (when request submitted).
    pub timestamp_ns: u64,
}

impl fmt::Display for InferenceRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "InferenceRequest(ct={:?}, model={:?}, input={}, max_output={}, priority={}, deadline={}ms)",
            &self.ct_id[..4], &self.model_id[..4], self.input_tokens, self.max_output_tokens, self.priority, self.deadline_ms
        )
    }
}

/// Result from a completed inference request.
///
/// Returned once request processing is complete, with metrics on
/// performance and resource utilization.
///
/// Reference: Engineering Plan § Result Completion
#[derive(Clone, Copy, Debug)]
pub struct InferenceResult {
    /// Output token count produced by model.
    pub output_tokens: u32,

    /// GPU execution time in milliseconds.
    pub gpu_ms: u32,

    /// Tokens per second throughput (output_tokens / gpu_ms * 1000).
    pub tokens_per_second: u32,

    /// TPC utilization percentage during execution (0-100).
    pub tpc_utilization: u8,

    /// Actual latency in milliseconds from submission to completion.
    pub total_latency_ms: u32,
}

impl fmt::Display for InferenceResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "InferenceResult(output={}, gpu_time={}ms, tps={}, util={}%, latency={}ms)",
            self.output_tokens, self.gpu_ms, self.tokens_per_second, self.tpc_utilization, self.total_latency_ms
        )
    }
}

/// Lifecycle stage of a request in the GPU pipeline.
///
/// Tracks request progression through each phase of GPU processing.
///
/// Reference: Engineering Plan § Request Lifecycle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DataFlowStage {
    /// Queued: request waiting for TPC allocation.
    Queued,

    /// TpcAllocated: TPCs allocated, waiting for kernel launch.
    TpcAllocated,

    /// KernelLaunched: kernel submitted to GPU, awaiting execution.
    KernelLaunched,

    /// Executing: kernels actively running on GPU.
    Executing,

    /// ResultReady: execution complete, result available for retrieval.
    ResultReady,

    /// Completed: result consumed, request fully processed.
    Completed,
}

impl fmt::Display for DataFlowStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataFlowStage::Queued => write!(f, "Queued"),
            DataFlowStage::TpcAllocated => write!(f, "TpcAllocated"),
            DataFlowStage::KernelLaunched => write!(f, "KernelLaunched"),
            DataFlowStage::Executing => write!(f, "Executing"),
            DataFlowStage::ResultReady => write!(f, "ResultReady"),
            DataFlowStage::Completed => write!(f, "Completed"),
        }
    }
}

/// Data flow trace entry for a single stage transition.
///
/// Records when a request transitions between pipeline stages,
/// enabling end-to-end latency breakdown.
///
/// Reference: Engineering Plan § Request Tracing
#[derive(Clone, Copy, Debug)]
pub struct DataFlowTraceEntry {
    /// Stage entered
    pub stage: DataFlowStage,

    /// Timestamp in nanoseconds since boot
    pub timestamp_ns: u64,

    /// Duration in this stage (ns), if transitioning out
    pub duration_ns: u64,
}

impl fmt::Display for DataFlowTraceEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Trace(stage={}, ts={}ns, duration={}ns)",
            self.stage, self.timestamp_ns, self.duration_ns
        )
    }
}

/// Complete request lifecycle trace.
///
/// Records the path of a single request through all pipeline stages.
/// Enables performance debugging and optimization.
///
/// Reference: Engineering Plan § End-to-End Tracing
#[derive(Clone, Debug)]
pub struct DataFlowTrace {
    /// Request identifier (derived from CT + request number)
    pub request_id: [u8; 16],

    /// Original request
    pub request: InferenceRequest,

    /// Stage transitions (in order)
    pub stages: [Option<DataFlowTraceEntry>; 6], // One entry per DataFlowStage

    /// Final result (if available)
    pub result: Option<InferenceResult>,
}

impl DataFlowTrace {
    /// Create a new trace for a request.
    pub fn new(request_id: [u8; 16], request: InferenceRequest) -> Self {
        DataFlowTrace {
            request_id,
            request,
            stages: [None; 6],
            result: None,
        }
    }

    /// Record a stage transition.
    ///
    /// # Arguments
    ///
    /// * `stage` - Stage being entered
    /// * `timestamp_ns` - Current timestamp
    pub fn record_stage(&mut self, stage: DataFlowStage, timestamp_ns: u64) {
        let stage_index = match stage {
            DataFlowStage::Queued => 0,
            DataFlowStage::TpcAllocated => 1,
            DataFlowStage::KernelLaunched => 2,
            DataFlowStage::Executing => 3,
            DataFlowStage::ResultReady => 4,
            DataFlowStage::Completed => 5,
        };

        // Calculate duration from previous stage
        let duration_ns = if stage_index > 0 {
            if let Some(prev_entry) = self.stages[stage_index - 1] {
                timestamp_ns.saturating_sub(prev_entry.timestamp_ns)
            } else {
                0
            }
        } else {
            0
        };

        self.stages[stage_index] = Some(DataFlowTraceEntry {
            stage,
            timestamp_ns,
            duration_ns,
        });
    }

    /// Get the current stage.
    pub fn current_stage(&self) -> Option<DataFlowStage> {
        // Find the last non-None entry
        for i in (0..6).rev() {
            if let Some(entry) = self.stages[i] {
                return Some(entry.stage);
            }
        }
        None
    }

    /// Get total end-to-end latency in nanoseconds.
    pub fn total_latency_ns(&self) -> u64 {
        if let (Some(first), Some(last)) = (self.stages[0], self.stages[5]) {
            last.timestamp_ns.saturating_sub(first.timestamp_ns)
        } else {
            0
        }
    }

    /// Get latency for a specific stage in nanoseconds.
    pub fn stage_latency_ns(&self, stage: DataFlowStage) -> u64 {
        let stage_index = match stage {
            DataFlowStage::Queued => 0,
            DataFlowStage::TpcAllocated => 1,
            DataFlowStage::KernelLaunched => 2,
            DataFlowStage::Executing => 3,
            DataFlowStage::ResultReady => 4,
            DataFlowStage::Completed => 5,
        };

        self.stages[stage_index].map(|e| e.duration_ns).unwrap_or(0)
    }
}

impl fmt::Display for DataFlowTrace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "DataFlowTrace(id={:?}, request={}, current={:?}, latency={}ns)",
            &self.request_id[..4],
            self.request,
            self.current_stage(),
            self.total_latency_ns()
        )
    }
}

/// Batching configuration for request coalescing.
///
/// Controls how the GPU Manager batches requests together for efficiency.
/// Smaller batches = lower latency, larger batches = higher throughput.
///
/// Reference: Engineering Plan § Batching Strategy
#[derive(Clone, Copy, Debug)]
pub struct BatchConfig {
    /// Maximum batch size (requests per GPU kernel call).
    ///
    /// Larger batches increase GPU utilization but increase per-request latency.
    pub max_batch_size: u32,

    /// Enable dynamic batching (coalesce pending requests).
    ///
    /// If true, wait for additional requests to accumulate (up to max_batch_size).
    /// If false, launch kernels as soon as first request arrives (latency-optimized).
    pub dynamic_batching: bool,

    /// Dynamic batching timeout in microseconds.
    ///
    /// If waiting for batch to fill, timeout and launch partial batch.
    /// Prevents indefinite queueing delays.
    pub batching_timeout_us: u32,

    /// Padding strategy for variable-length sequences.
    pub padding_strategy: PaddingStrategy,
}

impl BatchConfig {
    /// Create default batching config (latency-optimized).
    ///
    /// Small batch size, no dynamic batching, immediate launch.
    pub fn latency_optimized() -> Self {
        BatchConfig {
            max_batch_size: 1,
            dynamic_batching: false,
            batching_timeout_us: 0,
            padding_strategy: PaddingStrategy::NoPadding,
        }
    }

    /// Create throughput-optimized batching config.
    ///
    /// Large batch size, dynamic batching enabled.
    pub fn throughput_optimized() -> Self {
        BatchConfig {
            max_batch_size: 256,
            dynamic_batching: true,
            batching_timeout_us: 5000,
            padding_strategy: PaddingStrategy::PadToMax,
        }
    }

    /// Create balanced batching config.
    pub fn balanced() -> Self {
        BatchConfig {
            max_batch_size: 32,
            dynamic_batching: true,
            batching_timeout_us: 500,
            padding_strategy: PaddingStrategy::PadToMax,
        }
    }
}

impl fmt::Display for BatchConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BatchConfig(max_size={}, dynamic={}, timeout={}us, padding={})",
            self.max_batch_size, self.dynamic_batching, self.batching_timeout_us, self.padding_strategy
        )
    }
}

/// Padding strategy for variable-length sequences in batches.
///
/// Different strategies trade off memory efficiency vs. computation efficiency.
///
/// Reference: Engineering Plan § Sequence Padding
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaddingStrategy {
    /// No padding: each sequence processed individually.
    ///
    /// Minimum memory overhead, no wasted computation.
    /// Suitable for single-request batches.
    NoPadding,

    /// Pad to maximum length in batch.
    ///
    /// All sequences padded to length of longest sequence in batch.
    /// Balances memory/computation for variable-length inputs.
    PadToMax,

    /// Pad to fixed length.
    ///
    /// All sequences padded to fixed length (e.g., 2048 tokens).
    /// Predictable memory usage, may waste computation on short sequences.
    PadToFixed(u32),

    /// Custom packing (advanced).
    ///
    /// Interleave multiple shorter sequences to minimize padding.
    /// Requires sequence-level preemption support.
    CustomPacking,
}

impl fmt::Display for PaddingStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PaddingStrategy::NoPadding => write!(f, "NoPadding"),
            PaddingStrategy::PadToMax => write!(f, "PadToMax"),
            PaddingStrategy::PadToFixed(len) => write!(f, "PadToFixed({})", len),
            PaddingStrategy::CustomPacking => write!(f, "CustomPacking"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;

    #[test]
    fn test_inference_request_creation() {
        let request = InferenceRequest {
            ct_id: [1u8; 16],
            model_id: [2u8; 16],
            input_tokens: 100,
            max_output_tokens: 256,
            priority: 128,
            deadline_ms: 1000,
            timestamp_ns: 1_000_000_000,
        };

        assert_eq!(request.input_tokens, 100);
        assert_eq!(request.max_output_tokens, 256);
    }

    #[test]
    fn test_inference_result_creation() {
        let result = InferenceResult {
            output_tokens: 200,
            gpu_ms: 500,
            tokens_per_second: 400,
            tpc_utilization: 85,
            total_latency_ms: 600,
        };

        let display_str = format!("{}", result);
        assert!(display_str.contains("200"));
        assert!(display_str.contains("500ms"));
    }

    #[test]
    fn test_data_flow_stage_display() {
        assert_eq!(format!("{}", DataFlowStage::Queued), "Queued");
        assert_eq!(format!("{}", DataFlowStage::Executing), "Executing");
        assert_eq!(format!("{}", DataFlowStage::Completed), "Completed");
    }

    #[test]
    fn test_data_flow_trace_creation() {
        let request = InferenceRequest {
            ct_id: [1u8; 16],
            model_id: [2u8; 16],
            input_tokens: 100,
            max_output_tokens: 256,
            priority: 128,
            deadline_ms: 1000,
            timestamp_ns: 1_000_000_000,
        };

        let trace = DataFlowTrace::new([3u8; 16], request);
        assert_eq!(trace.current_stage(), None); // No stages recorded yet
    }

    #[test]
    fn test_data_flow_trace_record_stage() {
        let request = InferenceRequest {
            ct_id: [1u8; 16],
            model_id: [2u8; 16],
            input_tokens: 100,
            max_output_tokens: 256,
            priority: 128,
            deadline_ms: 1000,
            timestamp_ns: 1_000_000_000,
        };

        let mut trace = DataFlowTrace::new([3u8; 16], request);

        trace.record_stage(DataFlowStage::Queued, 1_000_000_000);
        assert_eq!(trace.current_stage(), Some(DataFlowStage::Queued));

        trace.record_stage(DataFlowStage::TpcAllocated, 1_000_001_000);
        assert_eq!(trace.current_stage(), Some(DataFlowStage::TpcAllocated));
    }

    #[test]
    fn test_data_flow_trace_latency_calculation() {
        let request = InferenceRequest {
            ct_id: [1u8; 16],
            model_id: [2u8; 16],
            input_tokens: 100,
            max_output_tokens: 256,
            priority: 128,
            deadline_ms: 1000,
            timestamp_ns: 1_000_000_000,
        };

        let mut trace = DataFlowTrace::new([3u8; 16], request);

        trace.record_stage(DataFlowStage::Queued, 1_000_000_000);
        trace.record_stage(DataFlowStage::Executing, 1_000_100_000);

        assert_eq!(trace.stage_latency_ns(DataFlowStage::Executing), 100_000);
    }

    #[test]
    fn test_data_flow_trace_full_pipeline() {
        let request = InferenceRequest {
            ct_id: [1u8; 16],
            model_id: [2u8; 16],
            input_tokens: 100,
            max_output_tokens: 256,
            priority: 128,
            deadline_ms: 1000,
            timestamp_ns: 1_000_000_000,
        };

        let mut trace = DataFlowTrace::new([3u8; 16], request);

        trace.record_stage(DataFlowStage::Queued, 1_000_000_000);
        trace.record_stage(DataFlowStage::TpcAllocated, 1_000_010_000);
        trace.record_stage(DataFlowStage::KernelLaunched, 1_000_020_000);
        trace.record_stage(DataFlowStage::Executing, 1_000_030_000);
        trace.record_stage(DataFlowStage::ResultReady, 1_000_500_000);
        trace.record_stage(DataFlowStage::Completed, 1_000_510_000);

        assert_eq!(trace.current_stage(), Some(DataFlowStage::Completed));
        assert_eq!(trace.total_latency_ns(), 510_000);
    }

    #[test]
    fn test_batch_config_latency_optimized() {
        let config = BatchConfig::latency_optimized();
        assert_eq!(config.max_batch_size, 1);
        assert!(!config.dynamic_batching);
    }

    #[test]
    fn test_batch_config_throughput_optimized() {
        let config = BatchConfig::throughput_optimized();
        assert_eq!(config.max_batch_size, 256);
        assert!(config.dynamic_batching);
    }

    #[test]
    fn test_batch_config_balanced() {
        let config = BatchConfig::balanced();
        assert_eq!(config.max_batch_size, 32);
        assert!(config.dynamic_batching);
    }

    #[test]
    fn test_padding_strategy_display() {
        assert_eq!(format!("{}", PaddingStrategy::NoPadding), "NoPadding");
        assert_eq!(format!("{}", PaddingStrategy::PadToMax), "PadToMax");
        assert_eq!(format!("{}", PaddingStrategy::CustomPacking), "CustomPacking");
    }

    #[test]
    fn test_padding_strategy_pad_to_fixed() {
        let strategy = PaddingStrategy::PadToFixed(2048);
        let display_str = format!("{}", strategy);
        assert!(display_str.contains("2048"));
    }

    #[test]
    fn test_inference_request_display() {
        let request = InferenceRequest {
            ct_id: [1u8; 16],
            model_id: [2u8; 16],
            input_tokens: 100,
            max_output_tokens: 256,
            priority: 128,
            deadline_ms: 1000,
            timestamp_ns: 1_000_000_000,
        };

        let display_str = format!("{}", request);
        assert!(display_str.contains("InferenceRequest"));
        assert!(display_str.contains("100"));
    }
}
