import { invoke } from "@tauri-apps/api/core";

export function command<T>(name: string, args?: Record<string, unknown>): Promise<T> {
  return invoke<T>(name, args);
}
