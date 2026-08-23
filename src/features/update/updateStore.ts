import { writable } from "svelte/store"; import type { UpdateSnapshot } from "$shared/types/app";
export const updateStore = writable<UpdateSnapshot>({ currentVersion: "", status: "idle" });
