import type { ContentSyncView } from "$shared/types/app";

export function syncStatusLabel(view: ContentSyncView) {
  switch (view.status) {
    case "checking": return "正在检查同步";
    case "synced": return "已同步";
    case "localPending": return "等待同步";
    case "remoteAhead": return "云端有更新";
    case "conflict": return `${view.conflicts.length} 个文件需处理`;
    case "authRequired": return "需登录";
    case "offline": return "离线";
    case "error": return "失败待重试";
    default: return "同步关闭";
  }
}
