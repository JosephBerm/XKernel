// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! # Copy-on-Write Page Table Forking
//!
//! This module implements Copy-on-Write (CoW) semantics for efficient checkpoint page table
//! forking. When a checkpoint is created, the original page table is cloned such that both
//! the original and checkpoint share the same physical frames initially. On write access,
//! a page fault handler allocates a new frame, copies the original, updates the page table
//! entry, and marks the frame as dirty in the bitmap.
//!
//! ## Architecture
//!
//! - **CoWPageTableFork**: Tracks original page table, checkpoint page table, shared frames,
//!   and dirty bitmap
//! - **fork_page_table_for_checkpoint()**: Clone page table and mark all PTEs read-only
//! - **CoW Fault Handler**: On write fault, allocate new frame, copy content, update PTE,
//!   mark dirty
//! - **Dirty Tracking**: Bitmap tracks which frames have been written since fork
//!
//! ## References
//!
//! - Engineering Plan § 6.3 (Checkpointing - Copy-on-Write)
//! - Week 6 Objective: CoW page table forking

use crate::Result;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;

/// Physical address type representing a machine page frame.
///
/// In a real implementation, this would be a physical memory address.
/// For now, we use u64 to represent physical page frame numbers.
pub type FrameNumber = u64;

/// Virtual address type for page table operations.
pub type VirtualAddr = u64;

/// Page table entry representing a single virtual-to-physical mapping.
///
/// Tracks permissions, presence, and copy-on-write state.
#[derive(Clone, Debug)]
pub struct PageTableEntry {
    /// Physical frame number this entry maps to
    pub frame: FrameNumber,
    /// Is this page present in memory?
    pub present: bool,
    /// Is this page readable?
    pub readable: bool,
    /// Is this page writable?
    pub writable: bool,
    /// Is this page executable?
    pub executable: bool,
    /// Is this page marked copy-on-write?
    pub copy_on_write: bool,
}

impl PageTableEntry {
    /// Create a new page table entry.
    pub fn new(frame: FrameNumber) -> Self {
        Self {
            frame,
            present: true,
            readable: true,
            writable: true,
            executable: false,
            copy_on_write: false,
        }
    }

    /// Mark this entry as read-only (for CoW).
    pub fn set_read_only(&mut self) {
        self.writable = false;
        self.copy_on_write = true;
    }

    /// Restore write permissions after copy-on-write fault.
    pub fn set_writable(&mut self) {
        self.writable = true;
        self.copy_on_write = false;
    }
}

/// A simplified page table representation mapping virtual addresses to page table entries.
///
/// In production, this would be the actual hardware page table structure
/// (e.g., x86-64 4-level paging, ARM TTBR0/TTBR1).
pub type PageTable = BTreeMap<VirtualAddr, PageTableEntry>;

/// Dirty page bitmap tracking which frames have been written since the fork.
///
/// Uses a simple bitvector representation where each bit corresponds to a frame.
pub struct DirtyBitmap {
    /// Bitmap where each bit represents dirty status of a frame
    bitmap: Vec<u64>,
    /// Maximum frame number we can track
    max_frames: u64,
}

impl DirtyBitmap {
    /// Create a new dirty bitmap for the given number of frames.
    pub fn new(max_frames: u64) -> Self {
        let num_words = ((max_frames + 63) / 64) as usize;
        Self {
            bitmap: alloc::vec![0u64; num_words],
            max_frames,
        }
    }

    /// Mark a frame as dirty.
    pub fn mark_dirty(&mut self, frame: FrameNumber) {
        if frame < self.max_frames {
            let word_idx = (frame / 64) as usize;
            let bit_idx = (frame % 64) as u32;
            if word_idx < self.bitmap.len() {
                self.bitmap[word_idx] |= 1u64 << bit_idx;
            }
        }
    }

    /// Check if a frame is dirty.
    pub fn is_dirty(&self, frame: FrameNumber) -> bool {
        if frame < self.max_frames {
            let word_idx = (frame / 64) as usize;
            let bit_idx = (frame % 64) as u32;
            word_idx < self.bitmap.len() && (self.bitmap[word_idx] & (1u64 << bit_idx)) != 0
        } else {
            false
        }
    }

    /// Get the count of dirty frames.
    pub fn dirty_count(&self) -> u64 {
        self.bitmap.iter().map(|w| w.count_ones() as u64).sum()
    }

    /// Clear the dirty bitmap.
    pub fn clear(&mut self) {
        for word in &mut self.bitmap {
            *word = 0;
        }
    }
}

/// Copy-on-Write Page Table Fork tracking state.
///
/// Maintains the original page table, checkpoint page table, and shared frames.
/// Implements efficient memory usage through shared frame tracking.
///
/// See Engineering Plan § 6.3 (Checkpointing - Copy-on-Write)
pub struct CoWPageTableFork {
    /// Original (parent) page table
    original_pt: PageTable,

    /// Checkpoint (forked) page table - initially shares frames with original
    checkpoint_pt: PageTable,

    /// Set of frames shared between original and checkpoint
    shared_frames: alloc::collections::BTreeSet<FrameNumber>,

    /// Bitmap tracking dirty frames in the checkpoint
    dirty_bitmap: DirtyBitmap,

    /// Total number of frames to track
    max_frames: u64,

    /// Whether this fork is still active
    is_active: bool,
}

impl CoWPageTableFork {
    /// Create a new CoW fork from an original page table.
    ///
    /// The checkpoint page table initially shares all frames with the original.
    /// All PTEs are marked read-only in both to trigger faults on write.
    ///
    /// # Arguments
    ///
    /// * `original_pt` - The original page table to fork from
    /// * `max_frames` - Maximum number of frames to track
    ///
    /// # Returns
    ///
    /// A new CoWPageTableFork with all frames initially shared
    pub fn new(original_pt: PageTable, max_frames: u64) -> Self {
        let mut checkpoint_pt = original_pt.clone();

        // Collect all shared frames
        let mut shared_frames = alloc::collections::BTreeSet::new();
        for entry in original_pt.values() {
            shared_frames.insert(entry.frame);
        }

        // Mark all PTEs as read-only in both original and checkpoint
        let mut original_pt_mut = original_pt.clone();
        for entry in original_pt_mut.values_mut() {
            entry.set_read_only();
        }
        for entry in checkpoint_pt.values_mut() {
            entry.set_read_only();
        }

        Self {
            original_pt: original_pt_mut,
            checkpoint_pt,
            shared_frames,
            dirty_bitmap: DirtyBitmap::new(max_frames),
            max_frames,
            is_active: true,
        }
    }

    /// Get a reference to the original page table.
    pub fn original_pt(&self) -> &PageTable {
        &self.original_pt
    }

    /// Get a reference to the checkpoint page table.
    pub fn checkpoint_pt(&self) -> &PageTable {
        &self.checkpoint_pt
    }

    /// Get a mutable reference to the checkpoint page table.
    pub fn checkpoint_pt_mut(&mut self) -> &mut PageTable {
        &mut self.checkpoint_pt
    }

    /// Get the set of currently shared frames.
    pub fn shared_frames(&self) -> &alloc::collections::BTreeSet<FrameNumber> {
        &self.shared_frames
    }

    /// Handle a copy-on-write page fault.
    ///
    /// When a write fault occurs on a CoW page:
    /// 1. Allocate a new frame
    /// 2. Copy content from original frame
    /// 3. Update the checkpoint page table entry
    /// 4. Mark frame as dirty
    /// 5. Remove from shared frames
    ///
    /// # Arguments
    ///
    /// * `va` - Virtual address that caused the fault
    /// * `new_frame` - Newly allocated frame to copy into
    ///
    /// # Returns
    ///
    /// Ok(()) on success, Err(err) if frame not found or fault handling failed
    pub fn handle_cow_fault(&mut self, va: VirtualAddr, new_frame: FrameNumber) -> Result<()> {
        // Find the entry in checkpoint page table
        if let Some(entry) = self.checkpoint_pt.get_mut(&va) {
            let old_frame = entry.frame;

            // SAFETY: This is a logical operation - in production, we would:
            // 1. Allocate new_frame via the physical memory allocator
            // 2. Copy memory from old_frame to new_frame
            // 3. Both would be physical memory operations with proper safety checks
            // For now, we perform the logical update
            entry.frame = new_frame;
            entry.set_writable();

            // Mark the new frame as dirty
            self.dirty_bitmap.mark_dirty(new_frame);

            // Remove from shared frames if this was the last reference
            if !self.shared_frames.contains(&old_frame) {
                return Err(crate::error::CsError::InvalidState(
                    alloc::string::String::from("Frame not in shared set"),
                ));
            }

            // In a real implementation, check reference count before removing
            self.shared_frames.remove(&old_frame);

            Ok(())
        } else {
            Err(crate::error::CsError::InvalidState(
                alloc::format!("Virtual address {:x} not found in page table", va),
            ))
        }
    }

    /// Check if a frame is currently shared.
    pub fn is_shared(&self, frame: FrameNumber) -> bool {
        self.shared_frames.contains(&frame)
    }

    /// Check if a frame has been dirtied since the fork.
    pub fn is_dirty(&self, frame: FrameNumber) -> bool {
        self.dirty_bitmap.is_dirty(frame)
    }

    /// Get the count of dirty frames.
    pub fn dirty_frame_count(&self) -> u64 {
        self.dirty_bitmap.dirty_count()
    }

    /// Get the count of shared frames.
    pub fn shared_frame_count(&self) -> u64 {
        self.shared_frames.len() as u64
    }

    /// Mark the fork as inactive (no longer used).
    pub fn mark_inactive(&mut self) {
        self.is_active = false;
    }

    /// Check if the fork is still active.
    pub fn is_active(&self) -> bool {
        self.is_active
    }
}

/// Fork a page table for checkpoint creation with CoW semantics.
///
/// This function creates a CoW fork of the original page table. Both the original
/// and checkpoint will share the same physical frames initially, with all pages
/// marked read-only. On write access, a page fault will trigger, and a new frame
/// will be allocated and copied.
///
/// # Arguments
///
/// * `original_pt` - The original page table to fork
/// * `max_frames` - Maximum number of frames to track in dirty bitmap
///
/// # Returns
///
/// A new CoWPageTableFork with all frames shared
pub fn fork_page_table_for_checkpoint(
    original_pt: PageTable,
    max_frames: u64,
) -> Result<CoWPageTableFork> {
    Ok(CoWPageTableFork::new(original_pt, max_frames))
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::collections::BTreeSet;
use alloc::format;
use alloc::string::String;
use alloc::vec;

    #[test]
    fn test_page_table_entry_new() {
        let entry = PageTableEntry::new(42);
        assert_eq!(entry.frame, 42);
        assert!(entry.present);
        assert!(entry.readable);
        assert!(entry.writable);
        assert!(!entry.executable);
        assert!(!entry.copy_on_write);
    }

    #[test]
    fn test_page_table_entry_set_read_only() {
        let mut entry = PageTableEntry::new(42);
        entry.set_read_only();
        assert!(!entry.writable);
        assert!(entry.copy_on_write);
    }

    #[test]
    fn test_page_table_entry_set_writable() {
        let mut entry = PageTableEntry::new(42);
        entry.set_read_only();
        entry.set_writable();
        assert!(entry.writable);
        assert!(!entry.copy_on_write);
    }

    #[test]
    fn test_dirty_bitmap_new() {
        let bitmap = DirtyBitmap::new(256);
        assert_eq!(bitmap.dirty_count(), 0);
    }

    #[test]
    fn test_dirty_bitmap_mark_and_check() {
        let mut bitmap = DirtyBitmap::new(256);
        bitmap.mark_dirty(42);
        assert!(bitmap.is_dirty(42));
        assert!(!bitmap.is_dirty(43));
        assert_eq!(bitmap.dirty_count(), 1);
    }

    #[test]
    fn test_dirty_bitmap_multiple() {
        let mut bitmap = DirtyBitmap::new(256);
        for i in 0..10 {
            bitmap.mark_dirty(i);
        }
        assert_eq!(bitmap.dirty_count(), 10);
        for i in 0..10 {
            assert!(bitmap.is_dirty(i));
        }
    }

    #[test]
    fn test_dirty_bitmap_clear() {
        let mut bitmap = DirtyBitmap::new(256);
        for i in 0..10 {
            bitmap.mark_dirty(i);
        }
        bitmap.clear();
        assert_eq!(bitmap.dirty_count(), 0);
    }

    #[test]
    fn test_cow_fork_new() {
        let mut original_pt = BTreeMap::new();
        original_pt.insert(0x1000, PageTableEntry::new(1));
        original_pt.insert(0x2000, PageTableEntry::new(2));

        let fork = CoWPageTableFork::new(original_pt, 256);
        assert_eq!(fork.shared_frame_count(), 2);
        assert!(fork.is_active());
    }

    #[test]
    fn test_cow_fork_read_only_on_creation() {
        let mut original_pt = BTreeMap::new();
        original_pt.insert(0x1000, PageTableEntry::new(1));

        let fork = CoWPageTableFork::new(original_pt, 256);
        let entry = fork.checkpoint_pt().get(&0x1000).unwrap();
        assert!(!entry.writable);
        assert!(entry.copy_on_write);
    }

    #[test]
    fn test_cow_fork_handle_fault() {
        let mut original_pt = BTreeMap::new();
        original_pt.insert(0x1000, PageTableEntry::new(1));

        let mut fork = CoWPageTableFork::new(original_pt, 256);
        assert!(fork.handle_cow_fault(0x1000, 100).is_ok());

        let entry = fork.checkpoint_pt().get(&0x1000).unwrap();
        assert_eq!(entry.frame, 100);
        assert!(entry.writable);
        assert!(!entry.copy_on_write);
        assert!(fork.is_dirty(100));
        assert!(!fork.is_shared(1));
    }

    #[test]
    fn test_cow_fork_handle_fault_multiple() {
        let mut original_pt = BTreeMap::new();
        original_pt.insert(0x1000, PageTableEntry::new(1));
        original_pt.insert(0x2000, PageTableEntry::new(2));

        let mut fork = CoWPageTableFork::new(original_pt, 256);
        assert!(fork.handle_cow_fault(0x1000, 100).is_ok());
        assert!(fork.handle_cow_fault(0x2000, 200).is_ok());

        assert_eq!(fork.dirty_frame_count(), 2);
        assert_eq!(fork.shared_frame_count(), 0);
    }

    #[test]
    fn test_fork_page_table_for_checkpoint() {
        let mut original_pt = BTreeMap::new();
        original_pt.insert(0x1000, PageTableEntry::new(1));
        original_pt.insert(0x2000, PageTableEntry::new(2));

        let fork = fork_page_table_for_checkpoint(original_pt, 256).unwrap();
        assert_eq!(fork.shared_frame_count(), 2);
        assert!(fork.is_active());
    }

    #[test]
    fn test_cow_fork_mark_inactive() {
        let mut original_pt = BTreeMap::new();
        original_pt.insert(0x1000, PageTableEntry::new(1));

        let mut fork = CoWPageTableFork::new(original_pt, 256);
        assert!(fork.is_active());
        fork.mark_inactive();
        assert!(!fork.is_active());
    }
}
