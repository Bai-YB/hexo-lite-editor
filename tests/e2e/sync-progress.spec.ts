import { expect, test } from "./fixtures";

test("a slow GitHub preflight leaves settings responsive and keeps provider forms separate", async ({ page }) => {
  await page.goto("/?demo=1&syncPublic=1");
  await expect(page.locator(".cm-editor")).toBeVisible({ timeout: 20_000 });
  await page.evaluate(async () => {
    const modulePath = "/src/platform/tauri.ts";
    const { platform } = await import(/* @vite-ignore */ modulePath);
    const original = platform.preflightContentSync;
    let release!: () => void;
    const gate = new Promise<void>((resolve) => { release = resolve; });
    (window as unknown as { releaseSyncPreflight: () => void }).releaseSyncPreflight = release;
    platform.preflightContentSync = async (...args: Parameters<typeof original>) => {
      await gate;
      return original(...args);
    };
  });
  await page.getByRole("button", { name: "设置", exact: true }).click();
  await page.getByRole("button", { name: /文件同步/ }).click();
  const panel = page.locator(".settings-content-panel");
  await expect(panel.getByRole("radio", { name: /^GitHub/ })).toHaveAttribute("aria-checked", "true");
  await expect(panel.getByLabel("WebDAV 服务器地址")).toHaveCount(0);
  await panel.getByRole("button", { name: "检查连接与差异" }).click();
  await expect(panel.getByText("正在检查 GitHub 连接和两端文件差异。")).toBeVisible();
  await expect(panel.getByRole("button", { name: "停止同步" })).toBeEnabled();
  await page.getByRole("button", { name: "常规", exact: true }).click();
  await expect(panel.getByRole("heading", { name: "常规", level: 2 })).toBeVisible();
  await page.evaluate(() => (window as unknown as { releaseSyncPreflight: () => void }).releaseSyncPreflight());
});

test("upload progress remains responsive, can stop, and only retries after the operation ends", async ({ page }) => {
  await page.goto("/?demo=1");
  await expect(page.locator(".cm-editor")).toBeVisible({ timeout: 20000 });
  await page.evaluate(async () => {
    const modulePath = "/src/platform/tauri.ts";
    const { platform } = await import(/* @vite-ignore */ modulePath);
    const status = { enabled: true, provider: "github", status: "synced", conflicts: [], lastSyncedAt: "2026-09-10T00:00:00Z" } as const;
    let handler: (event: unknown) => void = () => {};
    let finish: (value: unknown) => void = () => {};
    platform.getContentSyncStatus = async () => ({ ...status, conflicts: [] });
    platform.onContentSyncPhase = async (next: typeof handler) => { handler = next; return () => {}; };
    platform.runContentSync = async () => {
      handler({ projectId: "demo-project", sessionGeneration: 1, phase: "uploading", status: "checking", message: "正在上传 source/_data/links.yml", completedFiles: 3, totalFiles: 10 });
      return await new Promise(resolve => { finish = resolve; });
    };
    platform.cancelContentSync = async () => {
      setTimeout(() => finish({ ...status, conflicts: [], status: "error", message: "同步已停止，可重试。" }), 300);
      return true;
    };
  });
  await page.getByRole("button", { name: "设置", exact: true }).click();
  await page.getByRole("button", { name: /文件同步/ }).click();
  await page.getByRole("button", { name: "立即同步" }).click();
  await expect(page.getByRole("progressbar")).toHaveAttribute("value", "3");
  await expect(page.getByRole("button", { name: "立即同步" })).toBeDisabled();
  await page.getByRole("button", { name: "停止同步" }).click();
  await expect(page.getByText("正在停止同步...")).toBeVisible();
  await expect(page.getByRole("button", { name: "重试同步" })).toBeEnabled();
  await expect(page.getByRole("progressbar")).toHaveCount(0);
});
