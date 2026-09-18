import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, waitFor } from "@testing-library/svelte";
import { tick } from "svelte";
import SettingsPage from "./SettingsPage.svelte";
import { defaultConfig, type AppConfigV3, type ContentSyncView } from "$shared/types/app";
import { platform } from "$platform/tauri";
import { setLanguage } from "$shared/i18n";

vi.mock("$platform/tauri", () => ({
  normalizeError: (error: unknown) => ({ message: error instanceof Error ? error.message : String(error) }),
  platform: {
    credentialStatus: vi.fn(async () => ({ configured: false })),
    credentialLegacyAvailable: vi.fn(async () => false),
    detectContentSync: vi.fn(async () => ({ candidates: [], requiresSelection: false })),
    getContentSyncStatus: vi.fn(),
    getContentSyncSummary: vi.fn(async () => ({ fileCount: 9, totalBytes: 4096, pendingFileCount: 2, pendingBytes: 1024, deletedFileCount: 1, baselineAvailable: true, categories: [{ id: "articles", fileCount: 3, totalBytes: 1024 }, { id: "site", fileCount: 2, totalBytes: 1024 }, { id: "themes", fileCount: 2, totalBytes: 1024 }, { id: "assets", fileCount: 2, totalBytes: 1024 }] })),
    getContentSyncProgress: vi.fn(async () => null),
    getContentSyncConflicts: vi.fn(async () => [{ path: "source/_posts/post.md", kind: "markdown", localText: "local", remoteText: "remote" }]),
    webDavCredentialStatus: vi.fn(async () => ({ configured: true, username: "writer" })),
    testWebDavContentSync: vi.fn(),
    resolveContentSyncConflicts: vi.fn(async () => ({ enabled: true, provider: "webdav", status: "synced", conflicts: [] })),
    runContentSync: vi.fn(), cancelContentSync: vi.fn(async () => true),
    onContentSyncPhase: vi.fn(async () => () => {}), onContentSyncStatus: vi.fn(async () => () => {}), listPlugins: vi.fn(async () => [])
  }
}));

beforeEach(() => {
  localStorage.clear();
  vi.clearAllMocks();
  setLanguage("zh-CN");
  vi.stubGlobal("requestAnimationFrame", () => 0);
  Object.defineProperty(Element.prototype, "animate", { configurable: true, value: () => {
    const animation = { currentTime: 0, playState: "finished", effect: null, onfinish: null as null | (() => void), cancel() {}, finish() {} };
    queueMicrotask(() => animation.onfinish?.());
    return animation;
  } });
});
afterEach(() => { cleanup(); vi.unstubAllGlobals(); });

describe("settings interaction recovery", () => {
  it("defers project scanning until file sync opens and shows only the chosen provider form", async () => {
    vi.mocked(platform.getContentSyncStatus).mockResolvedValue({ enabled: false, provider: "github", status: "off", conflicts: [] });
    vi.mocked(platform.detectContentSync).mockResolvedValue({
      requiresSelection: false,
      candidates: [{ repository: "https://github.com/example/blog.git", source: "Hexo deploy 配置", visibility: "private" }]
    });
    const view = render(SettingsPage, {
      config: structuredClone(defaultConfig),
      session: { projectId: "project", generation: 1, name: "Project", displayPath: "fixture", warnings: [] },
      initialSection: "general"
    });
    await tick();
    expect(platform.detectContentSync).not.toHaveBeenCalled();
    await fireEvent.click(view.getByRole("button", { name: "文件同步" }));
    const webDavChoice = await waitFor(() => view.getByRole("radio", { name: /^WebDAV/ }));
    expect(view.getByRole("radio", { name: /^GitHub/ }).getAttribute("aria-checked")).toBe("true");
    expect(view.queryByLabelText("WebDAV 服务器地址")).toBeNull();
    await fireEvent.click(webDavChoice);
    expect(view.getByLabelText("WebDAV 服务器地址")).toBeTruthy();
    expect(view.queryByLabelText("内容分支")).toBeNull();
  });

  it("does not replace a newer WebDAV form with a late connection test", async () => {
    const status: ContentSyncView = { enabled: false, provider: "webdav", status: "off", conflicts: [] };
    vi.mocked(platform.getContentSyncStatus).mockResolvedValue(status);
    let finish!: (value: import("$shared/types/app").WebDavConnectionTestResult) => void;
    vi.mocked(platform.testWebDavContentSync).mockImplementation(() => new Promise(resolve => { finish = resolve; }));
    const view = render(SettingsPage, { config: structuredClone(defaultConfig), session: { projectId: "project", generation: 1, name: "Project", displayPath: "fixture", warnings: [] }, initialSection: "sync" });
    const webDavChoice = await waitFor(() => view.getByRole("radio", { name: /^WebDAV/ }));
    await fireEvent.click(webDavChoice);
    await fireEvent.input(view.getByLabelText("WebDAV 服务器地址"), { target: { value: "https://old.example/dav" } });
    await fireEvent.input(view.getByLabelText("WebDAV 用户名"), { target: { value: "writer" } });
    await fireEvent.input(view.getByLabelText("WebDAV 密码"), { target: { value: "old-password" } });
    await fireEvent.click(view.getByRole("button", { name: "保存并测试连接" }));
    await fireEvent.input(view.getByLabelText("WebDAV 服务器地址"), { target: { value: "https://new.example/dav" } });
    finish({ username: "writer", testedAt: "2026-09-11", sync: status, preflight: { endpoint: "https://old.example/dav", remoteDir: "hexo-lite-content", fileCount: 1, totalBytes: 4, remoteFileCount: 0, remoteTotalBytes: 0, localOnlyCount: 1, remoteOnlyCount: 0, differentCount: 0, remoteExists: false, remoteManifestValid: false } });
    await waitFor(() => expect(view.getByRole("alert").textContent).toContain("用当前输入重新测试"));
    expect((view.getByLabelText("WebDAV 服务器地址") as HTMLInputElement).value).toBe("https://new.example/dav");
    expect(view.queryByText("连接测试通过")).toBeNull();
  });
  it("shows real file progress, rejects duplicate upload, and keeps cancellation pending until the operation finishes", async () => {
    let finish!: (status: ContentSyncView) => void;
    const status: ContentSyncView = { enabled: true, provider: "github", status: "synced", conflicts: [], lastSyncedAt: "2026-09-10T00:00:00Z" };
    vi.mocked(platform.getContentSyncStatus).mockResolvedValue(status);
    vi.mocked(platform.runContentSync).mockImplementation(() => new Promise(resolve => { finish = resolve; }));
    const view = render(SettingsPage, { config: structuredClone(defaultConfig), session: { projectId: "project", generation: 1, name: "Project", displayPath: "fixture", warnings: [] }, initialSection: "sync" });
    await waitFor(() => expect(view.getByRole("button", { name: "立即同步" })).toBeTruthy());
    await fireEvent.click(view.getByRole("button", { name: "立即同步" }));
    await waitFor(() => expect(platform.runContentSync).toHaveBeenCalledOnce());
    const handler = vi.mocked(platform.onContentSyncPhase).mock.calls[0][0];
    handler({ projectId: "project", sessionGeneration: 1, phase: "uploading", status: "checking", message: "upload source/links.yml", completedFiles: 2, totalFiles: 6 });
    await tick();
    expect(view.getByRole("progressbar").getAttribute("value")).toBe("2");
    expect((view.getByRole("button", { name: "立即同步" }) as HTMLButtonElement).disabled).toBe(true);
    await fireEvent.click(view.getByRole("button", { name: "停止同步" }));
    expect(platform.cancelContentSync).toHaveBeenCalledWith("project", 1);
    expect(view.getByText("正在停止同步…")).toBeTruthy();
    finish({ ...status, status: "error", message: "stopped" });
    await waitFor(() => expect(view.getByRole("button", { name: "重试同步" })).toBeTruthy());
    await waitFor(() => expect(view.queryByRole("progressbar")).toBeNull());
  });
  it("switches between GitHub and WebDAV inline without turning sync off", async () => {
    const status: ContentSyncView = { enabled: true, provider: "github", status: "synced", conflicts: [], repository: "https://github.com/example/blog.git", branch: "hexo-lite-content", lastSyncedAt: "2026-09-10T00:00:00Z" };
    vi.mocked(platform.getContentSyncStatus).mockResolvedValue(status);
    vi.mocked(platform.detectContentSync).mockResolvedValue({ requiresSelection: false, candidates: [{ repository: "https://github.com/example/blog.git", source: "Hexo deploy 配置", visibility: "private" }] });
    const view = render(SettingsPage, { config: structuredClone(defaultConfig), session: { projectId: "project", generation: 1, name: "Project", displayPath: "fixture", warnings: [] }, initialSection: "sync" });
    await waitFor(() => expect(view.getByRole("tab", { name: "WebDAV" })).toBeTruthy());
    expect(view.queryByText("同步规划")).toBeNull();
    await waitFor(() => expect(view.getByText(/待上传/)).toBeTruthy());
    await fireEvent.click(view.getByRole("tab", { name: "WebDAV" }));
    expect(view.getByLabelText("WebDAV 服务器地址")).toBeTruthy();
    expect(view.getByText("切换后本机改用新通道同步，旧通道上的文件不会被删除。")).toBeTruthy();
    expect((view.getByRole("button", { name: "切换到 WebDAV 并合并" }) as HTMLButtonElement).disabled).toBe(true);
    await fireEvent.click(view.getByRole("tab", { name: "GitHub" }));
    expect(view.queryByLabelText("WebDAV 服务器地址")).toBeNull();
    expect(view.getByRole("button", { name: "立即同步" })).toBeTruthy();
  });
  it("writes after the debounce settles and re-persists changes typed afterwards", async () => {
    let resolve!: (config: AppConfigV3) => void;
    let submitted!: AppConfigV3;
    const save = vi.fn((value: AppConfigV3) => {
      submitted = value;
      return new Promise<AppConfigV3>((ok) => { resolve = ok; });
    });
    const config = structuredClone(defaultConfig); config.general.language = "zh-CN";
    const view = render(SettingsPage, { config, initialSection: "editing", onSaveConfig: save });
    await fireEvent.input(view.getByLabelText("字号"), { target: { value: "18" } });
    await waitFor(() => expect(save).toHaveBeenCalledOnce());
    expect(submitted.editor.fontSize).toBe(18);
    resolve(structuredClone(submitted));
    await fireEvent.input(view.getByLabelText("字号"), { target: { value: "20" } });
    await waitFor(() => expect(save).toHaveBeenCalledTimes(2));
    expect(submitted.editor.fontSize).toBe(20);
    resolve(structuredClone(submitted));
    await waitFor(() => expect(view.container.querySelector(".settings-save-state")?.textContent).toBe("已保存"));
    expect((view.getByLabelText("字号") as HTMLInputElement).value).toBe("20");
  });

  it("previews the saved theme when the selection returns to its original value", async () => {
    const config = structuredClone(defaultConfig); config.appearance.themeMode = "light";
    const preview = vi.fn();
    const save = vi.fn(async (value: AppConfigV3) => value);
    const view = render(SettingsPage, { config, initialSection: "editing", onThemePreview: preview, onSaveConfig: save });
    const theme = view.getByLabelText("主题模式");
    await fireEvent.change(theme, { target: { value: "dark" } });
    await fireEvent.change(theme, { target: { value: "light" } });
    expect(preview).toHaveBeenLastCalledWith("light");
    await new Promise((resolve) => setTimeout(resolve, 500));
    expect(save).not.toHaveBeenCalled();
  });

  it("rolls back an invalid value once the auto-save rejects it", async () => {
    const save = vi.fn(async (value: AppConfigV3) => value);
    const config = structuredClone(defaultConfig); config.general.language = "zh-CN";
    const view = render(SettingsPage, { config, initialSection: "editing", onSaveConfig: save });
    await fireEvent.input(view.getByLabelText("字号"), { target: { value: "18" } });
    await waitFor(() => expect(save).toHaveBeenCalledOnce());
    expect(save.mock.calls[0][0].editor.fontSize).toBe(18);
    await fireEvent.input(view.getByLabelText("字号"), { target: { value: "40" } });
    await waitFor(() => expect(view.getByRole("alert").textContent).toContain("12–28"));
    const field = view.getByLabelText("字号") as HTMLInputElement;
    expect(field.value).toBe("18");
    await waitFor(() => expect(document.activeElement).toBe(field));
    await new Promise((resolve) => setTimeout(resolve, 500));
    expect(save).toHaveBeenCalledTimes(1);
  });

  it("saves the service address on the fly and opens the Token dialog without a manual save", async () => {
    const config = structuredClone(defaultConfig); config.general.language = "zh-CN";
    const save = vi.fn(async (value: AppConfigV3) => value);
    const view = render(SettingsPage, { config, initialSection: "editing", onSaveConfig: save });
    await fireEvent.input(view.getByLabelText("字号"), { target: { value: "20" } });
    await fireEvent.click(view.getByRole("button", { name: /^图片/ }));
    await fireEvent.change(view.getByLabelText("图片保存到"), { target: { value: "cloudflare-imgbed" } });
    await fireEvent.change(view.getByLabelText("服务地址"), { target: { value: "https://img.example.com" } });
    await fireEvent.click(view.getByRole("button", { name: "一键获取 Token" }));
    await waitFor(() => expect(view.getByRole("dialog")).toBeTruthy());
    const latest = save.mock.calls.at(-1)![0] as AppConfigV3;
    expect(latest.editor.fontSize).toBe(20);
    expect(latest.imageBed.cloudflareApiUrl).toBe("https://img.example.com");
    expect(view.queryByRole("button", { name: /^保存$/ })).toBeNull();
  });

  it("shows WebDAV conflicts and requires an explicit choice for each file", async () => {
    vi.mocked(platform.getContentSyncStatus).mockResolvedValue({ enabled: true, provider: "webdav", status: "conflict", endpoint: "https://dav.example.com", remoteDir: "blog", conflicts: ["source/_posts/post.md"] });
    const guard = vi.fn(async () => true);
    const view = render(SettingsPage, { config: structuredClone(defaultConfig), session: { projectId: "project", generation: 1, name: "Project", displayPath: "fixture", warnings: [] }, initialSection: "sync", onBeforeSync: guard });
    await waitFor(() => expect(view.getByText("source/_posts/post.md")).toBeTruthy());
    const submit = view.getByRole("button", { name: "提交选择" }) as HTMLButtonElement;
    expect(submit.disabled).toBe(true);
    expect(view.getByRole("button", { name: "重新检查冲突" })).toBeTruthy();
    await fireEvent.click(view.getByLabelText("云端", { exact: true }));
    await tick();
    expect(submit.disabled).toBe(false);
    await fireEvent.click(submit);
    await waitFor(() => expect(platform.resolveContentSyncConflicts).toHaveBeenCalledWith("project", 1, { "source/_posts/post.md": "remote" }));
    expect(guard).toHaveBeenCalledOnce();
  });

  it("shows the shared remote-ahead choices for WebDAV", async () => {
    const status: ContentSyncView = { enabled: true, provider: "webdav", status: "remoteAhead", endpoint: "https://dav.example.com", remoteDir: "blog", conflicts: [], lastSyncedAt: "2026-09-10T00:00:00Z" };
    vi.mocked(platform.getContentSyncStatus).mockResolvedValue(status);
    const view = render(SettingsPage, { config: structuredClone(defaultConfig), session: { projectId: "project", generation: 1, name: "Project", displayPath: "fixture", warnings: [] }, initialSection: "sync" });
    await waitFor(() => expect(view.getByRole("button", { name: "合并云端改动" })).toBeTruthy());
    expect(view.getByRole("button", { name: "用本机项目覆盖云端" }).closest("details")?.open).toBe(false);
    await fireEvent.click(view.getByText("高级操作", { exact: true }));
    expect(view.container.querySelector(".sync-danger-zone")).toBeTruthy();
  });

  it("summarizes all source categories and merges remote changes without an overwrite dialog", async () => {
    const status: ContentSyncView = { enabled: true, provider: "github", status: "remoteAhead", conflicts: [], lastSyncedAt: "2026-09-10T00:00:00Z" };
    vi.mocked(platform.getContentSyncStatus).mockResolvedValue(status);
    vi.mocked(platform.runContentSync).mockResolvedValue({ ...status, status: "synced" });
    const publish = vi.fn(async () => {});
    const view = render(SettingsPage, { config: structuredClone(defaultConfig), session: { projectId: "project", generation: 1, name: "Project", displayPath: "fixture", warnings: [] }, initialSection: "sync", onPublish: publish });
    await waitFor(() => expect(view.getByText("配置与页面")).toBeTruthy());
    for (const label of ["文章与草稿", "配置与页面", "主题与模块", "图片与资源"]) expect(view.getByText(label)).toBeTruthy();
    expect(view.container.querySelector(".sync-change-summary")?.textContent).toContain("待上传 2 个文件 · 1.0 KB");
    expect(view.container.querySelector(".sync-change-summary")?.textContent).toContain("删除 1 个文件");
    expect((view.getByRole("button", { name: "发布网站" }) as HTMLButtonElement).disabled).toBe(true);
    await fireEvent.click(view.getByRole("button", { name: "合并云端改动" }));
    await waitFor(() => expect(platform.runContentSync).toHaveBeenCalledWith("project", 1, "auto", false));
    expect(view.queryByRole("dialog")).toBeNull();
    await waitFor(() => expect((view.getByRole("button", { name: "发布网站" }) as HTMLButtonElement).disabled).toBe(false));
    expect(publish).not.toHaveBeenCalled();
    await fireEvent.click(view.getByRole("button", { name: "发布网站" }));
    expect(publish).toHaveBeenCalledOnce();
  });

  it("keeps automatic download opt-in and preserves it when toggling startup checks", async () => {
    const save = vi.fn(async (value: AppConfigV3) => value);
    const view = render(SettingsPage, { config: structuredClone(defaultConfig), initialSection: "maintenance", onSaveConfig: save });
    const download = view.getByLabelText("后台下载更新") as HTMLInputElement;
    expect(download.checked).toBe(false);
    await fireEvent.click(download);
    await fireEvent.click(view.getByLabelText("启动时检查更新"));
    expect(download.disabled).toBe(true);
    expect(download.checked).toBe(true);
    await waitFor(() => expect(save).toHaveBeenCalledOnce());
    expect(save.mock.calls[0][0].update).toEqual({ checkOnStart: false, autoDownload: true });
  });
});
