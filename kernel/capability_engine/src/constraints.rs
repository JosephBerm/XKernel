// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Capability constraints: time bounds, rate limits, and volume limits.
//!
//! See Engineering Plan § 3.1.5: Time-Bounded Validity and § 3.1.6: Rate & Volume Constraints.

use alloc::string::{String, ToString};
use core::fmt::{self, Debug, Display};

use crate::error::CapError;

/// A timestamp representing a point in time (nanoseconds since Unix epoch).
///
/// This is used for time-based constraints and capability expiration.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Timestamp(pub u64);

impl Timestamp {
    /// Creates a new timestamp from nanoseconds since Unix epoch.
    pub const fn new(nanos: u64) -> Self {
        Timestamp(nanos)
    }

    /// Returns the timestamp as nanoseconds since Unix epoch.
    pub const fn nanos(&self) -> u64 {
        self.0
    }

    /// Checks if this timestamp is in the future relative to `now`.
    pub const fn is_future_of(&self, now: Timestamp) -> bool {
        self.0 > now.0
    }

    /// Checks if this timestamp is in the past relative to `now`.
    pub const fn is_past_of(&self, now: Timestamp) -> bool {
        self.0 < now.0
    }
}

impl Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}ns", self.0)
    }
}

/// A time-bounded validity constraint.
///
/// See Engineering Plan § 3.1.5: Time-Bounded Validity.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TimeBound {
    /// When the capability becomes valid (inclusive).
    pub start_timestamp: Timestamp,

    /// When the capability expires (exclusive).
    pub expiry_timestamp: Timestamp,
}

impl TimeBound {
    /// Creates a new time bound.
    pub const fn new(start: Timestamp, expiry: Timestamp) -> Self {
        TimeBound {
            start_timestamp: start,
            expiry_timestamp: expiry,
        }
    }

    /// Checks if a capability with this time bound is valid at the given timestamp.
    pub const fn is_valid_at(&self, now: Timestamp) -> bool {
        now.is_future_of(self.start_timestamp) && now.0 < self.expiry_timestamp.0
    }

    /// Returns the duration of the time bound in nanoseconds.
    pub const fn duration_nanos(&self) -> u64 {
        self.expiry_timestamp.0 - self.start_timestamp.0
    }
}

impl Display for TimeBound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TimeBound[{} - {}]",
            self.start_timestamp, self.expiry_timestamp
        )
    }
}

/// A rate limit constraint.
///
/// See Engineering Plan § 3.1.6: Rate & Volume Constraints.
/// Limits the number of operations that can be performed within a time period.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct RateLimit {
    /// Maximum number of operations per period.
    pub max_operations_per_period: u32,

    /// Duration of the period in nanoseconds.
    pub period_duration_nanos: u64,
}

impl RateLimit {
    /// Creates a new rate limit.
    pub const fn new(max_ops: u32, period_nanos: u64) -> Self {
        RateLimit {
            max_operations_per_period: max_ops,
            period_duration_nanos: period_nanos,
        }
    }

    /// Checks if the current count exceeds the rate limit.
    pub const fn check_rate(&self, current_count: u32) -> bool {
        current_count < self.max_operations_per_period
    }

    /// Calculates the allowed rate in operations per second.
    pub fn ops_per_second(&self) -> f64 {
        let secs = self.period_duration_nanos as f64 / 1_000_000_000.0;
        if secs > 0.0 {
            self.max_operations_per_period as f64 / secs
        } else {
            0.0
        }
    }
}

impl Display for RateLimit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "RateLimit[{} ops per {}ns]",
            self.max_operations_per_period, self.period_duration_nanos
        )
    }
}

/// A data volume limit constraint.
///
/// See Engineering Plan § 3.1.6: Rate & Volume Constraints.
/// Limits the total amount of data (in bytes) that can be transferred within a time period.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DataVolumeLimit {
    /// Maximum bytes per period.
    pub max_bytes_per_period: u64,

    /// Duration of the period in nanoseconds.
    pub period_duration_nanos: u64,
}

impl DataVolumeLimit {
    /// Creates a new data volume limit.
    pub const fn new(max_bytes: u64, period_nanos: u64) -> Self {
        DataVolumeLimit {
            max_bytes_per_period: max_bytes,
            period_duration_nanos: period_nanos,
        }
    }

    /// Checks if the current bytes exceed the volume limit.
    pub const fn check_volume(&self, current_bytes: u64) -> bool {
        current_bytes < self.max_bytes_per_period
    }

    /// Calculates the throughput limit in bytes per second.
    pub fn bytes_per_second(&self) -> f64 {
        let secs = self.period_duration_nanos as f64 / 1_000_000_000.0;
        if secs > 0.0 {
            self.max_bytes_per_period as f64 / secs
        } else {
            0.0
        }
    }
}

impl Display for DataVolumeLimit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "DataVolumeLimit[{} bytes per {}ns]",
            self.max_bytes_per_period, self.period_duration_nanos
        )
    }
}

/// A delegation depth limit constraint.
///
/// See Engineering Plan § 3.1.7: Delegation & Attenuation.
/// Limits how deeply a capability can be delegated (sub-delegated).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ChainDepthLimit {
    /// Maximum delegation depth (0 = no delegation allowed).
    pub max_delegation_depth: u32,
}

impl ChainDepthLimit {
    /// Creates a new chain depth limit.
    pub const fn new(max_depth: u32) -> Self {
        ChainDepthLimit {
            max_delegation_depth: max_depth,
        }
    }

    /// Checks if the current depth exceeds the limit.
    pub const fn can_delegate(&self, current_depth: u32) -> bool {
        current_depth < self.max_delegation_depth
    }
}

impl Display for ChainDepthLimit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ChainDepthLimit[max={}]", self.max_delegation_depth)
    }
}

/// All constraints that can be applied to a capability.
///
/// See Engineering Plan § 3.1.5 & § 3.1.6.
/// Constraints compose: a capability has at most one of each constraint type.
#[derive(Clone, Debug, Default)]
pub struct CapConstraints {
    /// Optional time bound on capability validity.
    pub time_bound: Option<TimeBound>,

    /// Optional rate limit on operations.
    pub rate_limited: Option<RateLimit>,

    /// Optional data volume limit.
    pub data_volume_limited: Option<DataVolumeLimit>,

    /// Optional delegation depth limit.
    pub chain_depth_limited: Option<ChainDepthLimit>,
}

impl CapConstraints {
    /// Creates an empty constraint set (no constraints).
    pub const fn new() -> Self {
        CapConstraints {
            time_bound: None,
            rate_limited: None,
            data_volume_limited: None,
            chain_depth_limited: None,
        }
    }

    /// Checks if all constraints are satisfied at a given timestamp.
    pub fn is_valid_at(&self, now: Timestamp) -> Result<(), CapError> {
        if let Some(tb) = self.time_bound {
            if !tb.is_valid_at(now) {
                return Err(CapError::Expired(format!("time bound expired at {}", now)));
            }
        }
        Ok(())
    }

    /// Checks if a rate limit is not yet exceeded.
    pub fn check_rate(&self, current_count: u32) -> Result<(), CapError> {
        if let Some(rl) = self.rate_limited {
            if !rl.check_rate(current_count) {
                return Err(CapError::RateLimitExceeded(format!(
                    "limit {} ops per {:?}, have {}",
                    rl.max_operations_per_period, rl.period_duration_nanos, current_count
                )));
            }
        }
        Ok(())
    }

    /// Checks if a data volume limit is not yet exceeded.
    pub fn check_volume(&self, current_bytes: u64) -> Result<(), CapError> {
        if let Some(dvl) = self.data_volume_limited {
            if !dvl.check_volume(current_bytes) {
                return Err(CapError::VolumeExceeded(format!(
                    "limit {} bytes per {:?}, have {}",
                    dvl.max_bytes_per_period, dvl.period_duration_nanos, current_bytes
                )));
            }
        }
        Ok(())
    }

    /// Checks if a delegation depth is within bounds.
    pub fn can_delegate(&self, current_depth: u32) -> Result<(), CapError> {
        if let Some(cdl) = self.chain_depth_limited {
            if !cdl.can_delegate(current_depth) {
                return Err(CapError::DepthExceeded(format!(
                    "limit depth {} but have {}",
                    cdl.max_delegation_depth, current_depth
                )));
            }
        }
        Ok(())
    }

    /// Checks all constraints at once.
    pub fn check_all(
        &self,
        now: Timestamp,
        rate_count: u32,
        volume_bytes: u64,
        depth: u32,
    ) -> Result<(), CapError> {
        self.is_valid_at(now)?;
        self.check_rate(rate_count)?;
        self.check_volume(volume_bytes)?;
        self.can_delegate(depth)?;
        Ok(())
    }
}

impl Display for CapConstraints {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CapConstraints{{")?;
        let mut first = true;

        if let Some(tb) = self.time_bound {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, "{}", tb)?;
            first = false;
        }

        if let Some(rl) = self.rate_limited {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, "{}", rl)?;
            first = false;
        }

        if let Some(dvl) = self.data_volume_limited {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, "{}", dvl)?;
            first = false;
        }

        if let Some(cdl) = self.chain_depth_limited {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, "{}", cdl)?;
            first = false;
        }

        write!(f, "}}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;
use alloc::string::String;
use alloc::string::ToString;

    #[test]
    fn test_timestamp_creation() {
        let ts = Timestamp::new(1000);
        assert_eq!(ts.nanos(), 1000);
    }

    #[test]
    fn test_timestamp_ordering() {
        let ts1 = Timestamp::new(100);
        let ts2 = Timestamp::new(200);
        assert!(ts1.is_past_of(ts2));
        assert!(ts2.is_future_of(ts1));
    }

    #[test]
    fn test_time_bound_creation() {
        let start = Timestamp::new(1000);
        let expiry = Timestamp::new(2000);
        let tb = TimeBound::new(start, expiry);
        assert_eq!(tb.duration_nanos(), 1000);
    }

    #[test]
    fn test_time_bound_is_valid_at() {
        let start = Timestamp::new(1000);
        let expiry = Timestamp::new(2000);
        let tb = TimeBound::new(start, expiry);

        assert!(!tb.is_valid_at(Timestamp::new(999)));
        assert!(tb.is_valid_at(Timestamp::new(1500)));
        assert!(!tb.is_valid_at(Timestamp::new(2000)));
    }

    #[test]
    fn test_rate_limit_creation() {
        let rl = RateLimit::new(100, 1_000_000_000);
        assert!(rl.check_rate(50));
        assert!(!rl.check_rate(100));
    }

    #[test]
    fn test_rate_limit_ops_per_second() {
        let rl = RateLimit::new(1000, 1_000_000_000); // 1000 ops per second
        let ops = rl.ops_per_second();
        assert!((ops - 1000.0).abs() < 0.1);
    }

    #[test]
    fn test_data_volume_limit_creation() {
        let dvl = DataVolumeLimit::new(1_000_000, 1_000_000_000);
        assert!(dvl.check_volume(500_000));
        assert!(!dvl.check_volume(1_000_000));
    }

    #[test]
    fn test_data_volume_limit_bytes_per_second() {
        let dvl = DataVolumeLimit::new(1_000_000, 1_000_000_000); // 1MB per second
        let bps = dvl.bytes_per_second();
        assert!((bps - 1_000_000.0).abs() < 0.1);
    }

    #[test]
    fn test_chain_depth_limit_can_delegate() {
        let cdl = ChainDepthLimit::new(3);
        assert!(cdl.can_delegate(0));
        assert!(cdl.can_delegate(2));
        assert!(!cdl.can_delegate(3));
    }

    #[test]
    fn test_cap_constraints_empty() {
        let constraints = CapConstraints::new();
        let now = Timestamp::new(1000);
        assert!(constraints.is_valid_at(now).is_ok());
        assert!(constraints.check_rate(1000).is_ok());
        assert!(constraints.check_volume(1_000_000).is_ok());
        assert!(constraints.can_delegate(10).is_ok());
    }

    #[test]
    fn test_cap_constraints_with_time_bound() {
        let start = Timestamp::new(1000);
        let expiry = Timestamp::new(2000);
        let mut constraints = CapConstraints::new();
        constraints.time_bound = Some(TimeBound::new(start, expiry));

        assert!(constraints.is_valid_at(Timestamp::new(1500)).is_ok());
        assert!(constraints.is_valid_at(Timestamp::new(2500)).is_err());
    }

    #[test]
    fn test_cap_constraints_with_rate_limit() {
        let mut constraints = CapConstraints::new();
        constraints.rate_limited = Some(RateLimit::new(100, 1_000_000_000));

        assert!(constraints.check_rate(50).is_ok());
        assert!(constraints.check_rate(100).is_err());
    }

    #[test]
    fn test_cap_constraints_with_volume_limit() {
        let mut constraints = CapConstraints::new();
        constraints.data_volume_limited = Some(DataVolumeLimit::new(1_000_000, 1_000_000_000));

        assert!(constraints.check_volume(500_000).is_ok());
        assert!(constraints.check_volume(1_000_000).is_err());
    }

    #[test]
    fn test_cap_constraints_check_all() {
        let mut constraints = CapConstraints::new();
        constraints.time_bound = Some(TimeBound::new(Timestamp::new(1000), Timestamp::new(2000)));
        constraints.rate_limited = Some(RateLimit::new(100, 1_000_000_000));
        constraints.data_volume_limited = Some(DataVolumeLimit::new(1_000_000, 1_000_000_000));
        constraints.chain_depth_limited = Some(ChainDepthLimit::new(3));

        let result = constraints.check_all(
            Timestamp::new(1500),
            50,
            500_000,
            2,
        );
        assert!(result.is_ok());

        let result = constraints.check_all(
            Timestamp::new(2500),
            50,
            500_000,
            2,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_cap_constraints_display() {
        let mut constraints = CapConstraints::new();
        constraints.time_bound = Some(TimeBound::new(Timestamp::new(1000), Timestamp::new(2000)));
        let display = constraints.to_string();
        assert!(display.contains("CapConstraints"));
    }
}
