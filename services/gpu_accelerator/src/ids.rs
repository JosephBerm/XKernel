// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! Strongly-typed resource identifiers for GPU Manager.
//!
//! This module defines unique identifiers for GPU resources, ensuring
//! type safety and preventing accidental mixing of different resource types.
//! All IDs use the ULID format for sortability and uniqueness.
//!
//! Reference: Engineering Plan § GPU Device Management

use alloc::fmt;

/// Unique identifier for a GPU device (NVIDIA or AMD).
///
/// Obtained from runtime device enumeration and bound to physical GPU hardware.
/// Used throughout the GPU Manager to reference a specific GPU in resource requests.
///
/// # Example
///
/// ```text
/// GpuDeviceID("01ARZ3NDEKTSV4RRFFQ69G5FAV")
/// ```
///
/// Reference: Engineering Plan § Device Registry
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GpuDeviceID([u8; 16]);

impl GpuDeviceID {
    /// Create a new GpuDeviceID from a 16-byte array.
    ///
    /// # Arguments
    ///
    /// * `bytes` - 16-byte ULID representation
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        GpuDeviceID(bytes)
    }

    /// Get the inner bytes.
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

impl fmt::Display for GpuDeviceID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GpuDeviceID({:?})", self.0)
    }
}

/// Unique identifier for a Streaming Multiprocessor (SM) or Tensor Processing Cluster (TPC).
///
/// TPC-level scheduling is fundamental to the GPU Manager architecture (LithOS-inspired).
/// Each TPC/SM can be independently allocated to different crews with distinct priorities.
///
/// Reference: Engineering Plan § TPC-Level Scheduling
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TpcID {
    /// GPU device this TPC belongs to
    pub device_id: GpuDeviceID,
    /// TPC index within the device (0 to tpc_count - 1)
    pub index: u32,
}

impl TpcID {
    /// Create a new TPC identifier.
    pub const fn new(device_id: GpuDeviceID, index: u32) -> Self {
        TpcID { device_id, index }
    }
}

impl fmt::Display for TpcID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TpcID({:?}:{})", self.device_id.as_bytes(), self.index)
    }
}

/// Unique identifier for a VRAM region allocated to a crew.
///
/// VRAM regions are the primary isolation mechanism. Each crew's KV-cache
/// and intermediate tensors are confined to a VRAM region with configurable
/// isolation mode (Strict, Selective, or Open).
///
/// Reference: Engineering Plan § VRAM Isolation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VramRegionID([u8; 16]);

impl VramRegionID {
    /// Create a new VramRegionID from a 16-byte array.
    ///
    /// # Arguments
    ///
    /// * `bytes` - 16-byte ULID representation
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        VramRegionID(bytes)
    }

    /// Get the inner bytes.
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

impl fmt::Display for VramRegionID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VramRegionID({:?})", self.0)
    }
}

/// Unique identifier for a kernel launch task.
///
/// Each kernel launch (forward pass segment, attention, MLP) is assigned a unique ID
/// for tracking, preemption, and checkpoint/restore operations.
///
/// Reference: Engineering Plan § Kernel Atomization
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct KernelLaunchID([u8; 16]);

impl KernelLaunchID {
    /// Create a new KernelLaunchID from a 16-byte array.
    ///
    /// # Arguments
    ///
    /// * `bytes` - 16-byte ULID representation
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        KernelLaunchID(bytes)
    }

    /// Get the inner bytes.
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

impl fmt::Display for KernelLaunchID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "KernelLaunchID({:?})", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_device_id_creation() {
        let bytes = [0u8; 16];
        let id = GpuDeviceID::from_bytes(bytes);
        assert_eq!(id.as_bytes(), &bytes);
    }

    #[test]
    fn test_gpu_device_id_equality() {
        let id1 = GpuDeviceID::from_bytes([1u8; 16]);
        let id2 = GpuDeviceID::from_bytes([1u8; 16]);
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_gpu_device_id_ordering() {
        let id1 = GpuDeviceID::from_bytes([1u8; 16]);
        let id2 = GpuDeviceID::from_bytes([2u8; 16]);
        assert!(id1 < id2);
    }

    #[test]
    fn test_tpc_id_creation() {
        let device_id = GpuDeviceID::from_bytes([0u8; 16]);
        let tpc_id = TpcID::new(device_id, 42);
        assert_eq!(tpc_id.device_id, device_id);
        assert_eq!(tpc_id.index, 42);
    }

    #[test]
    fn test_tpc_id_ordering() {
        let device_id = GpuDeviceID::from_bytes([0u8; 16]);
        let tpc_id1 = TpcID::new(device_id, 1);
        let tpc_id2 = TpcID::new(device_id, 2);
        assert!(tpc_id1 < tpc_id2);
    }

    #[test]
    fn test_vram_region_id_creation() {
        let bytes = [0u8; 16];
        let id = VramRegionID::from_bytes(bytes);
        assert_eq!(id.as_bytes(), &bytes);
    }

    #[test]
    fn test_kernel_launch_id_creation() {
        let bytes = [0u8; 16];
        let id = KernelLaunchID::from_bytes(bytes);
        assert_eq!(id.as_bytes(), &bytes);
    }

    #[test]
    fn test_id_hashing() {
        use alloc::collections::BTreeMap;
use alloc::format;

        let id1 = GpuDeviceID::from_bytes([1u8; 16]);
        let id2 = GpuDeviceID::from_bytes([2u8; 16]);

        let mut map = BTreeMap::new();
        map.insert(id1, "device1");
        map.insert(id2, "device2");

        assert_eq!(map.get(&id1), Some(&"device1"));
        assert_eq!(map.get(&id2), Some(&"device2"));
    }

    #[test]
    fn test_display_formatting() {
        let device_id = GpuDeviceID::from_bytes([1u8; 16]);
        let display_str = format!("{}", device_id);
        assert!(display_str.contains("GpuDeviceID"));

        let tpc_id = TpcID::new(device_id, 5);
        let display_str = format!("{}", tpc_id);
        assert!(display_str.contains("TpcID"));
    }
}
