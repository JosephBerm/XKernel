// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! Restart policies and backoff strategies.
//!
//! Defines restart policies and exponential backoff configuration for agent restart
//! behavior. Patterns are aligned with Kubernetes container restart policies.
//!
//! Reference: Engineering Plan § Agent Lifecycle Management § Restart Policies

use alloc::string::String;
use crate::Result;

/// Information about an agent failure for restart decision context.
///
/// Used to provide context when evaluating restart policies.
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Restart Policies
#[derive(Debug, Clone)]
pub struct FailureInfo {
    /// Number of failures that have occurred so far.
    pub failure_count: u32,

    /// Time since the last failure occurred (milliseconds).
    pub time_since_last_failure_ms: u64,

    /// Reason for the last failure.
    pub reason: String,
}

impl FailureInfo {
    /// Creates a new failure info instance.
    pub fn new(failure_count: u32, time_since_last_failure_ms: u64, reason: impl Into<String>) -> Self {
        Self {
            failure_count,
            time_since_last_failure_ms,
            reason: reason.into(),
        }
    }
}

/// Trait for evaluating restart decisions given failure context.
///
/// Implementors determine whether an agent should be restarted and
/// how long to wait before attempting restart.
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Restart Policies
pub trait RestartPolicyEngine {
    /// Evaluates whether to restart an agent given failure information.
    ///
    /// Arguments:
    /// - `failure_info`: Details about the failure
    ///
    /// Returns `Ok(RestartDecision)` if decision can be made, or `Err` if policy cannot decide.
    fn evaluate(&self, failure_info: &FailureInfo) -> Result<RestartDecision>;
}

/// Restart decision outcome from policy evaluation.
///
/// Encapsulates the decision of whether to restart and timing information.
///
/// # Fields
///
/// - `should_restart`: Whether restart should be attempted
/// - `delay_ms`: How long to wait before restart attempt (milliseconds)
/// - `reason`: Human-readable reason for the decision
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Restart Policies
#[derive(Debug, Clone)]
pub struct RestartDecisionOutcome {
    /// Whether to attempt restart.
    pub should_restart: bool,

    /// Delay in milliseconds before restart attempt.
    pub delay_ms: u64,

    /// Reason for the restart decision.
    pub reason: String,
}

impl RestartDecisionOutcome {
    /// Creates a new restart decision with given parameters.
    pub fn new(should_restart: bool, delay_ms: u64, reason: impl Into<String>) -> Self {
        Self {
            should_restart,
            delay_ms,
            reason: reason.into(),
        }
    }

    /// Creates a decision to restart.
    pub fn restart(delay_ms: u64, reason: impl Into<String>) -> Self {
        Self {
            should_restart: true,
            delay_ms,
            reason: reason.into(),
        }
    }

    /// Creates a decision to not restart.
    pub fn no_restart(reason: impl Into<String>) -> Self {
        Self {
            should_restart: false,
            delay_ms: 0,
            reason: reason.into(),
        }
    }
}

/// History of restart attempts for an agent.
///
/// Tracks restart statistics including total count, failures, and backoff state.
///
/// # Fields
///
/// - `restart_count`: Total number of restart attempts
/// - `total_failures`: Total failures that triggered restarts
/// - `last_restart_timestamp`: When the most recent restart occurred (milliseconds)
/// - `backoff_state`: Current backoff multiplier or attempt count
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Restart Policies
#[derive(Debug, Clone)]
pub struct RestartHistory {
    /// Total restart attempts made.
    pub restart_count: u32,

    /// Total failures that triggered restart attempts.
    pub total_failures: u32,

    /// Timestamp of most recent restart (milliseconds).
    pub last_restart_timestamp: u64,

    /// Current backoff attempt number (for exponential backoff calculation).
    pub backoff_state: u32,
}

impl RestartHistory {
    /// Creates a new empty restart history.
    pub fn new() -> Self {
        Self {
            restart_count: 0,
            total_failures: 0,
            last_restart_timestamp: 0,
            backoff_state: 0,
        }
    }

    /// Records a restart attempt.
    ///
    /// Arguments:
    /// - `current_timestamp_ms`: Current time in milliseconds
    pub fn record_restart(&mut self, current_timestamp_ms: u64) {
        self.restart_count += 1;
        self.last_restart_timestamp = current_timestamp_ms;
        self.backoff_state += 1;
    }

    /// Records a failure that triggered restart evaluation.
    pub fn record_failure(&mut self) {
        self.total_failures += 1;
    }

    /// Resets backoff state (successful restart).
    pub fn reset_backoff(&mut self) {
        self.backoff_state = 0;
    }

    /// Gets the current backoff attempt number.
    pub fn current_attempt(&self) -> u32 {
        self.backoff_state
    }
}

impl Default for RestartHistory {
    fn default() -> Self {
        Self::new()
    }
}

/// Restart policy determining behavior when an agent fails.
///
/// Aligned with Kubernetes container restart policy patterns to provide
/// familiar semantics for operators and consistency with industry standards.
///
/// # Variants
///
/// - **Always**: Always attempt to restart failed agents (suitable for long-running services)
/// - **OnFailure**: Only restart if agent exits with non-zero code (suitable for tasks)
/// - **Never**: Do not automatically restart (manual intervention required)
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Restart Policies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestartPolicy {
    /// Always restart the agent on failure.
    ///
    /// Use case: Long-running services that should remain available.
    /// Example: HTTP server, event listener, background worker.
    Always,

    /// Only restart if agent exits with failure status (non-zero exit code).
    ///
    /// Use case: Short-lived batch jobs and tasks that may complete successfully.
    /// Example: Data import job, scheduled task, one-time processor.
    OnFailure,

    /// Never automatically restart the agent.
    ///
    /// Use case: Debugging, manual lifecycle control, or agents requiring
    /// explicit restart authorization. Failure triggers alert but not restart.
    /// Example: Development agent, privileged operation handler.
    Never,
}

impl RestartPolicy {
    /// Returns true if this policy permits restart attempts.
    pub fn permits_restart(&self) -> bool {
        matches!(self, Self::Always | Self::OnFailure)
    }

    /// Returns true if this is the Always policy.
    pub fn is_always(&self) -> bool {
        matches!(self, Self::Always)
    }

    /// Returns true if this is the OnFailure policy.
    pub fn is_on_failure(&self) -> bool {
        matches!(self, Self::OnFailure)
    }

    /// Returns true if this is the Never policy.
    pub fn is_never(&self) -> bool {
        matches!(self, Self::Never)
    }

    /// Determines if restart should be attempted given exit status.
    ///
    /// Arguments:
    /// - `exit_code`: Agent exit code (0 = success, non-zero = failure)
    ///
    /// Returns `true` if restart should be attempted per this policy.
    pub fn should_restart(&self, exit_code: i32) -> bool {
        match self {
            Self::Always => true,
            Self::OnFailure => exit_code != 0,
            Self::Never => false,
        }
    }
}

/// Exponential backoff configuration for restart attempts.
///
/// Controls backoff timing when restarting failed agents. Prevents rapid
/// restart storms that can overwhelm the system or mask underlying issues.
///
/// # Fields
///
/// - `initial_delay_ms`: Initial delay before first restart attempt (milliseconds)
/// - `max_delay_ms`: Maximum delay between restart attempts (milliseconds)
/// - `multiplier`: Exponential multiplier for delay growth
/// - `max_retries`: Maximum number of restart attempts (0 = unlimited)
///
/// # Algorithm
///
/// delay = min(initial_delay * (multiplier ^ attempt), max_delay)
///
/// Example with initial_delay=1000, multiplier=2, max_delay=60000:
/// - Attempt 1: 1000ms delay
/// - Attempt 2: 2000ms delay
/// - Attempt 3: 4000ms delay
/// - Attempt 4: 8000ms delay
/// - Attempts 5+: capped at 60000ms
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Restart Policies
#[derive(Debug, Clone)]
pub struct BackoffConfig {
    /// Initial delay in milliseconds before first restart attempt.
    pub initial_delay_ms: u64,

    /// Maximum delay in milliseconds between restart attempts.
    pub max_delay_ms: u64,

    /// Exponential multiplier for delay calculation (e.g., 2 for doubling).
    pub multiplier: f64,

    /// Maximum number of restart attempts (0 = unlimited).
    pub max_retries: u32,
}

impl BackoffConfig {
    /// Creates a new backoff configuration with conservative defaults.
    ///
    /// Defaults:
    /// - initial_delay_ms: 1000 (1 second)
    /// - max_delay_ms: 60000 (1 minute)
    /// - multiplier: 2.0 (exponential doubling)
    /// - max_retries: 5
    ///
    /// Reference: Engineering Plan § Agent Lifecycle Management § Restart Policies
    pub fn new() -> Self {
        Self {
            initial_delay_ms: 1000,
            max_delay_ms: 60000,
            multiplier: 2.0,
            max_retries: 5,
        }
    }

    /// Creates a backoff configuration suitable for critical always-on services.
    ///
    /// More aggressive backoff with higher retry limit:
    /// - initial_delay_ms: 500
    /// - max_delay_ms: 120000 (2 minutes)
    /// - multiplier: 2.0
    /// - max_retries: 10
    pub fn for_critical_service() -> Self {
        Self {
            initial_delay_ms: 500,
            max_delay_ms: 120000,
            multiplier: 2.0,
            max_retries: 10,
        }
    }

    /// Creates a backoff configuration suitable for batch jobs.
    ///
    /// More conservative backoff with lower retry limit:
    /// - initial_delay_ms: 5000 (5 seconds)
    /// - max_delay_ms: 30000 (30 seconds)
    /// - multiplier: 1.5
    /// - max_retries: 3
    pub fn for_batch_job() -> Self {
        Self {
            initial_delay_ms: 5000,
            max_delay_ms: 30000,
            multiplier: 1.5,
            max_retries: 3,
        }
    }

    /// Sets the initial delay in milliseconds.
    pub fn with_initial_delay(mut self, delay_ms: u64) -> Self {
        self.initial_delay_ms = delay_ms;
        self
    }

    /// Sets the maximum delay in milliseconds.
    pub fn with_max_delay(mut self, delay_ms: u64) -> Self {
        self.max_delay_ms = delay_ms;
        self
    }

    /// Sets the exponential multiplier.
    pub fn with_multiplier(mut self, multiplier: f64) -> Self {
        self.multiplier = multiplier;
        self
    }

    /// Sets the maximum number of retries.
    pub fn with_max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    /// Calculates the backoff delay for the given attempt number.
    ///
    /// Arguments:
    /// - `attempt`: Zero-indexed attempt number (0, 1, 2, ...)
    ///
    /// Returns the delay in milliseconds before the next restart attempt.
    pub fn calculate_delay(&self, attempt: u32) -> u64 {
        let base_delay = self.initial_delay_ms as f64 * self.multiplier.powi(attempt as i32);
        (base_delay.min(self.max_delay_ms as f64)) as u64
    }

    /// Checks if we've exceeded the maximum retry limit.
    ///
    /// Arguments:
    /// - `attempt`: Zero-indexed attempt number (0, 1, 2, ...)
    ///
    /// Returns `true` if max_retries > 0 and we've exceeded the limit.
    pub fn is_retry_limit_exceeded(&self, attempt: u32) -> bool {
        self.max_retries > 0 && attempt >= self.max_retries
    }
}

impl Default for BackoffConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Combined restart decision for an agent.
///
/// Encapsulates both the restart policy and backoff configuration,
/// providing a complete restart strategy for an agent.
///
/// # Fields
///
/// - `policy`: The restart policy (Always, OnFailure, or Never)
/// - `backoff`: Backoff configuration for restart attempts
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Restart Policies
#[derive(Debug, Clone)]
pub struct RestartDecision {
    /// Restart policy for this agent.
    pub policy: RestartPolicy,

    /// Backoff configuration for restart attempts.
    pub backoff: BackoffConfig,
}

impl RestartDecision {
    /// Creates a new restart decision with the given policy and default backoff.
    pub fn new(policy: RestartPolicy) -> Self {
        Self {
            policy,
            backoff: BackoffConfig::new(),
        }
    }

    /// Creates an Always policy restart decision with critical service backoff.
    pub fn always_with_critical_backoff() -> Self {
        Self {
            policy: RestartPolicy::Always,
            backoff: BackoffConfig::for_critical_service(),
        }
    }

    /// Creates an OnFailure policy restart decision with batch job backoff.
    pub fn on_failure_with_batch_backoff() -> Self {
        Self {
            policy: RestartPolicy::OnFailure,
            backoff: BackoffConfig::for_batch_job(),
        }
    }

    /// Creates a Never policy restart decision (no restarts allowed).
    pub fn never() -> Self {
        Self {
            policy: RestartPolicy::Never,
            backoff: BackoffConfig::new(),
        }
    }

    /// Sets the backoff configuration.
    pub fn with_backoff(mut self, backoff: BackoffConfig) -> Self {
        self.backoff = backoff;
        self
    }

    /// Determines if restart should be attempted.
    ///
    /// Combines policy check with backoff limit validation.
    ///
    /// Arguments:
    /// - `exit_code`: Agent exit code
    /// - `attempt`: Current restart attempt number (zero-indexed)
    ///
    /// Returns `true` if restart should be attempted.
    pub fn should_restart(&self, exit_code: i32, attempt: u32) -> bool {
        self.policy.should_restart(exit_code) && !self.backoff.is_retry_limit_exceeded(attempt)
    }

    /// Gets the delay before the next restart attempt.
    pub fn get_delay_ms(&self, attempt: u32) -> u64 {
        self.backoff.calculate_delay(attempt)
    }
}

/// Always restart policy implementation.
///
/// Always attempts to restart the agent regardless of exit code.
/// Useful for long-running services that should always be available.
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Restart Policies
#[derive(Debug, Clone)]
pub struct AlwaysRestartPolicy {
    /// Backoff configuration for restart timing.
    pub backoff: BackoffConfig,
}

impl AlwaysRestartPolicy {
    /// Creates a new always-restart policy with default backoff.
    pub fn new() -> Self {
        Self {
            backoff: BackoffConfig::new(),
        }
    }

    /// Creates a new always-restart policy with specified backoff.
    pub fn with_backoff(backoff: BackoffConfig) -> Self {
        Self { backoff }
    }
}

impl Default for AlwaysRestartPolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl RestartPolicyEngine for AlwaysRestartPolicy {
    fn evaluate(&self, failure_info: &FailureInfo) -> Result<RestartDecision> {
        let delay_ms = self.backoff.calculate_delay(failure_info.failure_count);
        let should_restart = !self.backoff.is_retry_limit_exceeded(failure_info.failure_count);

        Ok(RestartDecision {
            should_restart,
            delay_ms,
            reason: if should_restart {
                alloc::format!("Always restart policy: attempt {}", failure_info.failure_count + 1)
            } else {
                "Restart limit exceeded".to_string()
            },
        })
    }
}

/// OnFailure restart policy implementation.
///
/// Only restarts agent if it exits with non-zero exit code (failure).
/// Useful for batch jobs and tasks that may complete successfully.
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Restart Policies
#[derive(Debug, Clone)]
pub struct OnFailureRestartPolicy {
    /// Backoff configuration for restart timing.
    pub backoff: BackoffConfig,
}

impl OnFailureRestartPolicy {
    /// Creates a new on-failure-restart policy with default backoff.
    pub fn new() -> Self {
        Self {
            backoff: BackoffConfig::new(),
        }
    }

    /// Creates a new on-failure-restart policy with specified backoff.
    pub fn with_backoff(backoff: BackoffConfig) -> Self {
        Self { backoff }
    }
}

impl Default for OnFailureRestartPolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl RestartPolicyEngine for OnFailureRestartPolicy {
    fn evaluate(&self, failure_info: &FailureInfo) -> Result<RestartDecision> {
        // Only restart if there's been an actual failure (count > 0)
        if failure_info.failure_count == 0 {
            return Ok(RestartDecision {
                should_restart: false,
                delay_ms: 0,
                reason: "Agent exited successfully".to_string(),
            });
        }

        let delay_ms = self.backoff.calculate_delay(failure_info.failure_count - 1);
        let should_restart = !self.backoff.is_retry_limit_exceeded(failure_info.failure_count - 1);

        Ok(RestartDecision {
            should_restart,
            delay_ms,
            reason: if should_restart {
                alloc::format!(
                    "On-failure restart: {} failures, waiting {}ms",
                    failure_info.failure_count, delay_ms
                )
            } else {
                "Restart limit exceeded".to_string()
            },
        })
    }
}

/// Never restart policy implementation.
///
/// Never automatically restarts the agent on failure.
/// Manual intervention is required to restart the agent.
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Restart Policies
#[derive(Debug, Clone)]
pub struct NeverRestartPolicy;

impl NeverRestartPolicy {
    /// Creates a new never-restart policy.
    pub fn new() -> Self {
        Self
    }
}

impl Default for NeverRestartPolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl RestartPolicyEngine for NeverRestartPolicy {
    fn evaluate(&self, _failure_info: &FailureInfo) -> Result<RestartDecision> {
        Ok(RestartDecision {
            should_restart: false,
            delay_ms: 0,
            reason: "Never restart policy: manual restart required".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;
use alloc::string::ToString;

    #[test]
    fn test_restart_policy_always() {
        let policy = RestartPolicy::Always;
        assert!(policy.permits_restart());
        assert!(policy.is_always());
        assert!(!policy.is_on_failure());
        assert!(!policy.is_never());
        assert!(policy.should_restart(0));
        assert!(policy.should_restart(1));
    }

    #[test]
    fn test_restart_policy_on_failure() {
        let policy = RestartPolicy::OnFailure;
        assert!(policy.permits_restart());
        assert!(!policy.is_always());
        assert!(policy.is_on_failure());
        assert!(!policy.is_never());
        assert!(!policy.should_restart(0));
        assert!(policy.should_restart(1));
    }

    #[test]
    fn test_restart_policy_never() {
        let policy = RestartPolicy::Never;
        assert!(!policy.permits_restart());
        assert!(!policy.is_always());
        assert!(!policy.is_on_failure());
        assert!(policy.is_never());
        assert!(!policy.should_restart(0));
        assert!(!policy.should_restart(1));
    }

    #[test]
    fn test_backoff_config_defaults() {
        let config = BackoffConfig::new();
        assert_eq!(config.initial_delay_ms, 1000);
        assert_eq!(config.max_delay_ms, 60000);
        assert_eq!(config.multiplier, 2.0);
        assert_eq!(config.max_retries, 5);
    }

    #[test]
    fn test_backoff_config_critical_service() {
        let config = BackoffConfig::for_critical_service();
        assert_eq!(config.initial_delay_ms, 500);
        assert_eq!(config.max_delay_ms, 120000);
        assert_eq!(config.max_retries, 10);
    }

    #[test]
    fn test_backoff_config_batch_job() {
        let config = BackoffConfig::for_batch_job();
        assert_eq!(config.initial_delay_ms, 5000);
        assert_eq!(config.max_delay_ms, 30000);
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_backoff_config_builder() {
        let config = BackoffConfig::new()
            .with_initial_delay(2000)
            .with_max_delay(120000)
            .with_multiplier(1.5)
            .with_max_retries(10);

        assert_eq!(config.initial_delay_ms, 2000);
        assert_eq!(config.max_delay_ms, 120000);
        assert_eq!(config.multiplier, 1.5);
        assert_eq!(config.max_retries, 10);
    }

    #[test]
    fn test_backoff_calculate_delay_exponential() {
        let config = BackoffConfig::new()
            .with_initial_delay(1000)
            .with_multiplier(2.0)
            .with_max_delay(60000);

        assert_eq!(config.calculate_delay(0), 1000); // 1000 * 2^0 = 1000
        assert_eq!(config.calculate_delay(1), 2000); // 1000 * 2^1 = 2000
        assert_eq!(config.calculate_delay(2), 4000); // 1000 * 2^2 = 4000
        assert_eq!(config.calculate_delay(3), 8000); // 1000 * 2^3 = 8000
        assert_eq!(config.calculate_delay(4), 16000); // 1000 * 2^4 = 16000
        assert_eq!(config.calculate_delay(5), 32000); // 1000 * 2^5 = 32000
        assert_eq!(config.calculate_delay(6), 60000); // capped at max_delay
    }

    #[test]
    fn test_backoff_retry_limit_exceeded() {
        let config = BackoffConfig::new().with_max_retries(5);
        assert!(!config.is_retry_limit_exceeded(0));
        assert!(!config.is_retry_limit_exceeded(4));
        assert!(config.is_retry_limit_exceeded(5));
        assert!(config.is_retry_limit_exceeded(10));
    }

    #[test]
    fn test_backoff_unlimited_retries() {
        let config = BackoffConfig::new().with_max_retries(0);
        assert!(!config.is_retry_limit_exceeded(0));
        assert!(!config.is_retry_limit_exceeded(100));
        assert!(!config.is_retry_limit_exceeded(1000));
    }

    #[test]
    fn test_restart_decision_new() {
        let decision = RestartDecision::new(RestartPolicy::Always);
        assert_eq!(decision.policy, RestartPolicy::Always);
        assert_eq!(decision.backoff.initial_delay_ms, 1000);
    }

    #[test]
    fn test_restart_decision_always_with_critical_backoff() {
        let decision = RestartDecision::always_with_critical_backoff();
        assert_eq!(decision.policy, RestartPolicy::Always);
        assert_eq!(decision.backoff.initial_delay_ms, 500);
        assert_eq!(decision.backoff.max_retries, 10);
    }

    #[test]
    fn test_restart_decision_on_failure_with_batch_backoff() {
        let decision = RestartDecision::on_failure_with_batch_backoff();
        assert_eq!(decision.policy, RestartPolicy::OnFailure);
        assert_eq!(decision.backoff.initial_delay_ms, 5000);
        assert_eq!(decision.backoff.max_retries, 3);
    }

    #[test]
    fn test_restart_decision_should_restart() {
        let decision = RestartDecision::new(RestartPolicy::Always);
        assert!(decision.should_restart(0, 0));
        assert!(decision.should_restart(1, 0));
        assert!(!decision.should_restart(0, 5)); // exceeds max_retries
    }

    #[test]
    fn test_restart_decision_get_delay_ms() {
        let decision = RestartDecision::new(RestartPolicy::Always);
        assert_eq!(decision.get_delay_ms(0), 1000);
        assert_eq!(decision.get_delay_ms(1), 2000);
        assert_eq!(decision.get_delay_ms(2), 4000);
    }

    // FailureInfo tests
    #[test]
    fn test_failure_info_new() {
        let info = FailureInfo::new(3, 5000, "Connection timeout");
        assert_eq!(info.failure_count, 3);
        assert_eq!(info.time_since_last_failure_ms, 5000);
        assert_eq!(info.reason, "Connection timeout");
    }

    // RestartDecisionOutcome tests
    #[test]
    fn test_restart_decision_outcome_new() {
        let outcome = RestartDecisionOutcome::new(true, 1000, "Testing");
        assert!(outcome.should_restart);
        assert_eq!(outcome.delay_ms, 1000);
        assert_eq!(outcome.reason, "Testing");
    }

    #[test]
    fn test_restart_decision_outcome_restart() {
        let outcome = RestartDecisionOutcome::restart(2000, "Restart now");
        assert!(outcome.should_restart);
        assert_eq!(outcome.delay_ms, 2000);
        assert_eq!(outcome.reason, "Restart now");
    }

    #[test]
    fn test_restart_decision_outcome_no_restart() {
        let outcome = RestartDecisionOutcome::no_restart("No restart needed");
        assert!(!outcome.should_restart);
        assert_eq!(outcome.delay_ms, 0);
        assert_eq!(outcome.reason, "No restart needed");
    }

    // RestartHistory tests
    #[test]
    fn test_restart_history_new() {
        let history = RestartHistory::new();
        assert_eq!(history.restart_count, 0);
        assert_eq!(history.total_failures, 0);
        assert_eq!(history.last_restart_timestamp, 0);
        assert_eq!(history.backoff_state, 0);
    }

    #[test]
    fn test_restart_history_record_restart() {
        let mut history = RestartHistory::new();
        history.record_restart(1000);

        assert_eq!(history.restart_count, 1);
        assert_eq!(history.last_restart_timestamp, 1000);
        assert_eq!(history.backoff_state, 1);
    }

    #[test]
    fn test_restart_history_record_failure() {
        let mut history = RestartHistory::new();
        history.record_failure();
        history.record_failure();

        assert_eq!(history.total_failures, 2);
    }

    #[test]
    fn test_restart_history_reset_backoff() {
        let mut history = RestartHistory::new();
        history.record_restart(1000);
        history.record_restart(2000);
        assert_eq!(history.backoff_state, 2);

        history.reset_backoff();
        assert_eq!(history.backoff_state, 0);
    }

    #[test]
    fn test_restart_history_current_attempt() {
        let mut history = RestartHistory::new();
        history.record_restart(1000);
        history.record_restart(2000);
        history.record_restart(3000);

        assert_eq!(history.current_attempt(), 3);
    }

    // AlwaysRestartPolicy tests
    #[test]
    fn test_always_restart_policy_evaluates_first_failure() {
        let policy = AlwaysRestartPolicy::new();
        let failure_info = FailureInfo::new(0, 1000, "Test failure");

        let result = policy.evaluate(&failure_info).expect("should succeed");
        assert!(result.should_restart);
        assert_eq!(result.delay_ms, 1000); // initial_delay
    }

    #[test]
    fn test_always_restart_policy_exponential_backoff() {
        let policy = AlwaysRestartPolicy::new();

        let failure1 = FailureInfo::new(0, 1000, "Failure 1");
        let result1 = policy.evaluate(&failure1).expect("should succeed");
        assert_eq!(result1.delay_ms, 1000);

        let failure2 = FailureInfo::new(1, 2000, "Failure 2");
        let result2 = policy.evaluate(&failure2).expect("should succeed");
        assert_eq!(result2.delay_ms, 2000);

        let failure3 = FailureInfo::new(2, 3000, "Failure 3");
        let result3 = policy.evaluate(&failure3).expect("should succeed");
        assert_eq!(result3.delay_ms, 4000);
    }

    #[test]
    fn test_always_restart_policy_respects_retry_limit() {
        let backoff = BackoffConfig::new().with_max_retries(3);
        let policy = AlwaysRestartPolicy::with_backoff(backoff);

        let failure3 = FailureInfo::new(2, 5000, "3rd failure");
        let result3 = policy.evaluate(&failure3).expect("should succeed");
        assert!(result3.should_restart); // attempt 3 (0-indexed: 2) is allowed

        let failure4 = FailureInfo::new(3, 6000, "4th failure");
        let result4 = policy.evaluate(&failure4).expect("should succeed");
        assert!(!result4.should_restart); // attempt 4 exceeds limit
    }

    // OnFailureRestartPolicy tests
    #[test]
    fn test_on_failure_restart_policy_successful_exit() {
        let policy = OnFailureRestartPolicy::new();
        let failure_info = FailureInfo::new(0, 0, "Successful exit");

        let result = policy.evaluate(&failure_info).expect("should succeed");
        assert!(!result.should_restart);
        assert!(result.reason.contains("successfully"));
    }

    #[test]
    fn test_on_failure_restart_policy_first_failure() {
        let policy = OnFailureRestartPolicy::new();
        let failure_info = FailureInfo::new(1, 1000, "First failure");

        let result = policy.evaluate(&failure_info).expect("should succeed");
        assert!(result.should_restart);
        assert_eq!(result.delay_ms, 1000); // initial_delay for attempt 0
    }

    #[test]
    fn test_on_failure_restart_policy_multiple_failures() {
        let policy = OnFailureRestartPolicy::new();

        let failure1 = FailureInfo::new(1, 1000, "Failure 1");
        let result1 = policy.evaluate(&failure1).expect("should succeed");
        assert!(result1.should_restart);
        assert_eq!(result1.delay_ms, 1000);

        let failure2 = FailureInfo::new(2, 2000, "Failure 2");
        let result2 = policy.evaluate(&failure2).expect("should succeed");
        assert!(result2.should_restart);
        assert_eq!(result2.delay_ms, 2000);
    }

    #[test]
    fn test_on_failure_restart_policy_respects_retry_limit() {
        let backoff = BackoffConfig::new().with_max_retries(2);
        let policy = OnFailureRestartPolicy::with_backoff(backoff);

        let failure2 = FailureInfo::new(2, 2000, "2nd failure");
        let result2 = policy.evaluate(&failure2).expect("should succeed");
        assert!(result2.should_restart); // attempt 1 is allowed

        let failure3 = FailureInfo::new(3, 3000, "3rd failure");
        let result3 = policy.evaluate(&failure3).expect("should succeed");
        assert!(!result3.should_restart); // attempt 2 exceeds limit
    }

    // NeverRestartPolicy tests
    #[test]
    fn test_never_restart_policy() {
        let policy = NeverRestartPolicy::new();
        let failure_info = FailureInfo::new(1, 1000, "Any failure");

        let result = policy.evaluate(&failure_info).expect("should succeed");
        assert!(!result.should_restart);
        assert!(result.reason.contains("Never restart"));
    }

    #[test]
    fn test_never_restart_policy_always_refuses() {
        let policy = NeverRestartPolicy::new();

        for count in 0..10 {
            let failure_info = FailureInfo::new(count, 1000, "Failure");
            let result = policy.evaluate(&failure_info).expect("should succeed");
            assert!(!result.should_restart);
        }
    }
}
