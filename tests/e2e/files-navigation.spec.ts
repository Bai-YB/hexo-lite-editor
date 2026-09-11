import { expect, test, type Page } from "@playwright/test";

async function openFiles(page: Page) {
  await page.routeWebSocket(/.*/, socket => socket.close());
  await page.goto("/?demo=1");
  await page.locator(".markdown-editor-host").waitFor();
  await page.keyboard.press("Control+6");
  await expect(page.getByRole("complementary", { name: "项目文件" })).toBeVisible();
}

async function openLinkFile(page: Page) {
  await openFiles(page);
  await page.getByRole("button", { name: "source", exact: true }).click();
  await page.getByRole("button", { name: "source/_data", exact: true }).click();
  await page.getByRole("button", { name: "source/_data/link.yml", exact: true }).click();
  await expect(page.locator(".file-workspace .cm-content")).toContainText("https://hexo.io");
}

async function appendFile(page: Page, value: string) {
  await page.locator(".file-workspace .cm-content").click();
  await page.keyboard.press("Control+End");
  await page.keyboard.insertText(value);
  await expect(page.locator(".file-editor-heading")).toContainText("未保存");
}

test("file tree opens friend links and a post path routes to the writing editor", async ({ page }) => {
  await openLinkFile(page);
  await page.getByRole("button", { name: "source/_posts", exact: true }).click();
  await page.getByRole("button", { name: "source/_posts/盛夏散步.md", exact: true }).click();
  await expect(page.locator(".markdown-editor-host .cm-content")).toContainText("# 盛夏散步");
  await expect(page.locator('[data-article-id="summer"]')).toHaveClass(/active/);
  await expect(page.locator(".nav-item.active")).toContainText("编辑器");
});

test("cancel preserves file edits and save-and-continue persists them before navigation", async ({ page }) => {
  await openLinkFile(page);
  await appendFile(page, "\n# keep friend link changes");
  await page.keyboard.press("Control+5");
  let dialog = page.getByRole("dialog");
  await expect(dialog).toContainText("离开全部文件");
  await dialog.getByRole("button", { name: "取消", exact: true }).click();
  await expect(dialog).toHaveCount(0);
  await expect(page.locator(".file-workspace .cm-content")).toContainText("keep friend link changes");
  await page.keyboard.press("Control+5");
  dialog = page.getByRole("dialog");
  await dialog.getByRole("button", { name: "保存并继续", exact: true }).click();
  await expect(page.getByRole("heading", { name: "插件", exact: true, level: 1 })).toBeVisible();
  // The first switch above verifies Ctrl+6. Use the explicit navigation target
  // for the return so this assertion tests persistence rather than transition timing.
  await page.getByRole("button", { name: "全部文件", exact: true }).click();
  await expect(page.locator(".file-workspace .cm-content")).toContainText("keep friend link changes");
  await expect(page.locator(".file-editor-heading")).toContainText("已保存");
});

test("failed file save keeps the close guard and cancel returns to the edited document", async ({ page }) => {
  await openLinkFile(page);
  await appendFile(page, "\n# close protection");
  await page.evaluate(async () => {
    const path = "/src/platform/browserMock.ts";
    const { browserMock } = await import(path);
    const original = browserMock.saveProjectFile.bind(browserMock);
    (window as unknown as { restoreFileSave: () => void }).restoreFileSave = () => { browserMock.saveProjectFile = original; };
    browserMock.saveProjectFile = async () => { throw new Error("file save fixture failed"); };
  });
  await page.locator(".window-control.close").click();
  const dialog = page.getByRole("dialog");
  await expect(dialog).toContainText("项目文件");
  await dialog.getByRole("button", { name: "保存并退出", exact: true }).click();
  await expect(page.getByText("file save fixture failed").first()).toBeVisible();
  await expect(dialog).toBeVisible();
  await dialog.getByRole("button", { name: "取消", exact: true }).click();
  await expect(dialog).toHaveCount(0);
  await expect(page.locator(".file-workspace .cm-content")).toContainText("close protection");
  await page.evaluate(() => (window as unknown as { restoreFileSave: () => void }).restoreFileSave());
  await page.keyboard.press("Control+s");
  await expect(page.locator(".file-editor-heading")).toContainText("已保存");
});

test("publishing from all files saves its document first and aborts when that save fails", async ({ page }) => {
  await openLinkFile(page);
  await appendFile(page, "\n# publish friend links");
  await page.evaluate(async () => {
    const path = "/src/platform/browserMock.ts";
    const { browserMock } = await import(path);
    const original = browserMock.saveProjectFile.bind(browserMock);
    (window as unknown as { restorePublishFileSave: () => void }).restorePublishFileSave = () => { browserMock.saveProjectFile = original; };
    browserMock.saveProjectFile = async () => { throw new Error("publish file save failed"); };
  });
  await page.keyboard.press("Control+Shift+P");
  await expect(page.getByText("publish file save failed").first()).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.dataset.taskStarts)).toBeUndefined();
  await expect(page.locator(".file-workspace .cm-content")).toContainText("publish friend links");
  await expect(page.locator(".file-editor-heading")).toContainText("未保存");
  await page.evaluate(() => (window as unknown as { restorePublishFileSave: () => void }).restorePublishFileSave());
  await page.keyboard.press("Control+Shift+P");
  await expect.poll(() => page.evaluate(() => document.documentElement.dataset.taskStarts)).toBe("1");
  await expect(page.locator(".file-editor-heading")).toContainText("已保存");
});
