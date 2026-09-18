import { chromium, expect, type FullConfig } from "@playwright/test";

export default async function warmApplication(config: FullConfig) {
  // Compile the full Vite module graph before timed interaction tests begin.
  const browser = await chromium.launch({ channel: config.projects[0].use.channel });
  try {
    const page = await browser.newPage({ locale: "zh-CN" });
    await page.goto(config.projects[0].use.baseURL!, { timeout: 120_000 });
    await expect(page.getByRole("button", { name: /Quiet Notes/ })).toBeVisible({ timeout: 120_000 });
    // AppShell warms deferred page modules after first paint. Wait until those
    // imports and any Vite dependency-optimizer reload have settled before a
    // timed test opens its own page.
    await page.waitForLoadState("networkidle", { timeout: 120_000 });
  } finally {
    await browser.close();
  }
}
