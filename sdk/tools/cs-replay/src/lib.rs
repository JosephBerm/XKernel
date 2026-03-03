// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! # Cognitive Substrate Replay Tool (cs-replay)
//!
//! The cs-replay crate provides deterministic replay of cognitive task execution
//! for debugging and analysis purposes.
//!
//! ## Architecture
//!
//! This crate defines:
//! - **ReplaySession**: Replay session management
//! - **ReplayConfig**: Replay configuration parameters
//! - **ReplayResult**: Replay execution results

#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use serde::{Deserialize, Serialize};
use alloc::string::ToString;

/// Replay mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplayMode {
    /// Step through execution
    Stepper,
    /// Run to breakpoint
    Breakpoint,
    /// Run with recording
    Recording,
    /// Full execution
    Full,
}

impl Default for ReplayMode {
    fn default() -> Self {
        ReplayMode::Full
    }
}

/// Breakpoint specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Breakpoint {
    /// Breakpoint ID
    pub id: u32,
    /// Task ID to break on
    pub task_id: Option<String>,
    /// Syscall name to break on
    pub syscall_name: Option<String>,
    /// Is breakpoint enabled
    pub enabled: bool,
}

/// Replay configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayConfig {
    /// Replay mode
    pub mode: ReplayMode,
    /// Record file path
    pub record_file: String,
    /// Enable strict ordering
    pub strict_ordering: bool,
    /// Breakpoints
    pub breakpoints: Vec<Breakpoint>,
    /// Speed factor (1.0 = normal, 0.5 = half speed, 2.0 = double)
    pub speed_factor: f32,
}

impl Default for ReplayConfig {
    fn default() -> Self {
        ReplayConfig {
            mode: ReplayMode::Full,
            record_file: String::new(),
            strict_ordering: true,
            breakpoints: Vec::new(),
            speed_factor: 1.0,
        }
    }
}

impl ReplayConfig {
    /// Validate configuration
    pub fn validate(&self) -> bool {
        if self.record_file.is_empty() {
            return false;
        }
        if self.speed_factor <= 0.0 {
            return false;
        }
        true
    }
}

/// Replay result information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayResult {
    /// Total execution time in milliseconds
    pub total_time_ms: u64,
    /// Number of syscalls executed
    pub syscall_count: u64,
    /// Number of tasks executed
    pub task_count: u64,
    /// Number of breakpoints hit
    pub breakpoints_hit: u32,
    /// Execution status
    pub status: ReplayStatus,
    /// Additional metadata
    pub metadata: BTreeMap<String, String>,
}

/// Replay execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplayStatus {
    /// Replay completed successfully
    Success,
    /// Replay stopped at breakpoint
    StoppedAtBreakpoint,
    /// Replay aborted
    Aborted,
    /// Replay failed with error
    Failed,
}

/// Replay session
#[derive(Debug)]
pub struct ReplaySession {
    /// Session ID
    pub session_id: String,
    /// Configuration
    pub config: ReplayConfig,
    /// Is running
    pub running: bool,
    /// Current execution step
    pub current_step: u64,
    /// Results
    pub result: Option<ReplayResult>,
}

impl ReplaySession {
    /// Create new replay session
    pub fn new(session_id: String, config: ReplayConfig) -> Option<Self> {
        if !config.validate() {
            return None;
        }

        Some(ReplaySession {
            session_id,
            config,
            running: false,
            current_step: 0,
            result: None,
        })
    }

    /// Start replay
    pub fn start(&mut self) -> bool {
        if self.running {
            return false;
        }
        self.running = true;
        self.current_step = 0;
        true
    }

    /// Stop replay
    pub fn stop(&mut self) {
        self.running = false;
    }

    /// Advance one step
    pub fn step(&mut self) -> bool {
        if !self.running {
            return false;
        }
        self.current_step += 1;
        true
    }

    /// Set result
    pub fn set_result(&mut self, result: ReplayResult) {
        self.result = Some(result);
    }

    /// Get current result
    pub fn get_result(&self) -> Option<&ReplayResult> {
        self.result.as_ref()
    }

    /// Add breakpoint
    pub fn add_breakpoint(&mut self, breakpoint: Breakpoint) {
        self.config.breakpoints.push(breakpoint);
    }

    /// Remove breakpoint by ID
    pub fn remove_breakpoint(&mut self, id: u32) -> bool {
        if let Some(pos) = self.config.breakpoints.iter().position(|bp| bp.id == id) {
            self.config.breakpoints.remove(pos);
            true
        } else {
            false
        }
    }

    /// Disable breakpoint
    pub fn disable_breakpoint(&mut self, id: u32) -> bool {
        if let Some(bp) = self.config.breakpoints.iter_mut().find(|bp| bp.id == id) {
            bp.enabled = false;
            return true;
        }
        false
    }

    /// Get breakpoints for task
    pub fn get_breakpoints_for_task(&self, task_id: &str) -> Vec<&Breakpoint> {
        self.config.breakpoints.iter()
            .filter(|bp| {
                bp.enabled && bp.task_id.as_ref().map_or(false, |id| id == task_id)
            })
            .collect()
    }

    /// Get breakpoints for syscall
    pub fn get_breakpoints_for_syscall(&self, syscall_name: &str) -> Vec<&Breakpoint> {
        self.config.breakpoints.iter()
            .filter(|bp| {
                bp.enabled && bp.syscall_name.as_ref().map_or(false, |sc| sc == syscall_name)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replay_mode_default() {
        let mode = ReplayMode::default();
        assert_eq!(mode, ReplayMode::Full);
    }

    #[test]
    fn test_replay_config_default() {
        let config = ReplayConfig::default();
        assert_eq!(config.mode, ReplayMode::Full);
        assert_eq!(config.speed_factor, 1.0);
    }

    #[test]
    fn test_replay_config_validate() {
        let config = ReplayConfig {
            record_file: "test.rec".to_string(),
            speed_factor: 1.0,
            ..Default::default()
        };
        assert!(config.validate());
    }

    #[test]
    fn test_replay_config_validate_empty_file() {
        let config = ReplayConfig::default();
        assert!(!config.validate());
    }

    #[test]
    fn test_replay_config_validate_invalid_speed() {
        let config = ReplayConfig {
            record_file: "test.rec".to_string(),
            speed_factor: 0.0,
            ..Default::default()
        };
        assert!(!config.validate());
    }

    #[test]
    fn test_replay_session_creation() {
        let config = ReplayConfig {
            record_file: "test.rec".to_string(),
            ..Default::default()
        };
        let session = ReplaySession::new("session1".to_string(), config);
        assert!(session.is_some());
    }

    #[test]
    fn test_replay_session_invalid_config() {
        let config = ReplayConfig::default();
        let session = ReplaySession::new("session1".to_string(), config);
        assert!(session.is_none());
    }

    #[test]
    fn test_replay_session_start_stop() {
        let config = ReplayConfig {
            record_file: "test.rec".to_string(),
            ..Default::default()
        };
        let mut session = ReplaySession::new("session1".to_string(), config).unwrap();

        assert!(!session.running);
        assert!(session.start());
        assert!(session.running);

        session.stop();
        assert!(!session.running);
    }

    #[test]
    fn test_replay_session_start_twice() {
        let config = ReplayConfig {
            record_file: "test.rec".to_string(),
            ..Default::default()
        };
        let mut session = ReplaySession::new("session1".to_string(), config).unwrap();

        assert!(session.start());
        assert!(!session.start());
    }

    #[test]
    fn test_replay_session_step() {
        let config = ReplayConfig {
            record_file: "test.rec".to_string(),
            ..Default::default()
        };
        let mut session = ReplaySession::new("session1".to_string(), config).unwrap();

        session.start();
        assert!(session.step());
        assert_eq!(session.current_step, 1);
    }

    #[test]
    fn test_replay_session_step_not_running() {
        let config = ReplayConfig {
            record_file: "test.rec".to_string(),
            ..Default::default()
        };
        let mut session = ReplaySession::new("session1".to_string(), config).unwrap();

        assert!(!session.step());
        assert_eq!(session.current_step, 0);
    }

    #[test]
    fn test_replay_result() {
        let result = ReplayResult {
            total_time_ms: 1000,
            syscall_count: 50,
            task_count: 5,
            breakpoints_hit: 2,
            status: ReplayStatus::Success,
            metadata: BTreeMap::new(),
        };
        assert_eq!(result.syscall_count, 50);
        assert_eq!(result.status, ReplayStatus::Success);
    }

    #[test]
    fn test_replay_session_set_result() {
        let config = ReplayConfig {
            record_file: "test.rec".to_string(),
            ..Default::default()
        };
        let mut session = ReplaySession::new("session1".to_string(), config).unwrap();

        let result = ReplayResult {
            total_time_ms: 1000,
            syscall_count: 50,
            task_count: 5,
            breakpoints_hit: 0,
            status: ReplayStatus::Success,
            metadata: BTreeMap::new(),
        };

        session.set_result(result);
        assert!(session.get_result().is_some());
    }

    #[test]
    fn test_breakpoint_add_remove() {
        let config = ReplayConfig {
            record_file: "test.rec".to_string(),
            ..Default::default()
        };
        let mut session = ReplaySession::new("session1".to_string(), config).unwrap();

        let bp = Breakpoint {
            id: 1,
            task_id: Some("task1".to_string()),
            syscall_name: None,
            enabled: true,
        };

        session.add_breakpoint(bp);
        assert_eq!(session.config.breakpoints.len(), 1);

        assert!(session.remove_breakpoint(1));
        assert_eq!(session.config.breakpoints.len(), 0);
    }

    #[test]
    fn test_breakpoint_disable() {
        let config = ReplayConfig {
            record_file: "test.rec".to_string(),
            ..Default::default()
        };
        let mut session = ReplaySession::new("session1".to_string(), config).unwrap();

        let bp = Breakpoint {
            id: 1,
            task_id: Some("task1".to_string()),
            syscall_name: None,
            enabled: true,
        };

        session.add_breakpoint(bp);
        assert!(session.disable_breakpoint(1));
        assert!(!session.config.breakpoints[0].enabled);
    }

    #[test]
    fn test_get_breakpoints_for_task() {
        let config = ReplayConfig {
            record_file: "test.rec".to_string(),
            ..Default::default()
        };
        let mut session = ReplaySession::new("session1".to_string(), config).unwrap();

        let bp1 = Breakpoint {
            id: 1,
            task_id: Some("task1".to_string()),
            syscall_name: None,
            enabled: true,
        };

        let bp2 = Breakpoint {
            id: 2,
            task_id: Some("task2".to_string()),
            syscall_name: None,
            enabled: true,
        };

        session.add_breakpoint(bp1);
        session.add_breakpoint(bp2);

        let bps = session.get_breakpoints_for_task("task1");
        assert_eq!(bps.len(), 1);
    }

    #[test]
    fn test_get_breakpoints_for_syscall() {
        let config = ReplayConfig {
            record_file: "test.rec".to_string(),
            ..Default::default()
        };
        let mut session = ReplaySession::new("session1".to_string(), config).unwrap();

        let bp = Breakpoint {
            id: 1,
            task_id: None,
            syscall_name: Some("spawn_task".to_string()),
            enabled: true,
        };

        session.add_breakpoint(bp);

        let bps = session.get_breakpoints_for_syscall("spawn_task");
        assert_eq!(bps.len(), 1);
    }
}
