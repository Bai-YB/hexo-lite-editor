import { describe, expect, it } from "vitest";
import { resolveReleaseVersion } from "./release-version.mjs";

describe("release version mapping", () => {
  it("keeps a four-part public version and valid updater SemVer", () => {
    expect(resolveReleaseVersion({ version: "1.0.6+1", releaseVersion: "1.0.6.1" })).toEqual({
      version: "1.0.6.1", runtimeVersion: "1.0.6+1", tag: "v1.0.6.1"
    });
  });
  it("rejects a mismatched package and release instead of building misleading assets", () => {
    expect(() => resolveReleaseVersion({ version: "1.0.6", releaseVersion: "1.0.6.1" })).toThrow();
    expect(() => resolveReleaseVersion({ version: "1.0.6.1", releaseVersion: "1.0.6.1" })).toThrow();
  });
});
