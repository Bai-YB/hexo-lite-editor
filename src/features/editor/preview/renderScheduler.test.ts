import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createAdaptiveScheduler, type AdaptiveSchedulerOptions } from "./renderScheduler";

const FRAME_MS = 16;

describe("createAdaptiveScheduler", () => {
  let clock = 0;
  let runs: number[] = [];
  let costMs = 0;

  beforeEach(() => {
    vi.useFakeTimers();
    clock = 0;
    runs = [];
    costMs = 0;
    vi.spyOn(performance, "now").mockImplementation(() => clock);
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  function makeScheduler(overrides?: Partial<AdaptiveSchedulerOptions>) {
    return createAdaptiveScheduler({
      run: () => {
        runs.push(clock);
        clock += costMs;
      },
      ...overrides
    });
  }

  it("coalesces repeated schedules into a single trailing run", async () => {
    const scheduler = makeScheduler();
    scheduler.schedule();
    scheduler.schedule();
    scheduler.schedule();
    await vi.advanceTimersByTimeAsync(FRAME_MS + 8);
    expect(runs).toHaveLength(1);
  });

  it("runs the pending job again after a new schedule", async () => {
    const scheduler = makeScheduler();
    scheduler.schedule();
    await vi.advanceTimersByTimeAsync(FRAME_MS + 8);
    scheduler.schedule();
    await vi.advanceTimersByTimeAsync(FRAME_MS + 8);
    expect(runs).toHaveLength(2);
  });

  it("cancel drops the pending job", async () => {
    const scheduler = makeScheduler();
    scheduler.schedule();
    scheduler.cancel();
    await vi.advanceTimersByTimeAsync(1000);
    expect(runs).toHaveLength(0);
  });

  it("force runs synchronously and leaves nothing pending", () => {
    const scheduler = makeScheduler();
    scheduler.force();
    expect(runs).toHaveLength(1);
    vi.advanceTimersByTime(1000);
    expect(runs).toHaveLength(1);
  });

  it("delay follows the cost of the previous render, clamped to maxDelay", async () => {
    costMs = 500; // lastRenderMs=500 → raw 750 → clamped to maxDelay 120
    const scheduler = makeScheduler({ minDelay: 8, maxDelay: 120 });
    scheduler.schedule();
    await vi.advanceTimersByTimeAsync(FRAME_MS + 8 + 40);
    expect(runs).toHaveLength(1);

    costMs = 0;
    scheduler.schedule();
    await vi.advanceTimersByTimeAsync(FRAME_MS + 60);
    expect(runs).toHaveLength(1);
    await vi.advanceTimersByTimeAsync(100);
    expect(runs).toHaveLength(2);
  });

  it("scales the delay between min and max by 1.5x render time", async () => {
    const scheduler = makeScheduler({ minDelay: 8, maxDelay: 120 });
    costMs = 30; // → next delay 45
    scheduler.schedule();
    await vi.advanceTimersByTimeAsync(FRAME_MS + 8 + 40);
    expect(runs).toHaveLength(1);

    costMs = 0;
    scheduler.schedule();
    await vi.advanceTimersByTimeAsync(FRAME_MS + 20);
    expect(runs).toHaveLength(1);
    await vi.advanceTimersByTimeAsync(60);
    expect(runs).toHaveLength(2);
  });

  it("escalates to the burst delay while runs stay inside the burst window", async () => {
    const scheduler = makeScheduler({ burstDelay: 250, burstWindowMs: 400, burstCount: 3 });
    for (let index = 0; index < 3; index += 1) {
      scheduler.schedule();
      await vi.advanceTimersByTimeAsync(FRAME_MS + 8 + 40);
    }
    expect(runs).toHaveLength(3);

    scheduler.schedule();
    await vi.advanceTimersByTimeAsync(FRAME_MS + 200);
    expect(runs).toHaveLength(3);
    await vi.advanceTimersByTimeAsync(100);
    expect(runs).toHaveLength(4);
  });

  it("decays out of burst mode once the window passes", async () => {
    const scheduler = makeScheduler({ burstDelay: 250, burstWindowMs: 400, burstCount: 3 });
    for (let index = 0; index < 3; index += 1) {
      scheduler.schedule();
      await vi.advanceTimersByTimeAsync(FRAME_MS + 8 + 40);
    }
    clock += 500; // burst window elapsed
    scheduler.schedule();
    await vi.advanceTimersByTimeAsync(FRAME_MS + 8 + 40);
    expect(runs).toHaveLength(4);
  });
});
