import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.goto("/?demo=1");
  await page.locator(".cm-content").waitFor();
  await page.locator('[data-article-id="summer"]').click();
  await expect(page.locator(".cm-content")).toContainText("# 盛夏散步");
});

test("切文章确认期间暂停自动保存，放弃只保留磁盘内容", async ({ page }) => {
  const editor = page.locator(".cm-content");
  await editor.click();
  await page.keyboard.press("Control+End");
  await page.keyboard.insertText("应被放弃的文字");
  await page.locator('[data-article-id="tauri"]').click();
  const dialog = page.getByRole("dialog", { name: "保存当前文章？" });
  await expect(dialog).toBeVisible();
  await page.waitForTimeout(2300);
  expect(await page.evaluate(() => document.documentElement.dataset.editorSaveCalls)).toBeUndefined();
  await dialog.getByRole("button", { name: "放弃更改", exact: true }).click();
  await expect(editor).toContainText("# Tauri");
  await page.locator('[data-article-id="summer"]').click();
  await expect(editor).not.toContainText("应被放弃的文字");
});

test("新建文件名跟随完整标题且日期使用本机年月日时分", async ({ page }) => {
  await page.getByRole("button", { name: "新建", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "新建文章" });
  await dialog.getByLabel("标题", { exact: true }).pressSequentially("hello-world");
  await expect(dialog.getByLabel("文件名", { exact: true })).toHaveValue("hello-world");
  await dialog.getByLabel("文件名", { exact: true }).fill("custom");
  await dialog.getByLabel("标题", { exact: true }).fill("new-title");
  await expect(dialog.getByLabel("文件名", { exact: true })).toHaveValue("custom");
  const expected = await page.evaluate(() => {
    const date = new Date();
    const pad = (value: number) => String(value).padStart(2, "0");
    return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}`;
  });
  await expect(dialog.getByLabel("日期", { exact: true })).toHaveValue(expected);
});

test("图片读取阶段就阻止换文和新建，并保留后续输入位置", async ({ page }) => {
  const editor = page.locator(".cm-content");
  await editor.click();
  await page.keyboard.press("Control+End");
  await editor.evaluate((element) => {
    const original = File.prototype.arrayBuffer;
    File.prototype.arrayBuffer = async function () {
      await new Promise<void>((resolve) => ((window as unknown as { releaseAuditImage: () => void }).releaseAuditImage = resolve));
      return original.call(this);
    };
    const data = new DataTransfer();
    data.items.add(new File([new Uint8Array([137, 80, 78, 71])], "regression.png", { type: "image/png" }));
    element.dispatchEvent(new ClipboardEvent("paste", { bubbles: true, cancelable: true, clipboardData: data }));
  });
  await page.locator('[data-article-id="tauri"]').click();
  await expect(page.locator(".article-item.active")).toHaveAttribute("data-article-id", "summer");
  await page.getByRole("button", { name: "新建", exact: true }).click();
  await expect(page.getByRole("dialog", { name: "新建文章" })).toHaveCount(0);
  await editor.click();
  await page.keyboard.press("Control+Home");
  await page.keyboard.insertText("前面追加\n");
  await page.evaluate(() => (window as unknown as { releaseAuditImage: () => void }).releaseAuditImage());
  await expect(editor).toContainText("regression.png");
  const content = await editor.innerText();
  expect(content.indexOf("前面追加")).toBeLessThan(content.indexOf("# 盛夏散步"));
  expect(content.indexOf("regression.png")).toBeGreaterThan(content.indexOf("城市很热"));
});

test("读取失败的重试仍请求失败文章", async ({ page }) => {
  await page.evaluate(async () => {
    const modulePath = "/src/platform/tauri.ts";
    const { platform } = await import(modulePath);
    const original = platform.loadDocument;
    const calls: string[] = [];
    (window as unknown as { auditLoads: string[] }).auditLoads = calls;
    platform.loadDocument = async (...args: [string, string, number]) => {
      calls.push(args[1]);
      if (calls.length === 1) throw new Error("audit read failure");
      return original(...args);
    };
  });
  await page.locator('[data-article-id="tauri"]').click();
  await expect(page.getByText(/audit read failure/)).toBeVisible();
  await page.getByRole("button", { name: "重试", exact: true }).click();
  await expect(page.locator(".cm-content")).toContainText("# Tauri");
  expect(await page.evaluate(() => (window as unknown as { auditLoads: string[] }).auditLoads)).toEqual(["tauri", "tauri"]);
});

test("上传替换不进入撤销历史且真实预览先保存当前文档", async ({ page }) => {
  await page.goto("/?demo=1&imageUpload=1");
  await page.locator(".cm-content").waitFor();
  await page.locator('[data-article-id="summer"]').click();
  await page.evaluate(async () => {
    const modulePath = "/src/platform/tauri.ts";
    const { platform } = await import(modulePath);
    const calls: string[] = [];
    (window as unknown as { auditPreviewCalls: string[] }).auditPreviewCalls = calls;
    platform.uploadCachedEditorImage = async () => {
      await new Promise(resolve => setTimeout(resolve, 700));
      return { fileName: "image.png", url: "https://img.example.com/ready.png" };
    };
    const originalSave = platform.saveDocument;
    platform.saveDocument = async (request: { content: string }) => {
      calls.push(`save:${request.content.includes("preview-new-content")}`);
      return originalSave(request);
    };
    platform.openHexoPreviewWebview = async (url: string) => { calls.push(`open:${url}`); };
  });
  const editor = page.locator(".cm-content");
  await editor.click();
  await page.keyboard.press("Control+End");
  await editor.evaluate(element => {
    const data = new DataTransfer();
    data.items.add(new File([new Uint8Array([137, 80, 78, 71])], "photo.png", { type: "image/png" }));
    element.dispatchEvent(new ClipboardEvent("paste", { bubbles: true, cancelable: true, clipboardData: data }));
  });
  await expect.poll(() => page.evaluate(() => document.documentElement.dataset.imageCacheFinalized)).toBe("1");
  await page.keyboard.press("Control+z");
  await expect(editor).not.toContainText("hlex-asset.localhost");
  await page.keyboard.press("Control+y");
  await expect(editor).toContainText("https://img.example.com/ready.png");
  await page.keyboard.insertText("preview-new-content");
  await page.getByRole("button", { name: "真实主题", exact: true }).click();
  await page.getByRole("button", { name: "启动并预览", exact: true }).click();
  await expect.poll(() => page.evaluate(() => (window as unknown as { auditPreviewCalls: string[] }).auditPreviewCalls.some(call => call.startsWith("open:")))).toBe(true);
  const calls = await page.evaluate(() => (window as unknown as { auditPreviewCalls: string[] }).auditPreviewCalls);
  expect(calls.findIndex(call => call === "save:true")).toBeLessThan(calls.findIndex(call => call.startsWith("open:")));
});
