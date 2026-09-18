export interface AdaptiveSchedulerOptions {
  run: () => void;
  /** Floor for the trailing delay so single edits still coalesce a frame. */
  minDelay?: number;
  /** Ceiling when renders stay cheap. */
  maxDelay?: number;
  /** Delay while the user is typing in bursts. */
  burstDelay?: number;
  burstWindowMs?: number;
  burstCount?: number;
}

export interface AdaptiveScheduler {
  /** Debounce a pending run; repeated calls within a burst extend the wait. */
  schedule(): void;
  /** Run the pending job immediately (bypasses the delay). */
  force(): void;
  /** Drop the pending job. */
  cancel(): void;
}

/**
 * Trailing-edge coalescing whose delay follows the cost of the last render:
 * cheap renders react quickly, expensive ones back off, and sustained typing
 * jumps to the burst delay so long documents parse once per pause.
 */
export function createAdaptiveScheduler(options: AdaptiveSchedulerOptions): AdaptiveScheduler {
  const minDelay = options.minDelay ?? 8;
  const maxDelay = options.maxDelay ?? 120;
  const burstDelay = options.burstDelay ?? 250;
  const burstWindowMs = options.burstWindowMs ?? 400;
  const burstCount = options.burstCount ?? 5;

  let frame = 0;
  let timer: ReturnType<typeof setTimeout> | undefined;
  let pending = false;
  let lastRenderMs = 0;
  const recentRunAt: number[] = [];

  function clearWaiting() {
    cancelAnimationFrame(frame);
    clearTimeout(timer);
    frame = 0;
    timer = undefined;
  }

  function currentDelay() {
    pruneRecentRuns(performance.now());
    if (recentRunAt.length >= burstCount) return burstDelay;
    return Math.min(maxDelay, Math.max(minDelay, lastRenderMs * 1.5));
  }

  function pruneRecentRuns(now: number) {
    while (recentRunAt.length && now - recentRunAt[0] > burstWindowMs) recentRunAt.shift();
  }

  function flush() {
    clearWaiting();
    if (!pending) return;
    pending = false;
    const start = performance.now();
    options.run();
    const end = performance.now();
    lastRenderMs = end - start;
    recentRunAt.push(end);
  }

  return {
    schedule() {
      pending = true;
      clearWaiting();
      // One frame first so the editor caret paints before the heavy render.
      frame = requestAnimationFrame(() => {
        timer = setTimeout(flush, currentDelay());
      });
    },
    force() {
      pending = true;
      flush();
    },
    cancel() {
      pending = false;
      clearWaiting();
    }
  };
}
