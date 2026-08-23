import { describe, expect, it, vi } from "vitest";
import { createNetworkRequestHandler, PluginHost } from "./PluginHost";
import type { PluginManifest } from "./types";

const manifest = (permissions: string[]): PluginManifest => ({ id: "com.example", name: "Example", version: "1.0.0", apiVersion: "0.1", entry: "index.js", permissions, contributes: {} });

describe("PluginHost", () => {
  it("rejects network origins that were not declared", async () => {
    const handler = vi.fn(); const host = new PluginHost(manifest(["network:https://allowed.example"]), { "network.request": handler });
    await expect(host.handle({ id: "1", method: "network.request", params: { url: "https://denied.example/a" } })).rejects.toMatchObject({ code: "plugin_permission_denied" });
    expect(handler).not.toHaveBeenCalled();
  });
  it("allows an exact declared origin", async () => {
    const host = new PluginHost(manifest(["network:https://allowed.example"]), { "network.request": async () => "ok" });
    await expect(host.handle({ id: "1", method: "network.request", params: { url: "https://allowed.example/a" } })).resolves.toBe("ok");
  });
  it("rejects non-HTTPS requests and redirects in the built-in network handler", async () => {
    const handler = createNetworkRequestHandler();
    await expect(handler({ url: "http://allowed.example/a" })).rejects.toMatchObject({ code: "plugin_network_invalid_url" });
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response(null, { status: 302, headers: { location: "https://other.example" } })));
    await expect(handler({ url: "https://allowed.example/a" })).rejects.toMatchObject({ code: "plugin_network_redirect_denied" });
    vi.unstubAllGlobals();
  });
});
