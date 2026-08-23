import { describe, expect, it } from "vitest"; import { updateViewModel } from "./updateViewModel"; import type { UpdateSnapshot, UpdateStatus } from "$shared/types/app";
const snapshot = (status: UpdateStatus): UpdateSnapshot => ({ currentVersion: "1.0.6", status });
describe("updateViewModel", () => {
  for (const status of ["idle", "checking", "upToDate", "available", "downloaded", "installing"] as UpdateStatus[]) it(`models ${status}`, () => expect(updateViewModel(snapshot(status))).toBeTruthy());
  it("calculates progress", () => expect(updateViewModel({ ...snapshot("downloading"), downloadedBytes: 25, totalBytes: 100 }).percent).toBe(25));
  it("uses indeterminate progress without total size", () => expect(updateViewModel({ ...snapshot("downloading"), downloadedBytes: 25 }).indeterminate).toBe(true));
  for (const errorStage of ["check", "download", "install"] as const) it(`keeps ${errorStage} errors`, () => expect(updateViewModel({ ...snapshot("error"), errorStage, errorMessage: "failed" }).errorLabel).toContain(errorStage));
});
