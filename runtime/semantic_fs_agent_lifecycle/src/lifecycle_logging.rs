//! Logging Infrastructure for Lifecycle Events
//!
//! Provides structured logging for all agent lifecycle events with JSON serialization,
//! log levels, rotation support, and queryable interfaces.
//! See RFC: Week 6 Lifecycle Logging subsystem design.

use alloc::collections::VecDeque;
// use std::fs removed - not available in no_std
use core::fmt::Write; // core Write instead of std::io
use alloc::vec::Vec; // PathBuf not available in no_std
use alloc::sync::Arc; // Mutex not available in no_std
// use std::time removed - not available in no_std
use crate::error::{LifecycleError, Result};

/// Log level for lifecycle events.
///
/// Controls verbosity and filtering of logged events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    /// Debug-level logging
    Debug = 0,
    /// Info-level logging
    Info = 1,
    /// Warning-level logging
    Warn = 2,
    /// Error-level logging
    Error = 3,
}

impl LogLevel {
    /// Returns string representation of log level.
    pub fn as_str(&self) -> &str {
        match self {
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        }
    }

    /// Parse a log level from string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "DEBUG" => Some(LogLevel::Debug),
            "INFO" => Some(LogLevel::Info),
            "WARN" => Some(LogLevel::Warn),
            "ERROR" => Some(LogLevel::Error),
            _ => None,
        }
    }
}

/// Lifecycle event type.
///
/// Categorizes different types of agent lifecycle events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventType {
    /// Agent startup event
    Startup,
    /// Agent shutdown event
    Shutdown,
    /// State transition event
    StateTransition,
    /// Health check event
    HealthCheck,
    /// Error or failure event
    Error,
    /// Resource allocation event
    ResourceAllocation,
    /// Resource cleanup event
    ResourceCleanup,
}

impl EventType {
    /// Returns string representation of event type.
    pub fn as_str(&self) -> &str {
        match self {
            EventType::Startup => "STARTUP",
            EventType::Shutdown => "SHUTDOWN",
            EventType::StateTransition => "STATE_TRANSITION",
            EventType::HealthCheck => "HEALTH_CHECK",
            EventType::Error => "ERROR",
            EventType::ResourceAllocation => "RESOURCE_ALLOCATION",
            EventType::ResourceCleanup => "RESOURCE_CLEANUP",
        }
    }
}

/// Structured lifecycle log entry.
///
/// Represents a single lifecycle event with metadata and context.
#[derive(Debug, Clone)]
pub struct LogEntry {
    /// Unique log entry ID
    pub id: u64,
    /// Timestamp in milliseconds since UNIX_EPOCH
    pub timestamp_ms: u64,
    /// Log level
    pub level: LogLevel,
    /// Event type
    pub event_type: EventType,
    /// Agent identifier
    pub agent_id: String,
    /// Event message
    pub message: String,
    /// Optional context data (JSON)
    pub context: Option<String>,
}

impl LogEntry {
    /// Create a new log entry.
    pub fn new(
        id: u64,
        level: LogLevel,
        event_type: EventType,
        agent_id: String,
        message: String,
    ) -> Self {
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        Self {
            id,
            timestamp_ms,
            level,
            event_type,
            agent_id,
            message,
            context: None,
        }
    }

    /// Add context data to this entry.
    pub fn with_context(mut self, context: String) -> Self {
        self.context = Some(context);
        self
    }

    /// Serialize this entry to JSON format.
    pub fn to_json(&self) -> String {
        let context_json = self.context.as_ref()
            .map(|c| format!(r#","context":{}"#, c))
            .unwrap_or_default();

        format!(
            r#"{{"id":{},"timestamp_ms":{},"level":"{}","event_type":"{}","agent_id":"{}","message":"{}"{}}}"#,
            self.id,
            self.timestamp_ms,
            self.level.as_str(),
            self.event_type.as_str(),
            self.agent_id.replace('"', "\\\""),
            self.message.replace('"', "\\\""),
            context_json
        )
    }
}

/// Log rotation policy.
///
/// Determines when and how log files are rotated.
#[derive(Debug, Clone)]
pub struct RotationPolicy {
    /// Maximum size per log file in bytes
    pub max_size_bytes: u64,
    /// Maximum number of retained log files
    pub max_files: usize,
}

impl RotationPolicy {
    /// Create a new rotation policy with defaults.
    /// Default: 10MB per file, keep 5 files.
    pub fn new() -> Self {
        Self {
            max_size_bytes: 10 * 1024 * 1024,
            max_files: 5,
        }
    }

    /// Set maximum size per file.
    pub fn with_max_size(mut self, bytes: u64) -> Self {
        self.max_size_bytes = bytes;
        self
    }

    /// Set maximum number of files to retain.
    pub fn with_max_files(mut self, count: usize) -> Self {
        self.max_files = count;
        self
    }
}

impl Default for RotationPolicy {
    fn default() -> Self {
        Self::new()
    }
}

/// Lifecycle event logger with rotation support.
///
/// Thread-safe logging of lifecycle events with structured JSON format.
#[derive(Clone)]
pub struct LifecycleLogger {
    /// Log file path
    log_path: Arc<Mutex<Option<PathBuf>>>,
    /// Current minimum log level
    min_level: Arc<Mutex<LogLevel>>,
    /// Rotation policy
    rotation_policy: Arc<Mutex<RotationPolicy>>,
    /// In-memory log buffer (circular)
    buffer: Arc<Mutex<VecDeque<LogEntry>>>,
    /// Buffer capacity (max entries)
    buffer_capacity: usize,
    /// Next entry ID
    next_id: Arc<Mutex<u64>>,
}

impl LifecycleLogger {
    /// Create a new lifecycle logger.
    ///
    /// Initializes with INFO level and default rotation policy.
    pub fn new() -> Self {
        Self {
            log_path: Arc::new(Mutex::new(None)),
            min_level: Arc::new(Mutex::new(LogLevel::Info)),
            rotation_policy: Arc::new(Mutex::new(RotationPolicy::new())),
            buffer: Arc::new(Mutex::new(VecDeque::with_capacity(1000))),
            buffer_capacity: 1000,
            next_id: Arc::new(Mutex::new(0)),
        }
    }

    /// Set the log file path.
    ///
    /// # Arguments
    /// * `path` - Path to log file
    ///
    /// # Returns
    /// Result indicating success or error.
    pub fn set_log_path(&self, path: PathBuf) -> Result<()> {
        let mut log_path = self.log_path.lock()
            .map_err(|_| LifecycleError::LoggingError("Failed to acquire path lock".to_string()))?;
        *log_path = Some(path);
        Ok(())
    }

    /// Set the minimum log level.
    ///
    /// # Arguments
    /// * `level` - Minimum level to log
    ///
    /// # Returns
    /// Result indicating success or error.
    pub fn set_level(&self, level: LogLevel) -> Result<()> {
        let mut min_level = self.min_level.lock()
            .map_err(|_| LifecycleError::LoggingError("Failed to acquire level lock".to_string()))?;
        *min_level = level;
        Ok(())
    }

    /// Set the rotation policy.
    ///
    /// # Arguments
    /// * `policy` - Rotation policy to apply
    ///
    /// # Returns
    /// Result indicating success or error.
    pub fn set_rotation_policy(&self, policy: RotationPolicy) -> Result<()> {
        let mut rotation = self.rotation_policy.lock()
            .map_err(|_| LifecycleError::LoggingError("Failed to acquire policy lock".to_string()))?;
        *rotation = policy;
        Ok(())
    }

    /// Log an event at debug level.
    pub fn debug(&self, event_type: EventType, agent_id: String, message: String) -> Result<()> {
        self.log(LogLevel::Debug, event_type, agent_id, message)
    }

    /// Log an event at info level.
    pub fn info(&self, event_type: EventType, agent_id: String, message: String) -> Result<()> {
        self.log(LogLevel::Info, event_type, agent_id, message)
    }

    /// Log an event at warn level.
    pub fn warn(&self, event_type: EventType, agent_id: String, message: String) -> Result<()> {
        self.log(LogLevel::Warn, event_type, agent_id, message)
    }

    /// Log an event at error level.
    pub fn error(&self, event_type: EventType, agent_id: String, message: String) -> Result<()> {
        self.log(LogLevel::Error, event_type, agent_id, message)
    }

    /// Log an event with specified level.
    ///
    /// # Arguments
    /// * `level` - Log level
    /// * `event_type` - Type of event
    /// * `agent_id` - Agent identifier
    /// * `message` - Log message
    ///
    /// # Returns
    /// Result indicating success or error.
    pub fn log(&self, level: LogLevel, event_type: EventType, agent_id: String, message: String) -> Result<()> {
        let min_level = *self.min_level.lock()
            .map_err(|_| LifecycleError::LoggingError("Failed to acquire level lock".to_string()))?;

        if level < min_level {
            return Ok(());
        }

        let mut next_id = self.next_id.lock()
            .map_err(|_| LifecycleError::LoggingError("Failed to acquire ID lock".to_string()))?;
        let id = *next_id;
        *next_id += 1;

        let entry = LogEntry::new(id, level, event_type, agent_id, message);
        self.write_entry(&entry)?;
        Ok(())
    }

    /// Write an entry to the log.
    fn write_entry(&self, entry: &LogEntry) -> Result<()> {
        // Add to in-memory buffer
        let mut buffer = self.buffer.lock()
            .map_err(|_| LifecycleError::LoggingError("Failed to acquire buffer lock".to_string()))?;

        if buffer.len() >= self.buffer_capacity {
            buffer.pop_front();
        }
        buffer.push_back(entry.clone());
        drop(buffer);

        // Write to file if path is set
        if let Ok(log_path_opt) = self.log_path.lock() {
            if let Some(log_path) = log_path_opt.as_ref() {
                let json = entry.to_json();
                let mut file = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(log_path)
                    .map_err(|e| LifecycleError::LoggingError(format!("Failed to open log file: {}", e)))?;

                writeln!(file, "{}", json)
                    .map_err(|e| LifecycleError::LoggingError(format!("Failed to write log entry: {}", e)))?;
            }
        }

        Ok(())
    }

    /// Get all logged entries.
    ///
    /// # Returns
    /// Result containing vector of log entries.
    pub fn get_entries(&self) -> Result<Vec<LogEntry>> {
        let buffer = self.buffer.lock()
            .map_err(|_| LifecycleError::LoggingError("Failed to acquire buffer lock".to_string()))?;
        Ok(buffer.iter().cloned().collect())
    }

    /// Get log entries matching filter criteria.
    ///
    /// # Arguments
    /// * `level` - Minimum log level to include
    /// * `agent_id` - Optional agent ID filter (None matches all)
    /// * `event_type` - Optional event type filter (None matches all)
    ///
    /// # Returns
    /// Result containing filtered log entries.
    pub fn query(&self, level: LogLevel, agent_id: Option<&str>, event_type: Option<&EventType>) -> Result<Vec<LogEntry>> {
        let buffer = self.buffer.lock()
            .map_err(|_| LifecycleError::LoggingError("Failed to acquire buffer lock".to_string()))?;

        let entries: Vec<LogEntry> = buffer.iter()
            .filter(|e| {
                e.level >= level &&
                agent_id.map_or(true, |id| e.agent_id == id) &&
                event_type.map_or(true, |et| &e.event_type == et)
            })
            .cloned()
            .collect();

        Ok(entries)
    }

    /// Get recent entries (tail functionality).
    ///
    /// # Arguments
    /// * `count` - Number of recent entries to return
    ///
    /// # Returns
    /// Result containing recent log entries.
    pub fn tail(&self, count: usize) -> Result<Vec<LogEntry>> {
        let buffer = self.buffer.lock()
            .map_err(|_| LifecycleError::LoggingError("Failed to acquire buffer lock".to_string()))?;

        let skip = if buffer.len() > count {
            buffer.len() - count
        } else {
            0
        };

        Ok(buffer.iter().skip(skip).cloned().collect())
    }

    /// Get log statistics.
    ///
    /// # Returns
    /// Result containing (total_entries, error_count, warn_count).
    pub fn stats(&self) -> Result<(usize, usize, usize)> {
        let buffer = self.buffer.lock()
            .map_err(|_| LifecycleError::LoggingError("Failed to acquire buffer lock".to_string()))?;

        let total = buffer.len();
        let error_count = buffer.iter().filter(|e| e.level == LogLevel::Error).count();
        let warn_count = buffer.iter().filter(|e| e.level == LogLevel::Warn).count();

        Ok((total, error_count, warn_count))
    }

    /// Clear all in-memory log entries.
    ///
    /// Note: This does not affect persisted log files.
    pub fn clear(&self) -> Result<()> {
        let mut buffer = self.buffer.lock()
            .map_err(|_| LifecycleError::LoggingError("Failed to acquire buffer lock".to_string()))?;
        buffer.clear();
        Ok(())
    }
}

impl Default for LifecycleLogger {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use alloc::sync::Arc;
use alloc::vec::Vec;

    #[test]
    fn test_log_level_ordering() {
        assert!(LogLevel::Debug < LogLevel::Info);
        assert!(LogLevel::Info < LogLevel::Warn);
        assert!(LogLevel::Warn < LogLevel::Error);
    }

    #[test]
    fn test_log_level_as_str() {
        assert_eq!(LogLevel::Debug.as_str(), "DEBUG");
        assert_eq!(LogLevel::Info.as_str(), "INFO");
        assert_eq!(LogLevel::Warn.as_str(), "WARN");
        assert_eq!(LogLevel::Error.as_str(), "ERROR");
    }

    #[test]
    fn test_log_level_from_str() {
        assert_eq!(LogLevel::from_str("DEBUG"), Some(LogLevel::Debug));
        assert_eq!(LogLevel::from_str("INFO"), Some(LogLevel::Info));
        assert_eq!(LogLevel::from_str("debug"), Some(LogLevel::Debug));
        assert_eq!(LogLevel::from_str("invalid"), None);
    }

    #[test]
    fn test_event_type_as_str() {
        assert_eq!(EventType::Startup.as_str(), "STARTUP");
        assert_eq!(EventType::Shutdown.as_str(), "SHUTDOWN");
        assert_eq!(EventType::Error.as_str(), "ERROR");
    }

    #[test]
    fn test_log_entry_creation() {
        let entry = LogEntry::new(
            1,
            LogLevel::Info,
            EventType::Startup,
            "agent-1".to_string(),
            "Agent started".to_string(),
        );

        assert_eq!(entry.id, 1);
        assert_eq!(entry.level, LogLevel::Info);
        assert_eq!(entry.event_type, EventType::Startup);
        assert_eq!(entry.agent_id, "agent-1");
        assert_eq!(entry.message, "Agent started");
    }

    #[test]
    fn test_log_entry_with_context() {
        let entry = LogEntry::new(
            1,
            LogLevel::Error,
            EventType::Error,
            "agent-1".to_string(),
            "Startup failed".to_string(),
        ).with_context(r#"{"error":"timeout"}"#.to_string());

        assert_eq!(entry.context, Some(r#"{"error":"timeout"}"#.to_string()));
    }

    #[test]
    fn test_log_entry_to_json() {
        let entry = LogEntry::new(
            1,
            LogLevel::Info,
            EventType::Startup,
            "agent-1".to_string(),
            "Started".to_string(),
        );

        let json = entry.to_json();
        assert!(json.contains(r#""id":1"#));
        assert!(json.contains(r#""level":"INFO""#));
        assert!(json.contains(r#""agent_id":"agent-1""#));
    }

    #[test]
    fn test_rotation_policy_defaults() {
        let policy = RotationPolicy::new();
        assert_eq!(policy.max_size_bytes, 10 * 1024 * 1024);
        assert_eq!(policy.max_files, 5);
    }

    #[test]
    fn test_rotation_policy_builder() {
        let policy = RotationPolicy::new()
            .with_max_size(100 * 1024 * 1024)
            .with_max_files(10);

        assert_eq!(policy.max_size_bytes, 100 * 1024 * 1024);
        assert_eq!(policy.max_files, 10);
    }

    #[test]
    fn test_lifecycle_logger_creation() {
        let logger = LifecycleLogger::new();
        let (total, errors, warns) = logger.stats().unwrap();
        assert_eq!(total, 0);
        assert_eq!(errors, 0);
        assert_eq!(warns, 0);
    }

    #[test]
    fn test_lifecycle_logger_info() {
        let logger = LifecycleLogger::new();
        logger.info(EventType::Startup, "agent-1".to_string(), "Started".to_string()).unwrap();
        let entries = logger.get_entries().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].level, LogLevel::Info);
    }

    #[test]
    fn test_lifecycle_logger_error() {
        let logger = LifecycleLogger::new();
        logger.error(EventType::Error, "agent-1".to_string(), "Failed".to_string()).unwrap();
        let entries = logger.get_entries().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].level, LogLevel::Error);
    }

    #[test]
    fn test_lifecycle_logger_level_filtering() {
        let logger = LifecycleLogger::new();
        logger.set_level(LogLevel::Warn).unwrap();

        logger.debug(EventType::Startup, "agent-1".to_string(), "Debug".to_string()).unwrap();
        logger.info(EventType::Startup, "agent-1".to_string(), "Info".to_string()).unwrap();
        logger.warn(EventType::Startup, "agent-1".to_string(), "Warn".to_string()).unwrap();

        let entries = logger.get_entries().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].level, LogLevel::Warn);
    }

    #[test]
    fn test_lifecycle_logger_query_by_agent() {
        let logger = LifecycleLogger::new();
        logger.info(EventType::Startup, "agent-1".to_string(), "Started".to_string()).unwrap();
        logger.info(EventType::Startup, "agent-2".to_string(), "Started".to_string()).unwrap();

        let entries = logger.query(LogLevel::Debug, Some("agent-1"), None).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].agent_id, "agent-1");
    }

    #[test]
    fn test_lifecycle_logger_query_by_event_type() {
        let logger = LifecycleLogger::new();
        logger.info(EventType::Startup, "agent-1".to_string(), "Started".to_string()).unwrap();
        logger.info(EventType::Shutdown, "agent-1".to_string(), "Stopped".to_string()).unwrap();

        let entries = logger.query(LogLevel::Debug, None, Some(&EventType::Startup)).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].event_type, EventType::Startup);
    }

    #[test]
    fn test_lifecycle_logger_tail() {
        let logger = LifecycleLogger::new();
        for i in 0..10 {
            logger.info(EventType::Startup, format!("agent-{}", i), "Started".to_string()).unwrap();
        }

        let recent = logger.tail(5).unwrap();
        assert_eq!(recent.len(), 5);
        assert_eq!(recent[0].id, 5);
        assert_eq!(recent[4].id, 9);
    }

    #[test]
    fn test_lifecycle_logger_stats() {
        let logger = LifecycleLogger::new();
        logger.info(EventType::Startup, "agent-1".to_string(), "Started".to_string()).unwrap();
        logger.warn(EventType::Startup, "agent-1".to_string(), "Warning".to_string()).unwrap();
        logger.error(EventType::Error, "agent-1".to_string(), "Failed".to_string()).unwrap();

        let (total, errors, warns) = logger.stats().unwrap();
        assert_eq!(total, 3);
        assert_eq!(errors, 1);
        assert_eq!(warns, 1);
    }

    #[test]
    fn test_lifecycle_logger_clear() {
        let logger = LifecycleLogger::new();
        logger.info(EventType::Startup, "agent-1".to_string(), "Started".to_string()).unwrap();
        assert_eq!(logger.get_entries().unwrap().len(), 1);

        logger.clear().unwrap();
        assert_eq!(logger.get_entries().unwrap().len(), 0);
    }

    #[test]
    fn test_lifecycle_logger_set_rotation_policy() {
        let logger = LifecycleLogger::new();
        let policy = RotationPolicy::new().with_max_size(50_000_000);
        logger.set_rotation_policy(policy).unwrap();
        let rotation = logger.rotation_policy.lock().unwrap();
        assert_eq!(rotation.max_size_bytes, 50_000_000);
    }
}
