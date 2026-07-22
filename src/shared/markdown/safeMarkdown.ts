import DOMPurify from "dompurify";
import MarkdownIt from "markdown-it";

const markdown = new MarkdownIt({
  html: true,
  linkify: true,
  breaks: false,
  typographer: false
});

const allowedTags = [
  "a",
  "abbr",
  "address",
  "article",
  "aside",
  "b",
  "bdi",
  "bdo",
  "blockquote",
  "br",
  "caption",
  "cite",
  "code",
  "col",
  "colgroup",
  "dd",
  "del",
  "details",
  "div",
  "dl",
  "dt",
  "em",
  "figcaption",
  "figure",
  "footer",
  "h1",
  "h2",
  "h3",
  "h4",
  "h5",
  "h6",
  "hr",
  "img",
  "kbd",
  "li",
  "main",
  "mark",
  "nav",
  "ol",
  "p",
  "pre",
  "q",
  "samp",
  "s",
  "section",
  "small",
  "span",
  "strong",
  "sub",
  "summary",
  "sup",
  "table",
  "tbody",
  "td",
  "th",
  "thead",
  "tr",
  "ul",
  "time",
  "var"
];

const allowedStyleProperties = new Set([
  "color", "background-color", "font-size", "font-style", "font-weight", "font-family",
  "line-height", "letter-spacing", "text-align", "text-decoration", "text-indent", "text-transform",
  "white-space", "word-break", "overflow-wrap",
  "margin", "margin-top", "margin-right", "margin-bottom", "margin-left",
  "padding", "padding-top", "padding-right", "padding-bottom", "padding-left",
  "border", "border-width", "border-style", "border-color", "border-radius",
  "border-top", "border-right", "border-bottom", "border-left",
  "display", "gap", "row-gap", "column-gap", "flex", "flex-basis", "flex-grow", "flex-shrink",
  "flex-direction", "flex-wrap", "align-items", "align-content", "align-self",
  "justify-content", "justify-items", "justify-self", "place-items", "place-content", "place-self",
  "grid-template-columns", "grid-template-rows", "grid-column", "grid-row",
  "width", "min-width", "max-width", "height", "min-height", "max-height"
]);

export function renderSafeMarkdown(
  source: string,
  localAssets: Record<string, string> = {},
  remoteAssets: Record<string, string | null> = {},
  remotePending = false
): string {
  const rendered = markdown.render(stripFrontMatter(source));
  const renderedDocument = new DOMParser().parseFromString(rendered, "text/html");
  sanitizeSourceAttributes(renderedDocument);
  renderedDocument.querySelectorAll("img").forEach((image) => {
    const original = image.getAttribute("src") ?? "";
    const rewritten = rewriteLocalImage(original, localAssets);
    if (isRemoteImageSource(rewritten)) {
      const resolved = remoteAssets[rewritten];
      if (resolved) image.setAttribute("src", resolved);
      else image.replaceWith(imagePlaceholder(renderedDocument, image.getAttribute("alt") ?? "", remotePending));
    } else {
      image.setAttribute("src", rewritten);
    }
  });
  const clean = DOMPurify.sanitize(renderedDocument.body.innerHTML, {
    ALLOWED_TAGS: allowedTags,
    ALLOWED_ATTR: [
      "alt", "aria-label", "class", "colspan", "datetime", "height", "href", "open",
      "reversed", "role", "rowspan", "scope", "src", "start", "style", "title", "value", "width"
    ],
    ALLOW_DATA_ATTR: false,
    FORBID_TAGS: ["form", "iframe", "object", "script", "style", "svg", "math"],
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
    if (!isSafeImageSource(src)) image.replaceWith(imagePlaceholder(document, image.getAttribute("alt") ?? "", false));
    else image.setAttribute("loading", "lazy");
  });
  return document.body.innerHTML;
}

export function extractRemoteImageUrls(
  source: string,
  localAssets: Record<string, string> = {}
): string[] {
  const rendered = markdown.render(stripFrontMatter(source));
  const document = new DOMParser().parseFromString(rendered, "text/html");
  const urls = new Set<string>();
  document.querySelectorAll("img").forEach((image) => {
    const value = rewriteLocalImage(image.getAttribute("src") ?? "", localAssets);
    if (isRemoteImageSource(value)) urls.add(value);
  });
  return [...urls];
}

function sanitizeSourceAttributes(document: Document) {
  document.querySelectorAll<HTMLElement>("[class]").forEach((element) => {
    const safe = [...element.classList].filter((name) => /^language-[a-z0-9_-]+$/i.test(name));
    if (safe.length) element.className = safe.join(" ");
    else element.removeAttribute("class");
  });
  document.querySelectorAll<HTMLElement>("[style]").forEach((element) => {
    const style = sanitizeInlineStyle(element.getAttribute("style") ?? "");
    if (style) element.setAttribute("style", style);
    else element.removeAttribute("style");
  });
}

export function sanitizeInlineStyle(value: string): string {
  return value
    .split(";")
    .map((declaration) => declaration.split(/:(.*)/s).slice(0, 2).map((part) => part.trim()))
    .filter(([property, cssValue]) => {
      if (!property || !cssValue || !allowedStyleProperties.has(property.toLowerCase())) return false;
      return !/(?:url\s*\(|expression\s*\(|@import|!important|\\)/i.test(cssValue);
    })
    .map(([property, cssValue]) => `${property.toLowerCase()}: ${cssValue}`)
    .join("; ");
}

function imagePlaceholder(document: Document, alt: string, pending: boolean): HTMLElement {
  const placeholder = document.createElement("span");
  placeholder.className = "remote-image-placeholder";
  placeholder.setAttribute("role", "img");
  placeholder.setAttribute("aria-label", alt || (pending ? "正在验证图片" : "图片不可用"));
  placeholder.textContent = pending ? "正在验证图片…" : `图片不可用${alt ? `：${alt}` : ""}`;
  return placeholder;
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
    if (parsed.protocol === "hlex-asset:") return true;
    return parsed.protocol === "http:" && parsed.hostname === "hlex-asset.localhost";
  } catch {
    return false;
  }
}

export function isRemoteImageSource(value: string): boolean {
  try {
    const parsed = new URL(value);
    return parsed.protocol === "https:" && parsed.username === "" && parsed.password === "";
  } catch {
    return false;
  }
}
