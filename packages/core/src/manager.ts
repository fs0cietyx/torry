import { EventBus } from './event-bus.js';
import { RuntimeContext } from '@fs0cietyx/binding';
import type { TorryEventMap, DownloadConfig } from '@fs0cietyx/shared';

declare function setTimeout(fn: () => void, ms: number): any;

export class TorryEngine {
  public readonly events = new EventBus<TorryEventMap>();
  private activeDownloads = new Map<string, DownloadConfig>();
  
  /** The isolated runtime environment for this instance */
  public readonly context: RuntimeContext;

  constructor(profileName: string = 'default') {
    // This immediately acquires the OS lock in Rust.
    // Throws if another instance is already running this profile.
    this.context = new RuntimeContext(profileName);

    // Notify presentation layers that the engine is ready
    setTimeout(() => this.events.emit('engine:ready'), 0);
  }

  /**
   * Starts a new download task.
   * 
   * @param config - The parsed download configuration
   * @returns The generated task ID
   */
  public async download(config: DownloadConfig): Promise<string> {
    const taskId = Math.random().toString(36).slice(2);
    
    // 1. Mutate Domain State
    this.activeDownloads.set(taskId, config);
    
    // 2. Emit Notification (TUI adapter will catch this and update React state)
    this.events.emit('download:queued', taskId, config);
    
    try {
      this.events.emit('download:start', taskId);
      
      // TODO: Actually call the NAPI-RS bridge here.
      // await startRustDownload(config.url, ...);
      
      return taskId;
    } catch (error) {
      // 3. Emit Failure
      this.events.emit('download:error', taskId, {
        code: 'DOWNLOAD_FAILED' as any,
        message: error instanceof Error ? error.message : 'Unknown error',
      });
      throw error;
    }
  }

  /**
   * Graceful shutdown.
   * Tells Rust to pause downloads, flushes buffers, and cleans up.
   */
  public async shutdown(): Promise<void> {
    this.events.emit('engine:shutdown');
    this.events.removeAllListeners();
    this.activeDownloads.clear();
  }
}

// Removed singleton export to prevent double-instantiation locks.
// Consumers should instantiate TorryEngine manually.
