//! System handlers — health checks, metrics, and telemetry events.

use axum::extract::{State, Query};
use axum::Json;
use std::collections::HashMap;

use crate::models::*;
use crate::state::*;

/// GET /healthz — Liveness probe.
pub async fn health(
    State(state): State<SharedState>,
) -> Json<HealthResponse> {
    let s = state.read().await;
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: s.uptime_seconds(),
    })
}

/// GET /readyz — Readiness probe.
pub async fn readiness(
    State(state): State<SharedState>,
) -> Json<HealthResponse> {
    let s = state.read().await;
    Json(HealthResponse {
        status: "ready".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: s.uptime_seconds(),
    })
}

/// GET /api/v1/metrics — Full system metrics from all kernel subsystems.
pub async fn metrics(
    State(state): State<SharedState>,
) -> Json<MetricsResponse> {
    let s = state.read().await;

    // Count agents by state
    let mut running = 0usize;
    let mut stopped = 0usize;
    let mut failed = 0usize;
    for agent in s.agents.values() {
        match agent.state {
            AgentState::Running | AgentState::Starting => running += 1,
            AgentState::Stopped => stopped += 1,
            AgentState::Failed => failed += 1,
            _ => {}
        }
    }

    // Count events by type
    let mut events_per_type: HashMap<String, u64> = HashMap::new();
    for event in &s.events {
        *events_per_type.entry(event.event_type.clone()).or_insert(0) += 1;
    }

    Json(MetricsResponse {
        uptime_seconds: s.uptime_seconds(),
        agents: AgentMetrics {
            total_created: s.total_agents_created,
            active: s.agents.len(),
            running,
            stopped,
            failed,
        },
        scheduler: SchedulerMetrics {
            queue_depth: s.scheduler.len(),
            total_scheduled: s.total_scheduled,
            total_completed: s.total_completed,
        },
        channels: ChannelMetrics {
            active_channels: s.channels.len(),
            total_messages_sent: s.total_messages_sent,
            total_messages_received: s.total_messages_received,
        },
        memory: MemoryMetrics {
            total_allocations: s.total_allocations,
            active_allocations: s.total_allocations as usize,
            total_bytes_allocated: s.total_bytes_allocated,
        },
        tools: ToolMetrics {
            registered_tools: s.tool_registry.binding_count(),
            total_invocations: s.total_tool_invocations,
        },
        telemetry: TelemetryMetrics {
            total_events: s.events.len() as u64,
            events_per_type,
        },
    })
}

#[derive(Debug, serde::Deserialize)]
pub struct EventQuery {
    /// Filter by event type
    pub event_type: Option<String>,
    /// Maximum number of events to return
    pub limit: Option<usize>,
}

/// GET /api/v1/events — Query telemetry events.
pub async fn events(
    State(state): State<SharedState>,
    Query(query): Query<EventQuery>,
) -> Json<EventResponse> {
    let s = state.read().await;
    let limit = query.limit.unwrap_or(100);

    let filtered: Vec<SystemEvent> = s.events.iter()
        .rev() // newest first
        .filter(|e| {
            if let Some(ref et) = query.event_type {
                e.event_type.contains(et)
            } else {
                true
            }
        })
        .take(limit)
        .cloned()
        .collect();

    let total = filtered.len();
    Json(EventResponse { events: filtered, total })
}
