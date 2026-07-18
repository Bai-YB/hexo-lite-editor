import { describe, expect, it } from "vitest";
import { EditorSessionStore } from "./EditorSessionStore";

const snapshot = {
  projectId: "project",
  articleId: "article",
  content: "one",
  revision: 0,
  sessionGeneration: 1
};

describe("EditorSessionStore", () => {
  it("keeps newer edits dirty when an older save finishes", async () => {
    let resolveSave!: (value: { articleId: string; acceptedRevision: number; savedAt: string }) => void;
    const store = new EditorSessionStore(
      () => new Promise((resolve) => (resolveSave = resolve))
    );
    store.load(snapshot);
    store.update("two");
    const save = store.save();
    await new Promise((resolve) => setTimeout(resolve, 0));
    store.update("three");
    resolveSave({ articleId: "article", acceptedRevision: 1, savedAt: "now" });
    await save;
    expect(store.getState().content).toBe("three");
    expect(store.getState().dirty).toBe(true);
    expect(store.getState().persistedRevision).toBe(1);
  });

  it("serializes repeated saves", async () => {
    const calls: number[] = [];
    const store = new EditorSessionStore(async (request) => {
      calls.push(request.revision);
      return {
        articleId: request.articleId,
        acceptedRevision: request.revision,
        savedAt: String(request.revision)
      };
    });
    store.load(snapshot);
    store.update("two");
    await store.save();
    store.update("three");
    await store.save();
    expect(calls).toEqual([1, 2]);
    expect(store.getState().dirty).toBe(false);
  });

  it("inserts an image at the document end before a cursor has been placed", () => {
    const store = new EditorSessionStore(async (request) => ({
      articleId: request.articleId,
      acceptedRevision: request.revision,
      savedAt: "now"
    }));
    store.load({ ...snapshot, content: "---\ntitle: Test\n---\n\nBody" });
    expect(store.insertMarkdown("![cover](/images/cover.png)")).toBe(true);
    expect(store.getState().content).toBe(
      "---\ntitle: Test\n---\n\nBody\n![cover](/images/cover.png)"
    );
  });
});
