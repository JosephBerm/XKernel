// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Mock tool implementations for testing and development.
//!
//! This module provides three concrete mock tools demonstrating different effect classes:
//! 1. MockWebSearchTool (ReadOnly) - queries external data without modification
//! 2. MockDatabaseTool (WriteReversible) - modifies state with undo capability
//! 3. MockEmailTool (WriteIrreversible) - sends immutable messages (no undo)
//!
//! These mocks are used for development and testing of the tool registry and
//! effect class enforcement system before integration with real tools.
//!
//! See Engineering Plan § 2.11: ToolBinding Entity & Tool Registry.
//! See Engineering Plan § 2.11.2: Effect Classes.

use crate::cache::CacheConfig;
use crate::commit_protocol::{CommitProtocol, RollbackStrategy};
use crate::effect_class::EffectClass;
use crate::ids::{AgentID, CapID, ToolBindingID, ToolID};
use crate::sandbox::SandboxConfig;
use crate::schema::{SchemaDefinition, TypeSchema};
use crate::tool_binding::ToolBinding;
use alloc::string::String;
use alloc::vec::Vec;

/// Mock web search tool - READ_ONLY effect class.
///
/// Represents a tool that queries external data sources without modifying any state.
/// Example: Web search, API queries, database reads.
///
/// # Effect Class: ReadOnly
///
/// - No state mutations allowed
/// - No confirmation required
/// - Safe for untrusted/unauthenticated execution
/// - Results are cacheable
///
/// # Configuration
///
/// - Sandbox: Network access allowed (queries external APIs)
/// - Cache: Enabled (24-hour TTL for search results)
/// - No commit protocol needed
///
/// See Engineering Plan § 2.11.2: Effect Classes - ReadOnly.
#[derive(Clone, Debug)]
pub struct MockWebSearchTool;

impl MockWebSearchTool {
    /// Creates a tool binding for the web search tool.
    ///
    /// # Returns
    ///
    /// A ToolBinding configured for read-only web search with network access and caching.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let binding = MockWebSearchTool::create_binding(
    ///     AgentID::new("agent-1"),
    ///     CapID::from_bytes([42u8; 32]),
    /// );
    /// assert!(binding.is_read_only());
    /// assert_eq!(binding.effect_class, EffectClass::ReadOnly);
    /// ```
    pub fn create_binding(agent: AgentID, capability: CapID) -> ToolBinding {
        let input_schema = SchemaDefinition::new("WebSearchInput");
        let output_schema = SchemaDefinition::new("WebSearchOutput");
        let schema = TypeSchema::new(input_schema, output_schema);

        let mut binding = ToolBinding::new(
            ToolBindingID::new("web-search"),
            ToolID::new("web-search-api"),
            agent,
            capability,
            schema,
        );

        // ReadOnly effect class - no mutations
        binding.effect_class = EffectClass::ReadOnly;

        // Network access for querying external APIs
        binding.sandbox_config = SandboxConfig::balanced();

        // Enable caching for search results (24-hour TTL)
        binding.response_cache = CacheConfig::long_lived();

        binding
    }

    /// Returns the tool ID for web search.
    pub fn tool_id() -> ToolID {
        ToolID::new("web-search-api")
    }

    /// Returns the binding ID for web search.
    pub fn binding_id() -> ToolBindingID {
        ToolBindingID::new("web-search")
    }
}

/// Mock database tool - WRITE_REVERSIBLE effect class.
///
/// Represents a tool that modifies database state but supports undo operations
/// through transaction logging and undo stack.
///
/// Example: Database updates, configuration changes, content edits.
///
/// # Effect Class: WriteReversible
///
/// - State mutations allowed
/// - All mutations logged and reversible
/// - Undo/redo stack support required
/// - Transactional consistency guaranteed
///
/// # Configuration
///
/// - Sandbox: Restricted (database access only, no external network)
/// - Cache: Disabled (mutable data shouldn't be cached)
/// - Commit protocol: PREPARE/COMMIT for transaction support
///
/// See Engineering Plan § 2.11.2: Effect Classes - WriteReversible.
#[derive(Clone, Debug)]
pub struct MockDatabaseTool;

impl MockDatabaseTool {
    /// Creates a tool binding for the database tool.
    ///
    /// # Returns
    ///
    /// A ToolBinding configured for write-reversible database operations.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let binding = MockDatabaseTool::create_binding(
    ///     AgentID::new("agent-1"),
    ///     CapID::from_bytes([42u8; 32]),
    /// );
    /// assert!(!binding.is_read_only());
    /// assert_eq!(binding.effect_class, EffectClass::WriteReversible);
    /// ```
    pub fn create_binding(agent: AgentID, capability: CapID) -> ToolBinding {
        let input_schema = SchemaDefinition::new("DatabaseInput");
        let output_schema = SchemaDefinition::new("DatabaseOutput");
        let schema = TypeSchema::new(input_schema, output_schema);

        let mut binding = ToolBinding::new(
            ToolBindingID::new("database-tool"),
            ToolID::new("database-api"),
            agent,
            capability,
            schema,
        );

        // WriteReversible effect class - mutations with undo support
        binding.effect_class = EffectClass::WriteReversible;

        // Restrictive sandbox - database access only
        binding.sandbox_config = SandboxConfig::restrictive();

        // No caching for mutable data
        binding.response_cache = CacheConfig::disabled();

        // Enable commit protocol with PREPARE/COMMIT for transactions
        binding.commit_protocol = Some(CommitProtocol::new(
            5000,  // Prepare timeout: 5 seconds
            10000, // Commit timeout: 10 seconds
            RollbackStrategy::Automatic,
        ));

        binding
    }

    /// Returns the tool ID for database operations.
    pub fn tool_id() -> ToolID {
        ToolID::new("database-api")
    }

    /// Returns the binding ID for database operations.
    pub fn binding_id() -> ToolBindingID {
        ToolBindingID::new("database-tool")
    }
}

/// Mock email tool - WRITE_IRREVERSIBLE effect class.
///
/// Represents a tool that sends immutable messages (emails) that cannot be undone
/// once delivered. This is the default effect class for undeclared tools.
///
/// Example: Email, SMS, notifications, irreversible database deletes.
///
/// # Effect Class: WriteIrreversible
///
/// - State mutations allowed
/// - Mutations cannot be undone or compensated
/// - User confirmation strongly recommended
/// - Default for undeclared tools (fail-safe)
///
/// # Configuration
///
/// - Sandbox: Network access for email delivery
/// - Cache: Disabled (messages shouldn't be cached)
/// - No commit protocol (no transactional semantics for emails)
/// - Requires confirmation
///
/// See Engineering Plan § 2.11.2: Effect Classes - WriteIrreversible.
#[derive(Clone, Debug)]
pub struct MockEmailTool;

impl MockEmailTool {
    /// Creates a tool binding for the email tool.
    ///
    /// # Returns
    ///
    /// A ToolBinding configured for write-irreversible email delivery.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let binding = MockEmailTool::create_binding(
    ///     AgentID::new("agent-1"),
    ///     CapID::from_bytes([42u8; 32]),
    /// );
    /// assert!(!binding.is_read_only());
    /// assert_eq!(binding.effect_class, EffectClass::WriteIrreversible);
    /// assert!(binding.requires_confirmation());
    /// ```
    pub fn create_binding(agent: AgentID, capability: CapID) -> ToolBinding {
        let input_schema = SchemaDefinition::new("EmailInput");
        let output_schema = SchemaDefinition::new("EmailOutput");
        let schema = TypeSchema::new(input_schema, output_schema);

        let mut binding = ToolBinding::new(
            ToolBindingID::new("email-tool"),
            ToolID::new("email-api"),
            agent,
            capability,
            schema,
        );

        // WriteIrreversible effect class - default for high-impact operations
        binding.effect_class = EffectClass::WriteIrreversible;

        // Network access for email delivery
        binding.sandbox_config = SandboxConfig::balanced();

        // No caching for email operations
        binding.response_cache = CacheConfig::disabled();

        // No commit protocol (emails are fire-and-forget)
        binding.commit_protocol = None;

        binding
    }

    /// Returns the tool ID for email operations.
    pub fn tool_id() -> ToolID {
        ToolID::new("email-api")
    }

    /// Returns the binding ID for email operations.
    pub fn binding_id() -> ToolBindingID {
        ToolBindingID::new("email-tool")
    }
}

/// Factory for creating and managing mock tool bindings.
///
/// Provides a convenient interface for creating all three mock tools
/// with consistent configurations.
#[derive(Clone, Debug)]
pub struct MockToolFactory;

impl MockToolFactory {
    /// Creates all three mock tool bindings for a given agent.
    ///
    /// # Arguments
    ///
    /// - `agent`: The agent that will hold all three bindings
    ///
    /// # Returns
    ///
    /// A vector containing three configured tool bindings:
    /// 1. MockWebSearchTool (ReadOnly)
    /// 2. MockDatabaseTool (WriteReversible)
    /// 3. MockEmailTool (WriteIrreversible)
    ///
    /// # Example
    ///
    /// ```ignore
    /// let bindings = MockToolFactory::create_all_bindings(
    ///     AgentID::new("agent-1"),
    /// );
    /// assert_eq!(bindings.len(), 3);
    /// assert_eq!(bindings[0].effect_class, EffectClass::ReadOnly);
    /// assert_eq!(bindings[1].effect_class, EffectClass::WriteReversible);
    /// assert_eq!(bindings[2].effect_class, EffectClass::WriteIrreversible);
    /// ```
    pub fn create_all_bindings(agent: AgentID) -> Vec<ToolBinding> {
        vec![
            MockWebSearchTool::create_binding(agent.clone(), Self::capability_for_tool(0)),
            MockDatabaseTool::create_binding(agent.clone(), Self::capability_for_tool(1)),
            MockEmailTool::create_binding(agent, Self::capability_for_tool(2)),
        ]
    }

    /// Generates a deterministic capability for a mock tool index.
    ///
    /// Used internally to create distinct capabilities for each mock tool.
    fn capability_for_tool(index: u8) -> CapID {
        let mut bytes = [42u8; 32];
        bytes[0] = index;
        CapID::from_bytes(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::vec;

    fn test_agent() -> AgentID {
        AgentID::new("test-agent")
    }

    fn test_capability() -> CapID {
        CapID::from_bytes([42u8; 32])
    }

    #[test]
    fn test_mock_web_search_tool_creation() {
        let binding = MockWebSearchTool::create_binding(test_agent(), test_capability());

        assert_eq!(binding.id.as_str(), "web-search");
        assert_eq!(binding.tool.as_str(), "web-search-api");
        assert_eq!(binding.effect_class, EffectClass::ReadOnly);
        assert!(binding.is_read_only());
        assert!(!binding.requires_confirmation());
    }

    #[test]
    fn test_mock_web_search_tool_caching() {
        let binding = MockWebSearchTool::create_binding(test_agent(), test_capability());

        assert!(binding.response_cache.enabled);
        assert_eq!(binding.response_cache.ttl_ms, 86400000); // 24 hours
    }

    #[test]
    fn test_mock_web_search_tool_ids() {
        assert_eq!(MockWebSearchTool::tool_id().as_str(), "web-search-api");
        assert_eq!(MockWebSearchTool::binding_id().as_str(), "web-search");
    }

    #[test]
    fn test_mock_database_tool_creation() {
        let binding = MockDatabaseTool::create_binding(test_agent(), test_capability());

        assert_eq!(binding.id.as_str(), "database-tool");
        assert_eq!(binding.tool.as_str(), "database-api");
        assert_eq!(binding.effect_class, EffectClass::WriteReversible);
        assert!(!binding.is_read_only());
        assert!(!binding.requires_confirmation());
    }

    #[test]
    fn test_mock_database_tool_transaction_support() {
        let binding = MockDatabaseTool::create_binding(test_agent(), test_capability());

        assert!(binding.commit_protocol.is_some());
        let protocol = binding.commit_protocol.unwrap();
        assert_eq!(protocol.prepare_timeout_ms, 5000);
        assert_eq!(protocol.commit_timeout_ms, 10000);
    }

    #[test]
    fn test_mock_database_tool_no_cache() {
        let binding = MockDatabaseTool::create_binding(test_agent(), test_capability());

        assert!(!binding.response_cache.enabled);
    }

    #[test]
    fn test_mock_database_tool_ids() {
        assert_eq!(MockDatabaseTool::tool_id().as_str(), "database-api");
        assert_eq!(MockDatabaseTool::binding_id().as_str(), "database-tool");
    }

    #[test]
    fn test_mock_email_tool_creation() {
        let binding = MockEmailTool::create_binding(test_agent(), test_capability());

        assert_eq!(binding.id.as_str(), "email-tool");
        assert_eq!(binding.tool.as_str(), "email-api");
        assert_eq!(binding.effect_class, EffectClass::WriteIrreversible);
        assert!(!binding.is_read_only());
        assert!(binding.requires_confirmation());
    }

    #[test]
    fn test_mock_email_tool_no_commit_protocol() {
        let binding = MockEmailTool::create_binding(test_agent(), test_capability());

        assert!(binding.commit_protocol.is_none());
    }

    #[test]
    fn test_mock_email_tool_ids() {
        assert_eq!(MockEmailTool::tool_id().as_str(), "email-api");
        assert_eq!(MockEmailTool::binding_id().as_str(), "email-tool");
    }

    #[test]
    fn test_mock_tool_factory_create_all() {
        let bindings = MockToolFactory::create_all_bindings(test_agent());

        assert_eq!(bindings.len(), 3);

        // First: ReadOnly
        assert_eq!(bindings[0].effect_class, EffectClass::ReadOnly);
        assert_eq!(bindings[0].id.as_str(), "web-search");

        // Second: WriteReversible
        assert_eq!(bindings[1].effect_class, EffectClass::WriteReversible);
        assert_eq!(bindings[1].id.as_str(), "database-tool");

        // Third: WriteIrreversible
        assert_eq!(bindings[2].effect_class, EffectClass::WriteIrreversible);
        assert_eq!(bindings[2].id.as_str(), "email-tool");
    }

    #[test]
    fn test_mock_tool_factory_distinct_capabilities() {
        let agent = test_agent();
        let bindings = MockToolFactory::create_all_bindings(agent);

        // Each tool should have a different capability
        assert_ne!(bindings[0].capability, bindings[1].capability);
        assert_ne!(bindings[1].capability, bindings[2].capability);
        assert_ne!(bindings[0].capability, bindings[2].capability);
    }

    #[test]
    fn test_mock_tool_factory_all_valid() {
        let bindings = MockToolFactory::create_all_bindings(test_agent());

        for binding in bindings {
            assert!(binding.validate().is_ok());
        }
    }

    #[test]
    fn test_mock_tools_effect_class_summary() {
        let web_search = MockWebSearchTool::create_binding(test_agent(), test_capability());
        let database = MockDatabaseTool::create_binding(test_agent(), test_capability());
        let email = MockEmailTool::create_binding(test_agent(), test_capability());

        // ReadOnly is safe, no confirmation needed
        assert!(web_search.effect_class.is_safe());
        assert!(!web_search.effect_class.requires_confirmation());

        // WriteReversible is not safe but no confirmation needed
        assert!(!database.effect_class.is_safe());
        assert!(!database.effect_class.requires_confirmation());
        assert!(database.effect_class.is_reversible());

        // WriteIrreversible is not safe, confirmation needed
        assert!(!email.effect_class.is_safe());
        assert!(email.effect_class.requires_confirmation());
        assert!(!email.effect_class.is_reversible());
    }

    #[test]
    fn test_mock_tools_all_validated() {
        let web_search = MockWebSearchTool::create_binding(test_agent(), test_capability());
        let database = MockDatabaseTool::create_binding(test_agent(), test_capability());
        let email = MockEmailTool::create_binding(test_agent(), test_capability());

        assert!(web_search.validate().is_ok());
        assert!(database.validate().is_ok());
        assert!(email.validate().is_ok());
    }
}
