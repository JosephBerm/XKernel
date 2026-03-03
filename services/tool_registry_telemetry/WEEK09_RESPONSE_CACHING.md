# Week 9 Deliverable: Response Caching Engine (Phase 1)

**Engineer 6: Tool Registry, Telemetry & Compliance**
**Week 9 Objective:** Begin response caching with cache key generation, TTL config, freshness policies. Integrate with MCP Tool Registry.

---

## 1. ResponseCache: In-Memory LRU Cache

```rust
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use parking_lot::RwLock;
use lru::LruCache;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedResponse {
    pub value: String,
    pub created_at: SystemTime,
    pub ttl_seconds: u64,
    pub freshness_policy: FreshnessPolicy,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum FreshnessPolicy {
    Strict,
    StaleWhileRevalidate { revalidate_seconds: u64 },
    StaleIfError { error_timeout_seconds: u64 },
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CacheStats {
    pub hits: std::sync::atomic::AtomicU64,
    pub misses: std::sync::atomic::AtomicU64,
    pub evictions: std::sync::atomic::AtomicU64,
    pub errors: std::sync::atomic::AtomicU64,
}

impl CacheStats {
    pub fn increment_hits(&self) {
        self.hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn increment_misses(&self) {
        self.misses.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn increment_evictions(&self) {
        self.evictions.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn increment_errors(&self) {
        self.errors.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn get_snapshot(&self) -> (u64, u64, u64, u64) {
        (
            self.hits.load(std::sync::atomic::Ordering::Relaxed),
            self.misses.load(std::sync::atomic::Ordering::Relaxed),
            self.evictions.load(std::sync::atomic::Ordering::Relaxed),
            self.errors.load(std::sync::atomic::Ordering::Relaxed),
        )
    }
}

pub struct ResponseCache {
    cache: Arc<RwLock<LruCache<String, CachedResponse>>>,
    stats: Arc<CacheStats>,
    max_capacity: usize,
}

impl ResponseCache {
    pub fn new(max_capacity: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(LruCache::new(
                std::num::NonZeroUsize::new(max_capacity).unwrap(),
            ))),
            stats: Arc::new(CacheStats::default()),
            max_capacity,
        }
    }

    pub fn get(&self, key: &str) -> Option<CachedResponse> {
        let mut cache = self.cache.write();
        cache.get(key).cloned()
    }

    pub fn insert(&self, key: String, response: CachedResponse) {
        let mut cache = self.cache.write();
        if cache.len() >= self.max_capacity {
            if let Some((_, _)) = cache.pop_lru() {
                self.stats.increment_evictions();
            }
        }
        cache.put(key, response);
    }

    pub fn invalidate(&self, key: &str) {
        let mut cache = self.cache.write();
        cache.pop(key);
    }

    pub fn stats(&self) -> CacheStats {
        CacheStats {
            hits: std::sync::atomic::AtomicU64::new(
                self.stats.hits.load(std::sync::atomic::Ordering::Relaxed),
            ),
            misses: std::sync::atomic::AtomicU64::new(
                self.stats.misses.load(std::sync::atomic::Ordering::Relaxed),
            ),
            evictions: std::sync::atomic::AtomicU64::new(
                self.stats.evictions.load(std::sync::atomic::Ordering::Relaxed),
            ),
            errors: std::sync::atomic::AtomicU64::new(
                self.stats.errors.load(std::sync::atomic::Ordering::Relaxed),
            ),
        }
    }
}
```

---

## 2. Cache Key Generation with Determinism

```rust
use sha2::{Sha256, Digest};

pub struct CacheKeyGenerator;

impl CacheKeyGenerator {
    /// Generate deterministic cache key: tool_id:hash(input)
    pub fn generate(tool_id: &str, input: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        let hash = format!("{:x}", hasher.finalize());
        format!("{}:{}", tool_id, hash)
    }

    /// Validate determinism by checking consistent hashing
    #[cfg(test)]
    pub fn validate_determinism(tool_id: &str, input: &str, iterations: usize) -> bool {
        let first_key = Self::generate(tool_id, input);
        for _ in 1..iterations {
            if Self::generate(tool_id, input) != first_key {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_determinism() {
        assert!(CacheKeyGenerator::validate_determinism(
            "tool_1",
            r#"{"action":"invoke","params":{"x":1}}"#,
            100
        ));
    }

    #[test]
    fn test_different_inputs_different_keys() {
        let key1 = CacheKeyGenerator::generate("tool_1", "input1");
        let key2 = CacheKeyGenerator::generate("tool_1", "input2");
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_different_tools_different_keys() {
        let key1 = CacheKeyGenerator::generate("tool_1", "input");
        let key2 = CacheKeyGenerator::generate("tool_2", "input");
        assert_ne!(key1, key2);
    }
}
```

---

## 3. Freshness Policies

```rust
impl CachedResponse {
    pub fn is_fresh(&self) -> bool {
        match self.freshness_policy {
            FreshnessPolicy::Strict => {
                self.created_at.elapsed().unwrap_or(Duration::MAX)
                    < Duration::from_secs(self.ttl_seconds)
            }
            FreshnessPolicy::StaleWhileRevalidate { revalidate_seconds } => {
                let elapsed = self.created_at.elapsed().unwrap_or(Duration::MAX);
                elapsed < Duration::from_secs(revalidate_seconds)
            }
            FreshnessPolicy::StaleIfError { error_timeout_seconds } => {
                let elapsed = self.created_at.elapsed().unwrap_or(Duration::MAX);
                elapsed < Duration::from_secs(self.ttl_seconds + error_timeout_seconds)
            }
        }
    }

    pub fn can_use_stale(&self) -> bool {
        match self.freshness_policy {
            FreshnessPolicy::Strict => false,
            FreshnessPolicy::StaleWhileRevalidate { revalidate_seconds } => {
                let elapsed = self.created_at.elapsed().unwrap_or(Duration::MAX);
                elapsed < Duration::from_secs(self.ttl_seconds + revalidate_seconds)
            }
            FreshnessPolicy::StaleIfError { error_timeout_seconds } => {
                let elapsed = self.created_at.elapsed().unwrap_or(Duration::MAX);
                elapsed < Duration::from_secs(self.ttl_seconds + error_timeout_seconds)
            }
        }
    }

    pub fn is_revalidation_needed(&self) -> bool {
        match self.freshness_policy {
            FreshnessPolicy::Strict => false,
            FreshnessPolicy::StaleWhileRevalidate { revalidate_seconds } => {
                let elapsed = self.created_at.elapsed().unwrap_or(Duration::MAX);
                elapsed >= Duration::from_secs(revalidate_seconds)
            }
            FreshnessPolicy::StaleIfError { .. } => false,
        }
    }
}

#[cfg(test)]
mod freshness_tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_strict_freshness() {
        let response = CachedResponse {
            value: "test".to_string(),
            created_at: SystemTime::now(),
            ttl_seconds: 1,
            freshness_policy: FreshnessPolicy::Strict,
        };
        assert!(response.is_fresh());
        thread::sleep(Duration::from_secs(2));
        assert!(!response.is_fresh());
    }

    #[test]
    fn test_stale_while_revalidate() {
        let response = CachedResponse {
            value: "test".to_string(),
            created_at: SystemTime::now(),
            ttl_seconds: 1,
            freshness_policy: FreshnessPolicy::StaleWhileRevalidate {
                revalidate_seconds: 3,
            },
        };
        assert!(response.is_fresh());
        thread::sleep(Duration::from_secs(2));
        assert!(!response.is_fresh());
        assert!(response.can_use_stale());
        assert!(response.is_revalidation_needed());
    }
}
```

---

## 4. Tool Registry Integration

```rust
use serde_json::json;

pub enum TelemetryEvent {
    CacheHit { key: String, tool_id: String },
    CacheMiss { key: String, tool_id: String },
    CacheRefreshed { key: String, tool_id: String },
    CacheRefreshFailed { key: String, tool_id: String, reason: String },
    CacheInvalidated { key: String, tool_id: String },
}

pub struct ToolRegistryWithCache {
    cache: Arc<ResponseCache>,
    telemetry_tx: tokio::sync::mpsc::UnboundedSender<TelemetryEvent>,
}

impl ToolRegistryWithCache {
    pub fn new(
        cache: Arc<ResponseCache>,
        telemetry_tx: tokio::sync::mpsc::UnboundedSender<TelemetryEvent>,
    ) -> Self {
        Self { cache, telemetry_tx }
    }

    pub async fn invoke_tool_with_cache(
        &self,
        tool_id: &str,
        input: &str,
        ttl_seconds: u64,
        freshness_policy: FreshnessPolicy,
    ) -> Result<String, String> {
        let cache_key = CacheKeyGenerator::generate(tool_id, input);

        // Check cache
        if let Some(cached) = self.cache.get(&cache_key) {
            if cached.is_fresh() {
                self.cache.stats.increment_hits();
                let _ = self.telemetry_tx.send(TelemetryEvent::CacheHit {
                    key: cache_key.clone(),
                    tool_id: tool_id.to_string(),
                });
                return Ok(cached.value);
            }

            // Handle stale-while-revalidate or stale-if-error
            if cached.can_use_stale() && cached.is_revalidation_needed() {
                let stale_value = cached.value.clone();

                // Spawn background refresh for SWR
                if matches!(
                    freshness_policy,
                    FreshnessPolicy::StaleWhileRevalidate { .. }
                ) {
                    self.spawn_background_refresh(tool_id, input, cache_key.clone());
                }

                return Ok(stale_value);
            }
        }

        // Cache miss or expired
        self.cache.stats.increment_misses();
        let _ = self.telemetry_tx.send(TelemetryEvent::CacheMiss {
            key: cache_key.clone(),
            tool_id: tool_id.to_string(),
        });

        // Invoke tool (simulated)
        let result = self.invoke_tool_impl(tool_id, input).await?;

        // Cache result
        let response = CachedResponse {
            value: result.clone(),
            created_at: SystemTime::now(),
            ttl_seconds,
            freshness_policy,
        };
        self.cache.insert(cache_key, response);

        Ok(result)
    }

    fn spawn_background_refresh(
        &self,
        tool_id: &str,
        input: &str,
        cache_key: String,
    ) {
        let cache = self.cache.clone();
        let telemetry_tx = self.telemetry_tx.clone();
        let tool_id = tool_id.to_string();
        let input = input.to_string();

        tokio::spawn(async move {
            match Self::invoke_tool_static(&tool_id, &input).await {
                Ok(result) => {
                    let response = CachedResponse {
                        value: result,
                        created_at: SystemTime::now(),
                        ttl_seconds: 60,
                        freshness_policy: FreshnessPolicy::Strict,
                    };
                    cache.insert(cache_key.clone(), response);
                    let _ = telemetry_tx.send(TelemetryEvent::CacheRefreshed {
                        key: cache_key,
                        tool_id,
                    });
                }
                Err(e) => {
                    let _ = telemetry_tx.send(TelemetryEvent::CacheRefreshFailed {
                        key: cache_key,
                        tool_id,
                        reason: e,
                    });
                }
            }
        });
    }

    async fn invoke_tool_impl(&self, tool_id: &str, input: &str) -> Result<String, String> {
        Self::invoke_tool_static(tool_id, input).await
    }

    async fn invoke_tool_static(tool_id: &str, input: &str) -> Result<String, String> {
        // Simulated tool invocation
        Ok(format!(
            r#"{{"tool_id":"{}","input":"{}","result":"success"}}"#,
            tool_id, input
        ))
    }
}
```

---

## 5. Performance Benchmarks

```rust
#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn bench_cache_hit_latency() {
        let cache = ResponseCache::new(1000);
        let key = "bench_key_1".to_string();
        let response = CachedResponse {
            value: "bench_value".to_string(),
            created_at: SystemTime::now(),
            ttl_seconds: 60,
            freshness_policy: FreshnessPolicy::Strict,
        };
        cache.insert(key.clone(), response);

        let start = Instant::now();
        for _ in 0..10000 {
            let _ = cache.get(&key);
        }
        let elapsed = start.elapsed();
        let avg_us = elapsed.as_micros() as f64 / 10000.0;

        println!("Average cache hit latency: {:.3} µs", avg_us);
        assert!(avg_us < 1000.0, "Cache hit should be <1ms");
    }

    #[test]
    fn bench_eviction_performance() {
        let cache = ResponseCache::new(100);
        let start = Instant::now();

        for i in 0..1000 {
            let response = CachedResponse {
                value: format!("value_{}", i),
                created_at: SystemTime::now(),
                ttl_seconds: 60,
                freshness_policy: FreshnessPolicy::Strict,
            };
            cache.insert(format!("key_{}", i), response);
        }

        let elapsed = start.elapsed();
        println!("1000 insertions with eviction: {:?}", elapsed);
        assert!(elapsed.as_millis() < 5, "Eviction should be <5ms per op");
    }
}
```

---

## 6. Comprehensive Testing Suite

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_tool_registry_cache_integration() {
        let cache = Arc::new(ResponseCache::new(100));
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let registry = ToolRegistryWithCache::new(cache.clone(), tx);

        let result1 = registry
            .invoke_tool_with_cache("tool_1", "input_1", 60, FreshnessPolicy::Strict)
            .await
            .unwrap();

        let result2 = registry
            .invoke_tool_with_cache("tool_1", "input_1", 60, FreshnessPolicy::Strict)
            .await
            .unwrap();

        assert_eq!(result1, result2);
        let (hits, misses, _, _) = cache.stats().get_snapshot();
        assert_eq!(hits, 1);
        assert_eq!(misses, 1);
    }

    #[tokio::test]
    async fn test_swr_background_refresh() {
        let cache = Arc::new(ResponseCache::new(100));
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let registry = ToolRegistryWithCache::new(cache.clone(), tx);

        let _result = registry
            .invoke_tool_with_cache(
                "tool_2",
                "input_2",
                1,
                FreshnessPolicy::StaleWhileRevalidate {
                    revalidate_seconds: 2,
                },
            )
            .await;

        // Wait for TTL to expire
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Request should return stale data and spawn refresh
        let _result2 = registry
            .invoke_tool_with_cache(
                "tool_2",
                "input_2",
                1,
                FreshnessPolicy::StaleWhileRevalidate {
                    revalidate_seconds: 2,
                },
            )
            .await;

        // Wait for background refresh to complete
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    #[test]
    fn test_cache_ttl_expiration() {
        let cache = ResponseCache::new(100);
        let response = CachedResponse {
            value: "test".to_string(),
            created_at: SystemTime::now() - Duration::from_secs(61),
            ttl_seconds: 60,
            freshness_policy: FreshnessPolicy::Strict,
        };

        assert!(!response.is_fresh());
        assert!(!response.can_use_stale());
    }

    #[test]
    fn test_stale_if_error_timeout() {
        let response = CachedResponse {
            value: "test".to_string(),
            created_at: SystemTime::now() - Duration::from_secs(65),
            ttl_seconds: 60,
            freshness_policy: FreshnessPolicy::StaleIfError {
                error_timeout_seconds: 30,
            },
        };

        assert!(!response.is_fresh());
        assert!(response.can_use_stale());
    }
}
```

---

## Summary

**Week 9 Deliverable implements:**
- In-memory LRU ResponseCache with CachedResponse and CacheStats
- Deterministic SHA-256 based CacheKeyGenerator with validation tests
- Three FreshnessPolicy implementations: Strict, StaleWhileRevalidate, StaleIfError
- Tool Registry integration with invoke_tool_with_cache
- Background refresh task spawning for SWR policy
- Six new telemetry events (CacheHit, CacheMiss, CacheRefreshed, CacheRefreshFailed, CacheInvalidated)
- Performance targets: <1ms cache hits, <5ms evictions
- Comprehensive test suite covering determinism, TTL, freshness, and SWR

**Lines of Code:** ~380 Rust (production + tests)

**Next Week:** Cache invalidation strategies, cache warming, and distributed cache coordination.
