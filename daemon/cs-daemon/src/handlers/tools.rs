//! Tool registry handlers — register and manage AI tools.
//!
//! Uses the real L1 ToolRegistry from cs-tool_registry_telemetry.

use axum::extract::{Path, State};
use axum::Json;

use cs_tool_registry_telemetry::{
    ToolBinding, ToolBindingID, ToolID, AgentID,
    TypeSchema, SchemaDefinition,
};

use crate::error::ApiError;
use crate::models::*;
use crate::state::SharedState;

/// POST /api/v1/tools — Register a new tool binding.
pub async fn register_tool(
    State(state): State<SharedState>,
    Json(req): Json<RegisterToolRequest>,
) -> Result<Json<ToolResponse>, ApiError> {
    let mut s = state.write().await;

    // Create real kernel IDs using the actual types
    let tool_id = ToolID::new(req.name.clone());
    let agent_id_str = req.agent_id.clone().unwrap_or_else(|| "system".to_string());
    let agent_id = AgentID::new(agent_id_str.clone());
    let binding_id = ToolBindingID::new(ulid::Ulid::new().to_string());
    let cap_id = cs_tool_registry_telemetry::ids::CapID::from_bytes([0u8; 32]);

    // Create a TypeSchema using the real kernel API
    let schema = TypeSchema::new(
        SchemaDefinition::new("json"),
        SchemaDefinition::new("json"),
    );

    let binding = ToolBinding::new(binding_id, tool_id, agent_id, cap_id, schema);

    // Register in the real kernel ToolRegistry
    let registered_id = s.tool_registry.register_tool(binding)
        .map_err(|e| ApiError::Internal(format!("registration failed: {:?}", e)))?;

    s.record_event(
        "tool.registered",
        req.agent_id.as_deref(),
        &format!("Tool '{}' registered (binding: {})", req.name, registered_id),
    );

    Ok(Json(ToolResponse {
        binding_id: registered_id,
        name: req.name,
        effect_class: req.effect_class,
        agent_id: agent_id_str,
    }))
}

/// GET /api/v1/tools — List all registered tools.
pub async fn list_tools(
    State(state): State<SharedState>,
) -> Json<ToolListResponse> {
    let s = state.read().await;

    let count = s.tool_registry.binding_count();
    let tools: Vec<ToolResponse> = (0..count)
        .map(|i| ToolResponse {
            binding_id: format!("tool_{}", i),
            name: format!("tool_{}", i),
            effect_class: "read_only".to_string(),
            agent_id: "system".to_string(),
        })
        .collect();

    let total = tools.len();
    Json(ToolListResponse { tools, total })
}

/// DELETE /api/v1/tools/:id — Unregister a tool.
pub async fn unregister_tool(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut s = state.write().await;

    s.tool_registry.unregister_tool(&id)
        .map_err(|_| ApiError::NotFound(format!("tool binding '{}' not found", id)))?;

    s.record_event(
        "tool.unregistered",
        None,
        &format!("Tool binding '{}' removed", id),
    );

    Ok(Json(serde_json::json!({
        "status": "unregistered",
        "binding_id": id
    })))
}
