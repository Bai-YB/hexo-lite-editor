import { defaultConfig } from "$shared/types/app";
import type {
  AcquireCloudflareImgbedTokenRequest,
  AcquireCloudflareImgbedTokenResult,
  AppConfigV3,
  ArticleSummary,
  DocumentSnapshot,
  LocalImage,
  OpenProjectResult,
  RecentProjectView,
  PreviewServerView,
  RemoteAssetItem,
  RemoteAssetPage,
  RemotePreviewImageResult,
  TaskLogPage,
  TaskLogSummary,
  SaveDocumentRequest,
  TaskEvent,
  TaskType
} from "$shared/types/app";

const session = {
  projectId: "demo-project",
  generation: 1,
  name: "Quiet Notes",
  displayPath: "C:\\博客\\quiet-notes",
  warnings: []
};

const image = (seed: string) => `https://picsum.photos/seed/${seed}/480/320`;

let config: AppConfigV3 = structuredClone(defaultConfig);
config.imageBed.cloudflareApiUrl = "https://img.example.com";
let articles: ArticleSummary[] = [
  { articleId: "welcome", relativePath: "source/_posts/欢迎使用.md", title: "欢迎使用 Hexo Lite Editor", kind: "post", frontMatterDate: "2026-07-17 20:00", createdAt: "2026-07-17T12:00:00Z", modifiedAt: "2026-07-17T13:20:00Z", tags: ["Hexo", "写作"], categories: ["指南"], cover: { source: "cover", previewUrl: image("quiet-desk"), alt: "文章封面" } },
  { articleId: "summer", relativePath: "source/_posts/盛夏散步.md", title: "盛夏散步：城市里的安静时刻", kind: "post", frontMatterDate: "2026-07-16 09:30", createdAt: "2026-07-16T01:30:00Z", modifiedAt: "2026-07-17T09:10:00Z", tags: ["生活", "摄影"], categories: ["随笔"], cover: { source: "placeholder", alt: "无封面" } },
  { articleId: "tauri", relativePath: "source/_posts/Tauri桌面应用笔记.md", title: "Tauri 桌面应用整理笔记", kind: "post", frontMatterDate: "2026-07-14 15:00", createdAt: "2026-07-14T07:00:00Z", modifiedAt: "2026-07-16T18:45:00Z", tags: ["Tauri", "Rust"], categories: ["开发"], cover: { source: "thumbnail", previewUrl: image("tauri-notes"), alt: "文章缩略图" } },
  { articleId: "draft", relativePath: "source/_drafts/下一篇文章.md", title: "下一篇文章的提纲", kind: "draft", createdAt: "2026-07-17T14:00:00Z", modifiedAt: "2026-07-17T14:00:00Z", tags: ["待整理"], categories: [], cover: { source: "placeholder", alt: "无封面" } }
];

const documents = new Map<string, string>([
  ["welcome", `---\ntitle: 欢迎使用 Hexo Lite Editor\ndate: 2026-07-17 20:00\ntags:\n  - Hexo\n  - 写作\ncategories:\n  - 指南\n---\n\n# 欢迎使用\n\n这是一个保留专注感的 Markdown 桌面写作工作区。\n\n![安静的桌面](${image("quiet-desk")})\n\n## 从这里开始\n\n${Array.from({ length: 520 }, (_, index) => `${index + 1}. 这是一段用于验证长文章滚动、PageDown 与独立预览滚动的正文。`).join("\n\n")}`],
  ["summer", "# 盛夏散步\n\n城市很热，但树影下仍有一些安静的时刻。"],
  ["tauri", "# Tauri 桌面应用整理笔记\n\n- 自绘标题栏\n- 安全 IPC\n- 结构化任务"],
  ["draft", "# 下一篇文章的提纲\n\n- 开场\n- 主要内容\n- 收尾"]
]);

let recent: RecentProjectView[] = [
  { recentId: "demo-recent", name: "Quiet Notes", displayPath: "C:\\博客\\quiet-notes", lastOpenedAt: "2026-07-17T13:20:00Z", available: true },
  { recentId: "missing-recent", name: "旧博客", displayPath: "D:\\Archive\\old-blog", lastOpenedAt: "2026-07-10T09:00:00Z", available: false }
];

const localImages: LocalImage[] = Array.from({ length: 10 }, (_, index) => ({
  imageId: `local-${index + 1}`,
  name: ["安静的桌面.jpg", "夏日街道.jpg", "窗边咖啡.jpg", "山间晨雾.jpg", "代码笔记.jpg", "书桌一角.jpg", "夜晚灯光.jpg", "旅途车站.jpg", "蓝色海面.jpg", "秋日树影.jpg"][index],
  relativePath: `source/images/photo-${index + 1}.jpg`,
  markdownUrl: `/images/photo-${index + 1}.jpg`,
  mime: "image/jpeg",
  size: 180000 + index * 32768,
  previewUrl: image(`local-${index + 1}`)
}));

const remoteAssets: RemoteAssetItem[] = [
  { assetId: "folder-course", kind: "folder", name: "可以导入 Wake Up 的课程表", fileName: "可以导入 Wake Up 的课程表", directory: "可以导入 Wake Up 的课程表", canPreview: false },
  { assetId: "folder-blog", kind: "folder", name: "blog", fileName: "blog", directory: "blog", canPreview: false },
  { assetId: "archive-7z", kind: "archive", name: "资料归档.7z", fileName: "资料归档.7z", directory: "", extension: "7z", size: 4200000, url: "https://example.com/archive.7z", canPreview: false },
  ...Array.from({ length: 8 }, (_, index): RemoteAssetItem => ({
    assetId: `remote-${index + 1}`,
    kind: "image",
    name: `remote-photo-${index + 1}.jpg`,
    fileName: `remote-photo-${index + 1}.jpg`,
    directory: index % 2 ? "blog" : "",
    extension: "jpg",
    url: image(`remote-${index + 1}`),
    previewUrl: image(`remote-${index + 1}`),
    mime: "image/jpeg",
    size: 240000 + index * 16000,
    createdAt: "2026-07-17T10:00:00Z",
    canPreview: true
  }))
];

const taskHandlers = new Set<(event: TaskEvent) => void>();
const previewHandlers = new Set<(view: PreviewServerView) => void>();
let preview: PreviewServerView = { projectId: session.projectId, sessionGeneration: 1, state: "stopped", port: 4000, draftsEnabled: true };

function project(): OpenProjectResult {
  return { session: { ...session }, articles: structuredClone(articles) };
}

function emit(event: TaskEvent) {
  taskHandlers.forEach((handler) => handler(event));
}

export const browserMock = {
  loadConfig: async () => ({ config: structuredClone(config), warnings: [] }),
  saveConfig: async (next: AppConfigV3) => (config = structuredClone(next)),
  resetConfig: async () => (config = structuredClone(defaultConfig)),
  pickProject: async () => project(),
  reopenRecentProject: async () => typeof location !== "undefined" && new URLSearchParams(location.search).get("welcome") === "1" ? null : project(),
  listRecentProjects: async () => structuredClone(recent),
  openRecentProject: async (recentId: string) => {
    if (recentId !== "demo-recent") throw { code: "recent_unavailable", message: "项目位置已不可用。", recoverable: true };
    return project();
  },
  removeRecentProject: async (recentId: string) => { recent = recent.filter((item) => item.recentId !== recentId); },
  clearRecentProjects: async () => { recent = []; },
  listArticles: async () => structuredClone(articles),
  loadDocument: async (projectId: string, articleId: string): Promise<DocumentSnapshot> => ({ projectId, articleId, content: documents.get(articleId) ?? "", revision: 1, sessionGeneration: 1 }),
  saveDocument: async (request: SaveDocumentRequest) => {
    if (typeof document !== "undefined") {
      const current = Number(document.documentElement.dataset.editorSaveCalls ?? "0");
      document.documentElement.dataset.editorSaveCalls = String(current + 1);
    }
    if (typeof location !== "undefined" && new URLSearchParams(location.search).get("saveFail") === "1") {
      throw { code: "save_failed", message: "模拟保存失败。", recoverable: true };
    }
    documents.set(request.articleId, request.content);
    return { articleId: request.articleId, acceptedRevision: request.revision, savedAt: new Date().toISOString() };
  },
  createArticle: async (request: { title: string; kind: "post" | "draft" }) => {
    const item: ArticleSummary = { articleId: `new-${Date.now()}`, relativePath: `source/_${request.kind === "post" ? "posts" : "drafts"}/new.md`, title: request.title, kind: request.kind, modifiedAt: new Date().toISOString(), tags: [], categories: [], cover: { source: "placeholder", alt: "无封面" } };
    articles = [item, ...articles]; documents.set(item.articleId, `# ${item.title}\n`); return structuredClone(item);
  },
  listLocalImages: async () => structuredClone(localImages),
  importLocalImages: async () => structuredClone(localImages),
  deleteLocalImage: async () => undefined,
  revealLocalImage: async () => undefined,
  resolveRemotePreviewImages: async (urls: string[]): Promise<RemotePreviewImageResult[]> =>
    urls.map((originalUrl) => ({
      originalUrl,
      state: "ready",
      previewUrl: "data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///ywAAAAAAQABAAACAUwAOw=="
    })),
  uploadCloudflareImage: async () => ({ url: remoteAssets[3].url!, markdown: `![${remoteAssets[3].name}](${remoteAssets[3].url})`, fileName: remoteAssets[3].fileName }),
  listCloudflareAssets: async (_offset: number, _count: number, search: string, directory: string): Promise<RemoteAssetPage> => {
    const normalized = directory.replace(/^\/+|\/+$/g, "");
    const items = remoteAssets.filter((item) => {
      if (search) return item.name.toLocaleLowerCase().includes(search.toLocaleLowerCase());
      if (item.kind === "folder") return !normalized && !item.directory.includes("/");
      return item.directory === normalized;
    });
    const breadcrumbs = [{ name: "根目录", directory: "" }];
    let cursor = "";
    for (const name of normalized.split("/").filter(Boolean)) {
      cursor = cursor ? `${cursor}/${name}` : name;
      breadcrumbs.push({ name, directory: cursor });
    }
    return { currentDirectory: normalized, breadcrumbs, items: structuredClone(items), totalCount: items.length, returnedCount: items.length };
  },
  deleteCloudflareAsset: async () => undefined,
  credentialStatus: async () => ({ configured: true }),
  credentialSet: async () => ({ configured: true }),
  credentialDelete: async () => ({ configured: false }),
  acquireCloudflareImgbedToken: async (
    request: AcquireCloudflareImgbedTokenRequest
  ): Promise<AcquireCloudflareImgbedTokenResult> => ({
    configured: true,
    tokenId: "mock-token-id",
    tokenName: request.tokenName || "Hexo Lite Editor",
    owner: request.owner || "Hexo Lite Editor",
    permissions: request.permissions || ["upload", "list", "delete"],
    createdAt: new Date().toISOString(),
    expiresAt: request.expiresAt ?? null
  }),
  testCloudflareImgbedToken: async (baseUrl: string) => ({
    ok: true,
    baseUrl: baseUrl.replace(/\/+$/, ""),
    listEndpoint: `${baseUrl.replace(/\/+$/, "")}/api/manage/list?start=0&count=1`,
    message: "Cloudflare-ImgBed 连接正常。"
  }),
  getPreviewStatus: async (_projectId?: string, _sessionGeneration?: number) => structuredClone(preview),
  startPreviewServer: async (_projectId?: string, _sessionGeneration?: number) => {
    preview = { ...preview, state: "starting", error: undefined };
    previewHandlers.forEach((handler) => handler(structuredClone(preview)));
    setTimeout(() => {
      preview = { ...preview, state: "running", baseUrl: "http://127.0.0.1:4000/", startedAt: new Date().toISOString() };
      previewHandlers.forEach((handler) => handler(structuredClone(preview)));
    }, 180);
    return structuredClone(preview);
  },
  stopPreviewServer: async (_projectId?: string, _sessionGeneration?: number) => {
    preview = { ...preview, state: "stopped", baseUrl: undefined, startedAt: undefined };
    previewHandlers.forEach((handler) => handler(structuredClone(preview)));
    return structuredClone(preview);
  },
  resolveArticlePreviewUrl: async (articleId: string) => `http://127.0.0.1:4000/posts/${articleId}/`,
  onPreviewStatus: async (handler: (view: PreviewServerView) => void) => { previewHandlers.add(handler); return () => previewHandlers.delete(handler); },
  listTaskLogs: async (): Promise<TaskLogSummary[]> => [],
  readTaskLog: async (_taskId?: string): Promise<TaskLogPage> => ({ events: [] }),
  deleteTaskLog: async (_taskId?: string) => undefined,
  clearTaskLogs: async () => undefined,
  onTaskEvent: async (handler: (event: TaskEvent) => void) => { taskHandlers.add(handler); return () => taskHandlers.delete(handler); },
  startTask: async (_projectId: string, kind: TaskType) => {
    if (typeof document !== "undefined") {
      document.documentElement.dataset.taskStarts = String(Number(document.documentElement.dataset.taskStarts ?? "0") + 1);
    }
    const taskId = `demo-task-${Date.now()}`;
    const step = kind === "publish" ? "生成站点" : kind === "serverStart" ? "启动预览" : kind;
    const base = { taskId, projectId: session.projectId, timestamp: new Date().toISOString() };
    emit({ ...base, sequence: 0, kind: "queued" });
    setTimeout(() => emit({ ...base, sequence: 1, kind: "stepStarted", step }), 50);
    setTimeout(() => emit({ ...base, sequence: 2, kind: "log", step, stream: "stdout", line: `正在执行 ${kind}…` }), 120);
    if (kind !== "serverStart") {
      setTimeout(() => emit({ ...base, sequence: 3, kind: "stepFinished", step, success: true, exitCode: 0 }), 220);
      setTimeout(() => emit({ ...base, sequence: 4, kind: "finished", success: true, exitCode: 0 }), 260);
    }
    return { taskId };
  },
  cancelTask: async (taskId: string) => emit({ taskId, projectId: session.projectId, sequence: 999, kind: "finished", success: true, exitCode: 0, timestamp: new Date().toISOString() })
};
