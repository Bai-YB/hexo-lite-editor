import { openUrl } from "@tauri-apps/plugin-opener";
import { command } from "./tauri";
import type { CommandResult } from "$lib/types/command";

export function startHexoServer(projectPath: string): Promise<string> {
  return command<string>("run_hexo_server", { projectPath });
}

export function stopHexoServer(): Promise<string> {
  return command<string>("stop_hexo_server");
}

export function generateSite(projectPath: string): Promise<CommandResult> {
  return command<CommandResult>("run_hexo_generate", { projectPath });
}

export function deploySite(projectPath: string): Promise<CommandResult> {
  return command<CommandResult>("run_hexo_deploy", { projectPath });
}

export function generateAndDeploy(projectPath: string): Promise<CommandResult> {
  return command<CommandResult>("run_hexo_generate_deploy", { projectPath });
}

export function getGitStatus(projectPath: string): Promise<CommandResult> {
  return command<CommandResult>("git_status", { projectPath });
}

export function openHexoPreview(): Promise<void> {
  return openUrl("http://localhost:4000");
}
