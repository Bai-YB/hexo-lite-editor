export type PluginMethod = "image.upload" | "image.validateConfig" | "image.testConnection" | "network.request" | "article.getCurrent";
export type PluginRequest = { id: string; method: PluginMethod; params: unknown };
export type PluginHostRequest = { kind: "hostRequest"; id: string; method: "network.request" | "article.getCurrent"; params: unknown };
export type PluginResponse = { id: string; result?: unknown; error?: { code: string; message: string } };
