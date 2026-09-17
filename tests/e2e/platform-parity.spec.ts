import { expect, test } from "./fixtures";

test("数字设置在保持焦点时保存，校验失败后可以取消", async ({ page }, testInfo) => {
  await page.goto("/?demo=1");
  await page.getByRole("button", { name: "设置", exact: true }).click();
  const nav = page.getByRole("navigation", { name: "设置分类" });
  await nav.getByRole("button", { name: "编辑器", exact: true }).click();
  const font = page.getByLabel("字号", { exact: true });
  const saveKey = testInfo.project.name === "desktop-webkit" ? "Meta+s" : "Control+s";
  await font.fill("22");
  await page.keyboard.press(saveKey);
  await expect(page.locator(".settings-save-state")).toHaveText("已保存");
  await expect(font).toHaveValue("22");
  await font.fill("0");
  await page.keyboard.press(saveKey);
  await expect(page.locator(".settings-page > [role=alert]")).toContainText("字号");
  await expect(font).toBeFocused();
  await page.locator(".settings-sticky-header").getByRole("button", { name: "取消", exact: true }).click();
  await expect(font).toHaveValue("22");
  await page.getByLabel("主题模式", { exact: true }).selectOption("dark");
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  await page.locator(".settings-sticky-header").getByRole("button", { name: "取消", exact: true }).click();
  await expect(page.getByLabel("主题模式", { exact: true })).toHaveValue("system");
});

for (const width of [1360, 1120, 640]) {
  test(`六组设置切换、键盘和吸顶互不遮挡 ${width}px`, async ({ page }, testInfo) => {
    await page.setViewportSize({ width, height: 720 });
    await page.goto("/?demo=1");
    await page.getByRole("button", { name: "设置", exact: true }).click();
    const nav = page.getByRole("navigation", { name: "设置分类" });
    const body = page.locator(".settings-page");
    const sections = ["常规", "编辑器", "图片", "预览与发布", "文件同步", "更新与恢复"];
    for (const section of sections) {
      await body.evaluate(element => { element.scrollTop = element.scrollHeight; });
      await nav.getByRole("button", { name: section, exact: true }).click();
      await expect(page.locator(".settings-content-heading h2")).toHaveText(section);
      await expect.poll(() => body.evaluate(element => element.scrollTop)).toBe(0);
      expect(await body.evaluate(element => element.scrollWidth - element.clientWidth)).toBeLessThanOrEqual(1);
    }
    await nav.getByRole("button", { name: "更新与恢复", exact: true }).press("Home");
    await expect(nav.getByRole("button", { name: "常规", exact: true })).toBeFocused();
    await expect(page.locator(".settings-content-heading h2")).toHaveText("常规");
    await page.getByLabel("自动保存延迟", { exact: true }).fill("2500");
    await body.evaluate(element => { element.scrollTop = 200; });
    const header = await page.locator(".settings-sticky-header").boundingBox();
    const navBox = await nav.boundingBox();
    if (width > 660) expect(navBox!.y).toBeGreaterThanOrEqual(header!.y + header!.height - 1);
    await page.screenshot({ path: `output/playwright/1063-parity/${testInfo.project.name}/settings-${width}.png` });
  });
}

test("真实编辑输入的 HTML 排版、折叠与代码在两种引擎一致", async ({ page }, testInfo) => {
  await page.goto("/?demo=1");
  await page.locator('[data-article-id="summer"]').click();
  const editor = page.locator(".cm-content");
  await editor.fill('<style>.html-card{padding:18px;background:linear-gradient(135deg,#667eea,#8058b5);color:white;border-radius:14px}.html-card:hover{transform:translateY(-2px)}body{display:none}</style>\n' +
    '<div class="html-card">HTML 样式卡片</div>\n\n' +
    '<div align="center"><font color="red" size="5">HTML 标题</font></div>\n\n' +
    '<table><tr><td align="right">合计</td></tr></table>\n\n' +
    '<details><summary>展开正文</summary><p>折叠内容</p></details>\n\n' +
    '```html\n<div onclick="alert(1)">源码</div>\n```\n\n<script>window.htmlExecuted = true</script>');
  const preview = page.locator(".markdown-preview");
  await expect(preview.locator(".html-card")).toHaveCSS("border-radius", "14px");
  await expect(preview.locator(".html-card")).toHaveCSS("color", "rgb(255, 255, 255)");
  await expect(preview.locator("div").filter({ hasText: "HTML 标题" })).toHaveCSS("text-align", "center");
  await expect(preview.locator("td")).toHaveCSS("text-align", "right");
  await expect(preview.getByText("折叠内容", { exact: true })).toBeHidden();
  await preview.locator("summary").click();
  await expect(preview.getByText("折叠内容", { exact: true })).toBeVisible();
  await expect(preview.locator("pre")).toContainText('<div onclick="alert(1)">源码</div>');
  await expect(preview.locator("script, iframe, [onclick]")).toHaveCount(0);
  expect(await page.evaluate(() => (window as unknown as { htmlExecuted?: boolean }).htmlExecuted)).toBeUndefined();
  await page.screenshot({ path: `output/playwright/1063-parity/${testInfo.project.name}/html.png` });
});
