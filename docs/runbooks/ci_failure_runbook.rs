//! CI Failure Runbook Module
//!
//! Comprehensive runbook for diagnosing and resolving the top 5 CI failure modes:
//! 1. Bazel cache miss/corruption
//! 2. Clippy lint failures
//! 3. Test timeout on QEMU
//! 4. Cross-compilation errors
//! 5. Dependency resolution failures
//!
//! Each failure mode includes:
//! - Symptoms and detection patterns
//! - Step-by-step diagnosis procedures
//! - Resolution strategies
//! - Prevention and best practices

use serde::{Deserialize, Serialize};
use alloc::collections::BTreeMap as HashMap;
use alloc::vec;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::string::ToString;

/// Severity level for CI failures
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "UPPERCASE")]
pub enum SeverityLevel {
    /// Can be ignored, non-critical
    Info,
    /// Should be addressed, may impact performance
    Warning,
    /// Must be fixed before merge
    Error,
    /// System-critical, blocks all CI
    Critical,
}

/// Step in a diagnosis procedure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosisStep {
    /// Step number
    pub step: u32,
    /// Command or action to execute
    pub action: String,
    /// Expected output pattern or result
    pub expected_output: String,
    /// What to look for in output
    pub check: String,
}

/// Resolution action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionAction {
    /// Action name
    pub name: String,
    /// Step-by-step instructions
    pub steps: Vec<String>,
    /// Estimated time to resolve in minutes
    pub estimated_time_minutes: u32,
    /// Risk level (low/medium/high)
    pub risk_level: String,
}

/// Prevention measure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreventionMeasure {
    /// Measure description
    pub description: String,
    /// Implementation priority (1=highest)
    pub priority: u32,
    /// Team/owner responsible
    pub owner: String,
    /// Category (automation/documentation/process)
    pub category: String,
}

/// Complete CI failure mode runbook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureRunbook {
    /// Unique failure mode identifier
    pub id: String,
    /// Descriptive name
    pub name: String,
    /// Detailed description of the failure
    pub description: String,
    /// Severity level
    pub severity: SeverityLevel,
    /// Symptoms to look for
    pub symptoms: Vec<String>,
    /// Impact description
    pub impact: String,
    /// Root causes
    pub root_causes: Vec<String>,
    /// Diagnosis procedure
    pub diagnosis: Vec<DiagnosisStep>,
    /// Possible resolutions
    pub resolutions: Vec<ResolutionAction>,
    /// Prevention measures
    pub prevention: Vec<PreventionMeasure>,
    /// Related documentation links
    pub related_docs: Vec<String>,
}

impl FailureRunbook {
    /// Create a new failure runbook
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            severity: SeverityLevel::Error,
            symptoms: Vec::new(),
            impact: String::new(),
            root_causes: Vec::new(),
            diagnosis: Vec::new(),
            resolutions: Vec::new(),
            prevention: Vec::new(),
            related_docs: Vec::new(),
        }
    }

    /// Add a symptom
    pub fn add_symptom(mut self, symptom: impl Into<String>) -> Self {
        self.symptoms.push(symptom.into());
        self
    }

    /// Add root cause
    pub fn add_root_cause(mut self, cause: impl Into<String>) -> Self {
        self.root_causes.push(cause.into());
        self
    }

    /// Add diagnosis step
    pub fn add_diagnosis(mut self, step: DiagnosisStep) -> Self {
        self.diagnosis.push(step);
        self
    }

    /// Add resolution action
    pub fn add_resolution(mut self, resolution: ResolutionAction) -> Self {
        self.resolutions.push(resolution);
        self
    }

    /// Add prevention measure
    pub fn add_prevention(mut self, measure: PreventionMeasure) -> Self {
        self.prevention.push(measure);
        self
    }

    /// Add related documentation link
    pub fn add_doc(mut self, url: impl Into<String>) -> Self {
        self.related_docs.push(url.into());
        self
    }
}

/// Failure runbook collection
pub struct RunbookLibrary {
    runbooks: HashMap<String, FailureRunbook>,
}

impl RunbookLibrary {
    /// Create new runbook library
    pub fn new() -> Self {
        Self {
            runbooks: HashMap::new(),
        }
    }

    /// Create standard library with top 5 failure modes
    pub fn standard() -> Self {
        let mut lib = Self::new();

        // Failure Mode 1: Bazel cache miss/corruption
        lib.add_runbook(
            "bazel_cache_miss",
            create_bazel_cache_runbook(),
        );

        // Failure Mode 2: Clippy lint failures
        lib.add_runbook(
            "clippy_lint_failures",
            create_clippy_runbook(),
        );

        // Failure Mode 3: Test timeout on QEMU
        lib.add_runbook(
            "test_timeout_qemu",
            create_test_timeout_runbook(),
        );

        // Failure Mode 4: Cross-compilation errors
        lib.add_runbook(
            "cross_compilation_error",
            create_cross_compilation_runbook(),
        );

        // Failure Mode 5: Dependency resolution
        lib.add_runbook(
            "dependency_resolution",
            create_dependency_resolution_runbook(),
        );

        lib
    }

    /// Add runbook to library
    pub fn add_runbook(&mut self, id: impl Into<String>, runbook: FailureRunbook) {
        self.runbooks.insert(id.into(), runbook);
    }

    /// Get runbook by ID
    pub fn get(&self, id: &str) -> Option<&FailureRunbook> {
        self.runbooks.get(id)
    }

    /// List all runbooks
    pub fn list(&self) -> Vec<&FailureRunbook> {
        self.runbooks.values().collect()
    }

    /// Export as JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.runbooks)
    }
}

impl Default for RunbookLibrary {
    fn default() -> Self {
        Self::standard()
    }
}

/// Create Bazel cache failure runbook
fn create_bazel_cache_runbook() -> FailureRunbook {
    FailureRunbook::new("bazel_cache_miss", "Bazel Cache Miss/Corruption")
        .description(
            "Bazel build cache is missing entries or corrupted, causing rebuild from source \
             or build failures. This typically manifests as unexpected full rebuilds or \
             cryptic cache validation errors."
        )
        .add_symptom("Build takes 3-5x longer than expected (full rebuild instead of incremental)")
        .add_symptom("Error: 'Failed to load cached file'")
        .add_symptom("Checksum mismatch errors in .bazel cache")
        .add_symptom("Intermittent build failures on same inputs")
        .add_root_cause("Disk corruption or filesystem issues")
        .add_root_cause("Concurrent cache access from multiple processes")
        .add_root_cause("Cache backend (remote cache) is unavailable")
        .add_root_cause("Stale or incompatible cache format version")
        .add_root_cause("Out-of-disk-space causing incomplete cache writes")
        .add_diagnosis(DiagnosisStep {
            step: 1,
            action: "bazel info execution_root".to_string(),
            expected_output: "Path to execution root directory".to_string(),
            check: "Note the execution root path".to_string(),
        })
        .add_diagnosis(DiagnosisStep {
            step: 2,
            action: "ls -lah ~/.cache/bazel".to_string(),
            expected_output: "Directory listing with cache files".to_string(),
            check: "Look for cache corruption signs or unusual file sizes".to_string(),
        })
        .add_diagnosis(DiagnosisStep {
            step: 3,
            action: "bazel clean --expunge".to_string(),
            expected_output: "Cache cleaned successfully".to_string(),
            check: "Verify no errors during cache expunge".to_string(),
        })
        .add_diagnosis(DiagnosisStep {
            step: 4,
            action: "df -h /".to_string(),
            expected_output: "Disk usage statistics".to_string(),
            check: "Ensure sufficient free disk space (>20% recommended)".to_string(),
        })
        .add_resolution(ResolutionAction {
            name: "Clear local cache".to_string(),
            steps: vec![
                "Run: bazel clean --expunge".to_string(),
                "Verify cache is gone: ls -la ~/.cache/bazel (should not exist)".to_string(),
                "Run: bazel build //... (full rebuild)".to_string(),
            ],
            estimated_time_minutes: 15,
            risk_level: "low".to_string(),
        })
        .add_resolution(ResolutionAction {
            name: "Invalidate remote cache".to_string(),
            steps: vec![
                "Run: bazel clean --expunge".to_string(),
                "bazel sync (regenerate local cache from remote)".to_string(),
                "Run: bazel build //...".to_string(),
            ],
            estimated_time_minutes: 30,
            risk_level: "low".to_string(),
        })
        .add_resolution(ResolutionAction {
            name: "Verify disk health".to_string(),
            steps: vec![
                "Run: df -h (check disk space, need >20% free)".to_string(),
                "Run: fsck or similar disk check (if permissions allow)".to_string(),
                "Clear temporary files if needed".to_string(),
            ],
            estimated_time_minutes: 10,
            risk_level: "medium".to_string(),
        })
        .add_prevention(PreventionMeasure {
            description: "Implement cache integrity checks in CI pipeline".to_string(),
            priority: 1,
            owner: "DevOps".to_string(),
            category: "automation".to_string(),
        })
        .add_prevention(PreventionMeasure {
            description: "Monitor cache hit ratio and alert on anomalies".to_string(),
            priority: 2,
            owner: "DevOps".to_string(),
            category: "automation".to_string(),
        })
        .add_prevention(PreventionMeasure {
            description: "Document cache backend health checks".to_string(),
            priority: 3,
            owner: "Documentation".to_string(),
            category: "documentation".to_string(),
        })
        .add_doc("https://bazel.build/run/bazel-caching")
        .add_doc("https://bazel.build/docs/remote-caching")
}

/// Create Clippy lint failure runbook
fn create_clippy_runbook() -> FailureRunbook {
    FailureRunbook::new("clippy_lint_failures", "Clippy Lint Failures")
        .description(
            "Clippy linting fails due to code quality violations. This is a CI gate that \
             prevents merge until lint issues are addressed. Common causes include \
             performance anti-patterns, code clarity issues, and dependency problems."
        )
        .add_symptom("CI pipeline fails with 'cargo clippy -- -D warnings'")
        .add_symptom("Error messages reference clippy lint codes (e.g., clippy::needless_borrow)")
        .add_symptom("Warnings treated as errors")
        .add_symptom("Issue persists across clean builds")
        .add_root_cause("Code violates Rust best practices")
        .add_root_cause("External dependency has lint issues")
        .add_root_cause("Clippy version mismatch with CI environment")
        .add_root_cause("Feature flag combinations trigger new lints")
        .add_diagnosis(DiagnosisStep {
            step: 1,
            action: "cargo clippy --all-targets 2>&1 | head -20".to_string(),
            expected_output: "List of clippy violations".to_string(),
            check: "Identify specific lint codes and file locations".to_string(),
        })
        .add_diagnosis(DiagnosisStep {
            step: 2,
            action: "cargo clippy --explain <LINT_CODE>".to_string(),
            expected_output: "Detailed lint explanation".to_string(),
            check: "Understand why lint is triggered and what to fix".to_string(),
        })
        .add_diagnosis(DiagnosisStep {
            step: 3,
            action: "rustc --version && cargo clippy --version".to_string(),
            expected_output: "Version information".to_string(),
            check: "Compare with CI environment versions".to_string(),
        })
        .add_resolution(ResolutionAction {
            name: "Fix underlying code issue".to_string(),
            steps: vec![
                "Review clippy output and identify problematic patterns".to_string(),
                "Refactor code to follow Rust best practices".to_string(),
                "Test locally: cargo clippy --all-targets -- -D warnings".to_string(),
            ],
            estimated_time_minutes: 20,
            risk_level: "low".to_string(),
        })
        .add_resolution(ResolutionAction {
            name: "Add targeted lint allow directive".to_string(),
            steps: vec![
                "Add #[allow(clippy::SPECIFIC_LINT)] if suppression is justified".to_string(),
                "Add comment explaining why lint is suppressed".to_string(),
                "Get code review approval for suppression".to_string(),
            ],
            estimated_time_minutes: 10,
            risk_level: "medium".to_string(),
        })
        .add_prevention(PreventionMeasure {
            description: "Run clippy in pre-commit hook".to_string(),
            priority: 1,
            owner: "DevOps".to_string(),
            category: "automation".to_string(),
        })
        .add_prevention(PreventionMeasure {
            description: "Document clippy configuration in CONTRIBUTING.md".to_string(),
            priority: 2,
            owner: "Documentation".to_string(),
            category: "documentation".to_string(),
        })
        .add_doc("https://doc.rust-lang.org/clippy/")
        .add_doc("https://github.com/rust-lang/rust-clippy")
}

/// Create test timeout runbook
fn create_test_timeout_runbook() -> FailureRunbook {
    FailureRunbook::new("test_timeout_qemu", "Test Timeout on QEMU")
        .description(
            "Tests fail due to timeout when running under QEMU emulation. QEMU runs at \
             reduced performance compared to native execution, causing timing-sensitive \
             tests to exceed configured timeout thresholds."
        )
        .add_symptom("Test panics with 'timeout: the monitored command took too long'")
        .add_symptom("Test passes locally but fails in CI on aarch64")
        .add_symptom("Random intermittent test failures under QEMU")
        .add_symptom("Performance-sensitive tests fail unexpectedly")
        .add_root_cause("QEMU performance is 5-20x slower than native hardware")
        .add_root_cause("Test timeout is set for native performance baseline")
        .add_root_cause("Resource contention in CI environment")
        .add_root_cause("Unoptimized busy-wait loops")
        .add_diagnosis(DiagnosisStep {
            step: 1,
            action: "grep -r 'timeout' test.rs".to_string(),
            expected_output: "List of timeout configurations".to_string(),
            check: "Find hardcoded timeout values".to_string(),
        })
        .add_diagnosis(DiagnosisStep {
            step: 2,
            action: "uname -m && cat /proc/cpuinfo".to_string(),
            expected_output: "CPU architecture and flags".to_string(),
            check: "Determine if running under QEMU".to_string(),
        })
        .add_diagnosis(DiagnosisStep {
            step: 3,
            action: "time cargo test -- --nocapture".to_string(),
            expected_output: "Test execution time".to_string(),
            check: "Measure actual test duration vs timeout".to_string(),
        })
        .add_resolution(ResolutionAction {
            name: "Increase test timeout".to_string(),
            steps: vec![
                "Identify timeout constant or configuration".to_string(),
                "Multiply timeout by 10x for QEMU-safe value".to_string(),
                "Test locally with timeout: timeout 60s cargo test".to_string(),
            ],
            estimated_time_minutes: 10,
            risk_level: "low".to_string(),
        })
        .add_resolution(ResolutionAction {
            name: "Skip test under QEMU".to_string(),
            steps: vec![
                "Add conditional: #[cfg_attr(all(test, target_arch=\"aarch64\"), ignore)]".to_string(),
                "Or use: if cfg!(target_arch = \"aarch64\") { return; }".to_string(),
                "Add comment explaining QEMU incompatibility".to_string(),
            ],
            estimated_time_minutes: 10,
            risk_level: "medium".to_string(),
        })
        .add_prevention(PreventionMeasure {
            description: "Use relative timeouts based on QEMU detection".to_string(),
            priority: 1,
            owner: "Testing".to_string(),
            category: "automation".to_string(),
        })
        .add_prevention(PreventionMeasure {
            description: "Document QEMU performance characteristics".to_string(),
            priority: 2,
            owner: "Documentation".to_string(),
            category: "documentation".to_string(),
        })
}

/// Create cross-compilation error runbook
fn create_cross_compilation_runbook() -> FailureRunbook {
    FailureRunbook::new("cross_compilation_error", "Cross-Compilation Errors")
        .description(
            "Build fails when cross-compiling for non-native targets (e.g., aarch64 on x86_64). \
             Common issues include linker errors, dependency resolution failures, and \
             architecture-specific code problems."
        )
        .add_symptom("Linker error: 'unknown architecture' or 'cannot find crt1.o'")
        .add_symptom("Error: 'native library not found' for platform-specific dependency")
        .add_symptom("Build succeeds for native target but fails for cross-target")
        .add_symptom("Feature activation fails for target-specific code")
        .add_root_cause("Missing cross-compilation toolchain for target")
        .add_root_cause("Dependency not available for target architecture")
        .add_root_cause("Incorrect linkage flags for target platform")
        .add_root_cause("Architecture-specific code not gated by #[cfg()]")
        .add_diagnosis(DiagnosisStep {
            step: 1,
            action: "rustup target list | grep installed".to_string(),
            expected_output: "Installed target triples".to_string(),
            check: "Verify target (aarch64-unknown-linux-gnu, etc.) is installed".to_string(),
        })
        .add_diagnosis(DiagnosisStep {
            step: 2,
            action: "cargo build --target aarch64-unknown-linux-gnu 2>&1 | head -50".to_string(),
            expected_output: "Detailed error message".to_string(),
            check: "Identify specific linkage or dependency issue".to_string(),
        })
        .add_diagnosis(DiagnosisStep {
            step: 3,
            action: "grep -r 'cfg(target' src/".to_string(),
            expected_output: "Architecture-specific code".to_string(),
            check: "Verify proper #[cfg()] guards around platform-specific code".to_string(),
        })
        .add_resolution(ResolutionAction {
            name: "Install cross-compilation target".to_string(),
            steps: vec![
                "Run: rustup target add aarch64-unknown-linux-gnu".to_string(),
                "Install system dependencies: apt-get install gcc-aarch64-linux-gnu".to_string(),
                "Retry build: cargo build --target aarch64-unknown-linux-gnu".to_string(),
            ],
            estimated_time_minutes: 15,
            risk_level: "low".to_string(),
        })
        .add_resolution(ResolutionAction {
            name: "Fix dependency features".to_string(),
            steps: vec![
                "Review Cargo.toml for target-specific dependencies".to_string(),
                "Add [target.'cfg(target_arch = \"aarch64\")'.dependencies] section".to_string(),
                "Use conditional features: features = [\"x86_only\"] with appropriate cfg".to_string(),
            ],
            estimated_time_minutes: 20,
            risk_level: "medium".to_string(),
        })
        .add_prevention(PreventionMeasure {
            description: "Test cross-compilation in CI for all target platforms".to_string(),
            priority: 1,
            owner: "DevOps".to_string(),
            category: "automation".to_string(),
        })
        .add_prevention(PreventionMeasure {
            description: "Use #[cfg()] guards for all platform-specific code".to_string(),
            priority: 2,
            owner: "Architecture".to_string(),
            category: "process".to_string(),
        })
}

/// Create dependency resolution runbook
fn create_dependency_resolution_runbook() -> FailureRunbook {
    FailureRunbook::new("dependency_resolution", "Dependency Resolution Failures")
        .description(
            "Cargo fails to resolve dependencies due to version conflicts, missing registries, \
             or network issues. This blocks the entire build pipeline."
        )
        .add_symptom("Error: 'failed to resolve: unresolved import'")
        .add_symptom("Error: 'no default toolchain configured'")
        .add_symptom("Cargo.lock conflicts between branches")
        .add_symptom("Network timeout fetching crate metadata")
        .add_root_cause("Incompatible dependency versions")
        .add_root_cause("Crate registry is unavailable or slow")
        .add_root_cause("Local Cargo.lock is stale or corrupted")
        .add_root_cause("Git dependency not available or branch deleted")
        .add_diagnosis(DiagnosisStep {
            step: 1,
            action: "cargo update --dry-run".to_string(),
            expected_output: "List of potential updates".to_string(),
            check: "Identify if version conflicts are suggested".to_string(),
        })
        .add_diagnosis(DiagnosisStep {
            step: 2,
            action: "cargo tree --duplicates".to_string(),
            expected_output: "Duplicate dependencies in tree".to_string(),
            check: "Find conflicting versions of same crate".to_string(),
        })
        .add_diagnosis(DiagnosisStep {
            step: 3,
            action: "curl -s https://crates.io/api/v1/crates/serde | jq".to_string(),
            expected_output: "Crate metadata".to_string(),
            check: "Verify registry is responding".to_string(),
        })
        .add_resolution(ResolutionAction {
            name: "Update and lock dependencies".to_string(),
            steps: vec![
                "Run: cargo update".to_string(),
                "Run: cargo check to validate".to_string(),
                "Commit updated Cargo.lock".to_string(),
            ],
            estimated_time_minutes: 10,
            risk_level: "low".to_string(),
        })
        .add_resolution(ResolutionAction {
            name: "Resolve version conflicts".to_string(),
            steps: vec![
                "Review conflict error from cargo tree".to_string(),
                "Update Cargo.toml with compatible version specifiers".to_string(),
                "Use SemVer-compatible versions: ^X.Y.Z".to_string(),
                "Test: cargo update && cargo check".to_string(),
            ],
            estimated_time_minutes: 20,
            risk_level: "medium".to_string(),
        })
        .add_prevention(PreventionMeasure {
            description: "Run 'cargo outdated' in CI to detect stale dependencies".to_string(),
            priority: 1,
            owner: "DevOps".to_string(),
            category: "automation".to_string(),
        })
        .add_prevention(PreventionMeasure {
            description: "Document dependency update strategy in CONTRIBUTING.md".to_string(),
            priority: 2,
            owner: "Documentation".to_string(),
            category: "documentation".to_string(),
        })
        .add_doc("https://doc.rust-lang.org/cargo/guide/dependencies.html")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runbook_creation() {
        let runbook = FailureRunbook::new("test", "Test Failure");
        assert_eq!(runbook.id, "test");
        assert_eq!(runbook.symptoms.len(), 0);
    }

    #[test]
    fn test_runbook_with_properties() {
        let runbook = FailureRunbook::new("test", "Test")
            .add_symptom("symptom1")
            .add_symptom("symptom2")
            .add_root_cause("cause1");

        assert_eq!(runbook.symptoms.len(), 2);
        assert_eq!(runbook.root_causes.len(), 1);
    }

    #[test]
    fn test_runbook_library() {
        let lib = RunbookLibrary::standard();
        assert!(lib.get("bazel_cache_miss").is_some());
        assert!(lib.get("clippy_lint_failures").is_some());
        assert_eq!(lib.list().len(), 5);
    }

    #[test]
    fn test_json_export() {
        let lib = RunbookLibrary::standard();
        let json = lib.to_json();
        assert!(json.is_ok());
        let json_str = json.unwrap();
        assert!(json_str.contains("bazel_cache_miss"));
    }
}
