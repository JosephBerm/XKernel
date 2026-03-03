// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.

//! Integration tests for semantic filesystem agent lifecycle management.
//!
//! Tests agent state machines, mount providers, CLI operations, and lifecycle hooks.

#[cfg(test)]
mod integration_tests {
    use cs_semantic_fs_agent_lifecycle::agent_lifecycle::{
        AgentUnit, AgentState, HealthProbe, RestartPolicy, StateMachine,
    };
    use cs_semantic_fs_agent_lifecycle::mounts::MountProvider;
    use cs_semantic_fs_agent_lifecycle::semantic_fs::{QueryEngine, TagSystem, PathResolver, MountManager};
    use cs_semantic_fs_agent_lifecycle::cli::AgentCtl;

    #[test]
    fn test_agent_unit_creation() {
        let unit = AgentUnit::new("test-agent".into(), "langchain".into());
        assert_eq!(unit.name, "test-agent");
        assert_eq!(unit.framework, "langchain");
        assert_eq!(unit.state, AgentState::Created);
    }

    #[test]
    fn test_health_probe_configuration() {
        let probe = HealthProbe::new("liveness".into(), "http".into(), 5000);
        assert_eq!(probe.name, "liveness");
        assert_eq!(probe.probe_type, "http");
        assert_eq!(probe.timeout_ms, 5000);
    }

    #[test]
    fn test_state_machine_full_lifecycle() {
        let mut sm = StateMachine::new();
        assert_eq!(sm.current_state(), AgentState::Created);

        assert!(sm.start().is_ok());
        assert_eq!(sm.current_state(), AgentState::Starting);

        assert!(sm.run().is_ok());
        assert_eq!(sm.current_state(), AgentState::Running);

        assert!(sm.stop().is_ok());
        assert_eq!(sm.current_state(), AgentState::Stopping);

        assert!(sm.stopped().is_ok());
        assert_eq!(sm.current_state(), AgentState::Stopped);
    }

    #[test]
    fn test_state_machine_failure_recovery() {
        let mut sm = StateMachine::new();

        assert!(sm.start().is_ok());
        assert!(sm.run().is_ok());
        assert!(sm.fail().is_ok());
        assert_eq!(sm.current_state(), AgentState::Failed);
    }

    #[test]
    fn test_semantic_fs_query_engine() {
        let engine = QueryEngine::new();
        let result = engine.query("SELECT agents WHERE state=running");
        assert!(result.is_ok());
    }

    #[test]
    fn test_semantic_fs_tag_system() {
        let system = TagSystem::new();
        assert!(system.tag("agent-1", "production").is_ok());
        assert!(system.tag("agent-1", "critical").is_ok());
    }

    #[test]
    fn test_semantic_fs_path_resolver() {
        let resolver = PathResolver::new();
        let result = resolver.resolve("/agents/agent-1/config");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "/agents/agent-1/config");
    }

    #[test]
    fn test_semantic_fs_mount_manager() {
        let manager = MountManager::new();
        assert!(manager.mount("/local/path", "/mnt/agents").is_ok());
    }

    #[test]
    fn test_agentctl_start_stop_status() {
        let ctl = AgentCtl::new();

        let start_result = ctl.start("test-agent");
        assert!(start_result.is_ok());

        let status_result = ctl.status("test-agent");
        assert!(status_result.is_ok());

        let stop_result = ctl.stop("test-agent");
        assert!(stop_result.is_ok());
    }

    #[test]
    fn test_agentctl_list_and_logs() {
        let ctl = AgentCtl::new();

        let list_result = ctl.list();
        assert!(list_result.is_ok());

        let logs_result = ctl.logs("test-agent");
        assert!(logs_result.is_ok());
    }

    #[test]
    fn test_mount_manager_multiple_mounts() {
        let manager = MountManager::new();

        assert!(manager.mount("/data/s3", "/mnt/s3-bucket").is_ok());
        assert!(manager.mount("/data/db", "/mnt/postgres").is_ok());
        assert!(manager.mount("/data/http", "/mnt/api").is_ok());
    }

    #[test]
    fn test_agent_with_health_probes() {
        let unit = AgentUnit::new("monitored-agent".into(), "semantic_kernel".into());
        let probe1 = HealthProbe::new("readiness".into(), "http".into(), 3000);
        let probe2 = HealthProbe::new("liveness".into(), "tcp".into(), 5000);

        assert_eq!(unit.state, AgentState::Created);
        assert_eq!(probe1.timeout_ms, 3000);
        assert_eq!(probe2.timeout_ms, 5000);
    }

    #[test]
    fn test_restart_policy_enumeration() {
        let never = RestartPolicy::Never;
        let always = RestartPolicy::Always;
        let on_failure = RestartPolicy::OnFailure;

        assert!(!format!("{:?}", never).is_empty());
        assert!(!format!("{:?}", always).is_empty());
        assert!(!format!("{:?}", on_failure).is_empty());
    }

    #[test]
    fn test_state_machine_invalid_transitions() {
        let mut sm = StateMachine::new();

        // Can't run without starting
        assert!(sm.run().is_err());

        // Can't stop without running
        assert!(sm.stop().is_err());

        // Valid: start -> run
        assert!(sm.start().is_ok());
        assert!(sm.run().is_ok());

        // Now stop is valid
        assert!(sm.stop().is_ok());
    }

    #[test]
    fn test_multiple_agents_independent_lifecycles() {
        let mut sm1 = StateMachine::new();
        let mut sm2 = StateMachine::new();

        assert!(sm1.start().is_ok());
        assert!(sm1.run().is_ok());

        assert!(sm2.start().is_ok());

        assert_eq!(sm1.current_state(), AgentState::Running);
        assert_eq!(sm2.current_state(), AgentState::Starting);

        assert!(sm1.stop().is_ok());
        assert!(sm2.run().is_ok());

        assert_eq!(sm1.current_state(), AgentState::Stopping);
        assert_eq!(sm2.current_state(), AgentState::Running);
    }

    #[test]
    fn test_semantic_fs_empty_query_error() {
        let engine = QueryEngine::new();
        let result = engine.query("");
        assert!(result.is_err());
    }

    #[test]
    fn test_agentctl_empty_agent_name_error() {
        let ctl = AgentCtl::new();
        let result = ctl.start("");
        assert!(result.is_err());
    }
}
