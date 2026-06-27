import { openUrl } from "@tauri-apps/plugin-opener";
import { command } from "./tauri";
import type { HexoProject } from "$lib/types/project";
import type { Post } from "$lib/types/post";

const BASE_URL = "http://localhost:4000";

type FrontMatter = Record<string, string>;

export async function openCurrentPostPreview(
  project: HexoProject | null,
  post: Post | null,
  content: string
): Promise<void> {
  await openUrl(await resolvePostPreviewUrl(project, post, content));
}

export async function resolvePostPreviewUrl(
  project: HexoProject | null,
  post: Post | null,
  content: string
): Promise<string> {
  if (!project || !post) return BASE_URL;

  const frontMatter = parseFrontMatter(content);
  const permalink = frontMatter.permalink?.trim();
  if (permalink) return absolutizePreviewUrl(permalink);

  const configPermalink = await readConfigPermalink(project.rootPath);
  const pattern = configPermalink || "/:year/:month/:day/:title/";
  return absolutizePreviewUrl(applyPermalinkPattern(pattern, post, frontMatter));
}

function parseFrontMatter(content: string): FrontMatter {
  if (!content.startsWith("---")) return {};
  const end = content.search(/\r?\n---\s*(\r?\n|$)/);
  if (end <= 0) return {};
  const yaml = content.slice(3, end).split(/\r?\n/);
  const values: FrontMatter = {};
  for (const line of yaml) {
    const match = /^([A-Za-z_][\w-]*)\s*:\s*(.*)$/.exec(line);
    if (!match) continue;
    values[match[1]] = stripYamlValue(match[2]);
  }
  return values;
}

async function readConfigPermalink(projectPath: string): Promise<string> {
  try {
    const content = await command<string>("read_text_file", { path: `${projectPath}/_config.yml` });
    const match = /^permalink\s*:\s*(.+)$/m.exec(content);
    return match ? stripYamlValue(match[1]) : "";
  } catch {
    return "";
  }
}

function applyPermalinkPattern(pattern: string, post: Post, frontMatter: FrontMatter): string {
  const date = parseDate(frontMatter.date || post.date);
  const title = slugFromFileName(post.fileName);
  const replacements: Record<string, string> = {
    ":year": date.year,
    ":month": date.month,
    ":i_month": String(Number(date.month)),
    ":day": date.day,
    ":i_day": String(Number(date.day)),
    ":title": title,
    ":name": title,
    ":post_title": title
  };
  let value = pattern || "/:year/:month/:day/:title/";
  for (const [token, replacement] of Object.entries(replacements)) {
    value = value.split(token).join(replacement);
  }
  return value;
}

function parseDate(value?: string) {
  const date = value ? new Date(value.replace(/-/g, "/")) : new Date();
  const safeDate = Number.isNaN(date.getTime()) ? new Date() : date;
  return {
    year: String(safeDate.getFullYear()),
    month: String(safeDate.getMonth() + 1).padStart(2, "0"),
    day: String(safeDate.getDate()).padStart(2, "0")
  };
}

function slugFromFileName(fileName: string) {
  return fileName.replace(/\.(md|markdown)$/i, "");
}

function stripYamlValue(value: string) {
  const withoutComment = value.replace(/\s+#.*$/, "").trim();
  return withoutComment.replace(/^["']|["']$/g, "");
}

function absolutizePreviewUrl(pathOrUrl: string) {
  if (/^https?:\/\//i.test(pathOrUrl)) return pathOrUrl;
  const cleanPath = pathOrUrl.startsWith("/") ? pathOrUrl : `/${pathOrUrl}`;
  const withTrailingSlash = cleanPath.endsWith("/") || cleanPath.includes(".") ? cleanPath : `${cleanPath}/`;
  return `${BASE_URL}${encodeURI(withTrailingSlash)}`;
}
