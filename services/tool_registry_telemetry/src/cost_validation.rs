// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Cost Attribution Validation Methodology
//!
//! Provides frameworks and sampling strategies for validating accuracy of attributed
//! costs against ground truth measurements, ensuring metering accuracy exceeds 99%.
//!
//! See Engineering Plan § 2.12.5: Cost Validation & Reconciliation.

use alloc::string::String;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use core::fmt;

use crate::error::{Result, ToolError};

/// Source of ground truth for cost validation.
///
/// See Engineering Plan § 2.12.5: Ground Truth Sources.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroundTruthSource {
    /// Hardware performance counters (most accurate)
    HardwareCounters,
    /// API metering systems
    ApiMetering,
    /// Manual instrumentation
    ManualInstrument,
}

impl fmt::Display for GroundTruthSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GroundTruthSource::HardwareCounters => write!(f, "HardwareCounters"),
            GroundTruthSource::ApiMetering => write!(f, "ApiMetering"),
            GroundTruthSource::ManualInstrument => write!(f, "ManualInstrument"),
        }
    }
}

/// Sampling strategy when full instrumentation is not feasible.
///
/// See Engineering Plan § 2.12.5: Sampling Strategies.
#[derive(Clone, Debug, PartialEq)]
pub enum SamplingStrategy {
    /// Fixed sample rate (0.0-1.0, where 1.0 = 100%)
    SampleRate {
        /// Sampling rate as fraction of all events
        rate: f64,
    },

    /// Adaptive sampling based on cost/relevance
    AdaptiveSampling {
        /// Minimum sample rate
        min_rate: f64,
        /// Maximum sample rate
        max_rate: f64,
        /// Cost threshold for automatic 100% sampling
        cost_threshold: f64,
    },

    /// Importance sampling based on deviation
    ImportanceSampling {
        /// Base sample rate
        base_rate: f64,
        /// Multiplier when deviation is detected
        deviation_multiplier: f64,
    },
}

impl SamplingStrategy {
    /// Fixed 1% sampling rate.
    pub fn one_percent() -> Self {
        SamplingStrategy::SampleRate { rate: 0.01 }
    }

    /// Fixed 10% sampling rate.
    pub fn ten_percent() -> Self {
        SamplingStrategy::SampleRate { rate: 0.1 }
    }

    /// No sampling (100% of events sampled).
    pub fn full_instrumentation() -> Self {
        SamplingStrategy::SampleRate { rate: 1.0 }
    }

    /// Determines if an event should be sampled.
    pub fn should_sample(&self, event_index: u64, cost_value: f64) -> bool {
        match self {
            SamplingStrategy::SampleRate { rate } => {
                // Use event index as pseudo-random seed
                ((event_index as f64 % 1000.0) / 1000.0) < *rate
            }
            SamplingStrategy::AdaptiveSampling {
                min_rate,
                max_rate,
                cost_threshold,
            } => {
                if cost_value >= *cost_threshold {
                    // Always sample high-cost events
                    true
                } else {
                    // Use adaptive rate based on cost
                    let adaptive_rate = min_rate + ((cost_value / cost_threshold) * (max_rate - min_rate));
                    ((event_index as f64 % 1000.0) / 1000.0) < adaptive_rate
                }
            }
            SamplingStrategy::ImportanceSampling {
                base_rate,
                deviation_multiplier,
            } => {
                // In production, would calculate actual deviation
                let deviation_factor = if event_index % 10 == 0 { *deviation_multiplier } else { 1.0 };
                let adjusted_rate = base_rate * deviation_factor;
                ((event_index as f64 % 1000.0) / 1000.0) < adjusted_rate.min(1.0)
            }
        }
    }
}

impl Default for SamplingStrategy {
    fn default() -> Self {
        SamplingStrategy::ten_percent()
    }
}

impl fmt::Display for SamplingStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SamplingStrategy::SampleRate { rate } => {
                write!(f, "SampleRate({}%)", (rate * 100.0) as u32)
            }
            SamplingStrategy::AdaptiveSampling {
                min_rate,
                max_rate,
                cost_threshold,
            } => {
                write!(f, "AdaptiveSampling({}%-{}%, threshold={})",
                    (min_rate * 100.0) as u32, (max_rate * 100.0) as u32, cost_threshold)
            }
            SamplingStrategy::ImportanceSampling {
                base_rate,
                deviation_multiplier,
            } => {
                write!(f, "ImportanceSampling({}%, mult={}x)",
                    (base_rate * 100.0) as u32, deviation_multiplier)
            }
        }
    }
}

/// Validation framework configuration.
///
/// See Engineering Plan § 2.12.5: Validation Framework.
#[derive(Clone, Debug)]
pub struct ValidationFramework {
    /// Source of ground truth measurements
    pub ground_truth_source: GroundTruthSource,

    /// Strategy for comparing attributed vs actual costs
    pub comparison_strategy: ComparisonStrategy,

    /// Accuracy threshold (percentage, e.g., 1.0 for 1%)
    pub threshold_pct: f64,

    /// Sampling strategy when full validation not feasible
    pub sampling_strategy: SamplingStrategy,
}

/// Strategy for comparing two cost values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComparisonStrategy {
    /// Absolute difference (suitable for small values)
    AbsoluteDifference,
    /// Relative percentage difference (suitable for scaled metrics)
    RelativePercentage,
    /// Mean absolute percentage error (MAPE)
    MeanAbsolutePercentageError,
}

impl ValidationFramework {
    /// Creates a new validation framework with defaults.
    pub fn new(ground_truth_source: GroundTruthSource) -> Self {
        ValidationFramework {
            ground_truth_source,
            comparison_strategy: ComparisonStrategy::RelativePercentage,
            threshold_pct: 1.0, // 1% = 99% accuracy target
            sampling_strategy: SamplingStrategy::default(),
        }
    }

    /// Creates framework for high-accuracy hardware counter validation.
    pub fn hardware_counters() -> Self {
        ValidationFramework {
            ground_truth_source: GroundTruthSource::HardwareCounters,
            comparison_strategy: ComparisonStrategy::RelativePercentage,
            threshold_pct: 0.5,
            sampling_strategy: SamplingStrategy::full_instrumentation(),
        }
    }

    /// Creates framework for API metering validation.
    pub fn api_metering() -> Self {
        ValidationFramework {
            ground_truth_source: GroundTruthSource::ApiMetering,
            comparison_strategy: ComparisonStrategy::RelativePercentage,
            threshold_pct: 1.0,
            sampling_strategy: SamplingStrategy::ten_percent(),
        }
    }

    /// Sets the sampling strategy.
    pub fn with_sampling(mut self, strategy: SamplingStrategy) -> Self {
        self.sampling_strategy = strategy;
        self
    }

    /// Sets the comparison strategy.
    pub fn with_comparison(mut self, strategy: ComparisonStrategy) -> Self {
        self.comparison_strategy = strategy;
        self
    }

    /// Sets the accuracy threshold.
    pub fn with_threshold(mut self, threshold_pct: f64) -> Self {
        self.threshold_pct = threshold_pct;
        self
    }
}

impl Default for ValidationFramework {
    fn default() -> Self {
        Self::new(GroundTruthSource::ApiMetering)
    }
}

/// Report of cost validation results.
///
/// See Engineering Plan § 2.12.5: Validation Report.
#[derive(Clone, Debug, PartialEq)]
pub struct ValidationReport {
    /// Overall accuracy percentage (100% = perfect match)
    pub accuracy_pct: f64,

    /// List of events that exceeded deviation threshold (outliers)
    pub outliers: Vec<OutlierEvent>,

    /// Per-tool accuracy breakdown
    pub per_tool_accuracy: BTreeMap<String, f64>,

    /// Per-metric accuracy breakdown (tokens, gpu_ms, wall_clock_ms, tpc_hours)
    pub per_metric_accuracy: BTreeMap<String, f64>,

    /// Number of samples validated
    pub samples_validated: u64,

    /// Number of samples that passed threshold
    pub samples_passed: u64,

    /// Summary status
    pub passed: bool,
}

impl ValidationReport {
    /// Creates a new validation report.
    pub fn new() -> Self {
        ValidationReport {
            accuracy_pct: 0.0,
            outliers: Vec::new(),
            per_tool_accuracy: BTreeMap::new(),
            per_metric_accuracy: BTreeMap::new(),
            samples_validated: 0,
            samples_passed: 0,
            passed: false,
        }
    }

    /// Returns outlier count.
    pub fn outlier_count(&self) -> usize {
        self.outliers.len()
    }

    /// Returns pass rate as percentage.
    pub fn pass_rate(&self) -> f64 {
        if self.samples_validated == 0 {
            100.0
        } else {
            (self.samples_passed as f64 / self.samples_validated as f64) * 100.0
        }
    }
}

impl Default for ValidationReport {
    fn default() -> Self {
        Self::new()
    }
}

/// Outlier event in cost validation.
#[derive(Clone, Debug, PartialEq)]
pub struct OutlierEvent {
    /// Event ID
    pub event_id: String,
    /// Attributed cost value
    pub attributed: f64,
    /// Actual/ground truth cost value
    pub actual: f64,
    /// Deviation percentage
    pub deviation_pct: f64,
    /// Which metric this is (e.g., "tokens", "gpu_ms")
    pub metric: String,
}

impl OutlierEvent {
    /// Creates a new outlier event.
    pub fn new(
        event_id: String,
        attributed: f64,
        actual: f64,
        metric: String,
    ) -> Self {
        let deviation_pct = if actual == 0.0 {
            if attributed == 0.0 {
                0.0
            } else {
                100.0
            }
        } else {
            ((attributed - actual).abs() / actual.abs()) * 100.0
        };

        OutlierEvent {
            event_id,
            attributed,
            actual,
            deviation_pct,
            metric,
        }
    }
}

/// Daily reconciliation process for cost validation.
///
/// See Engineering Plan § 2.12.5: Daily Reconciliation.
#[derive(Clone, Debug)]
pub struct DailyReconciliation {
    /// Framework used for validation
    pub framework: ValidationFramework,
    /// Reports for each day
    pub reports: BTreeMap<u32, ValidationReport>, // day of year -> report
}

impl DailyReconciliation {
    /// Creates a new daily reconciliation tracker.
    pub fn new(framework: ValidationFramework) -> Self {
        DailyReconciliation {
            framework,
            reports: BTreeMap::new(),
        }
    }

    /// Records a validation report for a specific day.
    pub fn record_report(&mut self, day_of_year: u32, report: ValidationReport) {
        self.reports.insert(day_of_year, report);
    }

    /// Gets report for a day.
    pub fn get_report(&self, day_of_year: u32) -> Option<&ValidationReport> {
        self.reports.get(&day_of_year)
    }

    /// Returns average accuracy across all days.
    pub fn average_accuracy(&self) -> f64 {
        if self.reports.is_empty() {
            0.0
        } else {
            let sum: f64 = self.reports.values().map(|r| r.accuracy_pct).sum();
            sum / self.reports.len() as f64
        }
    }

    /// Returns whether all reports passed validation.
    pub fn all_passed(&self) -> bool {
        self.reports.values().all(|r| r.passed)
    }

    /// Returns total outliers across all days.
    pub fn total_outliers(&self) -> usize {
        self.reports.values().map(|r| r.outlier_count()).sum()
    }
}

impl Default for DailyReconciliation {
    fn default() -> Self {
        DailyReconciliation::new(ValidationFramework::default())
    }
}

/// Runs validation of attributed costs against ground truth.
///
/// See Engineering Plan § 2.12.5: Validation Methodology.
pub fn run_validation(
    framework: &ValidationFramework,
    attributed_costs: &[(String, f64)], // (event_id, cost)
    ground_truth_costs: &[(String, f64)], // (event_id, cost)
) -> ValidationReport {
    let mut report = ValidationReport::new();

    // Build ground truth map
    let ground_truth_map: BTreeMap<String, f64> =
        ground_truth_costs.iter().cloned().collect();

    let mut total_deviation = 0.0;
    let mut passed_count = 0u64;

    for (event_id, attributed) in attributed_costs {
        if let Some(actual) = ground_truth_map.get(event_id) {
            let deviation = calculate_deviation(framework.comparison_strategy, *attributed, *actual);
            total_deviation += deviation;
            report.samples_validated += 1;

            if deviation <= framework.threshold_pct {
                passed_count += 1;
            } else {
                // Record outlier
                report.outliers.push(OutlierEvent::new(
                    event_id.clone(),
                    *attributed,
                    *actual,
                    "cost".to_string(),
                ));
            }
        }
    }

    report.samples_passed = passed_count;

    if report.samples_validated > 0 {
        report.accuracy_pct = 100.0 - (total_deviation / report.samples_validated as f64);
        report.passed = report.accuracy_pct >= (100.0 - framework.threshold_pct);
    } else {
        report.accuracy_pct = 100.0;
        report.passed = true;
    }

    report
}

/// Calculates deviation between attributed and actual values.
fn calculate_deviation(strategy: ComparisonStrategy, attributed: f64, actual: f64) -> f64 {
    match strategy {
        ComparisonStrategy::AbsoluteDifference => (attributed - actual).abs(),
        ComparisonStrategy::RelativePercentage => {
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
        ComparisonStrategy::MeanAbsolutePercentageError => {
            if actual == 0.0 {
                0.0
            } else {
                ((attributed - actual).abs() / actual.abs()) * 100.0
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::string::ToString;
use alloc::vec;

    #[test]
    fn test_ground_truth_source_display() {
        assert_eq!(GroundTruthSource::HardwareCounters.to_string(), "HardwareCounters");
        assert_eq!(GroundTruthSource::ApiMetering.to_string(), "ApiMetering");
        assert_eq!(GroundTruthSource::ManualInstrument.to_string(), "ManualInstrument");
    }

    #[test]
    fn test_sampling_strategy_one_percent() {
        let strategy = SamplingStrategy::one_percent();
        match strategy {
            SamplingStrategy::SampleRate { rate } => assert_eq!(rate, 0.01),
            _ => panic!("Expected SampleRate"),
        }
    }

    #[test]
    fn test_sampling_strategy_ten_percent() {
        let strategy = SamplingStrategy::ten_percent();
        match strategy {
            SamplingStrategy::SampleRate { rate } => assert_eq!(rate, 0.1),
            _ => panic!("Expected SampleRate"),
        }
    }

    #[test]
    fn test_sampling_strategy_full_instrumentation() {
        let strategy = SamplingStrategy::full_instrumentation();
        match strategy {
            SamplingStrategy::SampleRate { rate } => assert_eq!(rate, 1.0),
            _ => panic!("Expected SampleRate"),
        }
    }

    #[test]
    fn test_sampling_strategy_should_sample_fixed() {
        let strategy = SamplingStrategy::SampleRate { rate: 0.5 };
        let mut sampled_count = 0;
        for i in 0..100 {
            if strategy.should_sample(i, 1.0) {
                sampled_count += 1;
            }
        }
        // Approximately 50%
        assert!(sampled_count > 30 && sampled_count < 70);
    }

    #[test]
    fn test_sampling_strategy_should_sample_adaptive() {
        let strategy = SamplingStrategy::AdaptiveSampling {
            min_rate: 0.05,
            max_rate: 0.95,
            cost_threshold: 100.0,
        };
        // High-cost event should always be sampled
        assert!(strategy.should_sample(0, 150.0));
    }

    #[test]
    fn test_sampling_strategy_display_fixed() {
        let strategy = SamplingStrategy::SampleRate { rate: 0.1 };
        let s = strategy.to_string();
        assert!(s.contains("10%"));
    }

    #[test]
    fn test_sampling_strategy_default() {
        let strategy = SamplingStrategy::default();
        match strategy {
            SamplingStrategy::SampleRate { rate } => assert_eq!(rate, 0.1),
            _ => panic!("Expected SampleRate"),
        }
    }

    #[test]
    fn test_validation_framework_new() {
        let framework = ValidationFramework::new(GroundTruthSource::HardwareCounters);
        assert_eq!(framework.ground_truth_source, GroundTruthSource::HardwareCounters);
        assert_eq!(framework.threshold_pct, 1.0);
    }

    #[test]
    fn test_validation_framework_hardware_counters() {
        let framework = ValidationFramework::hardware_counters();
        assert_eq!(framework.ground_truth_source, GroundTruthSource::HardwareCounters);
        assert_eq!(framework.threshold_pct, 0.5);
    }

    #[test]
    fn test_validation_framework_api_metering() {
        let framework = ValidationFramework::api_metering();
        assert_eq!(framework.ground_truth_source, GroundTruthSource::ApiMetering);
        assert_eq!(framework.threshold_pct, 1.0);
    }

    #[test]
    fn test_validation_framework_with_sampling() {
        let framework = ValidationFramework::new(GroundTruthSource::ApiMetering)
            .with_sampling(SamplingStrategy::one_percent());
        match framework.sampling_strategy {
            SamplingStrategy::SampleRate { rate } => assert_eq!(rate, 0.01),
            _ => panic!("Expected SampleRate"),
        }
    }

    #[test]
    fn test_validation_report_new() {
        let report = ValidationReport::new();
        assert_eq!(report.accuracy_pct, 0.0);
        assert!(report.outliers.is_empty());
    }

    #[test]
    fn test_validation_report_pass_rate_perfect() {
        let report = ValidationReport {
            accuracy_pct: 100.0,
            outliers: Vec::new(),
            per_tool_accuracy: BTreeMap::new(),
            per_metric_accuracy: BTreeMap::new(),
            samples_validated: 100,
            samples_passed: 100,
            passed: true,
        };
        assert_eq!(report.pass_rate(), 100.0);
    }

    #[test]
    fn test_validation_report_pass_rate_partial() {
        let report = ValidationReport {
            accuracy_pct: 95.0,
            outliers: Vec::new(),
            per_tool_accuracy: BTreeMap::new(),
            per_metric_accuracy: BTreeMap::new(),
            samples_validated: 100,
            samples_passed: 95,
            passed: true,
        };
        assert_eq!(report.pass_rate(), 95.0);
    }

    #[test]
    fn test_outlier_event_new() {
        let outlier = OutlierEvent::new("e1".to_string(), 100.0, 95.0, "tokens".to_string());
        assert_eq!(outlier.event_id, "e1");
        assert!(outlier.deviation_pct > 0.0);
    }

    #[test]
    fn test_outlier_event_zero_actual() {
        let outlier = OutlierEvent::new("e1".to_string(), 10.0, 0.0, "tokens".to_string());
        assert_eq!(outlier.deviation_pct, 100.0);
    }

    #[test]
    fn test_daily_reconciliation_new() {
        let framework = ValidationFramework::default();
        let reconciliation = DailyReconciliation::new(framework);
        assert!(reconciliation.reports.is_empty());
    }

    #[test]
    fn test_daily_reconciliation_record_report() {
        let framework = ValidationFramework::default();
        let mut reconciliation = DailyReconciliation::new(framework);
        let report = ValidationReport {
            accuracy_pct: 99.5,
            outliers: Vec::new(),
            per_tool_accuracy: BTreeMap::new(),
            per_metric_accuracy: BTreeMap::new(),
            samples_validated: 1000,
            samples_passed: 995,
            passed: true,
        };

        reconciliation.record_report(1, report);
        assert!(reconciliation.get_report(1).is_some());
    }

    #[test]
    fn test_daily_reconciliation_average_accuracy() {
        let framework = ValidationFramework::default();
        let mut reconciliation = DailyReconciliation::new(framework);

        let report1 = ValidationReport {
            accuracy_pct: 99.0,
            ..ValidationReport::new()
        };
        let report2 = ValidationReport {
            accuracy_pct: 100.0,
            ..ValidationReport::new()
        };

        reconciliation.record_report(1, report1);
        reconciliation.record_report(2, report2);

        assert_eq!(reconciliation.average_accuracy(), 99.5);
    }

    #[test]
    fn test_daily_reconciliation_all_passed() {
        let framework = ValidationFramework::default();
        let mut reconciliation = DailyReconciliation::new(framework);

        let report = ValidationReport {
            accuracy_pct: 99.5,
            passed: true,
            ..ValidationReport::new()
        };

        reconciliation.record_report(1, report);
        assert!(reconciliation.all_passed());
    }

    #[test]
    fn test_run_validation_perfect() {
        let framework = ValidationFramework::default();
        let attributed = vec![
            ("e1".to_string(), 100.0),
            ("e2".to_string(), 200.0),
        ];
        let ground_truth = vec![
            ("e1".to_string(), 100.0),
            ("e2".to_string(), 200.0),
        ];

        let report = run_validation(&framework, &attributed, &ground_truth);
        assert_eq!(report.accuracy_pct, 100.0);
        assert!(report.passed);
    }

    #[test]
    fn test_run_validation_with_deviation() {
        let framework = ValidationFramework::default();
        let attributed = vec![
            ("e1".to_string(), 100.0),
            ("e2".to_string(), 200.0),
        ];
        let ground_truth = vec![
            ("e1".to_string(), 99.0),  // 1% deviation
            ("e2".to_string(), 200.0), // 0% deviation
        ];

        let report = run_validation(&framework, &attributed, &ground_truth);
        assert!(report.accuracy_pct > 0.0);
        assert!(report.accuracy_pct < 100.0);
    }

    #[test]
    fn test_run_validation_with_outliers() {
        let framework = ValidationFramework::default();
        let attributed = vec![
            ("e1".to_string(), 100.0),
            ("e2".to_string(), 500.0), // Large deviation
        ];
        let ground_truth = vec![
            ("e1".to_string(), 100.0),
            ("e2".to_string(), 200.0),
        ];

        let report = run_validation(&framework, &attributed, &ground_truth);
        assert!(!report.outliers.is_empty());
    }
}
