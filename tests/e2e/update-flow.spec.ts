import { expect, test } from "@playwright/test";

test("默认只后台检查，手动下载显示真实进度和五条日志，点击直接安装", async ({ page }) => {
  await page.goto("/?demo=1&updateAvailable=1");
  await expect(page.getByRole("button", { name: "查看更新", exact: true })).toBeVisible({ timeout: 8000 });
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await page.getByRole("button", { name: "查看更新", exact: true }).click();
  await expect(page.locator(".release-summary > ul > li")).toHaveCount(5);
  await expect(page.locator(".update-release-notes")).not.toBeVisible();
  await page.getByText("完整更新日志", { exact: true }).click();
  await expect(page.locator(".update-release-notes")).toContainText("支持减少动态效果");
  await page.getByRole("button", { name: "下载更新", exact: true }).click();
  const progress = page.getByRole("progressbar");
  await expect(progress).toBeVisible();
  await expect(progress).toHaveAttribute("aria-valuenow", /[1-9]/);
  await expect(page.locator(".progress-detail")).toContainText("/s");
  await expect(page.getByRole("button", { name: "安装更新", exact: true })).toBeVisible({ timeout: 8000 });
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await page.getByRole("button", { name: "安装更新", exact: true }).click();
  await expect(page.locator("html")).toHaveAttribute("data-update-installed", "true");
  await expect(page.getByRole("dialog")).toHaveCount(0);
});

test("未知总大小使用不确定进度，减少动态效果时停止循环动画", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto("/?demo=1&updateAvailable=1&updateUnknownSize=1");
  await page.getByRole("button", { name: "关于", exact: true }).click();
  await page.getByRole("button", { name: "检查更新", exact: true }).click();
  await page.getByRole("button", { name: "下载更新", exact: true }).click();
  const progress = page.getByRole("progressbar");
  await expect(progress).toBeVisible();
  expect(await progress.getAttribute("aria-valuenow")).toBeNull();
  await expect(page.locator(".progress-values strong")).toHaveText("");
  expect(await page.locator(".progress-fill").evaluate((element) => getComputedStyle(element).animationName)).toBe("none");
  await expect(page.getByRole("button", { name: "安装更新", exact: true })).toBeVisible({ timeout: 15000 });
});

test("开启自动下载后后台完成，不打断编辑，切换页面保留已下载状态", async ({ page }) => {
  await page.goto("/?demo=1&updateAvailable=1&updateAutoDownload=1");
  await expect(page.getByRole("button", { name: "安装更新", exact: true })).toBeVisible({ timeout: 10000 });
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await page.getByRole("button", { name: "关于", exact: true }).click();
  await expect(page.getByText("已准备好更新", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "编辑器", exact: true }).click();
  await expect(page.getByRole("button", { name: "安装更新", exact: true })).toBeVisible();
});
