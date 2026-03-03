//! Agent management handlers — the core of the daemon.
//!
//! Agents are the primary managed resource. Each agent maps to:
//! - A kernel TaskStateMachine (L0 lifecycle)
//! - A PriorityScheduler entry (L0 scheduling)
//! - An optional OS process (real execution)
//! - Capability grants (L0 permissions)

use axum::extract::{Path, State};
use axum::Json;
use std::collections::VecDeque;

use ct_lifecycle::{TaskStateMachine, TaskState, Priority};

use crate::error::ApiError;
use crate::models::*;
use crate::state::*;
use crate::supervisor;

/// POST /api/v1/agents — Create and optionally start a new agent.
pub async fn create_agent(
    State(state): State<SharedState>,
    Json(req): Json<CreateAgentRequest>,
) -> Result<Json<AgentResponse>, ApiError> {
    let agent_id = {
        let mut s = state.write().await;

        let task_id = s.next_task_id;
        s.next_task_id += 1;

        let task_sm = TaskStateMachine::new(task_id);
        let priority = Priority::new(req.priority, req.priority, 128, 64);
        if let Err(e) = s.scheduler.enqueue(task_id, priority) {
            tracing::warn!(task_id, error = %e, "scheduler enqueue failed");
        }
        s.total_scheduled += 1;

        let agent_id = ulid::Ulid::new().to_string().to_lowercase();
        let restart_policy = RestartPolicy::from_str(&req.restart_policy);

        let agent = ManagedAgent {
            id: agent_id.clone(),
            name: req.name.clone(),
            framework: req.framework.clone(),
            entrypoint: req.entrypoint.clone(),
            working_dir: req.working_dir.clone(),
            env: req.env.clone(),
            state: AgentState::Created,
            task_state_machine: task_sm,
            task_id,
            pid: None,
            created_at: chrono::Utc::now(),
            started_at: None,
            restart_count: 0,
            max_retries: req.max_retries,
            restart_policy,
            capabilities: req.capabilities.clone(),
            logs: VecDeque::new(),
            process_handle: None,
        };

        s.agents.insert(agent_id.clone(), agent);
        s.total_agents_created += 1;

        s.record_event(
            "agent.created",
            Some(&agent_id),
            &format!("Agent '{}' created (framework: {}, task_id: {})", req.name, req.framework, task_id),
        );

        agent_id
    }; // lock released

    if req.entrypoint.is_some() {
        supervisor::start_agent_process(state.clone(), &agent_id).await?;
    } else {
        let mut s = state.write().await;
        if let Some(agent) = s.agents.get_mut(&agent_id) {
            agent.state = AgentState::Running;
            agent.started_at = Some(chrono::Utc::now());
            let _ = agent.task_state_machine.transition(TaskState::Ready);
            let _ = agent.task_state_machine.transition(TaskState::Running);
        }
    }

    let s = state.read().await;
    let agent = s.agents.get(&agent_id)
        .ok_or_else(|| ApiError::Internal("agent disappeared".into()))?;
    Ok(Json(agent_to_response(agent)))
}

/// GET /api/v1/agents — List all agents.
pub async fn list_agents(
    State(state): State<SharedState>,
) -> Json<AgentListResponse> {
    let s = state.read().await;
    let agents: Vec<AgentResponse> = s.agents.values()
        .map(agent_to_response)
        .collect();
    let total = agents.len();
    Json(AgentListResponse { agents, total })
}

/// GET /api/v1/agents/:id — Get agent details.
pub async fn get_agent(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<AgentResponse>, ApiError> {
    let s = state.read().await;
    let agent = s.agents.get(&id)
        .ok_or_else(|| ApiError::NotFound(format!("agent '{}' not found", id)))?;
    Ok(Json(agent_to_response(agent)))
}

/// DELETE /api/v1/agents/:id — Stop and remove an agent.
pub async fn delete_agent(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<AgentResponse>, ApiError> {
    supervisor::stop_agent_process(state.clone(), &id).await?;

    {
        let mut s = state.write().await;
        // Extract what we need first
        let task_id = s.agents.get(&id)
            .map(|a| a.task_id)
            .ok_or_else(|| ApiError::NotFound(format!("agent '{}' not found", id)))?;
        let agent_name = s.agents.get(&id).map(|a| a.name.clone()).unwrap_or_default();

        let _ = s.scheduler.remove(task_id);
        s.total_completed += 1;

        if let Some(agent) = s.agents.get_mut(&id) {
            if !agent.task_state_machine.is_terminal() {
                let _ = agent.task_state_machine.transition(TaskState::Completed);
            }
            agent.state = AgentState::Stopped;
        }

        s.record_event("agent.stopped", Some(&id), &format!("Agent '{}' stopped", agent_name));
    }

    let s = state.read().await;
    let agent = s.agents.get(&id)
        .ok_or_else(|| ApiError::Internal("agent disappeared".into()))?;
    Ok(Json(agent_to_response(agent)))
}

/// POST /api/v1/agents/:id/signal — Send a signal to an agent.
pub async fn signal_agent(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(req): Json<SignalAgentRequest>,
) -> Result<Json<AgentResponse>, ApiError> {
    match req.signal.as_str() {
        "stop" => {
            supervisor::stop_agent_process(state.clone(), &id).await?;
            {
                let mut s = state.write().await;
                if let Some(agent) = s.agents.get_mut(&id) {
                    agent.state = AgentState::Stopped;
                    if !agent.task_state_machine.is_terminal() {
                        let _ = agent.task_state_machine.transition(TaskState::Completed);
                    }
                }
                s.record_event("agent.signal.stop", Some(&id), "Agent received stop signal");
            }
            let s = state.read().await;
            let agent = s.agents.get(&id)
                .ok_or_else(|| ApiError::NotFound(format!("agent '{}' not found", id)))?;
            Ok(Json(agent_to_response(agent)))
        }
        "checkpoint" => {
            {
                let mut s = state.write().await;
                if let Some(agent) = s.agents.get_mut(&id) {
                    if agent.task_state_machine.current_state() == TaskState::Running {
                        let _ = agent.task_state_machine.transition(TaskState::Checkpointed);
                        let _ = agent.task_state_machine.transition(TaskState::Ready);
                        let _ = agent.task_state_machine.transition(TaskState::Running);
                    }
                }
                s.record_event("agent.signal.checkpoint", Some(&id), "Agent checkpointed");
            }
            let s = state.read().await;
            let agent = s.agents.get(&id)
                .ok_or_else(|| ApiError::NotFound(format!("agent '{}' not found", id)))?;
            Ok(Json(agent_to_response(agent)))
        }
        "yield" => {
            {
                let mut s = state.write().await;
                if let Some(agent) = s.agents.get_mut(&id) {
                    if agent.task_state_machine.current_state() == TaskState::Running {
                        let _ = agent.task_state_machine.transition(TaskState::Waiting);
                        let _ = agent.task_state_machine.transition(TaskState::Ready);
                    }
                }
                let _ = s.scheduler.yield_now();
                s.record_event("agent.signal.yield", Some(&id), "Agent yielded");
            }
            let s = state.read().await;
            let agent = s.agents.get(&id)
                .ok_or_else(|| ApiError::NotFound(format!("agent '{}' not found", id)))?;
            Ok(Json(agent_to_response(agent)))
        }
        other => Err(ApiError::BadRequest(format!("unknown signal: '{}'", other))),
    }
}

/// GET /api/v1/agents/:id/logs — Get agent stdout/stderr logs.
pub async fn get_agent_logs(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<AgentLogsResponse>, ApiError> {
    let s = state.read().await;
    let agent = s.agents.get(&id)
        .ok_or_else(|| ApiError::NotFound(format!("agent '{}' not found", id)))?;

    let lines: Vec<LogLine> = agent.logs.iter().map(|entry| LogLine {
        timestamp: entry.timestamp.to_rfc3339(),
        stream: entry.stream.clone(),
        message: entry.message.clone(),
    }).collect();

    Ok(Json(AgentLogsResponse { agent_id: id, lines }))
}

/// Convert internal ManagedAgent to API response.
fn agent_to_response(agent: &ManagedAgent) -> AgentResponse {
    let uptime = agent.started_at.map(|started| {
        (chrono::Utc::now() - started).num_milliseconds() as f64 / 1000.0
    });

    AgentResponse {
        id: agent.id.clone(),
        name: agent.name.clone(),
        framework: agent.framework.clone(),
        state: agent.state.to_string(),
        pid: agent.pid,
        created_at: agent.created_at.to_rfc3339(),
        started_at: agent.started_at.map(|t| t.to_rfc3339()),
        uptime_seconds: uptime,
        restart_count: agent.restart_count,
        capabilities: agent.capabilities.clone(),
        task_phase: format!("{:?}", agent.task_state_machine.current_state()),
        scheduler_position: None,
    }
}
