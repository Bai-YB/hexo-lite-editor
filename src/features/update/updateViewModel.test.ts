import { describe, expect, it } from "vitest"; import { releaseSummary, sampleTransfer, transferViewModel, updateViewModel } from "./updateViewModel"; import { shouldAutoDownload } from "./updateStore"; import { defaultConfig, type UpdateSnapshot, type UpdateStatus } from "$shared/types/app";
const snapshot = (status: UpdateStatus): UpdateSnapshot => ({ currentVersion: "1.0.6", status });
describe("updateViewModel", () => {
  for (const status of ["idle", "checking", "upToDate", "available", "downloaded", "installing"] as UpdateStatus[]) it(`models ${status}`, () => expect(updateViewModel(snapshot(status))).toBeTruthy());
  it("calculates progress", () => expect(updateViewModel({ ...snapshot("downloading"), downloadedBytes: 25, totalBytes: 100 }).percent).toBe(25));
  it("uses indeterminate progress without total size", () => expect(updateViewModel({ ...snapshot("downloading"), downloadedBytes: 25 }).indeterminate).toBe(true));
  for (const errorStage of ["check", "download", "install"] as const) it(`keeps ${errorStage} errors`, () => expect(updateViewModel({ ...snapshot("error"), errorStage, errorMessage: "failed" }).errorLabel).toContain(errorStage));
  it("retries installation after an install failure without a new download", () => {
    const model = updateViewModel({ ...snapshot("error"), errorStage: "install", errorMessage: "locked" });
    expect(model.canInstall).toBe(true);
    expect(model.canDownload).toBe(false);
  });
  it("allows a fresh check after a withdrawn update resets to upToDate", () => {
    expect(updateViewModel(snapshot("upToDate")).canCheck).toBe(true);
    expect(updateViewModel(snapshot("upToDate")).indeterminate).toBe(false);
  });
  it("does not invent a percentage from invalid or inconsistent sizes", () => {
    for (const totalBytes of [0, -1, Number.NaN, 20]) {
      const model = updateViewModel({ ...snapshot("downloading"), downloadedBytes: 25, totalBytes });
      expect(model.percent).toBeNull();
      expect(model.indeterminate).toBe(true);
    }
  });
  it("keeps verification distinct from ready to install", () => {
    const model = updateViewModel(snapshot("verifying"));
    expect(model.canCheck).toBe(false);
    expect(model.canInstall).toBe(false);
    expect(model.stageLabel).toBe("正在验证更新包");
    expect(updateViewModel({ ...snapshot("error"), errorStage: "verify" }).canDownload).toBe(true);
  });
  it("uses measured transfer speed, resets on retry, and hides stale speed", () => {
    const initial = { ...snapshot("downloading"), downloadedBytes: 0, totalBytes: 2048 };
    let samples = sampleTransfer([], initial, 1000);
    samples = sampleTransfer(samples, { ...initial, downloadedBytes: 1024 }, 2000);
    expect(transferViewModel(samples, initial, 2000)).toEqual({ speed: "1.0 KB/s", remaining: "约 1 秒" });
    expect(transferViewModel(samples, initial, 6000).speed).toBeNull();
    expect(sampleTransfer(samples, initial, 6000)).toEqual([{ at: 6000, bytes: 0 }]);
    expect(sampleTransfer(samples, snapshot("downloaded"), 6000)).toEqual([]);
  });
  it("shows only five release entries until expanded", () => {
    const notes = "# Release\n" + Array.from({ length: 8 }, (_, index) => `- **Fix ${index + 1}**`).join("\n");
    expect(releaseSummary(notes)).toEqual(["Fix 1", "Fix 2", "Fix 3", "Fix 4", "Fix 5"]);
  });
  it("requires the explicit auto-download setting", () => {
    expect(defaultConfig.update.autoDownload).toBe(false);
    expect(shouldAutoDownload(snapshot("available"), false)).toBe(false);
    expect(shouldAutoDownload(snapshot("available"), true)).toBe(true);
    expect(shouldAutoDownload(snapshot("downloaded"), true)).toBe(false);
  });
});
