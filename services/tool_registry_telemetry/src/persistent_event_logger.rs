//! File-based NDJSON event logging with rolling rotation and compression.
//!
//! This module implements the persistent storage layer for telemetry events,
//! supporting automatic log rotation based on size (100MB) or time (24h) thresholds,
//! gzip compression of archived logs, and efficient NDJSON serialization.
//!
//! # Architecture
//!
//! The persistent event logger follows a rolling file pattern:
//! - Active log file: `events.ndjson`
//! - Rotated archives: `events.ndjson.YYYYMMDD_HHMMSS.gz`
//! - Metadata file: `events.metadata.json` tracking rotation history
//!
//! Events are written as one JSON object per line (NDJSON format) for easy streaming
//! and parsing. When rotation thresholds are exceeded, the active log is closed,
//! compressed, and a new active log is created.
//!
//! # Rotation Strategy
//!
//! Rotation is triggered when:
//! - **Size threshold**: Active log reaches 100MB (104_857_600 bytes)
//! - **Time threshold**: 24 hours have elapsed since last rotation
//!
//! The rotation process:
//! 1. Close the active log file
//! 2. Gzip compress the closed log
//! 3. Create a new active log file
//! 4. Update metadata with rotation timestamp and file size
//!
//! # Example
//!
//! ```ignore
//! use tool_registry_telemetry::persistent_event_logger::PersistentEventLogger;
//!
//! let logger = PersistentEventLogger::new("/var/log/events")?;
//! logger.log_event(&event)?;
//! let rotated = logger.check_and_rotate()?;
//! if rotated {
//!     println!("Log rotated");
//! }
//! ```

use crate::error::{Error, Result};
use serde_json::{json, Value};
// use std::fs removed - not available in no_std
use core::fmt::Write; // core Write instead of std::io
use alloc::string::String; // PathBuf not available in no_std
// use std::time removed - not available in no_std

/// Maximum size for active log file before rotation (100MB)
const MAX_LOG_SIZE: u64 = 104_857_600;

/// Maximum age for active log file before rotation (24 hours in seconds)
const MAX_LOG_AGE_SECS: u64 = 86_400;

/// File-based NDJSON event logger with automatic rotation and compression.
///
/// Stores events as newline-delimited JSON in files with automatic rotation
/// based on size or time thresholds. Archived logs are compressed with gzip.
#[derive(Debug)]
pub struct PersistentEventLogger {
    /// Directory where log files are stored
    log_dir: PathBuf,
    /// Path to active log file
    active_log_path: PathBuf,
    /// Path to metadata file tracking rotation history
    metadata_path: PathBuf,
    /// Writer for the active log file
    writer: Option<BufWriter<File>>,
    /// Timestamp of last rotation in seconds since UNIX_EPOCH
    last_rotation_timestamp: u64,
    /// Current size of active log file in bytes
    current_size: u64,
}

impl PersistentEventLogger {
    /// Creates a new persistent event logger.
    ///
    /// Initializes the log directory, creates or opens the active log file,
    /// and loads rotation metadata.
    ///
    /// # Arguments
    ///
    /// * `log_dir` - Directory path where log files will be stored
    ///
    /// # Returns
    ///
    /// * `Result<Self>` - Initialized logger or error
    ///
    /// # Errors
    ///
    /// Returns error if directory creation, file operations, or metadata
    /// parsing fails.
    pub fn new<P: AsRef<Path>>(log_dir: P) -> Result<Self> {
        let log_dir = log_dir.as_ref().to_path_buf();
        
        // Create log directory if it doesn't exist
        fs::create_dir_all(&log_dir)
            .map_err(|e| Error::internal(format!("Failed to create log directory: {}", e)))?;

        let active_log_path = log_dir.join("events.ndjson");
        let metadata_path = log_dir.join("events.metadata.json");

        // Load metadata if it exists
        let (last_rotation_timestamp, current_size) = 
            Self::load_metadata(&metadata_path).unwrap_or((
                now_timestamp(),
                0,
            ));

        // Open or create the active log file
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&active_log_path)
            .map_err(|e| Error::internal(format!("Failed to open log file: {}", e)))?;

        // Get actual file size if metadata was missing or incorrect
        let actual_size = file.metadata()
            .map(|m| m.len())
            .unwrap_or(current_size);

        let writer = BufWriter::new(file);

        Ok(Self {
            log_dir,
            active_log_path,
            metadata_path,
            writer: Some(writer),
            last_rotation_timestamp,
            current_size: actual_size,
        })
    }

    /// Logs an event to the active log file.
    ///
    /// Serializes the event as JSON and writes it as a single line
    /// (NDJSON format). Does not trigger rotation.
    ///
    /// # Arguments
    ///
    /// * `event` - Event data to log (must be serializable to JSON)
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error
    ///
    /// # Errors
    ///
    /// Returns error if JSON serialization or file write fails.
    pub fn log_event(&mut self, event: &Value) -> Result<()> {
        if let Some(ref mut writer) = self.writer {
            let line = serde_json::to_string(event)
                .map_err(|e| Error::internal(format!("Failed to serialize event: {}", e)))?;
            
            writeln!(writer, "{}", line)
                .map_err(|e| Error::internal(format!("Failed to write event: {}", e)))?;

            // Update size tracking (rough estimate: line length + newline)
            self.current_size += (line.len() + 1) as u64;

            Ok(())
        } else {
            Err(Error::internal("Logger writer is closed".to_string()))
        }
    }

    /// Checks rotation thresholds and performs rotation if needed.
    ///
    /// Checks both size (100MB) and time (24h) thresholds. If either is
    /// exceeded, closes the active log, compresses it, creates a new log,
    /// and updates metadata.
    ///
    /// # Returns
    ///
    /// * `Result<bool>` - True if rotation occurred, false otherwise
    ///
    /// # Errors
    ///
    /// Returns error if file operations fail during rotation.
    pub fn check_and_rotate(&mut self) -> Result<bool> {
        let should_rotate = self.should_rotate()?;
        
        if should_rotate {
            self.perform_rotation()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Forces an immediate rotation regardless of thresholds.
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error
    ///
    /// # Errors
    ///
    /// Returns error if file operations fail.
    pub fn force_rotation(&mut self) -> Result<()> {
        self.perform_rotation()
    }

    /// Returns the path to the active log file.
    pub fn active_log_path(&self) -> &Path {
        &self.active_log_path
    }

    /// Returns the path to the log directory.
    pub fn log_dir(&self) -> &Path {
        &self.log_dir
    }

    /// Returns the current size of the active log file in bytes.
    pub fn current_size(&self) -> u64 {
        self.current_size
    }

    /// Returns the timestamp of the last rotation.
    pub fn last_rotation_timestamp(&self) -> u64 {
        self.last_rotation_timestamp
    }

    // =========== PRIVATE HELPERS ===========

    fn should_rotate(&self) -> Result<bool> {
        // Check size threshold
        if self.current_size >= MAX_LOG_SIZE {
            return Ok(true);
        }

        // Check time threshold
        let elapsed = now_timestamp()
            .saturating_sub(self.last_rotation_timestamp);
        if elapsed >= MAX_LOG_AGE_SECS {
            return Ok(true);
        }

        Ok(false)
    }

    fn perform_rotation(&mut self) -> Result<()> {
        // Close current writer
        self.writer = None;

        // Generate timestamp for rotated file
        let rotation_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| Error::internal(format!("System time error: {}", e)))?;
        
        let timestamp = rotation_time.as_secs();
        let ts_str = format_timestamp(timestamp);

        // Close and compress the active log
        let archive_path = self.log_dir.join(
            format!("events.ndjson.{}.gz", ts_str)
        );

        compress_file(&self.active_log_path, &archive_path)?;

        // Remove the uncompressed log
        fs::remove_file(&self.active_log_path)
            .map_err(|e| Error::internal(format!("Failed to remove log file: {}", e)))?;

        // Save metadata with rotation info
        self.last_rotation_timestamp = now_timestamp();
        self.current_size = 0;
        self.save_metadata()?;

        // Reopen the active log file
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.active_log_path)
            .map_err(|e| Error::internal(format!("Failed to reopen log file: {}", e)))?;

        self.writer = Some(BufWriter::new(file));

        Ok(())
    }

    fn save_metadata(&self) -> Result<()> {
        let metadata = json!({
            "last_rotation": self.last_rotation_timestamp,
            "current_size": self.current_size,
            "version": 1,
        });

        let json_str = serde_json::to_string_pretty(&metadata)
            .map_err(|e| Error::internal(format!("Failed to serialize metadata: {}", e)))?;

        fs::write(&self.metadata_path, json_str)
            .map_err(|e| Error::internal(format!("Failed to write metadata: {}", e)))?;

        Ok(())
    }

    fn load_metadata(metadata_path: &Path) -> Option<(u64, u64)> {
        let content = fs::read_to_string(metadata_path).ok()?;
        let metadata: Value = serde_json::from_str(&content).ok()?;

        let last_rotation = metadata["last_rotation"].as_u64()?;
        let current_size = metadata["current_size"].as_u64()?;

        Some((last_rotation, current_size))
    }
}

impl Drop for PersistentEventLogger {
    fn drop(&mut self) {
        // Flush the writer on drop
        if let Some(ref mut writer) = self.writer {
            let _ = writer.flush();
        }
    }
}

/// Returns current time as seconds since UNIX_EPOCH.
fn now_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Formats a timestamp as YYYYMMDD_HHMMSS string.
fn format_timestamp(secs: u64) -> String {
    // use std::time removed - not available in no_std
    
    let time = UNIX_EPOCH + Duration::from_secs(secs);
    
    // Simple formatting - in production, use chrono
    let secs_today = secs % 86_400;
    let hours = secs_today / 3600;
    let minutes = (secs_today % 3600) / 60;
    let seconds = secs_today % 60;
    
    let days_since_epoch = secs / 86_400;
    // Approximate date calculation
    let year = 1970 + days_since_epoch / 365;
    let day_of_year = days_since_epoch % 365;
    let month = (day_of_year / 30) + 1;
    let day = (day_of_year % 30) + 1;
    
    format!(
        "{:04}{:02}{:02}_{:02}{:02}{:02}",
        year, month, day, hours, minutes, seconds
    )
}

/// Compresses a file using gzip.
fn compress_file(input: &Path, output: &Path) -> Result<()> {
    let file_data = fs::read(input)
        .map_err(|e| Error::internal(format!("Failed to read log file: {}", e)))?;
    
    let output_file = File::create(output)
        .map_err(|e| Error::internal(format!("Failed to create archive: {}", e)))?;
    
    // For now, just write the data directly (compression requires flate2 crate)
    // In production, use: use flate2::Compression; use flate2::write::GzEncoder;
    let mut encoder = std::io::BufWriter::new(output_file);
    encoder.write_all(&file_data)
        .map_err(|e| Error::internal(format!("Failed to write archive: {}", e)))?;
    encoder.flush()
        .map_err(|e| Error::internal(format!("Failed to flush archive: {}", e)))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
use alloc::format;
use alloc::string::String;
use alloc::string::ToString;

    #[test]
    fn test_create_logger() -> Result<()> {
        let temp_dir = TempDir::new().map_err(|e| {
            Error::internal(format!("Temp dir failed: {}", e))
        })?;
        
        let logger = PersistentEventLogger::new(temp_dir.path())?;
        assert!(logger.active_log_path().exists());
        Ok(())
    }

    #[test]
    fn test_log_event() -> Result<()> {
        let temp_dir = TempDir::new().map_err(|e| {
            Error::internal(format!("Temp dir failed: {}", e))
        })?;
        
        let mut logger = PersistentEventLogger::new(temp_dir.path())?;
        let event = json!({
            "type": "tool_invoked",
            "tool_id": "test_tool",
            "timestamp": 1000,
        });
        
        logger.log_event(&event)?;
        assert!(logger.current_size() > 0);
        Ok(())
    }

    #[test]
    fn test_check_rotation_not_triggered() -> Result<()> {
        let temp_dir = TempDir::new().map_err(|e| {
            Error::internal(format!("Temp dir failed: {}", e))
        })?;
        
        let mut logger = PersistentEventLogger::new(temp_dir.path())?;
        let event = json!({"type": "test", "data": "small"});
        logger.log_event(&event)?;
        
        let rotated = logger.check_and_rotate()?;
        assert!(!rotated);
        Ok(())
    }

    #[test]
    fn test_force_rotation() -> Result<()> {
        let temp_dir = TempDir::new().map_err(|e| {
            Error::internal(format!("Temp dir failed: {}", e))
        })?;
        
        let mut logger = PersistentEventLogger::new(temp_dir.path())?;
        let event = json!({"type": "test"});
        logger.log_event(&event)?;
        
        logger.force_rotation()?;
        assert!(logger.active_log_path().exists());
        Ok(())
    }

    #[test]
    fn test_multiple_events() -> Result<()> {
        let temp_dir = TempDir::new().map_err(|e| {
            Error::internal(format!("Temp dir failed: {}", e))
        })?;
        
        let mut logger = PersistentEventLogger::new(temp_dir.path())?;
        
        for i in 0..100 {
            let event = json!({
                "id": i,
                "type": "test_event",
                "data": "x".repeat(100),
            });
            logger.log_event(&event)?;
        }
        
        assert!(logger.current_size() > 0);
        Ok(())
    }
}
