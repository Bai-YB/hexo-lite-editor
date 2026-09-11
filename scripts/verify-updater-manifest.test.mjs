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

function verify(file, platforms, tag, version = "1.2.3") {
  return spawnSync(
    process.execPath,
    ["scripts/verify-updater-manifest.mjs", "--manifest", file, "--version", version, ...platforms.flatMap((value) => ["--platform", value]), ...(tag ? ["--tag", tag] : [])],
    { cwd: process.cwd(), encoding: "utf8", stdio: "pipe" }
  );
}

afterEach(async () => {
  await Promise.all(directories.splice(0).map((directory) => rm(directory, { recursive: true, force: true })));
});

describe("updater manifest verification", () => {
  it("accepts the four-part public release using its compatible internal version", async () => {
    const file = await manifestFile({ version: "1.0.6+1", notes: "1.0.6.1", pub_date: "2026-09-11T00:00:00Z", platforms: {
      "windows-x86_64": { url: "https://github.com/example/app/releases/download/v1.0.6.1/setup.exe", signature: "signed" }
    } });
    expect(verify(file, ["windows-x86_64"], "v1.0.6.1", "1.0.6+1").status).toBe(0);
    expect(verify(file, ["windows-x86_64"], "v1.0.6.1", "1.0.6.1").status).toBe(1);
  });
  it("rejects a repair release pointing at the original build", async () => {
    const manifest = { version: "1.2.3", notes: "Repair", pub_date: "2026-09-10T00:00:00Z", platforms: {
      "windows-x86_64": { url: "https://github.com/example/app/releases/download/v1.2.3/setup.exe", signature: "signature" }
    } };
    expect(verify(await manifestFile(manifest), ["windows-x86_64"], "v1.2.3-r1").status).toBe(1);
    manifest.platforms["windows-x86_64"].url = "https://github.com/example/app/releases/download/v1.2.3-r1/setup.exe";
    expect(verify(await manifestFile(manifest), ["windows-x86_64"], "v1.2.3-r1").status).toBe(0);
  });
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
