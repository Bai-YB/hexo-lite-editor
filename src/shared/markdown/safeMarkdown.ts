import DOMPurify from "dompurify";
import MarkdownIt from "markdown-it";
import type { PreviewImageResult } from "$shared/types/app";

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
  "center",
  "cite",
  "code",
  "col",
  "colgroup",
  "dd",
  "del",
  "dfn",
  "details",
  "div",
  "dl",
  "dt",
  "em",
  "figcaption",
  "figure",
  "footer",
  "header",
  "h1",
  "h2",
  "h3",
  "h4",
  "h5",
  "h6",
  "hr",
  "hgroup",
  "img",
  "ins",
  "kbd",
  "li",
  "main",
  "mark",
  "meter",
  "nav",
  "ol",
  "p",
  "pre",
  "progress",
  "q",
  "rp",
  "rt",
  "ruby",
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
  "tfoot",
  "u",
  "var",
  "wbr"
];

const allowedStyleProperties = new Set([
  "color", "background", "background-color", "background-clip", "-webkit-background-clip",
  "-webkit-text-fill-color", "font-size", "font-style", "font-weight", "font-family",
  "line-height", "letter-spacing", "text-align", "text-decoration", "text-indent", "text-transform",
  "white-space", "word-break", "overflow-wrap",
  "vertical-align", "opacity", "float", "clear",
  "margin", "margin-top", "margin-right", "margin-bottom", "margin-left",
  "padding", "padding-top", "padding-right", "padding-bottom", "padding-left",
  "border", "border-width", "border-style", "border-color", "border-radius",
  "border-top", "border-right", "border-bottom", "border-left",
  "border-collapse", "border-spacing", "caption-side", "table-layout", "empty-cells",
  "list-style", "list-style-position", "list-style-type",
  "display", "gap", "row-gap", "column-gap", "flex", "flex-basis", "flex-grow", "flex-shrink",
  "flex-direction", "flex-wrap", "align-items", "align-content", "align-self",
  "justify-content", "justify-items", "justify-self", "place-items", "place-content", "place-self",
  "grid-template-columns", "grid-template-rows", "grid-column", "grid-row",
  "width", "min-width", "max-width", "height", "min-height", "max-height"
]);

const safeSemanticAttributes = new Set([
  "abbr", "cite", "dir", "high", "lang", "low", "max", "min", "optimum"
]);

DOMPurify.addHook("uponSanitizeAttribute", (_node, event) => {
  if (safeSemanticAttributes.has(event.attrName)) event.forceKeepAttr = true;
});

export function renderSafeMarkdown(
  source: string,
  imageResults: Record<string, PreviewImageResult> = {},
  imagePending = false
): string {
  const rendered = markdown.render(stripFrontMatter(source));
  const renderedDocument = new DOMParser().parseFromString(rendered, "text/html");
  sanitizeSourceAttributes(renderedDocument);
  renderedDocument.querySelectorAll("img").forEach((image) => {
    const original = image.getAttribute("src") ?? "";
    image.dataset.imageSource = original;
    if (isEmbeddedImageSource(original)) return;
    if (isRemoteImageSource(original)) {
      image.setAttribute("src", original);
      return;
    }
    const result = imageResults[original];
    if (result?.state === "ready" && result.previewUrl) image.setAttribute("src", result.previewUrl);
    else image.replaceWith(imagePlaceholder(renderedDocument, image, result, imagePending));
  });
  const clean = DOMPurify.sanitize(renderedDocument.body.innerHTML, {
    ALLOWED_TAGS: allowedTags,
    ALLOWED_ATTR: [
      "abbr", "alt", "aria-label", "cite", "class", "colspan", "datetime", "dir", "height",
      "high", "href", "lang", "low", "max", "min", "open", "optimum", "reversed", "role",
      "data-image-source", "data-preview-image-retry", "rowspan", "scope", "src", "start", "style", "tabindex", "title", "value", "width"
    ],
    ADD_ATTR: [...safeSemanticAttributes, "data-image-source", "data-preview-image-retry"],
    ALLOW_DATA_ATTR: false,
    FORBID_TAGS: ["form", "iframe", "object", "script", "style", "svg", "math"],
    ALLOWED_URI_REGEXP: /^(?:(?:https?|hlex-asset):|blob:|data:image\/(?:png|jpeg|gif|webp);base64,|(?:\.{0,2}\/|\/)?[^:/?#][^:]*)/i
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
    const src = image.getAttribute("src") ?? "";
    if (isSafeImageSource(src)) image.setAttribute("loading", "lazy");
  });
  return document.body.innerHTML;
}

export function extractPreviewImageSources(source: string): string[] {
  const rendered = markdown.render(stripFrontMatter(source));
  const document = new DOMParser().parseFromString(rendered, "text/html");
  const urls = new Set<string>();
  document.querySelectorAll("img").forEach((image) => {
    const value = image.getAttribute("src") ?? "";
    if (value && !isEmbeddedImageSource(value) && !isRemoteImageSource(value)) urls.add(value);
  });
  return [...urls];
}

export function chunkPreviewImageSources(sources: string[], size = 32): string[][] {
  if (!Number.isInteger(size) || size < 1) throw new RangeError("Preview image batch size must be positive.");
  const batches: string[][] = [];
  for (let offset = 0; offset < sources.length; offset += size) {
    batches.push(sources.slice(offset, offset + size));
  }
  return batches;
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

function imagePlaceholder(
  document: Document,
  image: HTMLImageElement,
  result: PreviewImageResult | undefined,
  pending: boolean
): HTMLElement {
  const placeholder = document.createElement("span");
  const alt = image.getAttribute("alt")?.trim() ?? "";
  const source = image.getAttribute("src") ?? "";
  placeholder.className = `preview-image-error${hasDeclaredImageSize(image) ? " declared-size" : " default-size"}`;
  placeholder.setAttribute("role", "img");
  placeholder.setAttribute("aria-label", alt ? `${alt}：${pending ? "正在读取图片" : "图片不可用"}` : pending ? "正在读取图片" : "图片不可用");
  placeholder.dataset.imageSource = source;
  copyImageDimensions(image, placeholder);

  const heading = document.createElement("strong");
  heading.textContent = pending ? "正在读取图片…" : alt ? `图片不可用：${alt}` : "图片不可用";
  const reason = document.createElement("span");
  reason.className = "preview-image-reason";
  reason.textContent = pending ? "正在检查图片返回的实际内容。" : result?.message ?? "图片尚未成功解析。";
  const address = document.createElement("code");
  address.textContent = source;
  placeholder.append(heading, reason, address);
  if (!pending) {
    const retry = document.createElement("span");
    retry.className = "preview-image-retry";
    retry.setAttribute("role", "button");
    retry.setAttribute("tabindex", "0");
    retry.dataset.previewImageRetry = "true";
    retry.textContent = "重新加载";
    placeholder.append(retry);
  }
  return placeholder;
}

export function replacePreviewImageWithPlaceholder(image: HTMLImageElement, message: string) {
  const result: PreviewImageResult = {
    originalSource: image.dataset.imageSource || image.getAttribute("src") || "未知地址",
    state: "unavailable",
    failureKind: "notImage",
    message
  };
  image.replaceWith(imagePlaceholder(document, image, result, false));
}

function hasDeclaredImageSize(image: HTMLImageElement) {
  return Boolean(
    image.getAttribute("width")
    || image.getAttribute("height")
    || sanitizePlaceholderSizeStyle(image.getAttribute("style") ?? "")
  );
}

function copyImageDimensions(image: HTMLImageElement, placeholder: HTMLElement) {
  const declarations: string[] = [];
  const width = image.getAttribute("width");
  const height = image.getAttribute("height");
  const safeStyle = sanitizePlaceholderSizeStyle(image.getAttribute("style") ?? "");
  if (safeStyle) declarations.push(safeStyle);
  if (width && /^\d+(?:\.\d+)?$/.test(width)) declarations.push(`width: ${Math.min(2000, Number(width))}px`);
  if (height && /^\d+(?:\.\d+)?$/.test(height)) declarations.push(`height: ${Math.min(2000, Number(height))}px`);
  if (declarations.length) placeholder.setAttribute("style", declarations.join("; "));
}

function sanitizePlaceholderSizeStyle(value: string): string {
  const allowed = new Set(["width", "height", "min-width", "min-height", "max-width", "max-height"]);
  return sanitizeInlineStyle(value)
    .split(";")
    .map((declaration) => declaration.trim())
    .filter(Boolean)
    .filter((declaration) => {
      const [property, cssValue = ""] = declaration.split(/:(.*)/s).map((part) => part.trim());
      if (!allowed.has(property)) return false;
      if (cssValue === "auto") return true;
      const match = cssValue.match(/^(\d+(?:\.\d+)?)(px|%)$/);
      if (!match) return false;
      return Number(match[1]) <= (match[2] === "%" ? 100 : 2000);
    })
    .join("; ");
}

function isEmbeddedImageSource(value: string) {
  return /^data:image\/(?:png|jpeg|gif|webp);base64,/i.test(value)
    || value.startsWith("blob:")
    || value.startsWith("hlex-asset:")
    || value.startsWith("http://hlex-asset.localhost/");
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
    return (parsed.protocol === "https:" || parsed.protocol === "http:")
      && parsed.username === ""
      && parsed.password === "";
  } catch {
    return false;
  }
}

export function isRemoteImageSource(value: string): boolean {
  try {
    const parsed = new URL(value);
    return (parsed.protocol === "https:" || parsed.protocol === "http:")
      && parsed.username === ""
      && parsed.password === "";
  } catch {
    return false;
  }
}
