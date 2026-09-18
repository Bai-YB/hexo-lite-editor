import type { ContentSyncView } from "$shared/types/app";

export function syncStatusLabel(view: ContentSyncView) {
  switch (view.status) {
    case "checking": return "检查中";
    case "synced": return "已同步";
    case "localPending": return "待同步";
    case "remoteAhead": return "云端有更新";
    case "conflict": return `${view.conflicts.length} 个文件有冲突`;
    case "authRequired": return "需登录";
    case "offline": return "离线";
    case "error": return "同步出错";
    default: return "同步未开启";
  }
}
