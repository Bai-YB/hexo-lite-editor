import { describe, expect, it, vi } from "vitest";
import { EditorSessionStore } from "$features/editor/EditorSessionStore";
import type { ArticleSummary, DocumentSnapshot, ProjectSessionView, SaveDocumentRequest, SaveDocumentResult } from "$shared/types/app";
import { checkRecoveryRemote, matchesRecovery, persistRecoveryDraft, waitForRecovery, type RecoveryIdentity } from "./documentRecovery";

const original: DocumentSnapshot = { projectId: "project", articleId: "post", sessionGeneration: 2, revision: 0, content: "original" };
const deferred = <T>() => {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => { resolve = done; });
  return { promise, resolve };
};

function setup(save = vi.fn(async (request: SaveDocumentRequest) => ({ articleId: request.articleId, acceptedRevision: request.revision, savedAt: "now" }))) {
  const store = new EditorSessionStore(save);
  store.load(original);
  store.update("local edits");
  store.markExternalChange("changed");
  let session: ProjectSessionView = { projectId: "project", generation: 2, name: "Project", displayPath: "fixture", warnings: [] };
  const loadDocument = vi.fn(async () => ({ ...original, content: "remote 1" }));
  const context = { store, getSession: () => session, loadDocument };
  const identity: RecoveryIdentity = { projectId: "project", generation: 2, token: store.documentToken(), revision: store.getState().revision, externalChange: "changed" };
  return { store, context, identity, setSession: (value: ProjectSessionView) => { session = value; } };
}

describe("article recovery confirmation", () => {
  it("requires another decision when the remote content changes after it was displayed", async () => {
    const { context, identity, store } = setup();
    context.loadDocument.mockResolvedValue({ ...original, content: "remote 2" });
    const refreshed = await checkRecoveryRemote(identity, context, "remote 1");
    expect(refreshed.status).toBe("review");
    expect(store.getState().content).toBe("local edits");
    expect(store.getState().externalChange).toBe("changed");
    expect(await checkRecoveryRemote(identity, context, "remote 2")).toMatchObject({ status: "confirmed", snapshot: { content: "remote 2" } });
  });

  it("does not treat a failed comparison load as confirmation of an empty remote file", async () => {
    const { context, identity } = setup();
    context.loadDocument.mockResolvedValue({ ...original, content: "" });
    expect((await checkRecoveryRemote(identity, context, null)).status).toBe("review");
    expect((await checkRecoveryRemote(identity, context, "")).status).toBe("confirmed");
  });

  it.each(["editing", "reopening", "generation", "deletion"] as const)("rejects an in-flight comparison after %s", async (change) => {
    const test = setup();
    const remote = deferred<DocumentSnapshot>();
    test.context.loadDocument.mockReturnValue(remote.promise);
    const checking = checkRecoveryRemote(test.identity, test.context, "remote 1");
    if (change === "editing") test.store.update("new local edits");
    if (change === "reopening") test.store.load({ ...original, content: "reopened document" });
    if (change === "generation") test.setSession({ ...test.context.getSession()!, generation: 3 });
    if (change === "deletion") test.store.markExternalChange("deleted");
    remote.resolve({ ...original, content: "remote 1" });
    expect(await checking).toEqual({ status: "stale" });
    expect(matchesRecovery(test.identity, test.context)).toBe(false);
  });

  it("rejects a mismatched IPC document even when its content matches", async () => {
    const { context, identity } = setup();
    context.loadDocument.mockResolvedValue({ ...original, articleId: "other", content: "remote 1" });
    await expect(checkRecoveryRemote(identity, context, "remote 1")).rejects.toThrow("读取结果已过期");
  });

  it("checks the local revision again after an earlier save settles", async () => {
    const saving = deferred<SaveDocumentResult>();
    const test = setup(vi.fn(() => saving.promise));
    test.store.allowExternalOverwrite();
    const pendingSave = test.store.save();
    await Promise.resolve();
    await Promise.resolve();
    test.store.markExternalChange("changed");
    const identity = { ...test.identity, revision: test.store.getState().revision };
    const waiting = waitForRecovery(identity, test.context);
    test.store.update("typed while waiting for the save");
    saving.resolve({ articleId: "post", acceptedRevision: identity.revision, savedAt: "now" });
    await pendingSave;
    expect(await waiting).toBe(false);
    expect(test.store.getState().content).toBe("typed while waiting for the save");
  });
});

describe("recovery draft persistence", () => {
  function draftSetup() {
    const test = setup();
    const article = { articleId: "draft", relativePath: "source/_drafts/recovered.md" } as ArticleSummary;
    const draftSnapshot = { ...original, articleId: "draft", content: "new draft", revision: 5 };
    test.context.loadDocument.mockResolvedValue(draftSnapshot);
    const input = {
      ...test,
      article: null as ArticleSummary | null,
      request: { projectId: "project", sessionGeneration: 2, title: "Recovered", fileName: "recovered", kind: "draft" as const, date: "2026-09-10 10:00:00", tags: [], categories: [] },
      content: "local edits",
      createArticle: vi.fn(async () => article),
      remember: vi.fn((created: ArticleSummary) => { input.article = created; }),
      saveDocument: vi.fn(async (request: SaveDocumentRequest) => ({ articleId: request.articleId, acceptedRevision: request.revision, savedAt: "now" }))
    };
    return { input, article, draftSnapshot };
  }

  it("reuses the created draft when saving fails and the user retries", async () => {
    const { input, article, draftSnapshot } = draftSetup();
    input.saveDocument.mockRejectedValueOnce(new Error("disk full"));
    await expect(persistRecoveryDraft(input)).rejects.toThrow("disk full");
    expect(input.article).toBe(article);
    expect(input.store.getState().content).toBe("local edits");
    input.context.loadDocument.mockResolvedValueOnce(draftSnapshot).mockResolvedValueOnce({ ...draftSnapshot, content: "local edits", revision: 6 });
    expect(await persistRecoveryDraft(input)).toMatchObject({ articleId: "draft", content: "local edits" });
    expect(input.createArticle).toHaveBeenCalledOnce();
    expect(input.saveDocument).toHaveBeenCalledTimes(2);
    expect(input.saveDocument.mock.calls.map(([request]) => request.articleId)).toEqual(["draft", "draft"]);
  });

  it("preserves the original editor when final verification fails and retains the draft for retry", async () => {
    const { input, article, draftSnapshot } = draftSetup();
    input.context.loadDocument.mockResolvedValueOnce(draftSnapshot).mockRejectedValueOnce(new Error("read failed"));
    await expect(persistRecoveryDraft(input)).rejects.toThrow("read failed");
    expect(input.article).toBe(article);
    expect(input.store.getState()).toMatchObject({ content: "local edits", externalChange: "changed", snapshot: { articleId: "post" } });
  });

  it("does not write the draft if the document changes while creation is pending", async () => {
    const { input, article } = draftSetup();
    const creating = deferred<ArticleSummary>();
    input.createArticle.mockReturnValue(creating.promise);
    const recovery = persistRecoveryDraft(input);
    input.store.update("new local edits");
    creating.resolve(article);
    await expect(recovery).rejects.toThrow("文章版本已变化");
    expect(input.article).toBe(article);
    expect(input.saveDocument).not.toHaveBeenCalled();
  });

  it("does not accept a draft overwritten between the save and verification read", async () => {
    const { input, draftSnapshot } = draftSetup();
    input.context.loadDocument.mockResolvedValueOnce(draftSnapshot).mockResolvedValueOnce({ ...draftSnapshot, content: "changed externally" });
    await expect(persistRecoveryDraft(input)).rejects.toThrow("恢复草稿的内容又有变化");
    expect(input.store.getState().content).toBe("local edits");
  });
});
