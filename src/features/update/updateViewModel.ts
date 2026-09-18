import type { UpdateSnapshot, UpdateStatus } from "$shared/types/app";

export function formatBytes(value?: number): string | null {
  if (value === undefined || !Number.isFinite(value) || value < 0) return null;
  const units = ["B", "KB", "MB", "GB"];
  let amount = value;
  let unit = 0;
  while (amount >= 1024 && unit < units.length - 1) { amount /= 1024; unit += 1; }
  return `${amount.toFixed(unit ? 1 : 0)} ${units[unit]}`;
}

const stageLabels: Record<UpdateStatus, string> = {
  idle: "尚未检查更新", checking: "正在检查更新", upToDate: "已是最新版本",
  available: "有新版本可下载", downloading: "正在下载", verifying: "正在验证更新包",
  downloaded: "更新已就绪", installing: "正在安装，马上重启", error: "更新未完成"
};

type UpdateTranslator = (key: string, values?: Record<string, string | number>) => string;
const defaultTranslate: UpdateTranslator = (key, values) => key.replace(/\{(\w+)\}/g, (_, name: string) => String(values?.[name] ?? `{${name}}`));

export function updateViewModel(snapshot: UpdateSnapshot, translate: UpdateTranslator = defaultTranslate) {
  const total = snapshot.totalBytes;
  const downloaded = snapshot.downloadedBytes;
  const hasTotal = total !== undefined && Number.isFinite(total) && total > 0;
  const hasDownloaded = downloaded !== undefined && Number.isFinite(downloaded) && downloaded >= 0;
  // A mismatching manifest size must not present a false percentage.
  const percent = hasTotal && hasDownloaded && downloaded <= total ? downloaded / total * 100 : null;
  const valueText = !hasDownloaded ? null : hasTotal && downloaded <= total
    ? `${formatBytes(downloaded)} / ${formatBytes(total)}` : translate("已下载 {p0}", { p0: formatBytes(downloaded) ?? "" });
  return {
    percent, valueText,
    stageLabel: snapshot.status === "error" ? ({ check: "检查失败", download: "下载失败", verify: "验证失败", install: "安装失败" }[snapshot.errorStage ?? "check"]) : stageLabels[snapshot.status],
    indeterminate: snapshot.status === "downloading" && percent === null || snapshot.status === "verifying",
    canCheck: !["checking", "downloading", "verifying", "downloaded", "installing"].includes(snapshot.status),
    canDownload: snapshot.status === "available" || snapshot.status === "error" && ["download", "verify"].includes(snapshot.errorStage ?? ""),
    canInstall: snapshot.status === "downloaded" || snapshot.status === "error" && snapshot.errorStage === "install",
    errorLabel: snapshot.errorStage ? `${snapshot.errorStage}: ${snapshot.errorMessage ?? ""}` : null
  };
}

export interface TransferSample { at: number; bytes: number }
export function sampleTransfer(samples: TransferSample[], snapshot: UpdateSnapshot, now: number): TransferSample[] {
  if (snapshot.status !== "downloading" || snapshot.downloadedBytes === undefined) return [];
  const bytes = snapshot.downloadedBytes;
  const previous = samples.at(-1);
  if (!previous || bytes < previous.bytes) return [{ at: now, bytes }];
  if (bytes === previous.bytes) return samples;
  return [...samples.filter((sample) => now - sample.at <= 5000), { at: now, bytes }];
}

export function transferViewModel(samples: TransferSample[], snapshot: UpdateSnapshot, now: number, translate: UpdateTranslator = defaultTranslate) {
  const first = samples[0];
  const last = samples.at(-1);
  if (snapshot.status !== "downloading" || !first || !last || last.at - first.at < 400 || now - last.at > 3000) return { speed: null, remaining: null };
  const speed = (last.bytes - first.bytes) / ((last.at - first.at) / 1000);
  if (!Number.isFinite(speed) || speed <= 0) return { speed: null, remaining: null };
  const seconds = snapshot.totalBytes && snapshot.totalBytes > last.bytes ? (snapshot.totalBytes - last.bytes) / speed : null;
  return {
    speed: `${formatBytes(speed)}/s`,
    remaining: seconds === null ? null : seconds < 60 ? translate("约 {p0} 秒", { p0: Math.max(1, Math.ceil(seconds)) }) : translate("约 {p0} 分钟", { p0: Math.ceil(seconds / 60) })
  };
}

export function releaseSummary(notes?: string, limit = 5): string[] {
  if (!notes) return [];
  const lines = notes.split(/\r?\n/).map((line) => line.trim());
  const bullets = lines.filter((line) => /^[-*+]\s+/.test(line));
  const source = bullets.length ? bullets : lines.filter((line) => line && !/^(#|```|\||>)/.test(line));
  return source.slice(0, limit).map((line) => line.replace(/^[-*+]\s+/, "").replace(/\[([^\]]+)\]\([^)]*\)/g, "$1").replace(/\*\*|`/g, ""));
}
