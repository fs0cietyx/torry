/**
 * Error types for Torry.
 *
 * All errors in the Torry ecosystem extend TorryError and carry a
 * machine-readable TorryErrorCode. This enables programmatic error
 * handling without parsing error messages.
 */

/** Machine-readable error codes for all Torry operations. */
export enum TorryErrorCode {
  /** The download operation failed. */
  DOWNLOAD_FAILED = 'DOWNLOAD_FAILED',
  /** Hash verification did not match the expected value. */
  HASH_MISMATCH = 'HASH_MISMATCH',
  /** A network-level error occurred (DNS, timeout, connection refused). */
  NETWORK_ERROR = 'NETWORK_ERROR',
  /** Failed to write to the local filesystem. */
  FILE_WRITE_ERROR = 'FILE_WRITE_ERROR',
  /** The provided URL is not valid. */
  INVALID_URL = 'INVALID_URL',
  /** No provider found for the given URL scheme. */
  PROVIDER_NOT_FOUND = 'PROVIDER_NOT_FOUND',
  /** Configuration file is invalid or malformed. */
  CONFIG_INVALID = 'CONFIG_INVALID',
  /** Failed to load the native Rust binding. */
  BINDING_LOAD_FAILED = 'BINDING_LOAD_FAILED',
}

/** Base error class for all Torry errors. */
export class TorryError extends Error {
  /** Machine-readable error code for programmatic handling. */
  public readonly code: TorryErrorCode;

  constructor(code: TorryErrorCode, message: string, options?: ErrorOptions) {
    super(message, options);
    this.name = 'TorryError';
    this.code = code;

    // Maintain proper prototype chain for instanceof checks.
    // Required when extending built-in classes in TypeScript.
    Object.setPrototypeOf(this, new.target.prototype);
  }
}
