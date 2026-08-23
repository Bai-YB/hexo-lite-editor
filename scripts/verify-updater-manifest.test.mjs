import { spawnSync } from "node:child_process";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, describe, expect, it } from "vitest";

const directories = [];

async function manifestFile(manifest) {
  const directory = await mkdtemp(join(tmpdir(), "hlex-updater-"));
  directories.push(directory);
  const file = join(directory, "latest.json");
  await writeFile(file, `${JSON.stringify(manifest)}\n`, "utf8");
  return file;
}

function verify(file, platforms) {
  return spawnSync(
    process.execPath,
    ["scripts/verify-updater-manifest.mjs", "--manifest", file, "--version", "1.2.3", ...platforms.flatMap((value) => ["--platform", value])],
    { cwd: process.cwd(), encoding: "utf8", stdio: "pipe" }
  );
}

afterEach(async () => {
  await Promise.all(directories.splice(0).map((directory) => rm(directory, { recursive: true, force: true })));
});

describe("updater manifest verification", () => {
  it("accepts a complete, version-matched signed manifest", async () => {
    const file = await manifestFile({
      version: "1.2.3",
      notes: "Release notes",
      pub_date: "2026-08-23T00:00:00Z",
      platforms: {
        "windows-x86_64": { url: "https://example.com/windows.zip", signature: "windows-signature" },
        "darwin-x86_64": { url: "https://example.com/macos-x64.tar.gz", signature: "macos-x64-signature" },
        "darwin-aarch64": { url: "https://example.com/macos-arm64.tar.gz", signature: "macos-arm64-signature" }
      }
    });
    const result = verify(file, ["windows-x86_64", "darwin-x86_64", "darwin-aarch64"]);
    expect(result.status).toBe(0);
    expect(result.stdout).toContain("Verified");
  });

  it("rejects a manifest without the requested platform", async () => {
    const file = await manifestFile({
      version: "1.2.3",
      notes: "Release notes",
      pub_date: "2026-08-23T00:00:00Z",
      platforms: { "windows-x86_64": { url: "https://example.com/windows.zip", signature: "windows-signature" } }
    });
    const result = verify(file, ["darwin-aarch64"]);
    expect(result.status).toBe(1);
    expect(result.stderr).toContain("Manifest is missing platform darwin-aarch64");
  });
});
