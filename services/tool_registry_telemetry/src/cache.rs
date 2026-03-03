// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Response caching configuration for tool bindings.
//!
//! Defines caching behavior for tool output including TTL, freshness policies,
//! and cache key strategies.
//!
//! See Engineering Plan § 2.11.5: Response Caching.

use core::fmt;

/// Freshness policy for cached tool responses.
///
/// Determines when cached responses are considered fresh and can be returned.
///
/// See Engineering Plan § 2.11.5: Response Caching.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FreshnessPolicy {
    /// Strict freshness validation.
    ///
    /// Cached response is only returned if within TTL and no upstream invalidation occurred.
    /// Ensures high consistency at potential latency cost.
    Strict,

    /// Relaxed freshness validation.
    ///
    /// Cached response is returned even if slightly stale (within grace period).
    /// Balances consistency with performance.
    Relaxed,

    /// Best-effort freshness.
    ///
    /// Cached response is returned regardless of staleness if available.
    /// Prioritizes availability and performance over consistency.
    BestEffort,
}

impl fmt::Display for FreshnessPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FreshnessPolicy::Strict => write!(f, "Strict"),
            FreshnessPolicy::Relaxed => write!(f, "Relaxed"),
            FreshnessPolicy::BestEffort => write!(f, "BestEffort"),
        }
    }
}

/// Cache key strategy for tool requests.
///
/// Determines how cache entries are keyed and matched against requests.
///
/// See Engineering Plan § 2.11.5: Response Caching.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CacheKeyStrategy {
    /// Key based on input data hash.
    ///
    /// Cache key is deterministic hash of all input parameters.
    /// Works well for pure functions with deterministic outputs.
    InputHash,

    /// Key based on semantic input interpretation.
    ///
    /// Cache key is derived from semantic meaning rather than exact input bytes.
    /// Two semantically equivalent inputs produce same cache key.
    /// Enables smarter cache reuse.
    Semantic,

    /// Custom cache key computation.
    ///
    /// Tool defines custom logic for cache key generation.
    /// Suitable for complex tools with domain-specific caching rules.
    Custom,
}

impl fmt::Display for CacheKeyStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CacheKeyStrategy::InputHash => write!(f, "InputHash"),
            CacheKeyStrategy::Semantic => write!(f, "Semantic"),
            CacheKeyStrategy::Custom => write!(f, "Custom"),
        }
    }
}

/// Response caching configuration for a tool binding.
///
/// Configures whether, how, and when tool responses are cached.
///
/// See Engineering Plan § 2.11.5: Response Caching.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CacheConfig {
    /// Whether caching is enabled for this tool.
    pub enabled: bool,

    /// Time-to-live in milliseconds.
    ///
    /// Cached response is valid for this duration after creation.
    /// After TTL expires, response is considered stale.
    /// If 0, cached responses never expire (infinite TTL).
    pub ttl_ms: u64,

    /// Freshness policy for this cache.
    ///
    /// Determines when cached responses are considered fresh.
    pub freshness_policy: FreshnessPolicy,

    /// Maximum number of entries in cache.
    ///
    /// When cache exceeds this size, oldest entries are evicted.
    /// If 0, cache size is unbounded.
    pub max_entries: u32,

    /// Cache key strategy for this tool.
    ///
    /// Determines how cache lookups are performed.
    pub cache_key_strategy: CacheKeyStrategy,
}

impl CacheConfig {
    /// Creates a cache-disabled configuration.
    pub fn disabled() -> Self {
        CacheConfig {
            enabled: false,
            ttl_ms: 0,
            freshness_policy: FreshnessPolicy::Strict,
            max_entries: 0,
            cache_key_strategy: CacheKeyStrategy::InputHash,
        }
    }

    /// Creates a short-lived cache configuration (60 seconds, strict).
    ///
    /// Suitable for frequently changing data.
    pub fn short_lived() -> Self {
        CacheConfig {
            enabled: true,
            ttl_ms: 60_000,
            freshness_policy: FreshnessPolicy::Strict,
            max_entries: 100,
            cache_key_strategy: CacheKeyStrategy::InputHash,
        }
    }

    /// Creates a medium-lived cache configuration (1 hour, relaxed).
    ///
    /// Suitable for moderately changing data.
    pub fn medium_lived() -> Self {
        CacheConfig {
            enabled: true,
            ttl_ms: 3_600_000,
            freshness_policy: FreshnessPolicy::Relaxed,
            max_entries: 500,
            cache_key_strategy: CacheKeyStrategy::InputHash,
        }
    }

    /// Creates a long-lived cache configuration (1 day, best-effort).
    ///
    /// Suitable for stable, infrequently changing data.
    pub fn long_lived() -> Self {
        CacheConfig {
            enabled: true,
            ttl_ms: 86_400_000,
            freshness_policy: FreshnessPolicy::BestEffort,
            max_entries: 1000,
            cache_key_strategy: CacheKeyStrategy::InputHash,
        }
    }

    /// Returns true if caching is enabled and configured.
    pub fn is_enabled(&self) -> bool {
        self.enabled && (self.ttl_ms > 0 || self.ttl_ms == 0) // Always true if enabled
    }

    /// Returns true if cache is very permissive (long TTL, best-effort).
    pub fn is_permissive(&self) -> bool {
        self.enabled
            && self.ttl_ms >= 3_600_000 // At least 1 hour
            && self.freshness_policy == FreshnessPolicy::BestEffort
    }

    /// Returns true if cache is very restrictive (short TTL, strict).
    pub fn is_restrictive(&self) -> bool {
        self.enabled
            && self.ttl_ms <= 60_000 // At most 1 minute
            && self.freshness_policy == FreshnessPolicy::Strict
    }

    /// Effective TTL considering freshness policy.
    ///
    /// Returns the actual time a cached entry should be considered fresh.
    /// For Relaxed policy, this is increased by grace period (10% of TTL).
    /// For BestEffort, returns u64::MAX (infinite).
    pub fn effective_ttl_ms(&self) -> u64 {
        if !self.enabled {
            return 0;
        }

        match self.freshness_policy {
            FreshnessPolicy::Strict => self.ttl_ms,
            FreshnessPolicy::Relaxed => {
                let grace = self.ttl_ms.saturating_div(10);
                self.ttl_ms.saturating_add(grace)
            }
            FreshnessPolicy::BestEffort => u64::MAX,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_freshness_policy_display() {
        assert_eq!(FreshnessPolicy::Strict.to_string(), "Strict");
        assert_eq!(FreshnessPolicy::Relaxed.to_string(), "Relaxed");
        assert_eq!(FreshnessPolicy::BestEffort.to_string(), "BestEffort");
    }

    #[test]
    fn test_cache_key_strategy_display() {
        assert_eq!(CacheKeyStrategy::InputHash.to_string(), "InputHash");
        assert_eq!(CacheKeyStrategy::Semantic.to_string(), "Semantic");
        assert_eq!(CacheKeyStrategy::Custom.to_string(), "Custom");
    }

    #[test]
    fn test_cache_config_disabled() {
        let config = CacheConfig::disabled();
        assert!(!config.enabled);
        assert_eq!(config.ttl_ms, 0);
        assert_eq!(config.max_entries, 0);
    }

    #[test]
    fn test_cache_config_short_lived() {
        let config = CacheConfig::short_lived();
        assert!(config.enabled);
        assert_eq!(config.ttl_ms, 60_000);
        assert_eq!(config.freshness_policy, FreshnessPolicy::Strict);
        assert_eq!(config.max_entries, 100);
    }

    #[test]
    fn test_cache_config_medium_lived() {
        let config = CacheConfig::medium_lived();
        assert!(config.enabled);
        assert_eq!(config.ttl_ms, 3_600_000);
        assert_eq!(config.freshness_policy, FreshnessPolicy::Relaxed);
        assert_eq!(config.max_entries, 500);
    }

    #[test]
    fn test_cache_config_long_lived() {
        let config = CacheConfig::long_lived();
        assert!(config.enabled);
        assert_eq!(config.ttl_ms, 86_400_000);
        assert_eq!(config.freshness_policy, FreshnessPolicy::BestEffort);
        assert_eq!(config.max_entries, 1000);
    }

    #[test]
    fn test_is_enabled() {
        let enabled = CacheConfig::short_lived();
        assert!(enabled.is_enabled());

        let disabled = CacheConfig::disabled();
        assert!(!disabled.is_enabled());
    }

    #[test]
    fn test_is_permissive() {
        assert!(CacheConfig::long_lived().is_permissive());
        assert!(!CacheConfig::short_lived().is_permissive());
        assert!(!CacheConfig::medium_lived().is_permissive());
    }

    #[test]
    fn test_is_restrictive() {
        assert!(CacheConfig::short_lived().is_restrictive());
        assert!(!CacheConfig::long_lived().is_restrictive());
        assert!(!CacheConfig::medium_lived().is_restrictive());
    }

    #[test]
    fn test_effective_ttl_strict() {
        let config = CacheConfig::short_lived();
        assert_eq!(config.effective_ttl_ms(), 60_000);
    }

    #[test]
    fn test_effective_ttl_relaxed() {
        let config = CacheConfig::medium_lived();
        // 3_600_000 + 10% grace = 3_600_000 + 360_000 = 3_960_000
        assert_eq!(config.effective_ttl_ms(), 3_960_000);
    }

    #[test]
    fn test_effective_ttl_best_effort() {
        let config = CacheConfig::long_lived();
        assert_eq!(config.effective_ttl_ms(), u64::MAX);
    }

    #[test]
    fn test_effective_ttl_disabled() {
        let config = CacheConfig::disabled();
        assert_eq!(config.effective_ttl_ms(), 0);
    }

    #[test]
    fn test_cache_config_equality() {
        let c1 = CacheConfig::short_lived();
        let c2 = CacheConfig::short_lived();
        assert_eq!(c1, c2);

        let c3 = CacheConfig::long_lived();
        assert_ne!(c1, c3);
    }

    #[test]
    fn test_freshness_policy_equality() {
        assert_eq!(FreshnessPolicy::Strict, FreshnessPolicy::Strict);
        assert_ne!(FreshnessPolicy::Strict, FreshnessPolicy::Relaxed);
    }

    #[test]
    fn test_cache_key_strategy_equality() {
        assert_eq!(CacheKeyStrategy::InputHash, CacheKeyStrategy::InputHash);
        assert_ne!(CacheKeyStrategy::InputHash, CacheKeyStrategy::Semantic);
    }

    #[test]
    fn test_effective_ttl_with_zero_ttl() {
        // Zero TTL with Relaxed should handle gracefully
        let mut config = CacheConfig::short_lived();
        config.ttl_ms = 0;
        config.freshness_policy = FreshnessPolicy::Relaxed;
        assert_eq!(config.effective_ttl_ms(), 0);
    }

    #[test]
    fn test_cache_key_strategy_hash() {
        use core::collections::hash_map::DefaultHasher;
        use core::hash::{Hash, Hasher};
use alloc::string::ToString;

        let mut h1 = DefaultHasher::new();
        CacheKeyStrategy::InputHash.hash(&mut h1);
        let hash1 = h1.finish();

        let mut h2 = DefaultHasher::new();
        CacheKeyStrategy::InputHash.hash(&mut h2);
        let hash2 = h2.finish();

        assert_eq!(hash1, hash2);
    }
}
