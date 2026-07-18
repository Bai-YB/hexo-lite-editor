import { describe, expect, it } from "vitest";
import { isSafeThemePreviewUrl, previewStateLabel } from "./previewModel";

describe("preview model", () => {
  it("maps every preview state to quiet status text", () => {
    expect(previewStateLabel("starting")).toBe("启动中");
    expect(previewStateLabel("running")).toBe("运行中");
    expect(previewStateLabel("stopping")).toBe("停止中");
    expect(previewStateLabel("error")).toBe("异常");
    expect(previewStateLabel("stopped")).toBe("已停止");
  });

  it("only embeds explicit IPv4 loopback HTTP routes", () => {
    expect(isSafeThemePreviewUrl("http://127.0.0.1:4000/post/")).toBe(true);
    expect(isSafeThemePreviewUrl("http://localhost:4000/post/")).toBe(false);
    expect(isSafeThemePreviewUrl("https://example.com/post/")).toBe(false);
    expect(isSafeThemePreviewUrl("javascript:alert(1)")).toBe(false);
  });
});
