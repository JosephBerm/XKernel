// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 XKernal Contributors
//! Inter-process communication with channels, pubsub, and shared context

use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// IPC errors
#[derive(Debug, Clone, Error)]
pub enum IpcError {
    /// Channel closed
    #[error("channel {0} is closed")]
    ChannelClosed(u64),
    /// Send failed
    #[error("send failed: {0}")]
    SendFailed(alloc::string::String),
    /// Receive failed
    #[error("receive failed: {0}")]
    ReceiveFailed(alloc::string::String),
    /// Channel not found
    #[error("channel {0} not found")]
    ChannelNotFound(u64),
    /// Message queue full
    #[error("message queue is full")]
    QueueFull,
    /// Invalid topic
    #[error("invalid topic: {0}")]
    InvalidTopic(alloc::string::String),
}

pub type Result<T> = core::result::Result<T, IpcError>;

/// Message sent over IPC channels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Sender ID
    pub sender: u64,
    /// Receiver ID
    pub receiver: u64,
    /// Message payload
    pub payload: Vec<u8>,
    /// Message sequence number
    pub sequence: u64,
    /// Timestamp
    pub timestamp: u64,
}

impl Message {
    /// Create a new message
    pub fn new(sender: u64, receiver: u64, payload: Vec<u8>) -> Self {
        Self {
            sender,
            receiver,
            payload,
            sequence: 0,
            timestamp: 0,
        }
    }

    /// Get the payload size
    pub fn size(&self) -> usize {
        self.payload.len()
    }
}

/// Channel for inter-process communication
#[derive(Debug)]
pub struct Channel {
    id: u64,
    sender: u64,
    receiver: u64,
    messages: Vec<Message>,
    capacity: usize,
    is_closed: bool,
}

impl Channel {
    /// Create a new channel
    pub fn new(id: u64, sender: u64, receiver: u64, capacity: usize) -> Self {
        Self {
            id,
            sender,
            receiver,
            messages: Vec::with_capacity(capacity),
            capacity,
            is_closed: false,
        }
    }

    /// Send a message
    pub fn send(&mut self, message: Message) -> Result<()> {
        if self.is_closed {
            return Err(IpcError::ChannelClosed(self.id));
        }

        if self.messages.len() >= self.capacity {
            return Err(IpcError::QueueFull);
        }

        self.messages.push(message);
        Ok(())
    }

    /// Receive the next message
    pub fn receive(&mut self) -> Result<Message> {
        if self.messages.is_empty() {
            return Err(IpcError::ReceiveFailed("no messages available".into()));
        }

        Ok(self.messages.remove(0))
    }

    /// Peek at the next message without removing it
    pub fn peek(&self) -> Option<&Message> {
        self.messages.first()
    }

    /// Get the number of pending messages
    pub fn pending_count(&self) -> usize {
        self.messages.len()
    }

    /// Close this channel
    pub fn close(&mut self) {
        self.is_closed = true;
    }

    /// Check if channel is closed
    pub fn is_closed(&self) -> bool {
        self.is_closed
    }

    /// Get channel ID
    pub fn id(&self) -> u64 {
        self.id
    }
}

/// IPC endpoint trait
pub trait IpcEndpoint {
    /// Send a message
    fn send(&mut self, message: Message) -> Result<()>;

    /// Receive a message
    fn receive(&mut self) -> Result<Message>;

    /// Check if endpoint is active
    fn is_active(&self) -> bool;
}

impl IpcEndpoint for Channel {
    fn send(&mut self, message: Message) -> Result<()> {
        self.send(message)
    }

    fn receive(&mut self) -> Result<Message> {
        self.receive()
    }

    fn is_active(&self) -> bool {
        !self.is_closed
    }
}

/// Topic for publish-subscribe communication
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Topic {
    name: alloc::string::String,
}

impl Topic {
    /// Create a new topic
    pub fn new(name: alloc::string::String) -> Self {
        Self { name }
    }

    /// Get the topic name
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Shared context for inter-process synchronization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedContext {
    /// Context ID
    pub id: u64,
    /// Shared data
    pub data: Vec<u8>,
    /// Lock state (0 = free, 1 = locked)
    pub lock_state: u32,
    /// Version/timestamp for CRDT
    pub version: u64,
}

impl SharedContext {
    /// Create a new shared context
    pub fn new(id: u64, data: Vec<u8>) -> Self {
        Self {
            id,
            data,
            lock_state: 0,
            version: 0,
        }
    }

    /// Acquire the lock
    pub fn acquire_lock(&mut self) -> Result<()> {
        if self.lock_state != 0 {
            return Err(IpcError::SendFailed("lock already held".into()));
        }
        self.lock_state = 1;
        Ok(())
    }

    /// Release the lock
    pub fn release_lock(&mut self) -> Result<()> {
        if self.lock_state == 0 {
            return Err(IpcError::SendFailed("lock not held".into()));
        }
        self.lock_state = 0;
        self.version += 1;
        Ok(())
    }

    /// Check if locked
    pub fn is_locked(&self) -> bool {
        self.lock_state != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn test_message_creation() {
        let msg = Message::new(1, 2, vec![1, 2, 3]);
        assert_eq!(msg.sender, 1);
        assert_eq!(msg.receiver, 2);
        assert_eq!(msg.size(), 3);
    }

    #[test]
    fn test_channel_send_receive() {
        let mut channel = Channel::new(1, 1, 2, 10);
        let msg = Message::new(1, 2, vec![1, 2, 3]);

        channel.send(msg.clone()).unwrap();
        assert_eq!(channel.pending_count(), 1);

        let received = channel.receive().unwrap();
        assert_eq!(received.sender, msg.sender);
    }

    #[test]
    fn test_channel_closed() {
        let mut channel = Channel::new(1, 1, 2, 10);
        channel.close();
        assert!(channel.is_closed());

        let msg = Message::new(1, 2, vec![]);
        assert!(channel.send(msg).is_err());
    }

    #[test]
    fn test_shared_context_locking() {
        let mut ctx = SharedContext::new(1, vec![]);
        assert!(!ctx.is_locked());

        ctx.acquire_lock().unwrap();
        assert!(ctx.is_locked());

        ctx.release_lock().unwrap();
        assert!(!ctx.is_locked());
    }
}
