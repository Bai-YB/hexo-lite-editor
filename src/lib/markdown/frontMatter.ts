import matter from "gray-matter";

export function parseFrontMatter(content: string) {
  return matter(content);
}

export function contentWithoutFrontMatter(content: string): string {
  return matter(content).content;
}
