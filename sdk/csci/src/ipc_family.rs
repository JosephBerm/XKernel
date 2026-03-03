// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! # CSCI IPC (Inter-Process Communication) Family Syscalls
//!
//! IPC family syscalls enable message-based communication between cognitive tasks:
//! - **chan_open**: Create IPC channel endpoint
//! - **chan_send**: Send typed message on channel
//! - **chan_recv**: Receive typed message from channel
//!
//! # Engineering Plan Reference
//! Section 9: IPC Family Specification.

use crate::error_codes::CsciErrorCode;
use crate::syscall::{ParamType, ReturnType, SyscallDefinition, SyscallFamily, SyscallParam};
use crate::types::{
    CapabilitySet, ChannelID, DeliveryGuarantee, MessagePayload, ProtocolType,
};

/// IPC family syscall numbers.
pub mod number {
    /// chan_open syscall number within IPC family.
    pub const CHAN_OPEN: u8 = 0;
    /// chan_send syscall number within IPC family.
    pub const CHAN_SEND: u8 = 1;
    /// chan_recv syscall number within IPC family.
    pub const CHAN_RECV: u8 = 2;
}

/// Get the definition of the chan_open syscall.
///
/// **chan_open**: Create IPC channel endpoint.
///
/// Creates a new inter-process communication channel with the specified protocol
/// and delivery guarantees. The returned ChannelID is used for subsequent
/// send and receive operations.
///
/// # Parameters
/// - `protocol`: (ProtocolType) Communication protocol (ByteStream, MessageBased, etc.)
/// - `delivery_guarantee`: (DeliveryGuarantee) Delivery guarantee (BestEffort, AtLeastOnce, ExactlyOnce)
/// - `buffer_size`: (Numeric) Channel buffer size in bytes
///
/// # Returns
/// - Success: ChannelID of the newly created channel
/// - Error: CS_EPERM (no IPC capability), CS_ENOMEM (insufficient memory),
///          CS_EINVAL (invalid protocol or buffer size)
///
/// # Preconditions
/// - Caller must have IPC family capability
/// - `protocol` must be a valid protocol type
/// - `buffer_size` must be > 0 and <= maximum channel buffer size
///
/// # Postconditions
/// - Channel is created and ready for use
/// - Channel has immutable ChannelID
/// - Channel buffer is allocated and initialized
/// - Channel is empty (no pending messages)
///
/// # Engineering Plan Reference
/// Section 9.1: chan_open specification.
pub fn chan_open_definition() -> SyscallDefinition {
    SyscallDefinition::new(
        "chan_open",
        SyscallFamily::Channel,
        number::CHAN_OPEN,
        ReturnType::Identifier,
        CapabilitySet::CAP_CHANNEL_FAMILY,
        "Create IPC channel endpoint",
    )
    .with_param(SyscallParam::new(
        "protocol",
        ParamType::Enum,
        "Communication protocol type",
        false,
    ))
    .with_param(SyscallParam::new(
        "delivery_guarantee",
        ParamType::Enum,
        "Message delivery guarantee level",
        false,
    ))
    .with_param(SyscallParam::new(
        "buffer_size",
        ParamType::Numeric,
        "Channel buffer size in bytes",
        false,
    ))
    .with_error(CsciErrorCode::CsSuccess)
    .with_error(CsciErrorCode::CsEperm)
    .with_error(CsciErrorCode::CsEnomem)
    .with_error(CsciErrorCode::CsEinval)
    .with_preconditions(
        "Caller has Channel family capability; protocol valid; buffer_size > 0",
    )
    .with_postconditions(
        "Channel created with immutable ChannelID; buffer allocated; empty state",
    )
}

/// Get the definition of the chan_send syscall.
///
/// **chan_send**: Send typed message on channel.
///
/// Sends a typed message to a channel. The operation respects the channel's
/// delivery guarantees and buffer capacity. If the buffer is full and timeout
/// is specified, the syscall will wait up to the timeout before returning an error.
///
/// # Parameters
/// - `channel_id`: (ChannelID) Target channel ID
/// - `message`: (Memory) Typed message payload (MessagePayload)
/// - `timeout_ms`: (Numeric) Operation timeout in milliseconds, or 0 for non-blocking
///
/// # Returns
/// - Success: Unit (operation successful, message queued)
/// - Error: CS_EINVAL (invalid channel ID or message), CS_ECLOSED (channel closed),
///          CS_ETIMEOUT (operation timed out), CS_EMSGSIZE (message too large)
///
/// # Preconditions
/// - `channel_id` must reference an open channel
/// - Channel must not be closed
/// - Message must be a valid MessagePayload
/// - Message size must not exceed channel buffer capacity
///
/// # Postconditions
/// - Message is queued in channel buffer (respecting delivery guarantee)
/// - Any waiting receivers are notified
/// - If buffer full, sender blocked or timed out per timeout_ms
///
/// # Engineering Plan Reference
/// Section 9.2: chan_send specification.
pub fn chan_send_definition() -> SyscallDefinition {
    SyscallDefinition::new(
        "chan_send",
        SyscallFamily::Channel,
        number::CHAN_SEND,
        ReturnType::Unit,
        CapabilitySet::CAP_CHANNEL_FAMILY,
        "Send typed message on channel",
    )
    .with_param(SyscallParam::new(
        "channel_id",
        ParamType::Identifier,
        "Target channel ID",
        false,
    ))
    .with_param(SyscallParam::new(
        "message",
        ParamType::Memory,
        "Typed message payload",
        false,
    ))
    .with_param(SyscallParam::new(
        "timeout_ms",
        ParamType::Numeric,
        "Operation timeout in milliseconds (0 = non-blocking)",
        true,
    ))
    .with_error(CsciErrorCode::CsSuccess)
    .with_error(CsciErrorCode::CsEinval)
    .with_error(CsciErrorCode::CsEclosed)
    .with_error(CsciErrorCode::CsEtimeout)
    .with_error(CsciErrorCode::CsEmsgsize)
    .with_preconditions(
        "Channel exists and is open; message valid; message size within capacity",
    )
    .with_postconditions(
        "Message queued in buffer; receivers notified; sender may block if buffer full",
    )
}

/// Get the definition of the chan_recv syscall.
///
/// **chan_recv**: Receive typed message from channel.
///
/// Receives a typed message from a channel. If no message is available and timeout
/// is specified, the syscall will wait up to the timeout before returning an error.
///
/// # Parameters
/// - `channel_id`: (ChannelID) Source channel ID
/// - `timeout_ms`: (Numeric) Operation timeout in milliseconds, or 0 for non-blocking
///
/// # Returns
/// - Success: MessagePayload containing the received message
/// - Error: CS_EINVAL (invalid channel ID), CS_ECLOSED (channel closed),
///          CS_ETIMEOUT (operation timed out), CS_ENOMSG (no message available)
///
/// # Preconditions
/// - `channel_id` must reference an open channel
/// - Channel must not be closed
/// - Caller must have receive capability on this channel
///
/// # Postconditions
/// - Message is removed from channel buffer
/// - Sender waiting in buffer-full condition is notified
/// - If no message available, receiver blocked or timed out per timeout_ms
///
/// # Engineering Plan Reference
/// Section 9.3: chan_recv specification.
pub fn chan_recv_definition() -> SyscallDefinition {
    SyscallDefinition::new(
        "chan_recv",
        SyscallFamily::Channel,
        number::CHAN_RECV,
        ReturnType::Memory,
        CapabilitySet::CAP_CHANNEL_FAMILY,
        "Receive typed message from channel",
    )
    .with_param(SyscallParam::new(
        "channel_id",
        ParamType::Identifier,
        "Source channel ID",
        false,
    ))
    .with_param(SyscallParam::new(
        "timeout_ms",
        ParamType::Numeric,
        "Operation timeout in milliseconds (0 = non-blocking)",
        true,
    ))
    .with_error(CsciErrorCode::CsSuccess)
    .with_error(CsciErrorCode::CsEinval)
    .with_error(CsciErrorCode::CsEclosed)
    .with_error(CsciErrorCode::CsEtimeout)
    .with_error(CsciErrorCode::CsEnomsg)
    .with_preconditions("Channel exists and is open; caller has receive capability")
    .with_postconditions(
        "Message removed from buffer; senders notified; receiver may block if empty",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chan_open_definition() {
        let def = chan_open_definition();
        assert_eq!(def.name, "chan_open");
        assert_eq!(def.family, SyscallFamily::Channel);
        assert_eq!(def.number, number::CHAN_OPEN);
        assert_eq!(def.return_type, ReturnType::Identifier);
        assert_eq!(def.parameters.len(), 3);
    }

    #[test]
    fn test_chan_open_parameters() {
        let def = chan_open_definition();
        assert_eq!(def.parameters[0].name, "protocol");
        assert_eq!(def.parameters[1].name, "delivery_guarantee");
        assert_eq!(def.parameters[2].name, "buffer_size");
    }

    #[test]
    fn test_chan_open_errors() {
        let def = chan_open_definition();
        assert!(def.error_codes.len() >= 4);
        assert!(def.error_codes.contains(&CsciErrorCode::CsEperm));
        assert!(def.error_codes.contains(&CsciErrorCode::CsEnomem));
        assert!(def.error_codes.contains(&CsciErrorCode::CsEinval));
    }

    #[test]
    fn test_chan_send_definition() {
        let def = chan_send_definition();
        assert_eq!(def.name, "chan_send");
        assert_eq!(def.family, SyscallFamily::Channel);
        assert_eq!(def.number, number::CHAN_SEND);
        assert_eq!(def.return_type, ReturnType::Unit);
        assert_eq!(def.parameters.len(), 3);
    }

    #[test]
    fn test_chan_send_parameters() {
        let def = chan_send_definition();
        assert_eq!(def.parameters[0].name, "channel_id");
        assert_eq!(def.parameters[1].name, "message");
        assert_eq!(def.parameters[2].name, "timeout_ms");
        assert!(def.parameters[2].optional);
    }

    #[test]
    fn test_chan_send_errors() {
        let def = chan_send_definition();
        assert!(def.error_codes.len() >= 5);
        assert!(def.error_codes.contains(&CsciErrorCode::CsEclosed));
        assert!(def.error_codes.contains(&CsciErrorCode::CsEmsgsize));
    }

    #[test]
    fn test_chan_recv_definition() {
        let def = chan_recv_definition();
        assert_eq!(def.name, "chan_recv");
        assert_eq!(def.family, SyscallFamily::Channel);
        assert_eq!(def.number, number::CHAN_RECV);
        assert_eq!(def.return_type, ReturnType::Memory);
        assert_eq!(def.parameters.len(), 2);
    }

    #[test]
    fn test_chan_recv_parameters() {
        let def = chan_recv_definition();
        assert_eq!(def.parameters[0].name, "channel_id");
        assert_eq!(def.parameters[1].name, "timeout_ms");
        assert!(def.parameters[1].optional);
    }

    #[test]
    fn test_chan_recv_errors() {
        let def = chan_recv_definition();
        assert!(def.error_codes.len() >= 5);
        assert!(def.error_codes.contains(&CsciErrorCode::CsEnomsg));
    }

    #[test]
    fn test_ipc_family_syscall_numbers_unique() {
        assert_ne!(number::CHAN_OPEN, number::CHAN_SEND);
        assert_ne!(number::CHAN_SEND, number::CHAN_RECV);
        assert_ne!(number::CHAN_OPEN, number::CHAN_RECV);
    }

    #[test]
    fn test_all_definitions_have_preconditions() {
        assert!(!chan_open_definition().preconditions.is_empty());
        assert!(!chan_send_definition().preconditions.is_empty());
        assert!(!chan_recv_definition().preconditions.is_empty());
    }

    #[test]
    fn test_all_definitions_have_postconditions() {
        assert!(!chan_open_definition().postconditions.is_empty());
        assert!(!chan_send_definition().postconditions.is_empty());
        assert!(!chan_recv_definition().postconditions.is_empty());
    }
}
