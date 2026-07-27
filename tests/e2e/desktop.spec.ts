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

test("粘贴图片先保存本地地址，上传后只替换链接并清理缓存", async ({ page }) => {
  await page.goto("/?demo=1&imageUpload=1");
  await expect(page.getByRole("button", { name: /Quiet Notes/ })).toBeVisible({ timeout: 20_000 });
  const editor = page.locator(".cm-content");
  await editor.click();
  await page.keyboard.press("Control+End");
  await editor.evaluate((element) => {
    const data = new DataTransfer();
    data.items.add(new File([new Uint8Array([137, 80, 78, 71])], "old.png", { type: "image/png" }));
    element.dispatchEvent(new ClipboardEvent("paste", {
      bubbles: true,
      cancelable: true,
      clipboardData: data
    }));
  });

  await expect(editor).toContainText("old.png");
  const localUrl = "http://hlex-asset.localhost/0f5845c7-a9d8-40e9-97af-f770331f5000";
  await page.keyboard.press("Control+End");
  for (let index = 0; index < localUrl.length + 3; index += 1) {
    await page.keyboard.press("ArrowLeft");
  }
  for (let index = 0; index < "old.png".length; index += 1) {
    await page.keyboard.press("Shift+ArrowLeft");
  }
  await page.keyboard.type("用户描述");
  await page.keyboard.press("Control+Shift+P");
  expect(await page.evaluate(() => document.documentElement.dataset.taskStarts)).toBeUndefined();

  await expect(editor).toContainText("![用户描述](https://img.example.com/blog/$asset-ready.png)", { timeout: 15_000 });
  await expect.poll(() => page.evaluate(() => document.documentElement.dataset.imageCacheFinalized)).toBe("1");
  expect(await page.evaluate(() => Number(document.documentElement.dataset.editorSaveCalls ?? "0"))).toBeGreaterThanOrEqual(2);
  await page.keyboard.press("Control+Shift+P");
  await expect.poll(() => page.evaluate(() => document.documentElement.dataset.taskStarts)).toBe("1");
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
  await archive.locator(".asset-primary").press("Shift+F10");
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
  await image.locator(".asset-primary").focus();
  await image.locator(".asset-primary").press("Enter");
  await expect(page.getByRole("dialog", { name: /查看 remote-photo-2.jpg/ })).toBeVisible();
  await page.keyboard.press("Escape");
});

test("只保留即时预览和系统浏览器预览入口", async ({ page }) => {
  await expect(page.getByText("即时预览", { exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "主题预览" })).toHaveCount(0);
  await expect(page.locator("iframe.theme-preview-frame")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "浏览器预览" })).toBeVisible();
  await expect(page.locator(".markdown-preview img")).toHaveAttribute("src", /^https?:\/\//);
  await expect(page.locator("html")).not.toHaveAttribute("data-image-resolve-calls", /[1-9]/);
});

test("隐藏即时预览后编辑器占满文章列表之外的剩余空间", async ({ page }) => {
  const grid = page.locator(".editor-grid");
  const writingPane = page.locator(".writing-pane");
  const widthBefore = await writingPane.evaluate((element) => element.getBoundingClientRect().width);
  await page.getByRole("button", { name: "高级操作" }).click();
  await page.getByRole("button", { name: "隐藏即时预览" }).click();
  await expect(page.locator(".preview-pane")).toHaveCount(0);
  await expect(page.getByRole("separator", { name: "调整编辑与预览比例" })).toHaveCount(0);
  await expect(grid).toHaveClass(/preview-hidden/);
  const bounds = await Promise.all([
    grid.evaluate((element) => element.getBoundingClientRect().toJSON()),
    writingPane.evaluate((element) => element.getBoundingClientRect().toJSON())
  ]);
  expect(Math.abs((bounds[1].x + bounds[1].width) - (bounds[0].x + bounds[0].width))).toBeLessThan(2);
  expect(bounds[1].width).toBeGreaterThan(widthBefore + 100);
});

test("空图片响应显示保留尺寸的错误框且文章封面不回退默认图", async ({ page }) => {
  await page.goto("/?demo=1&imageFail=1");
  const placeholder = page.locator(".markdown-preview .preview-image-error").first();
  await expect(placeholder).toBeVisible({ timeout: 20_000 });
  await expect(placeholder).toContainText("图片不可用");
  await expect(placeholder).toContainText("图片返回为空");
  await expect(placeholder).toContainText("__empty-image");
  await expect(placeholder).toHaveCSS("width", "320px");
  await expect(placeholder).toHaveCSS("height", "180px");
  await placeholder.getByRole("button", { name: "重新加载" }).click();
  await expect(page.locator(".markdown-preview .preview-image-error").first()).toBeVisible();
  const article = page.locator(".article-item").filter({ hasText: "欢迎使用 Hexo Lite Editor" });
  await expect(article.locator(".article-cover.image-error")).toBeVisible();
  await expect(article.locator(".article-cover.placeholder")).toHaveCount(0);
  await article.evaluate((element) => {
    const observer = new MutationObserver(() => {
      if (element.querySelector("img.article-cover")) document.documentElement.dataset.coverFlash = "1";
    });
    observer.observe(element, { childList: true, subtree: true });
  });
  await page.locator(".article-item").filter({ hasText: "盛夏散步" }).click();
  await article.click();
  await expect(article.locator(".article-cover.image-error")).toBeVisible();
  await expect(article.locator("img.article-cover")).toHaveCount(0);
  await page.waitForTimeout(250);
  await expect(page.locator("html")).not.toHaveAttribute("data-cover-flash", "1");
});

test("有效的 404 图片体直接显示而不生成错误占位", async ({ page }) => {
  await page.route("https://picsum.photos/seed/quiet-desk/480/320", (route) => route.fulfill({
    status: 404,
    contentType: "image/gif",
    path: "static/favicon.png"
  }));
  await page.goto("/?demo=1");
  const image = page.locator(".markdown-preview img").first();
  await expect(image).toBeVisible({ timeout: 20_000 });
  await expect.poll(() => image.evaluate((element) => (element as HTMLImageElement).naturalWidth)).toBeGreaterThan(0);
  await expect(page.locator(".markdown-preview .preview-image-error")).toHaveCount(0);
  await expect(image).toHaveAttribute("src", "https://picsum.photos/seed/quiet-desk/480/320");
});

test("远程图片不触发后端解析且切换文章不残留旧图", async ({ page }) => {
  await page.goto("/?demo=1&imageDelay=1");
  await page.locator(".article-item").filter({ hasText: "盛夏散步" }).click();
  await expect(page.locator(".markdown-preview")).toContainText("盛夏散步");
  await page.waitForTimeout(500);
  await expect(page.locator(".markdown-preview img")).toHaveCount(0);
  await expect(page.locator(".markdown-preview .preview-image-error")).toHaveCount(0);
  expect(Number(await page.evaluate(() => document.documentElement.dataset.imageResolveCalls ?? "0"))).toBe(0);
});

test("内容同步向导要求公开仓库确认并展示首次同步预检", async ({ page }) => {
  await page.goto("/?demo=1&syncPublic=1");
  await page.getByRole("button", { name: "设置" }).click();
  await page.getByRole("button", { name: /内容同步/ }).click();
  const panel = page.locator(".settings-content-panel");
  await expect(panel.getByRole("heading", { name: "内容同步", level: 3 })).toBeVisible();
  await expect(panel.getByText(/内容分支会继承仓库可见性/)).toBeVisible();
  const enable = panel.getByRole("button", { name: "确认启用" });
  await expect(enable).toBeDisabled();
  await panel.getByRole("button", { name: "预检" }).click();
  await expect(panel.getByText("启用预检")).toBeVisible();
  await expect(panel.getByText(/本地 12 个文件/)).toBeVisible();
  await panel.locator(".sync-warning input").check();
  await expect(enable).toBeEnabled();
  await enable.click();
  await expect(panel.getByRole("button", { name: "上传本地内容" })).toBeVisible();
});

test("WebDAV 真实测试通过后启用且配置表单始终可编辑", async ({ page }) => {
  await page.goto("/?demo=1");
  await page.getByRole("button", { name: "设置" }).click();
  await page.getByRole("button", { name: /内容同步/ }).click();
  const panel = page.locator(".settings-content-panel");
  await panel.getByLabel("同步方式").selectOption("webdav");
  await panel.getByLabel("WebDAV 服务器地址").fill("https://dav.example.com/remote.php/dav/files/blogger");
  await panel.getByLabel("WebDAV 远端目录").fill("hexo/my-blog");
  await panel.getByLabel("WebDAV 用户名").fill("blogger");
  await panel.getByLabel("WebDAV 密码").fill("app-password");
  await panel.getByRole("button", { name: "保存并测试连接" }).click();
  await expect(panel.getByText("WebDAV 真实连接和预检通过")).toBeVisible();
  await expect(panel.getByLabel("WebDAV 密码")).toHaveValue("");
  await expect(panel.getByText(/hexo\/my-blog/)).toBeVisible();
  await panel.getByRole("button", { name: "确认启用 WebDAV" }).click();
  await expect(panel.getByText("当前已应用连接")).toBeVisible();
  await expect(panel.getByLabel("WebDAV 服务器地址")).toBeVisible();
  await expect(panel.getByLabel("WebDAV 远端目录")).toBeVisible();
  await expect(panel.getByLabel("WebDAV 用户名")).toHaveValue("blogger");
  await expect(panel.getByLabel("WebDAV 密码")).toBeVisible();
  await expect(panel.getByRole("button", { name: "上传本地内容" })).toBeVisible();

  await panel.getByLabel("WebDAV 远端目录").fill("hexo/another-blog");
  await expect(panel.getByText(/尚未应用/)).toBeVisible();
  await expect(panel.getByRole("button", { name: "应用连接设置" })).toBeDisabled();
  await panel.getByRole("button", { name: "保存并测试连接" }).click();
  await expect(panel.getByRole("button", { name: "应用连接设置" })).toBeEnabled();
});

test("WebDAV 认证失败后保留表单和输入并可直接修正", async ({ page }) => {
  await page.goto("/?demo=1&webdavAuthFail=1");
  await page.getByRole("button", { name: "设置" }).click();
  await page.getByRole("button", { name: /内容同步/ }).click();
  const panel = page.locator(".settings-content-panel");
  await panel.getByLabel("同步方式").selectOption("webdav");
  await panel.getByLabel("WebDAV 服务器地址").fill("https://dav.example.com/dav");
  await panel.getByLabel("WebDAV 远端目录").fill("hexo/my-blog");
  await panel.getByLabel("WebDAV 用户名").fill("blogger");
  await panel.getByLabel("WebDAV 密码").fill("wrong-password");
  await panel.getByRole("button", { name: "保存并测试连接" }).click();
  await expect(panel.getByRole("alert")).toContainText("WebDAV 认证失败");
  await expect(panel.getByLabel("WebDAV 用户名")).toHaveValue("blogger");
  await expect(panel.getByLabel("WebDAV 密码")).toHaveValue("wrong-password");
  await panel.getByLabel("WebDAV 密码").fill("correct-password");
  await panel.getByRole("button", { name: "保存并测试连接" }).click();
  await expect(panel.getByText("WebDAV 真实连接和预检通过")).toBeVisible();
  await expect(panel.getByLabel("WebDAV 密码")).toHaveValue("");
});

test("多个 deploy 仓库必须由用户明确选择", async ({ page }) => {
  await page.goto("/?demo=1&syncMultiple=1");
  await page.getByRole("button", { name: "设置" }).click();
  await page.getByRole("button", { name: /内容同步/ }).click();
  const repository = page.getByLabel("目标仓库");
  await expect(repository).toHaveValue("");
  await expect(page.getByText("选择目标仓库后才能预检和启用内容同步。")).toBeVisible();
  await repository.selectOption("git@github.com:example/quiet-mirror.git");
  await expect(page.getByRole("button", { name: "预检" })).toBeVisible();
});

test("内容同步冲突逐文件展示 Markdown 差异与二进制哈希选择", async ({ page }) => {
  await page.goto("/?demo=1&syncConflict=1");
  await page.getByRole("button", { name: "设置" }).click();
  await page.getByRole("button", { name: /内容同步/ }).click();
  const cards = page.locator(".sync-conflict-card");
  await expect(cards).toHaveCount(2);
  await expect(cards.nth(0)).toContainText("source/_posts/welcome.md");
  await cards.nth(0).getByText("查看两端内容").click();
  await expect(cards.nth(0)).toContainText("# 本地标题");
  await expect(cards.nth(0)).toContainText("# 远端标题");
  await expect(cards.nth(1)).toContainText("本地 2048 B / 远端 4096 B");
  await expect(cards.nth(1)).toContainText("local-bin");
  await expect(cards.nth(1)).toContainText("remote-bin");
  await cards.nth(0).getByLabel("远端").check();
  await page.getByRole("button", { name: "提交冲突选择" }).click();
  await expect(cards).toHaveCount(0);
  await expect(page.locator(".sync-status.synced")).toBeVisible();
});

test("维护页不向普通用户显示任务日志或终端输出，关于页保持精简", async ({ page }) => {
  await page.getByRole("button", { name: "设置" }).click();
  await page.getByRole("button", { name: /^维护/ }).click();
  await expect(page.getByRole("heading", { name: "更新与恢复" })).toBeVisible();
  await expect(page.getByText("任务日志")).toHaveCount(0);
  await expect(page.locator(".diagnostic-log-view")).toHaveCount(0);
  await page.getByRole("button", { name: "关于" }).click();
  await expect(page.getByText("版本 1.0.5")).toBeVisible();
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
  expect(caret).toBe("#aec4d0");
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
  await expect(page.getByRole("heading", { name: "从博客目录，直接开始写作。" })).toBeVisible({ timeout: 20_000 });
  await captureModes("welcome");
});
