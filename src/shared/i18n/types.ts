export interface TranslationTree {
  common: {
    confirm: string;
    cancel: string;
    save: string;
    delete: string;
    retry: string;
    close: string;
  };
  navigation: {
    label: string;
    editor: string;
    imageBed: string;
    plugins: string;
    files: string;
    settings: string;
    about: string;
  };
  window: {
    minimize: string;
    maximize: string;
    restore: string;
    close: string;
    unsaved: string;
  };
  loading: {
    workspace: string;
    page: string;
  };
  settings: {
    languageTitle: string;
    languageDescription: string;
    languageSystem: string;
    languageChinese: string;
    languageEnglish: string;
  };
  update: {
    availableNotice: string;
  };
  pages: {
    aboutTitle: string;
    aboutDescription: string;
    aboutProductDescription: string;
    projectHomepage: string;
    projectHomepageDescription: string;
    licenseDescription: string;
    version: string;
    update: string;
    notChecked: string;
    checkUpdate: string;
    checking: string;
    downloadUpdate: string;
    restartInstall: string;
  };
  errors: Record<string, string>;
}

type Join<Parent extends string, Child extends string> = Parent extends "" ? Child : `${Parent}.${Child}`;

export type TranslationKey = {
  [K in keyof TranslationTree & string]: TranslationTree[K] extends string
    ? K
    : {
        [C in keyof TranslationTree[K] & string]: TranslationTree[K][C] extends string
          ? Join<K, C>
          : never;
      }[keyof TranslationTree[K] & string];
}[keyof TranslationTree & string];
