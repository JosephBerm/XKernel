//! Memory management handlers — exposes L1 allocator operations.

use axum::extract::State;
use axum::Json;

use crate::error::ApiError;
use crate::models::*;
use crate::state::SharedState;

/// POST /api/v1/memory/allocate — Allocate memory pages.
pub async fn allocate_memory(
    State(state): State<SharedState>,
    Json(req): Json<AllocateMemoryRequest>,
) -> Result<Json<AllocationResponse>, ApiError> {
    let mut s = state.write().await;

    let allocation_id = s.total_allocations + 1;
    let size_bytes = req.pages * 4096;

    s.total_allocations += 1;
    s.total_bytes_allocated += size_bytes;

    s.record_event(
        "memory.allocated",
        None,
        &format!("Allocated {} pages ({} bytes) for CT {}, alloc_id={}",
                 req.pages, size_bytes, req.owner_ct_id, allocation_id),
    );

    Ok(Json(AllocationResponse {
        allocation_id,
        pages: req.pages,
        size_bytes,
        owner_ct_id: req.owner_ct_id,
    }))
}

/// POST /api/v1/memory/free — Free allocated memory.
pub async fn free_memory(
    State(state): State<SharedState>,
    Json(req): Json<FreeMemoryRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut s = state.write().await;

    s.record_event(
        "memory.freed",
        None,
        &format!("Freed allocation {}", req.allocation_id),
    );

    Ok(Json(serde_json::json!({
        "status": "freed",
        "allocation_id": req.allocation_id
    })))
}

/// GET /api/v1/memory — Get memory statistics.
pub async fn memory_stats(
    State(state): State<SharedState>,
) -> Json<MemoryStatsResponse> {
    let s = state.read().await;

    Json(MemoryStatsResponse {
        total_pages: 1048576,
        allocated_pages: s.total_bytes_allocated / 4096,
        free_pages: 1048576 - (s.total_bytes_allocated / 4096),
        active_allocations: s.total_allocations as usize,
        page_size_bytes: 4096,
    })
}
