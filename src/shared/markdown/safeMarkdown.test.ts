import { describe, expect, it } from "vitest";
import {
  isSafeExternalLink,
  isSafeImageSource,
  renderSafeMarkdown,
  rewriteLocalImage,
  stripFrontMatter
} from "./safeMarkdown";

describe("safe markdown", () => {
  it("escapes raw HTML and removes dangerous schemes", () => {
    const html = renderSafeMarkdown(
      '<script>alert(1)</script>\n[bad](javascript:alert(1))\n![x](javascript:alert(2))'
    );
    expect(html).not.toContain("<script");
    expect(html).not.toContain('href="javascript:');
    expect(html).not.toContain('src="javascript:');
    expect(html).toContain("&lt;script&gt;");
  });

  it("keeps controlled links and image protocols", () => {
    const html = renderSafeMarkdown("[site](https://example.com) ![local](hlex-asset://localhost/id)");
    expect(html).toContain('data-external-href="https://example.com"');
    expect(html).toContain("hlex-asset://localhost/id");
    expect(isSafeExternalLink("https://example.com")).toBe(true);
    expect(isSafeExternalLink("file:///secret")).toBe(false);
    expect(isSafeImageSource("http://example.com/a.png")).toBe(false);
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
  });
});
