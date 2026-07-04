/**
 * Custom typed EventBus with disposable subscriptions.
 *
 * ┌──────────────────────────────────────────────────────────────┐
 * │ WHY NOT Node.js EventEmitter?                                │
 * │                                                              │
 * │ 1. Disposable pattern: on() returns an unsubscribe function. │
 * │    Node's EventEmitter returns `this` — useless for cleanup. │
 * │                                                              │
 * │ 2. Type safety: Event names and payloads are enforced at     │
 * │    compile time. Node's EventEmitter uses `string | symbol`. │
 * │                                                              │
 * │ 3. Leak prevention: maxListeners warning, DisposableGroup    │
 * │    for bulk cleanup, idempotent dispose functions.            │
 * │                                                              │
 * │ 4. Error isolation: A failing listener doesn't kill other    │
 * │    listeners or the emitter.                                 │
 * └──────────────────────────────────────────────────────────────┘
/** A function that cleans up a subscription. Idempotent — safe to call multiple times. */
export type Disposable = () => void;

declare const console: {
  warn(msg: string): void;
  error(msg: string, ...args: any[]): void;
};

/** A listener function that receives event arguments. */
type Listener<T extends any[]> = (...args: T) => void;

export class EventBus<EventMap extends Record<string, any[]>> {
  private listeners = new Map<string, Set<Listener<any[]>>>();
  private readonly maxListeners: number;

  constructor(options?: { maxListeners?: number | undefined }) {
    this.maxListeners = options?.maxListeners ?? 20;
  }

  on<K extends keyof EventMap & string>(
    event: K,
    listener: Listener<EventMap[K]>,
  ): Disposable {
    if (!this.listeners.has(event)) {
      this.listeners.set(event, new Set());
    }

    const set = this.listeners.get(event)!;

    if (set.size >= this.maxListeners) {
      console.warn(
        `[EventBus] Warning: ${String(set.size + 1)} listeners for "${event}". ` +
          'Possible memory leak. Call removeAllListeners() or verify subscriptions.',
      );
    }

    const typedListener = listener as unknown as Listener<any[]>;
    set.add(typedListener);

    let disposed = false;
    return () => {
      if (!disposed) {
        disposed = true;
        set.delete(typedListener);
      }
    };
  }

  once<K extends keyof EventMap & string>(
    event: K,
    listener: Listener<EventMap[K]>,
  ): Disposable {
    const dispose = this.on(event, ((...args: EventMap[K]) => {
      dispose();
      listener(...args);
    }) as Listener<EventMap[K]>);
    return dispose;
  }

  emit<K extends keyof EventMap & string>(
    event: K,
    ...args: EventMap[K]
  ): void {
    const set = this.listeners.get(event);
    if (!set || set.size === 0) return;

    for (const listener of set) {
      try {
        (listener as unknown as Listener<EventMap[K]>)(...args);
      } catch (error) {
        console.error(
          `[EventBus] Error in listener for "${event}":`,
          error instanceof Error ? error.message : error,
        );
      }
    }
  }

  /**
   * Remove all listeners for a specific event, or all events if no
   * event name is provided.
   */
  removeAllListeners(event?: keyof EventMap & string): void {
    if (event !== undefined) {
      this.listeners.delete(event);
    } else {
      this.listeners.clear();
    }
  }

  /**
   * Returns the number of active listeners for a specific event.
   * Useful for debugging and testing.
   */
  listenerCount(event: keyof EventMap & string): number {
    return this.listeners.get(event)?.size ?? 0;
  }
}

/**
 * Collects multiple disposables for bulk cleanup.
 *
 * Essential for component lifecycle management:
 * - TUI component mounts → adds subscriptions to a DisposableGroup
 * - TUI component unmounts → calls group.dispose() to clean up all at once
 * - CLI command starts → adds subscriptions to a DisposableGroup
 * - CLI command exits → calls group.dispose()
 *
 * @example
 * ```typescript
 * const subs = new DisposableGroup();
 *
 * subs.add(engine.events.on('download:progress', renderProgress));
 * subs.add(engine.events.on('download:complete', renderDone));
 * subs.add(engine.events.on('download:error', renderError));
 *
 * // Later: clean up everything
 * subs.dispose();
 * ```
 */
export class DisposableGroup {
  private disposables: Disposable[] = [];

  /** Add a disposable to the group. */
  add(disposable: Disposable): void {
    this.disposables.push(disposable);
  }

  /** Dispose all collected disposables and clear the group. */
  dispose(): void {
    for (const d of this.disposables) {
      d();
    }
    this.disposables = [];
  }

  /** Number of disposables currently in the group. */
  get size(): number {
    return this.disposables.length;
  }
}
