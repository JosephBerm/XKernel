// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! # Cognitive Substrate Package Manager (cs-pkg)
//!
//! The cs-pkg crate provides the package management system for Cognitive Substrate,
//! enabling discovery, installation, and dependency resolution for cognitive libraries
//! and tools.
//!
//! ## Architecture
//!
//! This crate defines:
//! - **PackageManifest**: Package metadata including version and dependencies
//! - **PackageRegistry trait**: Abstract interface for package operations
//! - **PackageResolver**: Dependency graph resolution for installation
//! - **StubRegistry**: In-memory registry implementation
//!
//! ## Design Philosophy
//!
//! The package manager follows semantic versioning (SemVer) and capability-based
//! dependency declaration. Each package declares required capabilities, enabling
//! the substrate to validate compatibility before installation.

#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

extern crate alloc;

use alloc::vec::Vec;
use alloc::string::String;
use alloc::collections::BTreeMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use ulid::Ulid;
use alloc::format;
use alloc::string::ToString;

/// Package identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PackageID(Ulid);

impl PackageID {
    /// Generate a new Package ID
    pub fn new() -> Self {
        PackageID(Ulid::new())
    }
}

impl Default for PackageID {
    fn default() -> Self {
        Self::new()
    }
}

/// Result type for package operations
pub type PkgResult<T> = Result<T, PkgError>;

/// Error types for package manager operations
#[derive(Debug, Clone, Error, Serialize, Deserialize)]
pub enum PkgError {
    /// Package not found
    #[error("Package not found: {0}")]
    PackageNotFound(String),

    /// Package already exists
    #[error("Package already exists: {0}")]
    PackageExists(String),

    /// Invalid package manifest
    #[error("Invalid package manifest: {0}")]
    InvalidManifest(String),

    /// Version mismatch
    #[error("Version mismatch: {0}")]
    VersionMismatch(String),

    /// Dependency resolution failed
    #[error("Dependency resolution failed: {0}")]
    DependencyResolutionFailed(String),

    /// Capability not supported
    #[error("Capability not supported: {0}")]
    CapabilityNotSupported(String),

    /// Installation failed
    #[error("Installation failed: {0}")]
    InstallationFailed(String),

    /// Publishing failed
    #[error("Publishing failed: {0}")]
    PublishingFailed(String),
}

/// Semantic version representation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Version {
    /// Major version number
    pub major: u32,
    /// Minor version number
    pub minor: u32,
    /// Patch version number
    pub patch: u32,
}

impl Version {
    /// Create a new version
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Version { major, minor, patch }
    }

    /// Parse version from string (e.g., "1.2.3")
    pub fn parse(s: &str) -> PkgResult<Self> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err(PkgError::InvalidManifest(
                "Version must be in format MAJOR.MINOR.PATCH".to_string()
            ));
        }

        let major = parts[0].parse::<u32>()
            .map_err(|_| PkgError::InvalidManifest("Invalid major version".to_string()))?;
        let minor = parts[1].parse::<u32>()
            .map_err(|_| PkgError::InvalidManifest("Invalid minor version".to_string()))?;
        let patch = parts[2].parse::<u32>()
            .map_err(|_| PkgError::InvalidManifest("Invalid patch version".to_string()))?;

        Ok(Version { major, minor, patch })
    }

    /// Convert to string representation
    pub fn to_string(&self) -> String {
        let mut result = String::new();
        result.push_str(&self.major.to_string());
        result.push('.');
        result.push_str(&self.minor.to_string());
        result.push('.');
        result.push_str(&self.patch.to_string());
        result
    }
}

/// Dependency specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    /// Package name
    pub name: String,
    /// Required version (supports semantic versioning)
    pub version: String,
    /// Whether this is an optional dependency
    pub optional: bool,
}

/// Package capability requirement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityRequirement {
    /// Capability name
    pub name: String,
    /// Minimum version required
    pub min_version: String,
}

/// Package manifest - metadata for a package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageManifest {
    /// Unique package name
    pub name: String,
    /// Package version
    pub version: Version,
    /// Package description
    pub description: String,
    /// Author name
    pub author: String,
    /// License identifier
    pub license: String,
    /// Package dependencies
    pub dependencies: Vec<Dependency>,
    /// Required capabilities
    pub capabilities_required: Vec<CapabilityRequirement>,
    /// Package metadata
    pub metadata: BTreeMap<String, String>,
}

impl PackageManifest {
    /// Validate manifest correctness
    pub fn validate(&self) -> PkgResult<()> {
        if self.name.is_empty() {
            return Err(PkgError::InvalidManifest("Package name cannot be empty".to_string()));
        }

        if self.description.is_empty() {
            return Err(PkgError::InvalidManifest("Package description cannot be empty".to_string()));
        }

        if self.author.is_empty() {
            return Err(PkgError::InvalidManifest("Package author cannot be empty".to_string()));
        }

        for dep in &self.dependencies {
            if dep.name.is_empty() {
                return Err(PkgError::InvalidManifest(
                    "Dependency name cannot be empty".to_string()
                ));
            }
        }

        Ok(())
    }
}

/// Query parameters for package search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    /// Search term
    pub term: String,
    /// Filter by author
    pub author: Option<String>,
    /// Filter by capability
    pub capability: Option<String>,
    /// Limit results
    pub limit: usize,
}

/// Package search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Matching packages
    pub packages: Vec<PackageManifest>,
    /// Total matches found
    pub total: usize,
}

/// Dependency graph node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepGraphNode {
    /// Package manifest
    pub manifest: PackageManifest,
    /// Resolved version
    pub resolved_version: Version,
    /// Dependencies (package names)
    pub dependencies: Vec<String>,
}

/// Resolved dependency graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyGraph {
    /// Root package
    pub root: String,
    /// All nodes in graph
    pub nodes: BTreeMap<String, DepGraphNode>,
    /// Installation order
    pub install_order: Vec<String>,
}

/// Package Registry trait - defines package management operations
pub trait PackageRegistry {
    /// Search for packages
    fn search(&self, query: SearchQuery) -> PkgResult<SearchResult>;

    /// Get package manifest
    fn get_manifest(&self, name: &str, version: &str) -> PkgResult<PackageManifest>;

    /// Publish a package
    fn publish(&mut self, package: PackageManifest) -> PkgResult<PackageID>;

    /// Install a package by name and version
    fn install(&mut self, name: &str, version: &str) -> PkgResult<PackageID>;

    /// List installed packages
    fn list_installed(&self) -> PkgResult<Vec<PackageManifest>>;
}

/// Package Resolver - resolves dependency graphs for installation
#[derive(Debug)]
pub struct PackageResolver {
    registry: *const dyn PackageRegistry,
}

impl PackageResolver {
    /// Create new resolver
    pub fn new(_registry: &dyn PackageRegistry) -> Self {
        PackageResolver {
            registry: _registry as *const dyn PackageRegistry,
        }
    }

    /// Resolve dependencies for a package
    pub fn resolve(&self, name: &str, version: &str) -> PkgResult<DependencyGraph> {
        // Validate inputs
        if name.is_empty() {
            return Err(PkgError::InvalidManifest("Package name cannot be empty".to_string()));
        }

        if version.is_empty() {
            return Err(PkgError::InvalidManifest("Version cannot be empty".to_string()));
        }

        // Build stub graph
        let mut nodes = BTreeMap::new();
        let mut install_order = Vec::new();

        // Add root package
        let manifest = PackageManifest {
            name: name.to_string(),
            version: Version::parse(version)?,
            description: "resolved package".to_string(),
            author: "system".to_string(),
            license: "Apache-2.0".to_string(),
            dependencies: Vec::new(),
            capabilities_required: Vec::new(),
            metadata: BTreeMap::new(),
        };

        install_order.push(name.to_string());
        nodes.insert(name.to_string(), DepGraphNode {
            manifest: manifest.clone(),
            resolved_version: manifest.version.clone(),
            dependencies: Vec::new(),
        });

        Ok(DependencyGraph {
            root: name.to_string(),
            nodes,
            install_order,
        })
    }

    /// Check if capability is satisfied
    pub fn check_capability(&self, capability: &str, min_version: &str) -> PkgResult<bool> {
        if capability.is_empty() {
            return Err(PkgError::CapabilityNotSupported("Empty capability name".to_string()));
        }

        if min_version.is_empty() {
            return Err(PkgError::InvalidManifest("Version cannot be empty".to_string()));
        }

        // Stub implementation - always return true for now
        Ok(true)
    }
}

/// Stub Registry implementation for testing
#[derive(Debug)]
pub struct StubRegistry {
    packages: BTreeMap<String, Vec<PackageManifest>>,
}

impl StubRegistry {
    /// Create new stub registry
    pub fn new() -> Self {
        StubRegistry {
            packages: BTreeMap::new(),
        }
    }

    /// Add a package to the registry (for testing)
    pub fn add_package(&mut self, manifest: PackageManifest) {
        self.packages
            .entry(manifest.name.clone())
            .or_insert_with(Vec::new)
            .push(manifest);
    }
}

impl Default for StubRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PackageRegistry for StubRegistry {
    fn search(&self, query: SearchQuery) -> PkgResult<SearchResult> {
        if query.term.is_empty() {
            return Err(PkgError::InvalidManifest("Search term cannot be empty".to_string()));
        }

        let mut results = Vec::new();

        for packages in self.packages.values() {
            for pkg in packages {
                if pkg.name.contains(&query.term) || pkg.description.contains(&query.term) {
                    if let Some(ref author) = query.author {
                        if pkg.author != *author {
                            continue;
                        }
                    }
                    results.push(pkg.clone());
                    if results.len() >= query.limit {
                        break;
                    }
                }
            }
            if results.len() >= query.limit {
                break;
            }
        }

        let total = results.len();
        Ok(SearchResult {
            packages: results,
            total,
        })
    }

    fn get_manifest(&self, name: &str, version: &str) -> PkgResult<PackageManifest> {
        let packages = self.packages.get(name)
            .ok_or_else(|| PkgError::PackageNotFound(name.to_string()))?;

        packages.iter()
            .find(|p| p.version.to_string() == version)
            .cloned()
            .ok_or_else(|| PkgError::VersionMismatch(
                format!("Package {} version {} not found", name, version)
            ))
    }

    fn publish(&mut self, package: PackageManifest) -> PkgResult<PackageID> {
        package.validate()?;

        if let Some(versions) = self.packages.get(&package.name) {
            if versions.iter().any(|p| p.version == package.version) {
                return Err(PkgError::PackageExists(
                    format!("{}@{}", package.name, package.version.to_string())
                ));
            }
        }

        self.packages
            .entry(package.name.clone())
            .or_insert_with(Vec::new)
            .push(package);

        Ok(PackageID::new())
    }

    fn install(&mut self, name: &str, version: &str) -> PkgResult<PackageID> {
        let _manifest = self.get_manifest(name, version)?;
        // Stub - just return new ID on successful lookup
        Ok(PackageID::new())
    }

    fn list_installed(&self) -> PkgResult<Vec<PackageManifest>> {
        let mut result = Vec::new();
        for packages in self.packages.values() {
            for pkg in packages {
                result.push(pkg.clone());
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_id_generation() {
        let id1 = PackageID::new();
        let id2 = PackageID::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_version_creation() {
        let v = Version::new(1, 2, 3);
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
    }

    #[test]
    fn test_version_parse() {
        let v = Version::parse("1.2.3").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
    }

    #[test]
    fn test_version_parse_invalid() {
        assert!(Version::parse("1.2").is_err());
        assert!(Version::parse("a.b.c").is_err());
    }

    #[test]
    fn test_version_to_string() {
        let v = Version::new(2, 0, 1);
        assert_eq!(v.to_string(), "2.0.1");
    }

    #[test]
    fn test_manifest_validate() {
        let manifest = PackageManifest {
            name: "test-pkg".to_string(),
            version: Version::new(0, 1, 0),
            description: "A test package".to_string(),
            author: "Test Author".to_string(),
            license: "Apache-2.0".to_string(),
            dependencies: Vec::new(),
            capabilities_required: Vec::new(),
            metadata: BTreeMap::new(),
        };
        assert!(manifest.validate().is_ok());
    }

    #[test]
    fn test_manifest_validate_empty_name() {
        let manifest = PackageManifest {
            name: String::new(),
            version: Version::new(0, 1, 0),
            description: "A test package".to_string(),
            author: "Test Author".to_string(),
            license: "Apache-2.0".to_string(),
            dependencies: Vec::new(),
            capabilities_required: Vec::new(),
            metadata: BTreeMap::new(),
        };
        assert!(manifest.validate().is_err());
    }

    #[test]
    fn test_stub_registry_new() {
        let registry = StubRegistry::new();
        assert_eq!(registry.packages.len(), 0);
    }

    #[test]
    fn test_stub_registry_publish() {
        let mut registry = StubRegistry::new();
        let manifest = PackageManifest {
            name: "test-pkg".to_string(),
            version: Version::new(0, 1, 0),
            description: "A test package".to_string(),
            author: "Test Author".to_string(),
            license: "Apache-2.0".to_string(),
            dependencies: Vec::new(),
            capabilities_required: Vec::new(),
            metadata: BTreeMap::new(),
        };

        let id = registry.publish(manifest).unwrap();
        assert_eq!(registry.packages.len(), 1);
        assert_ne!(id, PackageID::new());
    }

    #[test]
    fn test_stub_registry_publish_duplicate() {
        let mut registry = StubRegistry::new();
        let manifest = PackageManifest {
            name: "test-pkg".to_string(),
            version: Version::new(0, 1, 0),
            description: "A test package".to_string(),
            author: "Test Author".to_string(),
            license: "Apache-2.0".to_string(),
            dependencies: Vec::new(),
            capabilities_required: Vec::new(),
            metadata: BTreeMap::new(),
        };

        registry.publish(manifest.clone()).unwrap();
        assert!(registry.publish(manifest).is_err());
    }

    #[test]
    fn test_stub_registry_get_manifest() {
        let mut registry = StubRegistry::new();
        let manifest = PackageManifest {
            name: "my-package".to_string(),
            version: Version::new(1, 0, 0),
            description: "Test".to_string(),
            author: "Author".to_string(),
            license: "Apache-2.0".to_string(),
            dependencies: Vec::new(),
            capabilities_required: Vec::new(),
            metadata: BTreeMap::new(),
        };

        registry.publish(manifest).unwrap();
        let found = registry.get_manifest("my-package", "1.0.0").unwrap();
        assert_eq!(found.name, "my-package");
    }

    #[test]
    fn test_stub_registry_get_manifest_not_found() {
        let registry = StubRegistry::new();
        assert!(registry.get_manifest("nonexistent", "1.0.0").is_err());
    }

    #[test]
    fn test_stub_registry_search() {
        let mut registry = StubRegistry::new();
        let manifest = PackageManifest {
            name: "neural-net".to_string(),
            version: Version::new(0, 1, 0),
            description: "Neural network library".to_string(),
            author: "AI Team".to_string(),
            license: "Apache-2.0".to_string(),
            dependencies: Vec::new(),
            capabilities_required: Vec::new(),
            metadata: BTreeMap::new(),
        };

        registry.publish(manifest).unwrap();

        let query = SearchQuery {
            term: "neural".to_string(),
            author: None,
            capability: None,
            limit: 10,
        };

        let results = registry.search(query).unwrap();
        assert_eq!(results.total, 1);
        assert_eq!(results.packages[0].name, "neural-net");
    }

    #[test]
    fn test_stub_registry_list_installed() {
        let mut registry = StubRegistry::new();
        let manifest = PackageManifest {
            name: "test-pkg".to_string(),
            version: Version::new(0, 1, 0),
            description: "Test".to_string(),
            author: "Author".to_string(),
            license: "Apache-2.0".to_string(),
            dependencies: Vec::new(),
            capabilities_required: Vec::new(),
            metadata: BTreeMap::new(),
        };

        registry.publish(manifest).unwrap();
        let installed = registry.list_installed().unwrap();
        assert_eq!(installed.len(), 1);
    }

    #[test]
    fn test_package_resolver() {
        let registry = StubRegistry::new();
        let resolver = PackageResolver::new(&registry);

        let graph = resolver.resolve("my-package", "1.0.0").unwrap();
        assert_eq!(graph.root, "my-package");
        assert!(graph.nodes.contains_key("my-package"));
    }

    #[test]
    fn test_package_resolver_invalid_name() {
        let registry = StubRegistry::new();
        let resolver = PackageResolver::new(&registry);

        assert!(resolver.resolve("", "1.0.0").is_err());
    }

    #[test]
    fn test_package_resolver_check_capability() {
        let registry = StubRegistry::new();
        let resolver = PackageResolver::new(&registry);

        let result = resolver.check_capability("gpu_support", "1.0.0").unwrap();
        assert!(result);
    }

    #[test]
    fn test_dependency_struct() {
        let dep = Dependency {
            name: "some-lib".to_string(),
            version: "1.0.0".to_string(),
            optional: false,
        };
        assert_eq!(dep.name, "some-lib");
    }

    #[test]
    fn test_capability_requirement_struct() {
        let cap = CapabilityRequirement {
            name: "gpu_compute".to_string(),
            min_version: "2.0.0".to_string(),
        };
        assert_eq!(cap.name, "gpu_compute");
    }
}
