import { describe, expect, it, vi } from "vitest";
import {
  escapeXml,
  fetchStargazers,
  normalizeStargazers,
  renderStarHistorySvg
} from "./generate-star-history.mjs";

describe("star history generator", () => {
  it("renders an honest and stable zero-star state", () => {
    const first = renderStarHistorySvg("Bai-YB/hexo-lite-editor", []);
    const second = renderStarHistorySvg("Bai-YB/hexo-lite-editor", []);
    expect(first).toBe(second);
    expect(first).toContain('data-state="empty"');
    expect(first).toContain("0 stars");
    expect(first).toContain("currently has no GitHub stars");
    expect(first).not.toContain("<polyline");
  });

  it("sorts timestamps and accumulates stars by UTC day", () => {
    expect(normalizeStargazers([
      { starred_at: "2026-07-03T01:00:00Z" },
      { starred_at: "2026-07-01T22:00:00Z" },
      { starred_at: "2026-07-01T02:00:00Z" }
    ])).toEqual([
      { date: "2026-07-01", stars: 2 },
      { date: "2026-07-03", stars: 3 }
    ]);
  });

  it("follows GitHub pagination and sends Basic-safe request headers", async () => {
    const fetchMock = vi.fn()
      .mockResolvedValueOnce({
        ok: true,
        json: async () => [{ starred_at: "2026-07-01T00:00:00Z" }],
        headers: new Headers({ link: '<https://api.github.com/repos/owner/repo/stargazers?per_page=100&page=2>; rel="next"' })
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => [{ starred_at: "2026-07-02T00:00:00Z" }],
        headers: new Headers()
      });
    const items = await fetchStargazers("owner/repo", { token: "test-secret", fetchImpl: fetchMock });
    expect(items).toHaveLength(2);
    expect(fetchMock).toHaveBeenCalledTimes(2);
    expect(fetchMock.mock.calls[0][1].headers.Authorization).toBe("Bearer test-secret");
    expect(fetchMock.mock.calls[1][0]).toContain("page=2");
  });

  it("escapes labels and never serializes credentials", () => {
    expect(escapeXml('owner/<repo>&"')).toBe("owner/&lt;repo&gt;&amp;&quot;");
    const svg = renderStarHistorySvg("owner/<repo>", [{ starred_at: "2026-07-01T00:00:00Z" }]);
    expect(svg).toContain("owner/&lt;repo&gt;");
    expect(svg).not.toContain("test-secret");
  });
});
