export interface HexoProject {
  rootPath: string;
  postsPath: string;
  configPath: string;
  packageJsonPath?: string;
  name: string;
  isValid: boolean;
  warnings: string[];
}
