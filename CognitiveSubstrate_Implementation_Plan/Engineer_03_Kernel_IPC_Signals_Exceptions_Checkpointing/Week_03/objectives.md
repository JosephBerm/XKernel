# Engineer 3 — Kernel: IPC, Signals, Exceptions & Checkpointing — Week 03

## Phase: PHASE 0 — Formalization & Synchronous IPC

## Weekly Objective

Implement synchronous request-response IPC using Cap'n Proto serialization with zero-copy optimization for co-located agents via shared physical page mappings. Establish baseline performance for single-machine IPC.

## Document References
- **Primary:** Section 3.2.4 (Request-Response IPC)
- **Supporting:** Section 7 (IPC Latency), Section 6.2 (Exit Criteria)

## Deliverables
- [ ] Cap'n Proto schema definitions for request/response message format
- [ ] RequestResponseChannel implementation with request buffering and response matching
- [ ] Zero-copy mechanisms: shared physical page mapping for co-located agents
- [ ] Kernel syscalls: chan_send (request), chan_recv (response)
- [ ] Request ID tracking and response correlation logic
- [ ] Timeout handling for request-response cycles
- [ ] Unit tests for basic request-response flow
- [ ] Benchmark: measure sub-microsecond latency on co-located channels

## Technical Specifications

### Request-Response IPC Flow
1. Sender: Allocate request buffer, copy/serialize request payload
2. Sender: Call chan_send syscall with request ID and destination CT
3. Kernel: Map sender's request buffer into receiver's address space (read-only for kernel-managed regions, copy on user payloads)
4. Receiver: Call chan_recv syscall, receives request ID and mapped buffer pointer
5. Receiver: Process request, write response
6. Receiver: Call chan_send syscall with response ID (matching request ID)
7. Kernel: Map response buffer back to sender
8. Sender: Call chan_recv syscall, receives response with matching ID
9. Timeout: If no response within deadline, return TIMEOUT error

### Zero-Copy Strategy
- **Co-located agents:** Kernel forks page tables to share physical pages in read-only mode for kernel metadata, writable for application payloads
- **Large payloads (>4KB):** Use descriptor-based access: request/response contains pointer to shared buffer, not full copy
- **Metadata:** 64-byte fixed header (request ID, timestamp, flags) lives in kernel-managed region

### Cap'n Proto Schema Structure
```
struct Request {
  id: UInt64,
  requesterId: UInt64,
  methodId: UInt32,
  deadline: UInt64,
  payload: Data,
}

struct Response {
  id: UInt64,
  responderId: UInt64,
  status: UInt32,
  payload: Data,
}
```

### RequestResponseChannel Implementation
```
pub struct RequestResponseChannel {
    pub id: ChannelId,
    pub requestor: ContextThreadRef,
    pub requestee: ContextThreadRef,
    pub pending_requests: HashMap<RequestId, PendingRequest>,
    pub response_buffers: Vec<ResponseBuffer>,
    pub deadline_ms: u64,
}

struct PendingRequest {
    request_id: RequestId,
    sender_id: ContextThreadId,
    timestamp: Timestamp,
    mapped_buffer: *const u8,
    size: usize,
}
```

## Dependencies
- **Blocked by:** Week 1-2 (Formalization), Week 3 starts concurrently
- **Blocking:** Week 4-5 Signal Dispatch, Week 7-8 Pub/Sub IPC

## Acceptance Criteria
1. Request-response round-trip latency < 10 microseconds for co-located agents on reference hardware
2. Zero-copy mappings verified (no memcpy for request/response payloads on co-located channels)
3. Request ID correlation prevents response mismatches
4. Timeout handling prevents indefinite blocking
5. Cap'n Proto serialization adds <1 microsecond overhead
6. Unit tests cover: basic request-response, timeout, multiple pending requests, large payloads
7. Benchmark results documented and compared to baseline

## Design Principles Alignment
- **Performance:** Zero-copy for co-located agents minimizes latency
- **Safety:** Page table sharing prevents out-of-bounds access to unmapped regions
- **Transparency:** Serialization is automatic; application code sees typed requests/responses
- **Capability-Based:** Only requestor/requestee can access the channel
