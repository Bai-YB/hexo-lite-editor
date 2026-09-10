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
    getContentSyncConflicts: vi.fn(async () => [{ path: "source/_posts/post.md", kind: "markdown", localText: "local", remoteText: "remote" }]),
    webDavCredentialStatus: vi.fn(async () => ({ configured: true, username: "writer" })),
    resolveContentSyncConflicts: vi.fn(async () => ({ enabled: true, provider: "webdav", status: "synced", conflicts: [] })),
    runContentSync: vi.fn(), onContentSyncStatus: vi.fn(async () => () => {}), listPlugins: vi.fn(async () => [])
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
