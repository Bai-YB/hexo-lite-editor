import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { resolve } from "node:path";

export function resolveReleaseVersion(packageInfo) {
  const runtimeVersion = packageInfo.version;
  const releaseVersion = packageInfo.releaseVersion ?? runtimeVersion;
  if (!/^\d+\.\d+\.\d+(?:\+\d+)?$/.test(runtimeVersion)) throw new Error("Invalid runtime version");
  if (releaseVersion !== runtimeVersion.replace("+", ".")) throw new Error("Release and runtime versions do not match");
  return { version: releaseVersion, runtimeVersion, tag: `v${releaseVersion}` };
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const release = resolveReleaseVersion(JSON.parse(readFileSync(new URL("../package.json", import.meta.url), "utf8")));
  const field = process.argv[2];
  if (field) {
    if (!Object.hasOwn(release, field)) throw new Error(`Unknown release field: ${field}`);
    console.log(release[field]);
  } else console.log(JSON.stringify(release));
}
