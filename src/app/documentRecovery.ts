import type { EditorSessionStore } from "$features/editor/EditorSessionStore";
import type { ArticleSummary, CreateArticleRequest, DocumentSnapshot, ProjectSessionView, SaveDocumentRequest, SaveDocumentResult } from "$shared/types/app";

export interface RecoveryIdentity {
  projectId: string;
  generation: number;
  token: number;
  revision: number;
  externalChange: "changed" | "deleted";
}

export interface RecoveryContext {
  store: EditorSessionStore;
  getSession(): ProjectSessionView | null;
  loadDocument(projectId: string, articleId: string, generation: number): Promise<DocumentSnapshot>;
}

export function matchesRecovery(identity: RecoveryIdentity | null, context: RecoveryContext) {
  const session = context.getSession();
  const state = context.store.getState();
  return Boolean(identity && session?.projectId === identity.projectId && session.generation === identity.generation
    && context.store.matchesDocument(identity.token) && state.revision === identity.revision
    && state.externalChange === identity.externalChange);
}

export async function waitForRecovery(identity: RecoveryIdentity, context: RecoveryContext) {
  await context.store.waitForSave().catch(() => null);
  return matchesRecovery(identity, context);
}

function assertSnapshot(snapshot: DocumentSnapshot, projectId: string, generation: number, articleId: string) {
  if (snapshot.projectId !== projectId || snapshot.sessionGeneration !== generation || snapshot.articleId !== articleId) {
    throw new Error("文章读取结果已过期，请重新核对内容。");
  }
}

export async function checkRecoveryRemote(identity: RecoveryIdentity, context: RecoveryContext, displayed: string | null) {
  const state = context.store.getState();
  if (!state.snapshot || !matchesRecovery(identity, context)) return { status: "stale" } as const;
  const snapshot = await context.loadDocument(identity.projectId, state.snapshot.articleId, identity.generation);
  if (!matchesRecovery(identity, context)) return { status: "stale" } as const;
  assertSnapshot(snapshot, identity.projectId, identity.generation, state.snapshot.articleId);
  return { status: displayed === snapshot.content ? "confirmed" : "review", snapshot } as const;
}

export async function persistRecoveryDraft(input: {
  identity: RecoveryIdentity;
  context: RecoveryContext;
  article: ArticleSummary | null;
  request: CreateArticleRequest;
  content: string;
  createArticle(request: CreateArticleRequest): Promise<ArticleSummary>;
  remember(article: ArticleSummary): void;
  saveDocument(request: SaveDocumentRequest): Promise<SaveDocumentResult>;
}) {
  const { identity, context } = input;
  const assertCurrent = () => {
    if (!matchesRecovery(identity, context)) throw new Error("文章版本已变化，请重新核对后继续另存草稿。");
  };
  assertCurrent();
  const article = input.article ?? await input.createArticle(input.request);
  // Remember the created file before any later IPC can fail, so retries reuse it.
  input.remember(article);
  assertCurrent();
  const snapshot = await context.loadDocument(identity.projectId, article.articleId, identity.generation);
  assertCurrent();
  assertSnapshot(snapshot, identity.projectId, identity.generation, article.articleId);
  await input.saveDocument({ ...snapshot, content: input.content, revision: snapshot.revision + 1 });
  assertCurrent();
  const saved = await context.loadDocument(identity.projectId, article.articleId, identity.generation);
  assertCurrent();
  assertSnapshot(saved, identity.projectId, identity.generation, article.articleId);
  if (saved.content !== input.content) throw new Error("恢复草稿的内容又有变化，本地内容已保留，请重新核对后继续。");
  return saved;
}
