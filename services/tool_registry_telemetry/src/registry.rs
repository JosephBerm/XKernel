// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Tool binding and MCP registry management with sandbox isolation.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

/// Tool binding descriptor for registered tools
#[derive(Debug, Clone)]
pub struct ToolBinding {
    pub tool_id: u64,
    pub tool_name: String,
    pub capability_hash: u64,
    pub sandbox_level: SandboxLevel,
    pub is_mcp_native: bool,
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SandboxLevel {
    /// No sandbox (system-level access)
    None,
    /// Process-level isolation
    Process,
    /// Container-level isolation
    Container,
    /// VM-level isolation
    Virtual,
}

impl fmt::Display for SandboxLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => write!(f, "None"),
            Self::Process => write!(f, "Process"),
            Self::Container => write!(f, "Container"),
            Self::Virtual => write!(f, "Virtual"),
        }
    }
}

impl ToolBinding {
    pub fn new(tool_id: u64, name: String, sandbox_level: SandboxLevel) -> Self {
        Self {
            tool_id,
            tool_name: name,
            capability_hash: 0,
            sandbox_level,
            is_mcp_native: false,
            enabled: true,
        }
    }

    pub fn with_mcp(mut self, is_mcp: bool) -> Self {
        self.is_mcp_native = is_mcp;
        self
    }

    pub fn with_hash(mut self, hash: u64) -> Self {
        self.capability_hash = hash;
        self
    }
}

/// MCP registry for model context protocol tools
#[derive(Debug, Clone)]
pub struct McpRegistry {
    bindings: Vec<ToolBinding>,
    max_tools: usize,
}

impl McpRegistry {
    pub fn new(max_tools: usize) -> Self {
        Self {
            bindings: Vec::new(),
            max_tools,
        }
    }

    /// Register a tool binding
    pub fn register(&mut self, binding: ToolBinding) -> Result<(), RegistryError> {
        if self.bindings.len() >= self.max_tools {
            return Err(RegistryError::RegistryFull {
                max: self.max_tools,
            });
        }

        // Check for duplicates
        if self.bindings.iter().any(|b| b.tool_id == binding.tool_id) {
            return Err(RegistryError::DuplicateToolId {
                tool_id: binding.tool_id,
            });
        }

        self.bindings.push(binding);
        Ok(())
    }

    /// Unregister a tool
    pub fn unregister(&mut self, tool_id: u64) -> bool {
        let old_len = self.bindings.len();
        self.bindings.retain(|b| b.tool_id != tool_id);
        self.bindings.len() < old_len
    }

    /// Get tool binding by ID
    pub fn get(&self, tool_id: u64) -> Option<&ToolBinding> {
        self.bindings.iter().find(|b| b.tool_id == tool_id)
    }

    /// Get tool binding by name
    pub fn get_by_name(&self, name: &str) -> Option<&ToolBinding> {
        self.bindings.iter().find(|b| b.tool_name == name)
    }

    /// List all enabled tools
    pub fn list_enabled(&self) -> Vec<&ToolBinding> {
        self.bindings.iter().filter(|b| b.enabled).collect()
    }

    /// Count tools by sandbox level
    pub fn count_by_sandbox(&self, level: SandboxLevel) -> usize {
        self.bindings.iter().filter(|b| b.sandbox_level == level).count()
    }

    pub fn total_tools(&self) -> usize {
        self.bindings.len()
    }
}

impl Default for McpRegistry {
    fn default() -> Self {
        Self::new(1000)
    }
}

/// Sandbox policy evaluator
#[derive(Debug, Clone)]
pub struct SandboxPolicy {
    /// Allowed sandbox levels for this execution context
    allowed_levels: Vec<SandboxLevel>,
    /// Enforce minimum sandbox level
    minimum_level: SandboxLevel,
}

impl SandboxPolicy {
    pub fn new(minimum_level: SandboxLevel) -> Self {
        let allowed_levels = match minimum_level {
            SandboxLevel::None => alloc::vec![SandboxLevel::None],
            SandboxLevel::Process => alloc::vec![SandboxLevel::Process, SandboxLevel::Container, SandboxLevel::Virtual],
            SandboxLevel::Container => alloc::vec![SandboxLevel::Container, SandboxLevel::Virtual],
            SandboxLevel::Virtual => alloc::vec![SandboxLevel::Virtual],
        };

        Self {
            allowed_levels,
            minimum_level,
        }
    }

    pub fn is_allowed(&self, level: SandboxLevel) -> bool {
        self.allowed_levels.contains(&level)
    }

    pub fn meets_minimum(&self, level: SandboxLevel) -> bool {
        level as u8 >= self.minimum_level as u8
    }
}

impl Default for SandboxPolicy {
    fn default() -> Self {
        Self::new(SandboxLevel::Process)
    }
}

/// Registry error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryError {
    RegistryFull { max: usize },
    DuplicateToolId { tool_id: u64 },
    ToolNotFound,
    SandboxViolation,
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RegistryFull { max } => write!(f, "registry full (max: {})", max),
            Self::DuplicateToolId { tool_id } => write!(f, "duplicate tool ID: {}", tool_id),
            Self::ToolNotFound => write!(f, "tool not found"),
            Self::SandboxViolation => write!(f, "sandbox violation"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_binding() {
        let binding = ToolBinding::new(1, String::from("test_tool"), SandboxLevel::Process)
            .with_mcp(true)
            .with_hash(12345);

        assert_eq!(binding.tool_id, 1);
        assert!(binding.is_mcp_native);
        assert_eq!(binding.capability_hash, 12345);
    }

    #[test]
    fn test_mcp_registry() {
        let mut registry = McpRegistry::new(10);

        let binding1 = ToolBinding::new(1, String::from("tool1"), SandboxLevel::Process);
        let binding2 = ToolBinding::new(2, String::from("tool2"), SandboxLevel::Container);

        assert!(registry.register(binding1).is_ok());
        assert!(registry.register(binding2).is_ok());

        assert_eq!(registry.total_tools(), 2);
        assert!(registry.get(1).is_some());
    }

    #[test]
    fn test_registry_full() {
        let mut registry = McpRegistry::new(2);

        registry.register(ToolBinding::new(1, String::from("t1"), SandboxLevel::Process)).unwrap();
        registry.register(ToolBinding::new(2, String::from("t2"), SandboxLevel::Process)).unwrap();

        assert!(matches!(
            registry.register(ToolBinding::new(3, String::from("t3"), SandboxLevel::Process)),
            Err(RegistryError::RegistryFull { .. })
        ));
    }

    #[test]
    fn test_duplicate_tool_id() {
        let mut registry = McpRegistry::new(10);

        registry.register(ToolBinding::new(1, String::from("t1"), SandboxLevel::Process)).unwrap();

        assert!(matches!(
            registry.register(ToolBinding::new(1, String::from("t2"), SandboxLevel::Process)),
            Err(RegistryError::DuplicateToolId { .. })
        ));
    }

    #[test]
    fn test_sandbox_policy() {
        let policy = SandboxPolicy::new(SandboxLevel::Container);

        assert!(policy.is_allowed(SandboxLevel::Container));
        assert!(policy.is_allowed(SandboxLevel::Virtual));
        assert!(!policy.is_allowed(SandboxLevel::Process));
        assert!(!policy.is_allowed(SandboxLevel::None));
    }

    #[test]
    fn test_list_by_sandbox() {
        let mut registry = McpRegistry::new(10);

        registry.register(ToolBinding::new(1, String::from("t1"), SandboxLevel::Process)).unwrap();
        registry.register(ToolBinding::new(2, String::from("t2"), SandboxLevel::Process)).unwrap();
        registry.register(ToolBinding::new(3, String::from("t3"), SandboxLevel::Container)).unwrap();

        assert_eq!(registry.count_by_sandbox(SandboxLevel::Process), 2);
        assert_eq!(registry.count_by_sandbox(SandboxLevel::Container), 1);
    }
}
