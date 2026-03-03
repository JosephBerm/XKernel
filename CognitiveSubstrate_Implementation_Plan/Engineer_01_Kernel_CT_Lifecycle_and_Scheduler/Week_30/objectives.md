# Engineer 1 — Kernel: CT Lifecycle & Scheduler — Week 30

## Phase: PHASE 3 — Production Hardening + Launch (Weeks 25-36)

## Weekly Objective
Continue adversarial testing. Attempt deliberate attacks: scheduler starvation, capability escalation, priority inversion exploitation, resource exhaustion attacks.

## Document References
- **Primary:** Section 6.4 (Weeks 28-32: Adversarial testing — attempt deadlock bypass, scheduler starvation, resource exhaustion attacks)
- **Supporting:** Section 3.2.2 (Scheduler fairness), Section 3.2.3 (Capability Enforcement Engine)

## Deliverables
- [ ] Scheduler starvation attack — attempt to starve low-priority CTs, verify they eventually run
- [ ] Capability escalation attack — attempt to escalate privileges, verify kernel blocks
- [ ] Priority inversion exploitation — create scenario where attacker CT achieves priority above legitimate CTs
- [ ] Resource exhaustion attack — attempt denial-of-service via resource exhaustion
- [ ] Deadlock bypass attack — attempt to create undetectable deadlock, verify detection
- [ ] Memory corruption attack — attempt buffer overflows in scheduler data structures
- [ ] Signal spoofing attack — attempt to forge signals, verify only kernel can send
- [ ] IPC tampering attack — attempt to modify IPC messages mid-flight, verify capability/signature checking
- [ ] Documentation — all attack vectors, mitigations, fixes

## Technical Specifications
**Scheduler Starvation Attack:**
- Attack: spawn high-priority CTs repeatedly, prevent low-priority CTs from running
- Mitigation: scheduler must guarantee minimum progress for all CTs (e.g., aging priority or reserve scheduling slots)
- Defense strategy: age low-priority CTs (increment priority over time), or reserve 10% CPU for lowest-priority tier
- Verify: even under repeated high-priority CT spawning, low-priority CTs eventually run

**Capability Escalation Attack:**
- Attack: attempt to grant self a capability beyond parent's authorization
- Mitigation: cap_grant syscall checks Invariant 1 (child capabilities ⊆ parent capabilities)
- Defense strategy: kernel enforces subset check before page table mapping
- Verify: escalation attempt fails with error, no capability granted

**Priority Inversion Exploitation:**
- Attack: create dependency graph designed to elevate attacker CT above legitimate CTs
- Example: attacker CT C depends on low-priority CT A, but many high-priority CTs depend on C
- Exploit: system elevates C's priority to unblock high-priority CTs, C runs at high priority
- Mitigation: priority elevation temporary (only while blocking others), reverts when dependency satisfied
- Verify: attacker cannot sustain high-priority elevation

**Resource Exhaustion Attack (DoS):**
- Attack: spawn unlimited CTs to exhaust system resources (memory, CPU scheduling table, GPU TPC allocations)
- Mitigation: resource quota per Agent (resource_budget field), enforce limit at ct_spawn time
- Defense strategy: if Agent exceeds quota, spawn rejected with BudgetExhausted exception
- Verify: system doesn't crash, rejects spawn when quota exceeded

**Deadlock Bypass Attack:**
- Attack: create complex dependency structure designed to evade cycle detection
- Example: A→B→C→A (simple cycle), or A→B, C→D, B→D, D→A (complex cycle)
- Mitigation: Tarjan's SCC algorithm detects all cycles, not just simple ones
- Verify: all cycles detected, spawn rejected for all cyclic structures

**Memory Corruption Attack:**
- Attack: buffer overflow in scheduler runqueue, priority heap, or dependency graph structures
- Mitigation: Rust memory safety (bounds checking, no unsafe buffer operations outside unsafe blocks)
- Defense strategy: minimize unsafe code; all unsafe must be reviewed and documented
- Verify: no overflow possible via normal syscalls

**Signal Spoofing Attack:**
- Attack: malicious CT attempts to send SIG_TERMINATE to other CTs
- Mitigation: signals only sent by kernel (not userspace), verified at delivery
- Defense strategy: sig_register syscall validates that handler is in CT's own address space
- Verify: user CT cannot send signals to other CTs

**IPC Tampering Attack:**
- Attack: modify IPC message mid-flight (between send and receive)
- Mitigation: messages stored in shared physical pages (same addresses in sender and receiver)
- Defense strategy: pages mapped via page tables (hardware-enforced isolation)
- Verify: modification attempt detectable via page access patterns

## Dependencies
- **Blocked by:** Week 29 (fuzz testing framework), Week 25-28 (baseline and benchmarks)
- **Blocking:** Week 31-32 (security audit and fixes)

## Acceptance Criteria
- [ ] Starvation attack tested and mitigation verified
- [ ] Capability escalation attack blocked
- [ ] Priority inversion exploitation limited
- [ ] Resource exhaustion attack rejected
- [ ] Deadlock bypass attempts blocked
- [ ] Memory corruption attempts prevented
- [ ] Signal spoofing prevented
- [ ] IPC tampering prevented
- [ ] All critical/high findings documented
- [ ] Fixes implemented for any vulnerabilities found

## Design Principles Alignment
- **P3 — Capability-Based Security from Day Zero:** Security testing validates capability model
- **P8 — Fault-Tolerant by Design:** Adversarial testing ensures fault tolerance
