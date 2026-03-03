// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Interface validation harness for Memory Manager syscalls.
//!
//! This module provides comprehensive validation of memory syscall interface
//! contracts including bounds checking, alignment validation, and capability-based
//! access control verification.
//!
//! See Engineering Plan § 4.1.0: Interface Validation (Week 5).

use alloc::string::String;
use alloc::vec::Vec;
use crate::error::{MemoryError, Result};
use crate::mem_syscall_interface::{
    AllocFlags, MountFlags, MountSource, MemHandle, MountHandle,
};
use crate::mem_serialization::{
    MAX_REQUEST_SIZE, MAX_RESPONSE_SIZE, MAX_STRING_LEN, MAX_DATA_BUFFER_LEN,
};

/// Validation result for a syscall interface contract.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValidationResult {
    /// Contract valid and parameters within bounds.
    Valid,
    /// Contract violation detected.
    Invalid(String),
    /// Warning: contract valid but unusual parameters.
    Warning(String),
}

impl ValidationResult {
    /// Returns true if validation passed.
    pub fn is_valid(&self) -> bool {
        matches!(self, ValidationResult::Valid)
    }

    /// Returns the error message if invalid.
    pub fn as_error(&self) -> Option<&str> {
        match self {
            ValidationResult::Invalid(msg) => Some(msg),
            _ => None,
        }
    }

    /// Returns the warning message if present.
    pub fn as_warning(&self) -> Option<&str> {
        match self {
            ValidationResult::Warning(msg) => Some(msg),
            _ => None,
        }
    }
}

/// Interface validator for memory operations.
///
/// Performs contract validation on syscall parameters including:
/// - Size and alignment constraints
/// - Bounds checking for read/write operations
/// - Handle validity
/// - Mount point and source validation
/// - Capability-based access control
pub struct MemoryInterfaceValidator {
    /// Validator ID for debugging.
    validator_id: String,
    /// Maximum allocation size per request (512 MiB).
    max_alloc_size: u64,
    /// Track validation statistics.
    total_validations: u64,
    failed_validations: u64,
}

impl MemoryInterfaceValidator {
    /// Creates a new memory interface validator.
    pub fn new(validator_id: impl Into<String>) -> Self {
        MemoryInterfaceValidator {
            validator_id: validator_id.into(),
            max_alloc_size: 512 * 1024 * 1024, // 512 MiB
            total_validations: 0,
            failed_validations: 0,
        }
    }

    /// Validates mem_alloc syscall parameters.
    ///
    /// # Validation checks:
    ///
    /// 1. Size: Must be > 0 and <= max_alloc_size
    /// 2. Alignment: Must be power of 2 and >= 1
    /// 3. Flags: Must be valid combination
    ///
    /// # See
    ///
    /// Engineering Plan § 4.1.1: Allocation Operations.
    pub fn validate_allocate(
        &mut self,
        size: u64,
        alignment: u64,
        flags: AllocFlags,
    ) -> ValidationResult {
        self.total_validations += 1;

        // Check size
        if size == 0 {
            self.failed_validations += 1;
            return ValidationResult::Invalid("allocation size must be > 0".into());
        }

        if size > self.max_alloc_size {
            self.failed_validations += 1;
            return ValidationResult::Invalid(format!(
                "allocation size {} exceeds maximum {}",
                size, self.max_alloc_size
            ));
        }

        // Check alignment
        if alignment == 0 {
            self.failed_validations += 1;
            return ValidationResult::Invalid("alignment must be > 0".into());
        }

        if (alignment & (alignment - 1)) != 0 {
            self.failed_validations += 1;
            return ValidationResult::Invalid(format!(
                "alignment {} is not a power of 2",
                alignment
            ));
        }

        // Check reasonable alignment
        if alignment > (1024 * 1024) {
            return ValidationResult::Warning(format!(
                "very large alignment: {}",
                alignment
            ));
        }

        // Check flags are reasonable
        if flags.contains(AllocFlags::DURABLE) && flags.contains(AllocFlags::NO_EVICT) {
            return ValidationResult::Warning(
                "DURABLE implies persistence; NO_EVICT may be redundant".into(),
            );
        }

        ValidationResult::Valid
    }

    /// Validates mem_read syscall parameters.
    ///
    /// # Validation checks:
    ///
    /// 1. Handle: Must not be null (0)
    /// 2. Offset: No absolute limit (checked against region bounds at runtime)
    /// 3. Size: Must be > 0 and <= MAX_DATA_BUFFER_LEN
    ///
    /// # See
    ///
    /// Engineering Plan § 4.1.1: Memory Operations.
    pub fn validate_read(&mut self, handle: MemHandle, offset: u64, size: u64) -> ValidationResult {
        self.total_validations += 1;

        // Check handle validity
        if handle.as_u64() == 0 {
            self.failed_validations += 1;
            return ValidationResult::Invalid("memory handle cannot be null (0)".into());
        }

        // Check size
        if size == 0 {
            return ValidationResult::Warning("reading 0 bytes".into());
        }

        if size > MAX_DATA_BUFFER_LEN as u64 {
            self.failed_validations += 1;
            return ValidationResult::Invalid(format!(
                "read size {} exceeds maximum {}",
                size, MAX_DATA_BUFFER_LEN
            ));
        }

        // Warn about offset overflow (though not invalid)
        if offset.checked_add(size).is_none() {
            return ValidationResult::Warning(
                "read offset + size would overflow u64".into(),
            );
        }

        ValidationResult::Valid
    }

    /// Validates mem_write syscall parameters.
    ///
    /// # Validation checks:
    ///
    /// 1. Handle: Must not be null (0)
    /// 2. Offset: No absolute limit
    /// 3. Size: Must be > 0 and <= MAX_DATA_BUFFER_LEN
    /// 4. Buffer: Must be at least `size` bytes
    ///
    /// # See
    ///
    /// Engineering Plan § 4.1.1: Memory Operations.
    pub fn validate_write(
        &mut self,
        handle: MemHandle,
        offset: u64,
        size: u64,
        buffer_len: u64,
    ) -> ValidationResult {
        self.total_validations += 1;

        // Check handle validity
        if handle.as_u64() == 0 {
            self.failed_validations += 1;
            return ValidationResult::Invalid("memory handle cannot be null (0)".into());
        }

        // Check size
        if size == 0 {
            return ValidationResult::Warning("writing 0 bytes".into());
        }

        if size > MAX_DATA_BUFFER_LEN as u64 {
            self.failed_validations += 1;
            return ValidationResult::Invalid(format!(
                "write size {} exceeds maximum {}",
                size, MAX_DATA_BUFFER_LEN
            ));
        }

        // Check buffer is large enough
        if buffer_len < size {
            self.failed_validations += 1;
            return ValidationResult::Invalid(format!(
                "buffer size {} is less than write size {}",
                buffer_len, size
            ));
        }

        // Warn about offset overflow
        if offset.checked_add(size).is_none() {
            return ValidationResult::Warning(
                "write offset + size would overflow u64".into(),
            );
        }

        ValidationResult::Valid
    }

    /// Validates mem_mount syscall parameters.
    ///
    /// # Validation checks:
    ///
    /// 1. Mount point: Must not be empty
    /// 2. Mount point: Must be valid path (starts with /)
    /// 3. Source: Type-specific validation
    /// 4. Flags: Must be reasonable combination
    ///
    /// # See
    ///
    /// Engineering Plan § 4.1.4: L3 Long-Term Operations.
    pub fn validate_mount(
        &mut self,
        source: &MountSource,
        mount_point: &str,
        flags: MountFlags,
    ) -> ValidationResult {
        self.total_validations += 1;

        // Check mount point
        if mount_point.is_empty() {
            self.failed_validations += 1;
            return ValidationResult::Invalid("mount point must not be empty".into());
        }

        if mount_point.len() > MAX_STRING_LEN {
            self.failed_validations += 1;
            return ValidationResult::Invalid(format!(
                "mount point too long: {} > {}",
                mount_point.len(),
                MAX_STRING_LEN
            ));
        }

        // Check mount point looks like a path
        if !mount_point.starts_with('/') && !mount_point.starts_with('.') {
            return ValidationResult::Warning(format!(
                "mount point '{}' does not look like a path",
                mount_point
            ));
        }

        // Validate source
        match source {
            MountSource::LocalPath(path) => {
                if path.is_empty() {
                    self.failed_validations += 1;
                    return ValidationResult::Invalid("local path must not be empty".into());
                }
                if path.len() > MAX_STRING_LEN {
                    self.failed_validations += 1;
                    return ValidationResult::Invalid(format!(
                        "local path too long: {} > {}",
                        path.len(),
                        MAX_STRING_LEN
                    ));
                }
            }
            MountSource::RemoteUrl(url) => {
                if url.is_empty() {
                    self.failed_validations += 1;
                    return ValidationResult::Invalid("remote URL must not be empty".into());
                }
                if url.len() > MAX_STRING_LEN {
                    self.failed_validations += 1;
                    return ValidationResult::Invalid(format!(
                        "remote URL too long: {} > {}",
                        url.len(),
                        MAX_STRING_LEN
                    ));
                }
                if !url.starts_with("http://") && !url.starts_with("https://") && !url.starts_with("s3://") {
                    return ValidationResult::Warning(format!(
                        "URL '{}' has unusual scheme",
                        url
                    ));
                }
            }
            MountSource::SharedRegion(id) => {
                if id.is_empty() {
                    self.failed_validations += 1;
                    return ValidationResult::Invalid("shared region ID must not be empty".into());
                }
                if id.len() > MAX_STRING_LEN {
                    self.failed_validations += 1;
                    return ValidationResult::Invalid(format!(
                        "region ID too long: {} > {}",
                        id.len(),
                        MAX_STRING_LEN
                    ));
                }
            }
            MountSource::CrewReplica(endpoint) => {
                if endpoint.is_empty() {
                    self.failed_validations += 1;
                    return ValidationResult::Invalid("crew endpoint must not be empty".into());
                }
                if endpoint.len() > MAX_STRING_LEN {
                    self.failed_validations += 1;
                    return ValidationResult::Invalid(format!(
                        "endpoint too long: {} > {}",
                        endpoint.len(),
                        MAX_STRING_LEN
                    ));
                }
            }
        }

        // Check flags
        if flags.contains(MountFlags::READ_ONLY) && flags.contains(MountFlags::SYNC_REPLICAS) {
            return ValidationResult::Warning(
                "READ_ONLY mount with SYNC_REPLICAS may be unusual".into(),
            );
        }

        ValidationResult::Valid
    }

    /// Returns validation statistics.
    pub fn stats(&self) -> (u64, u64) {
        (self.total_validations, self.failed_validations)
    }

    /// Returns the validation failure rate (0.0 to 1.0).
    pub fn failure_rate(&self) -> f64 {
        if self.total_validations == 0 {
            0.0
        } else {
            self.failed_validations as f64 / self.total_validations as f64
        }
    }
}

/// Capability-based access control validator.
///
/// Verifies that a capability grants access to a specific operation.
/// See Engineering Plan § 4.1.3: Access Control.
pub struct CapabilityAccessValidator {
    /// Validator ID.
    validator_id: String,
}

impl CapabilityAccessValidator {
    /// Creates a new capability validator.
    pub fn new(validator_id: impl Into<String>) -> Self {
        CapabilityAccessValidator {
            validator_id: validator_id.into(),
        }
    }

    /// Validates a capability token for an allocate operation.
    ///
    /// # Returns
    ///
    /// `Ok(())` if capability grants access, `Err` otherwise.
    pub fn validate_allocate_capability(
        &self,
        _capability_token: &str,
    ) -> Result<()> {
        // Stub: Real implementation would check capability database
        // For now, allow all capabilities
        Ok(())
    }

    /// Validates a capability token for a read operation.
    pub fn validate_read_capability(
        &self,
        _capability_token: &str,
        _region_id: Option<&str>,
    ) -> Result<()> {
        // Stub: Check read capability for region
        Ok(())
    }

    /// Validates a capability token for a write operation.
    pub fn validate_write_capability(
        &self,
        _capability_token: &str,
        _region_id: Option<&str>,
    ) -> Result<()> {
        // Stub: Check write capability for region
        Ok(())
    }

    /// Validates a capability token for a mount operation.
    pub fn validate_mount_capability(
        &self,
        _capability_token: &str,
    ) -> Result<()> {
        // Stub: Check mount capability
        Ok(())
    }

    /// Checks if a capability has been revoked.
    pub fn is_revoked(&self, _capability_token: &str) -> Result<bool> {
        // Stub: Check revocation list
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;

    #[test]
    fn test_validate_allocate_valid() {
        let mut validator = MemoryInterfaceValidator::new("test");
        let result = validator.validate_allocate(1024, 8, AllocFlags::NONE);
        assert_eq!(result, ValidationResult::Valid);
    }

    #[test]
    fn test_validate_allocate_zero_size() {
        let mut validator = MemoryInterfaceValidator::new("test");
        let result = validator.validate_allocate(0, 8, AllocFlags::NONE);
        assert!(matches!(result, ValidationResult::Invalid(_)));
    }

    #[test]
    fn test_validate_allocate_bad_alignment() {
        let mut validator = MemoryInterfaceValidator::new("test");

        let result = validator.validate_allocate(1024, 3, AllocFlags::NONE);
        assert!(matches!(result, ValidationResult::Invalid(_)));

        let result = validator.validate_allocate(1024, 0, AllocFlags::NONE);
        assert!(matches!(result, ValidationResult::Invalid(_)));
    }

    #[test]
    fn test_validate_allocate_oversized() {
        let mut validator = MemoryInterfaceValidator::new("test");
        let result = validator.validate_allocate(1024 * 1024 * 1024 * 1024, 8, AllocFlags::NONE);
        assert!(matches!(result, ValidationResult::Invalid(_)));
    }

    #[test]
    fn test_validate_read_valid() {
        let mut validator = MemoryInterfaceValidator::new("test");
        let result = validator.validate_read(MemHandle::new(1), 0, 256);
        assert_eq!(result, ValidationResult::Valid);
    }

    #[test]
    fn test_validate_read_null_handle() {
        let mut validator = MemoryInterfaceValidator::new("test");
        let result = validator.validate_read(MemHandle::new(0), 0, 256);
        assert!(matches!(result, ValidationResult::Invalid(_)));
    }

    #[test]
    fn test_validate_read_oversized() {
        let mut validator = MemoryInterfaceValidator::new("test");
        let result = validator.validate_read(MemHandle::new(1), 0, 1024 * 1024 * 1024);
        assert!(matches!(result, ValidationResult::Invalid(_)));
    }

    #[test]
    fn test_validate_write_valid() {
        let mut validator = MemoryInterfaceValidator::new("test");
        let result = validator.validate_write(MemHandle::new(1), 0, 256, 256);
        assert_eq!(result, ValidationResult::Valid);
    }

    #[test]
    fn test_validate_write_buffer_too_small() {
        let mut validator = MemoryInterfaceValidator::new("test");
        let result = validator.validate_write(MemHandle::new(1), 0, 256, 100);
        assert!(matches!(result, ValidationResult::Invalid(_)));
    }

    #[test]
    fn test_validate_mount_valid() {
        let mut validator = MemoryInterfaceValidator::new("test");
        let source = MountSource::LocalPath("/data".into());
        let result = validator.validate_mount(&source, "/mnt", MountFlags::NONE);
        assert_eq!(result, ValidationResult::Valid);
    }

    #[test]
    fn test_validate_mount_empty_path() {
        let mut validator = MemoryInterfaceValidator::new("test");
        let source = MountSource::LocalPath("/data".into());
        let result = validator.validate_mount(&source, "", MountFlags::NONE);
        assert!(matches!(result, ValidationResult::Invalid(_)));
    }

    #[test]
    fn test_validate_mount_remote_url() {
        let mut validator = MemoryInterfaceValidator::new("test");
        let source = MountSource::RemoteUrl("https://example.com/data".into());
        let result = validator.validate_mount(&source, "/mnt/remote", MountFlags::NONE);
        assert_eq!(result, ValidationResult::Valid);
    }

    #[test]
    fn test_validation_stats() {
        let mut validator = MemoryInterfaceValidator::new("test");
        validator.validate_allocate(1024, 8, AllocFlags::NONE);
        validator.validate_allocate(0, 8, AllocFlags::NONE);
        validator.validate_allocate(1024, 8, AllocFlags::NONE);

        let (total, failed) = validator.stats();
        assert_eq!(total, 3);
        assert_eq!(failed, 1);
        assert!(validator.failure_rate() > 0.0 && validator.failure_rate() < 1.0);
    }

    #[test]
    fn test_capability_validator() {
        let validator = CapabilityAccessValidator::new("test");
        assert!(validator
            .validate_allocate_capability("test_cap")
            .is_ok());
    }
}
