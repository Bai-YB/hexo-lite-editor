import { editorBoundary } from "./keyboard";
import { expect, test } from "./fixtures";

test.beforeEach(async ({ page }) => {
  await page.goto("/?demo=1");
  await expect(page.getByRole("button", { name: /Quiet Notes/ })).toBeVisible({ timeout: 20_000 });
});

test("长文章、文章列表和 Markdown 预览可以独立滚动", async ({ page }) => {
  const editorScroller = page.locator(".cm-scroller");
  const articleList = page.locator(".article-list");
  const preview = page.locator(".markdown-preview");
  await expect(editorScroller).toBeVisible({ timeout: 15_000 });
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

test("导航提供六个工作区并保留原有数字快捷键", async ({ page }) => {
  await expect(page.locator(".nav-rail").getByText("发布", { exact: true })).toHaveCount(0);
  await expect(page.locator(".nav-rail .nav-item")).toHaveCount(6);
  await page.keyboard.press("ControlOrMeta+3");
  await expect(page.getByRole("heading", { name: "设置" })).toBeVisible();
  await page.keyboard.press("ControlOrMeta+4");
  await expect(page.getByRole("heading", { name: "关于" })).toBeVisible();
  await page.keyboard.press("ControlOrMeta+5");
  await expect(page.getByRole("heading", { name: "插件", exact: true, level: 1 })).toBeVisible();
  await page.keyboard.press("ControlOrMeta+6");
  await expect(page.getByRole("complementary", { name: "项目文件" })).toBeVisible();
  await page.keyboard.press("ControlOrMeta+2");
  await expect(page.getByRole("heading", { name: "图床", exact: true })).toBeVisible();
  await page.keyboard.press("ControlOrMeta+1");
  await expect(page.locator(".markdown-editor-host")).toBeVisible();
});

test("Ctrl+Shift+P 单次发布并在保存失败时中止", async ({ page }) => {
  await page.evaluate(async () => {
    const modulePath = "/src/platform/tauri.ts";
    const { platform } = await import(/* @vite-ignore */ modulePath);
    const originalStartTask = platform.startTask;
    let releaseStart!: () => void;
    const startGate = new Promise<void>((resolve) => (releaseStart = resolve));
    (window as unknown as { releasePublishStart: () => void }).releasePublishStart = releaseStart;
    platform.startTask = async (...args: Parameters<typeof originalStartTask>) => {
      document.documentElement.dataset.publishStartCalls = String(Number(document.documentElement.dataset.publishStartCalls ?? "0") + 1);
      await startGate;
      return originalStartTask(...args);
    };
  });
  const editor = page.locator(".cm-content");
  await editor.click();
  await page.keyboard.type("追加内容");
  await page.keyboard.press("ControlOrMeta+Shift+P");
  await expect.poll(() => page.evaluate(() => document.documentElement.dataset.publishStartCalls)).toBe("1");
  await page.keyboard.press("ControlOrMeta+Shift+P");
  expect(await page.evaluate(() => document.documentElement.dataset.publishStartCalls)).toBe("1");
  await page.evaluate(() => (window as unknown as { releasePublishStart: () => void }).releasePublishStart());
  await expect.poll(() => page.evaluate(() => document.documentElement.dataset.taskStarts)).toBe("1");

  await page.goto("/?demo=1&saveFail=1");
  await expect(page.getByRole("button", { name: /Quiet Notes/ })).toBeVisible({ timeout: 20_000 });
  await page.locator(".cm-content").click();
  await page.keyboard.type("无法保存的内容");
  await page.keyboard.press("ControlOrMeta+Shift+P");
  await expect(page.getByText("模拟保存失败。")).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.dataset.taskStarts)).toBeUndefined();
});

test("发布保存后新启动的图片上传会在任务启动前中止发布", async ({ page }) => {
  await page.goto("/?demo=1&imageUpload=1");
  await expect(page.getByRole("button", { name: /Quiet Notes/ })).toBeVisible({ timeout: 20_000 });
  await page.evaluate(async () => {
    const modulePath = "/src/platform/tauri.ts";
    const { platform } = await import(/* @vite-ignore */ modulePath);
    const originalListArticles = platform.listArticles;
    let releaseArticleList!: () => void;
    const articleListGate = new Promise<void>((resolve) => (releaseArticleList = resolve));
    let blockNextArticleList = false;
    (window as unknown as {
      armPublishArticleList: () => void;
      releasePublishArticleList: () => void;
    }).armPublishArticleList = () => (blockNextArticleList = true);
    (window as unknown as { releasePublishArticleList: () => void }).releasePublishArticleList = releaseArticleList;
    platform.listArticles = async (...args: Parameters<typeof originalListArticles>) => {
      if (blockNextArticleList) {
        blockNextArticleList = false;
        document.documentElement.dataset.publishReachedArticleList = "1";
        await articleListGate;
      }
      return originalListArticles(...args);
    };

    let releaseUpload!: () => void;
    const uploadGate = new Promise<void>((resolve) => (releaseUpload = resolve));
    (window as unknown as { releaseRaceUpload: () => void }).releaseRaceUpload = releaseUpload;
    platform.uploadCachedEditorImage = async (_projectId: string, _sessionGeneration: number, _articleId: string, uploadId: string) => {
      document.documentElement.dataset.raceUploadStarted = "1";
      await uploadGate;
      return { fileName: "race.png", uploadId, url: "https://img.example.com/blog/race-ready.png" };
    };
  });

  // Arm and enter the publish path in the same browser task so the editor's
  // auto-save cannot consume the dirty change before publishing starts.
  await page.locator(".cm-editor").evaluate(async (node) => {
    (window as unknown as { armPublishArticleList: () => void }).armPublishArticleList();
    const modulePath = "/node_modules/@codemirror/view/dist/index.js";
    const { EditorView } = await import(/* @vite-ignore */ modulePath);
    const view = EditorView.findFromDOM(node);
    view.dispatch({ changes: { from: view.state.doc.length, insert: "\n发布竞态保存" } });
    window.dispatchEvent(new KeyboardEvent("keydown", {
      key: "p",
      code: "KeyP",
      ctrlKey: true,
      shiftKey: true,
      bubbles: true
    }));
  });
  await expect.poll(() => page.evaluate(() => document.documentElement.dataset.publishReachedArticleList)).toBe("1");
  expect(await page.evaluate(() => document.documentElement.dataset.taskStarts)).toBeUndefined();

  await page.locator(".cm-content").evaluate((element) => {
    const data = new DataTransfer();
    data.items.add(new File([new Uint8Array([137, 80, 78, 71])], "race.png", { type: "image/png" }));
    element.dispatchEvent(new ClipboardEvent("paste", {
      bubbles: true,
      cancelable: true,
      clipboardData: data
    }));
  });
  await expect.poll(() => page.evaluate(() => document.documentElement.dataset.raceUploadStarted)).toBe("1");
  await page.evaluate(() => (window as unknown as { releasePublishArticleList: () => void }).releasePublishArticleList());
  await expect(page.getByRole("alert").filter({ hasText: /\d+ 张图片还在上传/ })).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.dataset.taskStarts)).toBeUndefined();

  // Resolve the upload before teardown so no asynchronous editor work leaks
  // into the next test on slower WebKit workers.
  await page.evaluate(() => (window as unknown as { releaseRaceUpload: () => void }).releaseRaceUpload());
  await expect.poll(() => page.evaluate(() => document.documentElement.dataset.imageCacheFinalized)).toBe("1");
});

test("文章右键菜单支持草稿互转和移到回收站", async ({ page }) => {
  const article = page.locator(".article-item").filter({ hasText: "盛夏散步" });
  await article.click({ button: "right" });
  const menu = page.getByRole("menu");
  await expect(menu.getByRole("menuitem", { name: "在文件夹中显示" })).toBeVisible();
  await menu.getByRole("menuitem", { name: "移到草稿" }).click();
  await expect(article).toContainText("草稿");

  await article.press("Shift+F10");
  await menu.getByRole("menuitem", { name: "转为正式文章" }).click();
  await expect(article).toContainText("文章");

  await article.click({ button: "right" });
  await menu.getByRole("menuitem", { name: "移到回收站" }).click();
  const dialog = page.getByRole("dialog", { name: "将文章移到回收站？" });
  await expect(dialog).toContainText("可以找回");
  await dialog.getByRole("button", { name: "移到回收站" }).click();
  await expect(article).toHaveCount(0);
});

test("粘贴图片先保存本地地址，上传后只替换链接并清理缓存", async ({ page }) => {
  await page.goto("/?demo=1&imageUpload=1");
  await expect(page.getByRole("button", { name: /Quiet Notes/ })).toBeVisible({ timeout: 20_000 });
  await page.evaluate(async () => {
    const modulePath = "/src/platform/tauri.ts";
    const { platform } = await import(/* @vite-ignore */ modulePath);
    let releaseUpload!: () => void;
    const uploadGate = new Promise<void>((resolve) => (releaseUpload = resolve));
    (window as unknown as { releaseImageUpload: () => void }).releaseImageUpload = releaseUpload;
    platform.uploadCachedEditorImage = async (_projectId: string, _sessionGeneration: number, _articleId: string, uploadId: string) => {
      document.documentElement.dataset.imageUploadStarted = "1";
      await uploadGate;
      return { fileName: "image.png", uploadId, url: "https://img.example.com/blog/$asset-ready.png" };
    };
  });
  const editor = page.locator(".cm-content");
  await editor.click();
  await editorBoundary(page, "end");
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
  await expect.poll(() => page.evaluate(() => document.documentElement.dataset.imageUploadStarted)).toBe("1");
  const localUrl = "http://hlex-asset.localhost/0f5845c7-a9d8-40e9-97af-f770331f5000";
  await editorBoundary(page, "end");
  for (let index = 0; index < localUrl.length + 3; index += 1) {
    await page.keyboard.press("ArrowLeft");
  }
  for (let index = 0; index < "old.png".length; index += 1) {
    await page.keyboard.press("Shift+ArrowLeft");
  }
  await page.keyboard.type("用户描述");
  await page.keyboard.press("ControlOrMeta+Shift+P");
  expect(await page.evaluate(() => document.documentElement.dataset.taskStarts)).toBeUndefined();

  await page.evaluate(() => (window as unknown as { releaseImageUpload: () => void }).releaseImageUpload());
  await expect(editor).toContainText("![用户描述](https://img.example.com/blog/$asset-ready.png)", { timeout: 15_000 });
  await expect.poll(() => page.evaluate(() => document.documentElement.dataset.imageCacheFinalized)).toBe("1");
  expect(await page.evaluate(() => Number(document.documentElement.dataset.editorSaveCalls ?? "0"))).toBeGreaterThanOrEqual(2);
  await page.keyboard.press("ControlOrMeta+Shift+P");
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
  await expect(page.getByRole("menu")).toHaveCount(0);
  await expect(archive.locator(".asset-more")).toBeFocused();
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

test("保留快速预览并提供受限真实主题预览入口", async ({ page }) => {
  await expect(page.getByRole("button", { name: "即时预览", exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "主题预览", exact: true })).toBeVisible();
  await expect(page.locator("iframe.theme-preview-frame")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "在浏览器中打开" })).toBeVisible();
  await expect(page.locator(".markdown-preview img")).toHaveAttribute("src", /^https?:\/\//);
  await expect(page.locator("html")).not.toHaveAttribute("data-image-resolve-calls", /[1-9]/);
});

test("隐藏即时预览后编辑器占满文章列表之外的剩余空间", async ({ page }) => {
  const grid = page.locator(".editor-grid");
  const writingPane = page.locator(".writing-pane");
  const widthBefore = await writingPane.evaluate((element) => element.getBoundingClientRect().width);
  await page.getByRole("button", { name: "高级操作" }).click();
  await page.locator(".advanced-menu").getByRole("button", { name: "隐藏即时预览" }).click();
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
  await page.getByRole("button", { name: /文件同步/ }).click();
  const panel = page.locator(".settings-content-panel");
  await expect(panel.getByRole("heading", { name: "同步规划", level: 3 })).toBeVisible();
  await expect(panel.getByRole("radio", { name: /^GitHub/ })).toHaveAttribute("aria-checked", "true");
  await expect(panel.getByRole("radio", { name: /^WebDAV/ })).toBeVisible();
  await expect(panel.getByRole("heading", { name: "本次变化", level: 3 })).toHaveCount(0);
  await expect(panel.getByText(/我同意把草稿、配置和主题传到公开仓库/)).toBeVisible();
  const enable = panel.getByRole("button", { name: "合并并开始同步" });
  await expect(enable).toBeDisabled();
  await panel.getByRole("button", { name: "检查连接与差异" }).click();
  await expect(panel.getByText("启用预检")).toBeVisible();
  await expect(panel.getByText(/本地 12 个文件/)).toBeVisible();
  await panel.locator(".sync-warning input").check();
  await expect(enable).toBeEnabled();
  await enable.click();
  await expect(panel.getByRole("button", { name: "立即同步" })).toBeVisible();
});

test("WebDAV 真实测试通过后启用且配置表单始终可编辑", async ({ page }) => {
  await page.goto("/?demo=1");
  await page.getByRole("button", { name: "设置" }).click();
  await page.getByRole("button", { name: /文件同步/ }).click();
  const panel = page.locator(".settings-content-panel");
  await panel.getByRole("radio", { name: /^WebDAV/ }).click();
  await panel.getByLabel("WebDAV 服务器地址").fill("https://dav.example.com/remote.php/dav/files/blogger");
  await panel.getByLabel("WebDAV 云端目录").fill("hexo/my-blog");
  await panel.getByLabel("WebDAV 用户名").fill("blogger");
  await panel.getByLabel("WebDAV 密码").fill("app-password");
  await panel.getByRole("button", { name: "保存并测试连接" }).click();
  await expect(panel.getByText("连接测试通过")).toBeVisible();
  await expect(panel.getByLabel("WebDAV 密码")).toHaveValue("");
  await expect(panel.getByText(/hexo\/my-blog/)).toBeVisible();
  await panel.getByRole("button", { name: "合并并开始同步" }).click();
  await expect(panel.getByRole("button", { name: "立即同步" })).toBeVisible();
  await panel.locator(".sync-connection-details > summary").click();
  await expect(panel.getByLabel("WebDAV 服务器地址")).toBeVisible();
  await expect(panel.getByLabel("WebDAV 云端目录")).toBeVisible();
  await expect(panel.getByLabel("WebDAV 用户名")).toHaveValue("blogger");
  await expect(panel.getByLabel("WebDAV 密码")).toBeVisible();
  await expect(panel.getByRole("button", { name: "立即同步" })).toBeVisible();

  await panel.getByLabel("WebDAV 云端目录").fill("hexo/another-blog");
  await expect(panel.getByText(/连接信息改了/)).toBeVisible();
  await expect(panel.getByRole("button", { name: "应用连接设置" })).toBeDisabled();
  await panel.getByRole("button", { name: "保存并测试连接" }).click();
  await expect(panel.getByRole("button", { name: "应用连接设置" })).toBeEnabled();
});

test("WebDAV 认证失败后保留表单和输入并可直接修正", async ({ page }) => {
  await page.goto("/?demo=1&webdavAuthFail=1");
  await page.getByRole("button", { name: "设置" }).click();
  await page.getByRole("button", { name: /文件同步/ }).click();
  const panel = page.locator(".settings-content-panel");
  await panel.getByRole("radio", { name: /^WebDAV/ }).click();
  await panel.getByLabel("WebDAV 服务器地址").fill("https://dav.example.com/dav");
  await panel.getByLabel("WebDAV 云端目录").fill("hexo/my-blog");
  await panel.getByLabel("WebDAV 用户名").fill("blogger");
  await panel.getByLabel("WebDAV 密码").fill("wrong-password");
  await panel.getByRole("button", { name: "保存并测试连接" }).click();
  await expect(panel.getByRole("alert")).toContainText("WebDAV 认证失败");
  await expect(panel.getByLabel("WebDAV 用户名")).toHaveValue("blogger");
  await expect(panel.getByLabel("WebDAV 密码")).toHaveValue("wrong-password");
  await panel.getByLabel("WebDAV 密码").fill("correct-password");
  await panel.getByRole("button", { name: "保存并测试连接" }).click();
  await expect(panel.getByText("连接测试通过")).toBeVisible();
  await expect(panel.getByLabel("WebDAV 密码")).toHaveValue("");
});

test("多个 deploy 仓库必须由用户明确选择", async ({ page }) => {
  await page.goto("/?demo=1&syncMultiple=1");
  await page.getByRole("button", { name: "设置" }).click();
  await page.getByRole("button", { name: /文件同步/ }).click();
  const repository = page.getByLabel("目标仓库");
  await expect(repository).toHaveValue("");
  await expect(page.getByText("先选仓库，才能预检和启用同步。")).toBeVisible();
  await repository.selectOption("git@github.com:example/quiet-mirror.git");
  await expect(page.getByRole("button", { name: "检查连接与差异" })).toBeVisible();
});

test("内容同步冲突逐文件展示 Markdown 差异与二进制哈希选择", async ({ page }) => {
  await page.goto("/?demo=1&syncConflict=1");
  await page.getByRole("button", { name: "设置" }).click();
  await page.getByRole("button", { name: /文件同步/ }).click();
  const cards = page.locator(".sync-conflict-card");
  await expect(cards).toHaveCount(2);
  await expect(cards.nth(0)).toContainText("source/_posts/welcome.md");
  await cards.nth(0).getByText("查看两端内容").click();
  await expect(cards.nth(0)).toContainText("# 本地标题");
  await expect(cards.nth(0)).toContainText("# 远端标题");
  await expect(cards.nth(1)).toContainText("本地 2048 B / 云端 4096 B");
  await expect(cards.nth(1)).toContainText("local-bin");
  await expect(cards.nth(1)).toContainText("remote-bin");
  await cards.nth(0).getByLabel("云端").check();
  await expect(page.getByRole("button", { name: "提交选择" })).toBeDisabled();
  await cards.nth(1).getByLabel("本地").check();
  await page.getByRole("button", { name: "提交选择" }).click();
  await expect(cards).toHaveCount(0);
  await expect(page.locator(".sync-status.synced")).toBeVisible();
});

test("云端前进时可以确认使用最新云端或用本机覆盖", async ({ page }) => {
  await page.goto("/?demo=1&syncRemoteAhead=1");
  await page.getByRole("button", { name: "设置" }).click();
  await page.getByRole("button", { name: /文件同步/ }).click();
  const panel = page.locator(".settings-content-panel");
  await expect(panel.getByRole("button", { name: "合并云端改动" })).toBeVisible();
  await expect(panel.getByRole("button", { name: "用本机项目覆盖云端" })).toBeHidden();
  await panel.getByText("高级操作", { exact: true }).click();
  await expect(panel.getByRole("button", { name: "用本机项目覆盖云端" })).toBeVisible();
  await panel.getByRole("button", { name: "用本机项目覆盖云端" }).click();
  const dialog = page.getByRole("dialog", { name: "用本机项目覆盖云端？" });
  await expect(dialog).toContainText("云端会更新到和本机一致");
  await dialog.getByRole("button", { name: "确认覆盖" }).click();
  await expect(page.locator(".sync-status.synced")).toBeVisible();
});

test("启动后静默检查更新，不弹阻断式对话框，关于页显示摘要和真实进度", async ({ page }) => {
  await page.goto("/?demo=1&updateAvailable=1");
  await expect(page.getByRole("button", { name: "查看更新", exact: true })).toBeVisible({ timeout: 15000 });
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await page.getByRole("button", { name: "关于" }).click();
  await expect(page.getByRole("heading", { name: "更新", exact: true })).toBeVisible();
  await expect(page.getByText("有新版本可下载")).toBeVisible();
  await page.getByRole("button", { name: "下载更新" }).click();
  await expect(page.getByRole("progressbar")).toBeVisible();
  await expect(page.getByRole("progressbar")).toHaveAttribute("aria-valuenow", /[1-9]/);
  await expect(page.getByText("更新包已验证。点击安装后，应用将自动重启。")).toBeVisible({ timeout: 15000 });
  await expect(page.getByText("本次更新")).toBeVisible();
  await expect(page.getByText("完整更新日志")).toBeVisible();
});

test("维护页不向普通用户显示任务日志或终端输出，关于页保持精简", async ({ page }) => {
  await page.getByRole("button", { name: "设置" }).click();
  await page.getByRole("button", { name: /^更新与恢复/ }).click();
  await expect(page.getByRole("heading", { name: "更新与恢复" })).toBeVisible();
  await expect(page.getByText("任务日志")).toHaveCount(0);
  await expect(page.locator(".diagnostic-log-view")).toHaveCount(0);
  await page.getByRole("button", { name: "关于" }).click();
  await expect(page.getByText("版本 1.0.6.5.2")).toBeVisible();
  await expect(page.getByText("发布目标")).toHaveCount(0);
  await expect(page.getByText("操作系统")).toHaveCount(0);
});

test("设置分类状态持久化，未保存标记和图床来源正确联动", async ({ page }) => {
  await page.getByRole("button", { name: "设置" }).click();
  const editingNav = page.getByRole("navigation", { name: "设置分类" }).getByRole("button", { name: "编辑器", exact: true });
  await editingNav.click();
  await expect(editingNav).toHaveAttribute("aria-current", "page");
  await page.getByRole("button", { name: "关于" }).click();
  await page.getByRole("button", { name: "设置" }).click();
  await expect(page.getByRole("navigation", { name: "设置分类" }).getByRole("button", { name: "编辑器", exact: true })).toHaveAttribute("aria-current", "page");

  await page.getByRole("button", { name: /^图片/ }).click();
  const imageBed = page.locator(".settings-content-panel");
  await expect(imageBed.getByText("图片保存目录")).toHaveCount(0);
  await expect(imageBed.getByText("Markdown 访问前缀")).toHaveCount(0);
  await expect(imageBed.getByText("图床名称")).toHaveCount(0);
  await imageBed.locator("select").selectOption("cloudflare-imgbed");
  await expect(page.locator(".settings-save-state")).toHaveText("未保存");
  await expect(imageBed.getByText("图片保存目录")).toHaveCount(0);
  await expect(imageBed.getByText("Markdown 访问前缀")).toHaveCount(0);
  await expect(imageBed.getByText("图床名称")).toHaveCount(0);

  const cloudflareInputs = imageBed.locator('[data-provider="cloudflare-imgbed"] input');
  await expect(cloudflareInputs).toHaveCount(1);
  await cloudflareInputs.fill("https://img.example.com");
  await imageBed.getByRole("button", { name: /获取/ }).click();
  const tokenDialog = page.getByRole("dialog", { name: "获取 Cloudflare-ImgBed Token" });
  await expect(tokenDialog).toBeVisible();
  await tokenDialog.getByLabel("管理员用户名").fill("admin");
  await tokenDialog.getByLabel("管理员密码").fill("temporary-secret");
  await tokenDialog.getByRole("button", { name: "获取并保存" }).click();
  await expect(tokenDialog).toHaveCount(0);
  await expect(page.getByText("Token 已创建，存进了系统凭据库。")).toBeVisible();
  // Token acquisition persists the connection and the selected provider without a manual save.
  await expect(page.locator(".settings-save-state")).toHaveText("已保存");

  await imageBed.getByRole("button", { name: "测试连接", exact: true }).click();
  await expect(imageBed.getByText("Cloudflare-ImgBed 连接正常。")).toBeVisible();
  await imageBed.getByRole("button", { name: /重新获取|一键获取 Token/ }).click();
  await expect(tokenDialog.getByLabel("管理员密码")).toHaveValue("");
  await tokenDialog.getByRole("button", { name: "取消" }).click();

  await imageBed.getByRole("button", { name: "删除本地 Token" }).click();
  await expect(imageBed.getByText("Token 未配置")).toBeVisible();
  await imageBed.locator("select").selectOption("local");
  await expect(page.locator(".settings-save-state")).toHaveText("已保存");
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

    await page.getByRole("navigation", { name: "设置分类" }).getByRole("button", { name: "编辑器", exact: true }).click();
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

test("窄窗口中设置六组导航清晰，连接控件使用单列", async ({ page }) => {
  await page.setViewportSize({ width: 640, height: 900 });
  await page.getByRole("button", { name: "设置", exact: true }).click();
  const nav = page.getByRole("navigation", { name: "设置分类" });
  await expect(nav.getByRole("button")).toHaveText(["常规", "编辑器", "图片", "预览与发布", "文件同步", "更新与恢复"]);
  await nav.getByRole("button", { name: "文件同步", exact: true }).click();
  const panel = page.locator(".settings-content-panel");
  await expect(panel.getByText("同步规划", { exact: true })).toBeVisible();
  await panel.getByRole("radio", { name: /^WebDAV/ }).click();
  const address = panel.getByLabel("WebDAV 服务器地址");
  await expect(address).toBeVisible();
  const geometry = await address.evaluate((element) => {
    const row = element.closest(".setting-row")!;
    const label = row.querySelector(".setting-copy")!;
    return { row: row.getBoundingClientRect().width, input: element.getBoundingClientRect().width, inputTop: element.getBoundingClientRect().top, labelBottom: label.getBoundingClientRect().bottom, overflow: document.documentElement.scrollWidth > window.innerWidth };
  });
  expect(geometry.inputTop).toBeGreaterThan(geometry.labelBottom);
  expect(geometry.input).toBeGreaterThan(geometry.row - 30);
  expect(geometry.overflow).toBe(false);
});

test("页面过渡配置不超过 200ms 且离场页不拦截点击", async ({ page }) => {
  await page.getByRole("button", { name: "设置" }).click();
  const settingsTransition = page.locator('.page-transition[data-page-key="settings"]');
  await expect(settingsTransition).toBeVisible();
  const durations = await settingsTransition.evaluate((element) => ({
    enter: Number((element as HTMLElement).dataset.enterDurationMs),
    leave: Number((element as HTMLElement).dataset.leaveDurationMs)
  }));
  expect(durations.enter).toBeLessThanOrEqual(200);
  expect(durations.leave).toBeLessThanOrEqual(200);

  await page.getByRole("button", { name: "关于" }).click();
  await expect.poll(() => settingsTransition.evaluateAll((elements) => elements.every((element) =>
    (element as HTMLElement).dataset.transitionState === "leaving"
      && getComputedStyle(element).pointerEvents === "none"
  ))).toBe(true);
  await expect(page.getByRole("heading", { name: "关于" })).toBeVisible();
  await expect(settingsTransition).toHaveCount(0);
});

test("深色模式光标 token 可见并生成双尺寸回归截图", async ({ page }) => {
  await page.evaluate(() => { document.documentElement.dataset.theme = "light"; });
  await page.screenshot({ path: "output/playwright/editor-1360x860-light.png", fullPage: true });
  await page.setViewportSize({ width: 1120, height: 720 });
  await page.screenshot({ path: "output/playwright/editor-1120x720-light.png", fullPage: true });
  await page.setViewportSize({ width: 1360, height: 860 });
  await page.evaluate(() => { document.documentElement.dataset.theme = "dark"; });
  const caret = await page.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue("--editor-caret").trim());
  expect(caret).toBe("#8cc5f4");
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
  await expect(page.getByRole("heading", { name: "选择一个 Hexo 博客目录就能开始写。" })).toBeVisible({ timeout: 20_000 });
  await captureModes("welcome");
});

test("界面语言行和导航在双向切换时立即更新", async ({ page }) => {
  await page.getByRole("button", { name: "设置" }).click();
  await page.getByLabel("界面语言").selectOption("en-US");
  await expect(page.getByLabel("Interface language")).toHaveValue("en-US");
  await expect(page.locator(".nav-item").filter({ hasText: "Images" })).toBeVisible();
  await page.getByLabel("Interface language").selectOption("zh-CN");
  await expect(page.getByLabel("界面语言")).toHaveValue("zh-CN");
  await expect(page.locator(".nav-item").filter({ hasText: "图床" })).toBeVisible();
});

test("英文模式覆盖编辑器、图床、设置和插件管理 UI", async ({ page }) => {
  await page.goto("/?demo=1&plugin=1");
  await page.getByRole("button", { name: "设置" }).click();
  await page.getByLabel("界面语言").selectOption("en-US");
  await expect(page.getByText("Startup", { exact: true })).toBeVisible();
  await expect(page.locator(".settings-save-state")).toHaveText("Saved");
  await page.locator(".nav-item").filter({ hasText: "Plugins" }).click();
  await expect(page.getByRole("heading", { name: "Plugins", exact: true, level: 1 })).toBeVisible();
  await expect(page.getByRole("button", { name: "Use as image host" })).toBeVisible();
  await page.locator(".nav-item").filter({ hasText: "Images" }).click();
  await expect(page.getByRole("heading", { name: "Images", exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "Import", exact: true })).toBeVisible();
  await page.locator(".nav-rail").getByRole("button", { name: "Editor", exact: true }).click();
  await expect(page.getByRole("button", { name: "Quick preview", exact: true })).toBeVisible();
});
