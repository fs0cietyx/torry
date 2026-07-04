/**
 * @torry/core — TypeScript API layer for Torry.
 *
 * This is the public API that consumers (CLI, future GUI, programmatic usage)
 * interact with. It wraps the raw NAPI binding with a clean, typed interface
 * and handles orchestration, config, and provider management.
 */

export function version(): string {
  return "0.1.0";
}

// Re-export types so consumers don't need to depend on @torry/shared directly.
export type {
  DownloadConfig,
  DownloadResult,
  DownloadProgress,
  DownloadStatus,
} from '@torry/shared';

export { TorryError, TorryErrorCode } from '@torry/shared';

export { EventBus, DisposableGroup } from './event-bus.js';
export { TorryEngine } from './manager.js';
export type { EngineSnapshot, RuntimeContext, TorrentSnapshot } from '@torry/binding';

