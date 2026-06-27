import { command } from "./tauri";

export interface HexoConfigFile {
  exists: boolean;
  project_path: string;
  config_path: string;
  content: string;
  latest_backup_path?: string;
}

export interface BackupResult {
  backup_path: string;
}

export function readHexoConfig(projectPath: string): Promise<HexoConfigFile> {
  return command<HexoConfigFile>("read_hexo_config", { projectPath });
}

export function saveHexoConfig(projectPath: string, content: string): Promise<BackupResult> {
  return command<BackupResult>("save_hexo_config", { projectPath, content });
}

export function backupHexoConfig(projectPath: string): Promise<BackupResult> {
  return command<BackupResult>("backup_hexo_config", { projectPath });
}

export function restoreLatestHexoConfigBackup(projectPath: string): Promise<HexoConfigFile> {
  return command<HexoConfigFile>("restore_latest_hexo_config_backup", { projectPath });
}

export function openHexoConfigExternal(projectPath: string): Promise<void> {
  return command<void>("open_hexo_config_external", { projectPath });
}
