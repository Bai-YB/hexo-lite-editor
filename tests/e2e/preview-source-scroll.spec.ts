import { expect, test, type Page } from "@playwright/test";

const blocks = [
  "---", "title: Source scroll fixture", "---", "# Source scroll fixture", "",
  '<img src="https://scroll-fixture.test/tall.png" width="320" height="1000" alt="tall fixture">', "",
  ...Array.from({ length: 12 }, (_, index) => [
    `## Section ${index + 1}`, "",
    ("This paragraph wraps several times at the width of either pane. ").repeat(index % 3 + 2), "",
    "```js", ...Array.from({ length: index % 4 + 2 }, (_item, line) => `const row${line} = ${line};`), "```", ""
  ]).flat()
];
const source = blocks.join("\n");
const sectionLine = (section: number) => blocks.indexOf(`## Section ${section}`) + 1;

async function prepare(page: Page) {
  // Concurrent development must not reload an in-progress interaction fixture.
  await page.routeWebSocket(/.*/, socket => socket.close());
  await page.route("https://scroll-fixture.test/tall.png", (route) => route.fulfill({
    contentType: "image/svg+xml", body: '<svg xmlns="http://www.w3.org/2000/svg" width="320" height="1000"><rect width="320" height="1000" fill="#397d74"/></svg>'
  }));
  await page.goto("/?demo=1");
  await page.locator(".cm-content").waitFor();
  await page.locator('[data-article-id="summer"]').click();
  await expect(page.locator(".cm-content")).toContainText("# 盛夏散步");
  await page.locator(".cm-content").click();
  await page.keyboard.press("Control+A");
  await page.keyboard.insertText(source);
  await expect(page.locator(".markdown-preview h2")).toHaveCount(12);
}

async function editorPosition(page: Page) {
  return page.locator(".cm-editor").evaluate(async (node) => {
    const path = "/node_modules/@codemirror/view/dist/index.js";
    const { EditorView } = await import(path);
    const view = EditorView.findFromDOM(node);
    const y = Math.max(0, view.scrollDOM.scrollTop + 16 - view.documentPadding.top);
    const block = view.lineBlockAtHeight(y);
    return { line: view.state.doc.lineAt(block.from).number + Math.max(0, (y - block.top) / Math.max(1, block.height)), top: view.scrollDOM.scrollTop };
  });
}

async function scrollEditorToSection(page: Page, section: number) {
  await page.locator(".cm-editor").evaluate(async (node, line) => {
    const path = "/node_modules/@codemirror/view/dist/index.js";
    const { EditorView } = await import(path);
    const view = EditorView.findFromDOM(node);
    view.scrollDOM.dispatchEvent(new WheelEvent("wheel", { bubbles: true }));
    const pos = view.state.doc.line(line).from;
    view.dispatch({ effects: EditorView.scrollIntoView(pos, { y: "start", yMargin: 16 }) });
    for (let attempt = 0; attempt < 3; attempt += 1) {
      await new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
      view.lineBlockAtHeight(0);
      const block = view.lineBlockAt(pos);
      view.scrollDOM.scrollTop = view.documentPadding.top + block.top - 16;
    }
  }, sectionLine(section));
}

async function previewOffset(page: Page, section: number) {
  return page.locator(`.markdown-preview h2[data-source-line="${sectionLine(section)}"]`).evaluate((element) =>
    element.getBoundingClientRect().top - element.closest(".markdown-preview")!.getBoundingClientRect().top);
}

test("editor and preview keep the same section with tall images, wrapped paragraphs and code blocks", async ({ page }) => {
  await prepare(page);
  await scrollEditorToSection(page, 7);
  await expect.poll(async () => Math.abs(await previewOffset(page, 7) - 16)).toBeLessThan(2);
  const before = await editorPosition(page);
  await page.waitForTimeout(400);
  expect(await editorPosition(page)).toEqual(before);

  // A late image dimension change above the current paragraph must preserve it.
  await page.locator('.markdown-preview img[alt="tall fixture"]').evaluate((image) => {
    image.style.height = "1800px";
  });
  await expect.poll(async () => Math.abs(await previewOffset(page, 7) - 16)).toBeLessThan(2);
  expect((await editorPosition(page)).line).toBeCloseTo(sectionLine(7), 1);
});

test("scrolling preview moves the editor to the same source block without feedback, and unlink disables it", async ({ page }) => {
  await prepare(page);
  await page.locator(".markdown-preview").evaluate((node, line) => {
    node.dispatchEvent(new WheelEvent("wheel", { bubbles: true }));
    const heading = node.querySelector<HTMLElement>(`[data-source-line="${line}"]`)!;
    node.scrollTop += heading.getBoundingClientRect().top - node.getBoundingClientRect().top - 16;
  }, sectionLine(5));
  await expect.poll(async () => (await editorPosition(page)).line).toBeCloseTo(sectionLine(5), 1);
  const before = await editorPosition(page);
  await page.waitForTimeout(400);
  expect(await editorPosition(page)).toEqual(before);

  await page.getByTitle("关闭编辑器与预览同步滚动").click();
  await page.locator(".markdown-preview").evaluate((node) => { node.scrollTop = 0; });
  await page.waitForTimeout(150);
  expect(await editorPosition(page)).toEqual(before);
  await page.getByTitle("开启编辑器与预览同步滚动").click();
  await expect.poll(async () => Math.abs(await previewOffset(page, 5) - 16)).toBeLessThan(2);
});
