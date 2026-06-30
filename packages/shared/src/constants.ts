/**
 * Default values and constants for Torry.
 *
 * These are the system-wide defaults. Users can override them
 * via CLI flags or config file. Providers may further adjust
 * based on protocol requirements.
 */

/** Default chunk size for parallel downloads: 1 MB. */
export const DEFAULT_CHUNK_SIZE = 1024 * 1024;

/** Default number of parallel download chunks. */
export const DEFAULT_CHUNK_COUNT = 8;

/** Default maximum retries per chunk on failure. */
export const DEFAULT_MAX_RETRIES = 3;

/** Default connection timeout in milliseconds: 30 seconds. */
export const DEFAULT_TIMEOUT_MS = 30_000;

/** User agent string sent with HTTP requests. */
export const USER_AGENT = 'Torry/0.1.0';

/** Default config file name looked up in CWD and home directory. */
export const CONFIG_FILE_NAME = 'torry.config.json';
