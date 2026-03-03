//! IPC channel handlers — real message passing between agents.
//!
//! These handlers create kernel Channel instances and use them
//! for actual message send/receive operations.

use axum::extract::{Path, State};
use axum::Json;

use ipc_signals_exceptions::{Channel, Message};

use crate::error::ApiError;
use crate::models::*;
use crate::state::*;

/// POST /api/v1/channels — Create a new IPC channel between two agents.
pub async fn create_channel(
    State(state): State<SharedState>,
    Json(req): Json<CreateChannelRequest>,
) -> Result<Json<ChannelResponse>, ApiError> {
    let mut s = state.write().await;

    // Verify both agents exist
    if !s.agents.contains_key(&req.sender) {
        return Err(ApiError::NotFound(format!("sender agent '{}' not found", req.sender)));
    }
    if !s.agents.contains_key(&req.receiver) {
        return Err(ApiError::NotFound(format!("receiver agent '{}' not found", req.receiver)));
    }

    let channel_id = s.next_channel_id;
    s.next_channel_id += 1;

    // Create real kernel Channel
    // Channel::new(id, sender, receiver, capacity)
    // We use the channel_id as both the channel ID and encode agent task IDs
    let sender_task_id = s.agents.get(&req.sender).map(|a| a.task_id).unwrap_or(0);
    let receiver_task_id = s.agents.get(&req.receiver).map(|a| a.task_id).unwrap_or(0);

    let channel = Channel::new(channel_id, sender_task_id, receiver_task_id, req.capacity);

    let managed = ManagedChannel {
        channel,
        sender_agent: req.sender.clone(),
        receiver_agent: req.receiver.clone(),
    };

    s.channels.insert(channel_id, managed);

    s.record_event(
        "channel.created",
        None,
        &format!("Channel {} created: {} -> {} (capacity: {})", channel_id, req.sender, req.receiver, req.capacity),
    );

    let mc = s.channels.get(&channel_id).unwrap();
    Ok(Json(channel_to_response(channel_id, mc)))
}

/// GET /api/v1/channels — List all channels.
pub async fn list_channels(
    State(state): State<SharedState>,
) -> Json<ChannelListResponse> {
    let s = state.read().await;
    let channels: Vec<ChannelResponse> = s.channels.iter()
        .map(|(&id, mc)| channel_to_response(id, mc))
        .collect();
    let total = channels.len();
    Json(ChannelListResponse { channels, total })
}

/// POST /api/v1/channels/:id/send — Send a message through a channel.
pub async fn send_message(
    State(state): State<SharedState>,
    Path(id): Path<u64>,
    Json(req): Json<SendMessageRequest>,
) -> Result<Json<MessageResponse>, ApiError> {
    let mut s = state.write().await;

    let mc = s.channels.get_mut(&id)
        .ok_or_else(|| ApiError::NotFound(format!("channel {} not found", id)))?;

    let sender_id = mc.channel.id();
    let sender_agent = mc.sender_agent.clone();
    let receiver_agent = mc.receiver_agent.clone();

    // Create real kernel Message
    let msg = Message::new(
        mc.channel.id(),  // sender context
        mc.channel.id(),  // receiver context
        req.payload.as_bytes().to_vec(),
    );

    let sequence = msg.size() as u64;

    // Send through the real kernel channel
    mc.channel.send(msg)
        .map_err(|e| ApiError::BadRequest(format!("send failed: {:?}", e)))?;

    s.total_messages_sent += 1;

    s.record_event(
        "channel.message_sent",
        None,
        &format!("Message sent on channel {} ({} -> {})", id, sender_agent, receiver_agent),
    );

    Ok(Json(MessageResponse {
        sender: sender_agent,
        receiver: receiver_agent,
        payload: req.payload,
        sequence,
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
    }))
}

/// POST /api/v1/channels/:id/receive — Receive a message from a channel.
pub async fn receive_message(
    State(state): State<SharedState>,
    Path(id): Path<u64>,
) -> Result<Json<MessageResponse>, ApiError> {
    let mut s = state.write().await;

    let mc = s.channels.get_mut(&id)
        .ok_or_else(|| ApiError::NotFound(format!("channel {} not found", id)))?;

    let sender_agent = mc.sender_agent.clone();
    let receiver_agent = mc.receiver_agent.clone();

    // Receive from real kernel channel
    let msg = mc.channel.receive()
        .map_err(|e| ApiError::BadRequest(format!("receive failed: {:?}", e)))?;

    s.total_messages_received += 1;

    let payload = String::from_utf8_lossy(&msg.payload).to_string();

    Ok(Json(MessageResponse {
        sender: sender_agent,
        receiver: receiver_agent,
        payload,
        sequence: msg.sequence,
        timestamp: msg.timestamp,
    }))
}

fn channel_to_response(id: u64, mc: &ManagedChannel) -> ChannelResponse {
    ChannelResponse {
        id,
        sender: mc.sender_agent.clone(),
        receiver: mc.receiver_agent.clone(),
        capacity: 256, // Channel doesn't expose capacity getter; use default
        pending_messages: mc.channel.pending_count(),
        is_closed: mc.channel.is_closed(),
    }
}
