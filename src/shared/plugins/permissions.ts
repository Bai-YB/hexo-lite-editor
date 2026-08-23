import type { PluginMethod } from "./protocol";

export function hasPermission(permissions: string[], method: PluginMethod, params: unknown): boolean {
  if (method === "article.getCurrent") return permissions.includes("article:read");
  if (method === "image.upload") return permissions.includes("image:read-selected");
  if (method === "image.validateConfig" || method === "image.testConnection") return true;
  if (method === "network.request") {
    const url = typeof params === "object" && params && "url" in params ? String((params as { url: unknown }).url) : "";
    try { const origin = new URL(url).origin; return permissions.includes(`network:${origin}`); } catch { return false; }
  }
  return false;
}
