import type { EditorSessionStore } from "$features/editor/EditorSessionStore";
import type { DocumentSnapshot, ProjectRescanResult, ProjectSessionView } from "$shared/types/app";

export interface RescanContext {
  store: EditorSessionStore;
  getSession(): ProjectSessionView | null;
  accept(project: ProjectRescanResult): void;
  loadDocument(projectId: string, articleId: string, generation: number): Promise<DocumentSnapshot>;
}

export async function reconcileProjectRescan(project: ProjectRescanResult, context: RescanContext) {
  const { store } = context;
  const session = context.getSession();
  if (!session || session.projectId !== project.projectId
    || project.generation <= session.generation
    || (project.previousGeneration !== undefined && project.previousGeneration !== session.generation)) return "ignored";
  const previous = store.getState();
  const token = store.documentToken();
  context.accept(project);
  if (!previous.snapshot || previous.snapshot.projectId !== project.projectId) return "refreshed";
  store.rebaseSessionGeneration(project.generation);
  const articleId = previous.snapshot.articleId;
  if (!project.articles.some((article) => article.articleId === articleId)) {
    store.markExternalChange("deleted");
    return "deleted";
  }
  if (previous.dirty || previous.saving || previous.externalChange) {
    store.markExternalChange("changed");
    return "changed";
  }
  const current = () => {
    const active = context.getSession();
    return active?.projectId === project.projectId && active.generation === project.generation
      && store.matchesDocument(token);
  };
  try {
    const snapshot = await context.loadDocument(project.projectId, articleId, project.generation);
    if (!current()) return "ignored";
    const latest = store.getState();
    if (latest.revision !== previous.revision || latest.dirty || latest.saving || latest.externalChange) {
      store.markExternalChange("changed");
      return "changed";
    }
    if (snapshot.projectId !== project.projectId || snapshot.articleId !== articleId
      || snapshot.sessionGeneration !== project.generation) throw new Error("文章刷新结果已过期，请重新读取。");
    store.load(snapshot);
    return "refreshed";
  } catch (error) {
    if (!current()) return "ignored";
    store.markExternalChange("changed");
    throw error;
  }
}
