# Engineer 6 — Services: Tool Registry, Telemetry & Compliance — Week 9

## Phase: Phase 1 (Weeks 7-14)

## Weekly Objective
Begin response caching implementation with cache key generation, TTL configuration, and freshness policies. Integrate with MCP Tool Registry from Weeks 7-8 to optimize tool invocation throughput.

## Document References
- **Primary:** Section 6.2 (Phase 1, Week 9-10: Response caching), Section 3.3.3 (Tool Registry, response caching)
- **Supporting:** Section 2.11 (ToolBinding response_cache field), Week 7-8 (MCP Tool Registry)

## Deliverables
- [ ] Response cache architecture and design
  - In-memory cache with eviction policies (LRU)
  - Persistent cache backend (optional; defer to Phase 2)
  - Cache statistics tracking (hits, misses, evictions)
- [ ] Cache key generation strategy
  - Deterministic key from tool_id + input_hash
  - Input hashing function (SHA-256 for determinism)
  - Collision detection and logging
- [ ] TTL and freshness policy configuration
  - Per-tool TTL (from ToolBinding response_cache config)
  - Freshness policies: strict, stale_while_revalidate, stale_if_error
  - Cache invalidation triggers (explicit, time-based, event-based)
- [ ] Cache hit/miss telemetry
  - Emit CacheHit event on successful lookup
  - Emit CacheMiss event on lookup failure
  - Track cache performance metrics
- [ ] Stale-while-revalidate (SWR) implementation
  - Serve stale cache entry while refreshing in background
  - Nonblocking refresh task
  - Fallback to stale entry on refresh failure
- [ ] Unit and integration tests
  - Cache key generation determinism
  - TTL expiration and eviction
  - Freshness policies (all three types)
  - SWR behavior and background refresh

## Technical Specifications

### Response Cache Core
```rust
pub struct ResponseCache {
    cache: Arc<RwLock<LRUCache<String, CachedResponse>>>,
    max_entries: usize,
    stats: Arc<CacheStats>,
}

pub struct CachedResponse {
    value: String,
    created_at: Instant,
    ttl_seconds: u64,
    freshness_policy: FreshnessPolicy,
}

pub enum FreshnessPolicy {
    Strict,
    StaleWhileRevalidate { revalidate_seconds: u64 },
    StaleIfError { error_timeout_seconds: u64 },
}

pub struct CacheStats {
    hits: AtomicU64,
    misses: AtomicU64,
    evictions: AtomicU64,
    errors: AtomicU64,
}

impl ResponseCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(LRUCache::new(max_entries))),
            max_entries,
            stats: Arc::new(CacheStats {
                hits: AtomicU64::new(0),
                misses: AtomicU64::new(0),
                evictions: AtomicU64::new(0),
                errors: AtomicU64::new(0),
            }),
        }
    }

    pub async fn get(&self, key: &str) -> Option<String> {
        let cache = self.cache.read().await;
        if let Some(cached) = cache.get(key) {
            if self.is_fresh(cached) {
                self.stats.hits.fetch_add(1, Ordering::Relaxed);
                return Some(cached.value.clone());
            }
        }

        self.stats.misses.fetch_add(1, Ordering::Relaxed);
        None
    }

    pub async fn get_or_stale(&self, key: &str) -> Option<String> {
        let cache = self.cache.read().await;
        if let Some(cached) = cache.get(key) {
            if self.is_fresh(cached) {
                self.stats.hits.fetch_add(1, Ordering::Relaxed);
                return Some(cached.value.clone());
            }

            // Check if stale but within acceptable range
            match &cached.freshness_policy {
                FreshnessPolicy::Strict => {},
                FreshnessPolicy::StaleWhileRevalidate { .. } => {
                    return Some(cached.value.clone());
                }
                FreshnessPolicy::StaleIfError { .. } => {
                    return Some(cached.value.clone());
                }
            }
        }

        self.stats.misses.fetch_add(1, Ordering::Relaxed);
        None
    }

    pub async fn set(&self, key: String, value: String, ttl_seconds: u64,
                    policy: FreshnessPolicy)
    {
        let cached = CachedResponse {
            value,
            created_at: Instant::now(),
            ttl_seconds,
            freshness_policy: policy,
        };

        let mut cache = self.cache.write().await;
        if cache.len() >= self.max_entries {
            cache.pop_lru();
            self.stats.evictions.fetch_add(1, Ordering::Relaxed);
        }
        cache.insert(key, cached);
    }

    pub async fn invalidate(&self, key: &str) -> bool {
        let mut cache = self.cache.write().await;
        cache.remove(key).is_some()
    }

    pub async fn stats(&self) -> (u64, u64, u64, u64) {
        (
            self.stats.hits.load(Ordering::Relaxed),
            self.stats.misses.load(Ordering::Relaxed),
            self.stats.evictions.load(Ordering::Relaxed),
            self.stats.errors.load(Ordering::Relaxed),
        )
    }

    fn is_fresh(&self, cached: &CachedResponse) -> bool {
        cached.created_at.elapsed().as_secs() < cached.ttl_seconds
    }
}
```

### Cache Key Generation
```rust
pub struct CacheKeyGenerator;

impl CacheKeyGenerator {
    pub fn generate_key(tool_id: &str, input: &str) -> String {
        let input_hash = Self::hash_input(input);
        format!("{}:{}", tool_id, input_hash)
    }

    fn hash_input(input: &str) -> String {
        use sha2::{Sha256, Digest};

        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        let result = hasher.finalize();
        format!("{:x}", result)
    }

    pub fn validate_determinism(input: &str, iterations: usize) -> Result<(), String> {
        let first_key = Self::hash_input(input);

        for _ in 1..iterations {
            let key = Self::hash_input(input);
            if key != first_key {
                return Err("Cache key generation is not deterministic".to_string());
            }
        }

        Ok(())
    }
}
```

### Tool Registry Integration
```rust
impl MCPToolRegistry {
    pub async fn invoke_tool_with_cache(&self, tool_id: &str, input: String,
                                        cache: &ResponseCache,
                                        telemetry: &TelemetryEngine)
        -> Result<String, InvokeError>
    {
        let binding = self.get_binding(tool_id).await?;

        // Generate cache key
        let cache_key = CacheKeyGenerator::generate_key(tool_id, &input);

        // Try cache hit
        if let Some(cached_value) = cache.get(&cache_key).await {
            let cache_hit_event = CEFEvent {
                event_type: EventType::CacheHit,
                actor: "response_cache",
                resource: tool_id.to_string(),
                action: "LOOKUP",
                result: EventResult::COMPLETED,
                context: {
                    "cache_key": cache_key.clone(),
                    "ttl_seconds": binding.response_cache.ttl_seconds.to_string(),
                }.into(),
                ..Default::default()
            };
            telemetry.emit_event(cache_hit_event).await.ok();
            return Ok(cached_value);
        }

        // Cache miss: invoke tool
        let cache_miss_event = CEFEvent {
            event_type: EventType::CacheMiss,
            actor: "response_cache",
            resource: tool_id.to_string(),
            action: "LOOKUP",
            result: EventResult::COMPLETED,
            context: {
                "cache_key": cache_key.clone(),
            }.into(),
            ..Default::default()
        };
        telemetry.emit_event(cache_miss_event).await.ok();

        // Execute tool
        let output = self.invoke_tool_impl(tool_id, &input).await?;

        // Cache result
        let freshness_policy = match binding.response_cache.freshness_policy.as_str() {
            "stale_while_revalidate" => FreshnessPolicy::StaleWhileRevalidate {
                revalidate_seconds: binding.response_cache.ttl_seconds / 2,
            },
            "stale_if_error" => FreshnessPolicy::StaleIfError {
                error_timeout_seconds: binding.response_cache.ttl_seconds,
            },
            _ => FreshnessPolicy::Strict,
        };

        cache.set(cache_key, output.clone(), binding.response_cache.ttl_seconds, freshness_policy)
            .await;

        Ok(output)
    }

    async fn invoke_tool_impl(&self, tool_id: &str, input: &str) -> Result<String, InvokeError> {
        // Call actual MCP tool (stub for now)
        Ok(format!("Result for {}: {}", tool_id, input))
    }
}
```

### Stale-While-Revalidate Background Refresh
```rust
pub struct BackgroundRefresh {
    cache: Arc<ResponseCache>,
    tool_registry: Arc<MCPToolRegistry>,
    telemetry: Arc<TelemetryEngine>,
}

impl BackgroundRefresh {
    pub async fn schedule_refresh(&self, cache_key: &str, tool_id: &str, input: &str) {
        let cache = self.cache.clone();
        let registry = self.tool_registry.clone();
        let telemetry = self.telemetry.clone();
        let cache_key = cache_key.to_string();
        let tool_id = tool_id.to_string();
        let input = input.to_string();

        tokio::spawn(async move {
            match registry.invoke_tool_impl(&tool_id, &input).await {
                Ok(fresh_output) => {
                    cache.set(cache_key.clone(), fresh_output, 3600, FreshnessPolicy::Strict)
                        .await;

                    telemetry.emit_event(CEFEvent {
                        event_type: EventType::CacheRefreshed,
                        actor: "background_refresh",
                        resource: tool_id,
                        action: "REFRESH",
                        result: EventResult::COMPLETED,
                        context: {
                            "cache_key": cache_key.clone(),
                        }.into(),
                        ..Default::default()
                    }).await.ok();
                }
                Err(e) => {
                    telemetry.emit_event(CEFEvent {
                        event_type: EventType::CacheRefreshFailed,
                        actor: "background_refresh",
                        resource: tool_id,
                        action: "REFRESH",
                        result: EventResult::FAILED,
                        context: {
                            "cache_key": cache_key.clone(),
                            "error": format!("{:?}", e),
                        }.into(),
                        ..Default::default()
                    }).await.ok();
                }
            }
        });
    }
}
```

### Cache Events
```rust
pub enum EventType {
    // ... existing events ...
    CacheHit,
    CacheMiss,
    CacheRefreshed,
    CacheRefreshFailed,
    CacheInvalidated,
}
```

## Dependencies
- **Blocked by:** Week 7-8 (MCP Tool Registry complete)
- **Blocking:** Week 10 (complete response caching), Week 11-12 (telemetry integration)

## Acceptance Criteria
- [ ] ResponseCache implementation with LRU eviction functional
- [ ] Cache key generation deterministic; SHA-256 hash collisions tested
- [ ] TTL expiration working; stale entries evicted on access
- [ ] All three freshness policies (Strict, SWR, StaleIfError) implemented and tested
- [ ] CacheHit and CacheMiss events emitted and logged
- [ ] SWR background refresh spawned and tracked
- [ ] Cache statistics tracked (hits, misses, evictions, errors)
- [ ] Cache hit latency <1ms; eviction <5ms
- [ ] Unit tests cover all TTL/freshness scenarios
- [ ] Integration tests with Tool Registry pass

## Design Principles Alignment
- **Performance:** Cache hits serve instantly; SWR prevents stalls during refresh
- **Safety:** Stale data acceptable only within configured bounds; explicit policies
- **Observability:** All cache operations logged; statistics trackable
- **Efficiency:** LRU eviction prevents unbounded memory growth
- **Transparency:** Cache key generation transparent; collision detection enabled
