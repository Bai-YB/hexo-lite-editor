import { readFile } from "node:fs/promises";

const values = new Map();
const platforms = [];
for (let index = 2; index < process.argv.length; index += 2) {
  const key = process.argv[index];
  const value = process.argv[index + 1];
  if (!key || !value) throw new Error(`Missing value for ${key ?? "argument"}`);
  if (key === "--platform") platforms.push(value);
  else values.set(key, value);
}

const required = (name) => {
  const value = values.get(name);
  if (!value) throw new Error(`Missing ${name}`);
  return value;
};
const expectedVersion = required("--version").replace(/^v/, "");
const manifestPath = required("--manifest");
if (!platforms.length) throw new Error("Provide at least one --platform");

const manifest = JSON.parse(await readFile(manifestPath, "utf8"));
if (String(manifest.version ?? "").replace(/^v/, "") !== expectedVersion) {
  throw new Error(`Manifest version ${manifest.version ?? "<missing>"} does not match ${expectedVersion}`);
}
if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/.test(expectedVersion)) {
  throw new Error(`Manifest version is not SemVer: ${expectedVersion}`);
}
if (typeof manifest.notes !== "string" || !manifest.notes.trim()) throw new Error("Manifest notes are missing");
if (!manifest.pub_date || Number.isNaN(Date.parse(manifest.pub_date))) throw new Error("Manifest pub_date is invalid");
for (const platform of platforms) {
  const entry = manifest.platforms?.[platform];
  if (!entry) throw new Error(`Manifest is missing platform ${platform}`);
  if (typeof entry.signature !== "string" || !entry.signature.trim()) throw new Error(`Manifest signature is missing for ${platform}`);
  let url;
  try { url = new URL(entry.url); } catch { throw new Error(`Manifest URL is invalid for ${platform}`); }
  if (url.protocol !== "https:") throw new Error(`Manifest URL must use HTTPS for ${platform}`);
}
console.log(`Verified ${manifestPath}: ${expectedVersion} (${platforms.join(", ")})`);
