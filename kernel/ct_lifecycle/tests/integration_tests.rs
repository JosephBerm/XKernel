// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 XKernal Contributors
//! Integration tests for ct-lifecycle crate

use ct_lifecycle::*;

#[test]
fn test_task_creation_and_scheduling() {
    let task = CognitiveTask::new(
        TaskId::new(1),
        Priority::normal(),
        0x0F,
        None,
    );

    assert_eq!(task.id, TaskId::new(1));
    assert_eq!(task.phase, TaskPhase::Init);
    assert_eq!(task.capabilities, 0x0F);
}

#[test]
fn test_state_machine_transitions() {
    let mut sm = TaskStateMachine::new(42);
    
    assert_eq!(sm.current_state(), TaskState::Init);
    assert!(sm.transition(TaskState::Ready).is_ok());
    assert_eq!(sm.current_state(), TaskState::Ready);
    assert!(sm.transition(TaskState::Running).is_ok());
    assert_eq!(sm.current_state(), TaskState::Running);
}

#[test]
fn test_priority_scheduler() {
    let mut scheduler = PriorityScheduler::new();
    
    let low = Priority::low();
    let high = Priority::high();
    
    scheduler.enqueue(1, low).unwrap();
    scheduler.enqueue(2, high).unwrap();
    scheduler.enqueue(3, Priority::normal()).unwrap();
    
    let (id, _) = scheduler.dequeue().unwrap();
    assert_eq!(id, 2); // High priority should be first
    
    let (id, _) = scheduler.dequeue().unwrap();
    assert_eq!(id, 3); // Normal priority second
}

#[test]
fn test_arena_allocator() {
    let mut arena = ArenaAllocator::new(4096, 0x1000);
    
    let addr1 = arena.allocate(512).unwrap();
    let addr2 = arena.allocate(512).unwrap();
    
    assert!(addr1 < addr2);
    
    arena.deallocate(addr1).unwrap();
    let addr3 = arena.allocate(512).unwrap();
    assert_eq!(addr1, addr3); // Should reuse freed block
}

#[test]
fn test_memory_pool() {
    let mut pool = MemoryPool::new(64, 10);
    
    assert_eq!(pool.capacity(), 10);
    assert_eq!(pool.free_count(), 10);
    
    let obj1 = pool.allocate_object().unwrap();
    let obj2 = pool.allocate_object().unwrap();
    
    assert_eq!(pool.free_count(), 8);
    assert_eq!(pool.allocated_count(), 2);
    
    pool.deallocate_object(obj1).unwrap();
    assert_eq!(pool.free_count(), 9);
}

#[test]
fn test_task_capability_management() {
    let mut task = CognitiveTask::new(
        TaskId::new(5),
        Priority::normal(),
        0x00,
        None,
    );
    
    task.grant_capability(0x01);
    task.grant_capability(0x02);
    
    assert!(task.has_capability(0x01));
    assert!(task.has_capability(0x02));
    assert!(!task.has_capability(0x04));
    
    task.revoke_capability(0x01);
    assert!(!task.has_capability(0x01));
    assert!(task.has_capability(0x02));
}

#[test]
fn test_task_phase_transitions() {
    let mut task = CognitiveTask::new(
        TaskId::new(1),
        Priority::normal(),
        0,
        None,
    );
    
    assert!(!task.is_complete());
    
    task.transition_phase(TaskPhase::Running);
    assert_eq!(task.phase, TaskPhase::Running);
    
    task.transition_phase(TaskPhase::Completed);
    assert!(task.is_complete());
}

#[test]
fn test_scheduler_update_priority() {
    let mut scheduler = PriorityScheduler::new();
    let id = 1;
    
    scheduler.enqueue(id, Priority::low()).unwrap();
    scheduler.update_priority(id, Priority::high()).unwrap();
    
    let (dequeued_id, priority) = scheduler.dequeue().unwrap();
    assert_eq!(dequeued_id, id);
    assert_eq!(priority.score(), Priority::high().score());
}

#[test]
fn test_invalid_state_transitions() {
    let mut sm = TaskStateMachine::new(1);
    
    // Cannot go directly from Init to Completed
    assert!(sm.transition(TaskState::Completed).is_err());
}
