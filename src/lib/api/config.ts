import { command } from "./tauri";
import { defaultSettings, type AppSettings, type ColorScheme } from "$lib/types/settings";

type LooseSettings = Partial<AppSettings> & {
  auto_save?: boolean;
  autoSave?: boolean;
  auto_save_interval?: number;
  autoSaveInterval?: number;
  editor_font_size?: number;
  editorFontSize?: number;
  default_uploader?: string;
  hexo_command?: string;
  recent_projects?: string[];
};

export async function loadSettings(): Promise<AppSettings> {
  try {
    const settings = await command<LooseSettings>("load_app_config");
    return normalizeSettings(settings);
  } catch {
    return structuredClone(defaultSettings);
  }
}

export async function saveSettings(settings: AppSettings): Promise<void> {
  await command<void>("save_app_config", { config: settings });
}

export async function resetSettings(): Promise<AppSettings> {
  return normalizeSettings(await command<LooseSettings>("reset_app_config"));
}

export function normalizeSettings(input: LooseSettings = {}): AppSettings {
  const legacyAutoSave = input.autoSave ?? input.auto_save;
  const legacyInterval = input.autoSaveInterval ?? input.auto_save_interval;
  const legacyFontSize = input.editorFontSize ?? input.editor_font_size;
  const recentProjects = input.recentProjects ?? input.recent_projects ?? defaultSettings.recentProjects;

  const coverSourcePriority = sanitizeCoverSourcePriority(
    input.postList?.coverSourcePriority ?? defaultSettings.postList.coverSourcePriority
  );
  const layoutMigrated = input.layout?.splitLayoutMigrated === true;

  return {
    general: {
      ...defaultSettings.general,
      ...input.general,
      autoSave: input.general?.autoSave ?? legacyAutoSave ?? defaultSettings.general.autoSave,
      autoSaveInterval: input.general?.autoSaveInterval ?? legacyInterval ?? defaultSettings.general.autoSaveInterval
    },
    appearance: {
      ...defaultSettings.appearance,
      ...input.appearance,
      colorScheme: sanitizeColorScheme(input.appearance?.colorScheme)
    },
    editor: {
      ...defaultSettings.editor,
      ...input.editor,
      fontSize: input.editor?.fontSize ?? legacyFontSize ?? defaultSettings.editor.fontSize
    },
    layout: {
      ...defaultSettings.layout,
      ...input.layout,
      previewWidth: layoutMigrated ? (input.layout?.previewWidth ?? defaultSettings.layout.previewWidth) : 0,
      splitLayoutMigrated: true
    },
    ribbon: {
      ...defaultSettings.ribbon,
      ...input.ribbon
    },
    postList: {
      ...defaultSettings.postList,
      ...input.postList,
      coverSourcePriority
    },
    uploader: {
      ...defaultSettings.uploader,
      ...input.uploader,
      defaultType: sanitizeUploaderType(input.uploader?.defaultType ?? input.default_uploader)
    },
    publish: {
      ...defaultSettings.publish,
      ...input.publish,
      hexoServerCommand: input.publish?.hexoServerCommand ?? input.hexo_command ?? defaultSettings.publish.hexoServerCommand
    },
    sync: {
      ...defaultSettings.sync,
      ...input.sync,
      remoteName: sanitizeSyncName(input.sync?.remoteName, defaultSettings.sync.remoteName),
      branchName: sanitizeSyncName(input.sync?.branchName, defaultSettings.sync.branchName)
    },
    update: {
      ...defaultSettings.update,
      ...input.update
    },
    recentProjects
  };
}

function sanitizeColorScheme(value: unknown): ColorScheme {
  const allowed: ColorScheme[] = ["vscode-light", "vscode-dark"];
  return allowed.includes(value as ColorScheme) ? (value as ColorScheme) : defaultSettings.appearance.colorScheme;
}

function sanitizeCoverSourcePriority(priority: string[]): string[] {
  const allowed = new Set(["cover"]);
  const normalized = priority.filter((item) => allowed.has(item));
  return normalized.length ? normalized : defaultSettings.postList.coverSourcePriority;
}

function isUploaderType(value: unknown): value is AppSettings["uploader"]["defaultType"] {
  return value === "local" || value === "custom" || value === "smms" || value === "cloudflare-imgbed";
}

function sanitizeUploaderType(value: unknown): AppSettings["uploader"]["defaultType"] {
  return isUploaderType(value) ? value : defaultSettings.uploader.defaultType;
}

function sanitizeSyncName(value: unknown, fallback: string): string {
  return typeof value === "string" && value.trim() ? value.trim() : fallback;
}
