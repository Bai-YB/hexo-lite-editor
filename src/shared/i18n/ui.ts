import { derived } from "svelte/store";
import { language } from "./index";
import type { ResolvedLanguage } from "./language";
import { uiMessages } from "./uiMessages";

export type UiParams = Record<string, string | number>;

export function translateUi(source: string, locale: ResolvedLanguage, params: UiParams = {}): string {
  const message = locale === "en-US" ? uiMessages[source] ?? source : source;
  // One pass over the template only: placeholder-looking user text stays literal.
  return message.replace(/\{(\w+)\}/g, (placeholder, name: string) =>
    Object.prototype.hasOwnProperty.call(params, name) ? String(params[name]) : placeholder);
}

export const ui = derived(language, ($language) =>
  (source: string, params: UiParams = {}) => translateUi(source, $language, params));
