# Engineer 7 — Runtime: Framework Adapters — Week 7
## Phase: Phase 1 (Integration: Kernel Services & Translation Layer)
## Weekly Objective
Support kernel and services streams integration. Review and document IPC interface, memory interface contracts for adapter usage. Begin integration testing with kernel services. Prepare architecture for adapter-kernel communication.

## Document References
- **Primary:** Section 3.4 — L2 Agent Runtime, Section 3.2 — IPC & Memory Interfaces
- **Supporting:** Section 3.4.1 — Framework Adapters, Section 1.2 — P6: Framework-Agnostic Agent Runtime

## Deliverables
- [ ] IPC interface review: document all message types, serialization, handshake protocol
- [ ] Memory interface documentation: how adapters read/write L2 episodic and L3 semantic memory
- [ ] Adapter-kernel communication design: request-response patterns, error handling, timeouts
- [ ] Integration test harness: spawn kernel services, adapter IPC connectivity test
- [ ] Document kernel service contracts that adapters depend on: task service, memory service, capability service, channel service
- [ ] Compatibility layer for adapter code: bridge between adapter SDK and kernel service calls
- [ ] Phase 1 architecture diagram: adapter layer in relation to kernel services

## Technical Specifications
- Review IPC message format: protobuf/JSON schemas for all message types
- Document memory interface syscalls: mem_write(key, value, lifecycle), mem_read(key), mem_list(prefix)
- Design adapter request-response protocol: request ID tracking, timeout handling (5s default), retry logic
- Kernel service dependencies: TaskService, MemoryService, CapabilityService, ChannelService, TelemetryService
- Integration test scenarios: adapter startup, IPC handshake, single task spawn, memory write/read
- Compatibility layer mapping: adapter API → kernel service API calls
- Error codes: KERNEL_TIMEOUT, IPC_DISCONNECTED, MEMORY_NOT_FOUND, CAPABILITY_DENIED, INVALID_MESSAGE

## Dependencies
- **Blocked by:** Week 6
- **Blocking:** Week 8, Week 9, Week 10, Week 11

## Acceptance Criteria
- IPC and memory interfaces fully documented for adapter usage
- Integration test harness successfully connects to kernel services
- Adapter-kernel communication protocol designed and documented
- Compatibility layer reduces adapter code complexity by 30%+
- Phase 1 architecture clearly shows adapter positioning

## Design Principles Alignment
- **Kernel Integration:** Clean integration points with all kernel services
- **IPC Efficiency:** Minimize message round-trips and payload sizes
- **Error Resilience:** Graceful handling of kernel service latency and failures
