// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! Integration tests for Agent Lifecycle Manager.
//!
//! Comprehensive test suite covering complete agent lifecycle operations:
//! start/stop sequences, state transitions, error handling, resource management,
//! and CT spawn integration. Tests verify correct behavior across multiple agents
//! and complex state scenarios.
//!
//! Reference: Engineering Plan § Agent Lifecycle Manager § Integration Tests

#[cfg(test)]
mod lifecycle_integration_tests {
    use crate::agent_start::{AgentStartHandler, AgentStartParams};
    use crate::agent_stop::{AgentStopHandler, AgentStopParams, TerminationSignal};
    use crate::ct_spawn_integration::{CtSpawnTranslator, QuotaPolicy};
    use crate::lifecycle_manager::LifecycleManager;
    use crate::unit_file::AgentUnitFile;
    use crate::LifecycleState;

    fn create_test_unit_file(name: &str) -> AgentUnitFile {
        AgentUnitFile::new(name, "1.0.0", "Test agent")
    }

    // ===== START/STOP LIFECYCLE TESTS =====

    #[test]
    fn test_complete_agent_lifecycle_start_to_stop() {
        let manager = LifecycleManager::new();
        let unit_file = create_test_unit_file("lifecycle-agent");

        // Start the agent
        let start_params = AgentStartParams::new("lifecycle-agent", unit_file.clone(), 1000, 30000);
        let start_result = AgentStartHandler::start_agent(&manager, start_params);

        assert!(start_result.is_ok());
        let spawn_result = start_result.unwrap();
        assert!(spawn_result.process_id > 0);

        // Verify agent is running
        assert_eq!(manager.get_agent_state("lifecycle-agent").unwrap(), LifecycleState::Running);

        // Stop the agent
        let stop_params = AgentStopParams::new("lifecycle-agent", spawn_result.process_id, 5000, 1000, 2000);
        let stop_result = AgentStopHandler::stop_agent(&manager, stop_params);

        assert!(stop_result.is_ok());

        // Verify agent is stopped
        assert_eq!(manager.get_agent_state("lifecycle-agent").unwrap(), LifecycleState::Stopped);
    }

    #[test]
    fn test_agent_lifecycle_with_graceful_shutdown() {
        let manager = LifecycleManager::new();
        let unit_file = create_test_unit_file("graceful-agent");

        // Start agent
        let start_params = AgentStartParams::new("graceful-agent", unit_file.clone(), 1000, 30000);
        let start_result = AgentStartHandler::start_agent(&manager, start_params).unwrap();

        // Stop with long graceful timeout (should succeed gracefully)
        let stop_params = AgentStopParams::new("graceful-agent", start_result.process_id, 5000, 1000, 2000);
        let graceful = AgentStopHandler::stop_agent(&manager, stop_params).unwrap();

        assert!(graceful);
    }

    #[test]
    fn test_agent_lifecycle_with_forced_shutdown() {
        let manager = LifecycleManager::new();
        let unit_file = create_test_unit_file("forced-agent");

        // Start agent
        let start_params = AgentStartParams::new("forced-agent", unit_file.clone(), 1000, 30000);
        let start_result = AgentStartHandler::start_agent(&manager, start_params).unwrap();

        // Stop with short graceful timeout (should force shutdown)
        let stop_params = AgentStopParams::new("forced-agent", start_result.process_id, 50, 1000, 2000);
        let graceful = AgentStopHandler::stop_agent(&manager, stop_params).unwrap();

        assert!(!graceful);
    }

    // ===== MULTIPLE AGENTS TESTS =====

    #[test]
    fn test_multiple_agents_concurrent_lifecycle() {
        let manager = LifecycleManager::new();

        // Start multiple agents
        let mut process_ids = Vec::new();
        for i in 0..5 {
            let unit_file = create_test_unit_file(&format!("agent-{}", i));
            let params = AgentStartParams::new(
                format!("agent-{}", i),
                unit_file,
                1000 + (i as u64 * 100),
                30000,
            );

            let result = AgentStartHandler::start_agent(&manager, params).unwrap();
            process_ids.push((format!("agent-{}", i), result.process_id));
        }

        assert_eq!(manager.total_agents(), 5);

        // Verify all agents are running
        for i in 0..5 {
            assert_eq!(
                manager.get_agent_state(&format!("agent-{}", i)).unwrap(),
                LifecycleState::Running
            );
        }

        // Stop all agents
        for (agent_id, pid) in process_ids {
            let params = AgentStopParams::new(&agent_id, pid, 5000, 1000, 3000);
            AgentStopHandler::stop_agent(&manager, params).unwrap();

            assert_eq!(
                manager.get_agent_state(&agent_id).unwrap(),
                LifecycleState::Stopped
            );
        }

        assert_eq!(manager.total_agents(), 5);
    }

    #[test]
    fn test_agents_in_different_states() {
        let manager = LifecycleManager::new();

        // Create 3 agents
        for i in 0..3 {
            let unit_file = create_test_unit_file(&format!("state-agent-{}", i));
            manager.register_agent(format!("state-agent-{}", i), unit_file, 1000).unwrap();
        }

        // Transition to different states
        manager.transition_agent("state-agent-0", LifecycleState::Starting, 1100).unwrap();
        manager.transition_agent("state-agent-1", LifecycleState::Running, 1100).unwrap();
        manager.transition_agent("state-agent-2", LifecycleState::Stopping, 1100).unwrap();

        // Verify state counts
        let (init, start, run, deg, stop, stopped, failed) = manager.count_agents_by_state();
        assert_eq!(init, 0);
        assert_eq!(start, 1);
        assert_eq!(run, 1);
        assert_eq!(deg, 0);
        assert_eq!(stop, 1);
        assert_eq!(stopped, 0);
        assert_eq!(failed, 0);
    }

    // ===== ERROR HANDLING TESTS =====

    #[test]
    fn test_start_agent_with_invalid_configuration() {
        let manager = LifecycleManager::new();
        let mut unit_file = create_test_unit_file("invalid-agent");
        unit_file.memory_mb = Some(0); // Invalid: zero memory

        let params = AgentStartParams::new("invalid-agent", unit_file, 1000, 30000);
        let result = AgentStartHandler::start_agent(&manager, params);

        assert!(result.is_err());
        assert_eq!(manager.total_agents(), 0); // Agent should not be registered
    }

    #[test]
    fn test_start_duplicate_agent_fails() {
        let manager = LifecycleManager::new();
        let unit_file = create_test_unit_file("dup-agent");

        // Start first instance
        let params1 = AgentStartParams::new("dup-agent", unit_file.clone(), 1000, 30000);
        AgentStartHandler::start_agent(&manager, params1).unwrap();

        // Try to start duplicate
        let params2 = AgentStartParams::new("dup-agent", unit_file, 2000, 30000);
        let result = AgentStartHandler::start_agent(&manager, params2);

        assert!(result.is_err());
    }

    #[test]
    fn test_stop_nonexistent_agent() {
        let manager = LifecycleManager::new();
        let params = AgentStopParams::new("nonexistent", 12345, 5000, 1000, 1000);

        let result = AgentStopHandler::stop_agent(&manager, params);
        assert!(result.is_err());
    }

    #[test]
    fn test_restart_count_tracking() {
        let manager = LifecycleManager::new();
        let unit_file = create_test_unit_file("restart-agent");

        manager.register_agent("restart-agent", unit_file, 1000).unwrap();

        // Simulate restarts
        assert_eq!(manager.increment_restart_count("restart-agent").unwrap(), 1);
        assert_eq!(manager.increment_restart_count("restart-agent").unwrap(), 2);
        assert_eq!(manager.increment_restart_count("restart-agent").unwrap(), 3);

        let info = manager.get_agent_info("restart-agent").unwrap();
        assert_eq!(info.restart_count, 3);
    }

    // ===== STATE TRANSITION TESTS =====

    #[test]
    fn test_state_transition_sequence() {
        let manager = LifecycleManager::new();
        let unit_file = create_test_unit_file("transition-agent");

        manager.register_agent("transition-agent", unit_file, 1000).unwrap();

        // Valid transition sequence
        assert!(manager.transition_agent("transition-agent", LifecycleState::Starting, 1100).is_ok());
        assert!(manager.transition_agent("transition-agent", LifecycleState::Running, 1200).is_ok());
        assert!(manager.transition_agent("transition-agent", LifecycleState::Stopping, 1300).is_ok());
        assert!(manager.transition_agent("transition-agent", LifecycleState::Stopped, 1400).is_ok());

        // Terminal state - no further transitions allowed
        assert!(manager.transition_agent("transition-agent", LifecycleState::Running, 1500).is_err());
    }

    #[test]
    fn test_invalid_state_transition() {
        let manager = LifecycleManager::new();
        let unit_file = create_test_unit_file("invalid-transition");

        manager.register_agent("invalid-transition", unit_file, 1000).unwrap();

        // Invalid transition from Initializing to Running
        let result = manager.transition_agent("invalid-transition", LifecycleState::Running, 1100);
        assert!(result.is_err());
    }

    // ===== RESOURCE QUOTA TESTS =====

    #[test]
    fn test_resource_quota_enforcement_strict() {
        let unit_file = create_test_unit_file("quota-agent")
            .with_memory_mb(65537); // Exceeds default limit

        let result = CtSpawnTranslator::translate(&unit_file, QuotaPolicy::Strict);
        assert!(result.is_err());
    }

    #[test]
    fn test_resource_quota_enforcement_permissive() {
        let unit_file = create_test_unit_file("quota-agent")
            .with_memory_mb(65537); // Exceeds default limit

        let result = CtSpawnTranslator::translate(&unit_file, QuotaPolicy::Permissive);
        assert!(result.is_ok());
    }

    #[test]
    fn test_ct_spawn_parameter_translation() {
        let unit_file = create_test_unit_file("ct-agent")
            .with_memory_mb(512)
            .with_cpu_cores(2.0)
            .with_capability("net_admin");

        let result = CtSpawnTranslator::translate(&unit_file, QuotaPolicy::Strict);
        assert!(result.is_ok());

        let params = result.unwrap();
        assert_eq!(params.memory_limit_bytes, Some(512 * 1024 * 1024));
        assert_eq!(params.cpu_cores_limit, Some(2.0));
        assert!(params.capabilities.contains(&"net_admin".to_string()));
    }

    // ===== COMPLEX SCENARIO TESTS =====

    #[test]
    fn test_agent_failure_and_restart_flow() {
        let manager = LifecycleManager::new();
        let unit_file = create_test_unit_file("resilient-agent");

        // Initial start
        let params = AgentStartParams::new("resilient-agent", unit_file.clone(), 1000, 30000);
        let start1 = AgentStartHandler::start_agent(&manager, params).unwrap();

        assert_eq!(manager.get_agent_state("resilient-agent").unwrap(), LifecycleState::Running);

        // Simulate failure
        manager.transition_agent("resilient-agent", LifecycleState::Failed, 2000).unwrap();
        manager.set_agent_error("resilient-agent", "Simulated failure").unwrap();

        assert_eq!(manager.get_agent_state("resilient-agent").unwrap(), LifecycleState::Failed);

        // Note: In real scenario, restart policy would handle this
        // For now, we just verify error tracking
        let info = manager.get_agent_info("resilient-agent").unwrap();
        assert!(info.error_message.is_some());
    }

    #[test]
    fn test_degraded_state_recovery() {
        let manager = LifecycleManager::new();
        let unit_file = create_test_unit_file("degraded-agent");

        manager.register_agent("degraded-agent", unit_file, 1000).unwrap();
        manager.transition_agent("degraded-agent", LifecycleState::Starting, 1100).unwrap();
        manager.transition_agent("degraded-agent", LifecycleState::Running, 1200).unwrap();

        // Transition to Degraded
        manager.transition_agent("degraded-agent", LifecycleState::Degraded, 1300).unwrap();
        assert_eq!(manager.get_agent_state("degraded-agent").unwrap(), LifecycleState::Degraded);

        // Recover to Running
        manager.transition_agent("degraded-agent", LifecycleState::Running, 1400).unwrap();
        assert_eq!(manager.get_agent_state("degraded-agent").unwrap(), LifecycleState::Running);
    }

    #[test]
    fn test_agent_with_comprehensive_configuration() {
        let manager = LifecycleManager::new();

        let unit_file = create_test_unit_file("comprehensive-agent")
            .with_memory_mb(1024)
            .with_cpu_cores(4.0)
            .with_env("AGENT_ROLE", "primary")
            .with_env("LOG_LEVEL", "debug")
            .with_capability("net_admin")
            .with_capability("sys_ptrace");

        let params = AgentStartParams::new("comprehensive-agent", unit_file, 1000, 30000);
        let result = AgentStartHandler::start_agent(&manager, params).unwrap();

        // Verify agent is properly configured and running
        assert_eq!(manager.get_agent_state("comprehensive-agent").unwrap(), LifecycleState::Running);

        let info = manager.get_agent_info("comprehensive-agent").unwrap();
        assert_eq!(info.unit_file.memory_mb, Some(1024));
        assert_eq!(info.unit_file.cpu_cores, Some(4.0));
        assert!(info.ct_process_id.is_some());
    }

    #[test]
    fn test_sequential_agent_start_and_stop() {
        let manager = LifecycleManager::new();

        for i in 0..10 {
            let unit_file = create_test_unit_file(&format!("seq-agent-{}", i));
            let params = AgentStartParams::new(
                format!("seq-agent-{}", i),
                unit_file,
                1000 + (i as u64 * 100),
                30000,
            );

            AgentStartHandler::start_agent(&manager, params).unwrap();
        }

        assert_eq!(manager.total_agents(), 10);

        for i in 0..10 {
            let info = manager.get_agent_info(&format!("seq-agent-{}", i)).unwrap();
            let params = AgentStopParams::new(
                format!("seq-agent-{}", i),
                info.ct_process_id.unwrap(),
                5000,
                1000,
                2000 + (i as u64 * 100),
            );

            AgentStopHandler::stop_agent(&manager, params).unwrap();
        }

        // All agents should be stopped
        for i in 0..10 {
            assert_eq!(
                manager.get_agent_state(&format!("seq-agent-{}", i)).unwrap(),
                LifecycleState::Stopped
            );
        }
    }

    #[test]
    fn test_signal_delivery_with_various_timeouts() {
        // Test SIGTERM delivery
        let result1 = AgentStopHandler::send_signal("agent1", 12345, TerminationSignal::Terminate, 1000);
        assert!(result1.sent);

        // Test SIGKILL delivery
        let result2 = AgentStopHandler::send_signal("agent2", 12346, TerminationSignal::Kill, 1000);
        assert!(result2.sent);

        // Test with invalid PID
        let result3 = AgentStopHandler::send_signal("agent3", 0, TerminationSignal::Terminate, 1000);
        assert!(!result3.sent);
    }

    #[test]
    fn test_agent_lifecycle_info_retrieval() {
        let manager = LifecycleManager::new();
        let unit_file = create_test_unit_file("info-agent");

        manager.register_agent("info-agent", unit_file, 1000).unwrap();
        manager.transition_agent("info-agent", LifecycleState::Starting, 1100).unwrap();
        manager.set_agent_ct_process_id("info-agent", 54321).unwrap();

        let info = manager.get_agent_info("info-agent").unwrap();

        assert_eq!(info.agent_id, "info-agent");
        assert_eq!(info.state, LifecycleState::Starting);
        assert_eq!(info.ct_process_id, Some(54321));
        assert_eq!(info.restart_count, 0);
        assert!(info.error_message.is_none());
    }
}
