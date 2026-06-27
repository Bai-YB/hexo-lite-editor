import { command } from "./tauri";
import type { HexoProject } from "$lib/types/project";

interface RawProject {
  root_path: string;
  posts_path: string;
  config_path: string;
  package_json_path?: string;
  name: string;
  is_valid: boolean;
  warnings: string[];
}

export async function validateProject(path: string): Promise<HexoProject> {
  const project = await command<RawProject>("validate_hexo_project", { path });
  return {
    rootPath: project.root_path,
    postsPath: project.posts_path,
    configPath: project.config_path,
    packageJsonPath: project.package_json_path,
    name: project.name,
    isValid: project.is_valid,
    warnings: project.warnings
  };
}
