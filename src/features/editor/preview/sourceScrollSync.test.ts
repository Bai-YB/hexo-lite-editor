import { describe, expect, it } from "vitest";
import { collectPreviewAnchors, previewTopForSourceLine, sourceLineForPreviewTop, ScrollSyncOwner } from "./sourceScrollSync";
import { renderSafeMarkdown } from "$shared/markdown/safeMarkdown";

describe("source anchored scroll sync", () => {
  it("aligns the same paragraph despite image and code heights elsewhere", () => {
    const anchors = [{ line: 5, top: 28 }, { line: 7, top: 100 }, { line: 8, top: 1100 }, { line: 30, top: 1600 }];
    expect(previewTopForSourceLine(anchors, 8)).toBe(1100);
    expect(previewTopForSourceLine(anchors, 19)).toBe(1350);
    expect(sourceLineForPreviewTop(anchors, 1350)).toBe(19);
    expect(sourceLineForPreviewTop(anchors, 600)).toBe(7.5);
  });

  it("falls back to surrounding or nearest anchors when source has no rendered block", () => {
    const anchors = [{ line: 10, top: 30 }, { line: 20, top: 400 }];
    expect(previewTopForSourceLine(anchors, 3)).toBe(30);
    expect(previewTopForSourceLine(anchors, 100)).toBe(400);
    expect(previewTopForSourceLine(anchors, 15)).toBe(215);
    expect(previewTopForSourceLine([], 5)).toBe(0);
    expect(sourceLineForPreviewTop([], 5)).toBe(1);
  });

  it("preserves original source lines for front matter, fenced code, lists and raw HTML", () => {
    const source = "---\r\ntitle: test\r\n---\r\n# Heading\r\n\r\nParagraph\r\n\r\n```js\r\nconst x = 1;\r\n```\r\n\r\n- one\r\n- two\r\n\r\n<div>HTML</div>";
    const root = document.createElement("div");
    root.innerHTML = renderSafeMarkdown(source, {}, false, true);
    expect(root.querySelector("h1")?.getAttribute("data-source-line")).toBe("4");
    expect(root.querySelector("p")?.getAttribute("data-source-line")).toBe("6");
    expect(root.querySelector("pre")?.getAttribute("data-source-line")).toBe("8");
    expect(root.querySelector("pre")?.getAttribute("data-source-end")).toBe("11");
    expect([...root.querySelectorAll("li")].map((item) => item.getAttribute("data-source-line"))).toEqual(["12", "13"]);
    expect(root.querySelector('[data-source-line="15"]')).not.toBeNull();
    expect(root.textContent).toContain("HTML");
    expect(root.textContent).not.toContain("title: test");
  });

  it("remeasures nested element coordinates relative to the scroller after images resize", () => {
    const root = document.createElement("article");
    root.innerHTML = '<blockquote data-source-line="2"><p data-source-line="3" data-source-end="4">a</p></blockquote><p data-source-line="8" data-source-end="9">b</p>';
    root.scrollTop = 200;
    root.getBoundingClientRect = () => ({ top: 100 } as DOMRect);
    const elements = [...root.querySelectorAll<HTMLElement>("[data-source-line]")];
    let positions = [[120, 260], [140, 250], [400, 500]];
    elements.forEach((element, index) => { element.getBoundingClientRect = () => ({ top: positions[index][0], bottom: positions[index][1] } as DOMRect); });
    expect(collectPreviewAnchors(root)).toEqual([{ line: 2, top: 220 }, { line: 3, top: 240 }, { line: 4, top: 350 }, { line: 8, top: 500 }, { line: 9, top: 600 }]);
    positions = [[120, 460], [140, 450], [600, 700]];
    expect(previewTopForSourceLine(collectPreviewAnchors(root), 8)).toBe(700);
  });

  it("only permits the pane with user input to drive scroll updates", () => {
    const owner = new ScrollSyncOwner();
    expect(owner.canDrive("editor")).toBe(true);
    expect(owner.canDrive("preview")).toBe(false);
    owner.claim("preview");
    expect(owner.canDrive("editor")).toBe(false);
    expect(owner.canDrive("preview")).toBe(true);
  });
});
