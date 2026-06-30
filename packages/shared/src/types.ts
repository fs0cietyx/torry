/**
 * Core type definitions for Torry.
 *
 * These types are the shared language between the CLI, core, and providers.
 * Changes here affect the entire system — treat this as a public API contract.
 */

/** Status of a download task. */
export type DownloadStatus =
  | 'pending'
  | 'downloading'
  | 'paused'
  | 'verifying'
  | 'completed'
  | 'failed';

/** Progress information emitted during a download. */
export interface DownloadProgress {
  /** Unique task identifier. */
  readonly taskId: string;
  /** Total size in bytes, null if server doesn't report Content-Length. */
  readonly totalBytes: number | null;
  /** Bytes downloaded so far. */
  readonly downloadedBytes: number;
  /** Current download speed in bytes per second. */
  readonly bytesPerSecond: number;
  /** Current status of the download. */
  readonly status: DownloadStatus;
  /** Percentage complete (0–100), null if total size is unknown. */
  readonly percentage: number | null;
}

/** Configuration for initiating a download. */
export interface DownloadConfig {
  /** URL to download from. */
  readonly url: string;
  /** Output file path. */
  readonly outputPath: string;
  /** Number of parallel chunks. Defaults to DEFAULT_CHUNK_COUNT. */
  readonly chunks?: number | undefined;
  /** Expected hash for post-download verification. */
  readonly expectedHash?: string | undefined;
  /** Hash algorithm to use for verification. */
  readonly hashAlgorithm?: 'sha256' | 'blake3' | undefined;
  /** Maximum retries per chunk on failure. */
  readonly maxRetries?: number | undefined;
}

/** Result returned after a successful download. */
export interface DownloadResult {
  /** Absolute path to the downloaded file. */
  readonly filePath: string;
  /** Total bytes downloaded. */
  readonly totalBytes: number;
  /** Total duration in milliseconds. */
  readonly durationMs: number;
  /** Whether hash verification passed (true if no hash was provided). */
  readonly verified: boolean;
  /** Average download speed in bytes per second. */
  readonly averageSpeed: number;
}
