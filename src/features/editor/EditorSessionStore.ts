import type {
  DocumentSnapshot,
  SaveDocumentRequest,
  SaveDocumentResult
} from "$shared/types/app";

export interface EditorSessionState {
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

export class EditorSessionStore {
  private state: EditorSessionState = {
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
  private listeners = new Set<(state: EditorSessionState) => void>();

  constructor(
    private readonly saveDocument: (request: SaveDocumentRequest) => Promise<SaveDocumentResult>
  ) {}

  subscribe(listener: (state: EditorSessionState) => void) {
    this.listeners.add(listener);
    listener(this.getState());
    return () => this.listeners.delete(listener);
  }

  getState(): EditorSessionState {
    return { ...this.state, selection: { ...this.state.selection } };
  }

  load(snapshot: DocumentSnapshot) {
    this.lastSavedContent = snapshot.content;
    const selection = this.cursorByArticle.get(snapshot.articleId) ?? {
      from: snapshot.content.length,
      to: snapshot.content.length
    };
    this.state = {
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
    this.lastSavedContent = "";
    this.state = {
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

  update(content: string) {
    if (!this.state.snapshot || content === this.state.content) return;
    this.state.content = content;
    this.state.revision += 1;
    this.state.dirty = true;
    this.state.error = null;
    this.notify();
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
    this.notify();
  }

  insertMarkdown(markdown: string) {
    if (!this.state.snapshot || !markdown) return false;
    const { from, to } = this.state.selection;
    const before = this.state.content.slice(0, from);
    const after = this.state.content.slice(to);
    const prefix = before && !before.endsWith("\n") ? "\n" : "";
    const suffix = after && !after.startsWith("\n") ? "\n" : "";
    const insertion = `${prefix}${markdown}${suffix}`;
    const cursor = from + insertion.length;
    this.state.content = `${before}${insertion}${after}`;
    this.state.revision += 1;
    this.state.dirty = true;
    this.state.error = null;
    this.state.selection = { from: cursor, to: cursor };
    this.cursorByArticle.set(this.state.snapshot.articleId, this.state.selection);
    this.notify();
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
    this.state.content = this.lastSavedContent;
    this.state.revision = this.state.persistedRevision;
    this.state.dirty = false;
    this.state.error = null;
    const cursor = Math.min(this.state.selection.from, this.lastSavedContent.length);
    this.state.selection = { from: cursor, to: cursor };
    this.cursorByArticle.set(this.state.snapshot.articleId, this.state.selection);
    this.notify();
  }

  save(): Promise<SaveDocumentResult | null> {
    const snapshot = this.state.snapshot;
    if (!snapshot || !this.state.dirty) return this.queue;
    const request: SaveDocumentRequest = {
      projectId: snapshot.projectId,
      articleId: snapshot.articleId,
      content: this.state.content,
      revision: this.state.revision,
      sessionGeneration: snapshot.sessionGeneration
    };
    this.queue = this.queue
      .catch(() => null)
      .then(async () => {
        this.state.saving = true;
        this.state.error = null;
        this.notify();
        try {
          const result = await this.saveDocument(request);
          const current = this.state.snapshot;
          if (
            current?.projectId === request.projectId &&
            current.articleId === request.articleId &&
            current.sessionGeneration === request.sessionGeneration &&
            result.acceptedRevision >= this.state.persistedRevision
          ) {
            this.state.persistedRevision = result.acceptedRevision;
            if (result.acceptedRevision === this.state.revision) {
              this.lastSavedContent = request.content;
              this.state.dirty = false;
            }
            this.state.savedAt = result.savedAt;
          }
          return result;
        } catch (error) {
          this.state.error = error instanceof Error ? error.message : String(error);
          throw error;
        } finally {
          this.state.saving = false;
          this.notify();
        }
      });
    return this.queue;
  }

  private notify() {
    const copy = this.getState();
    this.listeners.forEach((listener) => listener(copy));
  }
}
