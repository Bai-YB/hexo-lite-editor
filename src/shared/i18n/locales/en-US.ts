import type { TranslationTree } from "../types";

export const enUS: TranslationTree = {
  common: { confirm: "Confirm", cancel: "Cancel", save: "Save", delete: "Delete", retry: "Retry", close: "Close" },
  navigation: { label: "Main navigation", editor: "Editor", imageBed: "Images", plugins: "Plugins", files: "All files", settings: "Settings", about: "About" },
  window: { minimize: "Minimize", maximize: "Maximize", restore: "Restore", close: "Close", unsaved: "Unsaved changes" },
  loading: { workspace: "Initializing desktop workspace", page: "Loading page" },
  settings: {
    languageTitle: "Interface language",
    languageDescription: "Follow the system language or choose one for this app.",
    languageSystem: "System default",
    languageChinese: "简体中文",
    languageEnglish: "English"
  },
  update: { availableNotice: "Version {version} is available. Open About to review it." },
  pages: {
    aboutTitle: "About", aboutDescription: "A calm and reliable Hexo desktop writing tool.",
    aboutProductDescription: "Focus on Markdown writing, image management, and reliable publishing without bundling a blog environment or taking ownership of your articles.",
    projectHomepage: "Project homepage", projectHomepageDescription: "Source code, issue reporting, and release history", licenseDescription: "View the open-source license",
    version: "Version", update: "Update", notChecked: "Not checked", checkUpdate: "Check for updates", checking: "Checking",
    downloadUpdate: "Download update", restartInstall: "Restart and install"
  },
  errors: {
    project_not_found: "The project could not be found. Select the Hexo project again.",
    image_list_failed: "The image list could not be loaded. Try again.",
    update_check_failed: "The update check failed. Check your connection and try again.",
    plugin_permission_denied: "The plugin does not have permission to perform this action."
  }
};
