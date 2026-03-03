# Engineer 2 — Kernel: Capability Engine & Security — Week 10

## Phase: PHASE 1 - Core Services + Multi-Agent

## Weekly Objective
Implement distributed IPC capability verification at network ingress/egress. Enable cryptographic verification of capability grants crossing kernel boundaries. Ensure no capability leakage or tampering in transit.

## Document References
- **Primary:** Section 3.2.4 (Distributed IPC - Cryptographic Verification at Network Boundaries), Section 3.2.3 (Cryptographic Signatures at Trust Boundaries)
- **Supporting:** Section 2.4 (Capability Formalization), Section 2.1 (Architecture Overview)

## Deliverables
- [ ] Cryptographic signature scheme for distributed capabilities (Ed25519)
- [ ] Network packet capability encoding format (CapID + delegation_chain + constraints)
- [ ] Ingress verification handler (decrypt and verify capabilities on receipt)
- [ ] Egress signature handler (sign capabilities before network transmission)
- [ ] Trust anchor establishment (public key exchange and verification)
- [ ] Replay attack prevention (sequence numbers and nonce validation)
- [ ] Revocation status checking at network boundary
- [ ] Comprehensive test suite (150+ tests for all IPC scenarios)
- [ ] Performance profiling (signature latency, throughput impact)
- [ ] Documentation of distributed capability protocol

## Technical Specifications
- **Cryptographic Signature Scheme:**
  - Algorithm: Ed25519 (deterministic, 64-byte signatures)
  - Signing key: kernel instance private key (generated during boot, stored securely)
  - Verification key: remote kernel's public key (exchanged during trust establishment)
  - Signature message: hash(capid || delegation_chain || constraints || timestamp || nonce)
  - Hash function: BLAKE3 (fast, cryptographically secure)
  - Signature generation latency target: <1000ns (fast path with hardware acceleration if available)
- **Network Packet Capability Encoding:**
  - IPC packet format: [capability_header | capid | delegation_chain | constraints | signature]
  - capid: 256-bit unique identifier
  - delegation_chain: variable-length list of (delegating_agent, delegated_to_agent, constraint, timestamp)
  - constraints: packed representation of (operations, time_bounds, rate_limits, data_volume_limits)
  - signature: 64-byte Ed25519 signature over hash of all fields
  - Total size: 32 (capid) + 8*chain_length (delegation) + 32 (constraints) + 64 (signature) = ~200 bytes typical
- **Ingress Verification Handler:**
  - Triggered on network packet reception with capability
  - Steps:
    1. Extract capid, delegation_chain, constraints, signature from packet
    2. Compute message_hash = hash(capid || delegation_chain || constraints)
    3. Lookup sender kernel's public key from trust registry
    4. Verify signature: ed25519_verify(sender_pubkey, message_hash, signature)
    5. Check revocation status: is capid in global revocation list?
    6. Validate constraints: expiry > now, rate limit not exceeded, etc.
    7. Update local kernel's delegation_chain entry with received delegation
  - On success: create local capability table entry, allow invocation
  - On failure: reject IPC packet, log security event, dispatch alert
  - Latency target: <5000ns (including key lookup and revocation check)
- **Egress Signature Handler:**
  - Triggered when capability is delegated via network IPC
  - Steps:
    1. Lookup local kernel's signing key
    2. Compute message_hash = hash(capid || delegation_chain || constraints)
    3. Sign: signature = ed25519_sign(signing_key, message_hash)
    4. Encode network packet: capid || delegation_chain || constraints || signature
    5. Send packet to remote kernel
  - Latency target: <1000ns (signing only, network latency separate)
- **Trust Anchor Establishment:**
  - During kernel bootstrapping: generate Ed25519 keypair
  - Publish public key to trusted registry (e.g., kernel directory service)
  - On receiving IPC: lookup sender's public key from registry
  - Key exchange protocol: TLS 1.3 handshake (outside capability scope, handled by network layer)
  - Key revocation: removed from registry, future IPC rejected
- **Replay Attack Prevention:**
  - Each IPC packet includes: global_sequence_number (monotonic counter)
  - Receiver maintains: last_seen_sequence_number for each sender
  - Reject packet if sequence_number ≤ last_seen_sequence_number
  - Additional defense: nonce in signature prevents replay of old packets
  - Nonce format: (timestamp_ns || random_u64)
  - Receiver checks: |local_time - packet_timestamp| < 5 seconds
- **Revocation Status Checking:**
  - Global revocation list: set of revoked CapIDs (maintained by central revocation service)
  - On ingress: query revocation list for incoming capid
  - If revoked: reject IPC, log security event
  - Revocation propagation: asynchronous updates from revocation service (batched)
  - Local cache: revoked CapIDs cached for <5 second TTL (performance vs timeliness tradeoff)

## Dependencies
- **Blocked by:** Week 6-7 (capability table, delegation chains), Stream 5 (network IPC implementation)
- **Blocking:** Week 11-12 (distributed IPC integration), Week 13-14 (multi-agent demo)

## Acceptance Criteria
- Ed25519 signatures correctly verify capabilities in transit
- Ingress verification correctly rejects tampered or revoked capabilities
- Egress signatures correctly sign outgoing capabilities
- Trust anchor establishment enables cross-kernel capability delegation
- Replay attack prevention works for all sequence scenarios
- Revocation status is checked on every ingress
- All 150+ tests pass (single-agent, multi-kernel, attack scenarios)
- Signature latency <1000ns p50, <2000ns p99
- Ingress verification latency <5000ns p50, <10000ns p99
- Code review completed by security and cryptography teams

## Design Principles Alignment
- **P1 (Security-First):** Cryptographic verification prevents capability tampering in transit
- **P2 (Transparency):** Signatures are cryptographically verifiable by all parties
- **P5 (Formal Verification):** Ed25519 signature scheme is formally analyzed and proven secure
- **P6 (Compliance & Audit):** IPC signatures provide non-repudiation for regulatory requirements
- **P8 (Robustness):** Replay attack prevention ensures idempotent capability delegation
