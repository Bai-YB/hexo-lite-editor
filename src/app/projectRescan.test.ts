import { describe, expect, it } from "vitest";
import { EditorSessionStore } from "$features/editor/EditorSessionStore";
import type { ArticleSummary, DocumentSnapshot, ProjectSessionView } from "$shared/types/app";
import { reconcileProjectRescan } from "./projectRescan";

const original: DocumentSnapshot = { projectId: "A", articleId: "post", sessionGeneration: 1, content: "original", revision: 0 };
const article = { articleId: "post" } as ArticleSummary;
function setup() {
  const store = new EditorSessionStore(async (request) => ({ articleId: request.articleId, acceptedRevision: request.revision, savedAt: "now" }));
  store.load(original);
  let session: ProjectSessionView = { projectId: "A", generation: 1, name: "A", displayPath: "A", warnings: [] };
  let finish!: (snapshot: DocumentSnapshot) => void;
  const pending = new Promise<DocumentSnapshot>((resolve) => { finish = resolve; });
  const context = { store, getSession: () => session, accept: (value: { generation: number }) => { session = { ...session, generation: value.generation }; }, loadDocument: () => pending };
  return { store, context, setSession: (value: ProjectSessionView) => { session = value; }, finish: () => finish({ ...original, sessionGeneration: 2, content: "remote" }) };
}
const rescan = { projectId: "A", previousGeneration: 1, generation: 2, articles: [article] };

describe("sync rescan and current editing", () => {
  it("loads a clean unchanged document", async () => {
    const test = setup();
    const result = reconcileProjectRescan(rescan, test.context);
    test.finish();
    expect(await result).toBe("refreshed");
    expect(test.store.getState().content).toBe("remote");
  });
  it("preserves typing while the reload is pending and requires a decision before saving", async () => {
    const test = setup();
    const result = reconcileProjectRescan(rescan, test.context);
    test.store.update("new local work");
    test.finish();
    expect(await result).toBe("changed");
    expect(test.store.getState().content).toBe("new local work");
    expect(test.store.getState().externalChange).toBe("changed");
    await expect(test.store.save()).rejects.toThrow();
  });
  it("ignores a response after the same article is reopened", async () => {
    const test = setup();
    const result = reconcileProjectRescan(rescan, test.context);
    test.store.clear();
    test.store.load({ ...original, sessionGeneration: 2, content: "reopened" });
    test.finish();
    expect(await result).toBe("ignored");
    expect(test.store.getState().content).toBe("reopened");
  });
  it("ignores a result after switching projects", async () => {
    const test = setup();
    const result = reconcileProjectRescan(rescan, test.context);
    test.setSession({ projectId: "B", generation: 3, name: "B", displayPath: "B", warnings: [] });
    test.store.load({ ...original, projectId: "B", sessionGeneration: 3, content: "B work" });
    test.finish();
    expect(await result).toBe("ignored");
    expect(test.store.getState().content).toBe("B work");
  });
  it("preserves remotely deleted content as recoverable work", async () => {
    const test = setup();
    expect(await reconcileProjectRescan({ ...rescan, articles: [] }, test.context)).toBe("deleted");
    expect(test.store.getState().content).toBe("original");
    expect(test.store.getState().externalChange).toBe("deleted");
    expect(test.store.hasDirty()).toBe(true);
    await expect(test.store.save()).rejects.toThrow();
  });
  it("rejects stale generations before touching state", async () => {
    const test = setup();
    expect(await reconcileProjectRescan({ ...rescan, previousGeneration: 0 }, test.context)).toBe("ignored");
    expect(test.store.getState().snapshot?.sessionGeneration).toBe(1);
  });
});
