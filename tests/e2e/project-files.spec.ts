import { test, expect } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.goto("/?demo=1");
  await page.getByRole("navigation", { name: "主导航" }).getByRole("button", { name: /^全部文件/ }).click();
  await expect(page.getByRole("complementary", { name: "项目文件" })).toBeVisible();
});

test("expands the project tree and saves friend links without routing them through article preview", async ({ page }) => {
  await page.getByRole("button", { name: "source", exact: true }).click();
  await page.getByRole("button", { name: "source/_data", exact: true }).click();
  await page.getByRole("button", { name: "source/_data/link.yml", exact: true }).click();
  const content = page.locator(".text-file-editor .cm-content");
  await expect(content).toContainText("https://hexo.io");
  await content.fill("- name: Updated friend\n  link: https://example.com\n");
  await page.getByRole("button", { name: "保存", exact: true }).click();
  await expect(page.locator(".file-editor-heading")).toContainText("已保存");
  await page.getByRole("button", { name: "_config.yml", exact: true }).click();
  await expect(content).toContainText("Quiet Notes");
  await page.getByRole("button", { name: "source/_data/link.yml", exact: true }).click();
  await expect(content).toContainText("Updated friend");
  await expect(page.locator(".markdown-preview")).toHaveCount(0);
});

test("opens deep paths and preserves dirty content when cancelling a file switch", async ({ page }) => {
  await page.getByRole("textbox", { name: "打开文件路径" }).fill("themes/quiet/_config.yml");
  await page.getByRole("textbox", { name: "打开文件路径" }).press("Enter");
  const content = page.locator(".text-file-editor .cm-content");
  await expect(content).toContainText("Home");
  await content.fill("menu:\n  Friends: /links/\n");
  await page.getByRole("button", { name: "_config.yml", exact: true }).click();
  await page.getByRole("dialog").getByRole("button", { name: "取消", exact: true }).click();
  await expect(content).toContainText("Friends");
  await page.getByRole("button", { name: "_config.yml", exact: true }).click();
  await page.getByRole("dialog").getByRole("button", { name: "保存后继续", exact: true }).click();
  await expect(content).toContainText("Quiet Notes");
});

test("keeps binary files read-only with an explicit explanation", async ({ page }) => {
  await page.getByRole("textbox", { name: "打开文件路径" }).fill("source/images/example.png");
  await page.getByRole("textbox", { name: "打开文件路径" }).press("Enter");
  await expect(page.getByText("这是二进制文件，无法作为文本编辑。", { exact: true })).toBeVisible();
  await expect(page.locator(".text-file-editor .cm-content")).toHaveAttribute("contenteditable", "false");
  await expect(page.getByRole("button", { name: "保存", exact: true })).toBeDisabled();
});

test("rejects a stale save and compares disk contents before explicit replacement", async ({ page }) => {
  await page.getByRole("button", { name: "_config.yml", exact: true }).click();
  const content = page.locator(".text-file-editor .cm-content");
  await expect(content).toContainText("Quiet Notes");
  await content.fill("title: Local draft\n");
  await page.evaluate(async () => {
    const path = "/src/platform/browserMock.ts";
    const { browserMock } = await import(/* @vite-ignore */ path);
    const snapshot = await browserMock.loadProjectFile("_config.yml");
    await browserMock.saveProjectFile(snapshot, "title: External change\n");
  });
  await page.getByRole("button", { name: "保存", exact: true }).click();
  await expect(page.getByRole("alert")).toContainText("文件已在外部修改");
  await expect(content).toContainText("Local draft");
  await page.locator(".file-editor-heading").getByRole("button", { name: "比较磁盘版本" }).click();
  await expect(page.getByRole("dialog").getByRole("textbox", { name: "磁盘内容", exact: true })).toHaveValue("title: External change\n");
  await page.getByRole("dialog").getByRole("button", { name: "保留编辑并保存" }).click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(page.locator(".file-editor-heading")).toContainText("已保存");
  await expect(content).toContainText("Local draft");
});
