# Engineer 5 — Services: GPU/Accelerator Manager — Week 05

## Phase: 0 (GPU Command Submission Queue)
## Weekly Objective
Implement GPU command submission queue infrastructure via CUDA streams and kernel launch APIs. Enable inference frameworks (vLLM, TensorRT-LLM) to submit GPU work through kernel interface. Establish async execution model with completion notification.

## Document References
- **Primary:** Section 3.3.2 — GPU/Accelerator Manager, Command Submission Queue subsection
- **Supporting:** Section 6.1 — Phase 0, Week 5-6

## Deliverables
- [ ] Command submission queue data structure (CUDA stream / HIP stream queue, entry format, kernel tracking)
- [ ] Kernel submission API: Submit GPU kernel launch via cuLaunchKernel/hipLaunchKernel with context, model ID, CT context
- [ ] Command entry format specification (kernel function handle, thread block count, grid/block dims, VRAM args)
- [ ] Async execution with completion callbacks (cuEventRecord/hipEventRecord for completion signals → CT resumption)
- [ ] Inference framework integration API (vLLM and TensorRT-LLM kernel submission wrappers using CUDA streams)
- [ ] Completion notification mechanism (GPU events via cuEventSynchronize/hipEventSynchronize)
- [ ] Error handling: Malformed command detection, timeout detection (cuStreamQuery), GPU fault reporting
- [ ] Unit tests: Stream operation, kernel submission/completion cycle, error conditions

## Technical Specifications
- Command queue: Kernel-managed CUDA stream / HIP stream (cuStreamCreate / hipStreamCreate)
- Command entry: Kernel function handle, thread blocks, shared memory size, grid/block dimensions, CUDA context
- Submission path: Userspace framework → syscall → GPU Manager → cuLaunchKernel (CUDA) or hipLaunchKernel (ROCm)
- Async model: Framework continues; GPU Manager monitors stream via cuStreamQuery / hipStreamQuery; event signals completion
- Completion feedback: GPU Manager updates CT state via GPU event; Cognitive Scheduler resumes waiting CTs
- Error handling: Stream synchronization for hung kernels; GPU event query for error detection

## Dependencies
- **Blocked by:** Week 4 (GPU Manager skeleton, single-model VRAM)
- **Blocking:** Week 6 (Phase 0 completion), Week 7-8 (TPC-Level Spatial Scheduling)

## Acceptance Criteria
- [ ] CUDA stream / HIP stream tested with kernel launches
- [ ] Async completion verified in single-model scenario (cuEventSynchronize)
- [ ] Inference framework integration API approved (vLLM/TensorRT-LLM teams)
- [ ] Error handling tested: malformed commands, stream query timeouts, GPU faults
- [ ] Performance baseline established (kernel submission latency < 100µs)
- [ ] Unit test suite covers all stream operations and GPU event handling

## Design Principles Alignment
- **Async-First Design:** Frameworks submit work via CUDA streams; GPU Manager handles async scheduling
- **Framework Integration:** Minimal framework changes; kernel provides standard CUDA/ROCm submission API
- **Safety:** Kernel encapsulates CUDA/ROCm API calls; no direct GPU access from userspace

## Addendum v2.5.1 — Correction 1: GPU Driver Strategy
**Status:** Phase A (v1.0) using CUDA stream/kernel launch APIs instead of raw command queues
**Rationale:** LithOS validates API-level kernel launch interception approach for command scheduling
**Implementation:** Use cuLaunchKernel / hipLaunchKernel with kernel atomization via API-level kernel launch splitting (not PTX modification)
