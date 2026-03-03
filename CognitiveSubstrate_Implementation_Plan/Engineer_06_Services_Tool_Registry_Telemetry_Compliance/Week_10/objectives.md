# Engineer 6 — Services: Tool Registry, Telemetry & Compliance — Week 10

## Phase: Phase 1 (Weeks 7-14)

## Weekly Objective
Complete response caching implementation with persistent cache backend, cache warming, and comprehensive testing. Register 5 real tools with caching enabled and production-ready cache layer.

## Document References
- **Primary:** Section 6.2 (Phase 1, Week 9-10: Response caching completion), Section 3.3.3 (Tool Registry, response caching)
- **Supporting:** Week 9 (caching started), Week 7-8 (Tool Registry)

## Deliverables
- [ ] Persistent cache backend (optional; Phase 1 Week 10)
  - SQLite for local persistence (with journal WAL mode)
  - Cache metadata: key, value, ttl, created_at, hit_count, last_accessed
  - Optional persistence flag per tool (read-only tools: yes; write tools: no)
- [ ] Cache warming strategy
  - Pre-populate cache for frequently-used tools on startup
  - Periodic cache refresh for long-TTL entries (background task)
  - Warm cache from previous session snapshots
- [ ] Cache statistics and monitoring
  - Per-tool cache metrics (hit rate, average response time, eviction count)
  - Overall cache health dashboard (memory usage, eviction rate, freshness ratio)
  - Emit CacheStats event periodically (every hour)
- [ ] Response size limits and compression
  - Maximum response size per tool (prevent cache bloat)
  - Optional response compression for large responses
  - Fallback to no-cache on oversized responses
- [ ] Cache invalidation strategies
  - Explicit invalidation API (for policy changes or data updates)
  - Dependency-based invalidation (one cache entry invalidates related entries)
  - Time-based invalidation (TTL)
  - Event-based invalidation (on tool sandbox changes)
- [ ] Comprehensive cache testing
  - End-to-end: tool invocation -> cache store -> subsequent lookup
  - Persistence: cache survives process restart
  - Cache warming: pre-populated entries valid after startup
  - Eviction: LRU policy enforced; oversized responses handled
  - Freshness policies: all three tested with timing
- [ ] Production readiness
  - Cache corruption recovery
  - Concurrent access (thread-safe, lock-free reads where possible)
  - Performance tuning (cache line alignment, memory layout)
- [ ] Documentation
  - Cache configuration guide (TTL, freshness policy per tool)
  - Cache troubleshooting and monitoring
  - Cache performance tuning guide

## Technical Specifications

### Persistent Cache Backend (SQLite)
```rust
pub struct PersistentCache {
    conn: Arc<Connection>,
    in_memory_cache: Arc<ResponseCache>,
    config: PersistentCacheConfig,
}

pub struct PersistentCacheConfig {
    db_path: PathBuf,
    enable_persistence: bool,
    wal_checkpoint_interval: u32,
    compression_threshold_bytes: usize,
}

impl PersistentCache {
    pub fn new(config: PersistentCacheConfig) -> Result<Self, CacheError> {
        let conn = Connection::open(&config.db_path)?;

        // Enable WAL mode for concurrent reads
        conn.execute("PRAGMA journal_mode = WAL", [])?;

        // Create cache table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS cache (
                key TEXT PRIMARY KEY,
                value BLOB,
                ttl_seconds INTEGER,
                created_at INTEGER,
                hit_count INTEGER,
                last_accessed INTEGER,
                compressed BOOLEAN,
                tool_id TEXT
            )",
            [],
        )?;

        Ok(Self {
            conn: Arc::new(conn),
            in_memory_cache: Arc::new(ResponseCache::new(10_000)),
            config,
        })
    }

    pub async fn get(&self, key: &str) -> Result<Option<String>, CacheError> {
        // Try in-memory cache first
        if let Some(value) = self.in_memory_cache.get(key).await {
            return Ok(Some(value));
        }

        // Fall back to persistent store
        if self.config.enable_persistence {
            let value = self.get_from_db(key)?;
            if let Some(v) = value {
                // Restore to in-memory cache
                self.in_memory_cache.set(key.to_string(), v.clone(), 3600, FreshnessPolicy::Strict)
                    .await;
                return Ok(Some(v));
            }
        }

        Ok(None)
    }

    pub async fn set(&self, key: String, value: String, ttl_seconds: u64,
                    policy: FreshnessPolicy, tool_id: &str) -> Result<(), CacheError>
    {
        // Store in in-memory cache
        self.in_memory_cache.set(key.clone(), value.clone(), ttl_seconds, policy)
            .await;

        // Optionally persist for read-only tools
        if self.config.enable_persistence {
            self.set_in_db(&key, &value, ttl_seconds, tool_id)?;
        }

        Ok(())
    }

    fn get_from_db(&self, key: &str) -> Result<Option<String>, CacheError> {
        let mut stmt = self.conn.prepare(
            "SELECT value, compressed FROM cache WHERE key = ? AND datetime('now') < datetime(created_at + ttl_seconds, 'unixepoch')"
        )?;

        let value = stmt.query_row([key], |row| {
            let compressed: bool = row.get(1)?;
            let blob: Vec<u8> = row.get(0)?;

            if compressed {
                Ok(self.decompress(&blob).ok())
            } else {
                Ok(String::from_utf8(blob).ok())
            }
        }).optional()?;

        Ok(value.flatten())
    }

    fn set_in_db(&self, key: &str, value: &str, ttl_seconds: u64,
                tool_id: &str) -> Result<(), CacheError>
    {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let value_bytes = value.as_bytes();
        let (compressed, storage_blob) = if value_bytes.len() > self.config.compression_threshold_bytes {
            (true, self.compress(value_bytes)?)
        } else {
            (false, value_bytes.to_vec())
        };

        self.conn.execute(
            "INSERT OR REPLACE INTO cache (key, value, ttl_seconds, created_at, hit_count, last_accessed, compressed, tool_id)
             VALUES (?, ?, ?, ?, 0, ?, ?, ?)",
            params![key, storage_blob, ttl_seconds, now, now, compressed, tool_id],
        )?;

        Ok(())
    }

    fn compress(&self, data: &[u8]) -> Result<Vec<u8>, CacheError> {
        use flate2::Compression;
        use flate2::write::GzEncoder;
        use std::io::Write;

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data)?;
        encoder.finish().map_err(|e| CacheError::CompressionError(e.to_string()))
    }

    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>, CacheError> {
        use flate2::read::GzDecoder;
        use std::io::Read;

        let mut decoder = GzDecoder::new(data);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;
        Ok(decompressed)
    }

    pub fn checkpoint(&self) -> Result<(), CacheError> {
        self.conn.execute("PRAGMA wal_checkpoint(RESTART)", [])?;
        Ok(())
    }
}
```

### Cache Warming
```rust
pub struct CacheWarmer {
    cache: Arc<PersistentCache>,
    tool_registry: Arc<MCPToolRegistry>,
    telemetry: Arc<TelemetryEngine>,
}

impl CacheWarmer {
    pub async fn warm_cache_from_snapshot(&self, snapshot_path: &Path) -> Result<u32, CacheError> {
        let snapshot = std::fs::read_to_string(snapshot_path)?;
        let entries: Vec<CacheWarmEntry> = serde_json::from_str(&snapshot)?;

        let mut warmed_count = 0;
        for entry in entries {
            if let Ok(()) = self.cache.set(
                entry.key,
                entry.value,
                entry.ttl_seconds,
                FreshnessPolicy::Strict,
                &entry.tool_id,
            ).await {
                warmed_count += 1;
            }
        }

        self.telemetry.emit_event(CEFEvent {
            event_type: EventType::CacheWarmed,
            actor: "cache_warmer",
            resource: "cache".to_string(),
            action: "WARM",
            result: EventResult::COMPLETED,
            context: {
                "warmed_count": format!("{}", warmed_count),
                "snapshot_path": snapshot_path.to_string_lossy().to_string(),
            }.into(),
            ..Default::default()
        }).await.ok();

        Ok(warmed_count)
    }

    pub async fn periodic_refresh(&self, interval: Duration) {
        loop {
            tokio::time::sleep(interval).await;

            // Refresh high-hit-count entries
            // Implementation depends on cache statistics tracking
        }
    }

    pub async fn save_snapshot(&self, output_path: &Path) -> Result<(), CacheError> {
        let entries = vec![]; // Collect from cache
        let json = serde_json::to_string(&entries)?;
        std::fs::write(output_path, json)?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
pub struct CacheWarmEntry {
    pub key: String,
    pub value: String,
    pub ttl_seconds: u64,
    pub tool_id: String,
}
```

### Per-Tool Cache Configuration
```rust
pub struct ToolCacheConfig {
    pub tool_id: String,
    pub cache_enabled: bool,
    pub ttl_seconds: u64,
    pub freshness_policy: FreshnessPolicy,
    pub persist: bool,
    pub max_response_size_bytes: usize,
}

impl ToolCacheConfig {
    pub fn for_web_search() -> Self {
        Self {
            tool_id: "web_search".to_string(),
            cache_enabled: true,
            ttl_seconds: 3600,
            freshness_policy: FreshnessPolicy::StaleWhileRevalidate { revalidate_seconds: 1800 },
            persist: true,
            max_response_size_bytes: 1_000_000,
        }
    }

    pub fn for_code_executor() -> Self {
        Self {
            tool_id: "code_executor".to_string(),
            cache_enabled: false, // No caching for code execution
            ttl_seconds: 0,
            freshness_policy: FreshnessPolicy::Strict,
            persist: false,
            max_response_size_bytes: 0,
        }
    }

    pub fn for_file_system() -> Self {
        Self {
            tool_id: "file_system".to_string(),
            cache_enabled: true,
            ttl_seconds: 300,
            freshness_policy: FreshnessPolicy::StaleIfError { error_timeout_seconds: 60 },
            persist: false, // File changes invalidate cache
            max_response_size_bytes: 10_000_000,
        }
    }

    pub fn for_database() -> Self {
        Self {
            tool_id: "database".to_string(),
            cache_enabled: false, // No caching for data mutations
            ttl_seconds: 0,
            freshness_policy: FreshnessPolicy::Strict,
            persist: false,
            max_response_size_bytes: 0,
        }
    }

    pub fn for_calculator() -> Self {
        Self {
            tool_id: "calculator".to_string(),
            cache_enabled: true,
            ttl_seconds: 86400,
            freshness_policy: FreshnessPolicy::Strict,
            persist: true,
            max_response_size_bytes: 100_000,
        }
    }
}
```

### Cache Statistics and Monitoring
```rust
pub struct CacheStatsCollector {
    cache: Arc<PersistentCache>,
    telemetry: Arc<TelemetryEngine>,
}

impl CacheStatsCollector {
    pub async fn emit_periodic_stats(&self, interval: Duration) {
        loop {
            tokio::time::sleep(interval).await;

            let (hits, misses, evictions, errors) = self.cache.in_memory_cache.stats().await;
            let hit_rate = if hits + misses > 0 {
                (hits as f64) / ((hits + misses) as f64)
            } else {
                0.0
            };

            self.telemetry.emit_event(CEFEvent {
                event_type: EventType::CacheStats,
                actor: "cache_monitor",
                resource: "cache".to_string(),
                action: "MONITOR",
                result: EventResult::COMPLETED,
                context: {
                    "hits": format!("{}", hits),
                    "misses": format!("{}", misses),
                    "evictions": format!("{}", evictions),
                    "errors": format!("{}", errors),
                    "hit_rate": format!("{:.2}%", hit_rate * 100.0),
                }.into(),
                ..Default::default()
            }).await.ok();
        }
    }
}
```

### Cache Invalidation Strategies
```rust
pub struct CacheInvalidator {
    cache: Arc<PersistentCache>,
}

impl CacheInvalidator {
    pub async fn invalidate_by_key(&self, key: &str) -> bool {
        self.cache.in_memory_cache.invalidate(key).await
    }

    pub async fn invalidate_by_tool(&self, tool_id: &str) -> u32 {
        // In production: query DB and invalidate all entries for tool_id
        0 // Placeholder
    }

    pub async fn invalidate_on_policy_change(&self, tool_id: &str) {
        // Called when sandbox policy changes for a tool
        self.invalidate_by_tool(tool_id).await;
    }

    pub async fn invalidate_on_ttl_expiry(&self) {
        // Periodic cleanup task
        // Database VACUUM on WAL checkpoint
    }
}
```

## Dependencies
- **Blocked by:** Week 9 (response caching started)
- **Blocking:** Week 11-12 (full telemetry implementation), Phase 2 compliance work

## Acceptance Criteria
- [ ] Persistent cache backend (SQLite WAL) functional
- [ ] Cache warming from snapshots works; pre-populated entries valid after restart
- [ ] Cache statistics collected and emitted hourly
- [ ] All 5 tools configured with appropriate cache policies (web_search: cached, code_executor: not cached, etc.)
- [ ] Response compression working for large responses
- [ ] Cache invalidation API functional (explicit, dependency-based, time-based, event-based)
- [ ] Cache corruption recovery tested and validated
- [ ] Concurrent access thread-safe; reads lock-free where possible
- [ ] Cache hit rate >80% for read-only tools; latency <1ms
- [ ] Unit and integration tests pass (persistence, warming, invalidation, concurrency)
- [ ] Documentation complete; cache configuration guide written

## Design Principles Alignment
- **Persistence:** Cache survives process restarts; snapshots enable fast warmup
- **Safety:** Write tools not cached; read-only tools cached aggressively
- **Performance:** In-memory cache for fast access; persistent store for recovery
- **Observability:** Cache statistics tracked; health dashboard available
- **Flexibility:** Per-tool cache configuration; easy to adjust TTL and freshness policies
