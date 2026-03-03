// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Stub Tool Registry with mock registration, effect class enforcement, and default handling.
//!
//! This module implements the foundational tool registry that manages tool bindings,
//! effect class validation, and default effect class assignment. This is a stub
//! implementation that will be replaced with MCP-native implementation in Phase 1.
//!
//! See Engineering Plan § 2.11: ToolBinding Entity & Tool Registry.
//! See Engineering Plan § 2.11.2: Effect Classes.

use crate::effect_class::EffectClass;
use crate::error::{Result, ToolError};
use crate::ids::ToolID;
use crate::tool_binding::ToolBinding;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

/// Registry error types specific to registry operations.
///
/// See Engineering Plan § 2.11: Tool Registry Error Handling.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegistryError {
    /// Tool binding already exists.
    BindingExists {
        /// ID of the binding that already exists
        binding_id: String,
    },

    /// Tool binding not found.
    BindingNotFound {
        /// ID of the binding that was not found
        binding_id: String,
    },

    /// Invalid tool binding configuration.
    InvalidBinding {
        /// Reason for invalidity
        reason: String,
    },

    /// Registration failed due to validation error.
    RegistrationFailed {
        /// Reason for registration failure
        reason: String,
    },
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RegistryError::BindingExists { binding_id } => {
                write!(f, "binding already exists: {}", binding_id)
            }
            RegistryError::BindingNotFound { binding_id } => {
                write!(f, "binding not found: {}", binding_id)
            }
            RegistryError::InvalidBinding { reason } => {
                write!(f, "invalid binding: {}", reason)
            }
            RegistryError::RegistrationFailed { reason } => {
                write!(f, "registration failed: {}", reason)
            }
        }
    }
}

/// Stub Tool Registry with in-memory store for tool bindings.
///
/// Manages registration, lookup, and listing of tool bindings with effect class validation.
/// Implements default behavior: undeclared tools get WRITE_IRREVERSIBLE effect class.
///
/// # Architecture
///
/// The registry uses a BTreeMap to store bindings by ID, enabling:
/// - O(log n) lookup by binding ID
/// - Sorted iteration for list operations
/// - No_std compatible (no Arc/RwLock required for stub)
///
/// # Effect Class Defaults
///
/// Per Engineering Plan § 2.11.2, undeclared tools default to WriteIrreversible
/// to fail-safe (assume worst case). This default is applied during registration
/// if no effect_class is provided.
///
/// # Future (Phase 1)
///
/// This stub will be replaced with:
/// - MCP-native tool discovery and registration
/// - Dynamic tool loading from plugin system
/// - Persistent storage and clustering support
///
/// See Engineering Plan § 2.11: ToolBinding Entity & Tool Registry.
#[derive(Clone, Debug)]
pub struct ToolRegistry {
    /// In-memory store of bindings: binding_id -> ToolBinding
    bindings: BTreeMap<String, ToolBinding>,
}

impl ToolRegistry {
    /// Creates a new empty tool registry.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let registry = ToolRegistry::new();
    /// assert_eq!(registry.binding_count(), 0);
    /// ```
    pub fn new() -> Self {
        ToolRegistry {
            bindings: BTreeMap::new(),
        }
    }

    /// Registers a tool binding in the registry.
    ///
    /// # Validation
    ///
    /// Performs the following validations:
    /// 1. Binding ID is not already registered
    /// 2. Binding fields are valid (via binding.validate())
    /// 3. Effect class is properly configured
    ///
    /// # Default Effect Class
    ///
    /// If the binding has no effect_class set (defaults to WriteIrreversible),
    /// the registration is still accepted. However, if effect_class is explicitly
    /// None in future versions, it will be defaulted and logged as an audit event.
    ///
    /// # Arguments
    ///
    /// - `binding`: Tool binding to register
    ///
    /// # Returns
    ///
    /// - `Ok(binding_id)`: Successfully registered, returns the binding ID
    /// - `Err(RegistryError::BindingExists)`: Binding ID already exists
    /// - `Err(RegistryError::InvalidBinding)`: Binding validation failed
    /// - `Err(RegistryError::RegistrationFailed)`: Other registration failure
    ///
    /// # Example
    ///
    /// ```ignore
    /// let binding = ToolBinding::new(
    ///     ToolBindingID::new("web-search"),
    ///     ToolID::new("web-search-api"),
    ///     AgentID::new("agent-1"),
    ///     CapID::from_bytes([42u8; 32]),
    ///     TypeSchema::new(...),
    /// ).with_effect_class(EffectClass::ReadOnly);
    ///
    /// let binding_id = registry.register_tool(binding)?;
    /// assert_eq!(binding_id, "web-search");
    /// ```
    ///
    /// See Engineering Plan § 2.11: Tool Registry - Registration.
    pub fn register_tool(&mut self, binding: ToolBinding) -> core::result::Result<String, RegistryError> {
        let binding_id = binding.id.as_str().to_string();

        // Check if binding already exists
        if self.bindings.contains_key(&binding_id) {
            return Err(RegistryError::BindingExists {
                binding_id: binding_id.clone(),
            });
        }

        // Validate the binding
        if let Err(e) = binding.validate() {
            return Err(RegistryError::InvalidBinding {
                reason: e.to_string(),
            });
        }

        // Apply default effect class if needed
        // Currently all bindings have explicit effect class, but log if WriteIrreversible
        if binding.effect_class == EffectClass::WriteIrreversible {
            // In future: log audit event "default effect class applied"
            // For now, this is expected behavior for undeclared/unsafe tools
        }

        // Store in registry
        self.bindings.insert(binding_id.clone(), binding);

        Ok(binding_id)
    }

    /// Looks up a tool binding by ID.
    ///
    /// # Arguments
    ///
    /// - `binding_id`: The binding ID to look up
    ///
    /// # Returns
    ///
    /// - `Some(binding)`: Binding found
    /// - `None`: Binding not found
    ///
    /// # Example
    ///
    /// ```ignore
    /// let binding = registry.get_binding("web-search")?;
    /// assert_eq!(binding.effect_class, EffectClass::ReadOnly);
    /// ```
    ///
    /// See Engineering Plan § 2.11: Tool Registry - Lookup.
    pub fn get_binding(&self, binding_id: &str) -> Option<ToolBinding> {
        self.bindings.get(binding_id).cloned()
    }

    /// Lists all bindings with a specific effect class.
    ///
    /// # Arguments
    ///
    /// - `effect_class`: The effect class to filter by
    ///
    /// # Returns
    ///
    /// A vector of all bindings with the specified effect class, sorted by binding ID.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let readonly_bindings = registry.list_by_effect_class(EffectClass::ReadOnly);
    /// for binding in readonly_bindings {
    ///     assert!(binding.is_read_only());
    /// }
    /// ```
    ///
    /// See Engineering Plan § 2.11: Tool Registry - Introspection.
    pub fn list_by_effect_class(&self, effect_class: EffectClass) -> Vec<ToolBinding> {
        self.bindings
            .values()
            .filter(|b| b.effect_class == effect_class)
            .cloned()
            .collect()
    }

    /// Lists all bindings for a specific tool.
    ///
    /// Multiple bindings can exist for the same tool (different agents/contexts).
    ///
    /// # Arguments
    ///
    /// - `tool_id`: The tool ID to filter by
    ///
    /// # Returns
    ///
    /// A vector of all bindings for the specified tool, sorted by binding ID.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let web_search_bindings = registry.list_by_tool(ToolID::new("web-search-api"));
    /// assert!(web_search_bindings.len() >= 1);
    /// ```
    ///
    /// See Engineering Plan § 2.11: Tool Registry - Introspection.
    pub fn list_by_tool(&self, tool_id: &ToolID) -> Vec<ToolBinding> {
        self.bindings
            .values()
            .filter(|b| b.tool == *tool_id)
            .cloned()
            .collect()
    }

    /// Lists all tool bindings in the registry.
    ///
    /// Returns bindings in sorted order by binding ID.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let all_bindings = registry.list_all();
    /// println!("Registry contains {} bindings", all_bindings.len());
    /// ```
    ///
    /// See Engineering Plan § 2.11: Tool Registry - Introspection.
    pub fn list_all(&self) -> Vec<ToolBinding> {
        self.bindings.values().cloned().collect()
    }

    /// Returns the number of bindings in the registry.
    ///
    /// # Example
    ///
    /// ```ignore
    /// assert_eq!(registry.binding_count(), 3);
    /// ```
    pub fn binding_count(&self) -> usize {
        self.bindings.len()
    }

    /// Unregisters a tool binding from the registry.
    ///
    /// # Arguments
    ///
    /// - `binding_id`: The binding ID to unregister
    ///
    /// # Returns
    ///
    /// - `Ok(binding)`: Successfully unregistered, returns the binding
    /// - `Err(RegistryError::BindingNotFound)`: Binding does not exist
    ///
    /// # Example
    ///
    /// ```ignore
    /// let binding = registry.unregister_tool("web-search")?;
    /// assert_eq!(registry.binding_count(), 2);
    /// ```
    pub fn unregister_tool(&mut self, binding_id: &str) -> core::result::Result<ToolBinding, RegistryError> {
        self.bindings.remove(binding_id).ok_or(RegistryError::BindingNotFound {
            binding_id: binding_id.to_string(),
        })
    }

    /// Clears all bindings from the registry.
    ///
    /// Used for testing and cleanup.
    pub fn clear(&mut self) {
        self.bindings.clear();
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::CacheConfig;
    use crate::ids::{AgentID, CapID, ToolBindingID};
    use crate::schema::SchemaDefinition;
    use crate::tool_binding::ToolBinding;
    use crate::schema::TypeSchema;
use alloc::format;
use alloc::string::ToString;

    fn create_test_binding(id: &str) -> ToolBinding {
        let input_schema = SchemaDefinition::new("TestInput");
        let output_schema = SchemaDefinition::new("TestOutput");
        let schema = TypeSchema::new(input_schema, output_schema);

        let capability_bytes = [42u8; 32];

        ToolBinding::new(
            ToolBindingID::new(id),
            ToolID::new("test-tool"),
            AgentID::new("agent-1"),
            CapID::from_bytes(capability_bytes),
            schema,
        )
    }

    #[test]
    fn test_registry_new() {
        let registry = ToolRegistry::new();
        assert_eq!(registry.binding_count(), 0);
    }

    #[test]
    fn test_registry_register_tool() {
        let mut registry = ToolRegistry::new();
        let binding = create_test_binding("tool-1");

        let result = registry.register_tool(binding);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "tool-1");
        assert_eq!(registry.binding_count(), 1);
    }

    #[test]
    fn test_registry_register_duplicate() {
        let mut registry = ToolRegistry::new();
        let binding1 = create_test_binding("tool-1");
        let binding2 = create_test_binding("tool-1");

        assert!(registry.register_tool(binding1).is_ok());
        let result = registry.register_tool(binding2);
        assert!(result.is_err());
        match result {
            Err(RegistryError::BindingExists { binding_id }) => {
                assert_eq!(binding_id, "tool-1");
            }
            _ => panic!("expected BindingExists error"),
        }
    }

    #[test]
    fn test_registry_get_binding() {
        let mut registry = ToolRegistry::new();
        let binding = create_test_binding("tool-1");
        let binding_clone = binding.clone();

        registry.register_tool(binding).unwrap();

        let retrieved = registry.get_binding("tool-1");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap(), binding_clone);
    }

    #[test]
    fn test_registry_get_binding_not_found() {
        let registry = ToolRegistry::new();
        let retrieved = registry.get_binding("nonexistent");
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_registry_list_by_effect_class() {
        let mut registry = ToolRegistry::new();

        let mut readonly_binding = create_test_binding("readonly-tool");
        readonly_binding.effect_class = EffectClass::ReadOnly;
        registry.register_tool(readonly_binding).unwrap();

        let mut write_binding = create_test_binding("write-tool");
        write_binding.effect_class = EffectClass::WriteReversible;
        registry.register_tool(write_binding).unwrap();

        let readonly_list = registry.list_by_effect_class(EffectClass::ReadOnly);
        assert_eq!(readonly_list.len(), 1);
        assert_eq!(readonly_list[0].id.as_str(), "readonly-tool");

        let write_list = registry.list_by_effect_class(EffectClass::WriteReversible);
        assert_eq!(write_list.len(), 1);
        assert_eq!(write_list[0].id.as_str(), "write-tool");
    }

    #[test]
    fn test_registry_list_by_tool() {
        let mut registry = ToolRegistry::new();

        let mut binding1 = create_test_binding("binding-1");
        binding1.tool = ToolID::new("tool-a");
        registry.register_tool(binding1).unwrap();

        let mut binding2 = create_test_binding("binding-2");
        binding2.tool = ToolID::new("tool-a");
        registry.register_tool(binding2).unwrap();

        let mut binding3 = create_test_binding("binding-3");
        binding3.tool = ToolID::new("tool-b");
        registry.register_tool(binding3).unwrap();

        let tool_a_bindings = registry.list_by_tool(&ToolID::new("tool-a"));
        assert_eq!(tool_a_bindings.len(), 2);

        let tool_b_bindings = registry.list_by_tool(&ToolID::new("tool-b"));
        assert_eq!(tool_b_bindings.len(), 1);
    }

    #[test]
    fn test_registry_list_all() {
        let mut registry = ToolRegistry::new();

        for i in 0..3 {
            let binding = create_test_binding(&format!("tool-{}", i));
            registry.register_tool(binding).unwrap();
        }

        let all = registry.list_all();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_registry_unregister_tool() {
        let mut registry = ToolRegistry::new();
        let binding = create_test_binding("tool-1");

        registry.register_tool(binding.clone()).unwrap();
        assert_eq!(registry.binding_count(), 1);

        let unregistered = registry.unregister_tool("tool-1");
        assert!(unregistered.is_ok());
        assert_eq!(unregistered.unwrap(), binding);
        assert_eq!(registry.binding_count(), 0);
    }

    #[test]
    fn test_registry_unregister_not_found() {
        let mut registry = ToolRegistry::new();
        let result = registry.unregister_tool("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_registry_clear() {
        let mut registry = ToolRegistry::new();

        for i in 0..3 {
            let binding = create_test_binding(&format!("tool-{}", i));
            registry.register_tool(binding).unwrap();
        }

        assert_eq!(registry.binding_count(), 3);
        registry.clear();
        assert_eq!(registry.binding_count(), 0);
    }

    #[test]
    fn test_registry_default_effect_class() {
        let mut registry = ToolRegistry::new();
        let binding = create_test_binding("tool-1");

        assert_eq!(binding.effect_class, EffectClass::WriteIrreversible);

        registry.register_tool(binding).unwrap();
        let retrieved = registry.get_binding("tool-1").unwrap();
        assert_eq!(retrieved.effect_class, EffectClass::WriteIrreversible);
    }

    #[test]
    fn test_registry_multiple_registrations() {
        let mut registry = ToolRegistry::new();

        let mut readonly = create_test_binding("readonly");
        readonly.effect_class = EffectClass::ReadOnly;

        let mut reversible = create_test_binding("reversible");
        reversible.effect_class = EffectClass::WriteReversible;

        let mut compensable = create_test_binding("compensable");
        compensable.effect_class = EffectClass::WriteCompensable;

        let irreversible = create_test_binding("irreversible");

        registry.register_tool(readonly).unwrap();
        registry.register_tool(reversible).unwrap();
        registry.register_tool(compensable).unwrap();
        registry.register_tool(irreversible).unwrap();

        assert_eq!(registry.binding_count(), 4);
        assert_eq!(registry.list_by_effect_class(EffectClass::ReadOnly).len(), 1);
        assert_eq!(registry.list_by_effect_class(EffectClass::WriteReversible).len(), 1);
        assert_eq!(registry.list_by_effect_class(EffectClass::WriteCompensable).len(), 1);
        assert_eq!(registry.list_by_effect_class(EffectClass::WriteIrreversible).len(), 1);
    }

    #[test]
    fn test_registry_default() {
        let registry = ToolRegistry::default();
        assert_eq!(registry.binding_count(), 0);
    }
}
