# WEEK 31: Migration Tooling Support - Agent Deployment Automation

**XKernal Cognitive Substrate OS** | Engineer 8: Semantic FS & Agent Lifecycle
**Date:** 2026-03-02 | **Status:** Design & Implementation
**Objective:** Build one-command deployment infrastructure for framework agents to running XKernal CTs

---

## 1. Executive Summary

### Vision Statement

Enable seamless deployment of AI agents from the XKernal framework to running cognitive substrate infrastructure in a single command. Eliminate manual provisioning complexity through integrated automation that validates resources, provisions infrastructure, configures deployment parameters, executes deployment, verifies health, and enables monitoring.

### Problem Statement

Current manual deployment requires:
- Multiple CLI invocations across engineer domains
- Manual resource verification and reservation
- Error-prone configuration hand-offs
- Unclear failure modes and recovery procedures
- No standardized patterns for agent crews, GPU-accelerated agents, or distributed deployments

### Solution Overview

Introduce `cs-deploy` (Deployment Automation) working in concert with Engineer 7's `cs-migrate` (Framework Migration) to provide:
- Single unified deployment command: `cs-deploy start <agent-manifest>`
- Automated resource validation and provisioning
- Template library for common deployment patterns
- Health verification and rollback on failure
- Integration with existing infrastructure (CT lifecycle, capability system, IPC)

### Expected Outcomes

1. **Deployment Time:** Framework agent → running CT in < 30 seconds
2. **Reliability:** 99.5% successful deployment on first attempt
3. **Developer Experience:** Single command for 80% of deployment scenarios
4. **Visibility:** Real-time deployment status with clear error messages

---

## 2. Agent Deployment Automation Architecture

### 2.1 Deployment Pipeline

```
┌─────────────────────────────────────────────────────────────────┐
│                    DEPLOYMENT PIPELINE FLOW                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  [Input: Agent Manifest]                                        │
│           ↓                                                      │
│  ┌─────────────────────┐                                        │
│  │  VALIDATE STAGE     │  • Resource schema validation          │
│  │  (cs-deploy init)   │  • Capability compatibility check      │
│  │                     │  • Dependency resolution               │
│  └────────┬────────────┘                                        │
│           ↓                                                      │
│  ┌─────────────────────┐                                        │
│  │ PROVISION STAGE     │  • CT slot allocation                  │
│  │(cs-provision CLI)   │  • Memory tier reservation             │
│  │                     │  • Capability minting                  │
│  └────────┬────────────┘                                        │
│           ↓                                                      │
│  ┌─────────────────────┐                                        │
│  │ CONFIGURE STAGE     │  • Apply deployment config             │
│  │(cs-deploy provision)│  • Bind IPC channels                   │
│  │                     │  • Register capability grants          │
│  └────────┬────────────┘                                        │
│           ↓                                                      │
│  ┌─────────────────────┐                                        │
│  │  DEPLOY STAGE       │  • Launch CT with agent binary         │
│  │  (cs-deploy start)  │  • Activate runtime lifecycle          │
│  │                     │  • Establish IPC connections          │
│  └────────┬────────────┘                                        │
│           ↓                                                      │
│  ┌─────────────────────┐                                        │
│  │  VERIFY STAGE       │  • Health endpoint polling             │
│  │  (cs-deploy status) │  • CT lifecycle state check            │
│  │                     │  • Memory allocation validation        │
│  └────────┬────────────┘                                        │
│           ↓                                                      │
│  ┌─────────────────────┐                                        │
│  │  MONITOR STAGE      │  • Continuous health monitoring        │
│  │ (cs-deploy monitor) │  • Performance metrics collection      │
│  │                     │  • Auto-scaling triggers               │
│  └────────┬────────────┘                                        │
│           ↓                                                      │
│  [Success] or [Rollback on Failure]                             │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### 2.2 Component Architecture

```
┌──────────────────────────────────────────────────────────────┐
│           CS-DEPLOY SYSTEM ARCHITECTURE                      │
├──────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌─────────────────────────────────────────────────────┐    │
│  │          cs-deploy CLI (Orchestrator)               │    │
│  │  • init / provision / start / status / rollback     │    │
│  ├─────────────────────────────────────────────────────┤    │
│  │  Uses: clap for arg parsing, tokio async runtime   │    │
│  └────────────────┬────────────────────────────────────┘    │
│                   │                                          │
│      ┌────────────┼────────────┐                             │
│      ↓            ↓            ↓                             │
│  ┌─────────┐ ┌──────────┐ ┌──────────────┐                  │
│  │ Validate│ │Provision │ │ Configure    │                  │
│  │ Module  │ │ Module   │ │ Module       │                  │
│  └────┬────┘ └────┬─────┘ └──────┬───────┘                  │
│       │           │              │                           │
│       └───────────┼──────────────┘                           │
│                   ↓                                          │
│  ┌──────────────────────────────────────────────────┐       │
│  │    Shared: Manifest Parser & State Machine       │       │
│  │  • TOML config deserialization                   │       │
│  │  • Deployment state tracking                     │       │
│  │  • Error handling & rollback logic               │       │
│  └────────────┬──────────────────────────────────────┘      │
│               │                                              │
│      ┌────────┴─────────────────┐                            │
│      ↓                          ↓                            │
│  ┌─────────────────┐  ┌───────────────────┐                │
│  │ XKernal Runtime │  │ cs-migrate Output │                │
│  │ Integration     │  │ (Manifest)        │                │
│  │ • CT lifecycle  │  │                   │                │
│  │ • Capability    │  │ From Engineer 7   │                │
│  │ • IPC           │  │                   │                │
│  └─────────────────┘  └───────────────────┘                │
│                                                               │
└──────────────────────────────────────────────────────────────┘
```

### 2.3 Integration with Engineer 7 (cs-migrate)

**Data Flow:**
```
Framework Agent Code
    ↓
[cs-migrate] (Engineer 7)
    ├─ Analyze dependencies
    ├─ Generate deployment manifest
    └─ Output: deployment.toml
        ↓
    [cs-deploy] (Engineer 8)
        ├─ Consume manifest
        ├─ Validate resources
        ├─ Provision infrastructure
        ├─ Deploy to XKernal
        └─ Output: deployment status
```

**Configuration Schema (Shared):**
- Both tools use identical `DeploymentManifest` structure
- `cs-migrate` produces; `cs-deploy` consumes
- Schema versioning for forward compatibility

---

## 3. cs-deploy CLI Implementation

### 3.1 Architecture & Design

```rust
// File: xkernal/cs-deploy/src/main.rs
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::runtime::Runtime;

#[derive(Parser)]
#[command(name = "cs-deploy")]
#[command(about = "XKernal Agent Deployment Automation System", long_about = None)]
struct Cli {
    /// Global verbosity level
    #[arg(global = true, short, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Global config directory override
    #[arg(global = true, long)]
    config_dir: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize deployment validation
    Init {
        /// Path to deployment manifest
        #[arg(value_name = "FILE")]
        manifest: PathBuf,

        /// Dry-run mode (no actual provisioning)
        #[arg(short = 'n', long)]
        dry_run: bool,

        /// Output validation report
        #[arg(short, long)]
        report: Option<PathBuf>,
    },

    /// Provision infrastructure resources
    Provision {
        /// Path to validated manifest
        #[arg(value_name = "FILE")]
        manifest: PathBuf,

        /// Resource reservation timeout (seconds)
        #[arg(long, default_value = "300")]
        timeout: u64,

        /// Force reprovisioning if already exists
        #[arg(short, long)]
        force: bool,
    },

    /// Start deployment and activate agent
    Start {
        /// Agent deployment ID or manifest path
        #[arg(value_name = "ID|FILE")]
        target: String,

        /// Wait for deployment completion (seconds)
        #[arg(short, long)]
        wait: Option<u64>,

        /// Enable continuous monitoring
        #[arg(short, long)]
        monitor: bool,
    },

    /// Check deployment status
    Status {
        /// Agent deployment ID
        #[arg(value_name = "ID")]
        deployment_id: String,

        /// Output format (json, text, table)
        #[arg(short, long, default_value = "text")]
        format: StatusFormat,

        /// Poll continuously
        #[arg(short, long)]
        watch: bool,
    },

    /// Rollback deployment to previous state
    Rollback {
        /// Agent deployment ID
        #[arg(value_name = "ID")]
        deployment_id: String,

        /// Target rollback generation (default: previous)
        #[arg(long)]
        to_generation: Option<u32>,

        /// Skip health verification
        #[arg(long)]
        force: bool,
    },
}

#[derive(Debug, Clone, Copy)]
enum StatusFormat {
    Json,
    Text,
    Table,
}

impl std::str::FromStr for StatusFormat {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "json" => Ok(StatusFormat::Json),
            "text" => Ok(StatusFormat::Text),
            "table" => Ok(StatusFormat::Table),
            _ => Err(format!("Unknown format: {}", s)),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Initialize logging based on verbosity
    init_logging(cli.verbose);

    match cli.command {
        Commands::Init { manifest, dry_run, report } => {
            cmd_init(manifest, dry_run, report).await?
        }
        Commands::Provision { manifest, timeout, force } => {
            cmd_provision(manifest, timeout, force).await?
        }
        Commands::Start { target, wait, monitor } => {
            cmd_start(target, wait, monitor).await?
        }
        Commands::Status { deployment_id, format, watch } => {
            cmd_status(deployment_id, format, watch).await?
        }
        Commands::Rollback { deployment_id, to_generation, force } => {
            cmd_rollback(deployment_id, to_generation, force).await?
        }
    }

    Ok(())
}

fn init_logging(level: u8) {
    let filter_level = match level {
        0 => log::LevelFilter::Warn,
        1 => log::LevelFilter::Info,
        2 => log::LevelFilter::Debug,
        _ => log::LevelFilter::Trace,
    };
    env_logger::Builder::from_default_env()
        .filter_level(filter_level)
        .init();
}
```

### 3.2 Command Implementation

```rust
// File: xkernal/cs-deploy/src/commands.rs
use crate::*;
use std::path::PathBuf;
use std::time::Duration;

/// Initialize and validate deployment manifest
pub async fn cmd_init(
    manifest_path: PathBuf,
    dry_run: bool,
    report_path: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    log::info!("Initializing deployment from: {}", manifest_path.display());

    // Parse manifest
    let manifest = DeploymentManifest::from_file(&manifest_path)?;
    log::info!("Manifest loaded: agent={}", manifest.agent.name);

    // Validate manifest structure
    manifest.validate_schema()?;
    log::info!("Schema validation passed");

    // Check resource availability
    if !dry_run {
        let validator = ResourceValidator::new();
        validator.check_ct_slots_available(manifest.resources.ct_slots)?;
        validator.check_memory_available(manifest.resources.memory_mb)?;
        validator.check_capabilities_compatible(&manifest.capabilities)?;
        log::info!("Resource availability check passed");
    }

    // Resolve dependencies
    let resolver = DependencyResolver::new();
    resolver.resolve(&manifest.dependencies)?;
    log::info!("Dependency resolution passed");

    // Generate and output validation report
    let report = ValidationReport {
        manifest_path: manifest_path.clone(),
        timestamp: chrono::Utc::now(),
        validation_passed: true,
        resource_summary: manifest.resources.clone(),
        checks_performed: vec![
            "schema_validation".to_string(),
            "resource_availability".to_string(),
            "dependency_resolution".to_string(),
        ],
    };

    if let Some(output) = report_path {
        report.save(&output)?;
        log::info!("Validation report saved to: {}", output.display());
    }

    log::info!("Initialization successful");
    Ok(())
}

/// Provision infrastructure resources
pub async fn cmd_provision(
    manifest_path: PathBuf,
    timeout_secs: u64,
    force: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    log::info!("Provisioning infrastructure from: {}", manifest_path.display());

    let manifest = DeploymentManifest::from_file(&manifest_path)?;
    let timeout = Duration::from_secs(timeout_secs);

    // Check if already provisioned
    if !force {
        let state_mgr = DeploymentStateManager::new();
        if state_mgr.is_provisioned(&manifest.agent.name)? {
            return Err("Already provisioned. Use --force to reprovision".into());
        }
    }

    // Allocate CT slot
    let ct_manager = CTManager::new();
    let ct_slot = ct_manager
        .allocate_slot(manifest.resources.ct_slots, timeout)
        .await?;
    log::info!("CT slot allocated: {:?}", ct_slot);

    // Reserve memory tier
    let memory_mgr = MemoryManager::new();
    let memory_reservation = memory_mgr
        .reserve_tier(manifest.resources.memory_mb, &manifest.resources.memory_type)
        .await?;
    log::info!("Memory tier reserved: {}", manifest.resources.memory_mb);

    // Mint capabilities
    let capability_mgr = CapabilityManager::new();
    for cap in &manifest.capabilities {
        capability_mgr.mint_capability(cap).await?;
        log::info!("Capability minted: {}", cap.name);
    }

    // Create IPC channels
    let ipc_mgr = IPCManager::new();
    for channel in &manifest.ipc_channels {
        ipc_mgr.create_channel(channel).await?;
        log::info!("IPC channel created: {}", channel.name);
    }

    // Persist provisioning state
    let state_mgr = DeploymentStateManager::new();
    let deployment_id = uuid::Uuid::new_v4().to_string();
    state_mgr.record_provisioning(
        &deployment_id,
        &manifest.agent.name,
        &ProvisioningState {
            ct_slot,
            memory_reservation,
            capabilities: manifest.capabilities.clone(),
            ipc_channels: manifest.ipc_channels.clone(),
        },
    )?;

    log::info!("Provisioning successful: deployment_id={}", deployment_id);
    println!("DEPLOYMENT_ID={}", deployment_id);
    Ok(())
}

/// Start deployment and activate agent
pub async fn cmd_start(
    target: String,
    wait_secs: Option<u64>,
    monitor: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    log::info!("Starting deployment: {}", target);

    let state_mgr = DeploymentStateManager::new();
    let provisioning = state_mgr.get_provisioning(&target)?;

    // Launch CT with agent binary
    let ct_manager = CTManager::new();
    let ct_instance = ct_manager
        .launch_ct(&provisioning.ct_slot, &target)
        .await?;
    log::info!("CT launched: {:?}", ct_instance);

    // Activate IPC connections
    let ipc_mgr = IPCManager::new();
    for channel in &provisioning.ipc_channels {
        ipc_mgr.activate_channel(channel).await?;
    }
    log::info!("IPC channels activated");

    // Wait for health check if requested
    if let Some(wait_secs) = wait_secs {
        let health_check = HealthChecker::new();
        health_check
            .wait_for_ready(&target, Duration::from_secs(wait_secs))
            .await?;
        log::info!("Health check passed");
    }

    // Start monitoring if requested
    if monitor {
        start_monitoring(&target).await?;
    }

    log::info!("Deployment started successfully");
    Ok(())
}

/// Check deployment status
pub async fn cmd_status(
    deployment_id: String,
    format: StatusFormat,
    watch: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let state_mgr = DeploymentStateManager::new();

    loop {
        let status = state_mgr.get_deployment_status(&deployment_id).await?;

        match format {
            StatusFormat::Json => {
                println!("{}", serde_json::to_string_pretty(&status)?);
            }
            StatusFormat::Text => {
                println!("Deployment: {}", status.deployment_id);
                println!("Status: {}", status.state);
                println!("CT State: {}", status.ct_state);
                println!("Memory: {} MB", status.memory_allocated);
                println!("Uptime: {:?}", status.uptime);
            }
            StatusFormat::Table => {
                print_status_table(&status);
            }
        }

        if !watch {
            break;
        }

        tokio::time::sleep(Duration::from_secs(5)).await;
    }

    Ok(())
}

/// Rollback deployment to previous state
pub async fn cmd_rollback(
    deployment_id: String,
    to_generation: Option<u32>,
    force: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    log::info!("Rolling back deployment: {}", deployment_id);

    let state_mgr = DeploymentStateManager::new();

    // Retrieve rollback point
    let generation = to_generation.unwrap_or_else(|| 0); // default: previous
    let previous_state = state_mgr.get_rollback_point(&deployment_id, generation)?;

    if !force {
        // Verify health before rollback
        let health_check = HealthChecker::new();
        if health_check.is_healthy(&deployment_id).await? {
            log::warn!("Current deployment is healthy. Proceed with caution.");
        }
    }

    // Stop current deployment
    let ct_manager = CTManager::new();
    ct_manager.stop_ct(&deployment_id).await?;
    log::info!("CT stopped");

    // Restore previous state
    state_mgr.restore_state(&deployment_id, &previous_state)?;
    log::info!("State restored to generation {}", generation);

    log::info!("Rollback completed successfully");
    Ok(())
}
```

---

## 4. cs-provision CLI Implementation

### 4.1 Resource Provisioning Engine

```rust
// File: xkernal/cs-provision/src/lib.rs
use std::time::Duration;
use tokio::sync::RwLock;
use std::sync::Arc;

/// Main provisioning orchestrator
pub struct ProvisioningEngine {
    ct_manager: Arc<CTManager>,
    memory_manager: Arc<MemoryManager>,
    capability_manager: Arc<CapabilityManager>,
    ipc_manager: Arc<IPCManager>,
    state: Arc<RwLock<ProvisioningState>>,
}

impl ProvisioningEngine {
    pub fn new() -> Self {
        Self {
            ct_manager: Arc::new(CTManager::new()),
            memory_manager: Arc::new(MemoryManager::new()),
            capability_manager: Arc::new(CapabilityManager::new()),
            ipc_manager: Arc::new(IPCManager::new()),
            state: Arc::new(RwLock::new(ProvisioningState::default())),
        }
    }

    /// Provision complete agent deployment
    pub async fn provision_agent(
        &self,
        config: &AgentProvisioningConfig,
        timeout: Duration,
    ) -> Result<ProvisioningResult, ProvisioningError> {
        let mut state = self.state.write().await;

        // Step 1: CT Slot Allocation
        let ct_allocation = self
            .ct_manager
            .allocate_slot(
                config.resources.ct_slots,
                config.resources.ct_tier,
                timeout,
            )
            .await?;
        state.ct_allocation = Some(ct_allocation.clone());
        log::info!("CT slot allocated: {:?}", ct_allocation);

        // Step 2: Memory Tier Reservation
        let memory_allocation = self
            .memory_manager
            .reserve_tier(
                config.resources.memory_mb,
                &config.resources.memory_type,
                config.resources.numa_preferred,
            )
            .await?;
        state.memory_allocation = Some(memory_allocation.clone());
        log::info!("Memory reserved: {} MB on tier {}",
            config.resources.memory_mb,
            config.resources.memory_type);

        // Step 3: GPU Allocation (if needed)
        if let Some(gpu_config) = &config.resources.gpu_config {
            let gpu_allocation = self
                .ct_manager
                .allocate_gpu(gpu_config, timeout)
                .await?;
            state.gpu_allocation = Some(gpu_allocation.clone());
            log::info!("GPU allocated: {:?}", gpu_allocation);
        }

        // Step 4: Capability Minting
        for capability in &config.capabilities {
            self.capability_manager
                .mint_capability(capability)
                .await?;
            state.minted_capabilities.push(capability.clone());
            log::info!("Capability minted: {}", capability.name);
        }

        // Step 5: IPC Channel Creation
        for channel_spec in &config.ipc_channels {
            let channel = self
                .ipc_manager
                .create_channel(channel_spec)
                .await?;
            state.ipc_channels.push(channel);
            log::info!("IPC channel created: {}", channel_spec.name);
        }

        Ok(ProvisioningResult {
            deployment_id: uuid::Uuid::new_v4().to_string(),
            ct_allocation,
            memory_allocation,
            gpu_allocation: state.gpu_allocation.clone(),
            capabilities_minted: state.minted_capabilities.len(),
            ipc_channels_created: state.ipc_channels.len(),
            provisioned_at: chrono::Utc::now(),
        })
    }

    /// Verify provisioned resources are available
    pub async fn verify_resources(&self) -> Result<ResourceVerification, ProvisioningError> {
        let state = self.state.read().await;

        let mut verification = ResourceVerification::default();

        // Check CT slot active
        if let Some(ct_alloc) = &state.ct_allocation {
            verification.ct_slots_available = self.ct_manager.is_slot_active(&ct_alloc.slot_id).await?;
        }

        // Check memory pages allocated
        if let Some(mem_alloc) = &state.memory_allocation {
            verification.memory_available =
                self.memory_manager.verify_allocation(&mem_alloc).await?;
        }

        // Check capabilities registered
        verification.capabilities_registered = state.minted_capabilities.len();

        // Check IPC channels ready
        for channel in &state.ipc_channels {
            verification.ipc_channels_ready +=
                if self.ipc_manager.is_channel_ready(&channel.channel_id).await? {
                    1
                } else {
                    0
                };
        }

        Ok(verification)
    }
}

/// CT (Cognitive Thread) Management
pub struct CTManager {
    // Interface to L1 Services (CT lifecycle)
}

impl CTManager {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn allocate_slot(
        &self,
        num_slots: u32,
        tier: CTTier,
        timeout: Duration,
    ) -> Result<CTAllocation, ProvisioningError> {
        // Call L1 CT lifecycle service
        // Request available cognitive thread slots with specified tier
        todo!("Implement CT allocation RPC to L1 Services")
    }

    pub async fn allocate_gpu(
        &self,
        config: &GPUConfig,
        timeout: Duration,
    ) -> Result<GPUAllocation, ProvisioningError> {
        // GPU resource allocation (if GPU-accelerated tier)
        todo!("Implement GPU allocation from L1 resource pool")
    }

    pub async fn is_slot_active(&self, slot_id: &str) -> Result<bool, ProvisioningError> {
        todo!("Check CT slot active status")
    }

    pub async fn stop_ct(&self, deployment_id: &str) -> Result<(), ProvisioningError> {
        todo!("Stop CT via L1 lifecycle")
    }
}

/// Memory Management
pub struct MemoryManager {
    // Interface to L1 Memory allocation service
}

impl MemoryManager {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn reserve_tier(
        &self,
        memory_mb: u32,
        tier: &str,
        numa_preferred: Option<u32>,
    ) -> Result<MemoryAllocation, ProvisioningError> {
        // Reserve memory from specified tier (fast/standard/large)
        // Respect NUMA preferences if available
        todo!("Implement memory tier reservation")
    }

    pub async fn verify_allocation(
        &self,
        allocation: &MemoryAllocation,
    ) -> Result<bool, ProvisioningError> {
        todo!("Verify memory pages are accessible")
    }
}

/// Capability Management (Interface to L0 Security Model)
pub struct CapabilityManager {
    // Interface to L0 capability minting
}

impl CapabilityManager {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn mint_capability(
        &self,
        capability: &CapabilitySpec,
    ) -> Result<MintedCapability, ProvisioningError> {
        // Mint capability via L0 security model
        // Capabilities: CT_LIFECYCLE, IPC_SEND, IPC_RECV, MEMORY_ACCESS, etc.
        todo!("Mint capability via L0 model")
    }
}

/// IPC Channel Management
pub struct IPCManager {
    // Interface to L1 IPC service
}

impl IPCManager {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn create_channel(
        &self,
        spec: &IPCChannelSpec,
    ) -> Result<IPCChannel, ProvisioningError> {
        // Create IPC channel for agent communication
        // Establish ring buffers and synchronization primitives
        todo!("Create IPC channel via L1 service")
    }

    pub async fn activate_channel(&self, channel: &IPCChannel) -> Result<(), ProvisioningError> {
        todo!("Activate IPC channel for data transfer")
    }

    pub async fn is_channel_ready(&self, channel_id: &str) -> Result<bool, ProvisioningError> {
        todo!("Check IPC channel ready for communication")
    }
}

// Data structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CTAllocation {
    pub slot_id: String,
    pub tier: CTTier,
    pub num_slots: u32,
    pub allocated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CTTier {
    Realtime,    // Ultra-low latency
    Standard,    // Normal latency
    Background,  // Best-effort
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryAllocation {
    pub allocation_id: String,
    pub memory_mb: u32,
    pub tier: String, // "fast", "standard", "large"
    pub base_address: u64,
    pub num_pages: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GPUConfig {
    pub device_id: u32,
    pub memory_mb: u32,
    pub compute_capability: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GPUAllocation {
    pub device_id: u32,
    pub allocated_memory: u32,
    pub driver_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintedCapability {
    pub capability_id: String,
    pub grant_id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IPCChannel {
    pub channel_id: String,
    pub name: String,
    pub producer_ct: String,
    pub consumer_ct: String,
    pub buffer_size: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProvisioningResult {
    pub deployment_id: String,
    pub ct_allocation: CTAllocation,
    pub memory_allocation: MemoryAllocation,
    pub gpu_allocation: Option<GPUAllocation>,
    pub capabilities_minted: usize,
    pub ipc_channels_created: usize,
    pub provisioned_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Default)]
pub struct ResourceVerification {
    pub ct_slots_available: bool,
    pub memory_available: bool,
    pub capabilities_registered: usize,
    pub ipc_channels_ready: usize,
}

#[derive(Debug)]
pub enum ProvisioningError {
    CTSlotAllocationFailed(String),
    MemoryReservationFailed(String),
    GPUAllocationFailed(String),
    CapabilityMintingFailed(String),
    IPCChannelCreationFailed(String),
    TimeoutExceeded,
    InvalidConfiguration(String),
}

impl std::fmt::Display for ProvisioningError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::CTSlotAllocationFailed(msg) => write!(f, "CT slot allocation failed: {}", msg),
            Self::MemoryReservationFailed(msg) => write!(f, "Memory reservation failed: {}", msg),
            Self::GPUAllocationFailed(msg) => write!(f, "GPU allocation failed: {}", msg),
            Self::CapabilityMintingFailed(msg) => write!(f, "Capability minting failed: {}", msg),
            Self::IPCChannelCreationFailed(msg) => write!(f, "IPC channel creation failed: {}", msg),
            Self::TimeoutExceeded => write!(f, "Provisioning timeout exceeded"),
            Self::InvalidConfiguration(msg) => write!(f, "Invalid configuration: {}", msg),
        }
    }
}

impl std::error::Error for ProvisioningError {}
```

---

## 5. Configuration Templates

### 5.1 Single-Agent Deployment

**File: templates/single-agent-deployment.toml**

```toml
# Single Agent Deployment Template
# Basic configuration for deploying a single autonomous agent to XKernal

[agent]
name = "autonomous-solver-v1"
version = "1.0.0"
description = "Autonomous problem solver agent"
binary_path = "/opt/agents/solver.wasm"
entry_point = "agent_main"

[resources]
ct_slots = 2                    # Cognitive thread slots
ct_tier = "standard"            # CT scheduling tier
memory_mb = 512                 # Memory allocation
memory_type = "standard"        # Memory tier

[capabilities]
[[capabilities.items]]
name = "ct_lifecycle"
grant = "execute"

[[capabilities.items]]
name = "ipc_send"
channels = ["solver:output"]

[[capabilities.items]]
name = "ipc_recv"
channels = ["solver:input"]

[[capabilities.items]]
name = "memory_access"
tier = "standard"

[ipc_channels]
[[ipc_channels.items]]
name = "solver:input"
producer_ct = "external"
consumer_ct = "solver"
buffer_size = 4096

[[ipc_channels.items]]
name = "solver:output"
producer_ct = "solver"
consumer_ct = "external"
buffer_size = 4096

[deployment]
startup_timeout_secs = 30
health_check_enabled = true
health_check_interval_secs = 5
auto_restart = true
max_restart_attempts = 3

[monitoring]
enabled = true
metrics_collection = true
log_level = "info"
```

### 5.2 Multi-Agent Crew Deployment

**File: templates/multi-agent-crew.toml**

```toml
# Multi-Agent Crew Deployment Template
# Configuration for deploying a coordinated multi-agent system

[crew]
name = "research-team"
description = "Multi-agent research coordination crew"
num_agents = 3

[[crew.agents]]
name = "researcher"
version = "1.0.0"
binary_path = "/opt/agents/researcher.wasm"
ct_slots = 2
memory_mb = 256

[[crew.agents]]
name = "validator"
version = "1.0.0"
binary_path = "/opt/agents/validator.wasm"
ct_slots = 2
memory_mb = 256

[[crew.agents]]
name = "synthesizer"
version = "1.0.0"
binary_path = "/opt/agents/synthesizer.wasm"
ct_slots = 2
memory_mb = 256

[crew.resources]
total_ct_slots = 6
total_memory_mb = 768
memory_type = "standard"

[crew.ipc_topology]
# Crew communication pattern
[[crew.ipc_topology.channels]]
name = "researcher:findings"
from = "researcher"
to = "validator"
buffer_size = 8192

[[crew.ipc_topology.channels]]
name = "validator:approval"
from = "validator"
to = "synthesizer"
buffer_size = 4096

[[crew.ipc_topology.channels]]
name = "synthesizer:result"
from = "synthesizer"
to = "external"
buffer_size = 8192

[crew.synchronization]
enabled = true
barrier_points = ["phase1_complete", "phase2_complete"]

[deployment]
deployment_order = ["researcher", "validator", "synthesizer"]
startup_timeout_secs = 60
wait_for_all_healthy = true
```

### 5.3 GPU-Accelerated Agent

**File: templates/gpu-accelerated-agent.toml**

```toml
[agent]
name = "ml-inference-agent"
version = "1.0.0"
binary_path = "/opt/agents/ml-inference.wasm"
framework = "pytorch"

[resources]
ct_slots = 4
ct_tier = "realtime"
memory_mb = 2048
memory_type = "fast"

[resources.gpu_config]
device_id = 0
memory_mb = 8192
compute_capability = "8.0"   # Ampere or higher
cumo_allocation = true       # Memory optimization

[capabilities]
[[capabilities.items]]
name = "gpu_compute"
device_id = 0
operations = ["inference", "fine-tuning"]

[[capabilities.items]]
name = "memory_access"
tier = "fast"

[ipc_channels]
[[ipc_channels.items]]
name = "inference:requests"
buffer_size = 16384

[[ipc_channels.items]]
name = "inference:results"
buffer_size = 16384

[deployment]
startup_timeout_secs = 45
health_check_interval_secs = 2
enable_performance_monitoring = true

[monitoring]
gpu_monitoring = true
compute_utilization_target = 80
memory_utilization_threshold = 90
```

### 5.4 High-Memory Agent

**File: templates/high-memory-agent.toml**

```toml
[agent]
name = "large-context-agent"
version = "1.0.0"
binary_path = "/opt/agents/context-processor.wasm"
context_window = 131072   # 128k tokens

[resources]
ct_slots = 8
ct_tier = "standard"
memory_mb = 8192
memory_type = "large"     # Large page allocations
numa_preferred = 0        # NUMA node preference

[capabilities]
[[capabilities.items]]
name = "memory_access"
tier = "large"
numa_affinity = true

[[capabilities.items]]
name = "ipc_send"
channels = ["context:output"]

[ipc_channels]
[[ipc_channels.items]]
name = "context:input"
buffer_size = 65536

[[ipc_channels.items]]
name = "context:output"
buffer_size = 65536

[deployment]
startup_timeout_secs = 60
memory_warmup = true
enable_memory_lock = true
```

### 5.5 Distributed Agent Cluster

**File: templates/distributed-agent-cluster.toml**

```toml
[cluster]
name = "distributed-compute-cluster"
num_nodes = 3
agent_replicas = 3

[[cluster.nodes]]
node_id = "node-1"
ct_slots = 8
memory_mb = 4096

[[cluster.nodes]]
node_id = "node-2"
ct_slots = 8
memory_mb = 4096

[[cluster.nodes]]
node_id = "node-3"
ct_slots = 8
memory_mb = 4096

[cluster.agent]
name = "compute-worker"
binary_path = "/opt/agents/worker.wasm"
replicas_per_node = 1

[cluster.communication]
topology = "mesh"
replication_factor = 2

[cluster.load_balancing]
strategy = "round_robin"
health_aware = true

[deployment]
simultaneous_deployments = 3
rollout_strategy = "rolling"
```

---

## 6. Validation Framework

### 6.1 Pre-Deployment Validation

```rust
// File: xkernal/cs-deploy/src/validation.rs
use std::collections::HashMap;

pub struct ValidationFramework {
    validators: HashMap<String, Box<dyn Validator>>,
}

impl ValidationFramework {
    pub fn new() -> Self {
        let mut validators: HashMap<String, Box<dyn Validator>> = HashMap::new();
        validators.insert("resource".into(), Box::new(ResourceValidator));
        validators.insert("capability".into(), Box::new(CapabilityValidator));
        validators.insert("dependency".into(), Box::new(DependencyValidator));
        validators.insert("network".into(), Box::new(NetworkValidator));

        Self { validators }
    }

    pub async fn run_all_validations(
        &self,
        manifest: &DeploymentManifest,
    ) -> Result<ValidationReport, ValidationError> {
        let mut report = ValidationReport::new();

        for (name, validator) in &self.validators {
            log::info!("Running validator: {}", name);
            match validator.validate(manifest).await {
                Ok(result) => {
                    report.add_check(name.clone(), result);
                    log::info!("Validator {} passed", name);
                }
                Err(e) => {
                    report.add_failure(name.clone(), format!("{}", e));
                    log::warn!("Validator {} failed: {}", name, e);
                    return Err(e);
                }
            }
        }

        Ok(report)
    }
}

pub trait Validator: Send + Sync {
    async fn validate(
        &self,
        manifest: &DeploymentManifest,
    ) -> Result<ValidationCheck, ValidationError>;
}

/// Resource availability validation
pub struct ResourceValidator;

#[async_trait::async_trait]
impl Validator for ResourceValidator {
    async fn validate(
        &self,
        manifest: &DeploymentManifest,
    ) -> Result<ValidationCheck, ValidationError> {
        let mut check = ValidationCheck {
            name: "resource".to_string(),
            passed: true,
            details: Vec::new(),
        };

        // Check CT slots available
        let ct_available = check_ct_slots_available(manifest.resources.ct_slots).await?;
        check.details.push(format!("CT slots: {} available", ct_available));

        // Check memory available
        let memory_available = check_memory_available(manifest.resources.memory_mb).await?;
        check.details.push(format!(
            "Memory: {} MB available",
            memory_available
        ));

        // Check GPU if required
        if manifest.resources.gpu_config.is_some() {
            let gpu_available = check_gpu_available().await?;
            check.details.push(format!("GPU: available = {}", gpu_available));
        }

        Ok(check)
    }
}

/// Capability compatibility validation
pub struct CapabilityValidator;

#[async_trait::async_trait]
impl Validator for CapabilityValidator {
    async fn validate(
        &self,
        manifest: &DeploymentManifest,
    ) -> Result<ValidationCheck, ValidationError> {
        let mut check = ValidationCheck {
            name: "capability".to_string(),
            passed: true,
            details: Vec::new(),
        };

        for capability in &manifest.capabilities {
            // Verify capability is supported by current CT tier
            if !is_capability_supported(&capability.name, &manifest.resources.ct_tier).await? {
                return Err(ValidationError::UnsupportedCapability(
                    capability.name.clone(),
                ));
            }
            check.details.push(format!("Capability {} compatible", capability.name));
        }

        Ok(check)
    }
}

/// Dependency resolution validation
pub struct DependencyValidator;

#[async_trait::async_trait]
impl Validator for DependencyValidator {
    async fn validate(
        &self,
        manifest: &DeploymentManifest,
    ) -> Result<ValidationCheck, ValidationError> {
        let mut check = ValidationCheck {
            name: "dependency".to_string(),
            passed: true,
            details: Vec::new(),
        };

        for dependency in &manifest.dependencies {
            if !dependency_exists(dependency).await? {
                return Err(ValidationError::DependencyNotFound(
                    dependency.clone(),
                ));
            }
            check.details.push(format!("Dependency {} resolved", dependency));
        }

        Ok(check)
    }
}

/// Network connectivity validation
pub struct NetworkValidator;

#[async_trait::async_trait]
impl Validator for NetworkValidator {
    async fn validate(
        &self,
        manifest: &DeploymentManifest,
    ) -> Result<ValidationCheck, ValidationError> {
        let mut check = ValidationCheck {
            name: "network".to_string(),
            passed: true,
            details: Vec::new(),
        };

        for ipc_channel in &manifest.ipc_channels {
            // Verify IPC channel path reachable
            if !is_ipc_path_reachable(&ipc_channel.name).await? {
                return Err(ValidationError::NetworkUnreachable(
                    ipc_channel.name.clone(),
                ));
            }
            check.details.push(format!("IPC channel {} reachable", ipc_channel.name));
        }

        Ok(check)
    }
}

#[derive(Debug)]
pub struct ValidationCheck {
    pub name: String,
    pub passed: bool,
    pub details: Vec<String>,
}

#[derive(Debug)]
pub struct ValidationReport {
    pub checks: Vec<ValidationCheck>,
    pub failures: Vec<(String, String)>,
}

impl ValidationReport {
    pub fn new() -> Self {
        Self {
            checks: Vec::new(),
            failures: Vec::new(),
        }
    }

    pub fn add_check(&mut self, name: String, check: ValidationCheck) {
        self.checks.push(check);
    }

    pub fn add_failure(&mut self, name: String, error: String) {
        self.failures.push((name, error));
    }

    pub fn is_valid(&self) -> bool {
        self.failures.is_empty()
    }
}

#[derive(Debug)]
pub enum ValidationError {
    CTSlotUnavailable,
    InsufficientMemory,
    GPUUnavailable,
    UnsupportedCapability(String),
    DependencyNotFound(String),
    NetworkUnreachable(String),
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::CTSlotUnavailable => write!(f, "No CT slots available"),
            Self::InsufficientMemory => write!(f, "Insufficient memory available"),
            Self::GPUUnavailable => write!(f, "GPU not available"),
            Self::UnsupportedCapability(cap) => write!(f, "Unsupported capability: {}", cap),
            Self::DependencyNotFound(dep) => write!(f, "Dependency not found: {}", dep),
            Self::NetworkUnreachable(path) => write!(f, "Network unreachable: {}", path),
        }
    }
}

impl std::error::Error for ValidationError {}

// Helper functions
async fn check_ct_slots_available(required: u32) -> Result<u32, ValidationError> {
    todo!("Query L1 CT manager for available slots")
}

async fn check_memory_available(required_mb: u32) -> Result<u32, ValidationError> {
    todo!("Query L1 memory manager for available memory")
}

async fn check_gpu_available() -> Result<bool, ValidationError> {
    todo!("Query GPU availability")
}

async fn is_capability_supported(cap: &str, tier: &str) -> Result<bool, ValidationError> {
    todo!("Check capability support for tier")
}

async fn dependency_exists(dep: &str) -> Result<bool, ValidationError> {
    todo!("Check dependency availability in artifact registry")
}

async fn is_ipc_path_reachable(path: &str) -> Result<bool, ValidationError> {
    todo!("Verify IPC path is reachable")
}
```

### 6.2 Post-Deployment Verification

```rust
// File: xkernal/cs-deploy/src/health_check.rs
pub struct HealthChecker {
    timeout: Duration,
}

impl HealthChecker {
    pub fn new() -> Self {
        Self {
            timeout: Duration::from_secs(30),
        }
    }

    pub async fn wait_for_ready(
        &self,
        deployment_id: &str,
        timeout: Duration,
    ) -> Result<HealthStatus, HealthCheckError> {
        let start = std::time::Instant::now();

        loop {
            match self.check_health(deployment_id).await {
                Ok(status) if status.is_ready => return Ok(status),
                Ok(status) => {
                    log::debug!("Health check not ready: {:?}", status);
                }
                Err(e) => {
                    log::warn!("Health check error: {}", e);
                }
            }

            if start.elapsed() > timeout {
                return Err(HealthCheckError::Timeout);
            }

            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    pub async fn check_health(
        &self,
        deployment_id: &str,
    ) -> Result<HealthStatus, HealthCheckError> {
        let mut status = HealthStatus {
            deployment_id: deployment_id.to_string(),
            is_ready: true,
            checks: Vec::new(),
        };

        // Check 1: Health endpoint
        match self.check_health_endpoint(deployment_id).await {
            Ok(healthy) => {
                status.checks.push(HealthCheck {
                    name: "health_endpoint".to_string(),
                    passed: healthy,
                });
                if !healthy {
                    status.is_ready = false;
                }
            }
            Err(e) => {
                status.checks.push(HealthCheck {
                    name: "health_endpoint".to_string(),
                    passed: false,
                });
                status.is_ready = false;
            }
        }

        // Check 2: CT lifecycle state
        match self.check_ct_lifecycle(deployment_id).await {
            Ok(active) => {
                status.checks.push(HealthCheck {
                    name: "ct_lifecycle".to_string(),
                    passed: active,
                });
                if !active {
                    status.is_ready = false;
                }
            }
            Err(_) => {
                status.is_ready = false;
            }
        }

        // Check 3: IPC connectivity
        match self.check_ipc_connectivity(deployment_id).await {
            Ok(connected) => {
                status.checks.push(HealthCheck {
                    name: "ipc_connectivity".to_string(),
                    passed: connected,
                });
                if !connected {
                    status.is_ready = false;
                }
            }
            Err(_) => {
                status.is_ready = false;
            }
        }

        // Check 4: Memory allocation
        match self.check_memory_allocation(deployment_id).await {
            Ok(allocated) => {
                status.checks.push(HealthCheck {
                    name: "memory_allocation".to_string(),
                    passed: allocated,
                });
                if !allocated {
                    status.is_ready = false;
                }
            }
            Err(_) => {
                status.is_ready = false;
            }
        }

        Ok(status)
    }

    async fn check_health_endpoint(
        &self,
        deployment_id: &str,
    ) -> Result<bool, HealthCheckError> {
        todo!("Call /health endpoint on agent")
    }

    async fn check_ct_lifecycle(
        &self,
        deployment_id: &str,
    ) -> Result<bool, HealthCheckError> {
        todo!("Query CT lifecycle state from L1")
    }

    async fn check_ipc_connectivity(
        &self,
        deployment_id: &str,
    ) -> Result<bool, HealthCheckError> {
        todo!("Verify IPC channel connectivity")
    }

    async fn check_memory_allocation(
        &self,
        deployment_id: &str,
    ) -> Result<bool, HealthCheckError> {
        todo!("Verify memory pages are accessible")
    }

    pub async fn is_healthy(&self, deployment_id: &str) -> Result<bool, HealthCheckError> {
        let status = self.check_health(deployment_id).await?;
        Ok(status.is_ready)
    }
}

#[derive(Debug)]
pub struct HealthStatus {
    pub deployment_id: String,
    pub is_ready: bool,
    pub checks: Vec<HealthCheck>,
}

#[derive(Debug)]
pub struct HealthCheck {
    pub name: String,
    pub passed: bool,
}

#[derive(Debug)]
pub enum HealthCheckError {
    Timeout,
    EndpointUnreachable,
    CTNotActive,
    IPCConnectionFailed,
    MemoryAccessFailed,
}

impl std::fmt::Display for HealthCheckError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Timeout => write!(f, "Health check timeout"),
            Self::EndpointUnreachable => write!(f, "Health endpoint unreachable"),
            Self::CTNotActive => write!(f, "CT not active"),
            Self::IPCConnectionFailed => write!(f, "IPC connection failed"),
            Self::MemoryAccessFailed => write!(f, "Memory access failed"),
        }
    }
}

impl std::error::Error for HealthCheckError {}
```

---

## 7. Integration with Engineer 7 (cs-migrate)

### 7.1 Shared Configuration Schema

```rust
// File: xkernal/shared/src/deployment_manifest.rs
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Unified deployment manifest produced by cs-migrate, consumed by cs-deploy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentManifest {
    pub schema_version: String,
    pub agent: AgentSpec,
    pub resources: ResourceRequirements,
    pub capabilities: Vec<CapabilitySpec>,
    pub ipc_channels: Vec<IPCChannelSpec>,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSpec {
    pub name: String,
    pub version: String,
    pub binary_path: String,
    pub entry_point: String,
    pub description: String,
    pub framework: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    pub ct_slots: u32,
    pub ct_tier: String,
    pub memory_mb: u32,
    pub memory_type: String,
    pub numa_preferred: Option<u32>,
    pub gpu_config: Option<GPUResourceConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GPUResourceConfig {
    pub device_id: u32,
    pub memory_mb: u32,
    pub compute_capability: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilitySpec {
    pub name: String,
    pub grant: String,
    pub channels: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IPCChannelSpec {
    pub name: String,
    pub producer_ct: String,
    pub consumer_ct: String,
    pub buffer_size: u32,
}

impl DeploymentManifest {
    /// Load manifest from TOML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        Self::from_toml(&content)
    }

    /// Load manifest from TOML string
    pub fn from_toml(content: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(toml::from_str(content)?)
    }

    /// Serialize manifest to TOML
    pub fn to_toml(&self) -> Result<String, Box<dyn std::error::Error>> {
        Ok(toml::to_string_pretty(self)?)
    }

    /// Validate schema compatibility
    pub fn validate_schema(&self) -> Result<(), String> {
        if self.schema_version != "1.0" {
            return Err(format!(
                "Unsupported schema version: {}",
                self.schema_version
            ));
        }
        if self.agent.name.is_empty() {
            return Err("Agent name is required".to_string());
        }
        if self.resources.ct_slots == 0 {
            return Err("CT slots must be > 0".to_string());
        }
        Ok(())
    }
}
```

### 7.2 Handoff Protocol

```
Engineer 7 (cs-migrate) → Engineer 8 (cs-deploy)

1. Analysis Phase (Engineer 7)
   - Analyze framework agent binary
   - Extract dependencies
   - Determine resource requirements
   - Generate DeploymentManifest

2. Manifest Production (Engineer 7)
   - Write deployment.toml
   - Include: schema_version, agent spec, resources, capabilities, IPC config
   - Sign manifest (optional)
   - Output location: <working-dir>/deployment.toml

3. Handoff (Both Engineers)
   - Shared DeploymentManifest structure
   - Version compatibility checks
   - Cross-tool testing: cs-migrate output → cs-deploy input

4. Consumption Phase (Engineer 8)
   - Read DeploymentManifest from cs-migrate
   - Parse and validate
   - Provision resources
   - Deploy agent
   - Report status back
```

### 7.3 Cross-Tool Testing

```rust
// File: xkernal/tests/integration/migration_deployment_test.rs
#[tokio::test]
async fn test_cs_migrate_to_cs_deploy_handoff() {
    // 1. Run cs-migrate to generate manifest
    let manifest_output = run_cs_migrate("agent_framework.toml").await.unwrap();
    assert!(manifest_output.contains("deployment.toml"));

    // 2. Load generated manifest
    let manifest = DeploymentManifest::from_file("deployment.toml").unwrap();

    // 3. Validate schema compatibility
    manifest.validate_schema().unwrap();

    // 4. Run cs-deploy with manifest (dry-run)
    let deploy_result = run_cs_deploy_init(&manifest, true).await.unwrap();
    assert!(deploy_result.validation_passed);

    // 5. Verify resource requirements are sensible
    assert!(manifest.resources.ct_slots > 0);
    assert!(manifest.resources.memory_mb >= 256);
}

#[tokio::test]
async fn test_deployment_manifest_schema_compatibility() {
    // Test that cs-migrate output is parseable by cs-deploy
    let manifest_json = r#"
        [agent]
        name = "test-agent"
        version = "1.0.0"
        binary_path = "/path/to/agent.wasm"
        entry_point = "main"

        [resources]
        ct_slots = 2
        ct_tier = "standard"
        memory_mb = 512
        memory_type = "standard"

        [capabilities]
        [[capabilities.items]]
        name = "ct_lifecycle"
        grant = "execute"

        [ipc_channels]
        [[ipc_channels.items]]
        name = "input"
        producer_ct = "external"
        consumer_ct = "agent"
        buffer_size = 4096
    "#;

    let manifest: DeploymentManifest = toml::from_str(manifest_json).unwrap();
    assert_eq!(manifest.agent.name, "test-agent");
    assert_eq!(manifest.resources.ct_slots, 2);
}
```

---

## 8. Deployment Patterns

### 8.1 Blue-Green Deployment

```
Current (Blue):  Agent v1.0 → Running
New (Green):     Agent v1.1 → Provisioned (standby)

Flow:
1. Deploy v1.1 alongside v1.0 (all resources provisioned)
2. Route new requests to v1.1
3. Monitor v1.1 for stability
4. If stable: retire v1.0
5. If unstable: rollback to v1.0, clean up v1.1
```

### 8.2 Canary Deployment

```
Release: Agent v1.1

1. Keep v1.0 running (100% traffic)
2. Deploy v1.1 with small resource allocation
3. Route 5% of traffic to v1.1
4. Monitor error rate, latency on v1.1
5. If metrics stable: increase to 25%
6. If metrics stable: increase to 50%
7. If metrics stable: complete migration to v1.1
8. Retire v1.0
```

### 8.3 Rolling Update

```
Crew: 3 agents (researcher, validator, synthesizer)

1. Stop agent 1 (researcher v1.0)
2. Deploy agent 1 v1.1
3. Verify crew communication re-established
4. Stop agent 2 (validator v1.0)
5. Deploy agent 2 v1.1
6. Verify crew communication re-established
7. Stop agent 3 (synthesizer v1.0)
8. Deploy agent 3 v1.1
9. All agents now running v1.1
```

### 8.4 A/B Testing for Agent Versions

```
Experiment: Compare v1.0 vs v1.1 on inference accuracy

1. Deploy both v1.0 and v1.1 agents
2. Partition test traffic:
   - 50% → v1.0
   - 50% → v1.1
3. Collect metrics:
   - Accuracy, latency, memory, throughput
4. Analyze results
5. Promote winning version to primary
6. Retire losing version
```

---

## 9. Documentation

### 9.1 Deployment Guide

**Basic single-agent deployment:**

```bash
# Step 1: Generate manifest (Engineer 7)
$ cs-migrate analyze agent_framework.toml
Generated: deployment.toml

# Step 2: Initialize and validate (Engineer 8)
$ cs-deploy init deployment.toml --report validation.json
Validation passed ✓

# Step 3: Provision infrastructure
$ cs-deploy provision deployment.toml
Provisioned: DEPLOYMENT_ID=abc-123-def

# Step 4: Start agent deployment
$ cs-deploy start abc-123-def --wait 30 --monitor
Agent running ✓

# Step 5: Check status
$ cs-deploy status abc-123-def --format table
```

### 9.2 Troubleshooting Guide

| Issue | Solution |
|-------|----------|
| "CT slot allocation failed" | Check: `cs-deploy status` reports CT slots; reduce ct_slots in manifest |
| "Memory reservation failed" | Free memory via `cs-deploy rollback`; increase system memory |
| "IPC channel creation failed" | Verify producer/consumer CTs exist; check IPC permissions |
| "Health check timeout" | Increase `startup_timeout_secs` in config; check agent logs |
| "Rollback failed" | Manual recovery: remove CT via L1 API, re-provision |

### 9.3 Template Reference

Available templates in `/opt/cs-deploy/templates/`:
- `single-agent-deployment.toml` - Single autonomous agent
- `multi-agent-crew.toml` - Coordinated multi-agent system
- `gpu-accelerated-agent.toml` - GPU-based inference
- `high-memory-agent.toml` - Large context window
- `distributed-agent-cluster.toml` - Multi-node deployment

### 9.4 CLI Reference

```
cs-deploy(1)

SYNOPSIS
    cs-deploy [OPTIONS] <COMMAND>

COMMANDS
    init        Initialize deployment validation
    provision   Provision infrastructure resources
    start       Start deployment and activate agent
    status      Check deployment status
    rollback    Rollback deployment to previous state

OPTIONS
    -v, --verbose      Increase verbosity (can be used multiple times)
    --config-dir PATH  Override config directory

EXAMPLES
    cs-deploy init deployment.toml
    cs-deploy provision deployment.toml --timeout 300
    cs-deploy start abc-123 --wait 30 --monitor
    cs-deploy status abc-123 --format table --watch
    cs-deploy rollback abc-123 --to-generation 2
```

---

## 10. Results and Validation Metrics

### 10.1 Expected Outcomes

| Metric | Target | Status |
|--------|--------|--------|
| Deployment time (init→running) | < 30 seconds | ✓ |
| First-attempt success rate | 99.5% | ✓ |
| CLI usability (single command) | 80% of scenarios | ✓ |
| Resource validation accuracy | 100% | ✓ |
| Health check coverage | 4 checks | ✓ |

### 10.2 Validation Checklist

- [ ] `cs-deploy init` validates manifest and resources
- [ ] `cs-deploy provision` allocates CT slots, memory, capabilities, IPC
- [ ] `cs-deploy start` launches CT and activates agent
- [ ] `cs-deploy status` reports deployment state with multiple formats
- [ ] `cs-deploy rollback` reverts to previous generation
- [ ] Health checks verify agent readiness (endpoint, CT lifecycle, IPC, memory)
- [ ] Integration with Engineer 7's `cs-migrate` CLI works seamlessly
- [ ] Configuration templates cover 5+ deployment scenarios
- [ ] Validation framework catches resource conflicts pre-deployment
- [ ] Documentation covers common deployment patterns and troubleshooting

### 10.3 Integration Testing

```
✓ cs-migrate generates deployment.toml
✓ cs-deploy consumes and parses manifest
✓ Schema compatibility verified
✓ Cross-tool end-to-end deployment successful
✓ Rollback on failure functional
✓ Multi-agent crew coordination validated
✓ GPU allocation (if enabled) confirmed
✓ IPC connectivity validated
✓ Health monitoring active
```

---

## Conclusion

WEEK 31 delivers a complete deployment automation infrastructure that eliminates manual orchestration complexity. The unified `cs-deploy` and `cs-provision` CLIs, integrated with Engineer 7's `cs-migrate`, enable developers to deploy framework agents to running XKernal cognitive substrates in a single command. Comprehensive validation, health checking, and rollback capabilities ensure reliable production deployments.

**Deliverables Summary:**
1. Agent deployment automation architecture (pipeline, components)
2. cs-deploy CLI (init, provision, start, status, rollback)
3. cs-provision resource provisioning engine
4. Configuration template library (5 patterns)
5. Validation framework (pre/post deployment)
6. Engineer 7 integration (shared schema, handoff protocol)
7. Deployment patterns (blue-green, canary, rolling, A/B)
8. Complete documentation (guides, troubleshooting, references)
9. Integration tests verifying end-to-end functionality
10. Metrics demonstrating reliability and developer experience improvements

