import { command } from "./tauri";
import type { SyncSettings } from "$lib/types/settings";
import type { ContentSyncStatus } from "$lib/types/sync";

interface RawContentSyncStatus {
  status: ContentSyncStatus["status"];
  message: string;
  worktree_path: string;
  remote_name: string;
  branch_name: string;
  ahead: number;
  behind: number;
  has_local_changes: boolean;
  conflicts: string[];
  last_sync_at?: string;
}

export async function getContentSyncStatus(projectPath: string, settings: SyncSettings): Promise<ContentSyncStatus> {
  return mapStatus(await command<RawContentSyncStatus>("get_content_sync_status", { projectPath, settings }));
}

export async function initContentSync(projectPath: string, settings: SyncSettings): Promise<ContentSyncStatus> {
  return mapStatus(await command<RawContentSyncStatus>("init_content_sync", { projectPath, settings }));
}

export async function pullContentSync(projectPath: string, settings: SyncSettings): Promise<ContentSyncStatus> {
  return mapStatus(await command<RawContentSyncStatus>("pull_content_sync", { projectPath, settings }));
}

export async function pushContentSync(projectPath: string, settings: SyncSettings): Promise<ContentSyncStatus> {
  return mapStatus(await command<RawContentSyncStatus>("push_content_sync", { projectPath, settings }));
}

export async function runContentSync(projectPath: string, settings: SyncSettings): Promise<ContentSyncStatus> {
  return mapStatus(await command<RawContentSyncStatus>("run_content_sync", { projectPath, settings }));
}

function mapStatus(status: RawContentSyncStatus): ContentSyncStatus {
  return {
    status: status.status,
    message: status.message,
    worktreePath: status.worktree_path,
    remoteName: status.remote_name,
    branchName: status.branch_name,
    ahead: status.ahead,
    behind: status.behind,
    hasLocalChanges: status.has_local_changes,
    conflicts: status.conflicts,
    lastSyncAt: status.last_sync_at
  };
}
