# Engineer 1 — Kernel: CT Lifecycle & Scheduler — Week 03

## Phase: PHASE 0 — Domain Model + Kernel Skeleton (Weeks 1-6)

## Weekly Objective
Implement baseline round-robin scheduler and boot the microkernel on QEMU. Spawn and schedule first 100 CognitiveTasks with phase transitions logged.

## Document References
- **Primary:** Section 3.2.1 (Boot Sequence), Section 3.2.2 (Cognitive Priority Scheduler — CPU Scheduling overview)
- **Supporting:** Section 3.1 (Layered Architecture overview), Section 2.1 (CT phase lifecycle)

## Deliverables
- [ ] Boot sequence implementation — UEFI firmware loader, MMU initialization, interrupt controller configuration
- [ ] Physical memory manager — page frame allocator, memory map scanning
- [ ] Virtual memory setup — kernel page tables, MMU enable
- [ ] Round-robin scheduler in Rust — O(n) fair distribution, no priority yet
- [ ] CT spawn syscall (ct_spawn) — allocate ULID, initialize phase to spawn, transition to plan
- [ ] CT yield syscall (ct_yield) — yield to scheduler, allow other CTs to run
- [ ] Trace logging — every phase transition logged with timestamp
- [ ] QEMU boot test passing — microkernel boots, initializes, spawns init CT
- [ ] Integration test — spawn 100 CTs, round-robin schedule 50 rounds, verify all phase transitions logged

## Technical Specifications
**Boot Sequence (Section 3.2.1):**
1. UEFI firmware loads kernel binary
2. Kernel initializes:
   - Physical memory manager — scans memory map from firmware, builds page frame allocator
   - Virtual memory — sets up kernel page tables, enables MMU (CR3/TTBR0)
   - Interrupt controller — configures APIC (x86-64) or GIC (ARM64), registers timer and fault handlers
   - GPU device interface — enumerates GPUs (stub for now), maps MMIO regions
   - Cognitive scheduler — creates init CT, enters scheduling loop

**Round-Robin Scheduler:**
- Maintain runqueue of CTs in runnable state (phase ≠ failed and ≠ complete)
- Each CT gets time quantum (default 10ms)
- On timer interrupt, switch to next CT in queue
- Log every context switch with timestamps
- Target: no scheduling latency >100µs under normal load

**Phase Transition Logging:**
- Every transition logged immediately to trace_log with: source phase, dest phase, timestamp, CT ID, reason
- Trace storage: kernel ring buffer, 1MB initial capacity

## Dependencies
- **Blocked by:** Week 01 (domain model), Week 02 (phase machine, CSCI spec)
- **Blocking:** Week 04 (DAG dependency handling), Week 05 (capability integration)

## Acceptance Criteria
- [ ] Microkernel boots on QEMU x86-64 (or ARM64 if available)
- [ ] Init CT spawned and scheduled
- [ ] 100 CTs spawned and round-robin scheduled for 50 complete rounds (5000 context switches)
- [ ] All phase transitions correctly logged (spawn→plan→reason→act→reflect→yield→complete)
- [ ] No page faults, no memory corruption
- [ ] Total boot time to first user CT execution <500ms
- [ ] Scheduler latency histogram shows p99 <10µs

## Design Principles Alignment
- **P2 — Cognitive Primitives as Kernel Abstractions:** CT scheduling is fundamental kernel responsibility, not userspace library
- **P7 — Production-Grade from Phase 1:** Round-robin baseline, even simple, is production-ready (no best-effort fairness)
- **P5 — Observable by Default:** Full phase transition tracing from boot
