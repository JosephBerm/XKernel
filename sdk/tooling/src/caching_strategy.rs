//! Bazel and Dependency Caching Strategy
//!
//! Comprehensive caching configuration for optimizing build times across:
//! - Remote cache configuration for Bazel build artifacts
//! - Rust dependency caching
//! - Node.js module caching
//! - Build artifact caching
//! - Test result caching (replay without re-running)
//! - Intelligent cache invalidation rules
//!
//! This module generates configurations suitable for CI/CD environments
//! and local development workflows.

use serde::{Deserialize, Serialize};
use alloc::collections::BTreeMap as HashMap;
use alloc::format;
use alloc::vec;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::string::ToString;
// no_std: std::time not available

/// Cache backend type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum CacheBackend {
    /// HTTP remote cache (GCS, S3, generic HTTP)
    Http,
    /// Bazel's built-in repository cache
    Bazel,
    /// Local file system cache
    Filesystem,
    /// Redis distributed cache
    Redis,
}

impl std::fmt::Display for CacheBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CacheBackend::Http => write!(f, "http"),
            CacheBackend::Bazel => write!(f, "bazel"),
            CacheBackend::Filesystem => write!(f, "filesystem"),
            CacheBackend::Redis => write!(f, "redis"),
        }
    }
}

/// Cache tier (local, CI, or shared)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum CacheTier {
    /// Local developer cache
    Local,
    /// Ephemeral CI runner cache
    CIRunner,
    /// Shared remote cache
    Remote,
}

/// Remote cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteCacheConfig {
    /// Backend type
    pub backend: CacheBackend,
    /// Base URL for HTTP backends
    pub url: String,
    /// Authentication token (if required)
    pub auth_token: Option<String>,
    /// TLS certificate path for validation
    pub cert_path: Option<String>,
    /// Read timeout in seconds
    pub read_timeout_seconds: u32,
    /// Write timeout in seconds
    pub write_timeout_seconds: u32,
    /// Maximum cache size in GB
    pub max_size_gb: u32,
    /// Compression enabled
    pub compression_enabled: bool,
    /// Cache eviction policy (lru, lfu, fifo)
    pub eviction_policy: String,
}

impl RemoteCacheConfig {
    /// Create new remote cache configuration
    pub fn new(backend: CacheBackend, url: impl Into<String>) -> Self {
        Self {
            backend,
            url: url.into(),
            auth_token: None,
            cert_path: None,
            read_timeout_seconds: 30,
            write_timeout_seconds: 60,
            max_size_gb: 500,
            compression_enabled: true,
            eviction_policy: "lru".to_string(),
        }
    }

    /// GCS bucket configuration
    pub fn gcs(bucket_name: impl Into<String>) -> Self {
        Self::new(
            CacheBackend::Http,
            format!("https://storage.googleapis.com/{}", bucket_name.into()),
        )
        .with_compression(true)
    }

    /// AWS S3 bucket configuration
    pub fn s3(bucket_region: &str, bucket_name: impl Into<String>) -> Self {
        Self::new(
            CacheBackend::Http,
            format!(
                "https://{}.s3.{}.amazonaws.com/{}",
                bucket_name.into(),
                bucket_region,
                "cache"
            ),
        )
        .with_compression(true)
    }

    /// Set authentication token
    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.auth_token = Some(token.into());
        self
    }

    /// Enable/disable compression
    pub fn with_compression(mut self, enabled: bool) -> Self {
        self.compression_enabled = enabled;
        self
    }

    /// Set eviction policy
    pub fn with_eviction_policy(mut self, policy: impl Into<String>) -> Self {
        self.eviction_policy = policy.into();
        self
    }

    /// Set maximum cache size
    pub fn with_max_size(mut self, size_gb: u32) -> Self {
        self.max_size_gb = size_gb;
        self
    }

    /// Generate Bazel .bazelrc configuration
    pub fn to_bazelrc(&self) -> String {
        let mut config = String::new();
        config.push_str(&format!("# Remote cache configuration\n"));
        config.push_str(&format!(
            "build --remote_cache={}://{}\n",
            self.backend, self.url
        ));

        if self.compression_enabled {
            config.push_str("build --remote_cache_compression\n");
        }

        config.push_str(&format!(
            "build --remote_timeout={}\n",
            self.read_timeout_seconds
        ));

        if let Some(token) = &self.auth_token {
            config.push_str(&format!("build --remote_header=Authorization=Bearer%20{}\n", token));
        }

        config
    }
}

/// Rust dependency cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustCacheConfig {
    /// Enable incremental compilation caching
    pub incremental_enabled: bool,
    /// Enable dependency caching
    pub dependency_cache_enabled: bool,
    /// Location of cargo cache directory
    pub cargo_home: String,
    /// Maximum cache size in GB
    pub max_size_gb: u32,
    /// Parallel jobs for compilation
    pub parallel_jobs: u32,
    /// Use sccache for distributed caching
    pub use_sccache: bool,
    /// sccache server URL (if enabled)
    pub sccache_url: Option<String>,
}

impl RustCacheConfig {
    /// Create default Rust cache configuration
    pub fn default_config() -> Self {
        Self {
            incremental_enabled: true,
            dependency_cache_enabled: true,
            cargo_home: "$HOME/.cargo".to_string(),
            max_size_gb: 100,
            parallel_jobs: num_cpus::get() as u32,
            use_sccache: false,
            sccache_url: None,
        }
    }

    /// Create CI-optimized configuration
    pub fn ci_config() -> Self {
        Self {
            incremental_enabled: true,
            dependency_cache_enabled: true,
            cargo_home: "/tmp/cargo".to_string(),
            max_size_gb: 500,
            parallel_jobs: 8,
            use_sccache: true,
            sccache_url: Some("http://sccache:9999".to_string()),
        }
    }

    /// Enable sccache
    pub fn with_sccache(mut self, url: impl Into<String>) -> Self {
        self.use_sccache = true;
        self.sccache_url = Some(url.into());
        self
    }

    /// Generate cargo environment variables
    pub fn to_env_vars(&self) -> HashMap<String, String> {
        let mut vars = HashMap::new();

        vars.insert("CARGO_HOME".to_string(), self.cargo_home.clone());
        vars.insert(
            "CARGO_INCREMENTAL".to_string(),
            if self.incremental_enabled { "1" } else { "0" }.to_string(),
        );
        vars.insert(
            "CARGO_NET_OFFLINE".to_string(),
            "false".to_string(),
        );
        vars.insert(
            "RUSTC_WRAPPER".to_string(),
            if self.use_sccache {
                "sccache".to_string()
            } else {
                "".to_string()
            },
        );

        vars
    }
}

/// Node.js module cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCacheConfig {
    /// Enable npm caching
    pub npm_cache_enabled: bool,
    /// Location of npm cache
    pub npm_cache_dir: String,
    /// Enable yarn offline mode
    pub yarn_offline_enabled: bool,
    /// Location of yarn cache
    pub yarn_cache_dir: String,
    /// Maximum cache size in GB
    pub max_size_gb: u32,
    /// Cache lock files to detect changes
    pub cache_lock_files: bool,
}

impl NodeCacheConfig {
    /// Create default Node cache configuration
    pub fn default_config() -> Self {
        Self {
            npm_cache_enabled: true,
            npm_cache_dir: "$HOME/.npm".to_string(),
            yarn_offline_enabled: true,
            yarn_cache_dir: "$HOME/.yarn/cache".to_string(),
            max_size_gb: 50,
            cache_lock_files: true,
        }
    }

    /// Create CI-optimized configuration
    pub fn ci_config() -> Self {
        Self {
            npm_cache_enabled: true,
            npm_cache_dir: "/tmp/npm-cache".to_string(),
            yarn_offline_enabled: true,
            yarn_cache_dir: "/tmp/yarn-cache".to_string(),
            max_size_gb: 200,
            cache_lock_files: true,
        }
    }

    /// Generate npm configuration
    pub fn to_npmrc(&self) -> String {
        let mut config = String::new();
        if self.npm_cache_enabled {
            config.push_str(&format!("cache={}\n", self.npm_cache_dir));
            config.push_str("prefer-offline=true\n");
        }
        config
    }
}

/// Test result caching configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCacheConfig {
    /// Enable test result caching
    pub enabled: bool,
    /// Cache location
    pub cache_dir: String,
    /// Test result retention period in days
    pub retention_days: u32,
    /// Invalidation strategies
    pub invalidation_rules: Vec<String>,
    /// Compression for cached results
    pub compression_enabled: bool,
}

impl TestCacheConfig {
    /// Create default test cache configuration
    pub fn default_config() -> Self {
        Self {
            enabled: true,
            cache_dir: "$PROJECT_ROOT/.test_cache".to_string(),
            retention_days: 7,
            invalidation_rules: vec![
                "source_code_changed".to_string(),
                "test_input_changed".to_string(),
                "dependencies_updated".to_string(),
            ],
            compression_enabled: true,
        }
    }

    /// Add invalidation rule
    pub fn add_invalidation_rule(mut self, rule: impl Into<String>) -> Self {
        self.invalidation_rules.push(rule.into());
        self
    }
}

/// Cache invalidation rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheInvalidationRule {
    /// Rule identifier
    pub id: String,
    /// Rule description
    pub description: String,
    /// Pattern to match (glob or regex)
    pub pattern: String,
    /// Action on match (invalidate/warn/skip)
    pub action: String,
    /// Components affected
    pub affects: Vec<String>,
}

impl CacheInvalidationRule {
    /// Create new invalidation rule
    pub fn new(id: impl Into<String>, pattern: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            description: String::new(),
            pattern: pattern.into(),
            action: "invalidate".to_string(),
            affects: Vec::new(),
        }
    }

    /// Add affected component
    pub fn add_component(mut self, comp: impl Into<String>) -> Self {
        self.affects.push(comp.into());
        self
    }

    /// Set action
    pub fn with_action(mut self, action: impl Into<String>) -> Self {
        self.action = action.into();
        self
    }
}

/// Complete caching strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachingStrategy {
    /// Strategy name
    pub name: String,
    /// Description
    pub description: String,
    /// Cache tier
    pub tier: CacheTier,
    /// Remote cache configuration
    pub remote_cache: Option<RemoteCacheConfig>,
    /// Rust cache configuration
    pub rust_cache: RustCacheConfig,
    /// Node cache configuration
    pub node_cache: NodeCacheConfig,
    /// Test cache configuration
    pub test_cache: TestCacheConfig,
    /// Cache invalidation rules
    pub invalidation_rules: Vec<CacheInvalidationRule>,
    /// Build artifact retention (days)
    pub artifact_retention_days: u32,
    /// Enable cache statistics
    pub collect_stats: bool,
}

impl CachingStrategy {
    /// Create new caching strategy
    pub fn new(name: impl Into<String>, tier: CacheTier) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            tier,
            remote_cache: None,
            rust_cache: RustCacheConfig::default_config(),
            node_cache: NodeCacheConfig::default_config(),
            test_cache: TestCacheConfig::default_config(),
            invalidation_rules: Vec::new(),
            artifact_retention_days: 30,
            collect_stats: true,
        }
    }

    /// Create local development strategy
    pub fn local_dev() -> Self {
        Self::new("local-development", CacheTier::Local)
            .with_description("Optimized for single developer local builds")
    }

    /// Create CI strategy
    pub fn ci_strategy(remote_url: impl Into<String>) -> Self {
        let mut strategy =
            Self::new("ci-pipeline", CacheTier::Remote)
                .with_description("Optimized for CI/CD pipeline execution");
        strategy.remote_cache = Some(RemoteCacheConfig::new(CacheBackend::Http, remote_url));
        strategy.rust_cache = RustCacheConfig::ci_config();
        strategy.node_cache = NodeCacheConfig::ci_config();
        strategy.artifact_retention_days = 90;
        strategy
    }

    /// Set description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Set remote cache
    pub fn with_remote_cache(mut self, cache: RemoteCacheConfig) -> Self {
        self.remote_cache = Some(cache);
        self
    }

    /// Add invalidation rule
    pub fn add_invalidation_rule(mut self, rule: CacheInvalidationRule) -> Self {
        self.invalidation_rules.push(rule);
        self
    }

    /// Generate .bazelrc configuration
    pub fn generate_bazelrc(&self) -> String {
        let mut config = String::new();
        config.push_str("# Generated Bazel configuration\n");
        config.push_str(&format!("# Strategy: {}\n", self.name));
        config.push_str("common --experimental_repository_cache_hardlinks\n");
        config.push_str("build --repository_cache=~/.bazel/repository_cache\n");

        if let Some(remote) = &self.remote_cache {
            config.push_str(&remote.to_bazelrc());
        }

        config.push_str("build --disk_cache=~/.bazel/disk_cache\n");
        config.push_str("build --save_temps\n");

        config
    }

    /// Export as JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self)
    }

    /// Get total cache capacity
    pub fn total_cache_capacity_gb(&self) -> u32 {
        let mut total = self.rust_cache.max_size_gb + self.node_cache.max_size_gb;
        if let Some(remote) = &self.remote_cache {
            total += remote.max_size_gb;
        }
        total
    }

    /// Generate caching recommendations
    pub fn recommendations(&self) -> Vec<String> {
        let mut recs = Vec::new();

        if self.tier == CacheTier::CIRunner && self.remote_cache.is_none() {
            recs.push(
                "Enable remote cache for CI runners to improve build times".to_string()
            );
        }

        if self.rust_cache.parallel_jobs > 16 {
            recs.push(
                "Consider reducing parallel jobs to avoid resource contention".to_string()
            );
        }

        if self.test_cache.retention_days < 7 {
            recs.push(
                "Increase test cache retention for better performance on dependent features".to_string()
            );
        }

        if self.invalidation_rules.is_empty() {
            recs.push(
                "Define cache invalidation rules to prevent stale cache hits".to_string()
            );
        }

        recs
    }
}

impl Default for CachingStrategy {
    fn default() -> Self {
        Self::local_dev()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remote_cache_creation() {
        let cache = RemoteCacheConfig::new(CacheBackend::Http, "https://example.com");
        assert_eq!(cache.backend, CacheBackend::Http);
        assert_eq!(cache.read_timeout_seconds, 30);
    }

    #[test]
    fn test_s3_cache() {
        let cache = RemoteCacheConfig::s3("us-east-1", "my-bucket");
        assert!(cache.url.contains("amazonaws.com"));
    }

    #[test]
    fn test_rust_cache_env_vars() {
        let cache = RustCacheConfig::default_config();
        let vars = cache.to_env_vars();
        assert_eq!(vars.get("CARGO_INCREMENTAL"), Some(&"1".to_string()));
    }

    #[test]
    fn test_caching_strategy() {
        let strategy = CachingStrategy::ci_strategy("https://cache.example.com");
        assert_eq!(strategy.tier, CacheTier::Remote);
        assert!(strategy.remote_cache.is_some());
    }

    #[test]
    fn test_bazelrc_generation() {
        let strategy = CachingStrategy::local_dev();
        let bazelrc = strategy.generate_bazelrc();
        assert!(bazelrc.contains("bazel"));
    }

    #[test]
    fn test_cache_capacity() {
        let strategy = CachingStrategy::default();
        let capacity = strategy.total_cache_capacity_gb();
        assert!(capacity > 0);
    }

    #[test]
    fn test_invalidation_rule() {
        let rule = CacheInvalidationRule::new("test", "*.rs")
            .add_component("rust");
        assert_eq!(rule.affects.len(), 1);
    }

    #[test]
    fn test_test_cache_config() {
        let mut cache = TestCacheConfig::default_config();
        cache = cache.add_invalidation_rule("custom_rule");
        assert_eq!(cache.invalidation_rules.len(), 4);
    }
}
