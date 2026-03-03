// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.

//! # Framework Tool → ToolBinding Translation
//!
//! Translates framework-specific tool definitions into CSCI-compatible ToolBinding configurations.
//! Each framework has different tool interfaces, invocation patterns, and result serialization requirements.
//!
//! This module provides framework-specific translators that handle:
//! - Input argument serialization (framework format → CSCI format)
//! - Output result deserialization (CSCI format → framework format)
//! - Sandbox and capability configuration
//!
//! Sec 4.2: Tool Translation Interface
//! Sec 4.2: Tool Binding Configuration

use crate::{AdapterError, framework_type::FrameworkType};

/// Framework-specific tool definition.
/// Sec 4.2: Framework Tool Definition
#[derive(Debug, Clone)]
pub struct FrameworkTool {
    /// Tool identifier within the framework
    pub name: String,
    /// Human-readable tool description
    pub description: String,
    /// Input parameter schema (JSON schema format)
    pub input_schema: String,
    /// Output result schema (JSON schema format)
    pub output_schema: String,
    /// Framework-specific metadata
    pub framework_metadata: Option<String>,
}

impl FrameworkTool {
    /// Creates a new framework tool.
    pub fn new(name: String, description: String, input_schema: String, output_schema: String) -> Self {
        FrameworkTool {
            name,
            description,
            input_schema,
            output_schema,
            framework_metadata: None,
        }
    }

    /// Sets framework-specific metadata.
    pub fn with_metadata(mut self, metadata: String) -> Self {
        self.framework_metadata = Some(metadata);
        self
    }
}

/// Sandbox configuration for tool execution.
/// Sec 4.2: Sandbox Configuration
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    /// Whether tool execution requires sandboxing
    pub enabled: bool,
    /// Maximum execution time in milliseconds
    pub timeout_ms: u64,
    /// Maximum memory allocation in bytes
    pub max_memory_bytes: u64,
    /// Whether tool can access filesystem
    pub allow_filesystem: bool,
    /// Whether tool can make network calls
    pub allow_network: bool,
    /// Resource isolation level
    pub isolation_level: String,
}

impl SandboxConfig {
    /// Creates a default sandbox configuration.
    pub fn default_secure() -> Self {
        SandboxConfig {
            enabled: true,
            timeout_ms: 30000,
            max_memory_bytes: 256 * 1024 * 1024, // 256MB
            allow_filesystem: false,
            allow_network: false,
            isolation_level: "strict".into(),
        }
    }

    /// Creates a permissive sandbox configuration.
    pub fn permissive() -> Self {
        SandboxConfig {
            enabled: true,
            timeout_ms: 60000,
            max_memory_bytes: 1024 * 1024 * 1024, // 1GB
            allow_filesystem: true,
            allow_network: true,
            isolation_level: "moderate".into(),
        }
    }
}

/// Effect classification for tool invocation.
/// Sec 4.2: Tool Effect Classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EffectClass {
    /// Read-only tool with no side effects
    ReadOnly,
    /// Tool with restricted side effects (local state only)
    RestrictedWrite,
    /// Tool with full side effects
    Full,
    /// Unknown effect class
    Unknown,
}

impl EffectClass {
    /// Returns string representation of the effect class.
    pub fn as_str(&self) -> &'static str {
        match self {
            EffectClass::ReadOnly => "read_only",
            EffectClass::RestrictedWrite => "restricted_write",
            EffectClass::Full => "full",
            EffectClass::Unknown => "unknown",
        }
    }
}

/// CSCI-compatible tool binding configuration.
/// Sec 4.2: ToolBindingConfig Structure
#[derive(Debug, Clone)]
pub struct ToolBindingConfig {
    /// Tool specification
    pub tool_spec: FrameworkTool,
    /// Sandbox execution configuration
    pub sandbox_config: SandboxConfig,
    /// Effect class for the tool
    pub effect_class: EffectClass,
    /// Capability requirements for execution
    pub capability_requirements: Vec<String>,
    /// Whether tool requires explicit authorization
    pub requires_authorization: bool,
}

impl ToolBindingConfig {
    /// Creates a new tool binding configuration.
    pub fn new(tool_spec: FrameworkTool, effect_class: EffectClass) -> Self {
        ToolBindingConfig {
            tool_spec,
            sandbox_config: SandboxConfig::default_secure(),
            effect_class,
            capability_requirements: Vec::new(),
            requires_authorization: effect_class == EffectClass::Full,
        }
    }

    /// Adds a capability requirement.
    pub fn add_capability_requirement(&mut self, capability: String) {
        self.capability_requirements.push(capability);
    }

    /// Sets the sandbox configuration.
    pub fn set_sandbox_config(&mut self, config: SandboxConfig) {
        self.sandbox_config = config;
    }

    /// Sets the authorization requirement.
    pub fn set_requires_authorization(&mut self, requires: bool) {
        self.requires_authorization = requires;
    }
}

/// Argument serialization trait for converting framework arguments to CSCI format.
/// Sec 4.2: ArgumentSerializer Interface
pub trait ArgumentSerializer {
    /// Serializes framework arguments to CSCI tool_invoke format.
    fn serialize(&self, framework_args: &str) -> Result<String, AdapterError>;
}

/// Result deserialization trait for converting CSCI results to framework format.
/// Sec 4.2: ResultDeserializer Interface
pub trait ResultDeserializer {
    /// Deserializes CSCI tool result to framework-specific format.
    fn deserialize(&self, csci_result: &str) -> Result<String, AdapterError>;
}

/// Translates a framework tool to a CSCI ToolBindingConfig.
/// Sec 4.2: Tool Translation Method
pub fn translate_tool(
    framework_tool: &FrameworkTool,
    framework_type: FrameworkType,
) -> Result<ToolBindingConfig, AdapterError> {
    let effect_class = classify_tool_effect(&framework_tool.name);
    let mut config = ToolBindingConfig::new(framework_tool.clone(), effect_class);

    // Framework-specific configuration
    match framework_type {
        FrameworkType::LangChain => {
            config.sandbox_config = SandboxConfig::permissive();
            config.add_capability_requirement("llm_access".into());
            config.add_capability_requirement("memory_read".into());
        }
        FrameworkType::SemanticKernel => {
            config.sandbox_config = SandboxConfig::default_secure();
            config.add_capability_requirement("semantic_kernel_execution".into());
        }
        FrameworkType::CrewAI => {
            config.sandbox_config = SandboxConfig::permissive();
            config.add_capability_requirement("agent_coordination".into());
            config.add_capability_requirement("memory_write".into());
        }
        FrameworkType::AutoGen => {
            config.sandbox_config = SandboxConfig::permissive();
            config.add_capability_requirement("conversation_management".into());
            config.add_capability_requirement("tool_chaining".into());
        }
    }

    Ok(config)
}

/// Classifies tool effect based on naming conventions.
fn classify_tool_effect(tool_name: &str) -> EffectClass {
    let lower = tool_name.to_lowercase();
    if lower.contains("read") || lower.contains("get") || lower.contains("query") {
        EffectClass::ReadOnly
    } else if lower.contains("write") || lower.contains("create") || lower.contains("update") {
        EffectClass::RestrictedWrite
    } else if lower.contains("delete") || lower.contains("execute") || lower.contains("invoke") {
        EffectClass::Full
    } else {
        EffectClass::Unknown
    }
}

/// LangChain-specific tool translator.
/// Sec 4.3: LangChain Tool Mapping
pub struct LangChainToolTranslator;

impl ArgumentSerializer for LangChainToolTranslator {
    /// Serializes LangChain tool arguments to CSCI format.
    /// LangChain uses dictionary-style arguments mapped to JSON.
    fn serialize(&self, framework_args: &str) -> Result<String, AdapterError> {
        // LangChain format: {"arg_name": value, ...}
        // CSCI format: same (JSON)
        Ok(framework_args.to_string())
    }
}

impl ResultDeserializer for LangChainToolTranslator {
    /// Deserializes CSCI result to LangChain format.
    fn deserialize(&self, csci_result: &str) -> Result<String, AdapterError> {
        // LangChain expects string or dict-like results
        Ok(csci_result.to_string())
    }
}

/// Semantic Kernel-specific tool translator.
/// Sec 4.3: Semantic Kernel Tool Mapping
pub struct SemanticKernelToolTranslator;

impl ArgumentSerializer for SemanticKernelToolTranslator {
    /// Serializes Semantic Kernel arguments to CSCI format.
    /// Semantic Kernel uses structured KernelArguments.
    fn serialize(&self, framework_args: &str) -> Result<String, AdapterError> {
        // Semantic Kernel format: structured args
        // CSCI format: JSON args
        Ok(framework_args.to_string())
    }
}

impl ResultDeserializer for SemanticKernelToolTranslator {
    /// Deserializes CSCI result to Semantic Kernel format.
    fn deserialize(&self, csci_result: &str) -> Result<String, AdapterError> {
        // Semantic Kernel expects FunctionResult
        Ok(format!(r#"{{"result": {}}}"#, csci_result))
    }
}

/// CrewAI-specific tool translator.
/// Sec 4.3: CrewAI Tool Mapping
pub struct CrewAIToolTranslator;

impl ArgumentSerializer for CrewAIToolTranslator {
    /// Serializes CrewAI tool arguments to CSCI format.
    /// CrewAI uses dictionary arguments.
    fn serialize(&self, framework_args: &str) -> Result<String, AdapterError> {
        // CrewAI format: dictionary
        // CSCI format: JSON
        Ok(framework_args.to_string())
    }
}

impl ResultDeserializer for CrewAIToolTranslator {
    /// Deserializes CSCI result to CrewAI format.
    fn deserialize(&self, csci_result: &str) -> Result<String, AdapterError> {
        // CrewAI expects string output
        Ok(csci_result.to_string())
    }
}

/// AutoGen-specific tool translator.
/// Sec 4.3: AutoGen Tool Mapping
pub struct AutoGenToolTranslator;

impl ArgumentSerializer for AutoGenToolTranslator {
    /// Serializes AutoGen tool arguments to CSCI format.
    /// AutoGen uses dictionary arguments with type hints.
    fn serialize(&self, framework_args: &str) -> Result<String, AdapterError> {
        // AutoGen format: dictionary with potential type hints
        // CSCI format: JSON
        Ok(framework_args.to_string())
    }
}

impl ResultDeserializer for AutoGenToolTranslator {
    /// Deserializes CSCI result to AutoGen format.
    fn deserialize(&self, csci_result: &str) -> Result<String, AdapterError> {
        // AutoGen expects JSON or string result
        Ok(csci_result.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_framework_tool_creation() {
        let tool = FrameworkTool::new(
            "search".into(),
            "Search the web".into(),
            "{}".into(),
            "{}".into(),
        );
        assert_eq!(tool.name, "search");
        assert_eq!(tool.description, "Search the web");
    }

    #[test]
    fn test_framework_tool_with_metadata() {
        let tool = FrameworkTool::new(
            "search".into(),
            "Search the web".into(),
            "{}".into(),
            "{}".into(),
        ).with_metadata("framework_specific_data".into());

        assert_eq!(tool.framework_metadata, Some("framework_specific_data".into()));
    }

    #[test]
    fn test_sandbox_config_default_secure() {
        let config = SandboxConfig::default_secure();
        assert!(config.enabled);
        assert!(!config.allow_filesystem);
        assert!(!config.allow_network);
        assert_eq!(config.isolation_level, "strict");
    }

    #[test]
    fn test_sandbox_config_permissive() {
        let config = SandboxConfig::permissive();
        assert!(config.enabled);
        assert!(config.allow_filesystem);
        assert!(config.allow_network);
        assert_eq!(config.isolation_level, "moderate");
    }

    #[test]
    fn test_effect_class_as_str() {
        assert_eq!(EffectClass::ReadOnly.as_str(), "read_only");
        assert_eq!(EffectClass::RestrictedWrite.as_str(), "restricted_write");
        assert_eq!(EffectClass::Full.as_str(), "full");
        assert_eq!(EffectClass::Unknown.as_str(), "unknown");
    }

    #[test]
    fn test_tool_binding_config_creation() {
        let tool = FrameworkTool::new(
            "tool1".into(),
            "desc".into(),
            "{}".into(),
            "{}".into(),
        );
        let config = ToolBindingConfig::new(tool, EffectClass::ReadOnly);
        assert_eq!(config.effect_class, EffectClass::ReadOnly);
        assert!(!config.requires_authorization);
    }

    #[test]
    fn test_tool_binding_config_full_effect_requires_auth() {
        let tool = FrameworkTool::new(
            "tool1".into(),
            "desc".into(),
            "{}".into(),
            "{}".into(),
        );
        let config = ToolBindingConfig::new(tool, EffectClass::Full);
        assert!(config.requires_authorization);
    }

    #[test]
    fn test_tool_binding_config_add_capability() {
        let tool = FrameworkTool::new(
            "tool1".into(),
            "desc".into(),
            "{}".into(),
            "{}".into(),
        );
        let mut config = ToolBindingConfig::new(tool, EffectClass::ReadOnly);
        config.add_capability_requirement("cap1".into());
        config.add_capability_requirement("cap2".into());

        assert_eq!(config.capability_requirements.len(), 2);
    }

    #[test]
    fn test_classify_tool_effect_readonly() {
        assert_eq!(classify_tool_effect("read_data"), EffectClass::ReadOnly);
        assert_eq!(classify_tool_effect("get_user"), EffectClass::ReadOnly);
        assert_eq!(classify_tool_effect("query_db"), EffectClass::ReadOnly);
    }

    #[test]
    fn test_classify_tool_effect_write() {
        assert_eq!(classify_tool_effect("write_data"), EffectClass::RestrictedWrite);
        assert_eq!(classify_tool_effect("create_user"), EffectClass::RestrictedWrite);
        assert_eq!(classify_tool_effect("update_record"), EffectClass::RestrictedWrite);
    }

    #[test]
    fn test_classify_tool_effect_full() {
        assert_eq!(classify_tool_effect("delete_user"), EffectClass::Full);
        assert_eq!(classify_tool_effect("execute_command"), EffectClass::Full);
        assert_eq!(classify_tool_effect("invoke_api"), EffectClass::Full);
    }

    #[test]
    fn test_translate_tool_langchain() {
        let tool = FrameworkTool::new(
            "search".into(),
            "Search".into(),
            "{}".into(),
            "{}".into(),
        );
        let config = translate_tool(&tool, FrameworkType::LangChain)
            .expect("translation failed");

        assert_eq!(config.effect_class, EffectClass::ReadOnly);
        assert!(config.capability_requirements.iter().any(|c| c.contains("llm")));
    }

    #[test]
    fn test_translate_tool_semantickernel() {
        let tool = FrameworkTool::new(
            "process".into(),
            "Process".into(),
            "{}".into(),
            "{}".into(),
        );
        let config = translate_tool(&tool, FrameworkType::SemanticKernel)
            .expect("translation failed");

        assert!(config.capability_requirements.iter().any(|c| c.contains("semantic")));
    }

    #[test]
    fn test_translate_tool_crewai() {
        let tool = FrameworkTool::new(
            "create_task".into(),
            "Create task".into(),
            "{}".into(),
            "{}".into(),
        );
        let config = translate_tool(&tool, FrameworkType::CrewAI)
            .expect("translation failed");

        assert!(config.capability_requirements.iter().any(|c| c.contains("coordination")));
    }

    #[test]
    fn test_translate_tool_autogen() {
        let tool = FrameworkTool::new(
            "invoke_tool".into(),
            "Invoke".into(),
            "{}".into(),
            "{}".into(),
        );
        let config = translate_tool(&tool, FrameworkType::AutoGen)
            .expect("translation failed");

        assert!(config.capability_requirements.iter().any(|c| c.contains("conversation")));
    }

    #[test]
    fn test_langchain_tool_translator_serialize() {
        let translator = LangChainToolTranslator;
        let result = translator.serialize(r#"{"query": "test"}"#);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), r#"{"query": "test"}"#);
    }

    #[test]
    fn test_langchain_tool_translator_deserialize() {
        let translator = LangChainToolTranslator;
        let result = translator.deserialize(r#"{"result": "ok"}"#);
        assert!(result.is_ok());
    }

    #[test]
    fn test_semantic_kernel_tool_translator_deserialize() {
        let translator = SemanticKernelToolTranslator;
        let result = translator.deserialize(r#"{"result": "ok"}"#);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("result"));
    }

    #[test]
    fn test_crewai_tool_translator() {
        let translator = CrewAIToolTranslator;
        let serialized = translator.serialize(r#"{"task": "search"}"#).unwrap();
        assert_eq!(serialized, r#"{"task": "search"}"#);

        let deserialized = translator.deserialize(r#"{"output": "found"}"#).unwrap();
        assert_eq!(deserialized, r#"{"output": "found"}"#);
    }

    #[test]
    fn test_autogen_tool_translator() {
        let translator = AutoGenToolTranslator;
        let serialized = translator.serialize(r#"{"command": "execute"}"#).unwrap();
        assert!(serialized.contains("command"));

        let deserialized = translator.deserialize(r#"{"status": "success"}"#).unwrap();
        assert!(deserialized.contains("status"));
    }
}
