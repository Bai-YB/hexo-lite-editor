import { describe, expect, it } from "vitest";
import {
  isSafeExternalLink,
  isSafeImageSource,
  chunkPreviewImageSources,
  extractPreviewImageSources,
  renderSafeMarkdown,
  rewriteLocalImage,
  sanitizeInlineStyle,
  stripFrontMatter,
} from "./safeMarkdown";

describe("safe markdown", () => {
  it("renders safe raw HTML while removing executable content", () => {
    const html = renderSafeMarkdown(
      '<details open><summary>说明</summary><strong style="color: red; position: fixed" onclick="alert(1)">正文</strong></details>\n<script>alert(1)</script>\n<iframe src="https://example.com"></iframe>'
    );
    expect(html).toContain("<details open");
    expect(html).toContain("<summary>说明</summary>");
    expect(html).toContain("color: red");
    expect(html).not.toContain("position");
    expect(html).not.toContain("onclick");
    expect(html).not.toContain("<script");
    expect(html).not.toContain("<iframe");
  });

  it("keeps fenced HTML as source code", () => {
    const html = renderSafeMarkdown("```html\n<figure>源码</figure>\n```");
    expect(html).toContain("&lt;figure&gt;源码&lt;/figure&gt;");
    expect(html).not.toContain("<figure>源码</figure>");
  });

  it("keeps controlled links and image protocols", () => {
    const html = renderSafeMarkdown("[site](https://example.com) ![local](hlex-asset://localhost/id)");
    expect(html).toContain('data-external-href="https://example.com"');
    expect(html).toContain("hlex-asset://localhost/id");
    expect(isSafeExternalLink("https://example.com")).toBe(true);
    expect(isSafeExternalLink("file:///secret")).toBe(false);
    expect(isSafeImageSource("http://example.com/a.png")).toBe(true);
    expect(isSafeImageSource("https://example.com/a.png")).toBe(true);
  });

  it("strips only complete front matter blocks", () => {
    expect(stripFrontMatter("---\ntitle: 示例\n---\n正文")).toBe("正文");
    expect(stripFrontMatter("---\ntitle: [\n正文")).toContain("title");
  });

  it("rewrites Hexo local images to session asset URLs", () => {
    const assets = {
      "/images/%E7%A4%BA%E4%BE%8B.png": {
        originalSource: "/images/%E7%A4%BA%E4%BE%8B.png",
        state: "ready" as const,
        previewUrl: "http://hlex-asset.localhost/token"
      }
    };
    expect(renderSafeMarkdown("![](/images/示例.png)", assets)).toContain(
      "http://hlex-asset.localhost/token"
    );
    expect(rewriteLocalImage("/media/posts/示例.png", {
      "/media/posts/示例.png": "http://hlex-asset.localhost/custom"
    })).toBe("http://hlex-asset.localhost/custom");
    const missing = renderSafeMarkdown("![已删除](/images/missing.png)");
    expect(missing).toContain("图片不可用");
    expect(missing).toContain('data-image-source="/images/missing.png"');
  });

  it("loads remote images directly and only resolves local sources", () => {
    const source = "![远程](https://example.com/a.png)\n<img src=\"https://example.com/b.png\" alt=\"HTML 图\">";
    expect(extractPreviewImageSources(source)).toEqual([]);
    const pending = renderSafeMarkdown(source, {}, true);
    expect(pending).toContain('src="https://example.com/a.png"');
    expect(pending).toContain('src="https://example.com/b.png"');
    expect(pending).not.toContain("正在读取图片");
  });

  it("preserves bounded placeholder dimensions and rejects oversized styles", () => {
    const html = renderSafeMarkdown(
      '<img src="/missing.png" width="320" height="180" style="max-width: 100%; width: 999999px; position: fixed">',
      { "/missing.png": { originalSource: "/missing.png", state: "unavailable", failureKind: "notFound", message: "本地图片不存在。" } }
    );
    expect(html).toContain("width: 320px");
    expect(html).toContain("height: 180px");
    expect(html).toContain("max-width: 100%");
    expect(html).not.toContain("999999");
    expect(html).not.toContain("position");
    const defaultSized = renderSafeMarkdown('<img src="/missing.png" style="position: fixed">');
    expect(defaultSized).toContain("preview-image-error default-size");
  });

  it("splits any number of preview images into backend-safe batches", () => {
    const sources = Array.from({ length: 65 }, (_, index) => `/images/${index}.png`);
    expect(chunkPreviewImageSources(sources).map((batch) => batch.length)).toEqual([32, 32, 1]);
    expect(chunkPreviewImageSources(sources).flat()).toEqual(sources);
  });

  it("supports common semantic HTML attributes while keeping active content blocked", () => {
    const html = renderSafeMarkdown(
      '<section lang="zh-CN" dir="ltr"><p><u>下划线</u><wbr><dfn title="定义">术语</dfn></p>' +
      '<meter min="0" max="10" low="3" high="8" optimum="7" value="6"></meter>' +
      '<progress max="100" value="40"></progress>' +
      '<table><tfoot><tr><td abbr="合计">总计</td></tr></tfoot></table></section>'
    );
    expect(html).toContain('<section lang="zh-CN" dir="ltr">');
    expect(html).toContain("<u>下划线</u><wbr>");
    expect(html).toContain('min="0" max="10" low="3" high="8" optimum="7" value="6"');
    expect(html).toContain('<progress max="100" value="40"></progress>');
    expect(html).toContain("<tfoot>");
  });

  it("allows only contained inline presentation styles", () => {
    expect(sanitizeInlineStyle("display:grid; gap: 8px; position:fixed; background:url(x); z-index:999"))
      .toBe("display: grid; gap: 8px");
  });

  it("keeps safe gradient text declarations as one visible effect", () => {
    const html = renderSafeMarkdown(
      '<span style="font-weight:800; background:linear-gradient(to right, #ff4b2b, #4facfe); ' +
      '-webkit-background-clip:text; -webkit-text-fill-color:transparent; color:transparent">渐变文字</span>'
    );
    expect(html).toContain("background: linear-gradient(to right, #ff4b2b, #4facfe)");
    expect(html).toContain("-webkit-background-clip: text");
    expect(html).toContain("-webkit-text-fill-color: transparent");
  });

});
