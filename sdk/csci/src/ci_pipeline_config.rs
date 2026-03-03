//! # CI/CD Pipeline Configuration Module
//!
//! Defines comprehensive CI/CD pipeline stages for the Cognitive Substrate OS SDK,
//! including lint rules, type checking, unit testing, build targets, and publish gates
//! for both TypeScript and C# environments.
//!
//! ## Pipeline Flow
//! 1. Lint & Code Quality (ESLint, StyleCop)
//! 2. Type Checking (tsc, dotnet compile)
//! 3. Unit Tests (jest, xUnit)
//! 4. Build Artifacts
//! 5. Publish Gates & Version Validation
//! 6. Publish to Registries (npm, NuGet)

use serde::{Deserialize, Serialize};
use alloc::collections::BTreeMap as HashMap;
use core::fmt;

/// Strongly-typed lint rule identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LintRuleId(String);

impl LintRuleId {
    /// Create a new lint rule identifier
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the rule identifier as a string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for LintRuleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Lint configuration for code quality checks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintConfig {
    pub enabled: bool,
    pub rules: HashMap<LintRuleId, LintSeverity>,
    pub exclude_patterns: Vec<String>,
}

/// Severity level for lint violations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LintSeverity {
    Warning,
    Error,
}

impl LintConfig {
    /// Create ESLint configuration for TypeScript
    pub fn eslint_typescript() -> Self {
        let mut rules = HashMap::new();
        rules.insert(LintRuleId::new("@typescript-eslint/no-explicit-any"), LintSeverity::Error);
        rules.insert(LintRuleId::new("@typescript-eslint/strict-boolean-expressions"), LintSeverity::Error);
        rules.insert(LintRuleId::new("@typescript-eslint/explicit-function-return-types"), LintSeverity::Error);
        rules.insert(LintRuleId::new("no-console"), LintSeverity::Warning);
        rules.insert(LintRuleId::new("prefer-const"), LintSeverity::Error);
        rules.insert(LintRuleId::new("no-var"), LintSeverity::Error);

        Self {
            enabled: true,
            rules,
            exclude_patterns: vec![
                "node_modules/**".to_string(),
                "dist/**".to_string(),
                "build/**".to_string(),
            ],
        }
    }

    /// Create StyleCop configuration for C#
    pub fn stylecop_csharp() -> Self {
        let mut rules = HashMap::new();
        rules.insert(LintRuleId::new("SA1600"), LintSeverity::Warning); // Missing XML comment
        rules.insert(LintRuleId::new("SA1101"), LintSeverity::Error);   // Use this prefix
        rules.insert(LintRuleId::new("SA1309"), LintSeverity::Error);   // Field names must not begin with underscore
        rules.insert(LintRuleId::new("SA1402"), LintSeverity::Error);   // File may only contain one type
        rules.insert(LintRuleId::new("SA1519"), LintSeverity::Error);   // Braces should not be omitted

        Self {
            enabled: true,
            rules,
            exclude_patterns: vec![
                "bin/**".to_string(),
                "obj/**".to_string(),
                "packages/**".to_string(),
            ],
        }
    }

    /// Add a rule to the configuration
    pub fn add_rule(&mut self, rule: LintRuleId, severity: LintSeverity) {
        self.rules.insert(rule, severity);
    }

    /// Get the severity of a specific rule
    pub fn rule_severity(&self, rule: &LintRuleId) -> Option<LintSeverity> {
        self.rules.get(rule).copied()
    }

    /// Check if a rule is configured as an error
    pub fn is_error_rule(&self, rule: &LintRuleId) -> bool {
        self.rule_severity(rule) == Some(LintSeverity::Error)
    }
}

/// Type checking configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeCheckConfig {
    pub enabled: bool,
    pub strict_mode: bool,
    pub target_version: String,
    pub lib_versions: Vec<String>,
}

impl TypeCheckConfig {
    /// Create TypeScript type checking configuration
    pub fn typescript_strict() -> Self {
        Self {
            enabled: true,
            strict_mode: true,
            target_version: "ES2020".to_string(),
            lib_versions: vec![
                "ES2020".to_string(),
                "DOM".to_string(),
                "DOM.Iterable".to_string(),
            ],
        }
    }

    /// Create C# type checking configuration (.NET 8.0)
    pub fn csharp_net8() -> Self {
        Self {
            enabled: true,
            strict_mode: true,
            target_version: "net8.0".to_string(),
            lib_versions: vec!["net8.0".to_string()],
        }
    }
}

/// Unit test configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConfig {
    pub enabled: bool,
    pub framework: TestFramework,
    pub coverage_threshold: f32, // Percentage
    pub timeout_ms: u64,
}

/// Test framework enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TestFramework {
    Jest,
    XUnit,
    Mocha,
}

impl fmt::Display for TestFramework {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Jest => write!(f, "Jest"),
            Self::XUnit => write!(f, "xUnit"),
            Self::Mocha => write!(f, "Mocha"),
        }
    }
}

impl TestConfig {
    /// Create Jest configuration for TypeScript
    pub fn jest_typescript() -> Self {
        Self {
            enabled: true,
            framework: TestFramework::Jest,
            coverage_threshold: 80.0,
            timeout_ms: 30000,
        }
    }

    /// Create xUnit configuration for C#
    pub fn xunit_csharp() -> Self {
        Self {
            enabled: true,
            framework: TestFramework::XUnit,
            coverage_threshold: 80.0,
            timeout_ms: 60000,
        }
    }
}

/// Build target configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildTarget {
    pub name: String,
    pub platform: BuildPlatform,
    pub output_dir: String,
    pub source_dir: String,
    pub optimization_level: OptimizationLevel,
}

/// Target platform enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildPlatform {
    Node,
    Browser,
    NetFramework,
}

impl fmt::Display for BuildPlatform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Node => write!(f, "Node.js"),
            Self::Browser => write!(f, "Browser"),
            Self::NetFramework => write!(f, ".NET Framework"),
        }
    }
}

/// Optimization level enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OptimizationLevel {
    Debug,
    Release,
}

impl BuildTarget {
    /// Create a Node.js target for TypeScript
    pub fn nodejs_typescript() -> Self {
        Self {
            name: "ts-node".to_string(),
            platform: BuildPlatform::Node,
            output_dir: "dist".to_string(),
            source_dir: "src".to_string(),
            optimization_level: OptimizationLevel::Release,
        }
    }

    /// Create a browser target for TypeScript
    pub fn browser_typescript() -> Self {
        Self {
            name: "ts-browser".to_string(),
            platform: BuildPlatform::Browser,
            output_dir: "dist/browser".to_string(),
            source_dir: "src".to_string(),
            optimization_level: OptimizationLevel::Release,
        }
    }

    /// Create a .NET target for C#
    pub fn dotnet_csharp() -> Self {
        Self {
            name: "dotnet-release".to_string(),
            platform: BuildPlatform::NetFramework,
            output_dir: "bin/Release/net8.0".to_string(),
            source_dir: "src".to_string(),
            optimization_level: OptimizationLevel::Release,
        }
    }
}

/// Publish configuration and gates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishGate {
    pub name: String,
    pub required: bool,
    pub check_fn: String, // Serialized function name
}

impl PublishGate {
    /// Create a version gate
    pub fn version_gate() -> Self {
        Self {
            name: "version-check".to_string(),
            required: true,
            check_fn: "validate_version_compatibility".to_string(),
        }
    }

    /// Create a test coverage gate
    pub fn coverage_gate() -> Self {
        Self {
            name: "coverage-check".to_string(),
            required: true,
            check_fn: "validate_test_coverage".to_string(),
        }
    }

    /// Create a lint gate
    pub fn lint_gate() -> Self {
        Self {
            name: "lint-check".to_string(),
            required: true,
            check_fn: "validate_lint_rules".to_string(),
        }
    }

    /// Create a security scan gate
    pub fn security_gate() -> Self {
        Self {
            name: "security-scan".to_string(),
            required: true,
            check_fn: "validate_security".to_string(),
        }
    }
}

/// Registry configuration for package publication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    pub registry_type: RegistryType,
    pub url: String,
    pub timeout_ms: u64,
}

/// Registry type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegistryType {
    Npm,
    NuGet,
}

impl fmt::Display for RegistryType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Npm => write!(f, "npm"),
            Self::NuGet => write!(f, "NuGet"),
        }
    }
}

impl RegistryConfig {
    /// Create npm registry configuration
    pub fn npm_registry() -> Self {
        Self {
            registry_type: RegistryType::Npm,
            url: "https://registry.npmjs.org".to_string(),
            timeout_ms: 120000,
        }
    }

    /// Create NuGet registry configuration
    pub fn nuget_registry() -> Self {
        Self {
            registry_type: RegistryType::NuGet,
            url: "https://api.nuget.org/v3/index.json".to_string(),
            timeout_ms: 120000,
        }
    }
}

/// Complete CI/CD pipeline configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    pub name: String,
    pub platform: PlatformType,
    pub lint: LintConfig,
    pub type_check: TypeCheckConfig,
    pub test: TestConfig,
    pub build_targets: Vec<BuildTarget>,
    pub publish_gates: Vec<PublishGate>,
    pub registry: RegistryConfig,
}

/// Platform type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlatformType {
    TypeScript,
    CSharp,
    Rust,
}

impl fmt::Display for PlatformType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TypeScript => write!(f, "TypeScript"),
            Self::CSharp => write!(f, "C#"),
            Self::Rust => write!(f, "Rust"),
        }
    }
}

impl PipelineConfig {
    /// Create TypeScript pipeline configuration
    pub fn typescript_pipeline() -> Self {
        Self {
            name: "ts-sdk-pipeline".to_string(),
            platform: PlatformType::TypeScript,
            lint: LintConfig::eslint_typescript(),
            type_check: TypeCheckConfig::typescript_strict(),
            test: TestConfig::jest_typescript(),
            build_targets: vec![
                BuildTarget::nodejs_typescript(),
                BuildTarget::browser_typescript(),
            ],
            publish_gates: vec![
                PublishGate::version_gate(),
                PublishGate::lint_gate(),
                PublishGate::coverage_gate(),
                PublishGate::security_gate(),
            ],
            registry: RegistryConfig::npm_registry(),
        }
    }

    /// Create C# pipeline configuration
    pub fn csharp_pipeline() -> Self {
        Self {
            name: "cs-sdk-pipeline".to_string(),
            platform: PlatformType::CSharp,
            lint: LintConfig::stylecop_csharp(),
            type_check: TypeCheckConfig::csharp_net8(),
            test: TestConfig::xunit_csharp(),
            build_targets: vec![BuildTarget::dotnet_csharp()],
            publish_gates: vec![
                PublishGate::version_gate(),
                PublishGate::lint_gate(),
                PublishGate::coverage_gate(),
                PublishGate::security_gate(),
            ],
            registry: RegistryConfig::nuget_registry(),
        }
    }

    /// Validate pipeline configuration consistency
    pub fn validate(&self) -> Result<(), PipelineError> {
        if self.build_targets.is_empty() {
            return Err(PipelineError::NoTargets);
        }

        if self.publish_gates.is_empty() {
            return Err(PipelineError::NoGates);
        }

        // Validate platform consistency
        match self.platform {
            PlatformType::TypeScript => {
                if self.registry.registry_type != RegistryType::Npm {
                    return Err(PipelineError::RegistryMismatch);
                }
            }
            PlatformType::CSharp => {
                if self.registry.registry_type != RegistryType::NuGet {
                    return Err(PipelineError::RegistryMismatch);
                }
            }
            PlatformType::Rust => {
                // Rust uses different registry (crates.io)
            }
        }

        Ok(())
    }
}

/// Error type for pipeline configuration
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PipelineError {
    NoTargets,
    NoGates,
    RegistryMismatch,
}

impl fmt::Display for PipelineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoTargets => write!(f, "Pipeline must have at least one build target"),
            Self::NoGates => write!(f, "Pipeline must have at least one publish gate"),
            Self::RegistryMismatch => write!(f, "Registry type does not match platform"),
        }
    }
}

impl std::error::Error for PipelineError {}

/// Pipeline stage enumeration for execution tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PipelineStage {
    Lint,
    TypeCheck,
    Test,
    Build,
    PublishGates,
    Publish,
}

impl fmt::Display for PipelineStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Lint => write!(f, "Linting"),
            Self::TypeCheck => write!(f, "Type Checking"),
            Self::Test => write!(f, "Testing"),
            Self::Build => write!(f, "Building"),
            Self::PublishGates => write!(f, "Publish Gates"),
            Self::Publish => write!(f, "Publishing"),
        }
    }
}

/// Pipeline execution result tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineResult {
    pub stage: PipelineStage,
    pub success: bool,
    pub message: String,
    pub duration_ms: u64,
}

impl PipelineResult {
    /// Create a successful result
    pub fn success(stage: PipelineStage, duration_ms: u64) -> Self {
        Self {
            stage,
            success: true,
            message: format!("{} completed successfully", stage),
            duration_ms,
        }
    }

    /// Create a failed result
    pub fn failure(stage: PipelineStage, message: String, duration_ms: u64) -> Self {
        Self {
            stage,
            success: false,
            message,
            duration_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use alloc::vec;

    #[test]
    fn test_lint_config_eslint() {
        let config = LintConfig::eslint_typescript();
        assert!(config.enabled);
        assert!(!config.rules.is_empty());
        assert!(config
            .rule_severity(&LintRuleId::new("@typescript-eslint/no-explicit-any"))
            .is_some());
    }

    #[test]
    fn test_lint_config_stylecop() {
        let config = LintConfig::stylecop_csharp();
        assert!(config.enabled);
        assert!(!config.rules.is_empty());
        assert!(config
            .rule_severity(&LintRuleId::new("SA1101"))
            .is_some());
    }

    #[test]
    fn test_lint_rule_severity() {
        let mut config = LintConfig::eslint_typescript();
        let rule = LintRuleId::new("custom-rule");
        config.add_rule(rule.clone(), LintSeverity::Error);

        assert_eq!(
            config.rule_severity(&rule),
            Some(LintSeverity::Error)
        );
        assert!(config.is_error_rule(&rule));
    }

    #[test]
    fn test_type_check_typescript() {
        let config = TypeCheckConfig::typescript_strict();
        assert!(config.enabled);
        assert!(config.strict_mode);
        assert_eq!(config.target_version, "ES2020");
    }

    #[test]
    fn test_type_check_csharp() {
        let config = TypeCheckConfig::csharp_net8();
        assert!(config.enabled);
        assert!(config.strict_mode);
        assert_eq!(config.target_version, "net8.0");
    }

    #[test]
    fn test_test_config_jest() {
        let config = TestConfig::jest_typescript();
        assert!(config.enabled);
        assert_eq!(config.framework, TestFramework::Jest);
        assert_eq!(config.coverage_threshold, 80.0);
    }

    #[test]
    fn test_test_config_xunit() {
        let config = TestConfig::xunit_csharp();
        assert!(config.enabled);
        assert_eq!(config.framework, TestFramework::XUnit);
        assert_eq!(config.coverage_threshold, 80.0);
    }

    #[test]
    fn test_build_target_nodejs() {
        let target = BuildTarget::nodejs_typescript();
        assert_eq!(target.platform, BuildPlatform::Node);
        assert_eq!(target.output_dir, "dist");
    }

    #[test]
    fn test_build_target_dotnet() {
        let target = BuildTarget::dotnet_csharp();
        assert_eq!(target.platform, BuildPlatform::NetFramework);
        assert_eq!(target.output_dir, "bin/Release/net8.0");
    }

    #[test]
    fn test_publish_gates() {
        let gate = PublishGate::version_gate();
        assert!(gate.required);
        assert_eq!(gate.name, "version-check");
    }

    #[test]
    fn test_registry_npm() {
        let registry = RegistryConfig::npm_registry();
        assert_eq!(registry.registry_type, RegistryType::Npm);
        assert!(registry.url.contains("npmjs"));
    }

    #[test]
    fn test_registry_nuget() {
        let registry = RegistryConfig::nuget_registry();
        assert_eq!(registry.registry_type, RegistryType::NuGet);
        assert!(registry.url.contains("nuget.org"));
    }

    #[test]
    fn test_pipeline_typescript() {
        let pipeline = PipelineConfig::typescript_pipeline();
        assert_eq!(pipeline.platform, PlatformType::TypeScript);
        assert!(pipeline.lint.enabled);
        assert!(pipeline.test.enabled);
        assert!(!pipeline.build_targets.is_empty());
        assert!(pipeline.validate().is_ok());
    }

    #[test]
    fn test_pipeline_csharp() {
        let pipeline = PipelineConfig::csharp_pipeline();
        assert_eq!(pipeline.platform, PlatformType::CSharp);
        assert!(pipeline.lint.enabled);
        assert!(pipeline.test.enabled);
        assert!(!pipeline.build_targets.is_empty());
        assert!(pipeline.validate().is_ok());
    }

    #[test]
    fn test_pipeline_validation() {
        let mut pipeline = PipelineConfig::typescript_pipeline();
        assert!(pipeline.validate().is_ok());

        pipeline.build_targets.clear();
        assert!(pipeline.validate().is_err());
    }

    #[test]
    fn test_pipeline_stage_display() {
        assert_eq!(PipelineStage::Lint.to_string(), "Linting");
        assert_eq!(PipelineStage::TypeCheck.to_string(), "Type Checking");
        assert_eq!(PipelineStage::Publish.to_string(), "Publishing");
    }

    #[test]
    fn test_pipeline_result() {
        let result = PipelineResult::success(PipelineStage::Lint, 1500);
        assert!(result.success);
        assert_eq!(result.stage, PipelineStage::Lint);

        let failure = PipelineResult::failure(
            PipelineStage::Test,
            "Test failed".to_string(),
            2000,
        );
        assert!(!failure.success);
    }
}
