import type { PluginRequest, PluginResponse } from "./protocol";

export interface WorkerLike {
  postMessage(message: unknown): void;
  terminate(): void;
  addEventListener(type: "message" | "error", listener: EventListener): void;
  removeEventListener(type: "message" | "error", listener: EventListener): void;
}

export class PluginWorkerRuntime {
  private pending = new Map<string, { reject: (error: Error) => void; cleanup: () => void }>();
  private stopped = false;
  constructor(private worker: WorkerLike, private timeoutMs = 30_000, private onStopped: () => void = () => {}) {}
  get active() { return !this.stopped; }
  request(request: PluginRequest): Promise<unknown> {
    if (this.stopped) return Promise.reject(new Error("插件已停止，请重试。"));
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => this.terminate(new Error("插件请求超时，请重试。")), this.timeoutMs);
      const onMessage = ((event: MessageEvent<PluginResponse>) => {
        if (!event.data || "kind" in event.data || event.data.id !== request.id) return;
        cleanup();
        event.data.error ? reject(Object.assign(new Error(event.data.error.message), { code: event.data.error.code })) : resolve(event.data.result);
      }) as EventListener;
      const onError = ((event: ErrorEvent) => this.terminate(event.error ?? new Error(event.message || "插件运行失败，请重试。"))) as EventListener;
      const cleanup = () => { clearTimeout(timer); this.pending.delete(request.id); this.worker.removeEventListener("message", onMessage); this.worker.removeEventListener("error", onError); };
      this.pending.set(request.id, { reject, cleanup });
      this.worker.addEventListener("message", onMessage); this.worker.addEventListener("error", onError);
      try { this.worker.postMessage(request); } catch (error) { this.terminate(error instanceof Error ? error : new Error(String(error))); }
    });
  }
  terminate(reason = new Error("插件已停止。")): void {
    if (this.stopped) return;
    this.stopped = true;
    this.worker.terminate();
    for (const pending of this.pending.values()) { pending.cleanup(); pending.reject(reason); }
    this.onStopped();
  }
}
