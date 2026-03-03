# XKernal CS-PKG Registry: Technical Design Document

**Project:** XKernal Cognitive Substrate OS
**Phase:** 2 (SDK/Tools Layer - Rust)
**Week:** 21
**Status:** Design & Implementation
**Last Updated:** 2026-03-02

---

## 1. Executive Summary

Week 21 delivers the cs-pkg registry infrastructure at **registry.cognitivesubstrate.dev**, enabling distributed package management for the XKernal SDK ecosystem. The registry provides cryptographically secured package publishing, discovery, and installation capabilities with comprehensive monitoring and analytics.

**Core Deliverables:**
- Registry backend (Axum + PostgreSQL) with REST API
- Ed25519-based package signing and verification
- 10+ initial packages across tool, framework, and policy categories
- CLI tooling with install/search/publish commands
- Production monitoring and analytics dashboard

**Success Metrics:**
- Zero package validation failures in staging
- Sub-100ms registry search latency (p95)
- 100% signature verification success
- Full CI/CD integration with XKernal build system

---

## 2. Architecture Overview

### 2.1 System Components

```
┌─────────────────────────────────────────────────────────────┐
│                    registry.cognitivesubstrate.dev           │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  Axum HTTP Server (Port 443)                         │   │
│  │  ├─ Package API Routes                              │   │
│  │  ├─ Search & Discovery                              │   │
│  │  ├─ Publish Endpoint                                │   │
│  │  └─ Version Management                              │   │
│  └──────────────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  PostgreSQL (14+)                                    │   │
│  │  ├─ packages table                                   │   │
│  │  ├─ versions table                                   │   │
│  │  ├─ signatures table                                 │   │
│  │  ├─ downloads table (analytics)                      │   │
│  │  └─ publishers table                                 │   │
│  └──────────────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  Monitoring Stack                                    │   │
│  │  ├─ Prometheus metrics                               │   │
│  │  ├─ Grafana dashboards                               │   │
│  │  └─ Structured logging (tracing)                     │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
         ↑                          ↑                         ↑
    cs-pkg CLI              CI/CD Pipeline              Analytics Engine
```

### 2.2 Technology Stack

- **Runtime:** Tokio async runtime (Rust)
- **Web Framework:** Axum with Tower middleware
- **Database:** PostgreSQL 14+ with sqlx
- **Cryptography:** Ed25519 via `ed25519-dalek`
- **Serialization:** serde + JSON
- **Monitoring:** prometheus, tracing-subscriber
- **CLI:** clap with subcommands

---

## 3. REST API Specification

### 3.1 Package Endpoints

#### Search Packages
```
GET /api/v1/packages/search?q=<query>&category=<category>&limit=20&offset=0

Response 200:
{
  "packages": [
    {
      "id": "uuid",
      "name": "cs-core-toolchain",
      "namespace": "xkernal",
      "version": "0.1.0",
      "description": "Core SDK toolchain for XKernal",
      "category": "tool",
      "downloads_total": 1250,
      "rating": 4.8,
      "published_at": "2026-02-28T10:30:00Z"
    }
  ],
  "total_count": 47,
  "has_more": true
}
```

#### Get Package Metadata
```
GET /api/v1/packages/{namespace}/{name}

Response 200:
{
  "id": "uuid",
  "namespace": "xkernal",
  "name": "cs-manifest-validator",
  "latest_version": "0.2.1",
  "versions": ["0.1.0", "0.1.5", "0.2.0", "0.2.1"],
  "description": "Validation crate for cs-manifest.toml",
  "homepage": "https://github.com/xkernal/cs-manifest-validator",
  "repository": "https://github.com/xkernal/cs-manifest-validator",
  "license": "Apache-2.0",
  "author": {
    "name": "XKernal Team",
    "email": "sdk@xkernal.dev"
  },
  "keywords": ["manifest", "validation", "sdk"],
  "requires_signature": true,
  "published_at": "2026-02-01T00:00:00Z",
  "updated_at": "2026-02-28T14:22:00Z"
}
```

#### Get Package Version
```
GET /api/v1/packages/{namespace}/{name}/versions/{version}

Response 200:
{
  "package_id": "uuid",
  "version": "0.2.1",
  "tarball_url": "https://registry.cognitivesubstrate.dev/download/{namespace}/{name}/0.2.1/cs-manifest-validator-0.2.1.tar.gz",
  "checksum_sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  "size_bytes": 125432,
  "published_at": "2026-02-28T14:22:00Z",
  "yanked": false,
  "dependencies": [
    {
      "name": "serde",
      "version_spec": "^1.0",
      "namespace": "serde"
    }
  ]
}
```

#### Download Package Tarball
```
GET /api/v1/packages/{namespace}/{name}/versions/{version}/download

Response 200:
[Binary tarball content]
[Streaming with Content-Length header]
```

#### Publish Package
```
POST /api/v1/packages/publish
Content-Type: application/json
Authorization: Bearer {token}

{
  "name": "cs-llm-adapter",
  "namespace": "xkernal",
  "version": "1.0.0",
  "description": "Framework adapter for LLM integration",
  "category": "framework_adapter",
  "tarball_sha256": "abc123...",
  "tarball_url": "https://cdn.xkernal.dev/uploads/cs-llm-adapter-1.0.0.tar.gz",
  "signature": "ed25519_signature_base64",
  "manifest": {
    "cs_version": "0.1.0",
    "capabilities": ["inference", "streaming"],
    "policy_constraints": ["rate_limit"]
  }
}

Response 201:
{
  "id": "uuid",
  "name": "cs-llm-adapter",
  "version": "1.0.0",
  "status": "published",
  "registry_url": "registry.cognitivesubstrate.dev/xkernal/cs-llm-adapter"
}
```

#### Verify Package Signature
```
POST /api/v1/packages/{namespace}/{name}/versions/{version}/verify-signature

{
  "signature": "ed25519_signature_base64",
  "public_key": "ed25519_public_key_base64"
}

Response 200:
{
  "valid": true,
  "message": "Signature verified successfully",
  "signer": "XKernal Official <sdk@xkernal.dev>"
}
```

### 3.2 Analytics Endpoints

#### Get Package Statistics
```
GET /api/v1/analytics/packages/{namespace}/{name}?period=30d

Response 200:
{
  "downloads_total": 5420,
  "downloads_daily": [
    {"date": "2026-02-28", "count": 142},
    {"date": "2026-02-27", "count": 138}
  ],
  "install_success_rate": 0.998,
  "signature_validation_rate": 1.0,
  "avg_download_time_ms": 342
}
```

#### Get Registry Health
```
GET /api/v1/health

Response 200:
{
  "status": "healthy",
  "database": {
    "status": "connected",
    "latency_ms": 2
  },
  "packages_count": 10,
  "versions_count": 24,
  "last_sync": "2026-03-02T15:45:00Z"
}
```

---

## 4. Cryptographic Package Signing

### 4.1 Signing Scheme

All packages MUST be signed with Ed25519 before publication. The signature provides:
- Package authenticity verification
- Integrity assurance
- Publisher attribution
- Tamper detection

### 4.2 Rust Implementation

```rust
use ed25519_dalek::{Keypair, Signer, SigningKey, VerifyingKey};
use sha2::{Sha256, Digest};
use std::fs;

/// Generate publisher keypair for package signing
pub fn generate_publisher_keypair() -> Result<(String, String), Box<dyn std::error::Error>> {
    let keypair = Keypair::generate(&mut rand::thread_rng());

    let secret_key_bytes = keypair.secret.to_bytes();
    let public_key_bytes = keypair.public.to_bytes();

    let secret_key_b64 = base64_encode(&secret_key_bytes);
    let public_key_b64 = base64_encode(&public_key_bytes);

    Ok((secret_key_b64, public_key_b64))
}

/// Sign a package tarball
pub async fn sign_package(
    tarball_path: &str,
    secret_key_b64: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    // Read tarball
    let tarball_data = tokio::fs::read(tarball_path).await?;

    // Compute SHA256 hash
    let mut hasher = Sha256::new();
    hasher.update(&tarball_data);
    let file_hash = hasher.finalize();

    // Decode secret key
    let secret_key_bytes = base64_decode(secret_key_b64)?;
    let signing_key = SigningKey::from_bytes(
        &secret_key_bytes[..32].try_into().map_err(|_| "Invalid key length")?
    );

    // Sign the hash
    let signature = signing_key.sign(&file_hash);
    let signature_b64 = base64_encode(signature.to_bytes().as_slice());

    Ok(signature_b64)
}

/// Verify package signature
pub async fn verify_package_signature(
    tarball_path: &str,
    signature_b64: &str,
    public_key_b64: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    // Read tarball
    let tarball_data = tokio::fs::read(tarball_path).await?;

    // Compute SHA256 hash
    let mut hasher = Sha256::new();
    hasher.update(&tarball_data);
    let file_hash = hasher.finalize();

    // Decode credentials
    let signature_bytes = base64_decode(signature_b64)?;
    let public_key_bytes = base64_decode(public_key_b64)?;

    // Convert to signature and verifying key
    let signature = ed25519_dalek::Signature::from_bytes(
        &signature_bytes[..64].try_into().map_err(|_| "Invalid signature length")?
    );
    let verifying_key = VerifyingKey::from_bytes(
        &public_key_bytes[..32].try_into().map_err(|_| "Invalid key length")?
    )?;

    // Verify
    match verifying_key.verify(&file_hash, &signature) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

fn base64_encode(data: &[u8]) -> String {
    use base64::{engine::general_purpose, Engine};
    general_purpose::STANDARD.encode(data)
}

fn base64_decode(data: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use base64::{engine::general_purpose, Engine};
    Ok(general_purpose::STANDARD.decode(data)?)
}
```

---

## 5. Registry Backend Implementation

### 5.1 Database Schema

```sql
CREATE TABLE packages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    namespace VARCHAR(128) NOT NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    category VARCHAR(64), -- tool, framework_adapter, agent_template, policy
    repository_url VARCHAR(512),
    homepage_url VARCHAR(512),
    license VARCHAR(64),
    author_name VARCHAR(255),
    author_email VARCHAR(255),
    downloads_total INTEGER DEFAULT 0,
    rating DECIMAL(3, 2) DEFAULT 0.0,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(namespace, name)
);

CREATE TABLE versions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    package_id UUID NOT NULL REFERENCES packages(id) ON DELETE CASCADE,
    version_string VARCHAR(32) NOT NULL,
    description TEXT,
    tarball_url VARCHAR(512) NOT NULL,
    checksum_sha256 VARCHAR(64) NOT NULL,
    size_bytes INTEGER,
    yanked BOOLEAN DEFAULT FALSE,
    published_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(package_id, version_string)
);

CREATE TABLE signatures (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    version_id UUID NOT NULL REFERENCES versions(id) ON DELETE CASCADE,
    signature_b64 TEXT NOT NULL,
    public_key_b64 TEXT NOT NULL,
    signer_identity VARCHAR(255),
    verified_at TIMESTAMP,
    is_valid BOOLEAN,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE publishers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    namespace VARCHAR(128) NOT NULL UNIQUE,
    public_key_b64 TEXT NOT NULL,
    verified BOOLEAN DEFAULT FALSE,
    verified_at TIMESTAMP,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE downloads (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    version_id UUID NOT NULL REFERENCES versions(id) ON DELETE CASCADE,
    user_agent VARCHAR(512),
    ip_hash VARCHAR(64),
    downloaded_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_packages_namespace_name ON packages(namespace, name);
CREATE INDEX idx_versions_package_id ON versions(package_id);
CREATE INDEX idx_downloads_version_id_date ON downloads(version_id, downloaded_at);
```

### 5.2 Axum Server Implementation

```rust
use axum::{
    extract::{Path, Query, State},
    http::{StatusCode, HeaderMap},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing::{info, error};

#[derive(Clone)]
pub struct AppState {
    db: PgPool,
    metrics: Arc<MetricsCollector>,
}

#[derive(Serialize)]
pub struct PackageResponse {
    id: String,
    namespace: String,
    name: String,
    version: String,
    description: Option<String>,
    category: String,
    downloads_total: i32,
    published_at: String,
}

#[derive(Serialize)]
pub struct SearchResponse {
    packages: Vec<PackageResponse>,
    total_count: i64,
    has_more: bool,
}

#[derive(Deserialize)]
pub struct SearchQuery {
    q: String,
    category: Option<String>,
    limit: Option<i32>,
    offset: Option<i32>,
}

#[derive(Deserialize)]
pub struct PublishPayload {
    name: String,
    namespace: String,
    version: String,
    description: String,
    category: String,
    tarball_sha256: String,
    tarball_url: String,
    signature: String,
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/health", get(health_check))
        .route("/api/v1/packages/search", get(search_packages))
        .route("/api/v1/packages/:namespace/:name", get(get_package))
        .route(
            "/api/v1/packages/:namespace/:name/versions/:version",
            get(get_version),
        )
        .route(
            "/api/v1/packages/:namespace/:name/versions/:version/download",
            get(download_package),
        )
        .route("/api/v1/packages/publish", post(publish_package))
        .route(
            "/api/v1/packages/:namespace/:name/versions/:version/verify-signature",
            post(verify_signature_endpoint),
        )
        .route("/api/v1/analytics/packages/:namespace/:name", get(get_analytics))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .into_inner(),
        )
        .with_state(state)
}

async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    match state.db.acquire().await {
        Ok(_) => {
            info!("Health check: OK");
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "status": "healthy",
                    "database": { "status": "connected" }
                })),
            )
        }
        Err(e) => {
            error!("Health check failed: {}", e);
            (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!(
                {"status": "unhealthy", "error": e.to_string()}
            )))
        }
    }
}

async fn search_packages(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(20).min(100);
    let offset = params.offset.unwrap_or(0);

    let query = format!("%{}%", params.q);

    let sql = if let Some(category) = params.category {
        sqlx::query_as::<_, (String, String, String, String, String, i32, String)>(
            "SELECT id, namespace, name, version, description, downloads_total, published_at
             FROM packages WHERE (name ILIKE $1 OR description ILIKE $1) AND category = $2
             ORDER BY downloads_total DESC LIMIT $3 OFFSET $4"
        )
        .bind(&query)
        .bind(category)
        .bind(limit)
        .bind(offset)
    } else {
        sqlx::query_as::<_, (String, String, String, String, String, i32, String)>(
            "SELECT id, namespace, name, version, description, downloads_total, published_at
             FROM packages WHERE name ILIKE $1 OR description ILIKE $1
             ORDER BY downloads_total DESC LIMIT $2 OFFSET $3"
        )
        .bind(&query)
        .bind(limit)
        .bind(offset)
    };

    match sql.fetch_all(&state.db).await {
        Ok(rows) => {
            state.metrics.record_search_query(&params.q);

            let packages = rows
                .into_iter()
                .map(|(id, namespace, name, version, desc, downloads, pub_at)| {
                    PackageResponse {
                        id,
                        namespace,
                        name,
                        version,
                        description: if desc.is_empty() { None } else { Some(desc) },
                        category: "tool".to_string(),
                        downloads_total: downloads,
                        published_at: pub_at,
                    }
                })
                .collect();

            (StatusCode::OK, Json(SearchResponse {
                packages,
                total_count: rows.len() as i64,
                has_more: rows.len() == limit as usize,
            }))
        }
        Err(e) => {
            error!("Search failed: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(SearchResponse {
                packages: vec![],
                total_count: 0,
                has_more: false,
            }))
        }
    }
}

async fn publish_package(
    State(state): State<AppState>,
    Json(payload): Json<PublishPayload>,
) -> impl IntoResponse {
    // Verify signature before accepting
    // Insert package and version records
    // Record metrics

    (StatusCode::CREATED, Json(serde_json::json!({
        "status": "published",
        "name": payload.name,
        "version": payload.version
    })))
}

async fn get_package(
    State(state): State<AppState>,
    Path((namespace, name)): Path<(String, String)>,
) -> impl IntoResponse {
    // Fetch package and all versions
    StatusCode::OK
}

async fn get_version(
    State(state): State<AppState>,
    Path((namespace, name, version)): Path<(String, String, String)>,
) -> impl IntoResponse {
    // Fetch specific version metadata
    StatusCode::OK
}

async fn download_package(
    State(state): State<AppState>,
    Path((namespace, name, version)): Path<(String, String, String)>,
) -> impl IntoResponse {
    // Log download event for analytics
    // Serve tarball from CDN or storage
    StatusCode::OK
}

async fn verify_signature_endpoint(
    State(state): State<AppState>,
    Path((namespace, name, version)): Path<(String, String, String)>,
) -> impl IntoResponse {
    // Verify signature from payload
    StatusCode::OK
}

async fn get_analytics(
    State(state): State<AppState>,
    Path((namespace, name)): Path<(String, String)>,
) -> impl IntoResponse {
    // Aggregate download stats and metrics
    StatusCode::OK
}

pub struct MetricsCollector {
    // prometheus metrics
}

impl MetricsCollector {
    pub fn record_search_query(&self, query: &str) {
        // Record metric
    }
}
```

---

## 6. Initial Package Catalog

### 6.1 10+ Initial Packages (v1.0)

#### Tools (3)
1. **cs-core-toolchain** (xkernal/cs-core-toolchain:0.1.0)
   - Description: Core SDK compiler and build tools
   - Size: ~45MB
   - Dependencies: None (foundation)

2. **cs-manifest-validator** (xkernal/cs-manifest-validator:0.2.1)
   - Description: TOML validation for cs-manifest.toml
   - Size: ~2.1MB
   - Dependencies: serde, toml

3. **cs-capgraph-inspector** (xkernal/cs-capgraph-inspector:0.1.0)
   - Description: Capability graph analysis and visualization
   - Size: ~8.3MB
   - Dependencies: petgraph, serde_json

#### Framework Adapters (2)
4. **cs-llm-adapter** (xkernal/cs-llm-adapter:1.0.0)
   - Description: Integration framework for LLM inference services
   - Size: ~12MB
   - Dependencies: tokio, serde

5. **cs-storage-adapter** (xkernal/cs-storage-adapter:0.1.0)
   - Description: Abstraction layer for distributed storage backends
   - Size: ~9.5MB
   - Dependencies: async-trait, tokio

#### Agent Templates (2)
6. **cs-agent-task-executor** (xkernal/cs-agent-task-executor:0.1.0)
   - Description: Template for multi-step task execution agents
   - Size: ~5.2MB
   - Dependencies: cs-core-toolchain, tokio

7. **cs-agent-reasoning** (xkernal/cs-agent-reasoning:0.1.0)
   - Description: Template for chain-of-thought reasoning systems
   - Size: ~4.8MB
   - Dependencies: cs-core-toolchain

#### Policy Packages (3)
8. **cs-policy-rate-limiting** (xkernal/cs-policy-rate-limiting:0.1.0)
   - Description: Token bucket rate limiting policy
   - Size: ~1.3MB
   - Dependencies: None

9. **cs-policy-audit-logging** (xkernal/cs-policy-audit-logging:0.1.0)
   - Description: Comprehensive audit trail policy
   - Size: ~2.1MB
   - Dependencies: None

10. **cs-policy-resource-quotas** (xkernal/cs-policy-resource-quotas:0.1.0)
    - Description: CPU/memory resource constraint policies
    - Size: ~1.8MB
    - Dependencies: None

---

## 7. CS-PKG CLI Implementation

### 7.1 Installation Command

```rust
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "cs-pkg", about = "XKernal package manager CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(global = true, long)]
    registry: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Install a package
    Install {
        #[arg(value_name = "PACKAGE")]
        package: String,

        #[arg(short, long)]
        version: Option<String>,

        #[arg(short, long)]
        dest: Option<PathBuf>,
    },

    /// Search registry
    Search {
        #[arg(value_name = "QUERY")]
        query: String,

        #[arg(short, long)]
        category: Option<String>,

        #[arg(short, long)]
        limit: Option<usize>,
    },

    /// Publish package
    Publish {
        #[arg(value_name = "MANIFEST")]
        manifest: PathBuf,

        #[arg(short, long)]
        secret_key: Option<PathBuf>,
    },

    /// Verify package signature
    Verify {
        #[arg(value_name = "TARBALL")]
        tarball: PathBuf,

        #[arg(short, long)]
        signature: PathBuf,

        #[arg(short, long)]
        public_key: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let registry = cli.registry.unwrap_or_else(
        || "https://registry.cognitivesubstrate.dev".to_string()
    );

    match cli.command {
        Commands::Install { package, version, dest } => {
            install_package(&registry, &package, version, dest).await?
        }
        Commands::Search { query, category, limit } => {
            search_packages(&registry, &query, category, limit).await?
        }
        Commands::Publish { manifest, secret_key } => {
            publish_package(&registry, &manifest, secret_key).await?
        }
        Commands::Verify { tarball, signature, public_key } => {
            verify_package(&tarball, &signature, &public_key).await?
        }
    }

    Ok(())
}

async fn install_package(
    registry: &str,
    package: &str,
    version: Option<String>,
    dest: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (namespace, name) = parse_package_identifier(package)?;
    let dest = dest.unwrap_or_else(|| PathBuf::from("."));

    println!("Installing {}@{} from {}", package,
             version.as_deref().unwrap_or("latest"), registry);

    // Fetch metadata
    let client = reqwest::Client::new();
    let url = format!("{}/api/v1/packages/{}/{}", registry, namespace, name);
    let resp = client.get(&url).send().await?;
    let metadata: serde_json::Value = resp.json().await?;

    // Get version
    let target_version = version.as_deref()
        .unwrap_or(metadata["latest_version"].as_str().unwrap());

    // Download tarball
    let download_url = format!(
        "{}/api/v1/packages/{}/{}/versions/{}/download",
        registry, namespace, name, target_version
    );

    println!("Downloading from {}", download_url);
    let tarball = client.get(&download_url).send().await?;
    let mut file = tokio::fs::File::create(
        dest.join(format!("{}-{}.tar.gz", name, target_version))
    ).await?;

    let mut stream = tarball.bytes_stream();
    while let Some(chunk) = stream.next().await {
        use tokio::io::AsyncWriteExt;
        file.write_all(&chunk?).await?;
    }

    println!("✓ Installation complete");
    Ok(())
}

async fn search_packages(
    registry: &str,
    query: &str,
    category: Option<String>,
    limit: Option<usize>,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let mut url = format!("{}/api/v1/packages/search?q={}", registry,
                          urlencoding::encode(query));

    if let Some(cat) = category {
        url.push_str(&format!("&category={}", cat));
    }
    if let Some(l) = limit {
        url.push_str(&format!("&limit={}", l));
    }

    let resp = client.get(&url).send().await?;
    let results: serde_json::Value = resp.json().await?;

    println!("Found {} packages:\n", results["total_count"]);

    for pkg in results["packages"].as_array().unwrap_or(&vec![]) {
        println!(
            "  {} ({}@{})",
            pkg["name"],
            pkg["namespace"],
            pkg["version"]
        );
        println!("    {}", pkg["description"]);
        println!("    Downloads: {}\n", pkg["downloads_total"]);
    }

    Ok(())
}

async fn publish_package(
    registry: &str,
    manifest: &PathBuf,
    secret_key: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Parse cs-manifest.toml
    let manifest_content = tokio::fs::read_to_string(manifest).await?;
    let manifest_data: toml::Value = toml::from_str(&manifest_content)?;

    // Build tarball
    println!("Creating package tarball...");
    let tarball_path = create_tarball(".")?;

    // Sign if secret key provided
    let signature = if let Some(sk_path) = secret_key {
        println!("Signing package...");
        let secret_key_content = tokio::fs::read_to_string(sk_path).await?;
        sign_package(&tarball_path, &secret_key_content).await?
    } else {
        String::new()
    };

    // Publish
    println!("Publishing to {}...", registry);
    let client = reqwest::Client::new();
    let token = std::env::var("CS_PKG_TOKEN")?;

    let payload = serde_json::json!({
        "name": manifest_data["package"]["name"],
        "version": manifest_data["package"]["version"],
        "namespace": "xkernal",
        "signature": signature,
    });

    let resp = client
        .post(format!("{}/api/v1/packages/publish", registry))
        .bearer_auth(token)
        .json(&payload)
        .send()
        .await?;

    if resp.status().is_success() {
        println!("✓ Package published successfully");
    } else {
        println!("✗ Publication failed: {}", resp.status());
    }

    Ok(())
}

fn parse_package_identifier(pkg: &str) -> Result<(String, String), Box<dyn std::error::Error>> {
    if let Some(pos) = pkg.find('/') {
        Ok((pkg[..pos].to_string(), pkg[pos+1..].to_string()))
    } else {
        Ok(("xkernal".to_string(), pkg.to_string()))
    }
}
```

---

## 8. Monitoring & Analytics

### 8.1 Prometheus Metrics

```rust
use prometheus::{Counter, Histogram, Registry, IntGauge};
use lazy_static::lazy_static;

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();

    pub static ref HTTP_REQUESTS_TOTAL: Counter = Counter::new(
        "http_requests_total",
        "Total HTTP requests"
    ).expect("metric creation failed");

    pub static ref HTTP_REQUEST_DURATION_SECONDS: Histogram =
        Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "http_request_duration_seconds",
                "HTTP request duration in seconds"
            ),
            vec!["method", "endpoint", "status"]
        ).expect("metric creation failed");

    pub static ref PACKAGE_DOWNLOADS_TOTAL: Counter = Counter::new(
        "package_downloads_total",
        "Total package downloads"
    ).expect("metric creation failed");

    pub static ref REGISTRY_PACKAGES: IntGauge = IntGauge::new(
        "registry_packages_total",
        "Total packages in registry"
    ).expect("metric creation failed");

    pub static ref SIGNATURE_VERIFICATION_FAILURES: Counter = Counter::new(
        "signature_verification_failures_total",
        "Failed signature verifications"
    ).expect("metric creation failed");
}

pub fn init_metrics() {
    REGISTRY.register(Box::new(HTTP_REQUESTS_TOTAL.clone())).ok();
    REGISTRY.register(Box::new(HTTP_REQUEST_DURATION_SECONDS.clone())).ok();
    REGISTRY.register(Box::new(PACKAGE_DOWNLOADS_TOTAL.clone())).ok();
    REGISTRY.register(Box::new(REGISTRY_PACKAGES.clone())).ok();
    REGISTRY.register(Box::new(SIGNATURE_VERIFICATION_FAILURES.clone())).ok();
}

pub fn metrics_handler() -> String {
    prometheus::TextEncoder::new()
        .encode(&REGISTRY.gather(), &mut vec![].as_mut_slice())
        .unwrap_or_default()
}
```

### 8.2 Grafana Dashboard Specification

**Key Panels:**
- Package download trends (daily/weekly)
- Top packages by downloads
- Signature verification success rate
- API latency (p50, p95, p99)
- Registry uptime
- Active publishers
- Category distribution

**Alerts:**
- Database connection failures
- Signature verification failure spike (>5%)
- API latency p95 > 500ms
- Low disk space on CDN
- Failed package publishes

---

## 9. Deployment & Operations

### 9.1 Infrastructure Requirements

- **Server:** 2-4 vCPU, 8GB RAM minimum
- **Database:** PostgreSQL 14+ (managed RDS recommended)
- **Storage:** S3 or equivalent for tarball storage (1TB initial)
- **CDN:** CloudFront or equivalent for tarball distribution
- **Monitoring:** Prometheus + Grafana stack
- **Load Balancer:** HTTPS with TLS 1.3

### 9.2 Deployment Checklist

- [ ] PostgreSQL database provisioned and migrated
- [ ] S3 bucket configured with lifecycle policies
- [ ] TLS certificates installed at registry.cognitivesubstrate.dev
- [ ] Axum server containerized and tested
- [ ] Prometheus and Grafana deployed
- [ ] All 10 initial packages signed and published
- [ ] CI/CD integration for automated publishes
- [ ] Backup strategy implemented (daily snapshots)
- [ ] Load testing completed (>1000 concurrent requests)
- [ ] Production security audit passed

---

## 10. Success Criteria & Metrics

| Criterion | Target | Status |
|-----------|--------|--------|
| Package search latency (p95) | <100ms | |
| Publish endpoint availability | 99.9% | |
| Signature verification success | 100% | |
| Zero critical security issues | 0 | |
| Initial packages published | 10+ | |
| Registry API documentation | 100% | |
| CLI test coverage | >90% | |
| Database query performance | <5ms avg | |

---

## 11. Future Roadmap (Phase 2.5+)

- Package deprecation workflow
- Yanking with audit trail
- Namespace-level access controls
- Package provenance attestation (SLSA)
- Offline registry support
- Mirror federation capabilities
- Web UI for package browsing

---

## 12. References

- RFC: cs-pkg Registry Design (Week 7-8)
- cs-manifest.toml Specification
- XKernal Capability Graph (Week 20)
- Ed25519 Standard: RFC 8032
- Axum Framework Documentation
- PostgreSQL 14 Best Practices
