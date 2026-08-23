import type { PluginRequest, PluginResponse } from "./protocol";

export interface WorkerLike {
  postMessage(message: unknown): void;
  terminate(): void;
  addEventListener(type: "message" | "error", listener: EventListener): void;
  removeEventListener(type: "message" | "error", listener: EventListener): void;
}

export class PluginWorkerRuntime {
  constructor(private worker: WorkerLike, private timeoutMs = 10_000) {}
  request(request: PluginRequest): Promise<unknown> {
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => { cleanup(); this.worker.terminate(); reject(new Error("Plugin request timed out")); }, this.timeoutMs);
      const onMessage = ((event: MessageEvent<PluginResponse>) => {
        if (!event.data || "kind" in event.data || event.data.id !== request.id) return;
        cleanup();
        event.data.error ? reject(Object.assign(new Error(event.data.error.message), { code: event.data.error.code })) : resolve(event.data.result);
      }) as EventListener;
      const onError = ((event: ErrorEvent) => { cleanup(); reject(event.error ?? new Error(event.message)); }) as EventListener;
      const cleanup = () => { clearTimeout(timer); this.worker.removeEventListener("message", onMessage); this.worker.removeEventListener("error", onError); };
      this.worker.addEventListener("message", onMessage); this.worker.addEventListener("error", onError); this.worker.postMessage(request);
    });
  }
  terminate(): void { this.worker.terminate(); }
}
