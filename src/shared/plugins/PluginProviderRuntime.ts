import { platform } from "$platform/tauri";
import { PluginHost } from "./PluginHost";
import { PluginWorkerRuntime, type WorkerLike } from "./PluginWorkerRuntime";
import type { PluginHostRequest, PluginRequest, PluginResponse } from "./protocol";
import type { PluginManifest, PluginUploadInput, PluginUploadResult, PluginView } from "./types";

const workers = new Map<string, HostedPluginWorker>();
let sequence = 0;

export function imageBedProviderId(pluginId: string): string {
  return `plugin:${pluginId}`;
}

export function contributedImageBedProviders(plugin: PluginView): string[] {
  if (!plugin.enabled || !plugin.entryUrl) return [];
  const value = plugin.manifest.contributes.imageBedProviders;
  return Array.isArray(value) && value.some((item) => typeof item === "string")
    ? [imageBedProviderId(plugin.manifest.id)]
    : [];
}

export function pluginForProvider(plugins: PluginView[], provider: string): PluginView | undefined {
  if (!provider.startsWith("plugin:")) return undefined;
  return plugins.find((plugin) => plugin.manifest.id === provider.slice(7) && contributedImageBedProviders(plugin).length);
}

export async function uploadWithPlugin(plugin: PluginView, input: PluginUploadInput): Promise<PluginUploadResult> {
  const settings = await platform.getPluginSettings(plugin.manifest.id);
  const result = await workerFor(plugin).runtime.request({ id: requestId(), method: "image.upload", params: { ...input, config: settings } });
  if (!result || typeof result !== "object" || typeof (result as PluginUploadResult).url !== "string") {
    throw Object.assign(new Error("Plugin returned an invalid upload result"), { code: "plugin_result_invalid" });
  }
  return result as PluginUploadResult;
}

export async function validatePluginConfig(plugin: PluginView, config: unknown): Promise<{ ok: boolean; message?: string }> {
  return workerFor(plugin).runtime.request({ id: requestId(), method: "image.validateConfig", params: config }) as Promise<{ ok: boolean; message?: string }>;
}

export async function testPluginConnection(plugin: PluginView, config: unknown): Promise<{ ok: boolean; message?: string }> {
  return workerFor(plugin).runtime.request({ id: requestId(), method: "image.testConnection", params: config }) as Promise<{ ok: boolean; message?: string }>;
}

export function disposePluginWorkers(): void {
  for (const hosted of workers.values()) hosted.runtime.terminate();
  workers.clear();
}

function workerFor(plugin: PluginView): HostedPluginWorker {
  const existing = workers.get(plugin.manifest.id);
  if (existing) return existing;
  if (!plugin.entryUrl) throw new Error("Plugin entry is unavailable");
  const hosted = new HostedPluginWorker(plugin.manifest, plugin.entryUrl);
  workers.set(plugin.manifest.id, hosted);
  return hosted;
}

function requestId(): string {
  sequence += 1;
  return `plugin-${Date.now()}-${sequence}`;
}

class HostedPluginWorker {
  readonly runtime: PluginWorkerRuntime;
  private readonly worker: Worker;
  private readonly host: PluginHost;

  constructor(manifest: PluginManifest, entryUrl: string) {
    this.worker = new Worker(entryUrl, { type: "module", name: manifest.id });
    this.host = new PluginHost(manifest, {
      "network.request": async (params) => {
        const request = params as { url: string; method?: string; headers?: Record<string, string>; body?: number[] };
        return platform.pluginHttpRequest({ pluginId: manifest.id, ...request });
      },
      "article.getCurrent": async () => { throw new Error("Current article access is not available in image uploads"); }
    });
    this.worker.addEventListener("message", ((event: MessageEvent<PluginHostRequest>) => {
      if (event.data?.kind !== "hostRequest") return;
      void this.replyToHostRequest(event.data);
    }) as EventListener);
    this.runtime = new PluginWorkerRuntime(this.worker as unknown as WorkerLike);
  }

  private async replyToHostRequest(request: PluginHostRequest): Promise<void> {
    let response: PluginResponse;
    try {
      const result = await this.host.handle(request as PluginRequest);
      response = { id: request.id, result };
    } catch (error) {
      const candidate = error as { code?: string; message?: string };
      response = { id: request.id, error: { code: candidate.code ?? "plugin_host_failed", message: candidate.message ?? String(error) } };
    }
    this.worker.postMessage(response);
  }
}
