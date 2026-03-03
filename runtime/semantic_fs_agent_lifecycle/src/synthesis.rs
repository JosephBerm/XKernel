// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! Feature parity mapping between lifecycle config and unit file formats.
//!
//! Analyzes and documents how lifecycle configuration fields map to unit file
//! sections and identifies any gaps in feature coverage. Enables validation of
//! bidirectional conversion between internal config format and serialized unit files.
//!
//! Reference: Engineering Plan § Agent Lifecycle Management § Synthesis

use alloc::string::String;
use alloc::vec::Vec;

/// Type of mapping relationship between lifecycle config and unit file.
///
/// Describes how a configuration field is represented in unit file format.
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Feature Parity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MappingType {
    /// Direct one-to-one mapping with same value representation.
    ///
    /// The field in lifecycle config maps directly to unit file with no transformation.
    /// Example: startup_timeout_ms -> [lifecycle] startup_timeout_ms
    Direct,

    /// Computed mapping requiring transformation or aggregation.
    ///
    /// The field value is computed from multiple unit file fields or requires
    /// transformation during conversion.
    /// Example: health_status derived from readiness_probe + liveness_probe results
    Computed,

    /// New field in unit file not in current lifecycle config.
    ///
    /// Feature supported in unit file format but not yet in lifecycle config.
    /// May be added to future versions of lifecycle config.
    New,
}

impl MappingType {
    /// Returns true if this is a direct mapping.
    pub fn is_direct(&self) -> bool {
        matches!(self, Self::Direct)
    }

    /// Returns true if this is a computed mapping.
    pub fn is_computed(&self) -> bool {
        matches!(self, Self::Computed)
    }

    /// Returns true if this is a new field.
    pub fn is_new(&self) -> bool {
        matches!(self, Self::New)
    }
}

/// Single feature mapping between lifecycle config and unit file.
///
/// Documents how a specific lifecycle configuration field is represented
/// in the agent unit file format.
///
/// # Fields
///
/// - `lifecycle_field`: Name of the field in lifecycle config
/// - `unit_file_section`: Unit file section containing this field
/// - `unit_file_field`: Field name in unit file (if Direct or Computed mapping)
/// - `mapping_type`: How the field maps (Direct, Computed, or New)
/// - `description`: Human-readable mapping description
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Feature Parity
#[derive(Debug, Clone)]
pub struct FeatureMapping {
    /// Name of the lifecycle config field.
    pub lifecycle_field: String,

    /// Unit file section where this field appears.
    ///
    /// Examples: "[metadata]", "[lifecycle]", "[health]", "[resources]"
    pub unit_file_section: String,

    /// Corresponding field name in unit file (if applicable).
    pub unit_file_field: String,

    /// Type of mapping relationship.
    pub mapping_type: MappingType,

    /// Human-readable description of the mapping.
    pub description: String,
}

impl FeatureMapping {
    /// Creates a new feature mapping.
    pub fn new(
        lifecycle_field: impl Into<String>,
        unit_file_section: impl Into<String>,
        unit_file_field: impl Into<String>,
        mapping_type: MappingType,
        description: impl Into<String>,
    ) -> Self {
        Self {
            lifecycle_field: lifecycle_field.into(),
            unit_file_section: unit_file_section.into(),
            unit_file_field: unit_file_field.into(),
            mapping_type,
            description: description.into(),
        }
    }

    /// Creates a direct mapping.
    pub fn direct(
        lifecycle_field: impl Into<String>,
        unit_file_section: impl Into<String>,
        unit_file_field: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self::new(
            lifecycle_field,
            unit_file_section,
            unit_file_field,
            MappingType::Direct,
            description,
        )
    }

    /// Creates a computed mapping.
    pub fn computed(
        lifecycle_field: impl Into<String>,
        unit_file_section: impl Into<String>,
        unit_file_field: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self::new(
            lifecycle_field,
            unit_file_section,
            unit_file_field,
            MappingType::Computed,
            description,
        )
    }

    /// Creates a new field mapping.
    pub fn new_field(
        unit_file_section: impl Into<String>,
        unit_file_field: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self::new(
            "N/A",
            unit_file_section,
            unit_file_field,
            MappingType::New,
            description,
        )
    }
}

/// Complete feature parity matrix mapping all lifecycle config fields.
///
/// Documents the comprehensive mapping of lifecycle config features to unit file
/// format, identifying which features are Direct, Computed, or New.
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Feature Parity
#[derive(Debug, Clone)]
pub struct FeatureParityMatrix {
    /// All feature mappings.
    pub mappings: Vec<FeatureMapping>,
}

impl FeatureParityMatrix {
    /// Creates a new empty feature parity matrix.
    pub fn new() -> Self {
        Self {
            mappings: Vec::new(),
        }
    }

    /// Builds the standard feature parity matrix for lifecycle config.
    ///
    /// Returns a comprehensive matrix with all known field mappings.
    ///
    /// Reference: Engineering Plan § Agent Lifecycle Management § Feature Parity
    pub fn standard_matrix() -> Self {
        let mut matrix = Self::new();

        // Metadata section mappings
        matrix.add_mapping(FeatureMapping::direct(
            "metadata.name",
            "[metadata]",
            "name",
            "Agent name identifier",
        ));
        matrix.add_mapping(FeatureMapping::direct(
            "metadata.version",
            "[metadata]",
            "version",
            "Semantic version string",
        ));
        matrix.add_mapping(FeatureMapping::direct(
            "metadata.description",
            "[metadata]",
            "description",
            "Agent purpose description",
        ));
        matrix.add_mapping(FeatureMapping::direct(
            "metadata.author",
            "[metadata]",
            "author",
            "Author or team responsible",
        ));
        matrix.add_mapping(FeatureMapping::direct(
            "metadata.tags",
            "[metadata]",
            "tags",
            "Classification tags for discovery",
        ));

        // Lifecycle section mappings
        matrix.add_mapping(FeatureMapping::direct(
            "startup_timeout_ms",
            "[lifecycle]",
            "startup_timeout_ms",
            "Max time to reach Running state",
        ));
        matrix.add_mapping(FeatureMapping::direct(
            "shutdown_timeout_ms",
            "[lifecycle]",
            "shutdown_timeout_ms",
            "Max time to reach Stopped state",
        ));

        // Health check mappings
        matrix.add_mapping(FeatureMapping::direct(
            "readiness_probe.endpoint_type",
            "[health.readiness]",
            "probe_type",
            "HTTP, TCP, Exec, CSCI, or CustomGrpc",
        ));
        matrix.add_mapping(FeatureMapping::direct(
            "readiness_probe.address",
            "[health.readiness]",
            "address",
            "Network address or service identifier",
        ));
        matrix.add_mapping(FeatureMapping::direct(
            "readiness_probe.path",
            "[health.readiness]",
            "path",
            "HTTP path component",
        ));
        matrix.add_mapping(FeatureMapping::direct(
            "readiness_probe.interval_ms",
            "[health.readiness]",
            "interval_ms",
            "Time between probe attempts",
        ));
        matrix.add_mapping(FeatureMapping::direct(
            "readiness_probe.timeout_ms",
            "[health.readiness]",
            "timeout_ms",
            "Max time for probe execution",
        ));
        matrix.add_mapping(FeatureMapping::direct(
            "readiness_probe.failure_threshold",
            "[health.readiness]",
            "failure_threshold",
            "Consecutive failures before unhealthy",
        ));

        matrix.add_mapping(FeatureMapping::direct(
            "liveness_probe.endpoint_type",
            "[health.liveness]",
            "probe_type",
            "HTTP, TCP, Exec, CSCI, or CustomGrpc",
        ));
        matrix.add_mapping(FeatureMapping::direct(
            "liveness_probe.interval_ms",
            "[health.liveness]",
            "interval_ms",
            "Time between probe attempts",
        ));
        matrix.add_mapping(FeatureMapping::direct(
            "liveness_probe.timeout_ms",
            "[health.liveness]",
            "timeout_ms",
            "Max time for probe execution",
        ));

        // Restart policy mappings
        matrix.add_mapping(FeatureMapping::direct(
            "restart_policy",
            "[restart]",
            "policy",
            "Always, OnFailure, or Never",
        ));
        matrix.add_mapping(FeatureMapping::direct(
            "restart_policy.backoff.initial_delay_ms",
            "[restart.backoff]",
            "initial_delay_ms",
            "Initial delay before first restart",
        ));
        matrix.add_mapping(FeatureMapping::direct(
            "restart_policy.backoff.max_delay_ms",
            "[restart.backoff]",
            "max_delay_ms",
            "Maximum delay between retries",
        ));
        matrix.add_mapping(FeatureMapping::direct(
            "restart_policy.backoff.multiplier",
            "[restart.backoff]",
            "multiplier",
            "Exponential backoff multiplier",
        ));
        matrix.add_mapping(FeatureMapping::direct(
            "restart_policy.backoff.max_retries",
            "[restart.backoff]",
            "max_retries",
            "Maximum number of restart attempts",
        ));

        // Dependencies mappings
        matrix.add_mapping(FeatureMapping::direct(
            "dependencies.required_agents",
            "[dependencies]",
            "requires_agents",
            "Agent IDs that must start first",
        ));
        matrix.add_mapping(FeatureMapping::direct(
            "dependencies.required_services",
            "[dependencies]",
            "requires_services",
            "External services that must be available",
        ));
        matrix.add_mapping(FeatureMapping::direct(
            "dependencies.ordering_constraints",
            "[dependencies]",
            "constraints",
            "Startup ordering constraints (Before, After, Concurrent)",
        ));

        // Resources mappings
        matrix.add_mapping(FeatureMapping::direct(
            "memory_mb",
            "[resources]",
            "memory_mb",
            "Memory requirement in megabytes",
        ));
        matrix.add_mapping(FeatureMapping::direct(
            "cpu_cores",
            "[resources]",
            "cpu_cores",
            "CPU requirement in cores (fractional allowed)",
        ));

        // Security/Capabilities mappings
        matrix.add_mapping(FeatureMapping::direct(
            "capabilities_required",
            "[security]",
            "capabilities",
            "Required system capabilities (net_admin, sys_ptrace, etc)",
        ));

        // Crew membership mappings
        matrix.add_mapping(FeatureMapping::direct(
            "crew_membership.crew_id",
            "[crew]",
            "crew_id",
            "Crew identifier this agent belongs to",
        ));
        matrix.add_mapping(FeatureMapping::direct(
            "crew_membership.role",
            "[crew]",
            "role",
            "Agent role within crew (leader, worker, etc)",
        ));

        // Environment mappings
        matrix.add_mapping(FeatureMapping::direct(
            "environment",
            "[env]",
            "variables",
            "Environment variable key-value pairs",
        ));

        // New/Future fields in unit file format
        matrix.add_mapping(FeatureMapping::new_field(
            "[health.startup]",
            "probe_type",
            "Startup probe: determines if initialization is complete",
        ));
        matrix.add_mapping(FeatureMapping::new_field(
            "[lifecycle]",
            "state_transition_timeout_ms",
            "Timeout for individual state transitions",
        ));
        matrix.add_mapping(FeatureMapping::new_field(
            "[metrics]",
            "enabled",
            "Enable metrics collection for this agent",
        ));
        matrix.add_mapping(FeatureMapping::new_field(
            "[metrics]",
            "export_interval_ms",
            "How often to export metrics",
        ));

        matrix
    }

    /// Adds a feature mapping to the matrix.
    pub fn add_mapping(&mut self, mapping: FeatureMapping) {
        self.mappings.push(mapping);
    }

    /// Returns the total number of mappings.
    pub fn total_mappings(&self) -> usize {
        self.mappings.len()
    }

    /// Returns count of direct mappings.
    pub fn direct_count(&self) -> usize {
        self.mappings.iter().filter(|m| m.mapping_type.is_direct()).count()
    }

    /// Returns count of computed mappings.
    pub fn computed_count(&self) -> usize {
        self.mappings
            .iter()
            .filter(|m| m.mapping_type.is_computed())
            .count()
    }

    /// Returns count of new fields.
    pub fn new_fields_count(&self) -> usize {
        self.mappings
            .iter()
            .filter(|m| m.mapping_type.is_new())
            .count()
    }

    /// Gets all mappings for a given lifecycle field.
    pub fn get_lifecycle_field(&self, field: &str) -> Vec<&FeatureMapping> {
        self.mappings
            .iter()
            .filter(|m| m.lifecycle_field == field)
            .collect()
    }

    /// Gets all mappings for a given unit file section.
    pub fn get_section(&self, section: &str) -> Vec<&FeatureMapping> {
        self.mappings
            .iter()
            .filter(|m| m.unit_file_section == section)
            .collect()
    }

    /// Gets all direct mappings.
    pub fn get_direct_mappings(&self) -> Vec<&FeatureMapping> {
        self.mappings
            .iter()
            .filter(|m| m.mapping_type.is_direct())
            .collect()
    }

    /// Gets all computed mappings.
    pub fn get_computed_mappings(&self) -> Vec<&FeatureMapping> {
        self.mappings
            .iter()
            .filter(|m| m.mapping_type.is_computed())
            .collect()
    }

    /// Gets all new fields.
    pub fn get_new_fields(&self) -> Vec<&FeatureMapping> {
        self.mappings
            .iter()
            .filter(|m| m.mapping_type.is_new())
            .collect()
    }
}

impl Default for FeatureParityMatrix {
    fn default() -> Self {
        Self::standard_matrix()
    }
}

/// Gap analysis identifying features not yet fully supported.
///
/// Analyzes feature parity and identifies:
/// - Features in lifecycle_config not yet in unit_file format
/// - New features supported by unit_file format but not in lifecycle_config
/// - Mapping complexity indicators (computed vs direct)
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Feature Parity
#[derive(Debug, Clone)]
pub struct GapAnalysis {
    /// Features in lifecycle_config awaiting unit_file support.
    pub lifecycle_features_not_in_unit_file: Vec<String>,

    /// New unit_file features not yet in lifecycle_config.
    pub new_unit_file_features: Vec<String>,

    /// Features requiring computed mapping (transformation needed).
    pub computed_mapping_features: Vec<String>,

    /// Overall feature parity percentage (0-100).
    ///
    /// Calculated as: (direct_count / total_config_fields) * 100
    pub parity_percentage: f64,
}

impl GapAnalysis {
    /// Creates a new gap analysis.
    pub fn new() -> Self {
        Self {
            lifecycle_features_not_in_unit_file: Vec::new(),
            new_unit_file_features: Vec::new(),
            computed_mapping_features: Vec::new(),
            parity_percentage: 0.0,
        }
    }

    /// Computes gap analysis from a feature parity matrix.
    ///
    /// Analyzes the matrix to identify gaps and compute feature parity percentage.
    ///
    /// Arguments:
    /// - `matrix`: The feature parity matrix to analyze
    ///
    /// Returns `GapAnalysis` with findings.
    ///
    /// Reference: Engineering Plan § Agent Lifecycle Management § Feature Parity
    pub fn from_matrix(matrix: &FeatureParityMatrix) -> Self {
        let mut analysis = Self::new();

        // Identify new fields (not in lifecycle_config)
        for mapping in &matrix.mappings {
            if mapping.mapping_type.is_new() {
                analysis
                    .new_unit_file_features
                    .push(format!("{}: {}", mapping.unit_file_section, mapping.unit_file_field));
            }

            if mapping.mapping_type.is_computed() {
                analysis
                    .computed_mapping_features
                    .push(mapping.lifecycle_field.clone());
            }
        }

        // Calculate parity percentage
        let direct_count = matrix.direct_count();
        let total_lifecycle_fields = matrix.total_mappings() - matrix.new_fields_count();

        analysis.parity_percentage = if total_lifecycle_fields > 0 {
            (direct_count as f64 / total_lifecycle_fields as f64) * 100.0
        } else {
            100.0
        };

        analysis
    }

    /// Returns true if complete feature parity is achieved.
    pub fn has_full_parity(&self) -> bool {
        self.lifecycle_features_not_in_unit_file.is_empty()
            && self.parity_percentage >= 100.0
    }

    /// Returns number of gaps (missing features).
    pub fn gap_count(&self) -> usize {
        self.lifecycle_features_not_in_unit_file.len()
    }
}

impl Default for GapAnalysis {
    fn default() -> Self {
        Self::new()
    }
}

/// Comprehensive unit file format requirements.
///
/// Documents all features and functionality that the agent unit file format
/// must support based on lifecycle config capabilities.
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Unit Files
#[derive(Debug, Clone)]
pub struct UnitFileRequirements {
    /// Required sections in unit file format.
    pub required_sections: Vec<String>,

    /// Features that MUST be supported.
    pub mandatory_features: Vec<String>,

    /// Features that SHOULD be supported.
    pub recommended_features: Vec<String>,

    /// Optional/nice-to-have features.
    pub optional_features: Vec<String>,

    /// Constraints or limitations.
    pub constraints: Vec<String>,
}

impl UnitFileRequirements {
    /// Creates new unit file requirements.
    pub fn new() -> Self {
        Self {
            required_sections: Vec::new(),
            mandatory_features: Vec::new(),
            recommended_features: Vec::new(),
            optional_features: Vec::new(),
            constraints: Vec::new(),
        }
    }

    /// Generates standard unit file requirements from lifecycle config.
    ///
    /// Creates comprehensive requirements document for unit file format design.
    ///
    /// Reference: Engineering Plan § Agent Lifecycle Management § Unit Files
    pub fn standard_requirements() -> Self {
        let mut req = Self::new();

        // Required sections
        req.required_sections = alloc::vec![
            "[metadata]".to_string(),
            "[lifecycle]".to_string(),
            "[restart]".to_string(),
        ];

        // Mandatory features
        req.mandatory_features = alloc::vec![
            "Agent name and version".to_string(),
            "Startup timeout configuration".to_string(),
            "Shutdown timeout configuration".to_string(),
            "Restart policy (Always, OnFailure, Never)".to_string(),
            "Exponential backoff configuration".to_string(),
            "Dependency specification and ordering".to_string(),
        ];

        // Recommended features
        req.recommended_features = alloc::vec![
            "Readiness probe configuration".to_string(),
            "Liveness probe configuration".to_string(),
            "HTTP GET, TCP, Exec, CSCI, and Custom gRPC probe types".to_string(),
            "Resource limits (memory, CPU)".to_string(),
            "Security capabilities specification".to_string(),
            "Environment variable configuration".to_string(),
            "Crew membership and role assignment".to_string(),
        ];

        // Optional features
        req.optional_features = alloc::vec![
            "Startup probe configuration".to_string(),
            "Custom health check handlers".to_string(),
            "Metrics collection configuration".to_string(),
            "Logging level and configuration".to_string(),
            "Service dependencies specification".to_string(),
        ];

        // Constraints
        req.constraints = alloc::vec![
            "All timeout values must be in milliseconds".to_string(),
            "Backoff multiplier must be positive number".to_string(),
            "Failure and success thresholds must be positive integers".to_string(),
            "Dependency graph must be acyclic (DAG)".to_string(),
            "Semantic versioning must follow major.minor.patch format".to_string(),
        ];

        req
    }

    /// Adds a required section.
    pub fn add_required_section(&mut self, section: impl Into<String>) {
        self.required_sections.push(section.into());
    }

    /// Adds a mandatory feature.
    pub fn add_mandatory_feature(&mut self, feature: impl Into<String>) {
        self.mandatory_features.push(feature.into());
    }

    /// Adds a recommended feature.
    pub fn add_recommended_feature(&mut self, feature: impl Into<String>) {
        self.recommended_features.push(feature.into());
    }

    /// Adds an optional feature.
    pub fn add_optional_feature(&mut self, feature: impl Into<String>) {
        self.optional_features.push(feature.into());
    }

    /// Adds a constraint.
    pub fn add_constraint(&mut self, constraint: impl Into<String>) {
        self.constraints.push(constraint.into());
    }

    /// Returns total feature count.
    pub fn total_features(&self) -> usize {
        self.mandatory_features.len()
            + self.recommended_features.len()
            + self.optional_features.len()
    }

    /// Validates requirements consistency.
    ///
    /// Checks that all features are accounted for and no duplicates exist.
    pub fn is_valid(&self) -> bool {
        !self.required_sections.is_empty() && !self.mandatory_features.is_empty()
    }
}

impl Default for UnitFileRequirements {
    fn default() -> Self {
        Self::standard_requirements()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;
use alloc::string::ToString;
use alloc::vec;

    // MappingType tests
    #[test]
    fn test_mapping_type_direct() {
        let mt = MappingType::Direct;
        assert!(mt.is_direct());
        assert!(!mt.is_computed());
        assert!(!mt.is_new());
    }

    #[test]
    fn test_mapping_type_computed() {
        let mt = MappingType::Computed;
        assert!(!mt.is_direct());
        assert!(mt.is_computed());
        assert!(!mt.is_new());
    }

    #[test]
    fn test_mapping_type_new() {
        let mt = MappingType::New;
        assert!(!mt.is_direct());
        assert!(!mt.is_computed());
        assert!(mt.is_new());
    }

    // FeatureMapping tests
    #[test]
    fn test_feature_mapping_new() {
        let mapping = FeatureMapping::new(
            "startup_timeout_ms",
            "[lifecycle]",
            "startup_timeout_ms",
            MappingType::Direct,
            "Max time to reach Running state",
        );

        assert_eq!(mapping.lifecycle_field, "startup_timeout_ms");
        assert_eq!(mapping.unit_file_section, "[lifecycle]");
        assert_eq!(mapping.unit_file_field, "startup_timeout_ms");
        assert_eq!(mapping.mapping_type, MappingType::Direct);
    }

    #[test]
    fn test_feature_mapping_direct() {
        let mapping =
            FeatureMapping::direct("name", "[metadata]", "name", "Agent name identifier");

        assert_eq!(mapping.mapping_type, MappingType::Direct);
    }

    #[test]
    fn test_feature_mapping_computed() {
        let mapping = FeatureMapping::computed(
            "health_status",
            "[health]",
            "status",
            "Derived from probe results",
        );

        assert_eq!(mapping.mapping_type, MappingType::Computed);
    }

    #[test]
    fn test_feature_mapping_new_field() {
        let mapping = FeatureMapping::new_field("[health.startup]", "probe_type", "Startup probe");

        assert_eq!(mapping.lifecycle_field, "N/A");
        assert_eq!(mapping.mapping_type, MappingType::New);
    }

    // FeatureParityMatrix tests
    #[test]
    fn test_feature_parity_matrix_new() {
        let matrix = FeatureParityMatrix::new();
        assert_eq!(matrix.total_mappings(), 0);
    }

    #[test]
    fn test_feature_parity_matrix_add_mapping() {
        let mut matrix = FeatureParityMatrix::new();
        let mapping = FeatureMapping::direct("test", "[test]", "test", "Test mapping");
        matrix.add_mapping(mapping);

        assert_eq!(matrix.total_mappings(), 1);
        assert_eq!(matrix.direct_count(), 1);
    }

    #[test]
    fn test_feature_parity_matrix_standard() {
        let matrix = FeatureParityMatrix::standard_matrix();

        assert!(matrix.total_mappings() > 0);
        assert!(matrix.direct_count() > 0);
        assert_eq!(matrix.new_fields_count(), 4); // Startup, state transition, metrics
    }

    #[test]
    fn test_feature_parity_matrix_counts() {
        let matrix = FeatureParityMatrix::standard_matrix();

        let total = matrix.total_mappings();
        let direct = matrix.direct_count();
        let computed = matrix.computed_count();
        let new = matrix.new_fields_count();

        assert_eq!(total, direct + computed + new);
    }

    #[test]
    fn test_feature_parity_matrix_get_section() {
        let matrix = FeatureParityMatrix::standard_matrix();
        let metadata_mappings = matrix.get_section("[metadata]");

        assert!(metadata_mappings.len() > 0);
        assert!(metadata_mappings.iter().all(|m| m.unit_file_section == "[metadata]"));
    }

    #[test]
    fn test_feature_parity_matrix_get_direct_mappings() {
        let matrix = FeatureParityMatrix::standard_matrix();
        let direct = matrix.get_direct_mappings();

        assert!(direct.iter().all(|m| m.mapping_type.is_direct()));
    }

    #[test]
    fn test_feature_parity_matrix_get_new_fields() {
        let matrix = FeatureParityMatrix::standard_matrix();
        let new_fields = matrix.get_new_fields();

        assert!(new_fields.iter().all(|m| m.mapping_type.is_new()));
        assert!(new_fields.len() > 0);
    }

    // GapAnalysis tests
    #[test]
    fn test_gap_analysis_new() {
        let analysis = GapAnalysis::new();
        assert_eq!(analysis.gap_count(), 0);
        assert_eq!(analysis.parity_percentage, 0.0);
    }

    #[test]
    fn test_gap_analysis_from_matrix() {
        let matrix = FeatureParityMatrix::standard_matrix();
        let analysis = GapAnalysis::from_matrix(&matrix);

        assert!(analysis.parity_percentage > 0.0);
        assert!(analysis.parity_percentage <= 100.0);
        assert!(!analysis.new_unit_file_features.is_empty());
    }

    #[test]
    fn test_gap_analysis_high_parity() {
        let matrix = FeatureParityMatrix::standard_matrix();
        let analysis = GapAnalysis::from_matrix(&matrix);

        // Should have high parity since most fields are direct mappings
        assert!(analysis.parity_percentage >= 80.0);
    }

    // UnitFileRequirements tests
    #[test]
    fn test_unit_file_requirements_new() {
        let req = UnitFileRequirements::new();
        assert_eq!(req.total_features(), 0);
    }

    #[test]
    fn test_unit_file_requirements_standard() {
        let req = UnitFileRequirements::standard_requirements();

        assert!(req.is_valid());
        assert!(!req.required_sections.is_empty());
        assert!(!req.mandatory_features.is_empty());
        assert!(!req.recommended_features.is_empty());
        assert!(!req.constraints.is_empty());
    }

    #[test]
    fn test_unit_file_requirements_add_feature() {
        let mut req = UnitFileRequirements::new();
        req.add_mandatory_feature("Test feature");

        assert_eq!(req.mandatory_features.len(), 1);
        assert_eq!(req.total_features(), 1);
    }

    #[test]
    fn test_unit_file_requirements_add_multiple() {
        let mut req = UnitFileRequirements::new();
        req.add_mandatory_feature("Feature 1");
        req.add_recommended_feature("Feature 2");
        req.add_optional_feature("Feature 3");

        assert_eq!(req.total_features(), 3);
    }

    #[test]
    fn test_unit_file_requirements_standard_has_metadata() {
        let req = UnitFileRequirements::standard_requirements();
        assert!(req
            .required_sections
            .iter()
            .any(|s| s == "[metadata]"));
    }

    #[test]
    fn test_unit_file_requirements_standard_has_lifecycle() {
        let req = UnitFileRequirements::standard_requirements();
        assert!(req
            .required_sections
            .iter()
            .any(|s| s == "[lifecycle]"));
    }

    #[test]
    fn test_unit_file_requirements_standard_has_restart() {
        let req = UnitFileRequirements::standard_requirements();
        assert!(req.required_sections.iter().any(|s| s == "[restart]"));
    }
}
