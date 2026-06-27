import { writable } from "svelte/store";

export type SaveStatus = "已保存" | "未保存" | "保存中" | "保存失败";

export const editorContent = writable("");
export const saveStatus = writable<SaveStatus>("已保存");
export const lastSavedAt = writable<string>("");
