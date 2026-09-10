import { describe, expect, it } from "vitest";
import {
  EditorSessionStore,
  findPendingEditorImages,
  replaceMarkdownImageUrl
} from "./EditorSessionStore";

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

  it("rebases a dirty draft onto a rescanned project generation", () => {
    const store = new EditorSessionStore(async (request) => ({
      articleId: request.articleId,
      acceptedRevision: request.revision,
      savedAt: "now"
    }));
    store.load(snapshot);
    store.update("unsaved draft");
    store.rebaseSessionGeneration(2);

    const state = store.getState();
    expect(state.snapshot?.sessionGeneration).toBe(2);
    expect(state.content).toBe("unsaved draft");
    expect(state.dirty).toBe(true);
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

  it("discards only edits after the last accepted save", async () => {
    let finish!: (value: { articleId: string; acceptedRevision: number; savedAt: string }) => void;
    const store = new EditorSessionStore(() => new Promise(resolve => (finish = resolve)));
    store.load(snapshot);
    store.update("two");
    const saving = store.save();
    await new Promise(resolve => setTimeout(resolve, 0));
    store.update("three");
    finish({ articleId: "article", acceptedRevision: 1, savedAt: "now" });
    await saving;
    store.discard();
    expect(store.getState().content).toBe("two");
    expect(store.hasDirty()).toBe(false);
  });

  it("does not apply a late acknowledgement to a reopened document", async () => {
    let finish!: (value: { articleId: string; acceptedRevision: number; savedAt: string }) => void;
    const store = new EditorSessionStore(() => new Promise(resolve => (finish = resolve)));
    store.load(snapshot);
    const oldToken = store.documentToken();
    store.update("two");
    const saving = store.save();
    await new Promise(resolve => setTimeout(resolve, 0));
    store.load({ ...snapshot, content: "reopened" });
    store.update("new edit");
    finish({ articleId: "article", acceptedRevision: 1, savedAt: "old time" });
    await saving;
    expect(store.matchesDocument(oldToken)).toBe(false);
    expect(store.getState()).toMatchObject({ content: "new edit", dirty: true, savedAt: null });
  });

  it("maps async insertion through user edits and never targets another document", () => {
    const store = new EditorSessionStore(async request => ({ articleId: request.articleId, acceptedRevision: request.revision, savedAt: "now" }));
    store.load({ ...snapshot, content: "hello world" });
    store.setSelection(6);
    const bookmark = store.createInsertionBookmark();
    store.update("prefix hello world");
    store.setSelection(0);
    expect(store.insertMarkdown("![photo](/photo.png)", bookmark)).toBe(true);
    expect(store.getState().content).toBe("prefix hello \n![photo](/photo.png)\nworld");
    const expired = store.createInsertionBookmark();
    store.load({ ...snapshot, articleId: "other", content: "untouched" });
    expect(store.insertMarkdown("wrong image", expired)).toBe(false);
    expect(store.getState().content).toBe("untouched");
  });

  it("does not overwrite selected text edited while image import is pending", () => {
    const store = new EditorSessionStore(async request => ({ articleId: request.articleId, acceptedRevision: request.revision, savedAt: "now" }));
    store.load({ ...snapshot, content: "hello world" });
    store.setSelection(6, 11);
    const bookmark = store.createInsertionBookmark();
    store.update("hello changed");
    expect(store.insertMarkdown("![photo](/photo.png)", bookmark)).toBe(false);
    expect(store.getState().content).toBe("hello changed");
  });

  it("protects externally changed clean content until explicitly resolved", async () => {
    const calls: string[] = [];
    const store = new EditorSessionStore(async request => {
      calls.push(request.content);
      return { articleId: request.articleId, acceptedRevision: request.revision, savedAt: "now" };
    });
    store.load(snapshot);
    store.markExternalChange("deleted");
    expect(store.hasDirty()).toBe(true);
    await expect(store.save()).rejects.toThrow("外部更改");
    expect(calls).toEqual([]);
    store.allowExternalOverwrite();
    await store.saveUntilClean();
    expect(calls).toEqual(["one"]);
  });

  it("saves an undo back to the original content after an older write completes", async () => {
    let finish!: () => void;
    const calls: string[] = [];
    const store = new EditorSessionStore(async request => {
      calls.push(request.content);
      if (calls.length === 1) await new Promise<void>(resolve => (finish = resolve));
      return { articleId: request.articleId, acceptedRevision: request.revision, savedAt: "now" };
    });
    store.load(snapshot);
    store.update("two");
    const older = store.save();
    await new Promise(resolve => setTimeout(resolve, 0));
    store.update("one");
    expect(store.getState()).toMatchObject({ dirty: false, saving: true });
    const settled = store.saveUntilClean();
    finish();
    await older;
    await settled;
    expect(calls).toEqual(["two", "one"]);
    expect(store.getState()).toMatchObject({ content: "one", dirty: false, saving: false });
  });

  it("can explicitly save preserved local work after an inflight save fails during external change", async () => {
    let fail!: (error: Error) => void;
    const calls: string[] = [];
    const store = new EditorSessionStore(async request => {
      calls.push(request.content);
      if (calls.length === 1) await new Promise((_resolve, reject) => (fail = reject));
      return { articleId: request.articleId, acceptedRevision: request.revision, savedAt: "now" };
    });
    store.load(snapshot);
    store.update("local work");
    const older = store.save();
    await new Promise(resolve => setTimeout(resolve, 0));
    store.rebaseSessionGeneration(2);
    store.markExternalChange("changed");
    fail(new Error("old generation expired"));
    await expect(older).rejects.toThrow("old generation expired");
    await store.waitForSave().catch(() => null);
    store.allowExternalOverwrite();
    await store.saveUntilClean();
    expect(calls).toEqual(["local work", "local work"]);
    expect(store.getState()).toMatchObject({ content: "local work", dirty: false, externalChange: null });
    expect(store.getState().snapshot?.sessionGeneration).toBe(2);
  });

  it("replaces only a pending image URL and preserves an edited description", () => {
    const pending = "http://hlex-asset.localhost/upload-1";
    const content = `![用户后来写的描述](${pending})\n\n![另一张](${pending}-other)`;
    expect(replaceMarkdownImageUrl(content, pending, "https://img.example.com/ready.png")).toBe(
      `![用户后来写的描述](https://img.example.com/ready.png)\n\n![另一张](${pending}-other)`
    );
  });

  it("keeps dollar signs in the remote image URL literal", () => {
    const pending = "http://hlex-asset.localhost/upload-1";
    expect(replaceMarkdownImageUrl(
      `![描述](${pending})`,
      pending,
      "https://img.example.com/$folder/$&-ready.png"
    )).toBe("![描述](https://img.example.com/$folder/$&-ready.png)");
  });

  it("finds cached markdown images that need upload recovery", () => {
    const first = "0f5845c7-a9d8-40e9-97af-f770331f56c1";
    const second = "bfbd7252-77bf-4c6c-9165-b71c1a16ae85";
    const content = [
      `![用户修改过的描述](http://hlex-asset.localhost/${first})`,
      `![同一张](http://hlex-asset.localhost/${first})`,
      `![带标题](hlex-asset://localhost/${second} "标题")`,
      "[普通链接](http://hlex-asset.localhost/34ae4a51-98b2-42d1-beb0-128ae10fba81)"
    ].join("\n");

    expect(findPendingEditorImages(content)).toEqual([
      { uploadId: first, localUrl: `http://hlex-asset.localhost/${first}` },
      { uploadId: second, localUrl: `hlex-asset://localhost/${second}` }
    ]);
  });
});
