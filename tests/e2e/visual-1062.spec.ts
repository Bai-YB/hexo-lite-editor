import { expect, test } from "@playwright/test";
const out = "output/playwright/1062-visual";
for (const viewport of [
  { width: 1360, height: 860, theme: "light" },
  { width: 1120, height: 720, theme: "dark" },
  { width: 640, height: 860, theme: "light" }
] as const) {
  test(`1.0.6.2 main surfaces ${viewport.width}x${viewport.height} ${viewport.theme}`, async ({ page }) => {
    await page.setViewportSize(viewport);
    const capture = async (name: string) => {
      await page.evaluate((theme) => { document.documentElement.dataset.theme = theme; }, viewport.theme);
      await page.locator(".workspace-page").evaluateAll((elements) => elements.forEach(element => element.scrollTop = 0));
      await page.waitForTimeout(180);
      const overflow = await page.evaluate(() => ({ x: document.documentElement.scrollWidth, w: window.innerWidth }));
      expect(overflow.x, `${name} horizontal overflow`).toBeLessThanOrEqual(overflow.w + 1);
      await page.screenshot({ path: `${out}/${name}-${viewport.width}x${viewport.height}-${viewport.theme}.png`, fullPage: true });
    };
    const settings = async (section: string) => {
      await page.getByRole("button", { name: "设置", exact: true }).click();
      await page.getByRole("navigation", { name: "设置分类" }).getByRole("button", { name: section, exact: true }).click();
    };
    await page.goto("/?demo=1");
    await settings("常规");
    await capture("settings-general");
    await settings("文件同步");
    await expect(page.getByText("主题与模块", { exact: true })).toBeVisible();
    await capture("sync-disconnected");
    await page.goto("/?demo=1&syncRemoteAhead=1");
    await settings("文件同步");
    await page.getByRole("button", { name: "合并云端变更", exact: true }).click();
    await expect(page.getByRole("button", { name: "立即同步", exact: true })).toBeVisible();
    await capture("sync-connected");
    await page.goto("/?demo=1&updateAvailable=1");
    await page.getByRole("button", { name: "关于", exact: true }).click();
    await page.getByRole("button", { name: "检查更新", exact: true }).click();
    await page.getByRole("button", { name: "下载更新", exact: true }).click();
    await expect(page.getByText("更新包已验证。点击安装后，应用将自动重启。", { exact: true })).toBeVisible({ timeout: 8000 });
    await capture("about-ready");
  });
}
