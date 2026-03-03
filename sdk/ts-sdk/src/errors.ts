/**
 * Cognitive Substrate SDK - Error Types
 * 
 * Error code definitions following POSIX-like errno conventions.
 * Part of @cognitive-substrate/sdk
 */

/**
 * CSCI Error Code enumeration
 * 
 * Numeric codes match POSIX where applicable, with custom codes for CSCI-specific errors.
 * @see https://github.com/cognitive-substrate/xkernal/blob/main/sdk/csci/docs/csci_v0.1_specification.md
 */
export enum CsciErrorCode {
  /** Success - operation completed without error */
  Success = 0,
  /** Permission denied - caller lacks required capability */
  PermissionDenied = 1,
  /** Not found - referenced resource does not exist */
  NotFound = 2,
  /** Out of memory - insufficient memory available */
  OutOfMemory = 12,
  /** Resource busy - resource is in use and cannot be modified */
  ResourceBusy = 16,
  /** Already exists - resource with this name/ID already exists */
  AlreadyExists = 17,
  /** Invalid argument - syscall arguments do not satisfy preconditions */
  InvalidArgument = 22,
  /** Operation timed out - operation exceeded deadline */
  TimedOut = 110,
  /** Budget exhausted - operation would exceed resource budget */
  BudgetExhausted = 200,
  /** Dependency cycle - cyclic dependency would be created */
  CyclicDependency = 201,
  /** Not implemented - feature not yet implemented */
  Unimplemented = 202,
  /** Channel closed - channel endpoint has been closed */
  ChannelClosed = 203,
  /** Message too large - message exceeds channel capacity */
  MessageTooLarge = 204,
  /** No message - no message available on channel */
  NoMessage = 205,
  /** Sandbox error - sandbox configuration or execution failed */
  SandboxError = 206,
  /** Tool error - tool execution failed */
  ToolError = 207,
  /** Invalid attenuation - attenuation spec is invalid */
  InvalidAttenuation = 208,
  /** Policy violation - operation violates security policy */
  PolicyViolation = 209,
  /** Resource full - resource at capacity cannot accept more */
  ResourceFull = 210,
  /** Buffer overflow - write would exceed buffer capacity */
  BufferOverflow = 211,
}

/**
 * CSCI Error class for syscall errors
 * 
 * Provides type-safe error handling with code and context information.
 */
export class CsciError extends Error {
  /**
   * Create a new CSCI error.
   * 
   * @param code - Error code
   * @param message - Human-readable error message
   * @param context - Optional context information
   */
  constructor(
    public readonly code: CsciErrorCode,
    message: string,
    public readonly context?: Record<string, unknown>,
  ) {
    super(message);
    this.name = 'CsciError';
    Object.setPrototypeOf(this, CsciError.prototype);
  }

  /**
   * Get the numeric error code.
   */
  getCode(): number {
    return this.code;
  }

  /**
   * Get a string representation of the error.
   */
  toString(): string {
    return `CsciError(${this.code}): ${this.message}`;
  }
}
