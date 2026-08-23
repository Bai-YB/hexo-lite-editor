import { hasPermission } from "./permissions";
import type { PluginRequest } from "./protocol";
import type { PluginManifest } from "./types";

export type PluginHandlers = Partial<Record<PluginRequest["method"], (params: unknown) => Promise<unknown>>>;

export interface NetworkRequestParams {
  url: string;
  method?: "GET" | "POST" | "PUT" | "PATCH" | "DELETE";
  headers?: Record<string, string>;
  body?: string;
  timeoutMs?: number;
}

export function createNetworkRequestHandler(maxResponseBytes = 5 * 1024 * 1024) {
  return async (params: unknown): Promise<{ status: number; headers: Record<string, string>; body: string }> => {
    const request = params as NetworkRequestParams;
    const initial = new URL(request.url);
    if (initial.protocol !== "https:") throw Object.assign(new Error("HTTPS is required"), { code: "plugin_network_invalid_url" });
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), Math.min(Math.max(request.timeoutMs ?? 10_000, 100), 30_000));
    try {
      const response = await fetch(initial, {
        method: request.method ?? "GET",
        headers: request.headers,
        body: request.body,
        redirect: "manual",
        signal: controller.signal
      });
      if (response.status >= 300 && response.status < 400) {
        throw Object.assign(new Error("Plugin network redirects are not followed"), { code: "plugin_network_redirect_denied" });
      }
      const declaredLength = Number(response.headers.get("content-length") ?? "0");
      if (declaredLength > maxResponseBytes) throw Object.assign(new Error("Plugin response is too large"), { code: "plugin_network_response_too_large" });
      const bytes = new Uint8Array(await response.arrayBuffer());
      if (bytes.byteLength > maxResponseBytes) throw Object.assign(new Error("Plugin response is too large"), { code: "plugin_network_response_too_large" });
      return {
        status: response.status,
        headers: Object.fromEntries(response.headers.entries()),
        body: new TextDecoder().decode(bytes)
      };
    } finally {
      clearTimeout(timer);
    }
  };
}

export class PluginHost {
  constructor(private manifest: PluginManifest, private handlers: PluginHandlers) {}
  async handle(request: PluginRequest): Promise<unknown> {
    if (!hasPermission(this.manifest.permissions, request.method, request.params)) {
      throw Object.assign(new Error("Plugin permission denied"), { code: "plugin_permission_denied" });
    }
    const handler = this.handlers[request.method];
    if (!handler) throw new Error(`Unsupported plugin method: ${request.method}`);
    return handler(request.params);
  }
}
