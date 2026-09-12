import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, describe, expect, it } from "vitest";

const directories = [];
const commit = "0123456789abcdef0123456789abcdef01234567";
const digest = (value) => createHash("sha256").update(value).digest("hex");

async function fixture({ mismatch = "" } = {}) {
  const directory = await mkdtemp(join(tmpdir(), "hlex-windows-release-"));
  directories.push(directory);
  const installer = join(directory, "setup.exe");
  const signature = join(directory, "setup.exe.sig");
  await writeFile(installer, "installer");
  await writeFile(signature, "signature");
  const manifest = join(directory, "release-manifest.json");
  await writeFile(manifest, JSON.stringify({
    version: mismatch === "version" ? "1.0.6" : "1.0.6.1",
    runtimeVersion: mismatch === "runtimeVersion" ? "1.0.6" : "1.0.6+1",
    platform: "windows",
    architecture: "x64",
    sourceCommit: mismatch === "commit" ? "deadbeef" : commit,
    assets: [
      { name: "setup.exe", size: 9, sha256: digest("installer") },
      { name: "setup.exe.sig", size: 9, sha256: digest("signature") }
    ]
  }));
  return { directory, manifest, installer, signature };
}

function verify(fixtureData) {
  return spawnSync(process.execPath, ["scripts/verify-windows-release.mjs", "--manifest", fixtureData.manifest, "--version", "1.0.6.1", "--runtime-version", "1.0.6+1", "--source-commit", commit, "--installer", fixtureData.installer, "--signature", fixtureData.signature], { encoding: "utf8" });
}

afterEach(async () => {
  await Promise.all(directories.splice(0).map((directory) => rm(directory, { recursive: true, force: true })));
});

describe("Windows release pairing verification", () => {
  it("accepts matching manifest and signed updater assets", async () => {
    expect(verify(await fixture()).status).toBe(0);
  });
  it.each(["version", "runtimeVersion", "commit"])("rejects a stale %s manifest", async (mismatch) => {
    expect(verify(await fixture({ mismatch })).status).toBe(1);
  });
  it("rejects a changed installer", async () => {
    const data = await fixture();
    await writeFile(data.installer, "tampered");
    expect(verify(data).status).toBe(1);
  });
  it("rejects a stale signature with the same size", async () => {
    const data = await fixture();
    await writeFile(data.signature, "different");
    const result = verify(data);
    expect(result.status).toBe(1);
    expect(result.stderr).toContain("SHA-256 mismatch for setup.exe.sig");
  });
  it("rejects an installer that has not finished downloading", async () => {
    const data = await fixture();
    await rm(data.installer);
    expect(verify(data).status).toBe(1);
  });
});
