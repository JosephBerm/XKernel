//! Retention policy enforcement with automated cleanup and audit logging.
//!
//! This module implements the data retention layer for telemetry events,
//! enforcing a 7-day retention window with automated daily cleanup. It maintains
//! an audit log of all purged events for compliance and debugging purposes.
//!
//! # Architecture
//!
//! The retention policy system consists of:
//! - **Retention policy**: 7-day window for active event storage
//! - **Cleanup scheduler**: Runs daily to remove events older than 7 days
//! - **Audit logger**: Immutable log of all purged events and timestamps
//! - **Metadata tracker**: Tracks last cleanup time and statistics
//!
//! # Cleanup Process
//!
//! Daily cleanup:
//! 1. Scan event log for entries older than 7 days (604,800 seconds)
//! 2. Move expired events to audit log before deletion
//! 3. Update metadata with purge timestamp and count
//! 4. Remove expired entries from active log
//! 5. Log cleanup statistics for monitoring
//!
//! # Audit Trail
//!
//! All purged events are logged to an immutable audit file with:
//! - Original event timestamp
//! - Purge timestamp
//! - Reason for purge (retention policy)
//! - Event digest (first 256 bytes)
//!
//! # Example
//!
//! ```ignore
//! use tool_registry_telemetry::retention_policy::RetentionPolicy;
//!
//! let mut policy = RetentionPolicy::new("/var/log/events", Duration::from_secs(604_800))?;
//! let stats = policy.run_cleanup()?;
//! println!("Purged {} events", stats.events_purged);
//! ```

use crate::error::{ToolError, Result};
use serde_json::{json, Value};
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use alloc::string::String;

/// Default retention window: 7 days
const DEFAULT_RETENTION_SECS: u64 = 604_800;

/// Statistics about a retention policy cleanup operation.
#[derive(Debug, Clone)]
pub struct CleanupStats {
    /// Number of events purged
    pub events_purged: u64,
    /// Timestamp when cleanup was performed (seconds since UNIX_EPOCH)
    pub cleanup_timestamp: u64,
    /// Total bytes removed
    pub bytes_removed: u64,
    /// Events that couldn't be parsed (corrupted)
    pub parse_errors: u64,
}

impl CleanupStats {
    /// Creates new cleanup statistics.
    pub fn new(cleanup_timestamp: u64) -> Self {
        Self {
            events_purged: 0,
            cleanup_timestamp,
            bytes_removed: 0,
            parse_errors: 0,
        }
    }
}

/// Retention policy enforcer with automated cleanup and audit logging.
///
/// Manages event retention according to a configurable time window (default 7 days),
/// maintains audit trails of purged events, and provides cleanup statistics.
#[derive(Debug)]
pub struct RetentionPolicy {
    /// Directory containing event logs
    log_dir: PathBuf,
    /// Active event log file path
    event_log_path: PathBuf,
    /// Audit log file path (immutable)
    audit_log_path: PathBuf,
    /// Metadata file tracking cleanup history
    metadata_path: PathBuf,
    /// Retention window in seconds
    retention_window_secs: u64,
    /// Last cleanup timestamp
    last_cleanup_timestamp: u64,
}

impl RetentionPolicy {
    /// Creates a new retention policy enforcer.
    ///
    /// Initializes the retention policy with default 7-day window. Creates
    /// audit log and metadata files if they don't exist.
    ///
    /// # Arguments
    ///
    /// * `log_dir` - Directory containing event logs
    ///
    /// # Returns
    ///
    /// * `Result<Self>` - Initialized retention policy or error
    ///
    /// # Errors
    ///
    /// Returns error if directory or file operations fail.
    pub fn new<P: AsRef<Path>>(log_dir: P) -> Result<Self> {
        Self::with_window(log_dir, DEFAULT_RETENTION_SECS)
    }

    /// Creates a retention policy with custom retention window.
    ///
    /// # Arguments
    ///
    /// * `log_dir` - Directory containing event logs
    /// * `retention_window_secs` - Custom retention window in seconds
    ///
    /// # Returns
    ///
    /// * `Result<Self>` - Initialized retention policy or error
    pub fn with_window<P: AsRef<Path>>(log_dir: P, retention_window_secs: u64) -> Result<Self> {
        let log_dir = log_dir.as_ref().to_path_buf();
        let event_log_path = log_dir.join("events.ndjson");
        let audit_log_path = log_dir.join("events.audit.ndjson");
        let metadata_path = log_dir.join("retention.metadata.json");

        // Ensure files exist
        if !audit_log_path.exists() {
            File::create(&audit_log_path)
                .map_err(|e| ToolError::Other(format!("Failed to create audit log: {}", e)))?;
        }

        let last_cleanup_timestamp = Self::load_metadata(&metadata_path)
            .map(|m| m.get("last_cleanup").and_then(|v| v.as_u64()).unwrap_or(0))
            .unwrap_or(0);

        Ok(Self {
            log_dir,
            event_log_path,
            audit_log_path,
            metadata_path,
            retention_window_secs,
            last_cleanup_timestamp,
        })
    }

    /// Runs the retention cleanup process.
    ///
    /// Scans event log for entries older than the retention window,
    /// moves them to audit log, and removes from active log.
    ///
    /// # Returns
    ///
    /// * `Result<CleanupStats>` - Statistics about the cleanup operation
    ///
    /// # Errors
    ///
    /// Returns error if file operations fail. Does not fail on individual
    /// event parsing errors (tracked in stats).
    pub fn run_cleanup(&mut self) -> Result<CleanupStats> {
        let mut stats = CleanupStats::new(now_timestamp());
        
        if !self.event_log_path.exists() {
            self.save_metadata(&stats)?;
            return Ok(stats);
        }

        let cutoff_time = stats.cleanup_timestamp
            .saturating_sub(self.retention_window_secs);

        // Read and process events
        let file = File::open(&self.event_log_path)
            .map_err(|e| ToolError::Other(format!("Failed to open event log: {}", e)))?;
        
        let reader = BufReader::new(file);
        let mut retained_lines = Vec::new();
        let mut audit_lines = Vec::new();

        for line in reader.lines() {
            match line {
                Ok(event_line) => {
                    if let Ok(event) = serde_json::from_str::<Value>(&event_line) {
                        if let Some(timestamp) = event.get("timestamp").and_then(|v| v.as_u64()) {
                            if timestamp < cutoff_time {
                                // Event is expired, add to audit
                                let audit_entry = json!({
                                    "original_timestamp": timestamp,
                                    "purge_timestamp": stats.cleanup_timestamp,
                                    "reason": "retention_policy_expiration",
                                    "digest": event_line.chars().take(256).collect::<String>(),
                                });
                                audit_lines.push(serde_json::to_string(&audit_entry)
                                    .unwrap_or_default());
                                stats.events_purged += 1;
                                stats.bytes_removed += event_line.len() as u64;
                            } else {
                                // Event is retained
                                retained_lines.push(event_line);
                            }
                        } else {
                            retained_lines.push(event_line);
                        }
                    } else {
                        stats.parse_errors += 1;
                        retained_lines.push(event_line);
                    }
                }
                Err(_) => {
                    stats.parse_errors += 1;
                }
            }
        }

        // Write audit entries
        if !audit_lines.is_empty() {
            let mut audit_file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.audit_log_path)
                .map_err(|e| ToolError::Other(format!("Failed to open audit log: {}", e)))?;

            for line in audit_lines {
                writeln!(audit_file, "{}", line)
                    .map_err(|e| ToolError::Other(format!("Failed to write audit: {}", e)))?;
            }
        }

        // Rewrite event log with retained entries
        let mut new_file = File::create(&self.event_log_path)
            .map_err(|e| ToolError::Other(format!("Failed to create event log: {}", e)))?;

        for line in retained_lines {
            writeln!(new_file, "{}", line)
                .map_err(|e| ToolError::Other(format!("Failed to write event: {}", e)))?;
        }

        self.last_cleanup_timestamp = stats.cleanup_timestamp;
        self.save_metadata(&stats)?;

        Ok(stats)
    }

    /// Returns the current retention window in seconds.
    pub fn retention_window_secs(&self) -> u64 {
        self.retention_window_secs
    }

    /// Returns the timestamp of the last cleanup.
    pub fn last_cleanup_timestamp(&self) -> u64 {
        self.last_cleanup_timestamp
    }

    /// Returns the path to the audit log file.
    pub fn audit_log_path(&self) -> &Path {
        &self.audit_log_path
    }

    /// Returns the path to the event log file.
    pub fn event_log_path(&self) -> &Path {
        &self.event_log_path
    }

    /// Reads audit log entries within a time range.
    ///
    /// # Arguments
    ///
    /// * `start_timestamp` - Include entries purged at or after this time
    /// * `end_timestamp` - Include entries purged before this time
    ///
    /// # Returns
    ///
    /// * `Result<Vec<Value>>` - Audit entries in the time range
    pub fn read_audit_log(&self, start_timestamp: u64, end_timestamp: u64) -> Result<Vec<Value>> {
        let mut entries = Vec::new();

        if !self.audit_log_path.exists() {
            return Ok(entries);
        }

        let file = File::open(&self.audit_log_path)
            .map_err(|e| ToolError::Other(format!("Failed to open audit log: {}", e)))?;
        
        let reader = BufReader::new(file);

        for line in reader.lines() {
            if let Ok(line_text) = line {
                if let Ok(entry) = serde_json::from_str::<Value>(&line_text) {
                    if let Some(purge_ts) = entry.get("purge_timestamp").and_then(|v| v.as_u64()) {
                        if purge_ts >= start_timestamp && purge_ts < end_timestamp {
                            entries.push(entry);
                        }
                    }
                }
            }
        }

        Ok(entries)
    }

    // =========== PRIVATE HELPERS ===========

    fn save_metadata(&self, stats: &CleanupStats) -> Result<()> {
        let metadata = json!({
            "last_cleanup": stats.cleanup_timestamp,
            "events_purged": stats.events_purged,
            "bytes_removed": stats.bytes_removed,
            "retention_window_secs": self.retention_window_secs,
            "version": 1,
        });

        let json_str = serde_json::to_string_pretty(&metadata)
            .map_err(|e| ToolError::Other(format!("Failed to serialize metadata: {}", e)))?;

        fs::write(&self.metadata_path, json_str)
            .map_err(|e| ToolError::Other(format!("Failed to write metadata: {}", e)))?;

        Ok(())
    }

    fn load_metadata(metadata_path: &Path) -> Option<Value> {
        let content = fs::read_to_string(metadata_path).ok()?;
        serde_json::from_str(&content).ok()
    }
}

/// Returns current time as seconds since UNIX_EPOCH.
fn now_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

    #[test]
    fn test_create_retention_policy() -> Result<()> {
        let temp_dir = TempDir::new().map_err(|e| {
            ToolError::Other(format!("Temp dir failed: {}", e))
        })?;
        
        let policy = RetentionPolicy::new(temp_dir.path())?;
        assert_eq!(policy.retention_window_secs(), DEFAULT_RETENTION_SECS);
        Ok(())
    }

    #[test]
    fn test_custom_retention_window() -> Result<()> {
        let temp_dir = TempDir::new().map_err(|e| {
            ToolError::Other(format!("Temp dir failed: {}", e))
        })?;
        
        let custom_window = 3600; // 1 hour
        let policy = RetentionPolicy::with_window(temp_dir.path(), custom_window)?;
        assert_eq!(policy.retention_window_secs(), custom_window);
        Ok(())
    }

    #[test]
    fn test_cleanup_no_events() -> Result<()> {
        let temp_dir = TempDir::new().map_err(|e| {
            ToolError::Other(format!("Temp dir failed: {}", e))
        })?;
        
        let mut policy = RetentionPolicy::new(temp_dir.path())?;
        let stats = policy.run_cleanup()?;
        
        assert_eq!(stats.events_purged, 0);
        assert_eq!(stats.bytes_removed, 0);
        Ok(())
    }

    #[test]
    fn test_audit_log_creation() -> Result<()> {
        let temp_dir = TempDir::new().map_err(|e| {
            ToolError::Other(format!("Temp dir failed: {}", e))
        })?;
        
        let policy = RetentionPolicy::new(temp_dir.path())?;
        assert!(policy.audit_log_path().exists());
        Ok(())
    }

    #[test]
    fn test_read_audit_log_empty() -> Result<()> {
        let temp_dir = TempDir::new().map_err(|e| {
            ToolError::Other(format!("Temp dir failed: {}", e))
        })?;
        
        let policy = RetentionPolicy::new(temp_dir.path())?;
        let entries = policy.read_audit_log(0, u64::MAX)?;
        
        assert_eq!(entries.len(), 0);
        Ok(())
    }

    #[test]
    fn test_cleanup_stats_creation() -> Result<()> {
        let stats = CleanupStats::new(1000);
        assert_eq!(stats.cleanup_timestamp, 1000);
        assert_eq!(stats.events_purged, 0);
        Ok(())
    }

    #[test]
    fn test_metadata_persistence() -> Result<()> {
        let temp_dir = TempDir::new().map_err(|e| {
            ToolError::Other(format!("Temp dir failed: {}", e))
        })?;
        
        let mut policy = RetentionPolicy::new(temp_dir.path())?;
        let _ = policy.run_cleanup()?;
        
        let mut policy2 = RetentionPolicy::new(temp_dir.path())?;
        assert!(policy2.last_cleanup_timestamp() > 0);
        Ok(())
    }
}
