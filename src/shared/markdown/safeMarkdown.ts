import DOMPurify from "dompurify";
import MarkdownIt from "markdown-it";

const markdown = new MarkdownIt({
  html: false,
  linkify: true,
  breaks: false,
  typographer: false
});

const allowedTags = [
  "a",
  "blockquote",
  "br",
  "code",
  "del",
  "em",
  "h1",
  "h2",
  "h3",
  "h4",
  "h5",
  "h6",
  "hr",
  "img",
  "li",
  "ol",
  "p",
  "pre",
  "s",
  "strong",
  "table",
  "tbody",
  "td",
  "th",
  "thead",
  "tr",
  "ul"
];

export function renderSafeMarkdown(
  source: string,
  localAssets: Record<string, string> = {}
): string {
  const rendered = markdown.render(stripFrontMatter(source));
  const renderedDocument = new DOMParser().parseFromString(rendered, "text/html");
  renderedDocument.querySelectorAll("img").forEach((image) => {
    const original = image.getAttribute("src") ?? "";
    image.setAttribute("src", rewriteLocalImage(original, localAssets));
  });
  const clean = DOMPurify.sanitize(renderedDocument.body.innerHTML, {
    ALLOWED_TAGS: allowedTags,
    ALLOWED_ATTR: ["alt", "class", "href", "src", "title"],
    ALLOW_DATA_ATTR: false,
    FORBID_TAGS: ["form", "iframe", "object", "script", "style", "svg", "math"],
    FORBID_ATTR: ["style"],
    ALLOWED_URI_REGEXP: /^(?:(?:https?|hlex-asset):|blob:|data:image\/(?:png|jpeg|gif|webp);base64,)/i
  });
  const document = new DOMParser().parseFromString(clean, "text/html");
  document.querySelectorAll("a").forEach((anchor) => {
    const href = anchor.getAttribute("href") ?? "";
    anchor.removeAttribute("href");
    if (isSafeExternalLink(href)) {
      anchor.dataset.externalHref = href;
      anchor.setAttribute("role", "link");
      anchor.setAttribute("tabindex", "0");
    }
  });
  document.querySelectorAll("img").forEach((image) => {
    const original = image.getAttribute("src") ?? "";
    const src = rewriteLocalImage(original, localAssets);
    if (src !== original) image.setAttribute("src", src);
    if (!isSafeImageSource(src)) image.remove();
    else image.setAttribute("loading", "lazy");
  });
  return document.body.innerHTML;
}

export function rewriteLocalImage(value: string, localAssets: Record<string, string>): string {
  const encodedPath = value.split(/[?#]/, 1)[0];
  let decodedPath = encodedPath;
  try {
    decodedPath = decodeURI(encodedPath);
  } catch {
    // Keep malformed URI input unchanged so it can be rejected by the safe-source check.
  }
  const clean = decodedPath.replace(/\\/g, "/");
  const normalized = clean.replace(/^\.\//, "").replace(/^\//, "");
  const candidates = [
    clean,
    normalized,
    normalized.startsWith("images/") ? `source/${normalized}` : "",
    normalized.startsWith("source/images/")
      ? normalized.replace(/^source\//, "")
      : ""
  ].filter(Boolean);
  for (const candidate of candidates) {
    if (localAssets[candidate]) return localAssets[candidate];
    if (localAssets[`/${candidate}`]) return localAssets[`/${candidate}`];
  }
  return value;
}

export function stripFrontMatter(source: string): string {
  const normalized = source.replace(/^\uFEFF/, "");
  if (!normalized.startsWith("---\n") && !normalized.startsWith("---\r\n")) return source;
  const match = normalized.match(/^---\r?\n[\s\S]*?\r?\n(?:---|\.\.\.)\r?\n/);
  return match ? normalized.slice(match[0].length) : source;
}

export function isSafeExternalLink(value: string): boolean {
  try {
    const parsed = new URL(value);
    return (
      (parsed.protocol === "https:" || parsed.protocol === "http:") &&
      parsed.username === "" &&
      parsed.password === ""
    );
  } catch {
    return false;
  }
}

export function isSafeImageSource(value: string): boolean {
  if (/^data:image\/(png|jpeg|gif|webp);base64,/i.test(value)) return true;
  if (value.startsWith("blob:")) return true;
  try {
    const parsed = new URL(value);
    if (parsed.protocol === "https:") return true;
    if (parsed.protocol === "hlex-asset:") return true;
    return parsed.protocol === "http:" && parsed.hostname === "hlex-asset.localhost";
  } catch {
    return false;
  }
}
