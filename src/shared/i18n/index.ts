import { writable } from "svelte/store";
import type { AppLanguage } from "$shared/types/app";
import { enUS } from "./locales/en-US";
import { zhCN } from "./locales/zh-CN";
import { applyDocumentLanguage, resolveLanguage, type ResolvedLanguage } from "./language";
import type { TranslationKey, TranslationTree } from "./types";

const dictionaries: Record<ResolvedLanguage, TranslationTree> = { "zh-CN": zhCN, "en-US": enUS };
let selectedLanguage: AppLanguage = "system";
let resolvedLanguage: ResolvedLanguage = "en-US";
export const language = writable<ResolvedLanguage>(resolvedLanguage);
export const translate = writable<typeof t>(t);

export function initI18n(selected: AppLanguage): void {
  setLanguage(selected);
}

export function setLanguage(selected: AppLanguage): void {
  selectedLanguage = selected;
  resolvedLanguage = resolveLanguage(selected);
  applyDocumentLanguage(resolvedLanguage);
  language.set(resolvedLanguage);
  translate.set(t);
}

export function getSelectedLanguage(): AppLanguage {
  return selectedLanguage;
}

export function getResolvedLanguage(): ResolvedLanguage {
  return resolvedLanguage;
}

export function t(path: TranslationKey, params: Record<string, string | number> = {}): string {
  const [group, key] = path.split(".");
  const value = (dictionaries[resolvedLanguage] as unknown as Record<string, Record<string, string>>)[group]?.[key];
  if (value === undefined) {
    if (import.meta.env.DEV) throw new Error(`Unknown translation key: ${path}`);
    return path;
  }
  return value.replace(/\{(\w+)\}/g, (match, name) => name in params ? String(params[name]) : match);
}

export type { ResolvedLanguage, TranslationKey, TranslationTree };
