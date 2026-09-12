import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import PluginManagerPage from "./PluginManagerPage.svelte";
import { setLanguage } from "$shared/i18n";

const mocks = vi.hoisted(() => ({
  api: { listPlugins: vi.fn(), getPluginSettingsSchema: vi.fn(), getPluginSettings: vi.fn(), savePluginSettings: vi.fn(), disablePlugin: vi.fn(), uninstallPlugin: vi.fn() },
  validate: vi.fn(), dispose: vi.fn()
}));
vi.mock("$platform/tauri", () => ({ platform: mocks.api, normalizeError: (error: Error) => ({ message: error.message }) }));
vi.mock("$shared/plugins/PluginProviderRuntime", () => ({
  contributedImageBedProviders: () => [], imageBedProviderId: (id: string) => `plugin:${id}`,
  validatePluginConfig: mocks.validate, testPluginConnection: vi.fn(), disposePluginWorker: mocks.dispose,
  reconcilePluginWorkers: vi.fn()
}));
const plugin = { enabled: true, entryUrl: "/worker.js", manifest: { id: "example", name: "Example", version: "1", contributes: { settings: "settings.schema.json" } } };
beforeEach(() => {
  vi.resetAllMocks();
  setLanguage("zh-CN");
  vi.stubGlobal("requestAnimationFrame", () => 0);
  Object.defineProperty(Element.prototype, "animate", { configurable: true, value: () => {
    const animation = { currentTime: 0, playState: "finished", effect: null, onfinish: null as null | (() => void), cancel() {}, finish() {} };
    queueMicrotask(() => animation.onfinish?.());
    return animation;
  } });
  mocks.api.listPlugins.mockResolvedValue([plugin]);
  mocks.api.getPluginSettings.mockResolvedValue({});
  mocks.api.getPluginSettingsSchema.mockResolvedValue({ required: ["endpoint"], properties: { endpoint: { type: "string", format: "uri", default: "https://example.com" } } });
  mocks.validate.mockResolvedValue({ ok: true });
});
afterEach(() => { cleanup(); vi.unstubAllGlobals(); });

describe("plugin manager interactions", () => {
  it("validates and saves the displayed schema defaults", async () => {
    render(PluginManagerPage);
    await screen.findByText("Example");
    await fireEvent.click(screen.getByRole("button", { name: "设置" }));
    await screen.findByDisplayValue("https://example.com");
    await fireEvent.click(screen.getByRole("button", { name: "保存" }));
    await waitFor(() => expect(mocks.api.savePluginSettings).toHaveBeenCalledWith("example", { endpoint: "https://example.com" }));
    expect(mocks.validate).toHaveBeenCalledWith(plugin, { endpoint: "https://example.com" });
  });
  it("does not persist a missing required field", async () => {
    render(PluginManagerPage);
    await screen.findByText("Example");
    await fireEvent.click(screen.getByRole("button", { name: "设置" }));
    const input = await screen.findByDisplayValue("https://example.com");
    await fireEvent.input(input, { target: { value: "" } });
    await fireEvent.click(screen.getByRole("button", { name: "保存" }));
    expect(screen.getByRole("alert").textContent).toContain("请填写");
    expect(mocks.api.savePluginSettings).not.toHaveBeenCalled();
  });
  it("preserves edited settings when continuing and closes only after explicit discard", async () => {
    render(PluginManagerPage);
    await screen.findByText("Example");
    await fireEvent.click(screen.getByRole("button", { name: "设置" }));
    const input = await screen.findByDisplayValue("https://example.com");
    await fireEvent.input(input, { target: { value: "https://changed.example.com" } });

    await fireEvent.click(screen.getByRole("button", { name: "取消" }));
    expect(screen.getByRole("dialog", { name: "放弃插件设置？" })).toBeTruthy();
    expect(screen.getByDisplayValue("https://changed.example.com")).toBeTruthy();
    expect(mocks.api.savePluginSettings).not.toHaveBeenCalled();

    await fireEvent.click(screen.getByRole("button", { name: "继续编辑" }));
    await waitFor(() => expect(screen.queryByRole("dialog", { name: "放弃插件设置？" })).toBeNull());
    expect(screen.getByRole("dialog", { name: "Example 设置" })).toBeTruthy();
    expect(screen.getByDisplayValue("https://changed.example.com")).toBeTruthy();

    await fireEvent.click(screen.getByRole("button", { name: "取消" }));
    await fireEvent.click(screen.getByRole("button", { name: "放弃修改" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(mocks.api.savePluginSettings).not.toHaveBeenCalled();

    await fireEvent.click(screen.getByRole("button", { name: "设置" }));
    await screen.findByDisplayValue("https://example.com");
    expect(screen.queryByDisplayValue("https://changed.example.com")).toBeNull();
  });
  it("recovers the plugin list after a failed load is retried", async () => {
    const notice = vi.fn();
    mocks.api.listPlugins.mockRejectedValueOnce(new Error("无法读取插件目录"));
    render(PluginManagerPage, { onNotice: notice });

    expect((await screen.findByRole("alert")).textContent).toContain("无法读取插件目录");
    expect(notice).toHaveBeenCalledWith("无法读取插件目录");
    expect(screen.queryByText("Example")).toBeNull();

    await fireEvent.click(screen.getByRole("button", { name: "重试" }));
    await screen.findByText("Example");
    expect(mocks.api.listPlugins).toHaveBeenCalledTimes(2);
    expect(screen.queryByRole("alert")).toBeNull();
    expect(screen.queryByRole("button", { name: "重试" })).toBeNull();
    expect((screen.getByRole("button", { name: "设置" }) as HTMLButtonElement).disabled).toBe(false);
  });
  it("requires confirmation and preserves settings by default when uninstalling", async () => {
    mocks.api.uninstallPlugin.mockResolvedValue([]);
    render(PluginManagerPage);
    await screen.findByText("Example");
    await fireEvent.click(screen.getByRole("button", { name: "卸载" }));
    expect(mocks.api.uninstallPlugin).not.toHaveBeenCalled();
    expect((screen.getByRole("checkbox") as HTMLInputElement).checked).toBe(true);
    await fireEvent.click(screen.getByRole("button", { name: "确认卸载" }));
    expect(mocks.api.uninstallPlugin).toHaveBeenCalledWith("example", true);
    expect(mocks.dispose).toHaveBeenCalledWith("example");
  });
  it("shows disable errors and leaves the plugin enabled", async () => {
    const notice = vi.fn();
    mocks.api.disablePlugin.mockRejectedValue(new Error("文件正在使用"));
    render(PluginManagerPage, { onNotice: notice });
    await screen.findByText("Example");
    await fireEvent.click(screen.getByRole("button", { name: "禁用" }));
    expect(notice).toHaveBeenCalledWith("文件正在使用");
    expect(screen.getByRole("button", { name: "禁用" })).toBeTruthy();
    expect(mocks.dispose).not.toHaveBeenCalled();
  });
});
