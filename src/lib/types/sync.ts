export type ContentSyncState =
  | "notConfigured"
  | "ready"
  | "syncing"
  | "hasLocalChanges"
  | "needsPull"
  | "needsPush"
  | "conflict"
  | "error";

export interface ContentSyncStatus {
  status: ContentSyncState;
  message: string;
  worktreePath: string;
  remoteName: string;
  branchName: string;
  ahead: number;
  behind: number;
  hasLocalChanges: boolean;
  conflicts: string[];
  lastSyncAt?: string;
}
