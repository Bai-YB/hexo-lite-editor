import { readFile, writeFile } from "node:fs/promises";
import { basename } from "node:path";

const values = new Map();
for (let index = 2; index < process.argv.length; index += 2) {
  values.set(process.argv[index], process.argv[index + 1]);
}
const required = (name) => {
  const value = values.get(name);
  if (!value) throw new Error(`Missing ${name}`);
  return value;
};
const version = required("--version");
const tag = values.get("--tag") ?? `v${version}`;
const repository = required("--repository");
const output = required("--output");
const notes = await readFile(required("--notes"), "utf8");
const assetUrl = (file) => `https://github.com/${repository}/releases/download/${tag}/${encodeURIComponent(basename(file))}`;
const platform = async (asset, signature) => ({
  signature: (await readFile(signature, "utf8")).trim(),
  url: assetUrl(asset)
});

const platforms = {};
if (values.has("--windows-asset")) {
  platforms["windows-x86_64"] = await platform(required("--windows-asset"), required("--windows-signature"));
}
if (values.has("--macos-asset")) {
  const macos = await platform(required("--macos-asset"), required("--macos-signature"));
  platforms["darwin-x86_64"] = macos;
  platforms["darwin-aarch64"] = macos;
}
if (!Object.keys(platforms).length) throw new Error("At least one platform is required");

await writeFile(output, `${JSON.stringify({
  version,
  notes,
  pub_date: new Date().toISOString(),
  platforms
}, null, 2)}\n`, "utf8");
