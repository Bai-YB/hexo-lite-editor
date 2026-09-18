import DOMPurify from "dompurify";
import type { PreviewImageResult } from "$shared/types/app";
import {
  PREVIEW_SANITIZE_OPTIONS,
  finalizeRenderedContainer,
  prepareRenderedDocument,
  renderTokenizedBlock,
  tokenizeWithSourceLines
} from "./safeMarkdown";

export interface PreviewBlock {
  /** Content identity (source hash + image states); stable across line shifts. */
  key: string;
  /** Sanitized HTML for this block only (no <style> nodes). */
  html: string;
}

export interface PreviewBlocksResult {
  blocks: PreviewBlock[];
  imageSources: string[];
  styles: string[];
  reused: number;
}

interface CachedBlock {
  html: string;
  imageSources: string[];
  styles: string[];
}

const imageSyntaxRe = /!\[[^\]\r\n]*\]\(\s*([^\s)]+)/g;
const htmlImageSrcRe = /<img[^>]*\bsrc\s*=\s*["']([^"']+)["']/gi;

export class PreviewBlockCache {
  private readonly entries = new Map<string, CachedBlock>();

  get(key: string): CachedBlock | undefined {
    return this.entries.get(key);
  }

  set(key: string, entry: CachedBlock) {
    this.entries.set(key, entry);
  }

  prune(liveKeys: Set<string>) {
    for (const key of this.entries.keys()) {
      if (!liveKeys.has(key)) this.entries.delete(key);
    }
  }

  clear() {
    this.entries.clear();
  }
}

/**
 * Renders the document one top-level block at a time and reuses cached
 * sanitized HTML for blocks whose source text (and image states they read)
 * did not change. Returns null when the document contains raw HTML blocks,
 * whose parse is cross-block stateful — the caller must fall back to the
 * whole-document pipeline there.
 */
export function renderPreviewBlocks(
  source: string,
  imageResults: Record<string, PreviewImageResult>,
  imagePending: boolean,
  cache: PreviewBlockCache,
  ownerDocument: Document
): PreviewBlocksResult | null {
  const { tokens, renderer, env, offset } = tokenizeWithSourceLines(source);
  if (tokens.some((token) => token.type === "html_block")) return null;

  const lines = source.split("\n");
  const groups: Array<{ tokens: typeof tokens; startLine: number; endLine: number; mapped: boolean }> = [];
  let index = 0;
  while (index < tokens.length) {
    const start = index;
    let depth = 0;
    do {
      depth += tokens[index].nesting;
      index += 1;
    } while (depth > 0);
    const group = tokens.slice(start, index);
    const mapped = group.find((token) => token.map)?.map;
    groups.push({
      tokens: group,
      mapped: Boolean(mapped),
      startLine: mapped ? mapped[0] + offset + 1 : start + 1,
      endLine: mapped ? mapped[1] + offset : index
    });
  }

  const blocks: PreviewBlock[] = [];
  const imageSources = new Set<string>();
  const styles: string[] = [];
  const liveKeys = new Set<string>();
  let reused = 0;

  for (const group of groups) {
    const rawText = group.mapped
      ? lines.slice(group.startLine - 1, group.endLine).join("\n")
      : null;
    const key = buildKey(rawText, group, renderer, env, imageResults, imagePending);
    liveKeys.add(key);
    let entry = cache.get(key);
    if (!entry) {
      entry = renderBlockEntry(group, renderer, env, imageResults, imagePending, ownerDocument);
      cache.set(key, entry);
    } else {
      reused += 1;
    }
    for (const imageSource of entry.imageSources) imageSources.add(imageSource);
    styles.push(...entry.styles);
    blocks.push({ key, html: entry.html });
  }

  cache.prune(liveKeys);
  return { blocks, imageSources: [...imageSources], styles, reused };
}

function buildKey(
  rawText: string | null,
  group: { tokens: ReturnType<typeof tokenizeWithSourceLines>["tokens"] },
  renderer: ReturnType<typeof tokenizeWithSourceLines>["renderer"],
  env: Record<string, unknown>,
  imageResults: Record<string, PreviewImageResult>,
  imagePending: boolean
): string {
  const content = rawText ?? renderTokenizedBlock({ renderer, env }, group.tokens);
  let signature = "";
  if (rawText !== null) {
    const refs: string[] = [];
    for (const match of rawText.matchAll(imageSyntaxRe)) refs.push(match[1]);
    for (const match of rawText.matchAll(htmlImageSrcRe)) refs.push(match[1]);
    for (const ref of refs.sort()) {
      const result = imageResults[ref];
      signature += `\u0000${ref}=${result?.state ?? ""}:${result?.previewUrl ?? ""}`;
    }
  }
  return hashPreviewText(content) + (signature ? `|${hashPreviewText(signature)}` : "") + (imagePending ? "|p" : "");
}

function renderBlockEntry(
  group: { tokens: ReturnType<typeof tokenizeWithSourceLines>["tokens"] },
  renderer: ReturnType<typeof tokenizeWithSourceLines>["renderer"],
  env: Record<string, unknown>,
  imageResults: Record<string, PreviewImageResult>,
  imagePending: boolean,
  ownerDocument: Document
): CachedBlock {
  const rendered = renderTokenizedBlock({ renderer, env }, group.tokens);
  const parsed = new DOMParser().parseFromString(rendered, "text/html");
  const { imageSources, embeddedStyles } = prepareRenderedDocument(parsed, imageResults, imagePending);
  const fragment = DOMPurify.sanitize(parsed.body.innerHTML, {
    ...PREVIEW_SANITIZE_OPTIONS,
    RETURN_DOM_FRAGMENT: true,
    NAMESPACE: "http://www.w3.org/1999/xhtml"
  }) as unknown as DocumentFragment;
  const template = ownerDocument.createElement("template");
  let child = fragment.firstChild;
  while (child) {
    const next = child.nextSibling;
    fragment.removeChild(child);
    template.content.appendChild(ownerDocument.importNode(child, true));
    child = next;
  }
  finalizeRenderedContainer(template.content);
  return {
    html: template.innerHTML,
    imageSources: [...imageSources],
    styles: embeddedStyles
  };
}

export function hashPreviewText(text: string) {
  let h1 = 0xdeadbeef;
  let h2 = 0x41c6ce57;
  for (let i = 0; i < text.length; i += 1) {
    const ch = text.charCodeAt(i);
    h1 = Math.imul(h1 ^ ch, 2654435761);
    h2 = Math.imul(h2 ^ ch, 1597334677);
  }
  h1 = Math.imul(h1 ^ (h1 >>> 16), 2246822507) ^ Math.imul(h2 ^ (h2 >>> 13), 3266489909);
  h2 = Math.imul(h2 ^ (h2 >>> 16), 2246822507) ^ Math.imul(h1 ^ (h1 >>> 13), 3266489909);
  return `${(h2 >>> 0).toString(36)}${(h1 >>> 0).toString(36)}`;
}
