//! Process supervisor — spawns and manages real OS processes for agents.
//!
//! This is what makes XKernal real: agents aren't just state machines,
//! they're actual running processes with captured I/O, health monitoring,
//! and restart policies.

use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use ct_lifecycle::TaskState;

use crate::error::ApiError;
use crate::state::*;

/// Start an agent's OS process and spawn a watcher for it.
///
/// This is the public entry point — it spawns the process, I/O readers,
/// AND a watcher task that handles exit/restart.
pub async fn start_agent_process(state: SharedState, agent_id: &str) -> Result<(), ApiError> {
    spawn_agent_process(state.clone(), agent_id).await?;

    // Spawn process watcher (handles exit, restarts)
    let state_clone = state.clone();
    let id = agent_id.to_string();
    tokio::spawn(async move {
        watch_agent_process(state_clone, id).await;
    });

    Ok(())
}

/// Core process spawning logic — starts the OS process and I/O readers
/// but does NOT spawn a watcher. Used by both the public API and the
/// watcher's restart path to avoid a recursive Send cycle.
async fn spawn_agent_process(state: SharedState, agent_id: &str) -> Result<(), ApiError> {
    // Extract what we need, then release the lock
    let (entrypoint, working_dir, env) = {
        let mut s = state.write().await;
        let agent = s.agents.get_mut(agent_id)
            .ok_or_else(|| ApiError::NotFound(format!("agent '{}' not found", agent_id)))?;

        let entrypoint = agent.entrypoint.clone()
            .ok_or_else(|| ApiError::BadRequest("agent has no entrypoint".into()))?;

        agent.state = AgentState::Starting;
        let _ = agent.task_state_machine.transition(TaskState::Ready);

        let wd = agent.working_dir.clone();
        let env = agent.env.clone();

        s.record_event(
            "agent.starting",
            Some(agent_id),
            &format!("Starting process: {}", entrypoint),
        );

        (entrypoint, wd, env)
    };

    // Parse the entrypoint command
    let parts: Vec<&str> = entrypoint.split_whitespace().collect();
    if parts.is_empty() {
        return Err(ApiError::BadRequest("empty entrypoint command".into()));
    }

    let program = parts[0];
    let args = &parts[1..];

    // Build the command
    let mut cmd = Command::new(program);
    cmd.args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    if let Some(ref wd) = working_dir {
        cmd.current_dir(wd);
    }

    for (key, val) in &env {
        cmd.env(key, val);
    }

    // Spawn the process
    let mut child = cmd.spawn().map_err(|e| {
        ApiError::Internal(format!("failed to spawn process '{}': {}", entrypoint, e))
    })?;

    let pid = child.id();

    // Capture stdout and stderr handles before moving child
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    // Update agent state
    {
        let mut s = state.write().await;
        if let Some(agent) = s.agents.get_mut(agent_id) {
            agent.state = AgentState::Running;
            agent.pid = pid;
            agent.started_at = Some(chrono::Utc::now());
            agent.process_handle = Some(child);
            let _ = agent.task_state_machine.transition(TaskState::Running);
        }
    }

    // Spawn stdout reader task
    if let Some(stdout) = stdout {
        let state_clone = state.clone();
        let id = agent_id.to_string();
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                tracing::debug!(agent_id = %id, stream = "stdout", "{}", line);
                let mut s = state_clone.write().await;
                s.push_agent_log(&id, "stdout", &line);
            }
        });
    }

    // Spawn stderr reader task
    if let Some(stderr) = stderr {
        let state_clone = state.clone();
        let id = agent_id.to_string();
        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                tracing::warn!(agent_id = %id, stream = "stderr", "{}", line);
                let mut s = state_clone.write().await;
                s.push_agent_log(&id, "stderr", &line);
            }
        });
    }

    Ok(())
}

/// Stop an agent's OS process.
pub async fn stop_agent_process(state: SharedState, agent_id: &str) -> Result<(), ApiError> {
    let mut s = state.write().await;
    let agent = s.agents.get_mut(agent_id)
        .ok_or_else(|| ApiError::NotFound(format!("agent '{}' not found", agent_id)))?;

    if let Some(ref mut child) = agent.process_handle {
        let _ = child.kill().await;
        agent.pid = None;
    }
    agent.process_handle = None;

    Ok(())
}

/// Watch a running agent process and handle exit/restart.
async fn watch_agent_process(state: SharedState, agent_id: String) {
    loop {
        // Non-blocking check if process has exited
        let exit_status = {
            let mut s = state.write().await;
            let agent = match s.agents.get_mut(&agent_id) {
                Some(a) => a,
                None => return,
            };

            match agent.process_handle.as_mut() {
                Some(child) => match child.try_wait() {
                    Ok(Some(status)) => Some(status),
                    Ok(None) => None,
                    Err(_) => Some(std::process::ExitStatus::default()),
                },
                None => return,
            }
        };

        match exit_status {
            Some(status) => {
                let success = status.success();

                // Extract restart decision data without holding agent borrow
                let (should_restart, restart_count, max_retries, agent_name) = {
                    let mut s = state.write().await;
                    let agent = match s.agents.get_mut(&agent_id) {
                        Some(a) => a,
                        None => return,
                    };

                    agent.process_handle = None;
                    agent.pid = None;

                    let should_restart = match agent.restart_policy {
                        RestartPolicy::Always => agent.restart_count < agent.max_retries,
                        RestartPolicy::OnFailure => !success && agent.restart_count < agent.max_retries,
                        RestartPolicy::Never => false,
                    };

                    if should_restart {
                        agent.state = AgentState::Restarting;
                        agent.restart_count += 1;
                    } else {
                        agent.state = if success { AgentState::Stopped } else { AgentState::Failed };
                        if !agent.task_state_machine.is_terminal() {
                            let target = if success { TaskState::Completed } else { TaskState::Failed };
                            let _ = agent.task_state_machine.transition(target);
                        }
                    }

                    (should_restart, agent.restart_count, agent.max_retries, agent.name.clone())
                };

                // Record event (separate borrow scope)
                {
                    let mut s = state.write().await;
                    if should_restart {
                        s.record_event(
                            "agent.restarting",
                            Some(&agent_id),
                            &format!("Process '{}' exited (success={}), restarting (attempt {}/{})",
                                     agent_name, success, restart_count, max_retries),
                        );
                    } else {
                        s.record_event(
                            if success { "agent.exited" } else { "agent.failed" },
                            Some(&agent_id),
                            &format!("Process '{}' exited (success={})", agent_name, success),
                        );
                        s.total_completed += 1;
                    }
                }

                if should_restart {
                    let delay = std::time::Duration::from_secs(1 << restart_count.min(5));
                    tokio::time::sleep(delay).await;

                    if let Err(e) = spawn_agent_process(state.clone(), &agent_id).await {
                        tracing::error!(agent_id = %agent_id, error = %e, "restart failed");
                        let mut s = state.write().await;
                        if let Some(agent) = s.agents.get_mut(&agent_id) {
                            agent.state = AgentState::Failed;
                        }
                    }
                }
                return;
            }
            None => {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
        }
    }
}
