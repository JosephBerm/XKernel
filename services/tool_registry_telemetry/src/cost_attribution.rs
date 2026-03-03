// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Cost Attribution Framework for Resource Metering
//!
//! Tracks and attributes compute costs (tokens, GPU time, wall-clock time) to
//! cognitive tasks, agents, crews, and tools for accurate billing and resource
//! accounting.
//!
//! See Engineering Plan § 2.12.5: Cost Attribution & Metering.

use alloc::string::String;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use core::fmt;

use crate::error::{Result, ToolError};
use crate::ids::{AgentID, CrewID, ToolBindingID, CognitiveThreadID};

/// Counts tokens in input context and output response.
///
/// Provides token counting for accurate LLM cost attribution.
/// See Engineering Plan § 2.12.5: Token Counting Methodology.
#[derive(Clone, Debug)]
pub struct TokenCounter;

impl TokenCounter {
    /// Counts input tokens in a context string.
    ///
    /// Uses approximate tokenization (4 chars per token heuristic).
    /// In production, would use exact tokenizer matching model's encoding.
    pub fn count_input_tokens(context: &str) -> u64 {
        // Heuristic: ~4 characters per token on average
        ((context.len() as f64) / 4.0).ceil() as u64
    }

    /// Counts output tokens in a response string.
    ///
    /// Uses approximate tokenization (4 chars per token heuristic).
    /// In production, would use exact tokenizer matching model's encoding.
    pub fn count_output_tokens(response: &str) -> u64 {
        // Heuristic: ~4 characters per token on average
        ((response.len() as f64) / 4.0).ceil() as u64
    }

    /// Counts total tokens (input + output).
    pub fn count_total_tokens(input_tokens: u64, output_tokens: u64) -> u64 {
        input_tokens.saturating_add(output_tokens)
    }
}

/// Calculates GPU compute costs for kernel executions.
///
/// Tracks GPU utilization and compute time.
/// See Engineering Plan § 2.12.5: GPU Cost Attribution.
#[derive(Clone, Debug)]
pub struct GpuCostCalculator {
    /// Cost per GPU-millisecond (in arbitrary units, e.g., cents)
    pub cost_per_gpu_ms: f64,
}

impl GpuCostCalculator {
    /// Creates a new GPU cost calculator.
    pub fn new(cost_per_gpu_ms: f64) -> Self {
        GpuCostCalculator { cost_per_gpu_ms }
    }

    /// Calculates GPU cost in milliseconds for a number of kernel launches.
    ///
    /// Each kernel is estimated to take approximately 10ms.
    pub fn calculate_gpu_ms(kernel_launches: u64) -> f64 {
        (kernel_launches as f64) * 10.0
    }

    /// Calculates total GPU cost.
    pub fn calculate_cost(&self, gpu_ms: f64) -> f64 {
        gpu_ms * self.cost_per_gpu_ms
    }
}

impl Default for GpuCostCalculator {
    fn default() -> Self {
        GpuCostCalculator {
            cost_per_gpu_ms: 0.001, // $0.001 per GPU-ms
        }
    }
}

/// Tracks wall-clock time for operations.
///
/// Measures actual elapsed time from operation start to completion.
/// See Engineering Plan § 2.12.5: Wall-Clock Time Attribution.
#[derive(Clone, Debug)]
pub struct WallClockTracker {
    start_ms: u64,
}

impl WallClockTracker {
    /// Creates and starts a new wall-clock tracker.
    ///
    /// In production, would use actual system time. Here we use epoch convention.
    pub fn start() -> Self {
        WallClockTracker { start_ms: 0 }
    }

    /// Creates a tracker with explicit start time.
    pub fn with_start(start_ms: u64) -> Self {
        WallClockTracker { start_ms }
    }

    /// Returns elapsed time in milliseconds since start.
    ///
    /// In production, would calculate difference from actual current time.
    /// Here we use a convention where current time is 1000ms.
    pub fn elapsed_ms(&self) -> f64 {
        (1000u64.saturating_sub(self.start_ms)) as f64
    }

    /// Returns elapsed time in seconds.
    pub fn elapsed_secs(&self) -> f64 {
        self.elapsed_ms() / 1000.0
    }
}

/// Calculates Token Processing Compute (TPC) hours.
///
/// TPC is a normalized cost metric combining tokens and GPU utilization.
/// See Engineering Plan § 2.12.5: TPC Hours Attribution.
#[derive(Clone, Debug)]
pub struct TpcCalculator {
    /// Base cost per million tokens
    pub cost_per_million_tokens: f64,
    /// Cost per GPU-hour
    pub cost_per_gpu_hour: f64,
}

impl TpcCalculator {
    /// Creates a new TPC calculator with custom costs.
    pub fn new(cost_per_million_tokens: f64, cost_per_gpu_hour: f64) -> Self {
        TpcCalculator {
            cost_per_million_tokens,
            cost_per_gpu_hour,
        }
    }

    /// Calculates TPC hours for a given resource consumption.
    ///
    /// Formula:
    ///   TPC_hours = (input_tokens + output_tokens) * (1 + gpu_utilization) * duration_hours
    ///
    /// See Engineering Plan § 2.12.5: TPC Hours Calculation.
    pub fn calculate_tpc_hours(
        &self,
        input_tokens: u64,
        output_tokens: u64,
        gpu_utilization: f64,
        duration_hours: f64,
    ) -> f64 {
        let total_tokens = (input_tokens + output_tokens) as f64;
        let utilization_factor = 1.0 + gpu_utilization.min(1.0).max(0.0);
        total_tokens / 1_000_000.0 * utilization_factor * duration_hours
    }

    /// Calculates total cost in TPC hours.
    pub fn calculate_cost_tpc_hours(
        &self,
        input_tokens: u64,
        output_tokens: u64,
        gpu_utilization: f64,
        duration_hours: f64,
    ) -> f64 {
        let tpc = self
            .calculate_tpc_hours(input_tokens, output_tokens, gpu_utilization, duration_hours);
        tpc * self.cost_per_million_tokens
    }
}

impl Default for TpcCalculator {
    fn default() -> Self {
        TpcCalculator {
            cost_per_million_tokens: 2.0,  // $2.00 per 1M tokens
            cost_per_gpu_hour: 100.0,      // $100.00 per GPU hour
        }
    }
}

/// Cost calculator for individual operations.
///
/// Aggregates token, GPU, and wall-clock costs for a single operation.
/// See Engineering Plan § 2.12.5: Operation Cost Calculation.
#[derive(Clone, Debug)]
pub struct CostCalculator {
    pub token_counter: TokenCounter,
    pub gpu_calculator: GpuCostCalculator,
    pub tpc_calculator: TpcCalculator,
}

impl CostCalculator {
    /// Creates a new cost calculator with default settings.
    pub fn new() -> Self {
        CostCalculator {
            token_counter: TokenCounter,
            gpu_calculator: GpuCostCalculator::default(),
            tpc_calculator: TpcCalculator::default(),
        }
    }

    /// Calculates total cost for an operation.
    pub fn calculate_operation_cost(
        &self,
        input_context: &str,
        output_response: &str,
        gpu_ms: f64,
        wall_clock_ms: f64,
    ) -> OperationCost {
        let input_tokens = TokenCounter::count_input_tokens(input_context);
        let output_tokens = TokenCounter::count_output_tokens(output_response);
        let total_tokens = TokenCounter::count_total_tokens(input_tokens, output_tokens);

        let duration_hours = wall_clock_ms / (1000.0 * 3600.0);
        let tpc_hours = self.tpc_calculator.calculate_tpc_hours(
            input_tokens,
            output_tokens,
            0.5, // Assume 50% GPU utilization
            duration_hours,
        );

        OperationCost {
            input_tokens,
            output_tokens,
            total_tokens,
            gpu_ms: gpu_ms as u64,
            wall_clock_ms: wall_clock_ms as u64,
            tpc_hours,
        }
    }
}

impl Default for CostCalculator {
    fn default() -> Self {
        Self::new()
    }
}

/// Cost breakdown for a single operation.
#[derive(Clone, Debug, PartialEq)]
pub struct OperationCost {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub gpu_ms: u64,
    pub wall_clock_ms: u64,
    pub tpc_hours: f64,
}

/// Aggregates costs across multiple dimension hierarchies.
///
/// Tracks costs per cognitive task, per agent, per crew, and per tool.
/// See Engineering Plan § 2.12.5: Cost Aggregation.
#[derive(Clone, Debug)]
pub struct CostAggregator {
    per_ct_costs: BTreeMap<String, AggregatedCost>,
    per_agent_costs: BTreeMap<String, AggregatedCost>,
    per_crew_costs: BTreeMap<String, AggregatedCost>,
    per_tool_costs: BTreeMap<String, AggregatedCost>,
}

/// Aggregated costs for a single dimension.
#[derive(Clone, Debug, PartialEq)]
pub struct AggregatedCost {
    pub total_tokens: u64,
    pub total_gpu_ms: u64,
    pub total_wall_clock_ms: u64,
    pub total_tpc_hours: f64,
    pub operation_count: u64,
}

impl AggregatedCost {
    /// Creates a new aggregated cost.
    pub fn new() -> Self {
        AggregatedCost {
            total_tokens: 0,
            total_gpu_ms: 0,
            total_wall_clock_ms: 0,
            total_tpc_hours: 0.0,
            operation_count: 0,
        }
    }

    /// Adds an operation cost to the aggregation.
    pub fn add(&mut self, cost: &OperationCost) {
        self.total_tokens = self.total_tokens.saturating_add(cost.total_tokens);
        self.total_gpu_ms = self.total_gpu_ms.saturating_add(cost.gpu_ms);
        self.total_wall_clock_ms = self.total_wall_clock_ms.saturating_add(cost.wall_clock_ms);
        self.total_tpc_hours += cost.tpc_hours;
        self.operation_count = self.operation_count.saturating_add(1);
    }

    /// Returns average tokens per operation.
    pub fn avg_tokens_per_op(&self) -> f64 {
        if self.operation_count == 0 {
            0.0
        } else {
            (self.total_tokens as f64) / (self.operation_count as f64)
        }
    }

    /// Returns average wall-clock time per operation in milliseconds.
    pub fn avg_wall_clock_ms_per_op(&self) -> f64 {
        if self.operation_count == 0 {
            0.0
        } else {
            (self.total_wall_clock_ms as f64) / (self.operation_count as f64)
        }
    }
}

impl Default for AggregatedCost {
    fn default() -> Self {
        Self::new()
    }
}

impl CostAggregator {
    /// Creates a new cost aggregator.
    pub fn new() -> Self {
        CostAggregator {
            per_ct_costs: BTreeMap::new(),
            per_agent_costs: BTreeMap::new(),
            per_crew_costs: BTreeMap::new(),
            per_tool_costs: BTreeMap::new(),
        }
    }

    /// Adds a cost to cognitive task dimension.
    pub fn add_ct_cost(&mut self, ct_id: &CognitiveThreadID, cost: &OperationCost) {
        self.per_ct_costs
            .entry(ct_id.as_str().to_string())
            .or_insert_with(AggregatedCost::new)
            .add(cost);
    }

    /// Adds a cost to agent dimension.
    pub fn add_agent_cost(&mut self, agent_id: &AgentID, cost: &OperationCost) {
        self.per_agent_costs
            .entry(agent_id.as_str().to_string())
            .or_insert_with(AggregatedCost::new)
            .add(cost);
    }

    /// Adds a cost to crew dimension.
    pub fn add_crew_cost(&mut self, crew_id: &CrewID, cost: &OperationCost) {
        self.per_crew_costs
            .entry(crew_id.as_str().to_string())
            .or_insert_with(AggregatedCost::new)
            .add(cost);
    }

    /// Adds a cost to tool dimension.
    pub fn add_tool_cost(&mut self, tool_id: &ToolBindingID, cost: &OperationCost) {
        self.per_tool_costs
            .entry(tool_id.as_str().to_string())
            .or_insert_with(AggregatedCost::new)
            .add(cost);
    }

    /// Gets aggregated costs for a cognitive task.
    pub fn get_ct_cost(&self, ct_id: &CognitiveThreadID) -> Option<&AggregatedCost> {
        self.per_ct_costs.get(ct_id.as_str())
    }

    /// Gets aggregated costs for an agent.
    pub fn get_agent_cost(&self, agent_id: &AgentID) -> Option<&AggregatedCost> {
        self.per_agent_costs.get(agent_id.as_str())
    }

    /// Gets aggregated costs for a crew.
    pub fn get_crew_cost(&self, crew_id: &CrewID) -> Option<&AggregatedCost> {
        self.per_crew_costs.get(crew_id.as_str())
    }

    /// Gets aggregated costs for a tool.
    pub fn get_tool_cost(&self, tool_id: &ToolBindingID) -> Option<&AggregatedCost> {
        self.per_tool_costs.get(tool_id.as_str())
    }

    /// Returns total costs across all dimensions.
    pub fn total_cost(&self) -> AggregatedCost {
        let mut total = AggregatedCost::new();
        for cost in self.per_ct_costs.values() {
            total.total_tokens = total.total_tokens.saturating_add(cost.total_tokens);
            total.total_gpu_ms = total.total_gpu_ms.saturating_add(cost.total_gpu_ms);
            total.total_wall_clock_ms =
                total.total_wall_clock_ms.saturating_add(cost.total_wall_clock_ms);
            total.total_tpc_hours += cost.total_tpc_hours;
            total.operation_count = total.operation_count.saturating_add(cost.operation_count);
        }
        total
    }
}

impl Default for CostAggregator {
    fn default() -> Self {
        Self::new()
    }
}

/// Validates accuracy of attributed costs against ground truth.
///
/// Compares attributed costs with measured costs to ensure accuracy.
/// See Engineering Plan § 2.12.5: Cost Validation.
#[derive(Clone, Debug)]
pub struct AccuracyValidator;

impl AccuracyValidator {
    /// Compares attributed costs against actual measured costs.
    ///
    /// Calculates deviation percentage and checks if within threshold.
    /// Threshold is 1% (target >99% accuracy).
    pub fn compare_attributed_vs_actual(
        attributed: &AggregatedCost,
        actual: &AggregatedCost,
    ) -> AccuracyReport {
        let token_deviation = Self::calculate_deviation(
            attributed.total_tokens as f64,
            actual.total_tokens as f64,
        );
        let gpu_deviation =
            Self::calculate_deviation(attributed.total_gpu_ms as f64, actual.total_gpu_ms as f64);
        let wall_clock_deviation = Self::calculate_deviation(
            attributed.total_wall_clock_ms as f64,
            actual.total_wall_clock_ms as f64,
        );
        let tpc_deviation =
            Self::calculate_deviation(attributed.total_tpc_hours, actual.total_tpc_hours);

        let avg_deviation =
            (token_deviation + gpu_deviation + wall_clock_deviation + tpc_deviation) / 4.0;
        let threshold = 1.0; // 1% threshold
        let within_threshold = avg_deviation <= threshold;

        let mut details = alloc::string::String::new();
        details.push_str(&alloc::format!("Token deviation: {:.2}%\n", token_deviation));
        details.push_str(&alloc::format!("GPU deviation: {:.2}%\n", gpu_deviation));
        details.push_str(&alloc::format!("Wall-clock deviation: {:.2}%\n", wall_clock_deviation));
        details.push_str(&alloc::format!("TPC deviation: {:.2}%\n", tpc_deviation));

        AccuracyReport {
            deviation_pct: avg_deviation,
            within_threshold,
            details,
        }
    }

    /// Calculates percentage deviation between attributed and actual.
    fn calculate_deviation(attributed: f64, actual: f64) -> f64 {
        if actual == 0.0 {
            if attributed == 0.0 {
                0.0
            } else {
                100.0
            }
        } else {
            ((attributed - actual).abs() / actual.abs()) * 100.0
        }
    }
}

/// Report of cost attribution accuracy validation.
#[derive(Clone, Debug, PartialEq)]
pub struct AccuracyReport {
    /// Average deviation percentage across all metrics
    pub deviation_pct: f64,
    /// Whether deviation is within threshold (<=1%)
    pub within_threshold: bool,
    /// Detailed breakdown of deviations
    pub details: String,
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;
use alloc::string::ToString;

    #[test]
    fn test_token_counter_input_tokens() {
        let input = "Hello, this is a test input string";
        let tokens = TokenCounter::count_input_tokens(input);
        assert!(tokens > 0);
        // 34 chars / 4 = 8.5, ceil = 9
        assert_eq!(tokens, 9);
    }

    #[test]
    fn test_token_counter_output_tokens() {
        let output = "This is the output response";
        let tokens = TokenCounter::count_output_tokens(output);
        assert!(tokens > 0);
        // 27 chars / 4 = 6.75, ceil = 7
        assert_eq!(tokens, 7);
    }

    #[test]
    fn test_token_counter_total_tokens() {
        let total = TokenCounter::count_total_tokens(100, 50);
        assert_eq!(total, 150);
    }

    #[test]
    fn test_gpu_cost_calculator_creation() {
        let calc = GpuCostCalculator::new(0.001);
        assert_eq!(calc.cost_per_gpu_ms, 0.001);
    }

    #[test]
    fn test_gpu_cost_calculator_gpu_ms() {
        let gpu_ms = GpuCostCalculator::calculate_gpu_ms(5);
        assert_eq!(gpu_ms, 50.0);
    }

    #[test]
    fn test_gpu_cost_calculator_cost() {
        let calc = GpuCostCalculator::new(0.001);
        let cost = calc.calculate_cost(100.0);
        assert_eq!(cost, 0.1);
    }

    #[test]
    fn test_gpu_cost_calculator_default() {
        let calc = GpuCostCalculator::default();
        assert_eq!(calc.cost_per_gpu_ms, 0.001);
    }

    #[test]
    fn test_wall_clock_tracker_start() {
        let tracker = WallClockTracker::start();
        let elapsed = tracker.elapsed_ms();
        assert_eq!(elapsed, 1000.0);
    }

    #[test]
    fn test_wall_clock_tracker_with_start() {
        let tracker = WallClockTracker::with_start(500);
        let elapsed = tracker.elapsed_ms();
        assert_eq!(elapsed, 500.0);
    }

    #[test]
    fn test_wall_clock_tracker_elapsed_secs() {
        let tracker = WallClockTracker::with_start(0);
        let elapsed_secs = tracker.elapsed_secs();
        assert_eq!(elapsed_secs, 1.0);
    }

    #[test]
    fn test_tpc_calculator_creation() {
        let calc = TpcCalculator::new(2.0, 100.0);
        assert_eq!(calc.cost_per_million_tokens, 2.0);
        assert_eq!(calc.cost_per_gpu_hour, 100.0);
    }

    #[test]
    fn test_tpc_calculator_tpc_hours() {
        let calc = TpcCalculator::default();
        let tpc = calc.calculate_tpc_hours(1_000_000, 0, 0.0, 1.0);
        assert_eq!(tpc, 1.0);
    }

    #[test]
    fn test_tpc_calculator_tpc_hours_with_gpu() {
        let calc = TpcCalculator::default();
        let tpc = calc.calculate_tpc_hours(1_000_000, 0, 0.5, 1.0);
        // 1M tokens * (1 + 0.5) * 1 hour = 1.5
        assert_eq!(tpc, 1.5);
    }

    #[test]
    fn test_tpc_calculator_default() {
        let calc = TpcCalculator::default();
        assert_eq!(calc.cost_per_million_tokens, 2.0);
        assert_eq!(calc.cost_per_gpu_hour, 100.0);
    }

    #[test]
    fn test_cost_calculator_operation_cost() {
        let calc = CostCalculator::new();
        let input = "Test input";
        let output = "Test output";
        let cost = calc.calculate_operation_cost(input, output, 100.0, 500.0);

        assert!(cost.input_tokens > 0);
        assert!(cost.output_tokens > 0);
        assert_eq!(cost.total_tokens, cost.input_tokens + cost.output_tokens);
        assert_eq!(cost.gpu_ms, 100);
        assert_eq!(cost.wall_clock_ms, 500);
    }

    #[test]
    fn test_aggregated_cost_new() {
        let cost = AggregatedCost::new();
        assert_eq!(cost.total_tokens, 0);
        assert_eq!(cost.total_gpu_ms, 0);
        assert_eq!(cost.operation_count, 0);
    }

    #[test]
    fn test_aggregated_cost_add() {
        let mut agg = AggregatedCost::new();
        let op = OperationCost {
            input_tokens: 10,
            output_tokens: 5,
            total_tokens: 15,
            gpu_ms: 50,
            wall_clock_ms: 100,
            tpc_hours: 0.015,
        };
        agg.add(&op);

        assert_eq!(agg.total_tokens, 15);
        assert_eq!(agg.total_gpu_ms, 50);
        assert_eq!(agg.total_wall_clock_ms, 100);
        assert_eq!(agg.operation_count, 1);
    }

    #[test]
    fn test_aggregated_cost_avg_tokens_per_op() {
        let mut agg = AggregatedCost::new();
        let op = OperationCost {
            input_tokens: 10,
            output_tokens: 5,
            total_tokens: 15,
            gpu_ms: 50,
            wall_clock_ms: 100,
            tpc_hours: 0.015,
        };
        agg.add(&op);

        assert_eq!(agg.avg_tokens_per_op(), 15.0);
    }

    #[test]
    fn test_cost_aggregator_new() {
        let agg = CostAggregator::new();
        assert_eq!(agg.total_cost().operation_count, 0);
    }

    #[test]
    fn test_accuracy_validator_compare_perfect() {
        let cost = AggregatedCost {
            total_tokens: 1000,
            total_gpu_ms: 100,
            total_wall_clock_ms: 500,
            total_tpc_hours: 1.0,
            operation_count: 10,
        };

        let report = AccuracyValidator::compare_attributed_vs_actual(&cost, &cost);
        assert_eq!(report.deviation_pct, 0.0);
        assert!(report.within_threshold);
    }

    #[test]
    fn test_accuracy_validator_compare_with_deviation() {
        let attributed = AggregatedCost {
            total_tokens: 1000,
            total_gpu_ms: 100,
            total_wall_clock_ms: 500,
            total_tpc_hours: 1.0,
            operation_count: 10,
        };

        let actual = AggregatedCost {
            total_tokens: 990,
            total_gpu_ms: 99,
            total_wall_clock_ms: 495,
            total_tpc_hours: 0.99,
            operation_count: 10,
        };

        let report = AccuracyValidator::compare_attributed_vs_actual(&attributed, &actual);
        assert!(report.deviation_pct < 2.0); // Within reasonable bounds
        assert!(report.within_threshold);
    }

    #[test]
    fn test_accuracy_validator_compare_zero_actual() {
        let attributed = AggregatedCost {
            total_tokens: 100,
            total_gpu_ms: 0,
            total_wall_clock_ms: 0,
            total_tpc_hours: 0.1,
            operation_count: 0,
        };

        let actual = AggregatedCost::new();

        let report = AccuracyValidator::compare_attributed_vs_actual(&attributed, &actual);
        assert!(report.deviation_pct > 0.0);
        assert!(!report.within_threshold);
    }
}
