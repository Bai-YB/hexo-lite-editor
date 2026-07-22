import { describe, expect, it } from "vitest";
import {
  isSafeExternalLink,
  isSafeImageSource,
  extractRemoteImageUrls,
  renderSafeMarkdown,
  rewriteLocalImage,
  sanitizeInlineStyle,
  stripFrontMatter
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
    expect(isSafeImageSource("http://example.com/a.png")).toBe(false);
    expect(isSafeImageSource("https://example.com/a.png")).toBe(false);
  });

  it("strips only complete front matter blocks", () => {
    expect(stripFrontMatter("---\ntitle: 示例\n---\n正文")).toBe("正文");
    expect(stripFrontMatter("---\ntitle: [\n正文")).toContain("title");
  });

  it("rewrites Hexo local images to session asset URLs", () => {
    const assets = {
      "source/images/示例.png": "http://hlex-asset.localhost/token",
      "images/示例.png": "http://hlex-asset.localhost/token"
    };
    expect(rewriteLocalImage("/images/示例.png", assets)).toContain("hlex-asset.localhost");
    expect(renderSafeMarkdown("![](/images/示例.png)", assets)).toContain(
      "http://hlex-asset.localhost/token"
    );
    expect(rewriteLocalImage("/media/posts/示例.png", {
      "/media/posts/示例.png": "http://hlex-asset.localhost/custom"
    })).toBe("http://hlex-asset.localhost/custom");
  });

  it("never leaves a direct remote image in preview HTML", () => {
    const source = "![远程](https://example.com/a.png)\n<img src=\"https://example.com/b.png\" alt=\"HTML 图\">";
    expect(extractRemoteImageUrls(source)).toEqual([
      "https://example.com/a.png",
      "https://example.com/b.png"
    ]);
    const pending = renderSafeMarkdown(source, {}, {}, true);
    expect(pending).not.toContain('src="https://');
    expect(pending).toContain("正在验证图片");
    const resolved = renderSafeMarkdown(source, {}, {
      "https://example.com/a.png": "http://hlex-asset.localhost/a",
      "https://example.com/b.png": null
    });
    expect(resolved).toContain("http://hlex-asset.localhost/a");
    expect(resolved).not.toContain('src="https://');
    expect(resolved).toContain("图片不可用");
  });

  it("allows only contained inline presentation styles", () => {
    expect(sanitizeInlineStyle("display:grid; gap: 8px; position:fixed; background:url(x); z-index:999"))
      .toBe("display: grid; gap: 8px");
  });
});
