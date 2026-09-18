import { describe, expect, it } from "vitest";
import type { PreviewImageResult } from "$shared/types/app";
import { renderMarkdownPreview } from "./safeMarkdown";
import { PreviewBlockCache, renderPreviewBlocks } from "./previewBlocks";

const fixtures: string[] = [
  "# 标题\n\n段落 one\n\n段落 two",
  "---\ntitle: 文章\n---\n\n# 标题\n\n开头段落。\n\n## 二级\n\n- 列表 A\n- 列表 B\n\n1. 有序一\n2. 有序二\n",
  "文字 **加粗** 与 `code`。\n\n```ts\nconst x = 1;\nconsole.log(x);\n```\n\n结尾段落。",
  "| a | b |\n| - | - |\n| 1 | 2 |\n\n后面表格的段落",
  "---\ntitle: x\n---\n\n![本地图片](images/pic.png)\n\n![远程](https://cdn.example.com/a.png)\n\n![内嵌](data:image/png;base64,AAAA)\n",
  "引用块：\n\n> 第一行引用\n> 第二行引用\n\n---\n\n分隔线之后的内容",
  "自动链接 https://example.com 与 [链接](relative/path.md)。\n\n换行前文",
  "![x](javascript:alert(1))\n\n文字 <img src=x onerror=alert(1)> 与 <b onmouseover=\"alert(2)\">粗</b> 及 [a](vbscript:msgbox)",
  "<div>独立 html 块</div>\n\n<p>另一 html 块</p>",
  "<p>段落内 <img src=\"local.png\" onerror=\"alert(1)\"> 内嵌 html 图片</p>"
];

const imageResults: Record<string, PreviewImageResult> = {
  "images/pic.png": { state: "ready", previewUrl: "https://cdn.example.com/pic.png" } as unknown as PreviewImageResult
};

function incrementalHtml(source: string, results: Record<string, PreviewImageResult>, pending: boolean) {
  const cache = new PreviewBlockCache();
  const rendered = renderPreviewBlocks(source, results, pending, cache, document);
  if (!rendered) return null;
  const styleNode = rendered.styles.length
    ? `<style data-markdown-style="true">${rendered.styles.join("\n")}</style>`
    : "";
  return { html: styleNode + rendered.blocks.map((block) => block.html).join(""), imageSources: rendered.imageSources };
}

describe("renderPreviewBlocks golden alignment", () => {
  it("produces the same sanitized HTML as the whole-document pipeline", () => {
    fixtures.forEach((source, index) => {
      const incremental = incrementalHtml(source, imageResults, false);
      if (!incremental) {
        // Documents with raw html_block tokens must fall back to the full pipeline.
        expect(source, `fixture ${index} unexpectedly fell back`).toMatch(/<div>|<p>/);
        return;
      }
      const full = renderMarkdownPreview(source, imageResults, false, true);
      expect(incremental.html, `fixture ${index}`).toBe(full.html);
      expect(incremental.imageSources, `fixture ${index} images`).toEqual(full.imageSources);
    });
  });

  it("falls back for standalone html blocks whose parse is cross-block stateful", () => {
    const cache = new PreviewBlockCache();
    expect(renderPreviewBlocks("<details>\n\n正文\n\n</details>", {}, false, cache, document)).toBeNull();
  });

  it("strips XSS payloads on the incremental path the same way", () => {
    const source = fixtures[7];
    const incremental = incrementalHtml(source, {}, false);
    expect(incremental).not.toBeNull();
    expect(incremental!.html).not.toContain("onerror");
    expect(incremental!.html).not.toContain("onmouseover");
    expect(incremental!.html).not.toMatch(/(src|href)\s*=\s*["'](?:javascript|vbscript):/i);
    expect(incremental!.html).not.toContain("<script");
    expect(incremental!.html).toBe(renderMarkdownPreview(source, {}, false, true).html);
  });

  it("reuses cached blocks when only a later block changes", () => {
    const source = "# 标题\n\n段落一内容。\n\n段落二内容。\n\n段落三内容。";
    const cache = new PreviewBlockCache();
    const first = renderPreviewBlocks(source, {}, false, cache, document);
    expect(first).not.toBeNull();
    expect(first!.reused).toBe(0);
    const edited = "# 标题\n\n段落一内容。\n\n段落二改过。新增一句。\n\n段落三内容。";
    const second = renderPreviewBlocks(edited, {}, false, cache, document);
    expect(second).not.toBeNull();
    // The heading, first and third paragraph keep their cached sanitized HTML.
    expect(second!.reused).toBe(3);
    expect(second!.blocks.map((block) => block.html).join("")).toBe(
      renderMarkdownPreview(edited, {}, false, true).html
    );
  });

  it("invalidates a block when the image results it reads change", () => {
    const source = "![本地图](images/pic.png)\n\n与图片无关的段落。";
    const cache = new PreviewBlockCache();
    const before = renderPreviewBlocks(source, {}, false, cache, document);
    const after = renderPreviewBlocks(
      source,
      { "images/pic.png": { state: "ready", previewUrl: "https://cdn.example.com/pic.png" } } as unknown as Record<string, PreviewImageResult>,
      false,
      cache,
      document
    );
    expect(before && after).toBeTruthy();
    const beforeHtml = before!.blocks.map((block) => block.html).join("");
    const afterHtml = after!.blocks.map((block) => block.html).join("");
    expect(afterHtml).toContain("https://cdn.example.com/pic.png");
    expect(afterHtml).not.toBe(beforeHtml);
    expect(after!.reused).toBe(1); // the plain paragraph stays cached
  });
});
