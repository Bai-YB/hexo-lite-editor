export interface PluginManifest {
  id: string; name: string; version: string; apiVersion: "0.1"; entry: string;
  permissions: string[]; contributes: Record<string, unknown>;
}
export interface PluginView { manifest: PluginManifest; enabled: boolean; entryUrl?: string }
export interface PluginUploadInput { name: string; mime: string; bytes: number[]; config?: unknown }
export interface PluginUploadResult { url: string; markdown?: string }
export interface ImageBedProviderPlugin {
  id: string; name: string;
  validateConfig(config: unknown): Promise<{ ok: boolean; message?: string }>;
  testConnection(config: unknown): Promise<{ ok: boolean; message?: string }>;
  upload(input: PluginUploadInput): Promise<PluginUploadResult>;
}
