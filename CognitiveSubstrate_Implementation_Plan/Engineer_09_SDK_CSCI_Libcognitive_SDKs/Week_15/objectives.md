# Engineer 9 — SDK: CSCI, libcognitive & SDKs — Week 15

## Phase: Phase 2

## Weekly Objective

Incorporate feedback from adapter team (LangChain, Semantic Kernel, CrewAI bridges) on CSCI v0.1. Refine syscall signatures, error handling, and performance characteristics for v0.5.

## Document References

- **Primary:** Section 3.5.1 — CSCI: Cognitive System Call Interface; Section 6.3 — Phase 2
- **Supporting:** Adapter team feedback; performance profiling; cross-framework compatibility analysis

## Deliverables

- [ ] Collect requirements from framework adapter team
- [ ] Identify CSCI gaps or misalignments with LangChain, Semantic Kernel, CrewAI patterns
- [ ] Propose refinements to syscall signatures or error handling
- [ ] Profile CSCI FFI overhead vs framework overhead
- [ ] Document refinement rationale and tradeoffs
- [ ] Draft CSCI v0.5 specification with refinements
- [ ] **CSCI v0.5 Internal milestone: full signatures in Rust, error code enumeration**
- [ ] **Capability requirements per syscall (which caps required for each call)**
- [ ] **Parameter validation semantics and bounds checking**
- [ ] **Intermediate spec before v1.0 freeze (Week 22)**

## Technical Specifications

### CSCI v0.5 Internal Specification

CSCI v0.5 is a refinement of v0.1 with full Rust signatures, error enumerations, and capability requirements per syscall.

**Example: Refined chan_open Syscall**
```rust
// csci_v0.5_syscalls.rs
pub mod ipc {
    /// Create a new IPC channel endpoint.
    ///
    /// # Arguments
    /// * `flags` - Channel behavior flags (NONBLOCKING, BUFFERED, etc.)
    ///
    /// # Capabilities Required
    /// * `cap_create_channel` - Required to create new channels
    ///
    /// # Returns
    /// - `Ok(ChanID)` - Channel successfully created
    /// - `Err(Error::NoMemory)` - Kernel memory exhausted
    /// - `Err(Error::InvalidFlags)` - Unsupported flag combination
    /// - `Err(Error::PermissionDenied)` - Agent lacks cap_create_channel
    ///
    /// # Examples
    /// ```ignore
    /// let chan_id = csci::ipc::chan_open(CHAN_NONBLOCKING)?;
    /// ```
    pub fn chan_open(flags: u32) -> Result<ChanID, CsciError>;

    /// Send message on IPC channel.
    ///
    /// # Arguments
    /// * `chan_id` - Target channel ID
    /// * `message` - Message to send (max 64KB)
    /// * `timeout_ms` - Maximum wait time, or None for blocking
    ///
    /// # Capabilities Required
    /// * `cap_send_on_channel` - Bound to specific channel via CapabilityConstraints
    ///
    /// # Returns
    /// - `Ok(())` - Message queued for delivery
    /// - `Err(Error::InvalidChannel)` - Channel ID not found
    /// - `Err(Error::ChannelClosed)` - Receiver closed channel
    /// - `Err(Error::Timeout)` - Backpressure timeout exceeded
    /// - `Err(Error::MessageTooLarge)` - Message exceeds 64KB
    /// - `Err(Error::PermissionDenied)` - cap_send_on_channel not held
    pub fn chan_send(chan_id: ChanID, message: &[u8], timeout_ms: Option<u64>)
        -> Result<(), CsciError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CsciError {
    /// Kernel out of memory
    NoMemory,

    /// Invalid channel ID or argument
    InvalidChannel,
    InvalidCapID,
    InvalidAgent,
    InvalidFlags,
    InvalidAttenuation,
    InvalidTool,

    /// Channel closed by peer
    ChannelClosed,

    /// Timeout waiting for operation
    Timeout,

    /// Message too large for channel
    MessageTooLarge,

    /// No message available (non-blocking mode)
    NoMessage,

    /// Permission denied (missing capability)
    PermissionDenied,

    /// Policy decision denies operation
    PolicyDenied,

    /// Resource limit exceeded
    ResourceExhausted,

    /// Invalid constraints or attenuation
    InvalidAttenuation,

    /// Tool execution failed
    ExecutionError,

    /// Sandbox violation
    SandboxError,

    /// Argument error (invalid tool args, etc.)
    ArgumentError,

    /// Unknown error
    Unknown,
}

/// Capability requirements per syscall
pub mod capability_requirements {
    use super::*;

    /// Each syscall maps to required capabilities
    pub const SYSCALL_CAPS: &[(&str, &[&str])] = &[
        ("chan_open", &["cap_create_channel"]),
        ("chan_send", &["cap_send_on_channel"]),
        ("chan_recv", &["cap_recv_on_channel"]),
        ("cap_grant", &["cap_grant_capability"]),
        ("cap_delegate", &["cap_delegate_capability"]),
        ("cap_revoke", &["cap_revoke_capability"]),
        ("tool_bind", &["cap_bind_tool"]),
        ("tool_invoke", &["cap_invoke_tool"]),
    ];

    /// Validate that agent holds all required capabilities
    pub fn validate_agent_caps(agent: &Agent, syscall_name: &str) -> Result<(), CsciError> {
        let required = SYSCALL_CAPS.iter()
            .find(|(name, _)| *name == syscall_name)
            .map(|(_, caps)| *caps)
            .unwrap_or_default();

        for cap in required {
            if !agent.has_capability(cap) {
                return Err(CsciError::PermissionDenied);
            }
        }
        Ok(())
    }
}
```

**Parameter Validation Semantics**
```rust
// Bounds checking and validation rules
pub mod validation {
    use super::*;

    /// Validate message size (max 64KB per channel message)
    pub fn validate_message(data: &[u8]) -> Result<(), CsciError> {
        if data.len() > 65536 {
            Err(CsciError::MessageTooLarge)
        } else {
            Ok(())
        }
    }

    /// Validate channel flags
    pub fn validate_chan_flags(flags: u32) -> Result<(), CsciError> {
        const VALID_FLAGS: u32 = CHAN_NONBLOCKING | CHAN_BUFFERED | CHAN_PRIORITY;
        if (flags & !VALID_FLAGS) != 0 {
            Err(CsciError::InvalidFlags)
        } else {
            Ok(())
        }
    }

    /// Validate capability constraints
    pub fn validate_constraints(constraints: &CapabilityConstraints) -> Result<(), CsciError> {
        // Check: expiry_time >= now
        // Check: rate_limit >= 1
        // Check: no negative bounds
        // etc.
        Ok(())
    }
}
```

- v0.5 preserves all 22 syscalls; refines parameters, error codes, or preconditions
- Refinements address: LangChain MemoryManager integration, SK function binding, CrewAI crew patterns
- Performance targets: FFI overhead < 5% of task execution time
- v0.5 includes additional documentation, examples, and edge case clarifications
- **Intermediate spec before v1.0 freeze (Week 22)**

## Dependencies

- **Blocked by:** Phase 1
- **Blocking:** Week 16-17 (CSCI v1.0 finalization)

## Acceptance Criteria

CSCI v0.5 specification finalized; ready for v1.0 preparation

## Design Principles Alignment

- **Cognitive-Native:** All syscall interfaces designed for tight integration with CT execution engine
- **Semantic Versioning:** CSCI follows major.minor.patch; SDKs track CSCI compatibility
- **Developer Experience:** TypeScript and C# SDKs provide strongly-typed, async-first APIs with IntelliSense
- **Interoperability:** CSCI syscalls are the unified contract; SDKs bridge language ecosystems
- **Testing:** Unit tests, integration tests with kernel team, and FFI layer validation
- **Documentation:** API docs, examples, tutorials for all public surface

