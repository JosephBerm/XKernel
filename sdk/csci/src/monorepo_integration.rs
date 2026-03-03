//! # Monorepo Integration Module
//!
//! Provides comprehensive workspace configuration validation, version synchronization,
//! cross-project dependency resolution, and build order management for the Cognitive Substrate OS SDK.
//!
//! ## Design Goals
//! - Enforce consistent versioning across CSCI (v0.1) and SDKs (v0.1.0)
//! - Validate workspace integrity and dependency graphs
//! - Manage build order to prevent circular dependencies
//! - Support polyglot environments (Rust, TypeScript, C#)

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap as HashMap;
use std::collections::BTreeSet as HashSet;
use std::collections::VecDeque;
use core::fmt;

/// Strongly-typed version identifier for workspace packages
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct WorkspaceVersion {
    major: u32,
    minor: u32,
    patch: u32,
}

impl WorkspaceVersion {
    /// Create a new semantic version.
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch }
    }

    /// Parse version string like "0.1.0"
    pub fn parse(s: &str) -> Result<Self, VersionParseError> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err(VersionParseError::InvalidFormat);
        }
        let major = parts[0].parse().map_err(|_| VersionParseError::InvalidFormat)?;
        let minor = parts[1].parse().map_err(|_| VersionParseError::InvalidFormat)?;
        let patch = parts[2].parse().map_err(|_| VersionParseError::InvalidFormat)?;
        Ok(Self { major, minor, patch })
    }

    /// Return string representation
    pub fn as_str(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }

    /// Check if this matches CSCI v0.1 specification (maps to SDK v0.1.0)
    pub fn is_csci_v01_compatible(&self) -> bool {
        self.major == 0 && self.minor == 1
    }
}

impl fmt::Display for WorkspaceVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Error type for version parsing and validation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionParseError {
    InvalidFormat,
}

impl fmt::Display for VersionParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFormat => write!(f, "Invalid version format, expected major.minor.patch"),
        }
    }
}

impl std::error::Error for VersionParseError {}

/// Package type in the workspace
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PackageType {
    Rust,
    TypeScript,
    CSharp,
}

/// Workspace package metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspacePackage {
    pub name: String,
    pub package_type: PackageType,
    pub version: WorkspaceVersion,
    pub dependencies: Vec<String>,
}

impl WorkspacePackage {
    /// Create a new workspace package
    pub fn new(name: String, package_type: PackageType, version: WorkspaceVersion) -> Self {
        Self {
            name,
            package_type,
            version,
            dependencies: Vec::new(),
        }
    }

    /// Add a dependency to this package
    pub fn add_dependency(&mut self, dep: String) {
        if !self.dependencies.contains(&dep) {
            self.dependencies.push(dep);
        }
    }
}

/// Result type for monorepo operations
pub type MonorepoResult<T> = Result<T, MonorepoError>;

/// Comprehensive error type for monorepo integration failures
#[derive(Debug, Clone)]
pub enum MonorepoError {
    VersionMismatch {
        package: String,
        expected: String,
        found: String,
    },
    CircularDependency {
        packages: Vec<String>,
    },
    MissingDependency {
        package: String,
        dependency: String,
    },
    InvalidPackageType {
        package: String,
        reason: String,
    },
    BuildOrderResolutionFailed {
        reason: String,
    },
    WorkspaceValidationFailed {
        reason: String,
    },
}

impl fmt::Display for MonorepoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::VersionMismatch { package, expected, found } => {
                write!(f, "Version mismatch in {}: expected {}, found {}", package, expected, found)
            }
            Self::CircularDependency { packages } => {
                write!(f, "Circular dependency detected: {}", packages.join(" -> "))
            }
            Self::MissingDependency { package, dependency } => {
                write!(f, "Package {} depends on missing package {}", package, dependency)
            }
            Self::InvalidPackageType { package, reason } => {
                write!(f, "Invalid package type for {}: {}", package, reason)
            }
            Self::BuildOrderResolutionFailed { reason } => {
                write!(f, "Failed to resolve build order: {}", reason)
            }
            Self::WorkspaceValidationFailed { reason } => {
                write!(f, "Workspace validation failed: {}", reason)
            }
        }
    }
}

impl std::error::Error for MonorepoError {}

/// Monorepo workspace configuration and validation
pub struct WorkspaceConfig {
    packages: HashMap<String, WorkspacePackage>,
    version_mapping: HashMap<String, WorkspaceVersion>,
}

impl WorkspaceConfig {
    /// Create a new workspace configuration
    pub fn new() -> Self {
        Self {
            packages: HashMap::new(),
            version_mapping: HashMap::new(),
        }
    }

    /// Add a package to the workspace
    pub fn add_package(&mut self, package: WorkspacePackage) -> MonorepoResult<()> {
        if self.packages.contains_key(&package.name) {
            return Err(MonorepoError::WorkspaceValidationFailed {
                reason: format!("Package {} already exists", package.name),
            });
        }
        self.packages.insert(package.name.clone(), package);
        Ok(())
    }

    /// Synchronize versions ensuring CSCI v0.1 = SDKs v0.1.0
    pub fn synchronize_versions(&mut self, csci_version: WorkspaceVersion) -> MonorepoResult<()> {
        if !csci_version.is_csci_v01_compatible() {
            return Err(MonorepoError::VersionMismatch {
                package: "csci".to_string(),
                expected: "0.1.x".to_string(),
                found: csci_version.to_string(),
            });
        }

        // All SDKs must match 0.1.0 when CSCI is 0.1
        let sdk_version = WorkspaceVersion::new(0, 1, 0);

        for package in self.packages.values_mut() {
            match package.package_type {
                PackageType::Rust => {
                    if package.name.contains("csci") && package.version != csci_version {
                        return Err(MonorepoError::VersionMismatch {
                            package: package.name.clone(),
                            expected: csci_version.to_string(),
                            found: package.version.to_string(),
                        });
                    }
                }
                PackageType::TypeScript | PackageType::CSharp => {
                    if package.version != sdk_version {
                        package.version = sdk_version;
                    }
                }
            }
        }

        self.version_mapping.insert("csci".to_string(), csci_version);
        self.version_mapping.insert("sdk".to_string(), sdk_version);

        Ok(())
    }

    /// Resolve cross-project dependencies
    pub fn resolve_dependencies(&self) -> MonorepoResult<HashMap<String, Vec<String>>> {
        let mut resolved: HashMap<String, Vec<String>> = HashMap::new();

        for (pkg_name, package) in &self.packages {
            let mut deps = Vec::new();
            for dep in &package.dependencies {
                if !self.packages.contains_key(dep) {
                    return Err(MonorepoError::MissingDependency {
                        package: pkg_name.clone(),
                        dependency: dep.clone(),
                    });
                }
                deps.push(dep.clone());
            }
            resolved.insert(pkg_name.clone(), deps);
        }

        Ok(resolved)
    }

    /// Detect circular dependencies in the workspace
    fn detect_cycles(&self, resolved: &HashMap<String, Vec<String>>) -> MonorepoResult<()> {
        for start_node in self.packages.keys() {
            let mut visited = HashSet::new();
            let mut rec_stack = HashSet::new();

            if self.has_cycle(start_node, resolved, &mut visited, &mut rec_stack)? {
                return Err(MonorepoError::CircularDependency {
                    packages: vec![start_node.clone()],
                });
            }
        }
        Ok(())
    }

    fn has_cycle(
        &self,
        node: &str,
        resolved: &HashMap<String, Vec<String>>,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
    ) -> MonorepoResult<bool> {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());

        if let Some(deps) = resolved.get(node) {
            for dep in deps {
                if !visited.contains(dep) {
                    if self.has_cycle(dep, resolved, visited, rec_stack)? {
                        return Ok(true);
                    }
                } else if rec_stack.contains(dep) {
                    return Ok(true);
                }
            }
        }

        rec_stack.remove(node);
        Ok(false)
    }

    /// Compute build order using topological sort
    pub fn compute_build_order(&self) -> MonorepoResult<Vec<String>> {
        let resolved = self.resolve_dependencies()?;
        self.detect_cycles(&resolved)?;

        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();

        for pkg_name in self.packages.keys() {
            in_degree.insert(pkg_name.clone(), 0);
            graph.insert(pkg_name.clone(), Vec::new());
        }

        for (pkg_name, deps) in &resolved {
            for dep in deps {
                let entry = graph.entry(dep.clone()).or_insert_with(Vec::new);
                entry.push(pkg_name.clone());
            }
            *in_degree.get_mut(pkg_name).unwrap() = deps.len();
        }

        let mut queue: VecDeque<String> = in_degree
            .iter()
            .filter(|(_, &degree)| degree == 0)
            .map(|(pkg, _)| pkg.clone())
            .collect();

        let mut build_order = Vec::new();

        while let Some(pkg) = queue.pop_front() {
            build_order.push(pkg.clone());

            if let Some(dependents) = graph.get(&pkg) {
                for dependent in dependents {
                    let degree = in_degree.get_mut(dependent).unwrap();
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push_back(dependent.clone());
                    }
                }
            }
        }

        if build_order.len() != self.packages.len() {
            return Err(MonorepoError::BuildOrderResolutionFailed {
                reason: "Not all packages could be ordered".to_string(),
            });
        }

        Ok(build_order)
    }

    /// Validate entire workspace configuration
    pub fn validate(&mut self) -> MonorepoResult<()> {
        // Resolve dependencies to check for missing packages
        self._validate_dependencies()?;

        // Check that versions are synchronized
        let csci_version = WorkspaceVersion::new(0, 1, 0);
        self.synchronize_versions(csci_version)?;

        // Verify build order can be computed
        let _ = self.compute_build_order()?;

        Ok(())
    }

    fn _validate_dependencies(&self) -> MonorepoResult<()> {
        self.resolve_dependencies()?;
        let resolved = self.resolve_dependencies()?;
        self.detect_cycles(&resolved)?;
        Ok(())
    }

    /// Get package by name
    pub fn get_package(&self, name: &str) -> Option<&WorkspacePackage> {
        self.packages.get(name)
    }

    /// Get all packages
    pub fn packages(&self) -> &HashMap<String, WorkspacePackage> {
        &self.packages
    }
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use std::collections::VecDeque;






    #[test]
    fn test_version_parsing() {
        let v = WorkspaceVersion::parse("0.1.0").unwrap();
        assert_eq!(v.major, 0);
        assert_eq!(v.minor, 1);
        assert_eq!(v.patch, 0);

        let v_display = v.to_string();
        assert_eq!(v_display, "0.1.0");
    }

    #[test]
    fn test_invalid_version_parsing() {
        assert_eq!(
            WorkspaceVersion::parse("invalid"),
            Err(VersionParseError::InvalidFormat)
        );
    }

    #[test]
    fn test_csci_v01_compatibility() {
        let v = WorkspaceVersion::new(0, 1, 0);
        assert!(v.is_csci_v01_compatible());

        let v_mismatch = WorkspaceVersion::new(0, 2, 0);
        assert!(!v_mismatch.is_csci_v01_compatible());
    }

    #[test]
    fn test_workspace_package_creation() {
        let mut pkg = WorkspacePackage::new(
            "csci".to_string(),
            PackageType::Rust,
            WorkspaceVersion::new(0, 1, 0),
        );
        pkg.add_dependency("base".to_string());
        pkg.add_dependency("base".to_string()); // Duplicate

        assert_eq!(pkg.dependencies.len(), 1);
    }

    #[test]
    fn test_workspace_config_add_package() {
        let mut config = WorkspaceConfig::new();
        let pkg = WorkspacePackage::new(
            "test".to_string(),
            PackageType::Rust,
            WorkspaceVersion::new(0, 1, 0),
        );
        assert!(config.add_package(pkg).is_ok());
    }

    #[test]
    fn test_duplicate_package_rejection() {
        let mut config = WorkspaceConfig::new();
        let pkg1 = WorkspacePackage::new(
            "dup".to_string(),
            PackageType::Rust,
            WorkspaceVersion::new(0, 1, 0),
        );
        let pkg2 = WorkspacePackage::new(
            "dup".to_string(),
            PackageType::TypeScript,
            WorkspaceVersion::new(0, 1, 0),
        );

        assert!(config.add_package(pkg1).is_ok());
        assert!(config.add_package(pkg2).is_err());
    }

    #[test]
    fn test_missing_dependency_detection() {
        let mut config = WorkspaceConfig::new();
        let mut pkg = WorkspacePackage::new(
            "app".to_string(),
            PackageType::Rust,
            WorkspaceVersion::new(0, 1, 0),
        );
        pkg.add_dependency("nonexistent".to_string());

        let _ = config.add_package(pkg);
        assert!(config.resolve_dependencies().is_err());
    }

    #[test]
    fn test_circular_dependency_detection() {
        let mut config = WorkspaceConfig::new();

        let mut pkg_a = WorkspacePackage::new(
            "a".to_string(),
            PackageType::Rust,
            WorkspaceVersion::new(0, 1, 0),
        );
        pkg_a.add_dependency("b".to_string());

        let mut pkg_b = WorkspacePackage::new(
            "b".to_string(),
            PackageType::Rust,
            WorkspaceVersion::new(0, 1, 0),
        );
        pkg_b.add_dependency("a".to_string());

        let _ = config.add_package(pkg_a);
        let _ = config.add_package(pkg_b);

        let resolved = config.resolve_dependencies().unwrap();
        assert!(config.detect_cycles(&resolved).is_err());
    }

    #[test]
    fn test_build_order_resolution() {
        let mut config = WorkspaceConfig::new();

        let pkg_base = WorkspacePackage::new(
            "base".to_string(),
            PackageType::Rust,
            WorkspaceVersion::new(0, 1, 0),
        );

        let mut pkg_app = WorkspacePackage::new(
            "app".to_string(),
            PackageType::TypeScript,
            WorkspaceVersion::new(0, 1, 0),
        );
        pkg_app.add_dependency("base".to_string());

        let _ = config.add_package(pkg_base);
        let _ = config.add_package(pkg_app);

        let order = config.compute_build_order().unwrap();
        assert_eq!(order[0], "base");
        assert_eq!(order[1], "app");
    }

    #[test]
    fn test_version_synchronization() {
        let mut config = WorkspaceConfig::new();

        let pkg_csci = WorkspacePackage::new(
            "csci".to_string(),
            PackageType::Rust,
            WorkspaceVersion::new(0, 1, 5),
        );

        let pkg_ts = WorkspacePackage::new(
            "ts-sdk".to_string(),
            PackageType::TypeScript,
            WorkspaceVersion::new(0, 0, 1),
        );

        let _ = config.add_package(pkg_csci);
        let _ = config.add_package(pkg_ts);

        let csci_version = WorkspaceVersion::new(0, 1, 0);
        assert!(config.synchronize_versions(csci_version).is_ok());

        let ts_pkg = config.get_package("ts-sdk").unwrap();
        assert_eq!(ts_pkg.version, WorkspaceVersion::new(0, 1, 0));
    }

    #[test]
    fn test_complete_workspace_validation() {
        let mut config = WorkspaceConfig::new();

        let pkg_base = WorkspacePackage::new(
            "base".to_string(),
            PackageType::Rust,
            WorkspaceVersion::new(0, 1, 0),
        );

        let mut pkg_ts = WorkspacePackage::new(
            "ts-sdk".to_string(),
            PackageType::TypeScript,
            WorkspaceVersion::new(0, 1, 0),
        );
        pkg_ts.add_dependency("base".to_string());

        let mut pkg_cs = WorkspacePackage::new(
            "cs-sdk".to_string(),
            PackageType::CSharp,
            WorkspaceVersion::new(0, 1, 0),
        );
        pkg_cs.add_dependency("base".to_string());

        let _ = config.add_package(pkg_base);
        let _ = config.add_package(pkg_ts);
        let _ = config.add_package(pkg_cs);

        assert!(config.validate().is_ok());
    }
}
