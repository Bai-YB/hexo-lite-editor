import type {
  DocumentSnapshot,
  SaveDocumentRequest,
  SaveDocumentResult
} from "$shared/types/app";
import { uiText } from "$shared/i18n/ui";
import type { ChangeDesc } from "@codemirror/state";
import { contentChanges } from "./editorChanges";

export interface EditorSessionState {
  documentInstance: number;
  externalChange: "changed" | "deleted" | null;
  imageUrlReplacements: Record<string, string>;
  snapshot: DocumentSnapshot | null;
  content: string;
  revision: number;
  persistedRevision: number;
  dirty: boolean;
  saving: boolean;
  savedAt: string | null;
  error: string | null;
  selection: { from: number; to: number };
}

export interface InsertionBookmark {
  documentInstance: number;
  from: number;
  to: number;
  valid: boolean;
}

export type EditorStateGroup = "content" | "selection" | "saving" | "session";

type StateListener = (state: EditorSessionState) => void;

const ALL_GROUPS: readonly EditorStateGroup[] = ["content", "selection", "saving", "session"];

export class EditorSessionStore {
  private state: EditorSessionState = {
    documentInstance: 0,
    externalChange: null,
    imageUrlReplacements: {},
    snapshot: null,
    content: "",
    revision: 0,
    persistedRevision: 0,
    dirty: false,
    saving: false,
    savedAt: null,
    error: null,
    selection: { from: 0, to: 0 }
  };
  private lastSavedContent = "";
  private cursorByArticle = new Map<string, { from: number; to: number }>();
  private queue: Promise<SaveDocumentResult | null> = Promise.resolve(null);
  private listeners = new Set<StateListener>();
  private channels: Record<EditorStateGroup, Set<StateListener>> = {
    content: new Set(),
    selection: new Set(),
    saving: new Set(),
    session: new Set()
  };
  private bookmarks = new Set<InsertionBookmark>();

  constructor(
    private readonly saveDocument: (request: SaveDocumentRequest) => Promise<SaveDocumentResult>
  ) {}

  subscribe(listener: StateListener) {
    this.listeners.add(listener);
    listener(this.getState());
    return () => this.listeners.delete(listener);
  }

  subscribeContent(listener: StateListener) { return this.subscribeGroup("content", listener); }
  subscribeSelection(listener: StateListener) { return this.subscribeGroup("selection", listener); }
  subscribeSaving(listener: StateListener) { return this.subscribeGroup("saving", listener); }
  subscribeSession(listener: StateListener) { return this.subscribeGroup("session", listener); }

  private subscribeGroup(group: EditorStateGroup, listener: StateListener) {
    this.channels[group].add(listener);
    listener(this.getState());
    return () => this.channels[group].delete(listener);
  }

  getSelection() {
    return { ...this.state.selection };
  }

  getState(): EditorSessionState {
    return { ...this.state, selection: { ...this.state.selection } };
  }

  load(snapshot: DocumentSnapshot) {
    this.bookmarks.clear();
    this.lastSavedContent = snapshot.content;
    const selection = this.cursorByArticle.get(snapshot.articleId) ?? {
      from: snapshot.content.length,
      to: snapshot.content.length
    };
    this.state = {
      documentInstance: this.state.documentInstance + 1,
      externalChange: null,
      imageUrlReplacements: {},
      snapshot,
      content: snapshot.content,
      revision: snapshot.revision,
      persistedRevision: snapshot.revision,
      dirty: false,
      saving: false,
      savedAt: null,
      error: null,
      selection
    };
    this.notify();
  }

  clear() {
    this.bookmarks.clear();
    this.lastSavedContent = "";
    this.state = {
      documentInstance: this.state.documentInstance + 1,
      externalChange: null,
      imageUrlReplacements: {},
      snapshot: null,
      content: "",
      revision: 0,
      persistedRevision: 0,
      dirty: false,
      saving: false,
      savedAt: null,
      error: null,
      selection: { from: 0, to: 0 }
    };
    this.notify();
  }

  rebaseSessionGeneration(sessionGeneration: number) {
    if (!this.state.snapshot || this.state.snapshot.sessionGeneration === sessionGeneration) return;
    this.state.snapshot = { ...this.state.snapshot, sessionGeneration };
    this.notify("session");
  }

  documentToken() { return this.state.documentInstance; }

  matchesDocument(token: number) {
    return Boolean(this.state.snapshot) && this.state.documentInstance === token;
  }

  markExternalChange(kind: "changed" | "deleted") {
    this.state.externalChange = kind;
    this.state.dirty = true;
    this.notify(["content", "session"]);
  }

  allowExternalOverwrite() {
    this.state.externalChange = null;
    this.state.dirty = true;
    this.state.revision += 1;
    this.notify(["content", "session"]);
  }

  createInsertionBookmark(): InsertionBookmark {
    const bookmark = { documentInstance: this.documentToken(), ...this.state.selection, valid: true };
    this.bookmarks.add(bookmark);
    return bookmark;
  }

  releaseInsertionBookmark(bookmark: InsertionBookmark) { this.bookmarks.delete(bookmark); }

  private mapBookmarks(changes: ChangeDesc) {
    for (const bookmark of this.bookmarks) {
      const collapsed = bookmark.from === bookmark.to;
      changes.iterChangedRanges((from, to) => {
        if (!collapsed && from < bookmark.to && to > bookmark.from) bookmark.valid = false;
      });
      bookmark.from = changes.mapPos(bookmark.from, -1);
      bookmark.to = changes.mapPos(bookmark.to, collapsed ? -1 : 1);
    }
  }

  update(content: string, changes?: ChangeDesc) {
    const normalized = resolveImageUrls(content, this.state.imageUrlReplacements);
    if (normalized !== content) changes = undefined;
    content = normalized;
    if (!this.state.snapshot || content === this.state.content) return;
    this.mapBookmarks(changes ?? contentChanges(this.state.content, content));
    this.state.content = content;
    this.state.revision += 1;
    this.state.dirty = Boolean(this.state.externalChange) || content !== this.lastSavedContent;
    this.state.error = null;
    this.notify("content");
  }

  setSelection(from: number, to = from) {
    if (!this.state.snapshot) return;
    const max = this.state.content.length;
    const selection = {
      from: Math.max(0, Math.min(from, max)),
      to: Math.max(0, Math.min(to, max))
    };
    if (
      selection.from === this.state.selection.from &&
      selection.to === this.state.selection.to
    ) return;
    this.state.selection = selection;
    this.cursorByArticle.set(this.state.snapshot.articleId, selection);
    this.notify("selection");
  }

  insertMarkdown(markdown: string, bookmark?: InsertionBookmark) {
    if (!this.state.snapshot || !markdown) return false;
    if (bookmark && (!bookmark.valid || !this.matchesDocument(bookmark.documentInstance))) return false;
    const { from, to } = bookmark ?? this.state.selection;
    const before = this.state.content.slice(0, from);
    const after = this.state.content.slice(to);
    const prefix = before && !before.endsWith("\n") ? "\n" : "";
    const suffix = after && !after.startsWith("\n") ? "\n" : "";
    const insertion = `${prefix}${markdown}${suffix}`;
    const cursor = from + insertion.length;
    const nextContent = `${before}${insertion}${after}`;
    const changes = contentChanges(this.state.content, nextContent);
    this.mapBookmarks(changes);
    const selection = bookmark
      ? { from: changes.mapPos(this.state.selection.from), to: changes.mapPos(this.state.selection.to) }
      : { from: cursor, to: cursor };
    this.state.content = nextContent;
    this.state.revision += 1;
    this.state.dirty = true;
    this.state.error = null;
    this.state.selection = selection;
    this.cursorByArticle.set(this.state.snapshot.articleId, this.state.selection);
    this.notify(["content", "selection"]);
    return true;
  }

  replaceMarkdownImageUrl(expectedUrl: string, replacementUrl: string, articleId: string) {
    if (!this.state.snapshot || this.state.snapshot.articleId !== articleId) return false;
    this.state.imageUrlReplacements = { ...this.state.imageUrlReplacements, [expectedUrl]: replacementUrl };
    const next = replaceMarkdownImageUrl(this.state.content, expectedUrl, replacementUrl);
    if (next === this.state.content) { this.notify("content"); return false; }
    const changes = contentChanges(this.state.content, next);
    this.mapBookmarks(changes);
    this.state.selection = {
      from: changes.mapPos(this.state.selection.from),
      to: changes.mapPos(this.state.selection.to)
    };
    this.state.content = next;
    this.state.revision += 1;
    this.state.dirty = true;
    this.state.error = null;
    this.notify(["content", "selection"]);
    return true;
  }

  hasDirty() {
    return this.state.dirty;
  }

  activeArticleId() {
    return this.state.snapshot?.articleId ?? null;
  }

  discard() {
    if (!this.state.snapshot) return;
    this.mapBookmarks(contentChanges(this.state.content, this.lastSavedContent));
    this.state.content = this.lastSavedContent;
    this.state.revision += 1;
    this.state.dirty = Boolean(this.state.externalChange);
    this.state.error = null;
    const cursor = Math.min(this.state.selection.from, this.lastSavedContent.length);
    this.state.selection = { from: cursor, to: cursor };
    this.cursorByArticle.set(this.state.snapshot.articleId, this.state.selection);
    this.notify(["content", "selection"]);
  }

  save(): Promise<SaveDocumentResult | null> {
    if (this.state.externalChange) return Promise.reject(new Error(uiText("云端有更新，暂时不能保存。先选保留本地内容，还是用磁盘版本。")));
    const snapshot = this.state.snapshot;
    if (!snapshot || !this.state.dirty) return this.queue.catch(() => null);
    const request: SaveDocumentRequest = {
      projectId: snapshot.projectId,
      articleId: snapshot.articleId,
      content: this.state.content,
      revision: this.state.revision,
      sessionGeneration: snapshot.sessionGeneration
    };
    const token = this.documentToken();
    this.state.saving = true;
    this.notify("saving");
    this.queue = this.queue
      .catch(() => null)
      .then(async () => {
        if (!this.matchesDocument(token)) return null;
        this.state.saving = true;
        this.state.error = null;
        this.notify(["saving", "content"]);
        try {
          if (this.state.externalChange) throw new Error(uiText("云端有更新，先处理差异再保存。"));
          const result = await this.saveDocument(request);
          const current = this.state.snapshot;
          if (
            this.matchesDocument(token) && current?.projectId === request.projectId &&
            current.articleId === request.articleId &&
            current.sessionGeneration === request.sessionGeneration &&
            result.acceptedRevision >= this.state.persistedRevision
          ) {
            this.state.persistedRevision = result.acceptedRevision;
            this.lastSavedContent = request.content;
            this.state.dirty = Boolean(this.state.externalChange) || this.state.content !== request.content;
            this.state.savedAt = result.savedAt;
          }
          return result;
        } catch (error) {
          if (this.matchesDocument(token)) this.state.error = error instanceof Error ? error.message : String(error);
          throw error;
        } finally {
          if (this.matchesDocument(token)) {
            this.state.saving = false;
            this.notify(["saving", "content"]);
          }
        }
      });
    return this.queue;
  }

  waitForSave() { return this.queue; }

  async saveUntilClean() {
    const token = this.documentToken();
    if (!this.state.snapshot) return;
    do {
      await this.save();
      if (!this.matchesDocument(token)) throw new Error(uiText("文章已经切换了，重新操作一次。"));
    } while (this.state.dirty);
  }

  private notify(groups: EditorStateGroup | readonly EditorStateGroup[] = ALL_GROUPS) {
    const copy = this.getState();
    this.listeners.forEach((listener) => listener(copy));
    const list: readonly EditorStateGroup[] = typeof groups === "string" ? [groups] : groups;
    for (const group of list) this.channels[group].forEach((listener) => listener(copy));
    if (import.meta.env.DEV) {
      const scope = globalThis as unknown as {
        __editorStats?: { notifyCount: number; selectionNotify: number };
      };
      const stats = (scope.__editorStats ??= { notifyCount: 0, selectionNotify: 0 });
      stats.notifyCount += 1;
      if (list.includes("selection")) stats.selectionNotify += 1;
    }
  }
}

export function resolveImageUrls(content: string, replacements: Record<string, string>) {
  for (const [localUrl, remoteUrl] of Object.entries(replacements)) {
    content = replaceMarkdownImageUrl(content, localUrl, remoteUrl);
  }
  return content;
}

export function replaceMarkdownImageUrl(content: string, expectedUrl: string, replacementUrl: string) {
  const escaped = expectedUrl.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const markdownImage = new RegExp(`(!\\[[^\\]\\r\\n]*\\]\\(\\s*)${escaped}(?=(?:\\s+['\"][^'\"]*['\"])?\\s*\\))`, "g");
  return content.replace(markdownImage, (_match, prefix: string) => `${prefix}${replacementUrl}`);
}

export interface PendingEditorImage {
  uploadId: string;
  localUrl: string;
}

export function findPendingEditorImages(content: string): PendingEditorImage[] {
  const pendingImage = /!\[[^\]\r\n]*\]\(\s*((?:hlex-asset:\/\/localhost\/|http:\/\/hlex-asset\.localhost\/)([0-9a-fA-F-]{36}))(?=(?:\s+['"][^'"]*['"])?\s*\))/g;
  const found = new Map<string, PendingEditorImage>();
  for (const match of content.matchAll(pendingImage)) {
    const localUrl = match[1];
    const uploadId = match[2];
    if (localUrl && uploadId && !found.has(uploadId)) {
      found.set(uploadId, { uploadId, localUrl });
    }
  }
  return [...found.values()];
}
