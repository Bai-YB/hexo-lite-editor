import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { resolveReleaseVersion } from "./release-version.mjs";

describe("release version mapping", () => {
  it("keeps a five-part public version and valid updater SemVer", () => {
    expect(resolveReleaseVersion({ version: "1.0.6+5.1", releaseVersion: "1.0.6.5.1" })).toEqual({
      version: "1.0.6.5.1", runtimeVersion: "1.0.6+5.1", tag: "v1.0.6.5.1"
    });
  });
  it("rejects a mismatched package and release instead of building misleading assets", () => {
    expect(() => resolveReleaseVersion({ version: "1.0.6", releaseVersion: "1.0.6.2" })).toThrow();
    expect(() => resolveReleaseVersion({ version: "1.0.6.5.1", releaseVersion: "1.0.6.5.1" })).toThrow();
  });
});

describe("release documentation gate", () => {
  it("keeps module documentation and release archives aligned with package.json", () => {
    const packageInfo = JSON.parse(readFileSync(resolve(process.cwd(), "package.json"), "utf8"));
    const { version } = resolveReleaseVersion(packageInfo);
    const moduleIndex = readFileSync(resolve(process.cwd(), "docs/modules/README.md"), "utf8");
    expect(moduleIndex).toContain(`文档基线版本：${version}`);
    for (const relative of [
      `docs/archive/${version}.md`,
      `docs/validation-${version}.md`,
      `.github/release-notes/v${version}.md`,
    ]) {
      const content = readFileSync(resolve(process.cwd(), relative), "utf8");
      expect(content).toContain(version);
    }
  });
});
