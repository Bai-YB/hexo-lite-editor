import { describe, expect, it } from "vitest";
import { contributedImageBedProviders, imageBedProviderId, pluginForProvider } from "./PluginProviderRuntime";
import type { PluginView } from "./types";

const plugin: PluginView = {
  manifest: {
    id: "com.example.imagebed",
    name: "Example",
    version: "1.0.0",
    apiVersion: "0.1",
    entry: "index.js",
    permissions: ["image:read-selected"],
    contributes: { imageBedProviders: ["example"] }
  },
  enabled: true,
  entryUrl: "hlex-plugin://com.example.imagebed/index.js"
};

describe("plugin image-bed providers", () => {
  it("exposes only enabled providers with a resolved worker entry", () => {
    expect(contributedImageBedProviders(plugin)).toEqual(["plugin:com.example.imagebed"]);
    expect(contributedImageBedProviders({ ...plugin, enabled: false })).toEqual([]);
    expect(contributedImageBedProviders({ ...plugin, entryUrl: undefined })).toEqual([]);
  });

  it("maps the selected provider back to its plugin", () => {
    const id = imageBedProviderId(plugin.manifest.id);
    expect(pluginForProvider([plugin], id)).toBe(plugin);
    expect(pluginForProvider([plugin], "local")).toBeUndefined();
  });
});
