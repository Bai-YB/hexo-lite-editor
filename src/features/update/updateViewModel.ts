import type { UpdateSnapshot } from "$shared/types/app";

export function formatBytes(value?: number): string | null {
  if (value === undefined) return null;
  const units = ["B", "KB", "MB", "GB"]; let amount = value; let unit = 0;
  while (amount >= 1024 && unit < units.length - 1) { amount /= 1024; unit += 1; }
  return `${amount.toFixed(unit ? 1 : 0)} ${units[unit]}`;
}
export function updateViewModel(snapshot: UpdateSnapshot) {
  const percent = snapshot.totalBytes && snapshot.downloadedBytes !== undefined ? Math.min(100, snapshot.downloadedBytes / snapshot.totalBytes * 100) : null;
  const valueText = snapshot.downloadedBytes === undefined ? null : snapshot.totalBytes ? `${formatBytes(snapshot.downloadedBytes)} / ${formatBytes(snapshot.totalBytes)}` : `已下载 ${formatBytes(snapshot.downloadedBytes)}`;
  return { percent, valueText, indeterminate: snapshot.status === "downloading" && percent === null, canCheck: !["checking", "downloading", "installing"].includes(snapshot.status), canDownload: snapshot.status === "available" || snapshot.status === "error" && snapshot.errorStage === "download", canInstall: snapshot.status === "downloaded", errorLabel: snapshot.errorStage ? `${snapshot.errorStage}: ${snapshot.errorMessage ?? ""}` : null };
}
