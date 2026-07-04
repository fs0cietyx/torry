/**
 * Event contracts for the Torry core engine.
 *
 * ┌──────────────────────────────────────────────────────────────────┐
 * │ RULES FOR THIS FILE                                             │
 * │                                                                  │
 * │ 1. All payloads MUST be serializable (JSON-safe).                │
 * │    No functions, no class instances, no circular references.     │
 * │                                                                  │
 * │ 2. Events are NOTIFICATIONS, not state.                          │
 * │    They describe "what happened," not "what the current state    │
 * │    is." Managers own state; events communicate changes.          │
 * │                                                                  │
 * │ 3. No UI-specific events.                                        │
 * │    Events describe domain occurrences. The presentation layer    │
 * │    decides how to render them.                                   │
 * │                                                                  │
 * │ 4. Payloads use named tuple elements for self-documenting APIs.  │
 * │    `[taskId: string, config: DownloadConfig]` not `[string, X]` │
 * └──────────────────────────────────────────────────────────────────┘
 */

import type {
  DownloadProgress,
  DownloadResult,
  DownloadConfig,
} from './types.js';

import type { TorryErrorCode } from './errors.js';

/**
 * Serializable error payload for event transport.
 *
 * We don't send `TorryError` class instances through events because
 * class instances aren't serializable. Instead, we extract the code
 * and message into a plain object.
 */
export interface ErrorPayload {
  /** Machine-readable error code. */
  readonly code: TorryErrorCode;
  /** Human-readable error message. */
  readonly message: string;
}

/**
 * Complete event map for the Torry core engine.
 *
 * Each key is a namespaced event name (domain:action).
 * Each value is a tuple of the event's payload arguments.
 *
 * Usage:
 *   engine.events.on('download:progress', (progress) => { ... })
 *   engine.events.emit('download:progress', progressData)
 */
export interface TorryEventMap {
  [key: string]: unknown[];

  // ─── Download Lifecycle ──────────────────────────────────
  /** A download has been added to the queue. */
  'download:queued': [taskId: string, config: DownloadConfig];
  /** A download has started transferring data. */
  'download:start': [taskId: string];
  /** A download's progress has been updated. */
  'download:progress': [progress: DownloadProgress];
  /** A download has completed successfully. */
  'download:complete': [result: DownloadResult];
  /** A download has failed with an error. */
  'download:error': [taskId: string, error: ErrorPayload];
  /** A download has been paused by the user. */
  'download:paused': [taskId: string];
  /** A paused download has been resumed. */
  'download:resumed': [taskId: string];
  /** A download has been cancelled by the user. */
  'download:cancelled': [taskId: string];

  // ─── Hash Verification ──────────────────────────────────
  /** Hash verification has started for a completed download. */
  'verify:start': [taskId: string, algorithm: string];
  /** Hash verification has completed. */
  'verify:complete': [taskId: string, matched: boolean];

  // ─── Engine Lifecycle ───────────────────────────────────
  /** The engine has initialized and is ready to accept commands. */
  'engine:ready': [];
  /** The engine is shutting down gracefully. */
  'engine:shutdown': [];
  /** An engine-level error has occurred. */
  'engine:error': [error: ErrorPayload];
}
