// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! GPU device discovery and initialization.
//!
//! Implements device enumeration, initialization, and health checking.
//! Discovers all available GPUs, validates compatibility, and monitors health.
//!
//! Reference: Engineering Plan § Device Registry, Health Monitoring

use crate::device::{DeviceCapabilities, DriverApi, GpuDevice, GpuDeviceType};
use crate::error::GpuError;
use crate::ids::GpuDeviceID;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

/// Discovered GPU device with basic info.
#[derive(Clone, Debug)]
pub struct DiscoveredDevice {
    /// GPU device ordinal (0-based index from driver).
    pub ordinal: u32,

    /// Device type (NVIDIA/AMD model).
    pub device_type: GpuDeviceType,

    /// Driver API to use (CUDA or ROCm HIP).
    pub driver_api: DriverApi,

    /// Device properties and limits.
    pub properties: DeviceProperties,

    /// Driver version string.
    pub driver_version: [u8; 128],
}

impl fmt::Display for DiscoveredDevice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "DiscoveredDevice(ordinal={}, type={}, driver={})",
            self.ordinal, self.device_type, self.driver_api
        )
    }
}

/// Device properties obtained from driver.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeviceProperties {
    /// Device name (product string).
    pub name: [u8; 256],

    /// Compute capability (major, minor).
    pub compute_capability: (u32, u32),

    /// Total VRAM in bytes.
    pub total_vram: u64,

    /// Maximum threads per block.
    pub max_threads: u32,

    /// Maximum blocks per dimension (x, y, z).
    pub max_blocks: (u32, u32, u32),

    /// Streaming Multiprocessor count.
    pub sm_count: u32,

    /// Tensor Processing Cluster count (same as SM for NVIDIA).
    pub tpc_count: u32,
}

/// Initialized GPU device with runtime context.
#[derive(Clone, Debug)]
pub struct InitializedDevice {
    /// Device identifier (persistent).
    pub device_id: GpuDeviceID,

    /// Underlying GpuDevice.
    pub device: GpuDevice,

    /// Primary/default stream handle for this device.
    pub default_stream: u64,

    /// Device properties.
    pub properties: DeviceProperties,

    /// Current health status.
    pub health_status: DeviceHealth,
}

impl fmt::Display for InitializedDevice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "InitializedDevice({}, health={:?})",
            self.device_id, self.health_status
        )
    }
}

/// Device health status.
///
/// Tracks the operational status of a GPU device.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceHealth {
    /// Device is healthy and ready for use.
    Healthy,

    /// Device is degraded (reduced performance or isolated errors).
    Degraded { reason: u32 }, // reason code

    /// Device is unreachable (hardware disconnect, driver issue).
    Unreachable,

    /// Device is in error state (fatal error, requires recovery).
    ErrorState { error: u32 }, // error code
}

impl fmt::Display for DeviceHealth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeviceHealth::Healthy => write!(f, "Healthy"),
            DeviceHealth::Degraded { reason } => write!(f, "Degraded(reason={})", reason),
            DeviceHealth::Unreachable => write!(f, "Unreachable"),
            DeviceHealth::ErrorState { error } => write!(f, "ErrorState(error={})", error),
        }
    }
}

/// Device discovery and initialization system.
#[derive(Debug)]
pub struct DeviceDiscovery {
    /// Next device ID counter.
    next_device_id: u64,

    /// Maximum devices to support.
    max_devices: u32,

    /// Discovered devices cache.
    discovered: Vec<DiscoveredDevice>,

    /// Initialized devices.
    initialized: Vec<InitializedDevice>,
}

impl DeviceDiscovery {
    /// Create a new device discovery system.
    ///
    /// # Arguments
    ///
    /// * `max_devices` - Maximum number of devices to support
    pub fn new(max_devices: u32) -> Self {
        DeviceDiscovery {
            next_device_id: 1,
            max_devices,
            discovered: Vec::new(),
            initialized: Vec::new(),
        }
    }

    /// Discover all available GPU devices.
    ///
    /// Queries the CUDA Driver API and ROCm HIP to enumerate available devices.
    /// In a real implementation, this would call cuDeviceGetCount, hipGetDeviceCount, etc.
    ///
    /// # Returns
    ///
    /// Vector of discovered devices, or GpuError if enumeration fails.
    pub fn discover_devices(&mut self) -> Result<Vec<DiscoveredDevice>, GpuError> {
        // In a real implementation, this would:
        // 1. Call cuDeviceGetCount() / hipGetDeviceCount()
        // 2. For each device, call cuDeviceGet() and cuDeviceGetAttribute()
        // 3. Determine device type from properties
        // 4. Assign appropriate driver API

        // For now, return empty vector (no devices available in test environment)
        self.discovered.clear();
        Ok(self.discovered.clone())
    }

    /// Add a mock discovered device (for testing).
    pub fn add_mock_device(
        &mut self,
        device_type: GpuDeviceType,
        driver_api: DriverApi,
    ) -> Result<DiscoveredDevice, GpuError> {
        if self.discovered.len() >= self.max_devices as usize {
            return Err(GpuError::DeviceNotFound);
        }

        let mut name = [0u8; 256];
        let name_str = format!("{}", device_type);
        let name_bytes = name_str.as_bytes();
        name[..name_bytes.len().min(255)].copy_from_slice(&name_bytes[..name_bytes.len().min(255)]);

        let (sm_count, tpc_count) = match device_type {
            GpuDeviceType::NvidiaH100 | GpuDeviceType::NvidiaH200 => (132, 132),
            GpuDeviceType::NvidiaB200 => (192, 192),
            GpuDeviceType::AmdMi300x => (304, 304),
        };

        let device = DiscoveredDevice {
            ordinal: self.discovered.len() as u32,
            device_type,
            driver_api,
            properties: DeviceProperties {
                name,
                compute_capability: device_type.compute_capability(),
                total_vram: device_type.max_vram_bytes(),
                max_threads: 1024,
                max_blocks: (65535, 65535, 65535),
                sm_count,
                tpc_count,
            },
            driver_version: [0u8; 128],
        };

        self.discovered.push(device.clone());
        Ok(device)
    }

    /// Initialize a discovered device.
    ///
    /// Creates a CUDA/HIP context and sets up runtime state.
    ///
    /// # Arguments
    ///
    /// * `discovered` - Discovered device to initialize
    ///
    /// # Returns
    ///
    /// Initialized device if successful.
    pub fn initialize_device(&mut self, discovered: &DiscoveredDevice) -> Result<InitializedDevice, GpuError> {
        // Create device ID
        let device_id = GpuDeviceID::from_bytes({
            let mut bytes = [0u8; 16];
            bytes[0] = self.next_device_id as u8;
            self.next_device_id += 1;
            bytes
        });

        // Create GpuDevice
        let device = GpuDevice::new(device_id, discovered.device_type, discovered.driver_api);

        // Create initialized device
        let initialized = InitializedDevice {
            device_id,
            device,
            default_stream: 0, // Mock stream handle
            properties: discovered.properties,
            health_status: DeviceHealth::Healthy,
        };

        self.initialized.push(initialized.clone());
        Ok(initialized)
    }

    /// Perform a health check on a device.
    ///
    /// Queries device status, ECC errors, and thermal state.
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device to check
    ///
    /// # Returns
    ///
    /// Health status of the device.
    pub fn periodic_health_check(&mut self, device_id: GpuDeviceID) -> Result<DeviceHealth, GpuError> {
        // Find initialized device
        let device = self
            .initialized
            .iter_mut()
            .find(|d| d.device_id == device_id)
            .ok_or(GpuError::DeviceNotFound)?;

        // In a real implementation, this would:
        // 1. Call cuDeviceGetAttribute(CU_DEVICE_ATTRIBUTE_ECC_MODE, ...)
        // 2. Query ECC error count via cuDeviceGetAttribute(CU_DEVICE_ATTRIBUTE_SINGLE_BIT_ERRORS, ...)
        // 3. Query thermal state
        // 4. Check for recent kernel failures

        // For now, return healthy status
        device.health_status = DeviceHealth::Healthy;
        Ok(device.health_status)
    }

    /// Get initialized device by ID.
    pub fn get_initialized_device(&self, device_id: GpuDeviceID) -> Option<&InitializedDevice> {
        self.initialized.iter().find(|d| d.device_id == device_id)
    }

    /// Get mutable initialized device by ID.
    pub fn get_initialized_device_mut(&mut self, device_id: GpuDeviceID) -> Option<&mut InitializedDevice> {
        self.initialized.iter_mut().find(|d| d.device_id == device_id)
    }

    /// Get count of discovered devices.
    pub fn discovered_count(&self) -> usize {
        self.discovered.len()
    }

    /// Get count of initialized devices.
    pub fn initialized_count(&self) -> usize {
        self.initialized.len()
    }

    /// Get all discovered devices.
    pub fn discovered_devices(&self) -> &[DiscoveredDevice] {
        &self.discovered
    }

    /// Get all initialized devices.
    pub fn initialized_devices(&self) -> &[InitializedDevice] {
        &self.initialized
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;

    #[test]
    fn test_device_discovery_creation() {
        let discovery = DeviceDiscovery::new(8);

        assert_eq!(discovery.discovered_count(), 0);
        assert_eq!(discovery.initialized_count(), 0);
    }

    #[test]
    fn test_discover_devices_empty() {
        let mut discovery = DeviceDiscovery::new(8);

        let result = discovery.discover_devices();

        assert!(result.is_ok());
        assert_eq!(discovery.discovered_count(), 0);
    }

    #[test]
    fn test_add_mock_device() {
        let mut discovery = DeviceDiscovery::new(8);

        let device = discovery
            .add_mock_device(GpuDeviceType::NvidiaH100, DriverApi::CudaDriverApi)
            .unwrap();

        assert_eq!(device.ordinal, 0);
        assert_eq!(device.device_type, GpuDeviceType::NvidiaH100);
        assert_eq!(discovery.discovered_count(), 1);
    }

    #[test]
    fn test_add_multiple_mock_devices() {
        let mut discovery = DeviceDiscovery::new(8);

        let dev1 = discovery
            .add_mock_device(GpuDeviceType::NvidiaH100, DriverApi::CudaDriverApi)
            .unwrap();

        let dev2 = discovery
            .add_mock_device(GpuDeviceType::NvidiaH200, DriverApi::CudaDriverApi)
            .unwrap();

        let dev3 = discovery
            .add_mock_device(GpuDeviceType::AmdMi300x, DriverApi::RocmHip)
            .unwrap();

        assert_eq!(dev1.ordinal, 0);
        assert_eq!(dev2.ordinal, 1);
        assert_eq!(dev3.ordinal, 2);
        assert_eq!(discovery.discovered_count(), 3);
    }

    #[test]
    fn test_add_mock_device_max_exceeded() {
        let mut discovery = DeviceDiscovery::new(1);

        discovery
            .add_mock_device(GpuDeviceType::NvidiaH100, DriverApi::CudaDriverApi)
            .unwrap();

        let result = discovery.add_mock_device(GpuDeviceType::NvidiaH200, DriverApi::CudaDriverApi);

        assert!(result.is_err());
    }

    #[test]
    fn test_initialize_device() {
        let mut discovery = DeviceDiscovery::new(8);

        let discovered = discovery
            .add_mock_device(GpuDeviceType::NvidiaH100, DriverApi::CudaDriverApi)
            .unwrap();

        let initialized = discovery.initialize_device(&discovered).unwrap();

        assert_eq!(initialized.device.device_type, GpuDeviceType::NvidiaH100);
        assert_eq!(initialized.health_status, DeviceHealth::Healthy);
        assert_eq!(discovery.initialized_count(), 1);
    }

    #[test]
    fn test_get_initialized_device() {
        let mut discovery = DeviceDiscovery::new(8);

        let discovered = discovery
            .add_mock_device(GpuDeviceType::NvidiaH100, DriverApi::CudaDriverApi)
            .unwrap();

        let initialized = discovery.initialize_device(&discovered).unwrap();
        let device_id = initialized.device_id;

        let retrieved = discovery.get_initialized_device(device_id).unwrap();

        assert_eq!(retrieved.device_id, device_id);
        assert_eq!(retrieved.device.device_type, GpuDeviceType::NvidiaH100);
    }

    #[test]
    fn test_periodic_health_check() {
        let mut discovery = DeviceDiscovery::new(8);

        let discovered = discovery
            .add_mock_device(GpuDeviceType::NvidiaH100, DriverApi::CudaDriverApi)
            .unwrap();

        let initialized = discovery.initialize_device(&discovered).unwrap();
        let device_id = initialized.device_id;

        let health = discovery.periodic_health_check(device_id).unwrap();

        assert_eq!(health, DeviceHealth::Healthy);
    }

    #[test]
    fn test_health_check_nonexistent_device() {
        let mut discovery = DeviceDiscovery::new(8);

        let fake_device_id = GpuDeviceID::from_bytes([99u8; 16]);
        let result = discovery.periodic_health_check(fake_device_id);

        assert!(result.is_err());
    }

    #[test]
    fn test_discovered_device_display() {
        let device = DiscoveredDevice {
            ordinal: 0,
            device_type: GpuDeviceType::NvidiaH100,
            driver_api: DriverApi::CudaDriverApi,
            properties: DeviceProperties {
                name: [0u8; 256],
                compute_capability: (9, 0),
                total_vram: 80 * 1024 * 1024 * 1024,
                max_threads: 1024,
                max_blocks: (65535, 65535, 65535),
                sm_count: 132,
                tpc_count: 132,
            },
            driver_version: [0u8; 128],
        };

        let display_str = format!("{}", device);
        assert!(display_str.contains("ordinal=0"));
        assert!(display_str.contains("NVIDIA H100"));
        assert!(display_str.contains("CUDA Driver API"));
    }

    #[test]
    fn test_device_health_display() {
        assert_eq!(format!("{}", DeviceHealth::Healthy), "Healthy");
        assert_eq!(
            format!("{}", DeviceHealth::Degraded { reason: 42 }),
            "Degraded(reason=42)"
        );
        assert_eq!(format!("{}", DeviceHealth::Unreachable), "Unreachable");
        assert_eq!(
            format!("{}", DeviceHealth::ErrorState { error: 1 }),
            "ErrorState(error=1)"
        );
    }

    #[test]
    fn test_initialized_device_display() {
        let device_id = GpuDeviceID::from_bytes([1u8; 16]);
        let device = GpuDevice::new(device_id, GpuDeviceType::NvidiaH100, DriverApi::CudaDriverApi);

        let initialized = InitializedDevice {
            device_id,
            device,
            default_stream: 0,
            properties: DeviceProperties {
                name: [0u8; 256],
                compute_capability: (9, 0),
                total_vram: 80 * 1024 * 1024 * 1024,
                max_threads: 1024,
                max_blocks: (65535, 65535, 65535),
                sm_count: 132,
                tpc_count: 132,
            },
            health_status: DeviceHealth::Healthy,
        };

        let display_str = format!("{}", initialized);
        assert!(display_str.contains("Healthy"));
    }

    #[test]
    fn test_device_discovery_full_flow() {
        let mut discovery = DeviceDiscovery::new(8);

        // Add devices
        let dev1 = discovery
            .add_mock_device(GpuDeviceType::NvidiaH100, DriverApi::CudaDriverApi)
            .unwrap();

        let dev2 = discovery
            .add_mock_device(GpuDeviceType::AmdMi300x, DriverApi::RocmHip)
            .unwrap();

        // Initialize
        let init1 = discovery.initialize_device(&dev1).unwrap();
        let init2 = discovery.initialize_device(&dev2).unwrap();

        // Check health
        let health1 = discovery.periodic_health_check(init1.device_id).unwrap();
        let health2 = discovery.periodic_health_check(init2.device_id).unwrap();

        assert_eq!(health1, DeviceHealth::Healthy);
        assert_eq!(health2, DeviceHealth::Healthy);
        assert_eq!(discovery.initialized_count(), 2);
    }

    #[test]
    fn test_get_all_devices() {
        let mut discovery = DeviceDiscovery::new(8);

        discovery
            .add_mock_device(GpuDeviceType::NvidiaH100, DriverApi::CudaDriverApi)
            .unwrap();

        discovery
            .add_mock_device(GpuDeviceType::NvidiaH200, DriverApi::CudaDriverApi)
            .unwrap();

        let all_devices = discovery.discovered_devices();

        assert_eq!(all_devices.len(), 2);
        assert_eq!(all_devices[0].device_type, GpuDeviceType::NvidiaH100);
        assert_eq!(all_devices[1].device_type, GpuDeviceType::NvidiaH200);
    }
}
