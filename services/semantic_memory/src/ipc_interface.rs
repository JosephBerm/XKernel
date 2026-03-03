// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! IPC interface for Memory Manager L1 service.
//!
//! This module provides the syscall routing and request/response types for inter-process
//! communication with the Memory Manager, enabling Cognitive Threads (CTs) to request
//! memory operations (allocate, read, write, mount, query, evict) through a capability-based
//! interface.
//!
//! # Request/Response Model
//!
//! The Memory Manager receives `MemoryRequest` enums from CTs and responds with
//! `MemoryResponse` types. Each request is validated for capability access before
//! dispatching to the appropriate tier handler.
//!
//! See Engineering Plan § 4.1.0 (IPC Interface) and § 4.1.3 (Access Control).

use alloc::string::String;
use alloc::vec::Vec;
use crate::error::{MemoryError, Result};
use crate::capability_control::MemoryCapability;
use crate::ids::MemoryRegionID;

/// Cost tracking for memory operations (tokens, bytes, latency).
///
/// Supports observability and resource accounting for IPC requests.
/// See Engineering Plan § 4.1.0: Request Cost Tracking.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequestCost {
    /// Tokens consumed (capability-based rate limiting)
    pub tokens_consumed: u64,
    /// Memory bytes allocated or transferred
    pub memory_bytes: u64,
    /// Operation latency in nanoseconds (estimated)
    pub latency_ns: u64,
}

impl RequestCost {
    /// Creates a new request cost tracker.
    pub fn new(tokens: u64, bytes: u64, latency_ns: u64) -> Self {
        RequestCost {
            tokens_consumed: tokens,
            memory_bytes: bytes,
            latency_ns,
        }
    }

    /// Creates a zero-cost operation (for tests or free operations).
    pub fn zero() -> Self {
        RequestCost {
            tokens_consumed: 0,
            memory_bytes: 0,
            latency_ns: 0,
        }
    }
}

/// Enumeration of memory tiers for allocation requests.
///
/// Specifies which tier to allocate memory in.
/// See Engineering Plan § 4.1.0: Tier Specification.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum MemoryTierSpec {
    /// L1 (Working Memory) - GPU-local HBM
    L1,
    /// L2 (Episodic Memory) - Host DRAM
    L2,
    /// L3 (Long-Term Memory) - NVMe-backed persistent
    L3,
}

impl MemoryTierSpec {
    /// Returns the tier name as a string.
    pub fn name(&self) -> &'static str {
        match self {
            MemoryTierSpec::L1 => "L1",
            MemoryTierSpec::L2 => "L2",
            MemoryTierSpec::L3 => "L3",
        }
    }
}

/// Memory request from a Cognitive Thread to the Memory Manager.
///
/// Represents all possible syscall operations: allocate, read, write, mount, query, evict.
/// See Engineering Plan § 4.1.1: Memory Operations.
#[derive(Clone, Debug)]
pub enum MemoryRequest {
    /// Allocate memory in the specified tier.
    ///
    /// Fields:
    /// - `tier`: Target tier (L1, L2, L3)
    /// - `size`: Bytes to allocate
    /// - `capability`: Capability token authorizing the allocation
    Allocate {
        tier: MemoryTierSpec,
        size: u64,
        capability: String,
    },

    /// Read data from a memory region.
    ///
    /// Fields:
    /// - `region_id`: ID of the region to read from
    /// - `offset`: Byte offset within the region
    /// - `len`: Number of bytes to read
    /// - `capability`: Capability token authorizing the read
    Read {
        region_id: String,
        offset: u64,
        len: u64,
        capability: String,
    },

    /// Write data to a memory region.
    ///
    /// Fields:
    /// - `region_id`: ID of the region to write to
    /// - `offset`: Byte offset within the region
    /// - `data`: Bytes to write
    /// - `capability`: Capability token authorizing the write
    Write {
        region_id: String,
        offset: u64,
        data: Vec<u8>,
        capability: String,
    },

    /// Mount an external source (e.g., knowledge base, persistent store).
    ///
    /// Fields:
    /// - `source`: Source identifier (path, URL, etc.)
    /// - `mount_point`: Where to mount in the virtual address space
    /// - `capability`: Capability token authorizing the mount
    Mount {
        source: String,
        mount_point: String,
        capability: String,
    },

    /// Query statistics about a memory region.
    ///
    /// Fields:
    /// - `region_id`: ID of the region to query
    /// - `capability`: Capability token authorizing the query
    Query {
        region_id: String,
        capability: String,
    },

    /// Evict data from a memory region (triggers tier migration).
    ///
    /// Fields:
    /// - `region_id`: ID of the region to evict from
    /// - `capability`: Capability token authorizing the eviction
    Evict {
        region_id: String,
        capability: String,
    },
}

/// Response from the Memory Manager to a Cognitive Thread.
///
/// Indicates success with relevant data or failure with error details.
/// See Engineering Plan § 4.1.0: Response Types.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MemoryResponse {
    /// Allocation succeeded, returning the region ID and mapped virtual address.
    Allocated {
        region_id: String,
        mapped_addr: u64,
    },

    /// Read succeeded, returning the requested data.
    ReadData {
        data: Vec<u8>,
    },

    /// Write succeeded (acknowledged).
    WriteAck,

    /// Mount succeeded, returning the mount ID.
    Mounted {
        mount_id: String,
    },

    /// Query succeeded, returning region statistics.
    QueryResult {
        stats: RegionStats,
    },

    /// Eviction succeeded (tier migration complete).
    Evicted,

    /// Operation failed with an error.
    Error {
        error: String,
    },
}

/// Statistics about a memory region (returned by Query).
///
/// Provides visibility into region usage and performance.
/// See Engineering Plan § 4.1.0: Region Monitoring.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegionStats {
    /// Total bytes allocated in region
    pub allocated_bytes: u64,
    /// Total bytes free in region
    pub free_bytes: u64,
    /// Number of active entries/allocations
    pub entry_count: u64,
    /// Current memory pressure level (0-100)
    pub pressure_level: u8,
}

impl RegionStats {
    /// Creates new region statistics.
    pub fn new(allocated: u64, free: u64, entries: u64, pressure: u8) -> Self {
        RegionStats {
            allocated_bytes: allocated,
            free_bytes: free,
            entry_count: entries,
            pressure_level: pressure,
        }
    }

    /// Returns utilization as a percentage (0-100).
    pub fn utilization_percent(&self) -> u64 {
        let total = self.allocated_bytes + self.free_bytes;
        if total == 0 {
            0
        } else {
            (self.allocated_bytes * 100) / total
        }
    }
}

/// Dispatcher for routing memory requests to appropriate tier handlers.
///
/// Acts as the main entry point for IPC syscalls, dispatching MemoryRequest
/// to tier-specific handlers (L1, L2, L3) after validation.
///
/// See Engineering Plan § 4.1.1: Request Routing.
pub struct RequestRouter {
    /// Maximum tokens available per request period
    max_tokens_per_period: u64,
    /// Current token budget
    tokens_available: u64,
}

impl RequestRouter {
    /// Creates a new request router with specified token budget.
    ///
    /// # Arguments
    ///
    /// * `max_tokens` - Maximum tokens available per request period
    pub fn new(max_tokens: u64) -> Self {
        RequestRouter {
            max_tokens_per_period: max_tokens,
            tokens_available: max_tokens,
        }
    }

    /// Routes a memory request to the appropriate tier handler.
    ///
    /// Validates capability and dispatches to tier-specific logic.
    /// Returns both the response and the cost incurred.
    ///
    /// # Arguments
    ///
    /// * `request` - The memory request to route
    /// * `validator` - Capability validator to check authorization
    ///
    /// # Returns
    ///
    /// `Result<(MemoryResponse, RequestCost)>` with response and cost tracking
    ///
    /// See Engineering Plan § 4.1.1: Request Routing & Dispatch.
    pub fn route_request(
        &mut self,
        request: MemoryRequest,
        validator: &dyn CapabilityValidator,
    ) -> Result<(MemoryResponse, RequestCost)> {
        match &request {
            MemoryRequest::Allocate {
                tier,
                size,
                capability,
            } => {
                validator.validate_capability(capability, "allocate")?;
                self.handle_allocate(tier.clone(), *size)
            }
            MemoryRequest::Read {
                region_id,
                offset,
                len,
                capability,
            } => {
                validator.validate_capability(capability, "read")?;
                self.handle_read(region_id.clone(), *offset, *len)
            }
            MemoryRequest::Write {
                region_id,
                offset,
                data,
                capability,
            } => {
                validator.validate_capability(capability, "write")?;
                self.handle_write(region_id.clone(), *offset, data.clone())
            }
            MemoryRequest::Mount {
                source,
                mount_point,
                capability,
            } => {
                validator.validate_capability(capability, "mount")?;
                self.handle_mount(source.clone(), mount_point.clone())
            }
            MemoryRequest::Query {
                region_id,
                capability,
            } => {
                validator.validate_capability(capability, "query")?;
                self.handle_query(region_id.clone())
            }
            MemoryRequest::Evict {
                region_id,
                capability,
            } => {
                validator.validate_capability(capability, "evict")?;
                self.handle_evict(region_id.clone())
            }
        }
    }

    fn handle_allocate(&mut self, _tier: MemoryTierSpec, size: u64) -> Result<(MemoryResponse, RequestCost)> {
        let cost = RequestCost::new(10, size, 100); // Placeholder costs
        let response = MemoryResponse::Allocated {
            region_id: "alloc-001".to_string(),
            mapped_addr: 0x1000,
        };
        Ok((response, cost))
    }

    fn handle_read(&mut self, _region_id: String, _offset: u64, len: u64) -> Result<(MemoryResponse, RequestCost)> {
        let cost = RequestCost::new(5, len, 50);
        let response = MemoryResponse::ReadData {
            data: Vec::new(),
        };
        Ok((response, cost))
    }

    fn handle_write(&mut self, _region_id: String, _offset: u64, data: Vec<u8>) -> Result<(MemoryResponse, RequestCost)> {
        let cost = RequestCost::new(5, data.len() as u64, 50);
        let response = MemoryResponse::WriteAck;
        Ok((response, cost))
    }

    fn handle_mount(&mut self, _source: String, _mount_point: String) -> Result<(MemoryResponse, RequestCost)> {
        let cost = RequestCost::new(20, 0, 200);
        let response = MemoryResponse::Mounted {
            mount_id: "mount-001".to_string(),
        };
        Ok((response, cost))
    }

    fn handle_query(&mut self, _region_id: String) -> Result<(MemoryResponse, RequestCost)> {
        let cost = RequestCost::new(1, 0, 10);
        let response = MemoryResponse::QueryResult {
            stats: RegionStats::new(4096, 4096, 10, 50),
        };
        Ok((response, cost))
    }

    fn handle_evict(&mut self, _region_id: String) -> Result<(MemoryResponse, RequestCost)> {
        let cost = RequestCost::new(15, 0, 150);
        let response = MemoryResponse::Evicted;
        Ok((response, cost))
    }
}

/// Validates memory operation capabilities before processing requests.
///
/// This trait defines the capability validation interface used by RequestRouter.
/// Implementations check that a given capability token authorizes a specific operation.
///
/// See Engineering Plan § 3.1: Capability-Based Security & § 4.1.3: Access Control.
pub trait CapabilityValidator {
    /// Validates a capability for a specific operation.
    ///
    /// # Arguments
    ///
    /// * `capability_token` - The capability being validated
    /// * `operation` - The operation being authorized (e.g., "allocate", "read", "write")
    ///
    /// # Returns
    ///
    /// `Result<()>` - Ok if valid, CapabilityDenied error if not
    fn validate_capability(&self, capability_token: &str, operation: &str) -> Result<()>;
}

/// Default implementation of CapabilityValidator for testing.
///
/// In production, a more sophisticated validator would verify cryptographic
/// proofs, check revocation lists, and enforce quota limits.
pub struct DefaultCapabilityValidator {
    /// Set of valid capability tokens (simplified for testing)
    valid_tokens: Vec<String>,
}

impl DefaultCapabilityValidator {
    /// Creates a new default validator with specified valid tokens.
    pub fn new(valid_tokens: Vec<String>) -> Self {
        DefaultCapabilityValidator { valid_tokens }
    }

    /// Creates a permissive validator that accepts all tokens (testing only).
    pub fn permissive() -> Self {
        DefaultCapabilityValidator {
            valid_tokens: vec![],
        }
    }
}

impl CapabilityValidator for DefaultCapabilityValidator {
    fn validate_capability(&self, capability_token: &str, _operation: &str) -> Result<()> {
        // In permissive mode (empty token list), accept all
        if self.valid_tokens.is_empty() {
            return Ok(());
        }

        // Check if token is in valid set
        if self.valid_tokens.iter().any(|t| t == capability_token) {
            Ok(())
        } else {
            Err(MemoryError::CapabilityDenied {
                operation: _operation.to_string(),
                resource: "memory_manager".to_string(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::string::ToString;
use alloc::vec;

    #[test]
    fn test_request_cost_creation() {
        let cost = RequestCost::new(100, 1024, 500);
        assert_eq!(cost.tokens_consumed, 100);
        assert_eq!(cost.memory_bytes, 1024);
        assert_eq!(cost.latency_ns, 500);
    }

    #[test]
    fn test_request_cost_zero() {
        let cost = RequestCost::zero();
        assert_eq!(cost.tokens_consumed, 0);
        assert_eq!(cost.memory_bytes, 0);
        assert_eq!(cost.latency_ns, 0);
    }

    #[test]
    fn test_memory_tier_spec_names() {
        assert_eq!(MemoryTierSpec::L1.name(), "L1");
        assert_eq!(MemoryTierSpec::L2.name(), "L2");
        assert_eq!(MemoryTierSpec::L3.name(), "L3");
    }

    #[test]
    fn test_region_stats_creation() {
        let stats = RegionStats::new(2048, 2048, 5, 50);
        assert_eq!(stats.allocated_bytes, 2048);
        assert_eq!(stats.free_bytes, 2048);
        assert_eq!(stats.entry_count, 5);
        assert_eq!(stats.pressure_level, 50);
    }

    #[test]
    fn test_region_stats_utilization() {
        let stats = RegionStats::new(5000, 5000, 10, 50);
        assert_eq!(stats.utilization_percent(), 50);

        let stats_full = RegionStats::new(10000, 0, 10, 100);
        assert_eq!(stats_full.utilization_percent(), 100);

        let stats_empty = RegionStats::new(0, 10000, 0, 0);
        assert_eq!(stats_empty.utilization_percent(), 0);
    }

    #[test]
    fn test_memory_response_allocated() {
        let response = MemoryResponse::Allocated {
            region_id: "test-001".to_string(),
            mapped_addr: 0x2000,
        };
        match response {
            MemoryResponse::Allocated {
                region_id,
                mapped_addr,
            } => {
                assert_eq!(region_id, "test-001");
                assert_eq!(mapped_addr, 0x2000);
            }
            _ => panic!("Expected Allocated response"),
        }
    }

    #[test]
    fn test_memory_response_read_data() {
        let data = vec![1, 2, 3, 4, 5];
        let response = MemoryResponse::ReadData { data: data.clone() };
        match response {
            MemoryResponse::ReadData { data: read_data } => {
                assert_eq!(read_data, data);
            }
            _ => panic!("Expected ReadData response"),
        }
    }

    #[test]
    fn test_memory_response_write_ack() {
        let response = MemoryResponse::WriteAck;
        match response {
            MemoryResponse::WriteAck => {
                // Success
            }
            _ => panic!("Expected WriteAck response"),
        }
    }

    #[test]
    fn test_request_router_creation() {
        let router = RequestRouter::new(10000);
        assert_eq!(router.max_tokens_per_period, 10000);
        assert_eq!(router.tokens_available, 10000);
    }

    #[test]
    fn test_default_capability_validator_permissive() {
        let validator = DefaultCapabilityValidator::permissive();
        assert!(validator.validate_capability("any-token", "read").is_ok());
        assert!(validator.validate_capability("any-token", "write").is_ok());
    }

    #[test]
    fn test_default_capability_validator_strict() {
        let validator = DefaultCapabilityValidator::new(vec!["cap-001".to_string()]);
        assert!(validator.validate_capability("cap-001", "read").is_ok());
        assert!(validator.validate_capability("invalid", "read").is_err());
    }

    #[test]
    fn test_request_router_allocate() {
        let mut router = RequestRouter::new(10000);
        let validator = DefaultCapabilityValidator::permissive();
        let request = MemoryRequest::Allocate {
            tier: MemoryTierSpec::L1,
            size: 1024,
            capability: "cap-001".to_string(),
        };

        let result = router.route_request(request, &validator);
        assert!(result.is_ok());
        let (response, cost) = result.unwrap();
        match response {
            MemoryResponse::Allocated { .. } => {
                assert_eq!(cost.memory_bytes, 1024);
            }
            _ => panic!("Expected Allocated response"),
        }
    }

    #[test]
    fn test_request_router_read() {
        let mut router = RequestRouter::new(10000);
        let validator = DefaultCapabilityValidator::permissive();
        let request = MemoryRequest::Read {
            region_id: "region-001".to_string(),
            offset: 100,
            len: 256,
            capability: "cap-001".to_string(),
        };

        let result = router.route_request(request, &validator);
        assert!(result.is_ok());
        let (response, cost) = result.unwrap();
        match response {
            MemoryResponse::ReadData { .. } => {
                assert_eq!(cost.memory_bytes, 256);
            }
            _ => panic!("Expected ReadData response"),
        }
    }

    #[test]
    fn test_request_router_write() {
        let mut router = RequestRouter::new(10000);
        let validator = DefaultCapabilityValidator::permissive();
        let data = vec![1, 2, 3, 4];
        let request = MemoryRequest::Write {
            region_id: "region-001".to_string(),
            offset: 100,
            data: data.clone(),
            capability: "cap-001".to_string(),
        };

        let result = router.route_request(request, &validator);
        assert!(result.is_ok());
        let (_response, cost) = result.unwrap();
        assert_eq!(cost.memory_bytes, 4);
    }

    #[test]
    fn test_request_router_capability_denied() {
        let mut router = RequestRouter::new(10000);
        let validator = DefaultCapabilityValidator::new(vec!["valid-cap".to_string()]);
        let request = MemoryRequest::Allocate {
            tier: MemoryTierSpec::L2,
            size: 512,
            capability: "invalid-cap".to_string(),
        };

        let result = router.route_request(request, &validator);
        assert!(result.is_err());
    }

    #[test]
    fn test_request_router_query() {
        let mut router = RequestRouter::new(10000);
        let validator = DefaultCapabilityValidator::permissive();
        let request = MemoryRequest::Query {
            region_id: "region-001".to_string(),
            capability: "cap-001".to_string(),
        };

        let result = router.route_request(request, &validator);
        assert!(result.is_ok());
        let (response, _cost) = result.unwrap();
        match response {
            MemoryResponse::QueryResult { stats } => {
                assert_eq!(stats.allocated_bytes, 4096);
            }
            _ => panic!("Expected QueryResult response"),
        }
    }

    #[test]
    fn test_request_router_mount() {
        let mut router = RequestRouter::new(10000);
        let validator = DefaultCapabilityValidator::permissive();
        let request = MemoryRequest::Mount {
            source: "/nvme/knowledge-base".to_string(),
            mount_point: "/l3/kb-001".to_string(),
            capability: "cap-001".to_string(),
        };

        let result = router.route_request(request, &validator);
        assert!(result.is_ok());
        let (response, _cost) = result.unwrap();
        match response {
            MemoryResponse::Mounted { mount_id } => {
                assert!(!mount_id.is_empty());
            }
            _ => panic!("Expected Mounted response"),
        }
    }

    #[test]
    fn test_request_router_evict() {
        let mut router = RequestRouter::new(10000);
        let validator = DefaultCapabilityValidator::permissive();
        let request = MemoryRequest::Evict {
            region_id: "region-001".to_string(),
            capability: "cap-001".to_string(),
        };

        let result = router.route_request(request, &validator);
        assert!(result.is_ok());
        let (response, _cost) = result.unwrap();
        match response {
            MemoryResponse::Evicted => {
                // Success
            }
            _ => panic!("Expected Evicted response"),
        }
    }
}
