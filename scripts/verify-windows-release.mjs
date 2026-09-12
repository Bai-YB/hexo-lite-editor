import { createHash } from "node:crypto";
import { readFile, stat } from "node:fs/promises";

const values = new Map();
for (let index = 2; index < process.argv.length; index += 2) {
  const key = process.argv[index];
  const value = process.argv[index + 1];
  if (!key || !value) throw new Error(`Missing value for ${key ?? "argument"}`);
  values.set(key, value);
}

const required = (name) => {
  const value = values.get(name);
  if (!value) throw new Error(`Missing ${name}`);
  return value;
};

const manifestPath = required("--manifest");
const expectedVersion = required("--version");
const expectedRuntimeVersion = required("--runtime-version");
const expectedSourceCommit = required("--source-commit");
const manifest = JSON.parse(await readFile(manifestPath, "utf8"));

if (manifest.version !== expectedVersion) throw new Error(`Windows manifest version ${manifest.version ?? "<missing>"} does not match ${expectedVersion}`);
if (manifest.runtimeVersion !== expectedRuntimeVersion) throw new Error(`Windows manifest runtime version ${manifest.runtimeVersion ?? "<missing>"} does not match ${expectedRuntimeVersion}`);
if (manifest.sourceCommit !== expectedSourceCommit) throw new Error(`Windows manifest source commit ${manifest.sourceCommit ?? "<missing>"} does not match ${expectedSourceCommit}`);
if (manifest.platform !== "windows" || manifest.architecture !== "x64") throw new Error("Windows manifest platform or architecture is invalid");

const assets = new Map((manifest.assets ?? []).map((asset) => [asset.name, asset]));
for (const option of ["--installer", "--signature"]) {
  const filePath = required(option);
  const name = filePath.replaceAll("\\", "/").split("/").at(-1);
  const expected = assets.get(name);
  if (!expected || !Number.isSafeInteger(expected.size) || !/^[0-9a-f]{64}$/i.test(expected.sha256 ?? "")) {
    throw new Error(`Windows manifest is missing a valid asset entry for ${name}`);
  }
  const info = await stat(filePath);
  if (info.size !== expected.size) throw new Error(`Windows asset size mismatch for ${name}: ${info.size} != ${expected.size}`);
  const hash = createHash("sha256");
  hash.update(await readFile(filePath));
  if (hash.digest("hex") !== expected.sha256.toLowerCase()) throw new Error(`Windows asset SHA-256 mismatch for ${name}`);
}

console.log(`Verified Windows release manifest ${manifestPath} for ${expectedSourceCommit}`);
