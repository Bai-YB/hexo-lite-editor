// @vitest-environment jsdom
import { describe, expect, it } from "vitest";
import { previewSurface, type PreviewSurfaceInput } from "./previewSurface";
import type { PreviewBlock } from "$shared/markdown/previewBlocks";

function article() {
  const node = document.createElement("article");
  document.body.append(node);
  return node;
}

function input(partial: Partial<PreviewSurfaceInput> & { blocks: PreviewBlock[] }): PreviewSurfaceInput {
  return {
    blocks: partial.blocks,
    styles: partial.styles ?? [],
    token: partial.token ?? 1
  };
}

function block(key: string, html: string): PreviewBlock {
  return { key, html };
}

describe("previewSurface", () => {
  it("parses initial blocks without wrapper elements", () => {
    const node = article();
    previewSurface(node, input({
      blocks: [block("a", "<p>one</p>"), block("b", "<h2 data-source-line=\"3\">Two</h2>")]
    }));
    expect(node.children).toHaveLength(2);
    expect(node.firstElementChild?.tagName).toBe("P");
    expect(node.querySelector("h2[data-source-line]")?.textContent).toBe("Two");
  });

  it("keeps prefix and suffix node identities and replaces only the changed middle", () => {
    const node = article();
    const first = block("a", "<p>one</p>");
    const middle = block("m1", "<p>two</p>");
    const last = block("z", "<p>nine</p>");
    const surface = previewSurface(node, input({ blocks: [first, middle, last] }));
    const [prefixNode, oldMiddle, suffixNode] = [...node.children];

    surface.update(input({
      token: 2,
      blocks: [first, block("m2", "<p>changed</p>"), block("m3", "<p>extra</p>"), last]
    }));

    expect(node.children[0]).toBe(prefixNode);
    expect(node.children[node.children.length - 1]).toBe(suffixNode);
    expect(oldMiddle.isConnected).toBe(false);
    expect([...node.children].map((child) => child.textContent)).toEqual(["one", "changed", "extra", "nine"]);
  });

  it("reuses image nodes when their block key is unchanged", () => {
    const node = article();
    const image = block("img", '<p><img src="a.png" alt="x"></p>');
    const surface = previewSurface(node, input({ blocks: [image] }));
    const img = node.querySelector("img");

    surface.update(input({ token: 2, blocks: [image, block("tail", "<p>appended</p>")] }));

    expect(node.querySelector("img")).toBe(img);
    expect(img?.isConnected).toBe(true);
    expect(node.lastElementChild?.textContent).toBe("appended");
  });

  it("drops all nodes when blocks become empty", () => {
    const node = article();
    const surface = previewSurface(node, input({ blocks: [block("a", "<p>one</p>"), block("b", "<p>two</p>")] }));
    surface.update(input({ token: 2, blocks: [] }));
    expect(node.childNodes).toHaveLength(0);
  });

  it("skips reconciliation when the token is unchanged", () => {
    const node = article();
    const surface = previewSurface(node, input({ blocks: [block("a", "<p>one</p>")] }));
    const paragraph = node.firstElementChild;
    surface.update(input({ token: 1, blocks: [block("b", "<p>other</p>")] }));
    expect(node.firstElementChild).toBe(paragraph);
    expect(paragraph?.textContent).toBe("one");
  });

  it("manages a single style node at the top and removes it when styles clear", () => {
    const node = article();
    const surface = previewSurface(node, input({
      blocks: [block("a", "<p>one</p>")],
      styles: ["p { color: red }"]
    }));
    const style = node.querySelector("style");
    expect(style?.dataset.markdownStyle).toBe("true");
    expect(style?.textContent).toBe("p { color: red }");
    expect(style).toBe(node.firstElementChild);

    surface.update(input({ token: 2, blocks: [block("a", "<p>one</p>")], styles: ["p { color: blue }"] }));
    expect(node.querySelectorAll("style")).toHaveLength(1);
    expect(node.querySelector("style")).toBe(style);
    expect(style?.textContent).toBe("p { color: blue }");

    surface.update(input({ token: 3, blocks: [block("a", "<p>one</p>")], styles: [] }));
    expect(node.querySelector("style")).toBeNull();
  });

  it("handles multi-node blocks as one unit during replacement", () => {
    const node = article();
    const multi = block("m", "<ul><li>1</li></ul><p>note</p>");
    const surface = previewSurface(node, input({ blocks: [multi] }));
    expect(node.children).toHaveLength(2);

    surface.update(input({ token: 2, blocks: [block("m2", "<blockquote>single</blockquote>")] }));
    expect(node.children).toHaveLength(1);
    expect(node.firstElementChild?.tagName).toBe("BLOCKQUOTE");
  });
});
