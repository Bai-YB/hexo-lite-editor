import { describe, expect, it, vi } from "vitest";
import { FileSessionStore } from "./FileSessionStore";
import type { ProjectFileSnapshot, ProjectSessionView } from "$shared/types/app";

const snapshot: ProjectFileSnapshot = { projectId: "project", sessionGeneration: 1, path: "source/_data/link.yml", content: "original", contentHash: "original-hash", editable: true };
const session: ProjectSessionView = { projectId: "project", generation: 2, name: "blog", displayPath: "/blog", warnings: [] };

describe("FileSessionStore", () => {
  it("serializes repeated saves and writes edits made during an in-flight save against the returned hash", async () => {
    let resolve!: (snapshot: ProjectFileSnapshot) => void;
    const write = vi.fn().mockImplementationOnce(() => new Promise<ProjectFileSnapshot>(done => resolve = done))
      .mockImplementation(async (base, content) => ({ ...base, content, contentHash: "third-hash" }));
    const store = new FileSessionStore(write);
    store.load(snapshot); store.update("second");
    const first = store.save(); const duplicate = store.save();
    store.update("third");
    resolve({ ...snapshot, content: "second", contentHash: "second-hash" });
    await Promise.all([first, duplicate]);
    expect(write).toHaveBeenCalledTimes(2);
    expect(write.mock.calls[1][0].contentHash).toBe("second-hash");
    expect(write.mock.calls[1][1]).toBe("third");
    expect(store.getState()).toMatchObject({ dirty: false, saving: false, content: "third" });
  });
  it("preserves local edits on save failure and waits before discarding", async () => {
    let reject!: (error: Error) => void;
    const store = new FileSessionStore(() => new Promise((_, fail) => reject = fail));
    store.load(snapshot); store.update("local");
    const save = store.save().catch(() => undefined);
    const discard = store.discard();
    expect(store.getState().content).toBe("local");
    reject(new Error("disk changed"));
    await save; await discard;
    expect(store.getState()).toMatchObject({ content: "original", dirty: false, saving: false });
  });
  it("preserves dirty text when a rescan changes the disk and blocks saving until the user chooses", async () => {
    const write = vi.fn(); const store = new FileSessionStore(write);
    store.load(snapshot); store.update("my edits");
    const disk = { ...snapshot, sessionGeneration: 2, content: "remote", contentHash: "remote-hash" };
    await store.refresh(session, async () => disk);
    expect(store.getState()).toMatchObject({ content: "my edits", dirty: true, conflict: disk });
    await expect(store.save()).rejects.toThrow();
    expect(write).not.toHaveBeenCalled();
    store.keepLocalAgainst(disk);
    expect(store.getState().snapshot?.contentHash).toBe("remote-hash");
    expect(store.getState().content).toBe("my edits");
  });
  it("rebases an unchanged file without losing new local input during the read", async () => {
    let resolve!: (snapshot: ProjectFileSnapshot) => void;
    const store = new FileSessionStore(vi.fn()); store.load(snapshot);
    const refresh = store.refresh(session, () => new Promise(done => resolve = done));
    await vi.waitFor(() => expect(resolve).toBeTypeOf("function")); store.update("typed during read");
    resolve({ ...snapshot, sessionGeneration: 2 }); await refresh;
    expect(store.getState()).toMatchObject({ content: "typed during read", dirty: true });
    expect(store.getState().snapshot?.sessionGeneration).toBe(2);
  });
  it("ignores a late disk refresh after switching documents", async () => {
    let resolve!: (snapshot: ProjectFileSnapshot) => void;
    const store = new FileSessionStore(vi.fn()); store.load(snapshot);
    const refresh = store.refresh(session, () => new Promise(done => resolve = done));
    await vi.waitFor(() => expect(resolve).toBeTypeOf("function"));
    store.load({ ...snapshot, path: "_config.yml", content: "another file" });
    resolve({ ...snapshot, content: "remote" }); await refresh;
    expect(store.getState().content).toBe("another file");
    expect(store.getState().conflict).toBeNull();
  });
  it("does not edit binary or oversized documents", () => {
    const store = new FileSessionStore(vi.fn()); store.load({ ...snapshot, editable: false });
    store.update("overwrite"); expect(store.hasDirty()).toBe(false);
    expect(store.getState().content).toBe("original");
  });
  it.each([true, false])("prioritizes the newest rescan when generation 2 returns first: %s", async olderFirst => {
    const store = new FileSessionStore(vi.fn()); store.load(snapshot); store.update("local");
    const resolvers = new Map<number, (value: ProjectFileSnapshot) => void>();
    const read = (project: typeof session) => new Promise<ProjectFileSnapshot>(resolve => resolvers.set(project.generation, resolve));
    const older = store.refresh(session, read);
    await vi.waitFor(() => expect(resolvers.has(2)).toBe(true));
    const newer = store.refresh({ ...session, generation: 3 }, read);
    await vi.waitFor(() => expect(resolvers.has(3)).toBe(true));
    const order = olderFirst ? [2, 3] : [3, 2];
    for (const generation of order) { resolvers.get(generation)!({ ...snapshot, sessionGeneration: generation }); await Promise.resolve(); }
    await Promise.all([older, newer]);
    expect(store.getState()).toMatchObject({ content: "local", dirty: true, snapshot: { sessionGeneration: 3 } });
  });
  it("keeps text and a recoverable error when the file is deleted during rescan", async () => {
    const store = new FileSessionStore(vi.fn()); store.load(snapshot); store.update("local");
    await expect(store.refresh(session, async () => { throw new Error("file removed"); })).rejects.toThrow("file removed");
    expect(store.getState()).toMatchObject({ content: "local", dirty: true });
    expect(store.getState().error).toContain("刷新文件树");
  });
});
