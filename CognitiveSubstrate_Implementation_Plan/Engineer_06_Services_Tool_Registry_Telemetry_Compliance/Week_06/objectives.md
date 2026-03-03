# Engineer 6 — Services: Tool Registry, Telemetry & Compliance — Week 6

## Phase: Phase 0 (Weeks 1-6)

## Weekly Objective
Complete Phase 0 telemetry engine baseline with persistent logging, event archival strategy, and integration testing across all Phase 0 components (ToolBinding, CEF events, Stub Tool Registry, cost attribution). Prepare for transition to MCP-native Phase 1.

## Document References
- **Primary:** Section 6.1 (Phase 0 completion), Section 3.3.4 (Telemetry engine, cost attribution, real-time streaming)
- **Supporting:** Sections 2.11, 3.3.3, 3.3.5, 3.3.6; Weeks 1-5 (all Phase 0 components)

## Deliverables
- [ ] Persistent event logging
  - File-based event log (newline-delimited JSON)
  - Rolling log files (rotate after 100MB or 24 hours)
  - Archival to local filesystem (data retention per policy: 7 days operational minimum)
- [ ] Event archival and cleanup
  - Implement retention policy: keep last 7 days of events
  - Automated cleanup task (runs daily at midnight UTC)
  - Audit log of purged events (for compliance)
- [ ] Phase 0 integration tests
  - End-to-end: register tool -> invoke tool -> emit events -> verify cost metrics
  - Effect class enforcement: attempt to violate irreversible-not-last rule
  - Subscriber functionality: connect subscriber, receive filtered events
  - Cost calculation accuracy: verify token counts and TPC calculation
- [ ] Performance baseline metrics
  - Event emission latency (p50, p95, p99)
  - Event buffer memory footprint (at capacity)
  - Cost calculation overhead (per invocation)
  - Subscriber throughput (events/sec)
- [ ] Documentation
  - Phase 0 architecture overview and design decisions
  - Telemetry engine API reference
  - Tool Registry API reference
  - Cost attribution methodology
  - Troubleshooting guide
- [ ] Phase 0 retrospective and Phase 1 transition plan
  - Known limitations of stub/mock implementation
  - Planned enhancements in Phase 1 (MCP-native, real hardware instrumentation, etc.)
  - Risk mitigation for transition

## Technical Specifications

### Persistent Event Logging
```rust
pub struct PersistentEventLogger {
    log_dir: PathBuf,
    current_file: Arc<Mutex<File>>,
    file_size: Arc<AtomicU64>,
    max_file_size: u64, // 100 MB
}

impl PersistentEventLogger {
    pub fn new(log_dir: PathBuf) -> Result<Self, LogError> {
        std::fs::create_dir_all(&log_dir)?;
        let current_file = Self::create_log_file(&log_dir, Utc::now())?;
        Ok(Self {
            log_dir,
            current_file: Arc::new(Mutex::new(current_file)),
            file_size: Arc::new(AtomicU64::new(0)),
            max_file_size: 100 * 1024 * 1024, // 100 MB
        })
    }

    pub async fn write_event(&self, event: &CEFEvent) -> Result<(), LogError> {
        let json_str = serde_json::to_string(event)?;
        let bytes = json_str.as_bytes();

        let mut file = self.current_file.lock().await;
        file.write_all(bytes)?;
        file.write_all(b"\n")?;

        let new_size = self.file_size.fetch_add(bytes.len() as u64, Ordering::SeqCst)
                       + bytes.len() as u64;

        if new_size > self.max_file_size {
            self.rotate_log_file().await?;
        }

        Ok(())
    }

    async fn rotate_log_file(&self) -> Result<(), LogError> {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let archived_name = format!("events_{}.log.gz", timestamp);
        // Compress current file and rotate
        // Create new current file
        let new_file = Self::create_log_file(&self.log_dir, Utc::now())?;
        let mut current = self.current_file.lock().await;
        *current = new_file;
        self.file_size.store(0, Ordering::SeqCst);
        Ok(())
    }

    fn create_log_file(log_dir: &Path, now: DateTime<Utc>) -> Result<File, LogError> {
        let filename = format!("events_{}.log", now.format("%Y%m%d_%H%M%S"));
        let path = log_dir.join(&filename);
        File::create(path).map_err(|e| LogError::FileError(e))
    }
}
```

### Retention Policy and Cleanup
```rust
pub struct EventRetentionPolicy {
    operational_retention_days: u64, // 7 days
    log_dir: PathBuf,
}

impl EventRetentionPolicy {
    pub async fn cleanup(&self) -> Result<u64, CleanupError> {
        let cutoff_time = Utc::now() - Duration::days(self.operational_retention_days as i64);
        let mut purged_count = 0;

        for entry in std::fs::read_dir(&self.log_dir)? {
            let entry = entry?;
            let path = entry.path();
            let metadata = std::fs::metadata(&path)?;

            if let Ok(modified) = metadata.modified() {
                let modified_time: DateTime<Utc> = modified.into();
                if modified_time < cutoff_time {
                    std::fs::remove_file(&path)?;
                    purged_count += 1;
                }
            }
        }

        Ok(purged_count)
    }
}
```

### Integration Test Suite
```rust
#[tokio::test]
async fn test_end_to_end_tool_invocation_and_event_emission() {
    let registry = ToolRegistry::new();
    let telemetry = TelemetryEngine::new(10_000);

    // Register a mock tool
    let binding = ToolBinding {
        tool: "mock-web-search".to_string(),
        effect_class: EffectClass::READ_ONLY,
        ..Default::default()
    };
    let tool_id = registry.register_tool(binding).await.unwrap();

    // Invoke the tool
    let result = registry.invoke_tool(&tool_id, "test query".to_string(), &telemetry).await;
    assert!(result.is_ok());

    // Verify events were emitted
    // (In real test: subscribe, wait for events, verify content)
}

#[tokio::test]
async fn test_effect_class_enforcement() {
    let registry = ToolRegistry::new();

    // Register three tools with different effect classes
    let read_only = ToolBinding {
        effect_class: EffectClass::READ_ONLY,
        ..Default::default()
    };
    let reversible = ToolBinding {
        effect_class: EffectClass::WRITE_REVERSIBLE,
        ..Default::default()
    };
    let irreversible = ToolBinding {
        effect_class: EffectClass::WRITE_IRREVERSIBLE,
        ..Default::default()
    };

    let read_id = registry.register_tool(read_only).await.unwrap();
    let rev_id = registry.register_tool(reversible).await.unwrap();
    let irrev_id = registry.register_tool(irreversible).await.unwrap();

    // Valid chain: READ_ONLY -> REVERSIBLE -> IRREVERSIBLE
    let valid_chain = vec![read_id.clone(), rev_id.clone(), irrev_id.clone()];
    assert!(registry.validate_execution_chain(&valid_chain).await.is_ok());

    // Invalid chain: IRREVERSIBLE -> READ_ONLY
    let invalid_chain = vec![irrev_id.clone(), read_id.clone()];
    assert!(registry.validate_execution_chain(&invalid_chain).await.is_err());
}

#[tokio::test]
async fn test_cost_calculation_accuracy() {
    let telemetry = TelemetryEngine::new(10_000);

    let input_tokens = 1024u64;
    let output_tokens = 512u64;
    let gpu_ms = 100.0;
    let wall_ms = 200.0;

    let cost = telemetry.calculate_cost(input_tokens, output_tokens, gpu_ms, wall_ms);

    assert_eq!(cost.input_tokens, 1024);
    assert_eq!(cost.output_tokens, 512);
    assert_eq!(cost.gpu_milliseconds, 100.0);
    assert_eq!(cost.wall_clock_milliseconds, 200.0);
    assert!(cost.tpc_hours > 0.0); // TPC calculation verified
}
```

### Performance Baselines
```
Baseline Metrics (Target):
  - Event emission latency: p50 < 1ms, p95 < 5ms, p99 < 10ms
  - In-memory buffer footprint: ~100 MB for 10k events
  - Cost calculation overhead: <0.1ms per invocation
  - Subscriber throughput: >10k events/sec
  - Log file I/O overhead: <1% CPU

Measurement Plan:
  - Run load tests: 1k, 10k, 100k events
  - Profile CPU and memory usage
  - Document results in performance_baselines.md
```

### Phase 0 Architecture Document
Include:
- High-level system diagram (ToolBinding -> Tool Registry -> Telemetry Engine)
- ToolBinding entity definition and effect class semantics
- CEF event structure and all 10 event types
- Stub Tool Registry design and limitations
- Telemetry engine architecture and cost attribution
- Integration points and data flow
- Known limitations and Phase 1 enhancements
- Glossary of terms

## Dependencies
- **Blocked by:** Weeks 1-5 (all Phase 0 components complete)
- **Blocking:** Phase 1 Week 7-8 (MCP-native Tool Registry), Week 9-10 (response caching), Week 11-12 (full telemetry)

## Acceptance Criteria
- [ ] Persistent event logging to newline-delimited JSON files
- [ ] Log rotation on size (100MB) or time (24h) threshold
- [ ] Retention policy cleanup runs automatically; purged events audited
- [ ] End-to-end integration tests pass: register, invoke, emit, verify
- [ ] Effect class enforcement prevents invalid chains
- [ ] Cost calculation accuracy validated (tokens, TPC-hours)
- [ ] Performance baselines captured and documented
- [ ] Phase 0 architecture document complete and reviewed
- [ ] Phase 0 retrospective and Phase 1 transition plan written
- [ ] All Phase 0 objectives from Weeks 1-6 verified complete
- [ ] Ready for Phase 1 handoff

## Design Principles Alignment
- **Completeness:** Phase 0 establishes full telemetry foundation; no gaps for Phase 1 to backfill
- **Observability:** All events persist; queryable for debugging and compliance
- **Compliance-ready:** Retention tiers enforce data governance from Phase 0
- **Testability:** Integration tests verify all critical paths before Phase 1
- **Transition clarity:** Architecture doc and retrospective enable clean Phase 1 start
