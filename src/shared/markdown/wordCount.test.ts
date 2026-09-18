import { describe, expect, it } from "vitest";
import { countWords, lineAt } from "./wordCount";

describe("countWords", () => {
  it("matches the inline implementation it replaces in EditorPage", () => {
    const samples = [
      "",
      "hello world",
      "你好，世界",
      "---\ntitle: 文章\n---\n\n正文 mixed words 123\n\nsecond paragraph",
      "don't-stop 下划线_word"
    ];
    for (const sample of samples) {
      const inline = (() => {
        const body = sample.replace(/^---[\s\S]*?---/, "");
        const chinese = body.match(/[㐀-鿿]/g)?.length ?? 0;
        const words = body.match(/[A-Za-z0-9_]+(?:[-'][A-Za-z0-9_]+)*/g)?.length ?? 0;
        return chinese + words;
      })();
      expect(countWords(sample)).toBe(inline);
    }
  });
});

describe("lineAt", () => {
  it("returns 1-based line numbers", () => {
    const content = "one\ntwo\nthree";
    expect(lineAt(content, 0)).toBe(1);
    expect(lineAt(content, 3)).toBe(1);
    expect(lineAt(content, 4)).toBe(2);
    expect(lineAt(content, 7)).toBe(2);
    expect(lineAt(content, 8)).toBe(3);
    expect(lineAt(content, 9)).toBe(3);
  });

  it("clamps out-of-range offsets", () => {
    expect(lineAt("a\nb", -5)).toBe(1);
    expect(lineAt("a\nb", 999)).toBe(2);
  });

  it("matches the slice-based implementation it replaces", () => {
    const content = "---\ntitle: x\n---\n\n# 标题\n\n段落\n下一行";
    for (const offset of [0, 5, 12, 14, content.length]) {
      expect(lineAt(content, offset)).toBe(content.slice(0, offset).split("\n").length);
    }
  });
});
