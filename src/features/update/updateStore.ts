import { writable } from "svelte/store";
import type { UpdateSnapshot } from "$shared/types/app";
import { appVersion } from "$shared/version";

export const updateStore = writable<UpdateSnapshot>({ currentVersion: appVersion, status: "idle" });

export function shouldAutoDownload(snapshot: UpdateSnapshot, autoDownload: boolean): boolean {
  return autoDownload && snapshot.status === "available";
}
