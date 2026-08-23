import type { AppLanguage } from "$shared/types/app";

export type ResolvedLanguage = "zh-CN" | "en-US";

export function resolveSystemLanguage(language = typeof navigator === "undefined" ? "en-US" : navigator.language): ResolvedLanguage {
  return /^zh(?:$|-)/i.test(language) ? "zh-CN" : "en-US";
}

export function resolveLanguage(language: AppLanguage, systemLanguage?: string): ResolvedLanguage {
  return language === "system" ? resolveSystemLanguage(systemLanguage) : language;
}

export function applyDocumentLanguage(language: ResolvedLanguage): void {
  if (typeof document !== "undefined") document.documentElement.lang = language;
}
