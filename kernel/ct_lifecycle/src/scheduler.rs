// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 XKernal Contributors
//! Priority queue-based scheduler for fair task scheduling

use alloc::collections::BinaryHeap;
use crate::types::Priority;
use core::cmp::Ordering;
use thiserror::Error;

/// Errors that can occur in the scheduler
#[derive(Debug, Clone, Error)]
pub enum SchedulingError {
    /// Queue is empty
    #[error("scheduler queue is empty")]
    EmptyQueue,
    /// Task already exists in scheduler
    #[error("task {0} already scheduled")]
    TaskExists(u64),
    /// Task not found in scheduler
    #[error("task {0} not found")]
    TaskNotFound(u64),
    /// Invalid priority value
    #[error("invalid priority: {0}")]
    InvalidPriority(alloc::string::String),
}

pub type Result<T> = core::result::Result<T, SchedulingError>;

/// Wrapper for priority-ordered task in binary heap
#[derive(Debug, Clone)]
struct PrioritizedTask {
    task_id: u64,
    priority: Priority,
}

impl Eq for PrioritizedTask {}

impl PartialEq for PrioritizedTask {
    fn eq(&self, other: &Self) -> bool {
        self.task_id == other.task_id
    }
}

impl Ord for PrioritizedTask {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for max-heap behavior (higher priority first)
        other.priority.cmp(&self.priority)
            .then_with(|| other.task_id.cmp(&self.task_id))
    }
}

impl PartialOrd for PrioritizedTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Priority-queue based fair scheduler for cognitive tasks
#[derive(Debug)]
pub struct PriorityScheduler {
    queue: BinaryHeap<PrioritizedTask>,
    next_task_id: u64,
}

impl PriorityScheduler {
    /// Create a new priority scheduler
    pub fn new() -> Self {
        Self {
            queue: BinaryHeap::new(),
            next_task_id: 0,
        }
    }

    /// Enqueue a task with its priority
    pub fn enqueue(&mut self, task_id: u64, priority: Priority) -> Result<()> {
        // Check if task already exists
        if self.queue.iter().any(|t| t.task_id == task_id) {
            return Err(SchedulingError::TaskExists(task_id));
        }

        self.queue.push(PrioritizedTask {
            task_id,
            priority,
        });

        Ok(())
    }

    /// Dequeue the next task to execute (highest priority)
    pub fn dequeue(&mut self) -> Result<(u64, Priority)> {
        self.queue
            .pop()
            .map(|t| (t.task_id, t.priority))
            .ok_or(SchedulingError::EmptyQueue)
    }

    /// Peek at the next task without removing it
    pub fn peek(&self) -> Option<(u64, Priority)> {
        self.queue.peek().map(|t| (t.task_id, t.priority))
    }

    /// Remove a task from the scheduler
    pub fn remove(&mut self, task_id: u64) -> Result<Priority> {
        let initial_len = self.queue.len();

        // Create a new queue without the task
        let mut new_queue = BinaryHeap::new();
        let mut removed_priority = None;

        for task in self.queue.drain() {
            if task.task_id == task_id {
                removed_priority = Some(task.priority);
            } else {
                new_queue.push(task);
            }
        }

        self.queue = new_queue;

        if self.queue.len() == initial_len - 1 {
            removed_priority.ok_or(SchedulingError::TaskNotFound(task_id))
        } else {
            Err(SchedulingError::TaskNotFound(task_id))
        }
    }

    /// Update a task's priority
    pub fn update_priority(&mut self, task_id: u64, new_priority: Priority) -> Result<()> {
        self.remove(task_id)?;
        self.enqueue(task_id, new_priority)
    }

    /// Get the current queue length
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// Check if the queue is empty
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Yield execution for the current task (move to back of queue)
    pub fn yield_now(&mut self) -> Result<Option<u64>> {
        match self.dequeue() {
            Ok((task_id, priority)) => {
                self.enqueue(task_id, priority)?;
                Ok(Some(task_id))
            }
            Err(SchedulingError::EmptyQueue) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Create a checkpoint for the current task
    pub fn checkpoint(&self, task_id: u64) -> Result<u64> {
        if self.queue.iter().any(|t| t.task_id == task_id) {
            Ok(task_id)
        } else {
            Err(SchedulingError::TaskNotFound(task_id))
        }
    }
}

impl Default for PriorityScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enqueue_dequeue() {
        let mut scheduler = PriorityScheduler::new();
        let priority = Priority::new(1, 1, 0, 0);

        scheduler.enqueue(1, priority).unwrap();
        let (id, _) = scheduler.dequeue().unwrap();
        assert_eq!(id, 1);
    }

    #[test]
    fn test_priority_ordering() {
        let mut scheduler = PriorityScheduler::new();
        let high_pri = Priority::new(10, 1, 0, 0);
        let low_pri = Priority::new(1, 1, 0, 0);

        scheduler.enqueue(1, low_pri).unwrap();
        scheduler.enqueue(2, high_pri).unwrap();

        let (id, _) = scheduler.dequeue().unwrap();
        assert_eq!(id, 2);
    }

    #[test]
    fn test_empty_queue() {
        let mut scheduler: PriorityScheduler = PriorityScheduler::new();
        assert!(scheduler.dequeue().is_err());
    }
}
