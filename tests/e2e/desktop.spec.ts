import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.goto("/?demo=1");
  await expect(page.getByRole("button", { name: /Quiet Notes/ })).toBeVisible({ timeout: 20_000 });
});

test("长文章、文章列表和 Markdown 预览可以独立滚动", async ({ page }) => {
  const editorScroller = page.locator(".cm-scroller");
  const articleList = page.locator(".article-list");
  const preview = page.locator(".markdown-preview");
  await expect(editorScroller).toBeVisible();
  await editorScroller.hover();
  await page.mouse.wheel(0, 2200);
  expect(await editorScroller.evaluate((element) => element.scrollTop)).toBeGreaterThan(0);
  await preview.hover();
  await page.mouse.wheel(0, 1800);
  expect(await preview.evaluate((element) => element.scrollTop)).toBeGreaterThan(0);
  await articleList.hover();
  await page.mouse.wheel(0, 500);
  expect(await articleList.evaluate((element) => element.scrollTop)).toBeGreaterThanOrEqual(0);
  await editorScroller.press("PageDown");
});

test("项目菜单提供最近项目和打开其他博客", async ({ page }) => {
  await page.getByRole("button", { name: /Quiet Notes/ }).click();
  await expect(page.locator(".project-menu-current").getByText("C:\\博客\\quiet-notes", { exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: /打开其他博客/ })).toBeVisible();
});

test("导航不再包含发布页且数字快捷键只对应四个工作区", async ({ page }) => {
  await expect(page.locator(".nav-rail").getByText("发布", { exact: true })).toHaveCount(0);
  await page.keyboard.press("Control+3");
  await expect(page.getByRole("heading", { name: "设置" })).toBeVisible();
  await page.keyboard.press("Control+4");
  await expect(page.getByRole("heading", { name: "关于" })).toBeVisible();
});

test("Ctrl+Shift+P 单次发布并在保存失败时中止", async ({ page }) => {
  const editor = page.locator(".cm-content");
  await editor.click();
  await page.keyboard.type("追加内容");
  await page.keyboard.press("Control+Shift+P");
  await expect.poll(() => page.evaluate(() => document.documentElement.dataset.taskStarts)).toBe("1");
  await page.keyboard.press("Control+Shift+P");
  expect(await page.evaluate(() => document.documentElement.dataset.taskStarts)).toBe("1");

  await page.goto("/?demo=1&saveFail=1");
  await expect(page.getByRole("button", { name: /Quiet Notes/ })).toBeVisible({ timeout: 20_000 });
  await page.locator(".cm-content").click();
  await page.keyboard.type("无法保存的内容");
  await page.keyboard.press("Control+Shift+P");
  await expect(page.getByText("模拟保存失败。")).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.dataset.taskStarts)).toBeUndefined();
});

test("重复点击关闭只显示一个未保存确认框", async ({ page }) => {
  await page.locator(".cm-content").click();
  await page.keyboard.type("准备关闭的未保存内容");
  await page.locator(".window-control.close").evaluate((button: HTMLButtonElement) => {
    button.click();
    button.click();
  });

  const dialog = page.getByRole("dialog", { name: "退出 Hexo Lite Editor？" });
  await expect(dialog).toHaveCount(1);
  await expect(dialog.getByRole("button", { name: "保存并退出" })).toBeVisible();
  await expect(dialog.getByRole("button", { name: "不保存退出" })).toBeVisible();
  await page.waitForTimeout(2200);
  expect(await page.evaluate(() => document.documentElement.dataset.editorSaveCalls)).toBeUndefined();
  await dialog.getByRole("button", { name: "取消" }).click();
  await expect(dialog).toHaveCount(0);
});

test("Cloudflare 资源按目录显示文件夹、压缩包和图片灯箱", async ({ page }) => {
  await page.getByRole("button", { name: "图床" }).click();
  await page.getByRole("button", { name: /本地图片/ }).click();
  await page.getByRole("button", { name: /Cloudflare-ImgBed/ }).click();
  await expect(page.getByText("可以导入 Wake Up 的课程表", { exact: true })).toBeVisible();
  await expect(page.getByText("资料归档.7z", { exact: true })).toBeVisible();
  const archive = page.locator(".asset-item").filter({ hasText: "资料归档.7z" });
  await archive.locator(".asset-more").click();
  await expect(page.getByRole("menu")).toBeVisible();
  await expect(page.getByRole("menuitem", { name: "插入当前文章" })).toHaveCount(0);
  await page.keyboard.press("Escape");
  await archive.press("Shift+F10");
  await expect(page.getByRole("menu")).toBeVisible();
  await page.keyboard.press("Escape");
  await archive.click({ button: "right" });
  await expect(page.getByRole("menu")).toBeVisible();
  await page.keyboard.press("Escape");
  await page.getByText("blog", { exact: true }).click();
  const image = page.locator(".asset-item").filter({ hasText: "remote-photo-2.jpg" });
  await expect(image).toBeVisible();
  await image.dblclick();
  await expect(page.getByRole("dialog", { name: /查看 remote-photo-2.jpg/ })).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog", { name: /查看/ })).toHaveCount(0);
  await image.focus();
  await image.press("Enter");
  await expect(page.getByRole("dialog", { name: /查看 remote-photo-2.jpg/ })).toBeVisible();
  await page.keyboard.press("Escape");
});

test("只保留即时预览和系统浏览器预览入口", async ({ page }) => {
  await expect(page.getByText("即时预览", { exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "主题预览" })).toHaveCount(0);
  await expect(page.locator("iframe.theme-preview-frame")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "浏览器预览" })).toBeVisible();
  await expect(page.locator(".markdown-preview img")).not.toHaveAttribute("src", /^https?:\/\//);
});

test("诊断页主动读取日志，关于页保持精简", async ({ page }) => {
  await page.getByRole("button", { name: "设置" }).click();
  await page.getByRole("button", { name: /诊断与维护/ }).click();
  await expect(page.getByText("目前没有日志")).toBeVisible();
  await page.getByRole("button", { name: "关于" }).click();
  await expect(page.getByText("版本 1.0.3")).toBeVisible();
  await expect(page.getByText("发布目标")).toHaveCount(0);
  await expect(page.getByText("操作系统")).toHaveCount(0);
});

test("设置分类状态持久化，未保存标记和图床来源正确联动", async ({ page }) => {
  await page.getByRole("button", { name: "设置" }).click();
  const editingNav = page.getByRole("button", { name: /编辑体验/ });
  await editingNav.click();
  await expect(editingNav).toHaveAttribute("aria-current", "page");
  await page.getByRole("button", { name: "关于" }).click();
  await page.getByRole("button", { name: "设置" }).click();
  await expect(page.getByRole("button", { name: /编辑体验/ })).toHaveAttribute("aria-current", "page");

  await page.getByRole("button", { name: /图片与图床/ }).click();
  const imageBed = page.locator(".settings-content-panel");
  await expect(imageBed.getByText("图片保存目录")).toBeVisible();
  await expect(imageBed.getByText("图床名称")).toHaveCount(0);
  await imageBed.locator("select").selectOption("cloudflare-imgbed");
  await expect(page.getByRole("button", { name: /图片与图床/ }).locator(".settings-dirty-dot")).toBeVisible();
  await expect(imageBed.getByText("图片保存目录")).toHaveCount(0);
  await expect(imageBed.getByText("图床名称")).toBeVisible();

  const cloudflareInputs = imageBed.locator('[data-provider="cloudflare-imgbed"] input');
  await cloudflareInputs.nth(0).fill("博客图床");
  await cloudflareInputs.nth(1).fill("https://img.example.com");
  await imageBed.getByRole("button", { name: /获取/ }).click();
  const tokenDialog = page.getByRole("dialog", { name: "获取 Cloudflare-ImgBed Token" });
  await expect(tokenDialog).toBeVisible();
  await tokenDialog.getByLabel("管理员用户名").fill("admin");
  await tokenDialog.getByLabel("管理员密码").fill("temporary-secret");
  await tokenDialog.getByRole("button", { name: "获取并保存" }).click();
  await expect(tokenDialog).toHaveCount(0);
  await expect(page.getByText("Token 已创建并保存到系统凭据库。")).toBeVisible();

  await imageBed.getByRole("button", { name: "测试连接" }).click();
  await expect(imageBed.getByText("Cloudflare-ImgBed 连接正常。")).toBeVisible();
  await imageBed.getByRole("button", { name: /重新获取|一键获取 Token/ }).click();
  await expect(tokenDialog.getByLabel("管理员密码")).toHaveValue("");
  await tokenDialog.getByRole("button", { name: "取消" }).click();

  await imageBed.getByRole("button", { name: "删除本地 Token" }).click();
  await expect(imageBed.getByText("Token 未配置")).toBeVisible();
  await imageBed.locator("select").selectOption("local");
  await page.getByRole("button", { name: "保存", exact: true }).click();
});

test("设置分类切换时内容宽度保持稳定", async ({ page }) => {
  await page.getByRole("button", { name: "设置" }).click();
  const settingsPage = page.locator(".settings-page");
  const layout = page.locator(".settings-layout");

  for (const viewport of [{ width: 1360, height: 860 }, { width: 1120, height: 720 }]) {
    await page.setViewportSize(viewport);
    await page.getByRole("button", { name: /常规/ }).click();
    const generalBox = await layout.boundingBox();
    const generalClientWidth = await settingsPage.evaluate((element) => element.clientWidth);

    await page.getByRole("button", { name: /编辑体验/ }).click();
    const editingBox = await layout.boundingBox();
    const editingClientWidth = await settingsPage.evaluate((element) => element.clientWidth);

    expect(generalBox).not.toBeNull();
    expect(editingBox).not.toBeNull();
    expect(editingBox!.x).toBeCloseTo(generalBox!.x, 1);
    expect(editingBox!.width).toBeCloseTo(generalBox!.width, 1);
    expect(editingClientWidth).toBe(generalClientWidth);
  }

  await expect(settingsPage).toHaveCSS("scrollbar-gutter", "stable");
});

test("页面过渡在 200ms 内结束且离场页不拦截点击", async ({ page }) => {
  await page.getByRole("button", { name: "设置" }).click();
  await expect(page.locator('.page-transition[data-page-key="settings"]')).toBeVisible();
  await page.getByRole("button", { name: "关于" }).click();
  await expect(page.getByRole("heading", { name: "关于" })).toBeVisible({ timeout: 200 });
  await expect(page.locator('.page-transition[style*="pointer-events: none"]')).toHaveCount(0, { timeout: 250 });
});

test("深色模式光标 token 可见并生成双尺寸回归截图", async ({ page }) => {
  await page.evaluate(() => { document.documentElement.dataset.theme = "light"; });
  await page.screenshot({ path: "output/playwright/editor-1360x860-light.png", fullPage: true });
  await page.setViewportSize({ width: 1120, height: 720 });
  await page.screenshot({ path: "output/playwright/editor-1120x720-light.png", fullPage: true });
  await page.setViewportSize({ width: 1360, height: 860 });
  await page.evaluate(() => { document.documentElement.dataset.theme = "dark"; });
  const caret = await page.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue("--editor-caret").trim());
  expect(caret).toBe("#f3f7f9");
  await page.screenshot({ path: "output/playwright/editor-1360x860-dark.png", fullPage: true });
  await page.setViewportSize({ width: 1120, height: 720 });
  await expect(page.locator(".editor-toolbar")).toBeVisible();
  await page.screenshot({ path: "output/playwright/editor-1120x720-dark.png", fullPage: true });
});

test("欢迎页、图床、设置和关于生成浅色深色回归截图", async ({ page }) => {
  const captureModes = async (name: string) => {
    for (const mode of ["light", "dark"] as const) {
      await page.evaluate((value) => { document.documentElement.dataset.theme = value; }, mode);
      await page.waitForTimeout(220);
      await page.screenshot({ path: `output/playwright/${name}-1360x860-${mode}.png`, fullPage: true });
    }
  };

  await page.getByRole("button", { name: "图床" }).click();
  await expect(page.getByRole("heading", { name: "图床" })).toBeVisible();
  await captureModes("image-bed");
  await page.getByRole("button", { name: "设置" }).click();
  await expect(page.getByRole("heading", { name: "设置" })).toBeVisible();
  await captureModes("settings");
  await page.getByRole("button", { name: "关于" }).click();
  await expect(page.getByRole("heading", { name: "关于" })).toBeVisible();
  await captureModes("about");

  await page.goto("/?demo=1&welcome=1");
  await expect(page.getByRole("heading", { name: "还没有打开项目" })).toBeVisible({ timeout: 20_000 });
  await captureModes("welcome");
});
