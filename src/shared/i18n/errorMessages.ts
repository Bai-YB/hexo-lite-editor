import type { AppError } from "$shared/types/app";
import { t, type TranslationKey } from "./index";

const knownErrors: Record<string, TranslationKey> = {
  project_not_found: "errors.project_not_found",
  image_list_failed: "errors.image_list_failed",
  update_check_failed: "errors.update_check_failed",
  plugin_permission_denied: "errors.plugin_permission_denied"
};

export function localizeAppError(error: AppError): string {
  const key = knownErrors[error.code];
  return key ? t(key) : error.message;
}
