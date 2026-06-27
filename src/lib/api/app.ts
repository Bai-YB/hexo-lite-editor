import { command } from "./tauri";
import type { UpdateCheckResult, UpdateSettings } from "$lib/types/settings";

export function getAppVersion(): Promise<string> {
  return command<string>("get_app_version");
}

export function openReleasePage(url: string): Promise<void> {
  return command<void>("open_release_page", { url });
}

export function openConfigDir(): Promise<void> {
  return command<void>("open_config_dir");
}

export function checkUpdate(settings: UpdateSettings): Promise<UpdateCheckResult> {
  return command<UpdateCheckResult>("check_update", { settings });
}
