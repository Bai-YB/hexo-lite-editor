import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import ImageBedPage from "./ImageBedPage.svelte";
import { setLanguage } from "$shared/i18n";
import { defaultConfig, type AppConfigV3, type ProjectSessionView } from "$shared/types/app";

const api = vi.hoisted(() => ({ credentialStatus: vi.fn(), listLocalImages: vi.fn(), importLocalImages: vi.fn(), listCloudflareAssets: vi.fn(), deleteCloudflareAsset: vi.fn(), renameCloudflareAsset: vi.fn(), moveCloudflareAsset: vi.fn() }));
vi.mock("$platform/tauri", () => ({ platform: api, normalizeError: (error: Error) => ({ message: error.message }) }));
const session: ProjectSessionView = { projectId: "project", generation: 1, name: "Test", displayPath: "fixture", warnings: [] };
const config = (remote = false): AppConfigV3 => ({ ...structuredClone(defaultConfig), imageBed: { ...defaultConfig.imageBed, defaultProvider: remote ? "cloudflare-imgbed" : "local", cloudflareApiUrl: "https://fixture.invalid" } });
const local = { imageId: "one", name: "one.png", markdownUrl: "/images/one.png", previewUrl: "/fixture.png", size: 20 };
const page = (name: string | null, totalCount = 1) => ({ items: name ? [{ assetId: name, name, fileName: name, kind: "image", directory: "", url: `https://fixture.invalid/${name}`, previewUrl: `https://fixture.invalid/${name}`, capabilities: { preview: true, delete: true, rename: true, move: true } }] : [], totalCount, breadcrumbs: [{ name: "根目录", directory: "" }], currentDirectory: "" });
const deferred = <T>() => { let resolve!: (value: T) => void; const promise = new Promise<T>((done) => { resolve = done; }); return { promise, resolve }; };

beforeEach(() => {
  vi.resetAllMocks();
  setLanguage("zh-CN");
  vi.stubGlobal("requestAnimationFrame", () => 0);
  Object.defineProperty(HTMLElement.prototype, "scrollTo", { configurable: true, value: () => {} });
  Object.defineProperty(Element.prototype, "animate", { configurable: true, value: () => {
    const animation = { currentTime: 0, playState: "finished", effect: null, onfinish: null as null | (() => void), cancel() {}, finish() {} };
    queueMicrotask(() => animation.onfinish?.());
    return animation;
  } });
  api.credentialStatus.mockResolvedValue({ configured: true });
  api.listLocalImages.mockResolvedValue([local]);
  api.listCloudflareAssets.mockResolvedValue(page("one.png"));
});
afterEach(() => { cleanup(); vi.unstubAllGlobals(); });

describe("image bed interactions", () => {
  it("keeps existing images and emits no success notice when import is cancelled", async () => {
    const notice = vi.fn();
    api.importLocalImages.mockResolvedValue({ canceled: true, importedCount: 0, images: [], failures: [] });
    render(ImageBedPage, { session, config: config(), onNotice: notice });
    await screen.findByRole("button", { name: /one.png，Enter/ });
    await fireEvent.click(screen.getByRole("button", { name: "导入" }));
    expect(screen.getByRole("button", { name: /one.png，Enter/ })).toBeTruthy();
    expect(notice).not.toHaveBeenCalled();
  });
  it("shows successful imports and identifies failed files", async () => {
    const notice = vi.fn();
    api.importLocalImages.mockResolvedValue({ canceled: false, importedCount: 1, images: [local], failures: [{ fileName: "large.png", error: { message: "图片不能超过 25 MB。" } }] });
    render(ImageBedPage, { session, config: config(), onNotice: notice });
    await screen.findByRole("button", { name: /one.png，Enter/ });
    await fireEvent.click(screen.getByRole("button", { name: "导入" }));
    expect(notice).toHaveBeenCalledWith(expect.stringContaining("1 张图片已导入，1 张失败：large.png"));
  });
  it("returns to the last valid page when deletion empties page two", async () => {
    let total = 49;
    api.listCloudflareAssets.mockImplementation((_project, _generation, offset) => Promise.resolve(offset ? page(total === 49 ? "last.png" : null, total) : page("first.png", total)));
    api.deleteCloudflareAsset.mockImplementation(async () => { total = 48; });
    render(ImageBedPage, { session, config: config(true) });
    await screen.findByRole("button", { name: /first.png，Enter/ });
    await fireEvent.click(screen.getByRole("button", { name: "下一页" }));
    await screen.findByRole("button", { name: /last.png，Enter/ });
    await fireEvent.click(screen.getByRole("button", { name: "打开 last.png 菜单" }));
    await fireEvent.click(screen.getByRole("menuitem", { name: "删除远程资源" }));
    await fireEvent.click(screen.getByRole("button", { name: "确认删除" }));
    await screen.findByRole("button", { name: /first.png，Enter/ });
    expect(screen.getByText("48 项")).toBeTruthy();
    expect(screen.queryByText("当前目录为空")).toBeNull();
  });
  it("ignores obsolete responses after a newer search has completed", async () => {
    const older = deferred<ReturnType<typeof page>>();
    const newer = deferred<ReturnType<typeof page>>();
    api.listCloudflareAssets.mockImplementation((_p, _g, _o, _c, query) => query === "older" ? older.promise : query === "newer" ? newer.promise : Promise.resolve(page("initial.png")));
    render(ImageBedPage, { session, config: config(true) });
    await screen.findByRole("button", { name: /initial.png，Enter/ });
    const input = screen.getByRole("textbox", { name: "搜索资源" });
    for (const value of ["older", "newer"]) {
      await fireEvent.input(input, { target: { value } });
      await fireEvent.submit(input.closest("form")!);
      await waitFor(() => expect(api.listCloudflareAssets).toHaveBeenCalledWith("project", 1, 0, 48, value, ""));
    }
    newer.resolve(page("newer.png"));
    await screen.findByRole("button", { name: /newer.png，Enter/ });
    older.resolve(page("older.png"));
    await waitFor(() => expect(screen.queryByRole("button", { name: /older.png，Enter/ })).toBeNull());
    expect(screen.getByText("搜索：newer")).toBeTruthy();
  });
  it("submits rename once and prevents cancellation while the mutation is pending", async () => {
    api.renameCloudflareAsset.mockReturnValue(new Promise(() => {}));
    render(ImageBedPage, { session, config: config(true) });
    await screen.findByRole("button", { name: /one.png，Enter/ });
    await fireEvent.click(screen.getByRole("button", { name: "打开 one.png 菜单" }));
    await fireEvent.click(screen.getByRole("menuitem", { name: "重命名" }));
    const input = screen.getByRole("textbox", { name: "新名称" });
    await fireEvent.input(input, { target: { value: "renamed.png" } });
    await fireEvent.keyDown(input, { key: "Enter" });
    expect(api.renameCloudflareAsset).toHaveBeenCalledTimes(1);
    expect((screen.getByRole("button", { name: "取消" }) as HTMLButtonElement).disabled).toBe(true);
  });
  it("keeps the delete target through Escape and removes it after success", async () => {
    const deletion = deferred<void>();
    api.deleteCloudflareAsset.mockReturnValue(deletion.promise);
    render(ImageBedPage, { session, config: config(true) });
    await screen.findByRole("button", { name: /one.png，Enter/ });
    await fireEvent.click(screen.getByRole("button", { name: "打开 one.png 菜单" }));
    await fireEvent.click(screen.getByRole("menuitem", { name: "删除远程资源" }));
    await fireEvent.click(screen.getByRole("button", { name: "确认删除" }));
    await fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.getByRole("dialog", { name: "删除远程资源？" })).toBeTruthy();
    api.listCloudflareAssets.mockResolvedValue(page(null, 0));
    deletion.resolve();
    await screen.findByText("当前目录为空");
    expect(screen.getByText("0 项")).toBeTruthy();
    expect(screen.queryByRole("button", { name: /one.png，Enter/ })).toBeNull();
  });
});
