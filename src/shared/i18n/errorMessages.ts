import type { AppError } from "$shared/types/app";
import { t, type TranslationKey } from "./index";

const knownErrors: Record<string, TranslationKey> = {
  project_not_found: "errors.project_not_found",
  image_list_failed: "errors.image_list_failed",
  update_check_failed: "errors.update_check_failed",
  plugin_permission_denied: "errors.plugin_permission_denied"
};

/** Rust errors arrive as finished Chinese text; unify glossary and toast punctuation. */
export function normalizeErrorMessage(message: string): string {
  const wording = message.replace(/远端|远程/g, "云端").replace(/请重新选择/g, "重新选择");
  if (!/[\u4e00-\u9fff]/.test(wording) || /[。！？…；：]$/.test(wording)) return wording;
  return `${wording}。`;
}

export function localizeAppError(error: AppError): string {
  const key = knownErrors[error.code];
  return key ? t(key) : error.message;
}
