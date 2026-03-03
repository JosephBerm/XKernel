// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.

//! Integration tests for framework adapters.
//!
//! Tests adapter lifecycle, event translation, agent spawning, and multi-adapter scenarios.

#[cfg(test)]
mod integration_tests {
    use cs_framework_adapters::adapter_base::{
        AdapterConfig, AdapterLifecycleState, AdapterResult, FrameworkAdapter, P95_LATENCY_TARGET_MS, MAX_MEMORY_PER_AGENT_MB,
    };
    use cs_framework_adapters::adapter_impl::{LangChainAdapter, SemanticKernelAdapter, AutoGenAdapter, CrewAIAdapter, CustomAdapter};

    #[test]
    fn test_langchain_full_lifecycle() {
        let mut adapter = LangChainAdapter::new();
        assert_eq!(adapter.state(), AdapterLifecycleState::Created);

        let config = AdapterConfig::new("lc-test".into(), "langchain".into());
        assert!(adapter.initialize(config.clone()).is_ok());
        assert_eq!(adapter.state(), AdapterLifecycleState::Initialized);

        let handle = adapter.spawn_agent(&config);
        assert!(handle.is_ok());

        assert!(adapter.shutdown().is_ok());
        assert_eq!(adapter.state(), AdapterLifecycleState::Shutdown);
    }

    #[test]
    fn test_semantic_kernel_event_translation() {
        let mut adapter = SemanticKernelAdapter::new();
        let config = AdapterConfig::new("sk-test".into(), "semantic_kernel".into());

        assert!(adapter.initialize(config.clone()).is_ok());

        let event_data = b"test_event_payload";
        let result = adapter.translate_event(event_data);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), event_data.len());
    }

    #[test]
    fn test_autogen_memory_tracking() {
        let mut adapter = AutoGenAdapter::new();
        let config = AdapterConfig::new("autogen-test".into(), "autogen".into());

        assert!(adapter.initialize(config.clone()).is_ok());
        assert_eq!(adapter.memory_used_mb(), 0);

        let _handle = adapter.spawn_agent(&config);
        assert!(adapter.memory_used_mb() > 0);
    }

    #[test]
    fn test_crewai_adapter_config_customization() {
        let mut adapter = CrewAIAdapter::new();
        let mut config = AdapterConfig::new("crew-test".into(), "crewai".into());
        config.max_agents = 50;
        config.memory_limit_mb = 20;
        config.timeout_ms = 1000;

        assert!(adapter.initialize(config).is_ok());
        assert_eq!(adapter.state(), AdapterLifecycleState::Initialized);
    }

    #[test]
    fn test_custom_adapter_multiple_agents() {
        let mut adapter = CustomAdapter::new();
        let config = AdapterConfig::new("custom-test".into(), "custom".into());

        assert!(adapter.initialize(config.clone()).is_ok());

        let handle1 = adapter.spawn_agent(&config);
        assert!(handle1.is_ok());

        let handle2 = adapter.spawn_agent(&config);
        assert!(handle2.is_ok());

        assert_eq!(handle1.unwrap().id(), 0);
        assert_eq!(handle2.unwrap().id(), 1);
    }

    #[test]
    fn test_adapter_performance_targets() {
        assert_eq!(P95_LATENCY_TARGET_MS, 500);
        assert_eq!(MAX_MEMORY_PER_AGENT_MB, 15);
    }

    #[test]
    fn test_adapter_error_handling_on_uninitialized() {
        let mut adapter = LangChainAdapter::new();
        let config = AdapterConfig::new("test".into(), "langchain".into());

        let result = adapter.spawn_agent(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_event_translation_empty_payload() {
        let mut adapter = SemanticKernelAdapter::new();
        let config = AdapterConfig::new("test".into(), "semantic_kernel".into());

        assert!(adapter.initialize(config).is_ok());
        let result = adapter.translate_event(b"");
        assert!(result.is_err());
    }

    #[test]
    fn test_adapter_state_transitions() {
        let mut adapter = LangChainAdapter::new();
        let config = AdapterConfig::new("test".into(), "langchain".into());

        assert_eq!(adapter.state(), AdapterLifecycleState::Created);
        assert!(adapter.initialize(config.clone()).is_ok());
        assert_eq!(adapter.state(), AdapterLifecycleState::Initialized);

        let _handle = adapter.spawn_agent(&config);
        assert_eq!(adapter.state(), AdapterLifecycleState::Running);

        assert!(adapter.shutdown().is_ok());
        assert_eq!(adapter.state(), AdapterLifecycleState::Shutdown);
    }

    #[test]
    fn test_multi_framework_coexistence() {
        let mut lc = LangChainAdapter::new();
        let mut sk = SemanticKernelAdapter::new();
        let mut crew = CrewAIAdapter::new();

        let lc_config = AdapterConfig::new("lc".into(), "langchain".into());
        let sk_config = AdapterConfig::new("sk".into(), "semantic_kernel".into());
        let crew_config = AdapterConfig::new("crew".into(), "crewai".into());

        assert!(lc.initialize(lc_config.clone()).is_ok());
        assert!(sk.initialize(sk_config.clone()).is_ok());
        assert!(crew.initialize(crew_config.clone()).is_ok());

        let lc_agent = lc.spawn_agent(&lc_config);
        let sk_agent = sk.spawn_agent(&sk_config);
        let crew_agent = crew.spawn_agent(&crew_config);

        assert!(lc_agent.is_ok());
        assert!(sk_agent.is_ok());
        assert!(crew_agent.is_ok());
    }
}
