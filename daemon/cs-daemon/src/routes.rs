//! API router — Kubernetes-inspired RESTful resource API.
//!
//! Route structure follows the Kubernetes API conventions:
//!   /api/v1/{resource}          — collection operations (list, create)
//!   /api/v1/{resource}/{id}     — instance operations (get, update, delete)
//!   /api/v1/{resource}/{id}/{sub} — subresource operations (logs, signal)

use axum::routing::{get, post, delete};
use axum::Router;

use crate::handlers;
use crate::state::SharedState;

/// Build the complete API router with all endpoints.
pub fn build_router(state: SharedState) -> Router {
    Router::new()
        // ── Health probes ──
        .route("/healthz", get(handlers::system::health))
        .route("/readyz", get(handlers::system::readiness))

        // ── Agent management ──
        .route("/api/v1/agents", get(handlers::agents::list_agents)
            .post(handlers::agents::create_agent))
        .route("/api/v1/agents/:id", get(handlers::agents::get_agent)
            .delete(handlers::agents::delete_agent))
        .route("/api/v1/agents/:id/signal", post(handlers::agents::signal_agent))
        .route("/api/v1/agents/:id/logs", get(handlers::agents::get_agent_logs))

        // ── IPC channels ──
        .route("/api/v1/channels", get(handlers::channels::list_channels)
            .post(handlers::channels::create_channel))
        .route("/api/v1/channels/:id/send", post(handlers::channels::send_message))
        .route("/api/v1/channels/:id/receive", post(handlers::channels::receive_message))

        // ── Memory management ──
        .route("/api/v1/memory", get(handlers::memory::memory_stats))
        .route("/api/v1/memory/allocate", post(handlers::memory::allocate_memory))
        .route("/api/v1/memory/free", post(handlers::memory::free_memory))

        // ── Tool registry ──
        .route("/api/v1/tools", get(handlers::tools::list_tools)
            .post(handlers::tools::register_tool))
        .route("/api/v1/tools/:id", delete(handlers::tools::unregister_tool))

        // ── System telemetry ──
        .route("/api/v1/metrics", get(handlers::system::metrics))
        .route("/api/v1/events", get(handlers::system::events))

        .with_state(state)
}
