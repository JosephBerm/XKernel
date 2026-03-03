"""CSCI error codes and exceptions.

Mirrors sdk/csci/src/error_codes.rs — every numeric code, name,
description, and category is kept in sync with the Rust source.
"""

from __future__ import annotations

from enum import IntEnum


class ErrorCategory(IntEnum):
    """Semantic classification of CSCI errors."""
    SUCCESS = 0
    CAPABILITY = 1
    NOT_FOUND = 2
    RESOURCE_EXHAUSTION = 3
    RESOURCE_CONFLICT = 4
    INVALID_ARGUMENT = 5
    TIMEOUT = 6
    LOGIC_ERROR = 7
    UNIMPLEMENTED = 8


class CsciErrorCode(IntEnum):
    """CSCI syscall error codes — POSIX-like with CS_ prefix.

    Numeric values match the Rust CsciErrorCode repr(u32).
    """
    CS_SUCCESS = 0
    CS_EPERM = 1
    CS_ENOENT = 2
    CS_ENOMEM = 12
    CS_EBUSY = 16
    CS_EEXIST = 17
    CS_EINVAL = 22
    CS_ETIMEOUT = 110
    CS_EBUDGET = 200
    CS_ECYCLE = 201
    CS_EUNIMPL = 202
    CS_ECLOSED = 203
    CS_EMSGSIZE = 204
    CS_ENOMSG = 205
    CS_ESANDBOX = 206
    CS_ETOOLERR = 207
    CS_ENOATTN = 208
    CS_EPOLICY = 209
    CS_EFULL = 210
    CS_EBUFFER = 211

    @property
    def short_name(self) -> str:
        """Short name like 'EPERM', 'ENOMEM'."""
        return _NAMES[self]

    @property
    def description(self) -> str:
        """Human-readable description."""
        return _DESCRIPTIONS[self]

    @property
    def category(self) -> ErrorCategory:
        """Semantic category for this error code."""
        return _CATEGORIES[self]

    @property
    def is_success(self) -> bool:
        return self == CsciErrorCode.CS_SUCCESS


# ── Lookup tables (mirrors the Rust match arms exactly) ──────────────────────

_NAMES: dict[CsciErrorCode, str] = {
    CsciErrorCode.CS_SUCCESS: "SUCCESS",
    CsciErrorCode.CS_EPERM: "EPERM",
    CsciErrorCode.CS_ENOENT: "ENOENT",
    CsciErrorCode.CS_ENOMEM: "ENOMEM",
    CsciErrorCode.CS_EBUSY: "EBUSY",
    CsciErrorCode.CS_EEXIST: "EEXIST",
    CsciErrorCode.CS_EINVAL: "EINVAL",
    CsciErrorCode.CS_ETIMEOUT: "ETIMEOUT",
    CsciErrorCode.CS_EBUDGET: "EBUDGET",
    CsciErrorCode.CS_ECYCLE: "ECYCLE",
    CsciErrorCode.CS_EUNIMPL: "EUNIMPL",
    CsciErrorCode.CS_ECLOSED: "ECLOSED",
    CsciErrorCode.CS_EMSGSIZE: "EMSGSIZE",
    CsciErrorCode.CS_ENOMSG: "ENOMSG",
    CsciErrorCode.CS_ESANDBOX: "ESANDBOX",
    CsciErrorCode.CS_ETOOLERR: "ETOOLERR",
    CsciErrorCode.CS_ENOATTN: "ENOATTN",
    CsciErrorCode.CS_EPOLICY: "EPOLICY",
    CsciErrorCode.CS_EFULL: "EFULL",
    CsciErrorCode.CS_EBUFFER: "EBUFFER",
}

_DESCRIPTIONS: dict[CsciErrorCode, str] = {
    CsciErrorCode.CS_SUCCESS: "Success: operation completed without error",
    CsciErrorCode.CS_EPERM: "Permission denied: caller lacks required capability",
    CsciErrorCode.CS_ENOENT: "Not found: referenced resource does not exist",
    CsciErrorCode.CS_ENOMEM: "Out of memory: insufficient memory available",
    CsciErrorCode.CS_EBUSY: "Resource busy: resource is in use and cannot be modified",
    CsciErrorCode.CS_EEXIST: "Already exists: resource with this name/ID already exists",
    CsciErrorCode.CS_EINVAL: "Invalid argument: syscall arguments do not satisfy preconditions",
    CsciErrorCode.CS_ETIMEOUT: "Operation timed out: operation exceeded deadline",
    CsciErrorCode.CS_EBUDGET: "Budget exhausted: operation would exceed resource budget",
    CsciErrorCode.CS_ECYCLE: "Dependency cycle: cyclic dependency would be created",
    CsciErrorCode.CS_EUNIMPL: "Not implemented: feature not yet implemented",
    CsciErrorCode.CS_ECLOSED: "Channel closed: channel endpoint has been closed",
    CsciErrorCode.CS_EMSGSIZE: "Message too large: message exceeds channel capacity",
    CsciErrorCode.CS_ENOMSG: "No message: no message available on channel",
    CsciErrorCode.CS_ESANDBOX: "Sandbox error: sandbox configuration or execution failed",
    CsciErrorCode.CS_ETOOLERR: "Tool error: tool execution failed",
    CsciErrorCode.CS_ENOATTN: "Invalid attenuation: attenuation spec is invalid",
    CsciErrorCode.CS_EPOLICY: "Policy violation: operation violates security policy",
    CsciErrorCode.CS_EFULL: "Resource full: resource at capacity cannot accept more",
    CsciErrorCode.CS_EBUFFER: "Buffer overflow: write would exceed buffer capacity",
}

_CATEGORIES: dict[CsciErrorCode, ErrorCategory] = {
    CsciErrorCode.CS_SUCCESS: ErrorCategory.SUCCESS,
    CsciErrorCode.CS_EPERM: ErrorCategory.CAPABILITY,
    CsciErrorCode.CS_ENOENT: ErrorCategory.NOT_FOUND,
    CsciErrorCode.CS_ENOMEM: ErrorCategory.RESOURCE_EXHAUSTION,
    CsciErrorCode.CS_EBUSY: ErrorCategory.RESOURCE_CONFLICT,
    CsciErrorCode.CS_EEXIST: ErrorCategory.RESOURCE_CONFLICT,
    CsciErrorCode.CS_EINVAL: ErrorCategory.INVALID_ARGUMENT,
    CsciErrorCode.CS_ETIMEOUT: ErrorCategory.TIMEOUT,
    CsciErrorCode.CS_EBUDGET: ErrorCategory.RESOURCE_EXHAUSTION,
    CsciErrorCode.CS_ECYCLE: ErrorCategory.LOGIC_ERROR,
    CsciErrorCode.CS_EUNIMPL: ErrorCategory.UNIMPLEMENTED,
    CsciErrorCode.CS_ECLOSED: ErrorCategory.RESOURCE_CONFLICT,
    CsciErrorCode.CS_EMSGSIZE: ErrorCategory.INVALID_ARGUMENT,
    CsciErrorCode.CS_ENOMSG: ErrorCategory.NOT_FOUND,
    CsciErrorCode.CS_ESANDBOX: ErrorCategory.CAPABILITY,
    CsciErrorCode.CS_ETOOLERR: ErrorCategory.RESOURCE_EXHAUSTION,
    CsciErrorCode.CS_ENOATTN: ErrorCategory.INVALID_ARGUMENT,
    CsciErrorCode.CS_EPOLICY: ErrorCategory.CAPABILITY,
    CsciErrorCode.CS_EFULL: ErrorCategory.RESOURCE_CONFLICT,
    CsciErrorCode.CS_EBUFFER: ErrorCategory.RESOURCE_EXHAUSTION,
}


# ── Exception hierarchy ─────────────────────────────────────────────────────


class CsciError(Exception):
    """Base exception for all XKernal CSCI errors.

    Carries the CSCI error code, HTTP status, and an optional detail message.
    """

    def __init__(
        self,
        code: CsciErrorCode,
        detail: str = "",
        http_status: int | None = None,
    ) -> None:
        self.code = code
        self.detail = detail or code.description
        self.http_status = http_status
        super().__init__(f"CS_{code.short_name} ({int(code)}): {self.detail}")


class PermissionError_(CsciError):
    """Agent lacks a required capability."""
    def __init__(self, detail: str = "") -> None:
        super().__init__(CsciErrorCode.CS_EPERM, detail, 403)


class NotFoundError(CsciError):
    """Referenced resource does not exist."""
    def __init__(self, detail: str = "") -> None:
        super().__init__(CsciErrorCode.CS_ENOENT, detail, 404)


class InvalidArgumentError(CsciError):
    """Syscall arguments are invalid."""
    def __init__(self, detail: str = "") -> None:
        super().__init__(CsciErrorCode.CS_EINVAL, detail, 400)


class TimeoutError_(CsciError):
    """Operation exceeded its deadline."""
    def __init__(self, detail: str = "") -> None:
        super().__init__(CsciErrorCode.CS_ETIMEOUT, detail, 408)


class ChannelClosedError(CsciError):
    """IPC channel has been closed."""
    def __init__(self, detail: str = "") -> None:
        super().__init__(CsciErrorCode.CS_ECLOSED, detail, 410)


class DaemonConnectionError(CsciError):
    """Cannot connect to the cs-daemon."""
    def __init__(self, detail: str = "") -> None:
        super().__init__(CsciErrorCode.CS_ENOENT, detail or "cannot connect to cs-daemon")
