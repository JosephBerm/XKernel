// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 XKernal Contributors
//! Integration tests for ipc-signals-exceptions crate

use ipc_signals_exceptions::*;

#[test]
fn test_channel_messaging() {
    let mut channel = Channel::new(1, 1, 2, 10);
    let msg = Message::new(1, 2, vec![1, 2, 3]);

    channel.send(msg.clone()).unwrap();
    assert_eq!(channel.pending_count(), 1);

    let received = channel.receive().unwrap();
    assert_eq!(received.sender, 1);
    assert_eq!(received.receiver, 2);
}

#[test]
fn test_shared_context_locking() {
    let mut ctx = SharedContext::new(1, vec![1, 2, 3]);
    assert!(!ctx.is_locked());

    ctx.acquire_lock().unwrap();
    assert!(ctx.is_locked());

    ctx.release_lock().unwrap();
    assert!(!ctx.is_locked());
}

#[test]
fn test_signal_dispatch() {
    let mut dispatcher = SignalDispatcher::new();
    dispatcher.register_handler(Signal::Terminate, 1);

    let handler_id = dispatcher.dispatch(Signal::Terminate).unwrap();
    assert_eq!(handler_id, 1);
}

#[test]
fn test_signal_blocking() {
    let mut dispatcher = SignalDispatcher::new();
    dispatcher.register_handler(Signal::Interrupt, 1);

    dispatcher.block_signal(Signal::Interrupt);
    assert!(dispatcher.dispatch(Signal::Interrupt).is_err());

    dispatcher.unblock_signal(Signal::Interrupt);
    assert!(dispatcher.dispatch(Signal::Interrupt).is_ok());
}

#[test]
fn test_exception_handling() {
    let mut handler = SimpleExceptionHandler::new(100);
    let exc = Exception::new(1, 1, 100, ExceptionSeverity::Recoverable, "test".into());

    assert!(handler.handle(&exc).is_ok());
    assert_eq!(handler.handled_count(), 1);
}

#[test]
fn test_exception_dispatch() {
    let mut dispatcher = FaultDispatcher::new(10);
    let exc = Exception::new(1, 1, 100, ExceptionSeverity::Recoverable, "test".into());

    dispatcher.dispatch(exc).unwrap();
    assert_eq!(dispatcher.pending_count(), 1);

    let retrieved = dispatcher.next_exception();
    assert!(retrieved.is_some());
}

#[test]
fn test_checkpoint_creation() {
    let mut mgr = CheckpointManager::new();
    let id = mgr
        .create_checkpoint(1, SnapshotFormat::Binary, vec![1, 2, 3])
        .unwrap();

    assert!(mgr.get_checkpoint(id).is_ok());
}

#[test]
fn test_checkpoint_restore() {
    let mut mgr = CheckpointManager::new();
    let id = mgr
        .create_checkpoint(1, SnapshotFormat::Binary, vec![42])
        .unwrap();

    let cp = mgr.get_checkpoint(id).unwrap();
    assert_eq!(cp.data[0], 42);
}

#[test]
fn test_multi_process_communication() {
    let mut channel1 = Channel::new(1, 1, 2, 5);
    let mut channel2 = Channel::new(2, 2, 1, 5);

    let msg1 = Message::new(1, 2, vec![100, 101]);
    channel1.send(msg1).unwrap();

    let msg2 = Message::new(2, 1, vec![102, 103]);
    channel2.send(msg2).unwrap();

    assert_eq!(channel1.pending_count(), 1);
    assert_eq!(channel2.pending_count(), 1);
}

#[test]
fn test_signal_handler_lifecycle() {
    let mut handler = SimpleSignalHandler::new(Signal::Checkpoint);
    
    handler.handle(Signal::Checkpoint).unwrap();
    handler.handle(Signal::Checkpoint).unwrap();

    assert_eq!(handler.invocation_count(), 2);
    assert!(handler.is_active());
}

#[test]
fn test_exception_stack_trace() {
    let mut exc = Exception::new(1, 1, 200, ExceptionSeverity::Severe, "error".into());
    
    exc.add_stack_frame(0x1000);
    exc.add_stack_frame(0x2000);
    exc.add_stack_frame(0x3000);

    assert_eq!(exc.stack_trace.len(), 3);
    assert!(!exc.is_fatal());
}

#[test]
fn test_crdt_state_merge() {
    let mut state1 = CrdtState::new(3);
    let mut state2 = CrdtState::new(3);

    state1.increment(0);
    state1.increment(1);
    
    state2.increment(0);
    state2.increment(2);

    state1.merge(&state2);

    assert_eq!(state1.clock[0], 1);
    assert_eq!(state1.clock[1], 1);
    assert_eq!(state1.clock[2], 1);
}
