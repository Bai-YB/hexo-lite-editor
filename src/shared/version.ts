import packageInfo from "../../package.json";

export const appVersion = packageInfo.releaseVersion;
export const runtimeVersion = packageInfo.version;

export function displayVersion(version: string) {
  return version.replace(/^(\d+\.\d+\.\d+)\+(\d+)$/, "$1.$2");
}
