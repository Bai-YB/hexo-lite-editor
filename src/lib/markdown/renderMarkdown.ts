import MarkdownIt from "markdown-it";
import matter from "gray-matter";

const renderer = new MarkdownIt({
  html: true,
  linkify: true,
  typographer: true,
  breaks: true
});

export function renderMarkdown(content: string): string {
  return renderer.render(stripFrontMatter(content || ""));
}

export function stripFrontMatter(content: string): string {
  if (!content.startsWith("---")) return content;
  try {
    return matter(content).content;
  } catch {
    return content.replace(/^---\r?\n[\s\S]*?\r?\n---\r?\n?/, "");
  }
}
