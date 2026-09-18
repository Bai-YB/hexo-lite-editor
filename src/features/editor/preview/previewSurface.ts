import type { PreviewBlock } from "$shared/markdown/previewBlocks";

export interface PreviewSurfaceInput {
  blocks: PreviewBlock[];
  /** Sanitized embedded stylesheet texts, in document order. */
  styles: string[];
  /** Bumped once per render commit; coordinates only when it changes. */
  token: number;
}

interface LiveBlock {
  key: string;
  nodes: Node[];
}

/**
 * Keyed, in-place reconciliation of the preview article: unchanged blocks
 * keep their exact DOM nodes (image loading state and layout survive), only
 * the changed span between the common prefix and suffix is replaced. Blocks
 * are inserted without wrapper elements so descendant and sibling CSS
 * selectors behave identically to the whole-document markup.
 */
export function previewSurface(node: HTMLElement, input: PreviewSurfaceInput) {
  let live: LiveBlock[] = [];
  let token = -1;

  function parseNodes(html: string): Node[] {
    const template = node.ownerDocument.createElement("template");
    template.innerHTML = html;
    return [...template.content.childNodes];
  }

  function commit(blocks: PreviewBlock[]) {
    let prefix = 0;
    while (prefix < live.length && prefix < blocks.length && live[prefix].key === blocks[prefix].key) prefix += 1;
    let suffix = 0;
    while (
      suffix < live.length - prefix &&
      suffix < blocks.length - prefix &&
      live[live.length - 1 - suffix].key === blocks[blocks.length - 1 - suffix].key
    ) suffix += 1;

    const suffixStartOld = live.length - suffix;
    let anchor: Node | null = null;
    for (let index = suffixStartOld; index < live.length && !anchor; index += 1) {
      anchor = live[index].nodes[0] ?? null;
    }
    for (const removed of live.slice(prefix, suffixStartOld)) {
      for (const child of removed.nodes) child.parentNode?.removeChild(child);
    }
    const next: LiveBlock[] = live.slice(0, prefix);
    for (const block of blocks.slice(prefix, blocks.length - suffix)) {
      const nodes = parseNodes(block.html);
      for (const child of nodes) node.insertBefore(child, anchor);
      next.push({ key: block.key, nodes });
    }
    next.push(...live.slice(suffixStartOld));
    live = next;
  }

  function apply(inputValue: PreviewSurfaceInput) {
    if (inputValue.token === token) return;
    token = inputValue.token;
    syncStyles(inputValue.styles.join("\n"));
    commit(inputValue.blocks);
  }

  let styleNode: HTMLStyleElement | null = null;
  function syncStyles(text: string) {
    if (!text) {
      styleNode?.remove();
      styleNode = null;
      return;
    }
    if (!styleNode) {
      styleNode = node.ownerDocument.createElement("style");
      styleNode.dataset.markdownStyle = "true";
      node.insertBefore(styleNode, node.firstChild);
    }
    if (styleNode.textContent !== text) styleNode.textContent = text;
  }

  apply(input);
  return { update: (next: PreviewSurfaceInput) => apply(next) };
}
