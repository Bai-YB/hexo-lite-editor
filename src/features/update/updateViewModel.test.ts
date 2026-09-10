import { describe, expect, it } from "vitest"; import { updateViewModel } from "./updateViewModel"; import type { UpdateSnapshot, UpdateStatus } from "$shared/types/app";
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
});
