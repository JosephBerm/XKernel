// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Sandbox configuration for tool invocation security.
//!
//! Defines per-tool security constraints including network access,
//! filesystem restrictions, execution timeouts, and allowed syscalls.
//!
//! See Engineering Plan § 2.11.4: Sandbox Configuration.

use alloc::collections::BTreeSet;
use alloc::string::String;
use core::fmt;

/// Network access policy for sandboxed tool execution.
///
/// See Engineering Plan § 2.11.4: Sandbox Configuration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NetworkPolicy {
    /// No network access allowed.
    ///
    /// Tool cannot make any network requests.
    /// Suitable for untrusted tools or sensitive operations.
    None,

    /// Allowlist-based network access.
    ///
    /// Tool can only connect to explicitly allowed domains/IPs.
    /// Connections to any other destination are blocked.
    AllowList(BTreeSet<String>),

    /// Denylist-based network access.
    ///
    /// Tool can connect to any destination except those in the denylist.
    /// Suitable for allowing most access while blocking specific services.
    DenyList(BTreeSet<String>),
}

impl NetworkPolicy {
    /// Returns true if this policy allows network access.
    pub fn allows_network(&self) -> bool {
        !matches!(self, NetworkPolicy::None)
    }

    /// Returns true if a destination is allowed under this policy.
    pub fn is_allowed(&self, destination: &str) -> bool {
        match self {
            NetworkPolicy::None => false,
            NetworkPolicy::AllowList(allowed) => allowed.contains(destination),
            NetworkPolicy::DenyList(denied) => !denied.contains(destination),
        }
    }
}

impl fmt::Display for NetworkPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetworkPolicy::None => write!(f, "None"),
            NetworkPolicy::AllowList(_) => write!(f, "AllowList"),
            NetworkPolicy::DenyList(_) => write!(f, "DenyList"),
        }
    }
}

/// Filesystem access policy for sandboxed tool execution.
///
/// See Engineering Plan § 2.11.4: Sandbox Configuration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FsPolicy {
    /// No filesystem access allowed.
    ///
    /// Tool cannot read or write any files.
    None,

    /// Allowlist-based filesystem access.
    ///
    /// Tool can only access explicitly allowed paths.
    /// Accesses outside allowlist are blocked.
    AllowList(BTreeSet<String>),

    /// Denylist-based filesystem access.
    ///
    /// Tool can access any file except those matching denylist patterns.
    /// Suitable for allowing broad access while protecting sensitive paths.
    DenyList(BTreeSet<String>),

    /// Read-only filesystem access.
    ///
    /// Tool can read files but cannot write or delete.
    /// All filesystem mutations are blocked.
    ReadOnly,
}

impl FsPolicy {
    /// Returns true if this policy allows any filesystem access.
    pub fn allows_filesystem(&self) -> bool {
        !matches!(self, FsPolicy::None)
    }

    /// Returns true if this policy allows write access.
    pub fn allows_write(&self) -> bool {
        !matches!(self, FsPolicy::None | FsPolicy::ReadOnly)
    }

    /// Returns true if a path is allowed under this policy.
    pub fn is_allowed(&self, path: &str) -> bool {
        match self {
            FsPolicy::None => false,
            FsPolicy::AllowList(allowed) => allowed.contains(path),
            FsPolicy::DenyList(denied) => !denied.contains(path),
            FsPolicy::ReadOnly => true,
        }
    }
}

impl fmt::Display for FsPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FsPolicy::None => write!(f, "None"),
            FsPolicy::AllowList(_) => write!(f, "AllowList"),
            FsPolicy::DenyList(_) => write!(f, "DenyList"),
            FsPolicy::ReadOnly => write!(f, "ReadOnly"),
        }
    }
}

/// Sandbox configuration for a tool binding.
///
/// Defines security constraints for tool invocation including resource limits,
/// network/filesystem restrictions, and allowed syscalls.
///
/// See Engineering Plan § 2.11.4: Sandbox Configuration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SandboxConfig {
    /// Network access policy for this tool.
    pub network_access: NetworkPolicy,

    /// Filesystem access policy for this tool.
    pub filesystem_access: FsPolicy,

    /// Maximum execution time in milliseconds.
    ///
    /// Tool invocation is terminated if it exceeds this duration.
    /// If 0, no timeout is enforced (unbounded execution).
    pub max_execution_time_ms: u64,

    /// Maximum memory usage in bytes.
    ///
    /// Tool invocation is terminated if it exceeds this memory limit.
    /// If 0, no memory limit is enforced.
    pub max_memory_bytes: u64,

    /// Set of allowed CSCI syscall names.
    ///
    /// Tool can only invoke syscalls in this set.
    /// Empty set means no syscalls allowed (pure computation only).
    pub allowed_syscalls: BTreeSet<String>,
}

impl SandboxConfig {
    /// Creates a restrictive default sandbox (most secure).
    ///
    /// - No network access
    /// - No filesystem access
    /// - Execution timeout: 5 seconds
    /// - Memory limit: 128 MiB
    /// - No syscalls allowed
    pub fn restrictive() -> Self {
        SandboxConfig {
            network_access: NetworkPolicy::None,
            filesystem_access: FsPolicy::None,
            max_execution_time_ms: 5000,
            max_memory_bytes: 128 * 1024 * 1024,
            allowed_syscalls: BTreeSet::new(),
        }
    }

    /// Creates a permissive default sandbox (least secure).
    ///
    /// - All network access allowed
    /// - All filesystem access allowed
    /// - Execution timeout: 30 seconds
    /// - Memory limit: 1 GiB
    /// - All common syscalls allowed
    pub fn permissive() -> Self {
        let mut syscalls = BTreeSet::new();
        syscalls.insert("read".to_string());
        syscalls.insert("write".to_string());
        syscalls.insert("open".to_string());
        syscalls.insert("close".to_string());
        syscalls.insert("mmap".to_string());
        syscalls.insert("munmap".to_string());

        SandboxConfig {
            network_access: NetworkPolicy::DenyList(BTreeSet::new()),
            filesystem_access: FsPolicy::ReadOnly,
            max_execution_time_ms: 30000,
            max_memory_bytes: 1024 * 1024 * 1024,
            allowed_syscalls: syscalls,
        }
    }

    /// Creates a balanced default sandbox.
    ///
    /// - Limited network access (no external APIs)
    /// - Read-only filesystem access
    /// - Execution timeout: 10 seconds
    /// - Memory limit: 256 MiB
    /// - Basic read/write syscalls allowed
    pub fn balanced() -> Self {
        let mut syscalls = BTreeSet::new();
        syscalls.insert("read".to_string());
        syscalls.insert("write".to_string());

        SandboxConfig {
            network_access: NetworkPolicy::None,
            filesystem_access: FsPolicy::ReadOnly,
            max_execution_time_ms: 10000,
            max_memory_bytes: 256 * 1024 * 1024,
            allowed_syscalls: syscalls,
        }
    }

    /// Returns true if this sandbox configuration is permissive overall.
    pub fn is_permissive(&self) -> bool {
        self.network_access.allows_network()
            && self.filesystem_access.allows_write()
            && (self.max_execution_time_ms == 0 || self.max_execution_time_ms >= 30000)
            && (self.max_memory_bytes == 0 || self.max_memory_bytes >= 1024 * 1024 * 1024)
    }

    /// Returns true if this sandbox configuration is restrictive overall.
    pub fn is_restrictive(&self) -> bool {
        !self.network_access.allows_network()
            && !self.filesystem_access.allows_write()
            && self.max_execution_time_ms > 0
            && self.max_execution_time_ms <= 5000
            && self.max_memory_bytes > 0
            && self.max_memory_bytes <= 128 * 1024 * 1024
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::string::ToString;

    #[test]
    fn test_network_policy_none() {
        let policy = NetworkPolicy::None;
        assert!(!policy.allows_network());
        assert!(!policy.is_allowed("example.com"));
        assert!(!policy.is_allowed("127.0.0.1"));
    }

    #[test]
    fn test_network_policy_allowlist() {
        let mut allowed = BTreeSet::new();
        allowed.insert("api.example.com".to_string());
        let policy = NetworkPolicy::AllowList(allowed);

        assert!(policy.allows_network());
        assert!(policy.is_allowed("api.example.com"));
        assert!(!policy.is_allowed("other.com"));
    }

    #[test]
    fn test_network_policy_denylist() {
        let mut denied = BTreeSet::new();
        denied.insert("malicious.com".to_string());
        let policy = NetworkPolicy::DenyList(denied);

        assert!(policy.allows_network());
        assert!(policy.is_allowed("safe.com"));
        assert!(!policy.is_allowed("malicious.com"));
    }

    #[test]
    fn test_fs_policy_none() {
        let policy = FsPolicy::None;
        assert!(!policy.allows_filesystem());
        assert!(!policy.allows_write());
        assert!(!policy.is_allowed("/etc/passwd"));
    }

    #[test]
    fn test_fs_policy_allowlist() {
        let mut allowed = BTreeSet::new();
        allowed.insert("/tmp".to_string());
        let policy = FsPolicy::AllowList(allowed);

        assert!(policy.allows_filesystem());
        assert!(policy.allows_write());
        assert!(policy.is_allowed("/tmp"));
        assert!(!policy.is_allowed("/etc"));
    }

    #[test]
    fn test_fs_policy_denylist() {
        let mut denied = BTreeSet::new();
        denied.insert("/etc".to_string());
        let policy = FsPolicy::DenyList(denied);

        assert!(policy.allows_filesystem());
        assert!(policy.allows_write());
        assert!(policy.is_allowed("/tmp"));
        assert!(!policy.is_allowed("/etc"));
    }

    #[test]
    fn test_fs_policy_readonly() {
        let policy = FsPolicy::ReadOnly;
        assert!(policy.allows_filesystem());
        assert!(!policy.allows_write());
        assert!(policy.is_allowed("/etc/passwd"));
        assert!(policy.is_allowed("/tmp"));
    }

    #[test]
    fn test_sandbox_config_restrictive() {
        let sb = SandboxConfig::restrictive();
        assert!(!sb.network_access.allows_network());
        assert!(!sb.filesystem_access.allows_write());
        assert_eq!(sb.max_execution_time_ms, 5000);
        assert_eq!(sb.max_memory_bytes, 128 * 1024 * 1024);
        assert!(sb.allowed_syscalls.is_empty());
        assert!(sb.is_restrictive());
    }

    #[test]
    fn test_sandbox_config_balanced() {
        let sb = SandboxConfig::balanced();
        assert!(!sb.network_access.allows_network());
        assert!(!sb.filesystem_access.allows_write());
        assert_eq!(sb.max_execution_time_ms, 10000);
        assert_eq!(sb.max_memory_bytes, 256 * 1024 * 1024);
        assert!(!sb.allowed_syscalls.is_empty());
    }

    #[test]
    fn test_sandbox_config_permissive() {
        let sb = SandboxConfig::permissive();
        assert!(sb.network_access.allows_network());
        assert!(sb.filesystem_access.allows_filesystem());
        assert_eq!(sb.max_execution_time_ms, 30000);
        assert_eq!(sb.max_memory_bytes, 1024 * 1024 * 1024);
        assert!(!sb.allowed_syscalls.is_empty());
        assert!(sb.is_permissive());
    }

    #[test]
    fn test_sandbox_config_equality() {
        let sb1 = SandboxConfig::restrictive();
        let sb2 = SandboxConfig::restrictive();
        assert_eq!(sb1, sb2);

        let sb3 = SandboxConfig::permissive();
        assert_ne!(sb1, sb3);
    }

    #[test]
    fn test_network_policy_display() {
        assert_eq!(NetworkPolicy::None.to_string(), "None");

        let mut allow = BTreeSet::new();
        allow.insert("example.com".to_string());
        assert_eq!(
            NetworkPolicy::AllowList(allow).to_string(),
            "AllowList"
        );

        let mut deny = BTreeSet::new();
        deny.insert("bad.com".to_string());
        assert_eq!(NetworkPolicy::DenyList(deny).to_string(), "DenyList");
    }

    #[test]
    fn test_fs_policy_display() {
        assert_eq!(FsPolicy::None.to_string(), "None");
        assert_eq!(FsPolicy::ReadOnly.to_string(), "ReadOnly");

        let mut allow = BTreeSet::new();
        allow.insert("/tmp".to_string());
        assert_eq!(FsPolicy::AllowList(allow).to_string(), "AllowList");

        let mut deny = BTreeSet::new();
        deny.insert("/etc".to_string());
        assert_eq!(FsPolicy::DenyList(deny).to_string(), "DenyList");
    }

    #[test]
    fn test_sandbox_permissive_threshold() {
        let mut config = SandboxConfig::permissive();
        assert!(config.is_permissive());

        // Reduce timeout and verify
        config.max_execution_time_ms = 10000;
        assert!(!config.is_permissive());
    }
}
