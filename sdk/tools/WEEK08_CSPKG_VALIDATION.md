# Week 8 Deliverable: cs-pkg Validation & Registry API (Phase 1)

**XKernal — Engineer 10: Tooling, Packaging & Documentation**
**Week 8 Objective:** Refine cs-pkg design. Implement package validation system. Design registry API endpoints. Create tool and adapter example packages.

---

## 1. Package Validation Library (cs-pkg-validate crate)

### Core Validation Functions

#### Manifest Validation

```rust
// src/lib.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("Invalid CSCI version: {0}")]
    InvalidCSCIVersion(String),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Invalid capability: {0}")]
    InvalidCapability(String),

    #[error("Cost metadata missing")]
    MissingCostMetadata,

    #[error("Invalid package structure: {0}")]
    InvalidStructure(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] toml::de::Error),
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PackageManifest {
    pub name: String,
    pub version: String,
    pub csci_version: String,
    pub description: String,
    pub author: String,
    pub license: String,
    pub capabilities: Vec<String>,
    pub cost_metadata: CostMetadata,
    pub repository: Option<String>,
    pub homepage: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CostMetadata {
    pub estimated_compute_tokens: u32,
    pub estimated_context_tokens: u32,
    pub tier: String, // "free", "standard", "premium"
}

pub fn validate_manifest(manifest: &PackageManifest) -> Result<(), ValidationError> {
    // CSCI version compatibility check
    if !is_valid_csci_version(&manifest.csci_version) {
        return Err(ValidationError::InvalidCSCIVersion(
            manifest.csci_version.clone(),
        ));
    }

    // Name validation
    if manifest.name.is_empty() || !is_valid_package_name(&manifest.name) {
        return Err(ValidationError::MissingField("name".to_string()));
    }

    // Version validation (semantic versioning)
    if !is_valid_semver(&manifest.version) {
        return Err(ValidationError::MissingField("version".to_string()));
    }

    // Description validation
    if manifest.description.is_empty() || manifest.description.len() > 500 {
        return Err(ValidationError::MissingField(
            "description (must be 1-500 chars)".to_string(),
        ));
    }

    // Capability validation
    if manifest.capabilities.is_empty() {
        return Err(ValidationError::InvalidCapability(
            "At least one capability required".to_string(),
        ));
    }

    for cap in &manifest.capabilities {
        if !is_valid_capability(cap) {
            return Err(ValidationError::InvalidCapability(cap.clone()));
        }
    }

    // Cost metadata validation
    validate_cost_metadata(&manifest.cost_metadata)?;

    Ok(())
}

pub fn validate_cost_metadata(cost: &CostMetadata) -> Result<(), ValidationError> {
    let valid_tiers = vec!["free", "standard", "premium"];
    if !valid_tiers.contains(&cost.tier.as_str()) {
        return Err(ValidationError::MissingCostMetadata);
    }

    if cost.estimated_compute_tokens == 0 {
        return Err(ValidationError::MissingCostMetadata);
    }

    Ok(())
}

// Helper validators
fn is_valid_csci_version(version: &str) -> bool {
    // Accept CSCI v1.x format
    version.starts_with("1.")
}

fn is_valid_package_name(name: &str) -> bool {
    name.chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        && name.len() >= 3
        && name.len() <= 64
}

fn is_valid_semver(version: &str) -> bool {
    let parts: Vec<&str> = version.split('.').collect();
    parts.len() == 3 && parts.iter().all(|p| p.parse::<u32>().is_ok())
}

fn is_valid_capability(cap: &str) -> bool {
    let valid_caps = vec![
        "ai-text-generation",
        "ai-code-generation",
        "data-analysis",
        "document-processing",
        "summarization",
        "translation",
        "custom-computation",
    ];
    valid_caps.contains(&cap)
}

pub struct PackageValidator;

impl PackageValidator {
    /// Validate a complete package archive
    pub fn validate_package_archive(archive_path: &str) -> Result<ValidatedPackage, ValidationError> {
        let manifest_content = std::fs::read_to_string(
            format!("{}/cs-manifest.toml", archive_path)
        )?;

        let manifest: PackageManifest = toml::from_str(&manifest_content)?;
        validate_manifest(&manifest)?;

        // Validate directory structure
        Self::validate_structure(archive_path)?;

        Ok(ValidatedPackage {
            manifest,
            archive_path: archive_path.to_string(),
        })
    }

    fn validate_structure(archive_path: &str) -> Result<(), ValidationError> {
        let required_files = vec!["cs-manifest.toml", "README.md"];

        for file in required_files {
            let path = format!("{}/{}", archive_path, file);
            if !std::path::Path::new(&path).exists() {
                return Err(ValidationError::InvalidStructure(
                    format!("Missing required file: {}", file),
                ));
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct ValidatedPackage {
    pub manifest: PackageManifest,
    pub archive_path: String,
}
```

---

## 2. Registry API Specification

### Endpoint Types & Request/Response Models

```rust
// src/registry_api.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Package {
    pub id: String,
    pub name: String,
    pub version: String,
    pub csci_version: String,
    pub description: String,
    pub author: String,
    pub capabilities: Vec<String>,
    pub cost_metadata: HashMap<String, serde_json::Value>,
    pub published_at: String,
    pub downloads: u64,
    pub rating: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PublishRequest {
    pub manifest_toml: String,
    pub archive_data: Vec<u8>,
    pub api_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PublishResponse {
    pub id: String,
    pub name: String,
    pub version: String,
    pub url: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResponse {
    pub packages: Vec<Package>,
    pub total: usize,
    pub offset: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VersionsResponse {
    pub name: String,
    pub versions: Vec<VersionEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VersionEntry {
    pub version: String,
    pub published_at: String,
    pub yanked: bool,
}

/// Registry API Endpoints (REST)
///
/// POST /v1/packages
///   Request: PublishRequest { manifest_toml, archive_data, api_token }
///   Response: PublishResponse { id, name, version, url, message }
///   Validation: Validates manifest, checks author auth, stores archive
///
/// GET /v1/packages/{name}/{version}
///   Response: Package { id, name, version, csci_version, description, ... }
///   Returns: Full package metadata and download URL
///
/// GET /v1/packages/search?q={query}
///   Query Params: q (package name/capability), limit (default 20), offset
///   Response: SearchResponse { packages, total, offset }
///   Searches: Package names, descriptions, capabilities
///
/// GET /v1/packages/{name}/versions
///   Response: VersionsResponse { name, versions: [VersionEntry, ...] }
///   Returns: All versions (including yanked)
///
/// DELETE /v1/packages/{name}/{version}
///   Headers: Authorization: Bearer {token}
///   Response: { success: bool, message: String }
///   Unpublish: Marks version as yanked (soft delete)
///
/// GET /v1/packages/{name}/latest
///   Response: Package { latest stable version }
```

---

## 3. Tool Package Example: cognitive-summarizer

### cs-manifest.toml

```toml
[package]
name = "cognitive-summarizer"
version = "1.0.0"
csci_version = "1.2"
description = "High-performance multi-document text summarization with extractive and abstractive modes"
author = "XKernal Labs"
license = "Apache-2.0"
repository = "https://github.com/xkernal/cognitive-summarizer"
homepage = "https://docs.xkernal.io/packages/cognitive-summarizer"

[capabilities]
items = [
    "ai-text-generation",
    "document-processing",
    "summarization"
]

[cost]
estimated_compute_tokens = 2500
estimated_context_tokens = 8000
tier = "standard"

[[example]]
name = "basic_summary"
description = "Summarize a single document"
input = """
{
  "text": "Long document content...",
  "mode": "extractive",
  "summary_length": 150
}
"""
output = """
{
  "summary": "Key points extracted from document...",
  "compression_ratio": 0.25
}
"""
```

### Package Implementation (Rust)

```rust
// src/lib.rs (cognitive-summarizer package)
use cs_sdk::{Tool, ToolConfig, ToolResult};
use serde_json::{json, Value};

pub struct CognitiveSummarizer {
    config: ToolConfig,
}

impl CognitiveSummarizer {
    pub fn new() -> Self {
        Self {
            config: ToolConfig::default(),
        }
    }
}

impl Tool for CognitiveSummarizer {
    fn name(&self) -> &str {
        "cognitive-summarizer"
    }

    fn execute(&self, input: Value) -> ToolResult {
        let text = input["text"].as_str().ok_or("Missing 'text' field")?;
        let mode = input["mode"].as_str().unwrap_or("extractive");
        let length = input["summary_length"].as_u64().unwrap_or(150) as usize;

        let summary = match mode {
            "abstractive" => self.abstractive_summarize(text, length),
            _ => self.extractive_summarize(text, length),
        };

        Ok(json!({
            "summary": summary,
            "compression_ratio": summary.len() as f32 / text.len() as f32
        }))
    }
}

impl CognitiveSummarizer {
    fn extractive_summarize(&self, text: &str, target_len: usize) -> String {
        // Extractive logic: select key sentences
        text.lines()
            .take((target_len / 80).max(1))
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn abstractive_summarize(&self, text: &str, _target_len: usize) -> String {
        // Stub for abstractive summarization
        format!("Abstractive summary of: {}...", &text[..100.min(text.len())])
    }
}
```

---

## 4. Framework Adapter Example: langchain-adapter

### cs-manifest.toml

```toml
[package]
name = "langchain-adapter"
version = "0.1.0"
csci_version = "1.2"
description = "LangChain framework integration adapter. Bridges XKernal tools with LangChain agents and chains"
author = "XKernal Labs"
license = "MIT"

[capabilities]
items = ["custom-computation"]

[cost]
estimated_compute_tokens = 500
estimated_context_tokens = 2000
tier = "free"
```

### Adapter Implementation

```rust
// src/lib.rs (langchain-adapter package)
use cs_sdk::{Adapter, Tool, AdapterConfig};
use serde_json::Value;

pub struct LangChainAdapter {
    config: AdapterConfig,
}

impl LangChainAdapter {
    pub fn new(config: AdapterConfig) -> Self {
        Self { config }
    }
}

impl Adapter for LangChainAdapter {
    fn framework_name(&self) -> &str {
        "langchain"
    }

    fn adapt_tool(&self, tool: Box<dyn Tool>) -> Box<dyn Tool> {
        // Wraps XKernal Tool as LangChain ToolUse
        Box::new(LangChainWrappedTool { inner: tool })
    }
}

struct LangChainWrappedTool {
    inner: Box<dyn Tool>,
}

impl Tool for LangChainWrappedTool {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn execute(&self, input: Value) -> cs_sdk::ToolResult {
        self.inner.execute(input)
    }
}
```

---

## 5. cs-pkg CLI Design

### Command Structure

```bash
# Publish a package
cs-pkg publish ./cognitive-summarizer --token $REGISTRY_TOKEN
# Output: Published cognitive-summarizer:1.0.0 (id: pkg_abc123)

# Search for packages
cs-pkg search "summarization" --limit 10
# Output: Lists matching packages with versions and ratings

# Install a package
cs-pkg install cognitive-summarizer:1.0.0
# Output: Downloaded to ~/.cs-pkg/packages/cognitive-summarizer/

# Show package info
cs-pkg info cognitive-summarizer:1.0.0
# Output: Full manifest, download count, rating

# Validate a package
cs-pkg validate ./my-package
# Output: Validation passed. Ready to publish.

# List local packages
cs-pkg list
# Output: Installed packages and versions

# Remove package
cs-pkg remove cognitive-summarizer:1.0.0
```

---

## 6. Developer Guide: Creating & Publishing Packages

### Step-by-Step: Create a cs-pkg Package

**1. Initialize Package Structure**
```bash
mkdir my-tool && cd my-tool
cs-pkg init --name my-tool --author "Your Name"
```

**2. Define cs-manifest.toml**
```toml
[package]
name = "my-tool"
version = "1.0.0"
csci_version = "1.2"
description = "Brief description"
author = "Your Name"
license = "MIT"

[capabilities]
items = ["ai-text-generation"]

[cost]
estimated_compute_tokens = 1000
estimated_context_tokens = 4000
tier = "free"
```

**3. Implement Tool Logic** (src/lib.rs)
- Implement `Tool` trait from cs-sdk
- Implement `execute(input: Value) -> ToolResult`

**4. Add Documentation** (README.md)
- Usage examples
- Input/output schema
- Configuration options

**5. Validate Package**
```bash
cs-pkg validate .
```

**6. Publish to Registry**
```bash
cs-pkg publish . --token $REGISTRY_TOKEN
```

### Validation Checklist
- [ ] cs-manifest.toml valid and complete
- [ ] README.md present (min 100 chars)
- [ ] CSCI version compatible
- [ ] At least one capability declared
- [ ] Cost metadata realistic
- [ ] All dependencies listed
- [ ] No secrets in code

---

## Summary

**Phase 1 Deliverables:**
- ✅ Manifest validation library (cs-pkg-validate crate)
- ✅ Registry API specification (5 endpoints)
- ✅ Functional tool package example (cognitive-summarizer)
- ✅ Framework adapter stub (langchain-adapter)
- ✅ CLI command design
- ✅ Developer guide

**Next Steps (Week 9):**
- Implement registry backend (PostgreSQL)
- Build CLI tool (Clap-based)
- Create upload/download service
- Set up authentication (JWT tokens)
