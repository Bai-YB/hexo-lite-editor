import type { Page } from "@playwright/test";

// Editor keymaps follow the host platform, just as the shipped native WebView does.
export async function editorBoundary(page: Page, boundary: "start" | "end") {
  const mac = await page.evaluate(() => /Mac/.test(navigator.platform));
  await page.keyboard.press(mac
    ? `Meta+Arrow${boundary === "start" ? "Up" : "Down"}`
    : `Control+${boundary === "start" ? "Home" : "End"}`);
}

export async function editorRedo(page: Page) {
  const mac = await page.evaluate(() => /Mac/.test(navigator.platform));
  await page.keyboard.press(mac ? "Meta+Shift+z" : "Control+y");
}
