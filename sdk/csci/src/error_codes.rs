// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! # CSCI Error Codes
//!
//! Syscall error codes following POSIX-like errno conventions with CS_ prefix.
//!
//! Each error code includes:
//! - **Numeric code**: For protocol/ABI compatibility
//! - **Description**: Human-readable explanation
//! - **Category**: Semantic classification (system, capability, resource, etc.)
//!
//! # Engineering Plan Reference
//! Section 4: CSCI Error Codes specification.

use core::fmt;

/// CSCI error code following POSIX-like errno conventions.
///
/// Error codes are numbered to match Unix errno semantics where applicable,
/// with CS_ prefix to avoid collisions. Each error code has a numeric value,
/// description, and category for classification.
///
/// # Numeric Code Allocation
///
/// - CS_SUCCESS: 0
/// - CS_EINVAL: 22 (matches POSIX EINVAL)
/// - CS_ENOMEM: 12 (matches POSIX ENOMEM)
/// - CS_EPERM: 1 (matches POSIX EPERM)
/// - CS_EBUSY: 16 (matches POSIX EBUSY)
/// - CS_ENOENT: 2 (matches POSIX ENOENT)
/// - CS_EEXIST: 17 (matches POSIX EEXIST)
/// - CS_ETIMEOUT: 110 (matches POSIX ETIMEDOUT)
/// - CS_EBUDGET: 200 (CSCI-specific: operation would exceed budget)
/// - CS_ECYCLE: 201 (CSCI-specific: dependency cycle detected)
/// - CS_EUNIMPL: 202 (CSCI-specific: not yet implemented)
///
/// The numeric codes are allocated to avoid conflicts with common POSIX errno values
/// while maintaining semantic compatibility where possible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum CsciErrorCode {
    /// Success: syscall completed without error.
    CsSuccess = 0,

    /// Permission denied: caller lacks required capability.
    ///
    /// The calling agent does not have the capability required to perform
    /// this operation. For example, attempting to allocate memory without
    /// the memory allocation capability.
    CsEperm = 1,

    /// Not found: referenced resource does not exist.
    ///
    /// A syscall parameter referenced a resource (region, task, checkpoint)
    /// that does not exist or has been deallocated.
    CsEnoent = 2,

    /// Out of memory: insufficient memory available.
    ///
    /// The syscall could not allocate the required memory. This may indicate
    /// a system-wide memory shortage or that the caller has exceeded their
    /// memory quota.
    CsEnomem = 12,

    /// Resource busy: resource is in use and cannot be modified.
    ///
    /// A resource is locked or in use by another agent. For example,
    /// attempting to resume a task that is currently executing.
    CsEbusy = 16,

    /// Already exists: resource with this name/ID already exists.
    ///
    /// Attempting to create a resource that already exists. For example,
    /// mounting a memory region at a mount point that is already occupied.
    CsEexist = 17,

    /// Invalid argument: syscall arguments are invalid.
    ///
    /// One or more syscall arguments do not satisfy preconditions. For example,
    /// passing an invalid size, misaligned offset, or unsupported configuration.
    CsEinval = 22,

    /// Operation timed out: operation did not complete within time limit.
    ///
    /// A timed operation (yield with timeout, etc.) exceeded its deadline
    /// without completing.
    CsEtimeout = 110,

    /// Budget exhausted: operation would exceed resource budget.
    ///
    /// The syscall would consume more of the agent's resource budget than
    /// is available. This is specific to cognitive substrate budget tracking.
    CsEbudget = 200,

    /// Dependency cycle: cyclic dependency would be created.
    ///
    /// Creating the requested resource would introduce a dependency cycle
    /// in the system. For example, spawning a child task with a circular
    /// dependency on its parent.
    CsEcycle = 201,

    /// Not implemented: syscall or feature not yet implemented.
    ///
    /// The requested syscall or feature is not yet implemented in this
    /// version of CSCI. This indicates the interface exists but the
    /// implementation is not available.
    CsEunimpl = 202,

    /// Channel closed: channel endpoint has been closed.
    ///
    /// An operation was attempted on a channel that has been closed
    /// or deallocated.
    CsEclosed = 203,

    /// Message size exceeded: message too large for channel.
    ///
    /// A message being sent exceeds the maximum size for the channel buffer.
    CsEmsgsize = 204,

    /// No message: no message available on channel.
    ///
    /// A receive operation on a channel returned no pending message.
    CsEnomsg = 205,

    /// Sandbox error: sandbox configuration or execution failure.
    ///
    /// A tool binding or invocation failed due to sandbox configuration,
    /// policy violation, or execution environment failure.
    CsEsandbox = 206,

    /// Tool error: tool execution failed.
    ///
    /// A tool invocation completed but returned an error status.
    CsEtoolerr = 207,

    /// Invalid attenuation: attenuation specification invalid.
    ///
    /// A capability attenuation cannot be applied; the spec is invalid
    /// or insufficient.
    CsEnoattn = 208,

    /// Policy violation: operation violates security policy.
    ///
    /// An operation was denied due to a mandatory security policy constraint.
    CsEpolicy = 209,

    /// Resource full: resource is at capacity and cannot accept more.
    ///
    /// A resource (crew, channel buffer, etc.) is at its maximum capacity
    /// and cannot accept additional elements.
    CsEfull = 210,

    /// Buffer overflow: write would exceed buffer capacity.
    ///
    /// A telemetry or trace operation would exceed the available buffer space.
    CsEbuffer = 211,
}

/// Error category for semantic classification.
///
/// Error codes are grouped into categories to help callers understand
/// the class of error and determine appropriate recovery strategies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    /// Success (no error).
    Success,
    /// Capability/permission issues.
    Capability,
    /// Resource not found.
    NotFound,
    /// Resource exhaustion (memory, budget).
    ResourceExhaustion,
    /// Resource conflict (busy, exists).
    ResourceConflict,
    /// Argument validation.
    InvalidArgument,
    /// Timeout/deadline.
    Timeout,
    /// Logic error (cycles).
    LogicError,
    /// Feature not yet implemented.
    Unimplemented,
}

impl CsciErrorCode {
    /// Get the numeric value of this error code.
    ///
    /// # Engineering Plan Reference
    /// Section 4.1: Error code numeric values.
    pub const fn code(&self) -> u32 {
        *self as u32
    }

    /// Check if this error code indicates success.
    pub const fn is_success(&self) -> bool {
        matches!(self, Self::CsSuccess)
    }

    /// Get the category of this error code.
    ///
    /// # Engineering Plan Reference
    /// Section 4.2: Error code categories.
    pub const fn category(&self) -> ErrorCategory {
        match self {
            Self::CsSuccess => ErrorCategory::Success,
            Self::CsEperm => ErrorCategory::Capability,
            Self::CsEnoent => ErrorCategory::NotFound,
            Self::CsEnomem => ErrorCategory::ResourceExhaustion,
            Self::CsEbusy => ErrorCategory::ResourceConflict,
            Self::CsEexist => ErrorCategory::ResourceConflict,
            Self::CsEinval => ErrorCategory::InvalidArgument,
            Self::CsEtimeout => ErrorCategory::Timeout,
            Self::CsEbudget => ErrorCategory::ResourceExhaustion,
            Self::CsEcycle => ErrorCategory::LogicError,
            Self::CsEunimpl => ErrorCategory::Unimplemented,
            Self::CsEclosed => ErrorCategory::ResourceConflict,
            Self::CsEmsgsize => ErrorCategory::InvalidArgument,
            Self::CsEnomsg => ErrorCategory::NotFound,
            Self::CsEsandbox => ErrorCategory::Capability,
            Self::CsEtoolerr => ErrorCategory::ResourceExhaustion,
            Self::CsEnoattn => ErrorCategory::InvalidArgument,
            Self::CsEpolicy => ErrorCategory::Capability,
            Self::CsEfull => ErrorCategory::ResourceConflict,
            Self::CsEbuffer => ErrorCategory::ResourceExhaustion,
        }
    }

    /// Get a short name for this error code (e.g., "EPERM", "ENOMEM").
    pub const fn name(&self) -> &'static str {
        match self {
            Self::CsSuccess => "SUCCESS",
            Self::CsEperm => "EPERM",
            Self::CsEnoent => "ENOENT",
            Self::CsEnomem => "ENOMEM",
            Self::CsEbusy => "EBUSY",
            Self::CsEexist => "EEXIST",
            Self::CsEinval => "EINVAL",
            Self::CsEtimeout => "ETIMEOUT",
            Self::CsEbudget => "EBUDGET",
            Self::CsEcycle => "ECYCLE",
            Self::CsEunimpl => "EUNIMPL",
            Self::CsEclosed => "ECLOSED",
            Self::CsEmsgsize => "EMSGSIZE",
            Self::CsEnomsg => "ENOMSG",
            Self::CsEsandbox => "ESANDBOX",
            Self::CsEtoolerr => "ETOOLERR",
            Self::CsEnoattn => "ENOATTN",
            Self::CsEpolicy => "EPOLICY",
            Self::CsEfull => "EFULL",
            Self::CsEbuffer => "EBUFFER",
        }
    }

    /// Get a human-readable description of this error.
    pub const fn description(&self) -> &'static str {
        match self {
            Self::CsSuccess => "Success: operation completed without error",
            Self::CsEperm => {
                "Permission denied: caller lacks required capability"
            }
            Self::CsEnoent => {
                "Not found: referenced resource does not exist"
            }
            Self::CsEnomem => {
                "Out of memory: insufficient memory available"
            }
            Self::CsEbusy => {
                "Resource busy: resource is in use and cannot be modified"
            }
            Self::CsEexist => {
                "Already exists: resource with this name/ID already exists"
            }
            Self::CsEinval => {
                "Invalid argument: syscall arguments do not satisfy preconditions"
            }
            Self::CsEtimeout => {
                "Operation timed out: operation exceeded deadline"
            }
            Self::CsEbudget => {
                "Budget exhausted: operation would exceed resource budget"
            }
            Self::CsEcycle => {
                "Dependency cycle: cyclic dependency would be created"
            }
            Self::CsEunimpl => {
                "Not implemented: feature not yet implemented"
            }
            Self::CsEclosed => {
                "Channel closed: channel endpoint has been closed"
            }
            Self::CsEmsgsize => {
                "Message too large: message exceeds channel capacity"
            }
            Self::CsEnomsg => {
                "No message: no message available on channel"
            }
            Self::CsEsandbox => {
                "Sandbox error: sandbox configuration or execution failed"
            }
            Self::CsEtoolerr => {
                "Tool error: tool execution failed"
            }
            Self::CsEnoattn => {
                "Invalid attenuation: attenuation spec is invalid"
            }
            Self::CsEpolicy => {
                "Policy violation: operation violates security policy"
            }
            Self::CsEfull => {
                "Resource full: resource at capacity cannot accept more"
            }
            Self::CsEbuffer => {
                "Buffer overflow: write would exceed buffer capacity"
            }
        }
    }

    /// Try to convert a numeric code to a CsciErrorCode.
    pub const fn from_code(code: u32) -> Option<Self> {
        match code {
            0 => Some(Self::CsSuccess),
            1 => Some(Self::CsEperm),
            2 => Some(Self::CsEnoent),
            12 => Some(Self::CsEnomem),
            16 => Some(Self::CsEbusy),
            17 => Some(Self::CsEexist),
            22 => Some(Self::CsEinval),
            110 => Some(Self::CsEtimeout),
            200 => Some(Self::CsEbudget),
            201 => Some(Self::CsEcycle),
            202 => Some(Self::CsEunimpl),
            203 => Some(Self::CsEclosed),
            204 => Some(Self::CsEmsgsize),
            205 => Some(Self::CsEnomsg),
            206 => Some(Self::CsEsandbox),
            207 => Some(Self::CsEtoolerr),
            208 => Some(Self::CsEnoattn),
            209 => Some(Self::CsEpolicy),
            210 => Some(Self::CsEfull),
            211 => Some(Self::CsEbuffer),
            _ => None,
        }
    }
}

impl fmt::Display for CsciErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CS_{} ({}): {}", self.name(), self.code(), self.description())
    }
}

impl ErrorCategory {
    /// Get a description of this error category.
    pub const fn description(&self) -> &'static str {
        match self {
            Self::Success => "Operation completed successfully",
            Self::Capability => "Caller lacks required capability or permission",
            Self::NotFound => "Referenced resource does not exist",
            Self::ResourceExhaustion => "System or caller resource limit reached",
            Self::ResourceConflict => {
                "Resource is locked or already exists"
            }
            Self::InvalidArgument => "Syscall arguments violate preconditions",
            Self::Timeout => "Operation exceeded deadline",
            Self::LogicError => "Logic error in operation (e.g., cycles)",
            Self::Unimplemented => "Feature not yet implemented",
        }
    }
}

impl fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_error_code_values() {
        assert_eq!(CsciErrorCode::CsSuccess.code(), 0);
        assert_eq!(CsciErrorCode::CsEperm.code(), 1);
        assert_eq!(CsciErrorCode::CsEnoent.code(), 2);
        assert_eq!(CsciErrorCode::CsEnomem.code(), 12);
        assert_eq!(CsciErrorCode::CsEbusy.code(), 16);
        assert_eq!(CsciErrorCode::CsEexist.code(), 17);
        assert_eq!(CsciErrorCode::CsEinval.code(), 22);
        assert_eq!(CsciErrorCode::CsEtimeout.code(), 110);
        assert_eq!(CsciErrorCode::CsEbudget.code(), 200);
        assert_eq!(CsciErrorCode::CsEcycle.code(), 201);
        assert_eq!(CsciErrorCode::CsEunimpl.code(), 202);
    }

    #[test]
    fn test_error_code_success() {
        assert!(CsciErrorCode::CsSuccess.is_success());
        assert!(!CsciErrorCode::CsEinval.is_success());
        assert!(!CsciErrorCode::CsEnomem.is_success());
    }

    #[test]
    fn test_error_code_categories() {
        assert_eq!(
            CsciErrorCode::CsSuccess.category(),
            ErrorCategory::Success
        );
        assert_eq!(CsciErrorCode::CsEperm.category(), ErrorCategory::Capability);
        assert_eq!(
            CsciErrorCode::CsEnoent.category(),
            ErrorCategory::NotFound
        );
        assert_eq!(
            CsciErrorCode::CsEnomem.category(),
            ErrorCategory::ResourceExhaustion
        );
        assert_eq!(CsciErrorCode::CsEbusy.category(), ErrorCategory::ResourceConflict);
        assert_eq!(
            CsciErrorCode::CsEexist.category(),
            ErrorCategory::ResourceConflict
        );
        assert_eq!(
            CsciErrorCode::CsEinval.category(),
            ErrorCategory::InvalidArgument
        );
        assert_eq!(
            CsciErrorCode::CsEtimeout.category(),
            ErrorCategory::Timeout
        );
        assert_eq!(
            CsciErrorCode::CsEbudget.category(),
            ErrorCategory::ResourceExhaustion
        );
        assert_eq!(CsciErrorCode::CsEcycle.category(), ErrorCategory::LogicError);
        assert_eq!(
            CsciErrorCode::CsEunimpl.category(),
            ErrorCategory::Unimplemented
        );
    }

    #[test]
    fn test_error_code_names() {
        assert_eq!(CsciErrorCode::CsSuccess.name(), "SUCCESS");
        assert_eq!(CsciErrorCode::CsEperm.name(), "EPERM");
        assert_eq!(CsciErrorCode::CsEnoent.name(), "ENOENT");
        assert_eq!(CsciErrorCode::CsEnomem.name(), "ENOMEM");
        assert_eq!(CsciErrorCode::CsEbudget.name(), "EBUDGET");
    }

    #[test]
    fn test_error_code_from_code() {
        assert_eq!(CsciErrorCode::from_code(0), Some(CsciErrorCode::CsSuccess));
        assert_eq!(CsciErrorCode::from_code(1), Some(CsciErrorCode::CsEperm));
        assert_eq!(CsciErrorCode::from_code(200), Some(CsciErrorCode::CsEbudget));
        assert_eq!(CsciErrorCode::from_code(999), None);
    }

    #[test]
    fn test_error_code_display() {
        let err = CsciErrorCode::CsEinval;
        let display_str = err.to_string();
        assert!(display_str.contains("EINVAL"));
        assert!(display_str.contains("22"));
    }

    #[test]
    fn test_error_category_descriptions() {
        assert!(!ErrorCategory::Success.description().is_empty());
        assert!(!ErrorCategory::Capability.description().is_empty());
        assert!(!ErrorCategory::NotFound.description().is_empty());
    }

    #[test]
    fn test_error_code_descriptions_not_empty() {
        assert!(!CsciErrorCode::CsSuccess.description().is_empty());
        assert!(!CsciErrorCode::CsEperm.description().is_empty());
        assert!(!CsciErrorCode::CsEbudget.description().is_empty());
    }
}
