import type { AppConfigV3, SettingsSectionId } from "$shared/types/app";

// Rebase only fields the user has not edited since this save was submitted.
export function mergeSavedDraft<T>(submitted: T, current: T, persisted: T): T {
  if (JSON.stringify(current) === JSON.stringify(submitted)) return structuredClone(persisted);
  if (current && submitted && persisted && typeof current === "object" && typeof submitted === "object"
    && typeof persisted === "object" && !Array.isArray(current)) {
    const result = { ...current } as Record<string, unknown>;
    const before = submitted as Record<string, unknown>;
    const after = persisted as Record<string, unknown>;
    for (const key of new Set([...Object.keys(result), ...Object.keys(before), ...Object.keys(after)])) {
      result[key] = mergeSavedDraft(before[key], result[key], after[key]);
    }
    return result as T;
  }
  return structuredClone(current);
}

export function validateSettings(config: AppConfigV3): { section: SettingsSectionId; field: string; message: string } | null {
  const limits: Array<[SettingsSectionId, string, number, number, number, boolean, string]> = [
    ["general", "autoSaveDelayMs", config.general.autoSaveDelayMs, 500, 30000, true, "自动保存延迟必须在 500–30000 毫秒之间。"],
    ["editing", "fontSize", config.editor.fontSize, 12, 28, true, "字号必须是 12–28 之间的整数。"],
    ["editing", "lineHeight", config.editor.lineHeight, 1.2, 2.2, false, "行高必须在 1.2–2.2 之间。"],
    ["hexoPublish", "previewPort", config.hexo.previewPort, 300, 65535, true, "预览端口必须是 300–65535 之间的整数。"]
  ];
  for (const [section, field, value, min, max, integer, message] of limits) {
    if (!Number.isFinite(value) || value < min || value > max || integer && !Number.isInteger(value)) return { section, field, message };
  }
  return null;
}
