import { afterEach, describe, expect, it, vi } from "vitest";
import { PluginWorkerRuntime } from "./PluginWorkerRuntime";
import { disposePluginWorkers, reconcilePluginWorkers, validatePluginConfig } from "./PluginProviderRuntime";
import type { PluginView } from "./types";

class FakeWorker extends EventTarget {
  static instances: FakeWorker[] = [];
  terminated = false;
  requests: unknown[] = [];
  constructor() { super(); FakeWorker.instances.push(this); }
  postMessage(request: unknown) { this.requests.push(request); }
  terminate() { this.terminated = true; }
  respond(id: string) { this.dispatchEvent(new MessageEvent("message", { data: { id, result: { ok: true } } })); }
}
const plugin: PluginView = { enabled: true, entryUrl: "/worker.js", manifest: { id: "test", name: "Test", version: "1", apiVersion: "0.1", entry: "index.js", permissions: [], contributes: {} } };
afterEach(() => { disposePluginWorkers(); vi.useRealTimers(); vi.unstubAllGlobals(); FakeWorker.instances = []; });

describe("plugin worker recovery", () => {
  it("rejects all pending requests on timeout and creates a worker for the next attempt", async () => {
    vi.useFakeTimers();
    vi.stubGlobal("Worker", FakeWorker);
    const first = validatePluginConfig(plugin, {}).catch((error) => error);
    const concurrent = validatePluginConfig(plugin, {}).catch((error) => error);
    await vi.advanceTimersByTimeAsync(30_000);
    expect((await first).message).toContain("超时");
    expect((await concurrent).message).toContain("超时");
    expect(FakeWorker.instances[0].terminated).toBe(true);
    const retry = validatePluginConfig(plugin, {});
    expect(FakeWorker.instances).toHaveLength(2);
    const worker = FakeWorker.instances[1];
    worker.respond((worker.requests[0] as { id: string }).id);
    await expect(retry).resolves.toEqual({ ok: true });
  });
  it("terminates pending work when the plugin is disabled", async () => {
    vi.stubGlobal("Worker", FakeWorker);
    const pending = validatePluginConfig(plugin, {}).catch((error) => error);
    reconcilePluginWorkers([{ ...plugin, enabled: false }]);
    expect((await pending).message).toContain("停止");
    expect(FakeWorker.instances[0].terminated).toBe(true);
    await expect(validatePluginConfig({ ...plugin, enabled: false }, {})).rejects.toThrow("启用");
  });
  it("rejects concurrent work immediately when a worker crashes", async () => {
    const worker = new FakeWorker();
    const runtime = new PluginWorkerRuntime(worker);
    const one = runtime.request({ id: "1", method: "image.validateConfig", params: {} }).catch((error) => error);
    const two = runtime.request({ id: "2", method: "image.validateConfig", params: {} }).catch((error) => error);
    worker.dispatchEvent(new ErrorEvent("error", { message: "crashed" }));
    expect((await one as Error).message).toBe("crashed");
    expect((await two as Error).message).toBe("crashed");
    expect(runtime.active).toBe(false);
  });
});
