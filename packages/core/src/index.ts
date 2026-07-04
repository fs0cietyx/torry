/**
 * @fs0cietyx/core — TypeScript API layer for Torry.
 *
 * This is the public API that consumers (CLI, future GUI, programmatic usage)
 * interact with. It wraps the raw NAPI binding with a clean, typed interface
 * and handles orchestration, config, and provider management.
 */

export function version(): string {
  return "0.1.0";
}

// Re-export types so consumers don't need to depend on @fs0cietyx/shared directly.
export type {
  DownloadConfig,
  DownloadResult,
  DownloadProgress,
  DownloadStatus,
} from '@fs0cietyx/shared';

export { TorryError, TorryErrorCode } from '@fs0cietyx/shared';

export { EventBus, DisposableGroup } from './event-bus.js';
export { TorryEngine } from './manager.js';
export type { EngineSnapshot, RuntimeContext, TorrentSnapshot } from '@fs0cietyx/binding';

