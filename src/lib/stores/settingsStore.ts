import { writable } from "svelte/store";
import { defaultSettings, type AppSettings } from "$lib/types/settings";

export const settingsStore = writable<AppSettings>(defaultSettings);
