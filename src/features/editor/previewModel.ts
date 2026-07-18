import type { PreviewServerState } from "$shared/types/app";

export function previewStateLabel(state?: PreviewServerState) {
  if (state === "running") return "运行中";
  if (state === "starting") return "启动中";
  if (state === "stopping") return "停止中";
  if (state === "error") return "异常";
  return "已停止";
}

export function isSafeThemePreviewUrl(value: string) {
  try {
    const url = new URL(value);
    return url.protocol === "http:" && url.hostname === "127.0.0.1";
  } catch {
    return false;
  }
}
