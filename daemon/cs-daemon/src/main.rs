//! cs-daemon — Cognitive Substrate control plane daemon.
//!
//! This is the entry point for the XKernal runtime. It hosts all kernel
//! subsystems behind a REST API, manages AI agent processes, and provides
//! real-time telemetry.
//!
//! # Architecture
//!
//! The daemon follows a Kubernetes-inspired control plane pattern:
//! - REST API for agent lifecycle management (create, start, stop, signal)
//! - Real process supervision with stdout/stderr capture
//! - IPC channels backed by the L0 kernel channel implementation
//! - Capability-based access control from the L0 capability engine
//! - Tool registry from the L1 services layer
//! - Structured telemetry events compatible with CEF format
//!
//! # Usage
//!
//! ```bash
//! cs-daemon                          # Start on default port 7600
//! CS_PORT=8080 cs-daemon             # Custom port
//! CS_HOST=0.0.0.0 cs-daemon          # Listen on all interfaces
//! CS_LOG=debug cs-daemon             # Debug logging
//! ```

mod error;
mod handlers;
mod models;
mod routes;
mod state;
mod supervisor;

use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use state::AppState;

#[tokio::main]
async fn main() {
    // ── Initialize structured logging ──
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_env("CS_LOG")
                .unwrap_or_else(|_| EnvFilter::new("info,cs_daemon=debug"))
        )
        .with_target(true)
        .init();

    // ── Parse config from env vars ──
    let port = std::env::var("CS_PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(7600);

    let host = std::env::var("CS_HOST")
        .unwrap_or_else(|_| "127.0.0.1".to_string());

    // ── Initialize shared state with all kernel subsystems ──
    let state = Arc::new(RwLock::new(AppState::new()));

    // Record daemon startup event
    {
        let mut s = state.write().await;
        s.record_event("daemon.started", None, &format!(
            "Cognitive Substrate daemon v{} starting on {}:{}",
            env!("CARGO_PKG_VERSION"), host, port
        ));
    }

    // ── Build the API router ──
    let app = routes::build_router(state)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive());

    // ── Start the server ──
    let addr: SocketAddr = format!("{}:{}", host, port).parse().expect("invalid address");

    tracing::info!("=========================================================");
    tracing::info!("  Cognitive Substrate OS - cs-daemon v{}", env!("CARGO_PKG_VERSION"));
    tracing::info!("  Listening on http://{}", addr);
    tracing::info!("---------------------------------------------------------");
    tracing::info!("  API:        http://{}/api/v1/", addr);
    tracing::info!("  Health:     http://{}/healthz", addr);
    tracing::info!("  Metrics:    http://{}/api/v1/metrics", addr);
    tracing::info!("=========================================================");

    let listener = tokio::net::TcpListener::bind(addr).await.expect("failed to bind");
    axum::serve(listener, app).await.expect("server error");
}
