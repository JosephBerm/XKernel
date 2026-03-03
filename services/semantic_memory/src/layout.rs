// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Memory layout specifications for the 3-tier hierarchy.
//!
//! This module defines the physical memory layout constraints and configuration
//! for each tier, including address ranges, page sizes, and replication factors.
//!
//! See Engineering Plan § 4.1.0: Memory Layout & Physical Addressing.

use alloc::string::String;

/// L1 Working Memory (GPU-local HBM) layout specification.
///
/// Defines the physical address range and layout constraints for L1.
/// Target: 2-8GB HBM per compute thread.
///
/// See Engineering Plan § 4.1.1: L1 Physical Layout.
#[derive(Clone, Debug)]
pub struct L1Layout {
    /// Base address of L1 memory region
    pub base_address: u64,

    /// Total size in bytes (typical: 2-8 GB per GPU)
    pub size_bytes: u64,

    /// Page size for L1 (4KB or 2MB huge pages)
    pub page_size: u64,

    /// Whether this layout uses transparent huge pages
    pub use_huge_pages: bool,
}

impl L1Layout {
    /// Creates a new L1 layout specification.
    ///
    /// # Arguments
    ///
    /// * `base_address` - Starting address
    /// * `size_bytes` - Total capacity
    /// * `page_size` - Physical page size (4096 or 2097152)
    pub fn new(base_address: u64, size_bytes: u64, page_size: u64) -> Self {
        let use_huge_pages = page_size == 2 * 1024 * 1024; // 2MB threshold

        L1Layout {
            base_address,
            size_bytes,
            page_size,
            use_huge_pages,
        }
    }

    /// Creates a default L1 layout (8GB, 4KB pages).
    pub fn default_8gb_hbm() -> Self {
        L1Layout {
            base_address: 0x0000_0000_0000_0000,
            size_bytes: 8 * 1024 * 1024 * 1024, // 8GB
            page_size: 4 * 1024,                  // 4KB pages
            use_huge_pages: false,
        }
    }

    /// Creates an L1 layout with 2MB huge pages (better TLB performance).
    pub fn with_huge_pages(size_bytes: u64) -> Self {
        L1Layout {
            base_address: 0x0000_0000_0000_0000,
            size_bytes,
            page_size: 2 * 1024 * 1024, // 2MB pages
            use_huge_pages: true,
        }
    }

    /// Returns the end address of this L1 region.
    pub fn end_address(&self) -> u64 {
        self.base_address.saturating_add(self.size_bytes)
    }

    /// Returns the number of pages.
    pub fn page_count(&self) -> u64 {
        self.size_bytes.saturating_div(self.page_size)
    }

    /// Returns cost of page table update (O(1) for HBM).
    pub fn remapping_cost_ns(&self) -> u64 {
        100 // ~100ns per page table update
    }
}

/// L2 Episodic Memory (Host DRAM) layout specification.
///
/// Defines the physical memory range and segment layout for L2.
/// Target: 16-64GB DRAM per agent.
///
/// See Engineering Plan § 4.1.2: L2 Physical Layout.
#[derive(Clone, Debug)]
pub struct L2Layout {
    /// Base address of L2 memory region
    pub base_address: u64,

    /// Total size in bytes (typical: 16-64 GB per agent)
    pub size_bytes: u64,

    /// Segment size for L2 regions (typically 64MB or 256MB)
    pub segment_size: u64,

    /// Number of segments
    pub segment_count: u64,
}

impl L2Layout {
    /// Creates a new L2 layout specification.
    ///
    /// # Arguments
    ///
    /// * `base_address` - Starting address
    /// * `size_bytes` - Total capacity
    /// * `segment_size` - Size of each addressable segment
    pub fn new(base_address: u64, size_bytes: u64, segment_size: u64) -> Self {
        let segment_count = size_bytes.saturating_div(segment_size);

        L2Layout {
            base_address,
            size_bytes,
            segment_size,
            segment_count,
        }
    }

    /// Creates a default L2 layout (32GB, 256MB segments).
    pub fn default_32gb_dram() -> Self {
        L2Layout {
            base_address: 0x0100_0000_0000_0000,
            size_bytes: 32 * 1024 * 1024 * 1024, // 32GB
            segment_size: 256 * 1024 * 1024,     // 256MB segments
            segment_count: 128,
        }
    }

    /// Creates an L2 layout with 64MB segments (for smaller agents).
    pub fn compact_16gb_64mb_segments() -> Self {
        L2Layout {
            base_address: 0x0100_0000_0000_0000,
            size_bytes: 16 * 1024 * 1024 * 1024, // 16GB
            segment_size: 64 * 1024 * 1024,      // 64MB segments
            segment_count: 256,
        }
    }

    /// Returns the end address of this L2 region.
    pub fn end_address(&self) -> u64 {
        self.base_address.saturating_add(self.size_bytes)
    }

    /// Returns whether an address falls within this L2 layout.
    pub fn contains_address(&self, address: u64) -> bool {
        address >= self.base_address && address < self.end_address()
    }
}

/// L3 Long-Term Memory (NVMe persistent) layout specification.
///
/// Defines the mount point, block layout, and replication for L3.
/// Target: 1TB+ NVMe with crew-wide replication.
///
/// See Engineering Plan § 4.1.3: L3 Physical Layout.
#[derive(Clone, Debug)]
pub struct L3Layout {
    /// Mount point or path to L3 storage
    pub mount_point: String,

    /// Total size in bytes (typical: 1TB+ per crew)
    pub size_bytes: u64,

    /// Block size for I/O operations (typically 4KB or 64KB)
    pub block_size: u64,

    /// Replication factor (3 for crew-wide durability)
    pub replication_factor: u32,

    /// Number of replicas across different physical locations
    pub distribution_zones: u32,
}

impl L3Layout {
    /// Creates a new L3 layout specification.
    ///
    /// # Arguments
    ///
    /// * `mount_point` - Path to L3 storage
    /// * `size_bytes` - Total capacity
    /// * `block_size` - I/O block size
    /// * `replication_factor` - Number of replicas
    pub fn new(
        mount_point: impl Into<String>,
        size_bytes: u64,
        block_size: u64,
        replication_factor: u32,
    ) -> Self {
        L3Layout {
            mount_point: mount_point.into(),
            size_bytes,
            block_size,
            replication_factor,
            distribution_zones: replication_factor.max(1),
        }
    }

    /// Creates a default L3 layout (1TB NVMe, 3-way replication).
    pub fn default_1tb_nvme() -> Self {
        L3Layout {
            mount_point: "/mnt/nvme/l3".to_string(),
            size_bytes: 1024 * 1024 * 1024 * 1024, // 1TB
            block_size: 4 * 1024,                   // 4KB blocks
            replication_factor: 3,                  // 3-way crew replication
            distribution_zones: 3,
        }
    }

    /// Returns the number of blocks.
    pub fn block_count(&self) -> u64 {
        self.size_bytes.saturating_div(self.block_size)
    }

    /// Returns the total storage capacity accounting for replication.
    pub fn effective_capacity(&self) -> u64 {
        self.size_bytes.saturating_div(self.replication_factor as u64)
    }
}

/// Page granularity and remapping cost specification.
///
/// Defines how page table updates are costed in the memory system.
///
/// See Engineering Plan § 4.1.0: Page Table Management.
#[derive(Clone, Debug)]
pub struct PageGranularity {
    /// Physical page size in bytes (typically 4096 or 2097152)
    pub physical_page_size: u64,

    /// Cost of remapping a single page in nanoseconds
    pub remapping_cost_ns: u64,

    /// Whether this tier supports transparent huge pages
    pub supports_huge_pages: bool,
}

impl PageGranularity {
    /// Creates a new page granularity specification.
    pub fn new(physical_page_size: u64, remapping_cost_ns: u64) -> Self {
        let supports_huge_pages = physical_page_size > 4 * 1024;

        PageGranularity {
            physical_page_size,
            remapping_cost_ns,
            supports_huge_pages,
        }
    }

    /// Creates 4KB page specification (standard, O(1) remapping).
    pub fn page_4kb_standard() -> Self {
        PageGranularity {
            physical_page_size: 4 * 1024,
            remapping_cost_ns: 100,
            supports_huge_pages: false,
        }
    }

    /// Creates 2MB huge page specification (better TLB, O(1) remapping).
    pub fn page_2mb_huge() -> Self {
        PageGranularity {
            physical_page_size: 2 * 1024 * 1024,
            remapping_cost_ns: 100,
            supports_huge_pages: true,
        }
    }

    /// Returns the cost to remap N pages.
    pub fn total_remapping_cost_ns(&self, page_count: u64) -> u64 {
        page_count.saturating_mul(self.remapping_cost_ns)
    }
}

/// Memory bounds specification for a tier.
///
/// Defines minimum, maximum, and preferred sizes for memory allocation.
///
/// See Engineering Plan § 4.1.0: Memory Bounds.
#[derive(Clone, Debug)]
pub struct MemoryBound {
    /// Minimum allocation size in bytes
    pub min_bytes: u64,

    /// Maximum allocation size in bytes
    pub max_bytes: u64,

    /// Preferred allocation size (for optimization hints)
    pub preferred_bytes: u64,
}

impl MemoryBound {
    /// Creates a new memory bound specification.
    pub fn new(min_bytes: u64, max_bytes: u64, preferred_bytes: u64) -> Self {
        MemoryBound {
            min_bytes,
            max_bytes,
            preferred_bytes,
        }
    }

    /// Creates bounds for L1 (2-8GB range).
    pub fn l1_typical() -> Self {
        MemoryBound {
            min_bytes: 2 * 1024 * 1024 * 1024,  // 2GB minimum
            max_bytes: 8 * 1024 * 1024 * 1024,  // 8GB maximum
            preferred_bytes: 4 * 1024 * 1024 * 1024, // 4GB preferred
        }
    }

    /// Creates bounds for L2 (16-64GB range).
    pub fn l2_typical() -> Self {
        MemoryBound {
            min_bytes: 16 * 1024 * 1024 * 1024,  // 16GB minimum
            max_bytes: 64 * 1024 * 1024 * 1024,  // 64GB maximum
            preferred_bytes: 32 * 1024 * 1024 * 1024, // 32GB preferred
        }
    }

    /// Creates bounds for L3 (1TB+ range).
    pub fn l3_typical() -> Self {
        MemoryBound {
            min_bytes: 512 * 1024 * 1024 * 1024,      // 512GB minimum
            max_bytes: 10 * 1024 * 1024 * 1024 * 1024, // 10TB maximum
            preferred_bytes: 1024 * 1024 * 1024 * 1024, // 1TB preferred
        }
    }

    /// Returns whether a size is within bounds.
    pub fn contains_size(&self, size_bytes: u64) -> bool {
        size_bytes >= self.min_bytes && size_bytes <= self.max_bytes
    }

    /// Clamps a size to be within bounds.
    pub fn clamp_size(&self, size_bytes: u64) -> u64 {
        if size_bytes < self.min_bytes {
            self.min_bytes
        } else if size_bytes > self.max_bytes {
            self.max_bytes
        } else {
            size_bytes
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::string::ToString;

    #[test]
    fn test_l1_layout_creation() {
        let layout = L1Layout::new(0x0000_0000_0000_0000, 8 * 1024 * 1024 * 1024, 4 * 1024);

        assert_eq!(layout.base_address, 0);
        assert_eq!(layout.size_bytes, 8 * 1024 * 1024 * 1024);
        assert_eq!(layout.page_size, 4 * 1024);
        assert!(!layout.use_huge_pages);
    }

    #[test]
    fn test_l1_layout_end_address() {
        let layout = L1Layout::new(0x1000, 0x1000, 4 * 1024);
        assert_eq!(layout.end_address(), 0x2000);
    }

    #[test]
    fn test_l1_layout_page_count() {
        let layout = L1Layout::new(0, 4 * 1024 * 1024, 4 * 1024);
        assert_eq!(layout.page_count(), 1024); // 4MB / 4KB
    }

    #[test]
    fn test_l1_layout_default() {
        let layout = L1Layout::default_8gb_hbm();
        assert_eq!(layout.size_bytes, 8 * 1024 * 1024 * 1024);
        assert_eq!(layout.page_size, 4 * 1024);
    }

    #[test]
    fn test_l1_layout_with_huge_pages() {
        let layout = L1Layout::with_huge_pages(8 * 1024 * 1024 * 1024);
        assert_eq!(layout.page_size, 2 * 1024 * 1024);
        assert!(layout.use_huge_pages);
    }

    #[test]
    fn test_l2_layout_creation() {
        let layout = L2Layout::new(0x0100_0000_0000_0000, 32 * 1024 * 1024 * 1024, 256 * 1024 * 1024);

        assert_eq!(layout.base_address, 0x0100_0000_0000_0000);
        assert_eq!(layout.size_bytes, 32 * 1024 * 1024 * 1024);
        assert_eq!(layout.segment_size, 256 * 1024 * 1024);
        assert_eq!(layout.segment_count, 128);
    }

    #[test]
    fn test_l2_layout_end_address() {
        let layout = L2Layout::new(0x1000, 0x2000, 0x1000);
        assert_eq!(layout.end_address(), 0x3000);
    }

    #[test]
    fn test_l2_layout_contains_address() {
        let layout = L2Layout::new(0x1000, 0x1000, 0x100);

        assert!(layout.contains_address(0x1000));
        assert!(layout.contains_address(0x1500));
        assert!(!layout.contains_address(0x2000));
        assert!(!layout.contains_address(0x500));
    }

    #[test]
    fn test_l2_layout_default() {
        let layout = L2Layout::default_32gb_dram();
        assert_eq!(layout.size_bytes, 32 * 1024 * 1024 * 1024);
        assert_eq!(layout.segment_count, 128);
    }

    #[test]
    fn test_l3_layout_creation() {
        let layout = L3Layout::new("/mnt/nvme/l3", 1024 * 1024 * 1024 * 1024, 4 * 1024, 3);

        assert_eq!(layout.mount_point, "/mnt/nvme/l3");
        assert_eq!(layout.size_bytes, 1024 * 1024 * 1024 * 1024);
        assert_eq!(layout.block_size, 4 * 1024);
        assert_eq!(layout.replication_factor, 3);
    }

    #[test]
    fn test_l3_layout_block_count() {
        let layout = L3Layout::new("/mnt/nvme", 4 * 1024, 1024, 1);
        assert_eq!(layout.block_count(), 4); // 4KB / 1KB
    }

    #[test]
    fn test_l3_layout_effective_capacity() {
        let layout = L3Layout::new("/mnt/nvme", 1024 * 1024 * 1024, 4 * 1024, 3);
        assert_eq!(layout.effective_capacity(), (1024 * 1024 * 1024) / 3);
    }

    #[test]
    fn test_l3_layout_default() {
        let layout = L3Layout::default_1tb_nvme();
        assert_eq!(layout.size_bytes, 1024 * 1024 * 1024 * 1024);
        assert_eq!(layout.replication_factor, 3);
    }

    #[test]
    fn test_page_granularity_4kb() {
        let pg = PageGranularity::page_4kb_standard();
        assert_eq!(pg.physical_page_size, 4 * 1024);
        assert_eq!(pg.remapping_cost_ns, 100);
        assert!(!pg.supports_huge_pages);
    }

    #[test]
    fn test_page_granularity_2mb() {
        let pg = PageGranularity::page_2mb_huge();
        assert_eq!(pg.physical_page_size, 2 * 1024 * 1024);
        assert!(pg.supports_huge_pages);
    }

    #[test]
    fn test_page_granularity_total_cost() {
        let pg = PageGranularity::page_4kb_standard();
        assert_eq!(pg.total_remapping_cost_ns(100), 10_000);
    }

    #[test]
    fn test_memory_bound_creation() {
        let bound = MemoryBound::new(1024, 4096, 2048);

        assert_eq!(bound.min_bytes, 1024);
        assert_eq!(bound.max_bytes, 4096);
        assert_eq!(bound.preferred_bytes, 2048);
    }

    #[test]
    fn test_memory_bound_l1() {
        let bound = MemoryBound::l1_typical();
        assert_eq!(bound.min_bytes, 2 * 1024 * 1024 * 1024);
        assert_eq!(bound.max_bytes, 8 * 1024 * 1024 * 1024);
    }

    #[test]
    fn test_memory_bound_l2() {
        let bound = MemoryBound::l2_typical();
        assert_eq!(bound.min_bytes, 16 * 1024 * 1024 * 1024);
        assert_eq!(bound.max_bytes, 64 * 1024 * 1024 * 1024);
    }

    #[test]
    fn test_memory_bound_l3() {
        let bound = MemoryBound::l3_typical();
        assert_eq!(bound.min_bytes, 512 * 1024 * 1024 * 1024);
    }

    #[test]
    fn test_memory_bound_contains_size() {
        let bound = MemoryBound::new(1024, 4096, 2048);

        assert!(bound.contains_size(1024));
        assert!(bound.contains_size(2048));
        assert!(bound.contains_size(4096));
        assert!(!bound.contains_size(512));
        assert!(!bound.contains_size(8192));
    }

    #[test]
    fn test_memory_bound_clamp_size() {
        let bound = MemoryBound::new(1000, 5000, 3000);

        assert_eq!(bound.clamp_size(500), 1000);    // Below min -> min
        assert_eq!(bound.clamp_size(3000), 3000);   // Within bounds
        assert_eq!(bound.clamp_size(6000), 5000);   // Above max -> max
    }
}
