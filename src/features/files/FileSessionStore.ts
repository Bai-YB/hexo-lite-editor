import type { ProjectFileSnapshot, ProjectSessionView } from "$shared/types/app";

export interface FileSessionState {
  snapshot: ProjectFileSnapshot | null;
  content: string;
  dirty: boolean;
  saving: boolean;
  error: string | null;
  conflict: ProjectFileSnapshot | null;
  documentInstance: number;
}
export type SaveFile = (snapshot: ProjectFileSnapshot, content: string) => Promise<ProjectFileSnapshot>;

/** One persistent text document, independent of article preview and image insertion. */
export class FileSessionStore {
  private state: FileSessionState = { snapshot: null, content: "", dirty: false, saving: false, error: null, conflict: null, documentInstance: 0 };
  private listeners = new Set<(state: FileSessionState) => void>();
  private pending: Promise<void> | null = null;
  private refreshSequence = 0;
  constructor(private readonly write: SaveFile) {}
  getState() { return { ...this.state }; }
  subscribe(listener: (state: FileSessionState) => void) { this.listeners.add(listener); listener(this.getState()); return () => this.listeners.delete(listener); }
  private notify() { for (const listener of this.listeners) listener(this.getState()); }
  load(snapshot: ProjectFileSnapshot) {
    this.state = { snapshot, content: snapshot.content, dirty: false, saving: false, error: null, conflict: null, documentInstance: this.state.documentInstance + 1 };
    this.notify();
  }
  clear() { this.state = { snapshot: null, content: "", dirty: false, saving: false, error: null, conflict: null, documentInstance: this.state.documentInstance + 1 }; this.notify(); }
  update(content: string) {
    if (!this.state.snapshot?.editable || content === this.state.content) return;
    this.state.content = content;
    this.state.dirty = content !== this.state.snapshot.content;
    this.state.error = null;
    this.notify();
  }
  hasDirty() { return this.state.dirty || this.state.saving; }
  waitForSave() { return this.pending ?? Promise.resolve(); }
  async discard() { await this.waitForSave().catch(() => undefined); if (this.state.conflict) this.load(this.state.conflict); else if (this.state.snapshot) this.load(this.state.snapshot); }
  save(): Promise<void> {
    if (this.pending) return this.pending;
    if (!this.state.snapshot || !this.state.dirty) return Promise.resolve();
    const instance = this.state.documentInstance;
    this.state.saving = true;
    this.notify();
    const operation = async () => {
      try {
        do {
          if (this.state.conflict) throw new Error("文件已在磁盘更改，请先比较两个版本。");
          const snapshot = this.state.snapshot!;
          const content = this.state.content;
          const saved = await this.write(snapshot, content);
          if (this.state.documentInstance !== instance) return;
          this.state.snapshot = saved;
          this.state.dirty = this.state.content !== content;
          this.state.error = null;
          this.notify();
        } while (this.state.dirty);
      } catch (error) {
        if (this.state.documentInstance === instance) {
          this.state.error = error && typeof error === "object" && "message" in error ? String(error.message) : String(error);
          this.notify();
        }
        throw error;
      } finally {
        if (this.state.documentInstance === instance) { this.state.saving = false; this.notify(); }
      }
    };
    this.pending = operation().finally(() => { this.pending = null; });
    return this.pending;
  }
  /** Rebase on a sync generation only after comparing the disk version. */
  async refresh(session: ProjectSessionView, read: (session: ProjectSessionView, path: string) => Promise<ProjectFileSnapshot>) {
    const sequence = ++this.refreshSequence;
    await this.waitForSave().catch(() => undefined);
    if (sequence !== this.refreshSequence) return;
    const snapshot = this.state.snapshot;
    if (!snapshot || snapshot.projectId !== session.projectId) return;
    const instance = this.state.documentInstance;
    let disk: ProjectFileSnapshot;
    try { disk = await read(session, snapshot.path); }
    catch (error) {
      if (sequence !== this.refreshSequence || instance !== this.state.documentInstance) return;
      this.state.error = `无法重新读取文件，当前输入已保留。请刷新文件树后重试：${error && typeof error === "object" && "message" in error ? String(error.message) : String(error)}`;
      this.notify();
      throw error;
    }
    if (sequence !== this.refreshSequence || instance !== this.state.documentInstance || this.state.snapshot !== snapshot) return;
    if (disk.contentHash === snapshot.contentHash) {
      this.state.snapshot = { ...snapshot, sessionGeneration: session.generation };
      this.state.error = null;
      this.state.conflict = null;
    } else if (!this.state.dirty) { this.load(disk); return; }
    else { this.state.conflict = disk; this.state.error = "文件已在磁盘更改，当前输入已保留。"; }
    this.notify();
  }
  acceptDisk(disk: ProjectFileSnapshot) { this.load(disk); }
  keepLocalAgainst(disk: ProjectFileSnapshot) {
    if (!this.state.snapshot || disk.path !== this.state.snapshot.path || disk.projectId !== this.state.snapshot.projectId) return;
    this.state.snapshot = disk;
    this.state.conflict = null;
    this.state.dirty = this.state.content !== disk.content;
    this.state.error = null;
    this.notify();
  }
}
