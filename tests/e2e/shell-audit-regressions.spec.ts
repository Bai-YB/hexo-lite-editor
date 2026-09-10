import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.goto("/?demo=1");
  await page.locator(".cm-content").waitFor();
});

test("新建弹窗阻挡发布和全局导航快捷键", async ({ page }) => {
  await page.getByRole("button", { name: "新建", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "新建文章" });
  await dialog.getByLabel("标题", { exact: true }).fill("保留输入");
  await page.keyboard.press("Control+Shift+P");
  await page.keyboard.press("Control+3");
  await expect(dialog).toBeVisible();
  await expect(dialog.getByLabel("标题", { exact: true })).toHaveValue("保留输入");
  expect(await page.evaluate(() => document.documentElement.dataset.taskStarts)).toBeUndefined();
});

test("往返切换界面语言保留正文和输入并恢复控件文字", async ({ page }) => {
  const editor = page.locator(".cm-content");
  await editor.click();
  await page.keyboard.press("Control+End");
  await page.keyboard.insertText("\n\n删除 / 保存 / 上传失败\n");
  await page.evaluate(async () => {
    const modulePath = "/src/shared/i18n/index.ts";
    const { setLanguage } = await import(modulePath);
    setLanguage("en-US");
  });
  await expect(page.getByRole("button", { name: "New", exact: true })).toBeVisible();
  await expect(page.locator(".markdown-preview")).toContainText("删除 / 保存 / 上传失败");
  await expect(editor).toContainText("删除 / 保存 / 上传失败");
  await page.keyboard.press("Control+3");
  await expect(page.getByRole("heading", { name: "Startup", exact: true })).toBeVisible();
  await page.evaluate(async () => {
    const modulePath = "/src/shared/i18n/index.ts";
    const { setLanguage } = await import(modulePath);
    setLanguage("zh-CN");
  });
  await expect(page.getByRole("heading", { name: "启动", exact: true })).toBeVisible();
  await expect(page.getByRole("checkbox", { name: "自动保存", exact: true })).toBeVisible();
});
