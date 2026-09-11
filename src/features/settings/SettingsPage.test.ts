import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, waitFor } from "@testing-library/svelte";
import { tick } from "svelte";
import SettingsPage from "./SettingsPage.svelte";
import { defaultConfig, type AppConfigV3, type ContentSyncView } from "$shared/types/app";
import { platform } from "$platform/tauri";
import { setLanguage } from "$shared/i18n";
import type { SettingsController } from "./controller";

vi.mock("$platform/tauri", () => ({
  normalizeError: (error: unknown) => ({ message: error instanceof Error ? error.message : String(error) }),
  platform: {
    credentialStatus: vi.fn(async () => ({ configured: false })),
    credentialLegacyAvailable: vi.fn(async () => false),
    detectContentSync: vi.fn(async () => ({ candidates: [], requiresSelection: false })),
    getContentSyncStatus: vi.fn(),
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
  it("does not replace a newer WebDAV form with a late connection test", async () => {
    const status: ContentSyncView = { enabled: false, provider: "webdav", status: "off", conflicts: [] };
    vi.mocked(platform.getContentSyncStatus).mockResolvedValue(status);
    let finish!: (value: import("$shared/types/app").WebDavConnectionTestResult) => void;
    vi.mocked(platform.testWebDavContentSync).mockImplementation(() => new Promise(resolve => { finish = resolve; }));
    const view = render(SettingsPage, { config: structuredClone(defaultConfig), session: { projectId: "project", generation: 1, name: "Project", displayPath: "fixture", warnings: [] }, initialSection: "sync" });
    await waitFor(() => expect(platform.getContentSyncStatus).toHaveBeenCalled());
    await fireEvent.change(view.getByLabelText("同步方式"), { target: { value: "webdav" } });
    await fireEvent.input(view.getByLabelText("WebDAV 服务器地址"), { target: { value: "https://old.example/dav" } });
    await fireEvent.input(view.getByLabelText("WebDAV 用户名"), { target: { value: "writer" } });
    await fireEvent.input(view.getByLabelText("WebDAV 密码"), { target: { value: "old-password" } });
    await fireEvent.click(view.getByRole("button", { name: "保存并测试连接" }));
    await fireEvent.input(view.getByLabelText("WebDAV 服务器地址"), { target: { value: "https://new.example/dav" } });
    finish({ username: "writer", testedAt: "2026-09-11", sync: status, preflight: { endpoint: "https://old.example/dav", remoteDir: "hexo-lite-content", fileCount: 1, totalBytes: 4, remoteFileCount: 0, remoteTotalBytes: 0, localOnlyCount: 1, remoteOnlyCount: 0, differentCount: 0, remoteExists: false, remoteManifestValid: false } });
    await waitFor(() => expect(view.getByRole("alert").textContent).toContain("请重新测试当前输入"));
    expect((view.getByLabelText("WebDAV 服务器地址") as HTMLInputElement).value).toBe("https://new.example/dav");
    expect(view.queryByText("WebDAV 真实连接和预检通过")).toBeNull();
  });
  it("shows real file progress, rejects duplicate upload, and keeps cancellation pending until the operation finishes", async () => {
    let finish!: (status: ContentSyncView) => void;
    const status: ContentSyncView = { enabled: true, provider: "github", status: "synced", conflicts: [], lastSyncedAt: "2026-09-10T00:00:00Z" };
    vi.mocked(platform.getContentSyncStatus).mockResolvedValue(status);
    vi.mocked(platform.runContentSync).mockImplementation(() => new Promise(resolve => { finish = resolve; }));
    const view = render(SettingsPage, { config: structuredClone(defaultConfig), session: { projectId: "project", generation: 1, name: "Project", displayPath: "fixture", warnings: [] }, initialSection: "sync" });
    await waitFor(() => expect(view.getByRole("button", { name: "立即上传变更" })).toBeTruthy());
    await fireEvent.click(view.getByRole("button", { name: "立即上传变更" }));
    await waitFor(() => expect(platform.runContentSync).toHaveBeenCalledOnce());
    const handler = vi.mocked(platform.onContentSyncPhase).mock.calls[0][0];
    handler({ projectId: "project", sessionGeneration: 1, phase: "uploading", status: "checking", message: "upload source/links.yml", completedFiles: 2, totalFiles: 6 });
    await tick();
    expect(view.getByRole("progressbar").getAttribute("value")).toBe("2");
    expect((view.getByRole("button", { name: "立即上传变更" }) as HTMLButtonElement).disabled).toBe(true);
    await fireEvent.click(view.getByRole("button", { name: "停止同步" }));
    expect(platform.cancelContentSync).toHaveBeenCalledWith("project", 1);
    expect(view.getByText("正在停止同步...")).toBeTruthy();
    finish({ ...status, status: "error", message: "stopped" });
    await waitFor(() => expect(view.getByRole("button", { name: "重试同步" })).toBeTruthy());
    await waitFor(() => expect(view.queryByRole("progressbar")).toBeNull());
  });
  it.each(["success", "failure"])("waits for an in-flight %s before discarding to the actual saved settings", async (outcome) => {
    let resolve!: (config: AppConfigV3) => void;
    let reject!: (error: Error) => void;
    let submitted!: AppConfigV3;
    let controller!: SettingsController;
    const config = structuredClone(defaultConfig); config.general.language = "zh-CN";
    const save = vi.fn((value: AppConfigV3) => {
      submitted = value;
      return new Promise<AppConfigV3>((ok, fail) => { resolve = ok; reject = fail; });
    });
    const view = render(SettingsPage, { config, initialSection: "editing", onSaveConfig: save,
      onRegisterSettingsController: (value) => { if (value) controller = value; } });
    await fireEvent.change(view.getByLabelText("字号"), { target: { value: "18" } });
    await fireEvent.click(view.getByRole("button", { name: /^保存$/ }));
    await fireEvent.change(view.getByLabelText("字号"), { target: { value: "20" } });
    let discarded = false;
    const discard = Promise.resolve(controller.discard()).then(() => { discarded = true; });
    await tick();
    expect(discarded).toBe(false);
    expect(controller.hasDirty()).toBe(true);
    if (outcome === "success") resolve(submitted); else reject(new Error("config write failed"));
    await discard;
    await tick();
    expect((view.getByLabelText("字号") as HTMLInputElement).value).toBe(String(outcome === "success" ? 18 : config.editor.fontSize));
    expect(controller.hasDirty()).toBe(false);
    expect(view.queryByRole("button", { name: /^保存$/ })).toBeNull();
  });

  it("previews the saved theme when the selection returns to its original value", async () => {
    const config = structuredClone(defaultConfig); config.appearance.themeMode = "light";
    const preview = vi.fn();
    const view = render(SettingsPage, { config, initialSection: "editing", onThemePreview: preview });
    const theme = view.getByLabelText("主题模式");
    await fireEvent.change(theme, { target: { value: "dark" } });
    await fireEvent.change(theme, { target: { value: "light" } });
    expect(preview).toHaveBeenLastCalledWith("light");
    expect(view.queryByRole("button", { name: /^保存$/ })).toBeNull();
  });

  it("retains form input typed while saving and reports a validation failure", async () => {
    let finish!: (config: AppConfigV3) => void;
    let submitted!: AppConfigV3;
    const save = vi.fn((config: AppConfigV3) => { submitted = config; return new Promise<AppConfigV3>(resolve => { finish = resolve; }); });
    const config = structuredClone(defaultConfig); config.general.language = "zh-CN";
    const view = render(SettingsPage, { config, initialSection: "editing", onSaveConfig: save });
    await fireEvent.change(view.getByLabelText("字号"), { target: { value: "18" } });
    await fireEvent.click(view.getByRole("button", { name: /^保存$/ }));
    await fireEvent.change(view.getByLabelText("字号"), { target: { value: "20" } });
    finish(submitted);
    await waitFor(() => expect(view.getByRole("alert").textContent).toContain("保存期间又有设置变化"));
    expect((view.getByLabelText("字号") as HTMLInputElement).value).toBe("20");
    await fireEvent.change(view.getByLabelText("字号"), { target: { value: "40" } });
    await fireEvent.click(view.getByRole("button", { name: /^保存$/ }));
    await waitFor(() => expect(view.getByRole("alert").textContent).toContain("12–28"));
    expect(save).toHaveBeenCalledTimes(1);
    expect(document.activeElement).toBe(view.getByLabelText("字号"));
  });

  it("persists only the connection when preparing a Token and leaves other settings dirty", async () => {
    const config = structuredClone(defaultConfig); config.general.language = "zh-CN";
    const save = vi.fn(async (value: AppConfigV3) => value);
    const view = render(SettingsPage, { config, initialSection: "editing", onSaveConfig: save });
    await fireEvent.change(view.getByLabelText("字号"), { target: { value: "20" } });
    await fireEvent.click(view.getByRole("button", { name: /图片与图床/ }));
    await fireEvent.change(view.getByLabelText("默认来源"), { target: { value: "cloudflare-imgbed" } });
    await fireEvent.change(view.getByLabelText("服务地址"), { target: { value: "https://img.example.com" } });
    await fireEvent.click(view.getByRole("button", { name: "一键获取 Token" }));
    await waitFor(() => expect(save).toHaveBeenCalledOnce());
    expect(save.mock.calls[0][0].editor.fontSize).toBe(config.editor.fontSize);
    expect(save.mock.calls[0][0].imageBed.cloudflareApiUrl).toBe("https://img.example.com");
    await waitFor(() => expect(view.getByRole("dialog")).toBeTruthy());
    expect(view.getByRole("button", { name: /^保存$/ })).toBeTruthy();
  });

  it("shows WebDAV conflicts and requires an explicit choice for each file", async () => {
    vi.mocked(platform.getContentSyncStatus).mockResolvedValue({ enabled: true, provider: "webdav", status: "conflict", endpoint: "https://dav.example.com", remoteDir: "blog", conflicts: ["source/_posts/post.md"] });
    const guard = vi.fn(async () => true);
    const view = render(SettingsPage, { config: structuredClone(defaultConfig), session: { projectId: "project", generation: 1, name: "Project", displayPath: "fixture", warnings: [] }, initialSection: "sync", onBeforeSync: guard });
    await waitFor(() => expect(view.getByText("source/_posts/post.md")).toBeTruthy());
    const submit = view.getByRole("button", { name: "提交冲突选择" }) as HTMLButtonElement;
    expect(submit.disabled).toBe(true);
    expect(view.getByRole("button", { name: "重新检查冲突" })).toBeTruthy();
    await fireEvent.click(view.getByLabelText("远端", { exact: true }));
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
    await waitFor(() => expect(view.getByRole("button", { name: "使用云端最新版本" })).toBeTruthy());
    expect(view.getByRole("button", { name: "用本机项目覆盖云端" })).toBeTruthy();
  });
});
