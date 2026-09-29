import { expect, test } from "./fixtures";

test.use({
  userAgent: "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_0) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Safari/605.1.15",
  viewport: { width: 980, height: 700 }
});

test("小屏文章列表展开时占据独立列，编辑与预览不会被盖住", async ({ page }) => {
  await page.goto("/?demo=1");
  const grid = page.locator(".editor-grid.layout-single");
  await expect(grid).toBeVisible({ timeout: 20_000 });
  await page.getByRole("button", { name: "打开文章列表" }).click();
  const article = grid.locator(".article-pane");
  const writing = grid.locator(".writing-pane");
  await expect(article).toBeVisible();
  await expect(writing).toBeVisible();
  const positions = await page.evaluate(() => {
    const article = document.querySelector(".editor-grid .article-pane")!.getBoundingClientRect();
    const writing = document.querySelector(".editor-grid .writing-pane")!.getBoundingClientRect();
    return { articleRight: article.right, writingLeft: writing.left, writingWidth: writing.width };
  });
  expect(positions.articleRight).toBeLessThanOrEqual(positions.writingLeft + 2);
  expect(positions.writingWidth).toBeGreaterThan(400);

  await page.locator(".compact-view-switch").getByRole("button", { name: "预览" }).click();
  const preview = grid.locator(".preview-pane");
  await expect(preview).toBeVisible();
  await expect(article).toBeVisible();
  const previewLeft = await preview.evaluate((element) => element.getBoundingClientRect().left);
  expect(positions.articleRight).toBeLessThanOrEqual(previewLeft + 2);

  await page.getByRole("button", { name: "关闭文章列表" }).click();
  await expect(article).toBeHidden();
  const expandedWidth = await preview.evaluate((element) => element.getBoundingClientRect().width);
  expect(expandedWidth).toBeGreaterThan(positions.writingWidth);
});

test("Markdown 标题与代码着色可见，右下角更新入口可点击跳转", async ({ page }) => {
  await page.goto("/?demo=1&updateAvailable=1");
  await expect(page.locator(".cm-content")).toBeVisible({ timeout: 20_000 });
  await page.locator(".cm-content").fill("# 一级标题\n## 二级标题\n```js\nconst answer = 42;\n```");
  await expect(page.locator(".cm-content .cm-line").filter({ hasText: "一级标题" }).locator("span").first()).toBeVisible();
  await page.locator(".compact-view-switch").getByRole("button", { name: "预览" }).click();
  const headings = page.locator(".markdown-preview h1, .markdown-preview h2");
  await expect(headings).toHaveCount(2);
  const headingColors = await headings.evaluateAll((elements) => elements.map((element) => getComputedStyle(element).color));
  expect(headingColors[0]).not.toBe(headingColors[1]);
  await expect(page.locator(".markdown-preview pre code .hljs-keyword")).toContainText("const");

  const notice = page.locator(".status-toasts .notice-indicator");
  const openUpdates = notice.getByRole("button", { name: "查看更新" });
  await expect(openUpdates).toBeVisible({ timeout: 15_000 });
  await openUpdates.click();
  await expect(page.getByRole("heading", { name: "更新", exact: true })).toBeVisible();
});

test("右下角任务通知可点击打开详情", async ({ page }) => {
  await page.goto("/?demo=1");
  await expect(page.locator(".cm-content")).toBeVisible({ timeout: 20_000 });
  await page.evaluate(async () => {
    const modulePath = "/src/platform/tauri.ts";
    const { platform } = await import(/* @vite-ignore */ modulePath);
    await platform.startTask("demo-project", "serverStart");
  });
  const taskDetails = page.locator(".status-toasts").getByRole("button", { name: "任务详情" });
  await expect(taskDetails).toBeVisible();
  await taskDetails.click();
  await expect(page.getByRole("dialog", { name: "后台任务" })).toBeVisible();
});
