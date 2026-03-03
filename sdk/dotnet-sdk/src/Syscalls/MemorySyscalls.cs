// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.

#nullable enable

namespace CognitiveSubstrate.SDK.Syscalls;

using System.Threading.Tasks;
using Types;

/// <summary>
/// Memory family syscalls for memory management.
/// 
/// Syscalls:
/// - MemAllocAsync (0x0100): Allocate a memory region
/// - MemFreeAsync (0x0101): Free a memory region
/// - MemMountAsync (0x0102): Mount a memory region at a path
/// - MemUnmountAsync (0x0103): Unmount a memory region
/// 
/// Total: 4 syscalls
/// </summary>
public static class MemorySyscalls
{
    /// <summary>
    /// Allocate a memory region (mem_alloc).
    /// Syscall number: 0x0100
    /// </summary>
    public static Task<MemoryRegionId> MemAllocAsync(
        ulong size,
        ulong? alignment = null,
        uint? flags = null)
    {
        throw new CsciException(
            CsciErrorCode.Unimplemented,
            "MemAllocAsync is not yet implemented");
    }

    /// <summary>
    /// Free a memory region (mem_free).
    /// Syscall number: 0x0101
    /// </summary>
    public static Task MemFreeAsync(MemoryRegionId regionId)
    {
        throw new CsciException(
            CsciErrorCode.Unimplemented,
            "MemFreeAsync is not yet implemented");
    }

    /// <summary>
    /// Mount a memory region (mem_mount).
    /// Syscall number: 0x0102
    /// </summary>
    public static Task MemMountAsync(
        MemoryRegionId regionId,
        string mountPoint)
    {
        throw new CsciException(
            CsciErrorCode.Unimplemented,
            "MemMountAsync is not yet implemented");
    }

    /// <summary>
    /// Unmount a memory region (mem_unmount).
    /// Syscall number: 0x0103
    /// </summary>
    public static Task MemUnmountAsync(MemoryRegionId regionId)
    {
        throw new CsciException(
            CsciErrorCode.Unimplemented,
            "MemUnmountAsync is not yet implemented");
    }
}
