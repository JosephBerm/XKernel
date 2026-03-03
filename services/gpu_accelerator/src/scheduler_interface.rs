// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! Cognitive Scheduler ↔ GPU Manager interface.
//!
//! Defines the bidirectional communication protocol between the Cognitive Scheduler
//! and GPU Manager. Scheduler sends directives to allocate/release resources;
//! GPU Manager responds with feedback on utilization, thermal state, and power.
//!
//! Reference: Engineering Plan § Scheduler Integration, IPC Protocol

use crate::error::GpuError;
use crate::ids::TpcID;
use core::fmt;

/// Directive from Cognitive Scheduler to GPU Manager.
///
/// Commands the GPU Manager to perform resource management operations.
/// Directives are prioritized and may be queued if resource constraints exist.
///
/// Reference: Engineering Plan § Scheduling Directives
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SchedulerDirective {
    /// Allocate TPC (Tensor Processing Cluster) resources to a crew.
    ///
    /// Reserves TPCs for exclusive use by the specified crew.
    /// May fail if insufficient TPCs are available.
    AllocateTpcs {
        /// TPCs to allocate
        tpc_ids: [u8; 16], // Opaque TPC allocation vector (actual is dynamic)
        /// Crew identifier
        crew_id: [u8; 16],
        /// Priority (0=lowest, 255=highest)
        priority: u8,
    },

    /// Release TPC resources previously allocated to a crew.
    ///
    /// Deallocates TPCs and frees resources for reallocation.
    ReleaseTpcs {
        /// Crew to release TPCs from
        crew_id: [u8; 16],
        /// TPC count to release (or 0 for all)
        count: u32,
    },

    /// Preempt a currently executing kernel.
    ///
    /// Interrupts kernel execution (if preemption points available).
    /// May initiate checkpoint if recovery needed.
    PreemptKernel {
        /// Crew owning the kernel
        crew_id: [u8; 16],
    },

    /// Migrate workload from one GPU to another.
    ///
    /// Checkpoint on source GPU, restore on target GPU.
    MigrateWorkload {
        /// Source device
        source_device_id: [u8; 16],
        /// Target device
        target_device_id: [u8; 16],
        /// Crew to migrate
        crew_id: [u8; 16],
    },

    /// Checkpoint GPU state for a crew.
    ///
    /// Save crew's GPU state (VRAM, registers) for later recovery.
    CheckpointGpu {
        /// Crew to checkpoint
        crew_id: [u8; 16],
        /// Priority of checkpoint operation
        priority: u8,
    },

    /// Restore GPU state from a checkpoint.
    ///
    /// Restore previously saved crew state to GPU.
    RestoreGpu {
        /// Crew to restore
        crew_id: [u8; 16],
        /// Checkpoint identifier
        checkpoint_id: [u8; 16],
    },
}

impl fmt::Display for SchedulerDirective {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SchedulerDirective::AllocateTpcs { crew_id, priority, .. } => {
                write!(f, "AllocateTpcs(crew={:?}, priority={})", &crew_id[..4], priority)
            }
            SchedulerDirective::ReleaseTpcs { crew_id, count } => {
                write!(f, "ReleaseTpcs(crew={:?}, count={})", &crew_id[..4], count)
            }
            SchedulerDirective::PreemptKernel { crew_id } => {
                write!(f, "PreemptKernel(crew={:?})", &crew_id[..4])
            }
            SchedulerDirective::MigrateWorkload {
                source_device_id,
                target_device_id,
                crew_id,
            } => {
                write!(
                    f,
                    "MigrateWorkload(crew={:?}, src={:?}, dst={:?})",
                    &crew_id[..4], &source_device_id[..4], &target_device_id[..4]
                )
            }
            SchedulerDirective::CheckpointGpu { crew_id, priority } => {
                write!(f, "CheckpointGpu(crew={:?}, priority={})", &crew_id[..4], priority)
            }
            SchedulerDirective::RestoreGpu { crew_id, checkpoint_id } => {
                write!(f, "RestoreGpu(crew={:?}, checkpoint={:?})", &crew_id[..4], &checkpoint_id[..4])
            }
        }
    }
}

/// Response from GPU Manager to Scheduler for a directive.
///
/// Indicates success or failure of a directive execution.
/// Contains result-specific data (allocated TPCs, feedback metrics).
///
/// Reference: Engineering Plan § Scheduler Response Protocol
#[derive(Clone, Debug)]
pub enum SchedulerResponse {
    /// TPC allocation succeeded.
    Allocated {
        /// TPCs actually allocated (may be subset if insufficient resources)
        tpc_count: u32,
        /// Time taken to allocate in nanoseconds
        allocation_time_ns: u64,
    },

    /// TPC release succeeded.
    Released {
        /// TPCs released
        tpc_count: u32,
    },

    /// Kernel preemption succeeded.
    Preempted {
        /// Time taken to preempt in nanoseconds
        preempt_time_ns: u64,
    },

    /// Workload migration completed.
    Migrated {
        /// Time taken to migrate in nanoseconds
        migration_time_ns: u64,
    },

    /// Checkpoint operation succeeded.
    Checkpointed {
        /// Checkpoint ID
        checkpoint_id: [u8; 16],
        /// Checkpoint size in bytes
        size_bytes: u64,
        /// Time taken in nanoseconds
        checkpoint_time_ns: u64,
    },

    /// Restore operation succeeded.
    Restored {
        /// Time taken to restore in nanoseconds
        restore_time_ns: u64,
    },

    /// Operation failed.
    Failed(GpuError),
}

impl fmt::Display for SchedulerResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SchedulerResponse::Allocated {
                tpc_count,
                allocation_time_ns,
            } => write!(f, "Allocated(tpcs={}, time={}ns)", tpc_count, allocation_time_ns),
            SchedulerResponse::Released { tpc_count } => write!(f, "Released(tpcs={})", tpc_count),
            SchedulerResponse::Preempted { preempt_time_ns } => write!(f, "Preempted(time={}ns)", preempt_time_ns),
            SchedulerResponse::Migrated { migration_time_ns } => write!(f, "Migrated(time={}ns)", migration_time_ns),
            SchedulerResponse::Checkpointed {
                checkpoint_id,
                size_bytes,
                checkpoint_time_ns,
            } => write!(
                f,
                "Checkpointed(id={:?}, size={}B, time={}ns)",
                &checkpoint_id[..4], size_bytes, checkpoint_time_ns
            ),
            SchedulerResponse::Restored { restore_time_ns } => write!(f, "Restored(time={}ns)", restore_time_ns),
            SchedulerResponse::Failed(err) => write!(f, "Failed({})", err),
        }
    }
}

/// Feedback metrics from GPU Manager to Scheduler.
///
/// Provides operational metrics for scheduler decision-making:
/// - Resource utilization (VRAM, TPCs)
/// - Thermal state
/// - Power consumption
/// - Queue depth
///
/// Reference: Engineering Plan § Performance Feedback
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GpuFeedback {
    /// TPC utilization percentage (0-100).
    ///
    /// Portion of TPCs actively executing kernels.
    pub utilization_percent: u32,

    /// VRAM used in bytes.
    ///
    /// Total allocated VRAM across all crews.
    pub vram_used_bytes: u64,

    /// Thermal throttling active (true if temperature limiting performance).
    pub thermal_throttling: bool,

    /// Current power consumption in watts.
    ///
    /// Estimated from kernel activity and voltage rails.
    pub power_watts: u32,

    /// Number of kernels pending execution in queue.
    pub queue_depth: u32,
}

impl fmt::Display for GpuFeedback {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GpuFeedback(util={}%, vram={}B, thermal={}, power={}W, queue={})",
            self.utilization_percent, self.vram_used_bytes, self.thermal_throttling, self.power_watts, self.queue_depth
        )
    }
}

/// Priority level for scheduler directives and operations.
///
/// Used to prioritize directive execution and resource allocation.
/// Higher priority operations are processed first.
///
/// Reference: Engineering Plan § Directive Prioritization
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DirectivePriority {
    /// Background: lowest priority, deferred operations (cleanup, telemetry).
    Background = 0,

    /// Low: routine maintenance operations.
    Low = 1,

    /// Normal: standard user workloads.
    Normal = 2,

    /// High: latency-sensitive inference requests.
    High = 3,

    /// Critical: system maintenance, emergency recovery.
    Critical = 4,
}

impl fmt::Display for DirectivePriority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DirectivePriority::Background => write!(f, "Background"),
            DirectivePriority::Low => write!(f, "Low"),
            DirectivePriority::Normal => write!(f, "Normal"),
            DirectivePriority::High => write!(f, "High"),
            DirectivePriority::Critical => write!(f, "Critical"),
        }
    }
}

/// Scheduler ↔ GPU Manager interface (trait-based contract).
///
/// Defines the operations the GPU Manager exposes to the Scheduler.
/// Implementations are responsible for queuing, prioritization, and execution.
///
/// Reference: Engineering Plan § Scheduler Interface
pub trait SchedulerGpuInterface: core::fmt::Debug {
    /// Submit a directive to the GPU Manager.
    ///
    /// Queues the directive for execution with specified priority.
    /// Returns immediately (asynchronous execution).
    ///
    /// # Arguments
    ///
    /// * `directive` - Command to execute
    /// * `priority` - Priority level for execution
    fn submit_directive(&self, directive: SchedulerDirective, priority: DirectivePriority) -> Result<(), GpuError>;

    /// Get current GPU feedback metrics.
    ///
    /// Returns snapshot of current operational metrics for scheduler decision-making.
    fn get_feedback(&self) -> Result<GpuFeedback, GpuError>;

    /// Get detailed device status.
    ///
    /// Returns comprehensive status information about a device.
    fn get_device_status(&self, device_id: [u8; 16]) -> Result<DeviceStatus, GpuError>;
}

/// Detailed status snapshot of a GPU device.
///
/// Comprehensive view of device state for scheduler monitoring.
///
/// Reference: Engineering Plan § Device Monitoring
#[derive(Clone, Copy, Debug)]
pub struct DeviceStatus {
    /// Device identifier
    pub device_id: [u8; 16],

    /// Total VRAM in bytes
    pub total_vram_bytes: u64,

    /// Available (unallocated) VRAM in bytes
    pub available_vram_bytes: u64,

    /// Total TPC count
    pub total_tpcs: u32,

    /// Allocated TPC count
    pub allocated_tpcs: u32,

    /// Current temperature in Celsius (0 if not available)
    pub temperature_celsius: u32,

    /// Maximum safe temperature in Celsius
    pub max_temperature_celsius: u32,

    /// Current frequency in MHz
    pub frequency_mhz: u32,

    /// Maximum frequency in MHz
    pub max_frequency_mhz: u32,

    /// Current voltage in millivolts
    pub voltage_mv: u32,

    /// Device health status (0=healthy, 1=warning, 2=critical)
    pub health_status: u8,
}

impl fmt::Display for DeviceStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "DeviceStatus(id={:?}, vram={}/{}, tpcs={}/{}, temp={}C, health={})",
            &self.device_id[..4],
            self.available_vram_bytes,
            self.total_vram_bytes,
            self.allocated_tpcs,
            self.total_tpcs,
            self.temperature_celsius,
            self.health_status
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;

    #[test]
    fn test_scheduler_directive_allocate_tpcs() {
        let directive = SchedulerDirective::AllocateTpcs {
            tpc_ids: [1u8; 16],
            crew_id: [2u8; 16],
            priority: 128,
        };

        let display_str = format!("{}", directive);
        assert!(display_str.contains("AllocateTpcs"));
    }

    #[test]
    fn test_scheduler_directive_release_tpcs() {
        let directive = SchedulerDirective::ReleaseTpcs {
            crew_id: [1u8; 16],
            count: 10,
        };

        let display_str = format!("{}", directive);
        assert!(display_str.contains("ReleaseTpcs"));
        assert!(display_str.contains("10"));
    }

    #[test]
    fn test_scheduler_directive_preempt_kernel() {
        let directive = SchedulerDirective::PreemptKernel { crew_id: [1u8; 16] };

        let display_str = format!("{}", directive);
        assert!(display_str.contains("PreemptKernel"));
    }

    #[test]
    fn test_scheduler_directive_migrate_workload() {
        let directive = SchedulerDirective::MigrateWorkload {
            source_device_id: [1u8; 16],
            target_device_id: [2u8; 16],
            crew_id: [3u8; 16],
        };

        let display_str = format!("{}", directive);
        assert!(display_str.contains("MigrateWorkload"));
    }

    #[test]
    fn test_scheduler_directive_checkpoint() {
        let directive = SchedulerDirective::CheckpointGpu {
            crew_id: [1u8; 16],
            priority: 10,
        };

        let display_str = format!("{}", directive);
        assert!(display_str.contains("CheckpointGpu"));
    }

    #[test]
    fn test_scheduler_response_allocated() {
        let response = SchedulerResponse::Allocated {
            tpc_count: 32,
            allocation_time_ns: 50000,
        };

        let display_str = format!("{}", response);
        assert!(display_str.contains("Allocated"));
        assert!(display_str.contains("32"));
    }

    #[test]
    fn test_scheduler_response_failed() {
        let response = SchedulerResponse::Failed(GpuError::VramExhausted);

        let display_str = format!("{}", response);
        assert!(display_str.contains("Failed"));
    }

    #[test]
    fn test_gpu_feedback_creation() {
        let feedback = GpuFeedback {
            utilization_percent: 75,
            vram_used_bytes: 40_000_000_000,
            thermal_throttling: false,
            power_watts: 450,
            queue_depth: 5,
        };

        let display_str = format!("{}", feedback);
        assert!(display_str.contains("75%"));
        assert!(display_str.contains("450W"));
    }

    #[test]
    fn test_directive_priority_ordering() {
        assert!(DirectivePriority::Critical > DirectivePriority::High);
        assert!(DirectivePriority::High > DirectivePriority::Normal);
        assert!(DirectivePriority::Normal > DirectivePriority::Low);
        assert!(DirectivePriority::Low > DirectivePriority::Background);
    }

    #[test]
    fn test_directive_priority_display() {
        assert_eq!(format!("{}", DirectivePriority::Critical), "Critical");
        assert_eq!(format!("{}", DirectivePriority::Normal), "Normal");
        assert_eq!(format!("{}", DirectivePriority::Background), "Background");
    }

    #[test]
    fn test_device_status_creation() {
        let status = DeviceStatus {
            device_id: [1u8; 16],
            total_vram_bytes: 80_000_000_000,
            available_vram_bytes: 40_000_000_000,
            total_tpcs: 132,
            allocated_tpcs: 66,
            temperature_celsius: 45,
            max_temperature_celsius: 80,
            frequency_mhz: 2400,
            max_frequency_mhz: 2500,
            voltage_mv: 800,
            health_status: 0,
        };

        let display_str = format!("{}", status);
        assert!(display_str.contains("DeviceStatus"));
        assert!(display_str.contains("132"));
    }

    #[test]
    fn test_directive_equality() {
        let d1 = SchedulerDirective::ReleaseTpcs {
            crew_id: [1u8; 16],
            count: 5,
        };
        let d2 = SchedulerDirective::ReleaseTpcs {
            crew_id: [1u8; 16],
            count: 5,
        };

        assert_eq!(d1, d2);
    }

    #[test]
    fn test_checkpoint_response() {
        let response = SchedulerResponse::Checkpointed {
            checkpoint_id: [1u8; 16],
            size_bytes: 1_000_000,
            checkpoint_time_ns: 5_000_000,
        };

        let display_str = format!("{}", response);
        assert!(display_str.contains("Checkpointed"));
    }
}
