export interface SourceAnchor {
  line: number;
  top: number;
}

export const SCROLL_ANCHOR_INSET = 16;

/** Piecewise interpolation is local to a Markdown block, never the whole page. */
export function previewTopForSourceLine(anchors: SourceAnchor[], line: number): number {
  return interpolate(anchors, line, "line", "top");
}

export function sourceLineForPreviewTop(anchors: SourceAnchor[], top: number): number {
  return interpolate(anchors, top, "top", "line");
}

function interpolate(anchors: SourceAnchor[], value: number, from: keyof SourceAnchor, to: keyof SourceAnchor): number {
  if (!anchors.length) return from === "line" ? 0 : 1;
  if (value <= anchors[0][from]) return anchors[0][to];
  for (let index = 1; index < anchors.length; index += 1) {
    const previous = anchors[index - 1];
    const next = anchors[index];
    if (value > next[from]) continue;
    const span = next[from] - previous[from];
    if (span <= 0) return next[to];
    return previous[to] + (next[to] - previous[to]) * (value - previous[from]) / span;
  }
  return anchors[anchors.length - 1][to];
}

export function collectPreviewAnchors(container: HTMLElement): SourceAnchor[] {
  const origin = container.getBoundingClientRect().top + container.clientTop - container.scrollTop;
  const elements = [...container.querySelectorAll<HTMLElement>("[data-source-line]")];
  const starts = elements.map((element) => ({
    line: Number(element.dataset.sourceLine),
    top: element.getBoundingClientRect().top - origin
  }));
  const candidates: SourceAnchor[] = [];
  for (let index = 0; index < elements.length; index += 1) {
    const element = elements[index];
    const start = starts[index];
    if (!Number.isInteger(start.line) || start.line < 1) continue;
    candidates.push(start);
    const end = Number(element.dataset.sourceEnd);
    // Parent list/blockquote ends must not compete with their inner paragraphs.
    if (Number.isInteger(end) && end > start.line && !element.querySelector("[data-source-line]")) {
      candidates.push({ line: end, top: element.getBoundingClientRect().bottom - origin });
    }
  }
  // Actual starts win over an adjacent block's end (including collapsed margins).
  const byLine = new Map(candidates.map((anchor) => [anchor.line, anchor]));
  for (const start of starts) if (Number.isInteger(start.line) && start.line > 0) byLine.set(start.line, start);
  const ordered = [...byLine.values()].sort((left, right) => left.line - right.line);
  return ordered.reduce<SourceAnchor[]>((result, anchor) => {
    // Floats, collapsed details and user HTML may be visually out of source order.
    // Skip those points and use the nearest surrounding monotonic anchors.
    if (!result.length || anchor.top > result[result.length - 1].top + 0.1) result.push(anchor);
    return result;
  }, []);
}

/** Only the pane receiving user input may drive the other pane. */
export class ScrollSyncOwner {
  current: "editor" | "preview" = "editor";
  claim(pane: "editor" | "preview") { this.current = pane; }
  canDrive(pane: "editor" | "preview") { return this.current === pane; }
}
