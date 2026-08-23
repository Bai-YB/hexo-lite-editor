import { beforeEach, describe, expect, it } from "vitest";
import { setLanguage } from "./index";
import { translateLegacyDom, translateLegacyUiText } from "./legacyDom";

describe("legacy UI translation", () => {
  beforeEach(() => { document.body.innerHTML = ""; setLanguage("en-US"); });

  it("translates exact and parameterized application text", () => {
    expect(translateLegacyUiText("图床")).toBe("Images");
    expect(translateLegacyUiText("打开 demo 菜单")).toBe("Open demo menu");
  });

  it("translates DOM text and accessibility labels but preserves editor content", () => {
    document.body.innerHTML = '<button aria-label="关闭通知">取消</button><div class="cm-editor">用户中文正文</div>';
    translateLegacyDom();
    expect(document.querySelector("button")?.textContent).toBe("Cancel");
    expect(document.querySelector("button")?.getAttribute("aria-label")).toBe("Dismiss notification");
    expect(document.querySelector(".cm-editor")?.textContent).toBe("用户中文正文");
  });
});
