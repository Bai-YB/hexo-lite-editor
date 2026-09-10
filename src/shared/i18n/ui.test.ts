import { afterEach, describe, expect, it } from "vitest";
import { get } from "svelte/store";
import { setLanguage } from "./index";
import { translateUi, ui } from "./ui";
import { uiMessages } from "./uiMessages";
import { assetKindLabel } from "$features/image-bed/model";

const uiSources = import.meta.glob<string>(["/src/**/*.svelte", "/src/**/*.ts", "!/src/**/*.test.ts", "!/src/shared/i18n/uiMessages.ts"], {
  query: "?raw", import: "default", eager: true
});

afterEach(() => setLanguage("zh-CN"));
describe("explicit UI translations", () => {
  it("updates existing subscribers when switching Chinese to English and back", () => {
    const rendered: string[] = [];
    setLanguage("zh-CN");
    const unsubscribe = ui.subscribe((translate) => rendered.push(translate("保存")));
    setLanguage("en-US");
    setLanguage("zh-CN");
    expect(rendered).toEqual(["保存", "Save", "保存"]);
    unsubscribe();
  });
  it("preserves user names, paths, Chinese text and placeholder-like parameters", () => {
    setLanguage("en-US");
    const name = "未保存的文章/中文-{p1}.png";
    expect(get(ui)("打开 {p0} 菜单", { p0: name, p1: "must not replace" })).toBe(`Open ${name} menu`);
    expect(translateUi("二进制 · 本地 {p0} B / 远端 {p1} B", "en-US", { p0: 0, p1: 123 })).toBe("Binary · Local 0 B / Remote 123 B");
  });
  it("leaves unknown content unchanged instead of applying approximate replacements", () => {
    expect(translateUi("我的文章标题：已保存", "en-US")).toBe("我的文章标题：已保存");
    expect(translateUi("打开 {p0} 菜单", "zh-CN", { p0: "my file" })).toBe("打开 my file 菜单");
  });
  it("keeps each translation's exact set of placeholders", () => {
    for (const [source, target] of Object.entries(uiMessages)) {
      expect(target.match(/\{\w+\}/g)?.sort() ?? [], source).toEqual(source.match(/\{\w+\}/g)?.sort() ?? []);
      expect(target, source).not.toMatch(/[\u3400-\u9fff]/);
    }
  });
  it("includes every explicit source key used by the application", () => {
    for (const [file, source] of Object.entries(uiSources)) {
      for (const match of source.matchAll(/(?:\$ui|translateUi)\(\s*("(?:\\.|[^"\\])*")/g)) {
        const key: string = JSON.parse(match[1]);
        expect(Object.hasOwn(uiMessages, key), `${file}: ${key}`).toBe(true);
      }
    }
  });
  it("translates all dynamic resource type labels", () => {
    for (const kind of ["folder", "image", "archive", "document", "audio", "video", "file"] as const) {
      expect(translateUi(assetKindLabel(kind), "en-US"), kind).not.toMatch(/[\u3400-\u9fff]/);
    }
  });
});
