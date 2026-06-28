import MarkdownIt from "markdown-it";
import matter from "gray-matter";

const renderer = new MarkdownIt({
  html: true,
  linkify: true,
  typographer: true,
  breaks: true
});

export interface RenderMarkdownOptions {
  imageCacheBust?: string;
}

export function renderMarkdown(content: string, options: RenderMarkdownOptions = {}): string {
  const html = renderer.render(stripFrontMatter(content || ""));
  return options.imageCacheBust ? cacheBustPreviewImages(html, options.imageCacheBust) : html;
}

export function stripFrontMatter(content: string): string {
  if (!content.startsWith("---")) return content;
  try {
    return matter(content).content;
  } catch {
    return content.replace(/^---\r?\n[\s\S]*?\r?\n---\r?\n?/, "");
  }
}

function cacheBustPreviewImages(html: string, token: string): string {
  return html.replace(/<img\b([^>]*?)\bsrc=(["'])(.*?)\2([^>]*)>/gi, (match, before, quote, src, after) => {
    const nextSrc = cacheBustImageUrl(src, token);
    if (nextSrc === src) return match;
    return `<img${before}src=${quote}${nextSrc}${quote}${after}>`;
  });
}

function cacheBustImageUrl(src: string, token: string): string {
  if (!src || !/^https?:\/\//i.test(src)) return src;
  try {
    const url = new URL(src);
    url.searchParams.set("hlex-refresh", token);
    return url.toString();
  } catch {
    const [baseAndQuery, hash = ""] = src.split("#", 2);
    const separator = baseAndQuery.includes("?") ? "&" : "?";
    const next = `${baseAndQuery}${separator}hlex-refresh=${encodeURIComponent(token)}`;
    return hash ? `${next}#${hash}` : next;
  }
}
