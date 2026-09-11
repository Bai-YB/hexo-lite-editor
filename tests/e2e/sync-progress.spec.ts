import { expect, test } from "@playwright/test";

test("upload progress remains responsive, can stop, and only retries after the operation ends", async ({ page }) => {
  await page.goto("/?demo=1");
  await expect(page.locator(".cm-editor")).toBeVisible();
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
  await page.getByRole("button", { name: /内容同步/ }).click();
  await page.getByRole("button", { name: "立即上传变更" }).click();
  await expect(page.getByRole("progressbar")).toHaveAttribute("value", "3");
  await expect(page.getByRole("button", { name: "立即上传变更" })).toBeDisabled();
  await page.getByRole("button", { name: "停止同步" }).click();
  await expect(page.getByText("正在停止同步...")).toBeVisible();
  await expect(page.getByRole("button", { name: "重试同步" })).toBeEnabled();
  await expect(page.getByRole("progressbar")).toHaveCount(0);
});
