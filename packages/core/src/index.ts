/**
 * @torry/core — TypeScript API layer for Torry.
 *
 * This is the public API that consumers (CLI, future GUI, programmatic usage)
 * interact with. It wraps the raw NAPI binding with a clean, typed interface
 * and handles orchestration, config, and provider management.
 *
 * Dependency chain: @torry/core → @torry/binding → torry-core (Rust)
 */

import { getCoreVersion, add } from '@torry/binding';
import type { DownloadConfig, DownloadResult } from '@torry/shared';

/**
 * Returns the version of the underlying Rust core engine.
 *
 * This call goes through the full pipeline:
 * TypeScript → NAPI binding → Rust → NAPI binding → TypeScript
 */
export function version(): string {
  return getCoreVersion();
}

/**
 * Adds two numbers via the Rust core engine.
 *
 * Pipeline verification function — proves the entire chain works:
 * TS (here) → @torry/binding (NAPI) → torry-core (Rust) → back.
 *
 * @param a - First number
 * @param b - Second number
 * @returns Sum computed by the Rust engine
 */
export function addViaRust(a: number, b: number): number {
  return add(a, b);
}

// Re-export types so consumers don't need to depend on @torry/shared directly.
// This keeps the public API surface clean: users only need @torry/core.
export type {
  DownloadConfig,
  DownloadResult,
  DownloadProgress,
  DownloadStatus,
} from '@torry/shared';

export { TorryError, TorryErrorCode } from '@torry/shared';
