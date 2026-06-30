/**
 * @torry/shared — Shared types, errors, and constants for the Torry ecosystem.
 *
 * This package is a leaf dependency with zero external dependencies.
 * Both @torry/core and torry (CLI) depend on it.
 */

export type {
  DownloadStatus,
  DownloadProgress,
  DownloadConfig,
  DownloadResult,
} from './types.js';

export { TorryError, TorryErrorCode } from './errors.js';

export {
  DEFAULT_CHUNK_SIZE,
  DEFAULT_CHUNK_COUNT,
  DEFAULT_MAX_RETRIES,
  DEFAULT_TIMEOUT_MS,
  USER_AGENT,
  CONFIG_FILE_NAME,
} from './constants.js';
