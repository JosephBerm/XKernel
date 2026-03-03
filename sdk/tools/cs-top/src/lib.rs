// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! # Cognitive Substrate Top Utility (cs-top)
//!
//! The cs-top crate provides real-time system monitoring similar to Unix 'top' command,
//! enabling real-time observation of cognitive task execution and resource usage.
//!
//! ## Architecture
//!
//! This crate defines:
//! - **TopSession**: Monitoring session management
//! - **TopView**: Real-time metrics display data
//! - **TopMetrics**: Individual task metrics

#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use serde::{Deserialize, Serialize};
use alloc::format;
use alloc::string::ToString;

/// Task metrics for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMetrics {
    /// Task ID
    pub task_id: String,
    /// Task name
    pub name: String,
    /// CPU usage percentage
    pub cpu_percent: f32,
    /// Memory usage in bytes
    pub memory_bytes: u64,
    /// Task state
    pub state: String,
    /// Priority level
    pub priority: u8,
    /// Execution time in milliseconds
    pub exec_time_ms: u64,
}

/// Real-time view of system metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopView {
    /// Timestamp of snapshot (nanoseconds)
    pub timestamp_ns: u64,
    /// Total tasks running
    pub total_tasks: usize,
    /// Active tasks
    pub active_tasks: usize,
    /// System CPU usage percentage
    pub total_cpu_percent: f32,
    /// System memory usage in bytes
    pub total_memory_bytes: u64,
    /// Task metrics sorted by CPU usage
    pub task_metrics: Vec<TaskMetrics>,
}

impl TopView {
    /// Create new top view
    pub fn new(timestamp_ns: u64) -> Self {
        TopView {
            timestamp_ns,
            total_tasks: 0,
            active_tasks: 0,
            total_cpu_percent: 0.0,
            total_memory_bytes: 0,
            task_metrics: Vec::new(),
        }
    }

    /// Add task metrics
    pub fn add_task(&mut self, metrics: TaskMetrics) {
        self.task_metrics.push(metrics);
    }

    /// Sort tasks by CPU usage (descending)
    pub fn sort_by_cpu(&mut self) {
        self.task_metrics.sort_by(|a, b| b.cpu_percent.partial_cmp(&a.cpu_percent).unwrap_or(core::cmp::Ordering::Equal));
    }

    /// Sort tasks by memory usage (descending)
    pub fn sort_by_memory(&mut self) {
        self.task_metrics.sort_by(|a, b| b.memory_bytes.cmp(&a.memory_bytes));
    }

    /// Get top N tasks by CPU
    pub fn get_top_cpu(&self, n: usize) -> Vec<&TaskMetrics> {
        self.task_metrics.iter()
            .take(n)
            .collect()
    }

    /// Format as text display
    pub fn format_text(&self, line_limit: usize) -> Vec<String> {
        let mut lines = Vec::new();

        // Header
        lines.push(format!("Cognitive Substrate Top - {}", self.timestamp_ns));
        lines.push(format!("Tasks: {} total, {} active", self.total_tasks, self.active_tasks));
        lines.push(format!("CPU: {:.1}% | Memory: {} bytes", self.total_cpu_percent, self.total_memory_bytes));
        lines.push(String::new());

        // Column headers
        lines.push("TASK_ID         NAME            CPU%    MEM(B)  STATE   PRIORITY TIME_MS".to_string());
        lines.push("-".repeat(80));

        // Task rows
        for task in self.task_metrics.iter().take(line_limit) {
            let line = format!(
                "{:<15} {:<15} {:<7.1} {:<7} {:<7} {:<8} {:<8}",
                &task.task_id[..core::cmp::min(15, task.task_id.len())],
                &task.name[..core::cmp::min(15, task.name.len())],
                task.cpu_percent,
                task.memory_bytes,
                &task.state[..core::cmp::min(7, task.state.len())],
                task.priority,
                task.exec_time_ms,
            );
            lines.push(line);
        }

        lines
    }

    /// Get summary statistics
    pub fn get_summary(&self) -> BTreeMap<String, String> {
        let mut summary = BTreeMap::new();
        summary.insert("timestamp_ns".to_string(), self.timestamp_ns.to_string());
        summary.insert("total_tasks".to_string(), self.total_tasks.to_string());
        summary.insert("active_tasks".to_string(), self.active_tasks.to_string());
        summary.insert("total_cpu_percent".to_string(), format!("{:.1}", self.total_cpu_percent));
        summary.insert("total_memory_bytes".to_string(), self.total_memory_bytes.to_string());
        summary
    }
}

/// Display sorting mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortMode {
    /// Sort by CPU usage
    Cpu,
    /// Sort by memory usage
    Memory,
    /// Sort by task ID
    TaskId,
    /// Sort by execution time
    Time,
}

impl Default for SortMode {
    fn default() -> Self {
        SortMode::Cpu
    }
}

/// Top monitoring session
#[derive(Debug)]
pub struct TopSession {
    /// Session ID
    pub session_id: String,
    /// Is monitoring
    pub monitoring: bool,
    /// Current view
    pub current_view: TopView,
    /// Sort mode
    pub sort_mode: SortMode,
    /// Update interval in milliseconds
    pub update_interval_ms: u64,
    /// View history
    pub history: Vec<TopView>,
}

impl TopSession {
    /// Create new top session
    pub fn new(session_id: String, update_interval_ms: u64) -> Self {
        TopSession {
            session_id,
            monitoring: false,
            current_view: TopView::new(0),
            sort_mode: SortMode::default(),
            update_interval_ms,
            history: Vec::new(),
        }
    }

    /// Start monitoring
    pub fn start(&mut self) {
        self.monitoring = true;
    }

    /// Stop monitoring
    pub fn stop(&mut self) {
        self.monitoring = false;
    }

    /// Update view
    pub fn update_view(&mut self, view: TopView) {
        if self.monitoring {
            self.history.push(self.current_view.clone());
            self.current_view = view;
            self.apply_sort();
        }
    }

    /// Apply current sort mode
    pub fn apply_sort(&mut self) {
        match self.sort_mode {
            SortMode::Cpu => self.current_view.sort_by_cpu(),
            SortMode::Memory => self.current_view.sort_by_memory(),
            SortMode::TaskId => {
                self.current_view.task_metrics.sort_by(|a, b| a.task_id.cmp(&b.task_id));
            }
            SortMode::Time => {
                self.current_view.task_metrics.sort_by(|a, b| b.exec_time_ms.cmp(&a.exec_time_ms));
            }
        }
    }

    /// Set sort mode
    pub fn set_sort_mode(&mut self, mode: SortMode) {
        self.sort_mode = mode;
        self.apply_sort();
    }

    /// Get display lines
    pub fn get_display(&self, line_limit: usize) -> Vec<String> {
        self.current_view.format_text(line_limit)
    }

    /// Get task by ID
    pub fn get_task(&self, task_id: &str) -> Option<&TaskMetrics> {
        self.current_view.task_metrics.iter()
            .find(|t| t.task_id == task_id)
    }

    /// Get history length
    pub fn history_length(&self) -> usize {
        self.history.len()
    }

    /// Get average CPU over history
    pub fn get_average_cpu(&self) -> f32 {
        if self.history.is_empty() {
            return self.current_view.total_cpu_percent;
        }

        let sum: f32 = self.history.iter().map(|v| v.total_cpu_percent).sum::<f32>() + self.current_view.total_cpu_percent;
        sum / (self.history.len() as f32 + 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_metrics() {
        let metrics = TaskMetrics {
            task_id: "task1".to_string(),
            name: "process".to_string(),
            cpu_percent: 25.5,
            memory_bytes: 1024,
            state: "Running".to_string(),
            priority: 128,
            exec_time_ms: 500,
        };
        assert_eq!(metrics.task_id, "task1");
        assert_eq!(metrics.cpu_percent, 25.5);
    }

    #[test]
    fn test_top_view_creation() {
        let view = TopView::new(1000);
        assert_eq!(view.timestamp_ns, 1000);
        assert_eq!(view.task_metrics.len(), 0);
    }

    #[test]
    fn test_top_view_add_task() {
        let mut view = TopView::new(1000);
        let metrics = TaskMetrics {
            task_id: "task1".to_string(),
            name: "process".to_string(),
            cpu_percent: 25.5,
            memory_bytes: 1024,
            state: "Running".to_string(),
            priority: 128,
            exec_time_ms: 500,
        };

        view.add_task(metrics);
        assert_eq!(view.task_metrics.len(), 1);
    }

    #[test]
    fn test_top_view_sort_by_cpu() {
        let mut view = TopView::new(1000);

        for i in 0..3 {
            let metrics = TaskMetrics {
                task_id: format!("task{}", i),
                name: "process".to_string(),
                cpu_percent: (i as f32) * 10.0,
                memory_bytes: 1024,
                state: "Running".to_string(),
                priority: 128,
                exec_time_ms: 500,
            };
            view.add_task(metrics);
        }

        view.sort_by_cpu();
        assert_eq!(view.task_metrics[0].cpu_percent, 20.0);
    }

    #[test]
    fn test_top_view_sort_by_memory() {
        let mut view = TopView::new(1000);

        for i in 0..3 {
            let metrics = TaskMetrics {
                task_id: format!("task{}", i),
                name: "process".to_string(),
                cpu_percent: 10.0,
                memory_bytes: (i as u64 + 1) * 1024,
                state: "Running".to_string(),
                priority: 128,
                exec_time_ms: 500,
            };
            view.add_task(metrics);
        }

        view.sort_by_memory();
        assert_eq!(view.task_metrics[0].memory_bytes, 3072);
    }

    #[test]
    fn test_top_view_get_top_cpu() {
        let mut view = TopView::new(1000);

        for i in 0..5 {
            let metrics = TaskMetrics {
                task_id: format!("task{}", i),
                name: "process".to_string(),
                cpu_percent: (i as f32) * 10.0,
                memory_bytes: 1024,
                state: "Running".to_string(),
                priority: 128,
                exec_time_ms: 500,
            };
            view.add_task(metrics);
        }

        view.sort_by_cpu();
        let top = view.get_top_cpu(3);
        assert_eq!(top.len(), 3);
    }

    #[test]
    fn test_top_view_format_text() {
        let mut view = TopView::new(1000);
        view.total_tasks = 1;
        view.active_tasks = 1;
        view.total_cpu_percent = 25.5;
        view.total_memory_bytes = 1024;

        let metrics = TaskMetrics {
            task_id: "task1".to_string(),
            name: "process".to_string(),
            cpu_percent: 25.5,
            memory_bytes: 1024,
            state: "Running".to_string(),
            priority: 128,
            exec_time_ms: 500,
        };

        view.add_task(metrics);

        let text = view.format_text(10);
        assert!(!text.is_empty());
        assert!(text[0].contains("Cognitive Substrate Top"));
    }

    #[test]
    fn test_top_view_get_summary() {
        let view = TopView::new(1000);
        let summary = view.get_summary();
        assert!(summary.contains_key("timestamp_ns"));
        assert!(summary.contains_key("total_tasks"));
    }

    #[test]
    fn test_sort_mode_default() {
        let mode = SortMode::default();
        assert_eq!(mode, SortMode::Cpu);
    }

    #[test]
    fn test_top_session_creation() {
        let session = TopSession::new("session1".to_string(), 1000);
        assert_eq!(session.session_id, "session1");
        assert!(!session.monitoring);
    }

    #[test]
    fn test_top_session_start_stop() {
        let mut session = TopSession::new("session1".to_string(), 1000);
        session.start();
        assert!(session.monitoring);

        session.stop();
        assert!(!session.monitoring);
    }

    #[test]
    fn test_top_session_update_view() {
        let mut session = TopSession::new("session1".to_string(), 1000);
        session.start();

        let mut view = TopView::new(2000);
        let metrics = TaskMetrics {
            task_id: "task1".to_string(),
            name: "process".to_string(),
            cpu_percent: 25.5,
            memory_bytes: 1024,
            state: "Running".to_string(),
            priority: 128,
            exec_time_ms: 500,
        };
        view.add_task(metrics);

        session.update_view(view);
        assert_eq!(session.history_length(), 1);
    }

    #[test]
    fn test_top_session_set_sort_mode() {
        let mut session = TopSession::new("session1".to_string(), 1000);
        session.set_sort_mode(SortMode::Memory);
        assert_eq!(session.sort_mode, SortMode::Memory);
    }

    #[test]
    fn test_top_session_get_display() {
        let mut session = TopSession::new("session1".to_string(), 1000);
        session.start();

        let mut view = TopView::new(2000);
        view.total_tasks = 1;

        session.update_view(view);
        let display = session.get_display(10);
        assert!(!display.is_empty());
    }

    #[test]
    fn test_top_session_get_task() {
        let mut session = TopSession::new("session1".to_string(), 1000);
        session.start();

        let mut view = TopView::new(2000);
        let metrics = TaskMetrics {
            task_id: "task1".to_string(),
            name: "process".to_string(),
            cpu_percent: 25.5,
            memory_bytes: 1024,
            state: "Running".to_string(),
            priority: 128,
            exec_time_ms: 500,
        };
        view.add_task(metrics);

        session.update_view(view);
        assert!(session.get_task("task1").is_some());
        assert!(session.get_task("task2").is_none());
    }

    #[test]
    fn test_top_session_get_average_cpu() {
        let mut session = TopSession::new("session1".to_string(), 1000);
        session.start();

        let mut view1 = TopView::new(1000);
        view1.total_cpu_percent = 50.0;
        session.update_view(view1);

        let mut view2 = TopView::new(2000);
        view2.total_cpu_percent = 100.0;
        session.update_view(view2);

        let avg = session.get_average_cpu();
        assert!(avg > 0.0 && avg <= 100.0);
    }
}
