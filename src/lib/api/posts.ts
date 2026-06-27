import dayjs from "dayjs";
import { command } from "./tauri";
import type { CreatePostInput, Post } from "$lib/types/post";

interface RawPost {
  id: string;
  title: string;
  file_name: string;
  file_path: string;
  date?: string;
  cover?: string;
  top_img?: string;
  banner?: string;
  thumbnail?: string;
  index_img?: string;
  description?: string;
  tags: string[];
  categories: string[];
  created_at?: string;
  updated_at?: string;
  is_draft: boolean;
}

export async function scanPosts(projectPath: string): Promise<Post[]> {
  const posts = await command<RawPost[]>("scan_posts", { projectPath });
  return posts.map(mapPost);
}

export async function readPostContent(filePath: string): Promise<string> {
  return command<string>("read_text_file", { path: filePath });
}

export async function savePostContent(filePath: string, content: string): Promise<void> {
  await command<void>("write_text_file", { path: filePath, content });
}

export async function backupPostContent(projectPath: string, filePath: string): Promise<string> {
  const result = await command<{ backup_path: string }>("backup_text_file", { projectPath, filePath });
  return result.backup_path;
}

export async function createPost(projectPath: string, input: CreatePostInput): Promise<string> {
  return command<string>("create_post", {
    projectPath,
    fileName: input.fileName,
    content: buildPostContent(input)
  });
}

export function buildPostContent(input: CreatePostInput): string {
  const tags = input.tags.length ? input.tags.map((tag) => `  - ${tag}`).join("\n") : "  - ";
  const categories = input.categories.length
    ? input.categories.map((category) => `  - ${category}`).join("\n")
    : "  - ";

  return `---\ntitle: ${input.title}\ndate: ${input.date}\ntags:\n${tags}\ncategories:\n${categories}\n---\n\n这里开始写正文。\n`;
}

export function slugifyTitle(title: string): string {
  const ascii = title
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9\u4e00-\u9fa5]+/g, "-")
    .replace(/^-+|-+$/g, "");
  return ascii || `post-${dayjs().format("YYYYMMDD-HHmmss")}`;
}

function mapPost(post: RawPost): Post {
  return {
    id: post.id,
    title: post.title,
    fileName: post.file_name,
    filePath: post.file_path,
    date: post.date,
    cover: post.cover,
    topImg: post.top_img,
    banner: post.banner,
    thumbnail: post.thumbnail,
    indexImg: post.index_img,
    description: post.description,
    tags: post.tags,
    categories: post.categories,
    createdAt: formatUnix(post.created_at),
    updatedAt: formatUnix(post.updated_at),
    isDraft: post.is_draft,
    isDirty: false
  };
}

function formatUnix(value?: string): string | undefined {
  if (!value) return undefined;
  const timestamp = Number(value);
  if (!Number.isFinite(timestamp)) return value;
  return dayjs(timestamp * 1000).format("YYYY-MM-DD HH:mm:ss");
}
