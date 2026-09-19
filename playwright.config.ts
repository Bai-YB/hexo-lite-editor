import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  retries: process.env.CI ? 2 : 0,
  globalSetup: "./tests/e2e/global-setup.ts",
  testDir: "./tests/e2e",
  timeout: 60_000,
  // The full matrix already executes every case in Chromium and WebKit.
  // A second worker makes two heavyweight browser instances compile and boot
  // the Vite application at once; hosted Windows/macOS runners can then spend
  // 20+ seconds on a blank initial document and fail an unrelated first
  // assertion. Keep local feedback parallel, but make release gates
  // deterministic on the smaller hosted machines.
  workers: process.env.CI ? 1 : 2,
  outputDir: "./output/playwright/results",
  reporter: [["list"], ["html", { outputFolder: "output/playwright/report", open: "never" }]],
  use: {
    baseURL: "http://127.0.0.1:1420/?demo=1",
    trace: "retain-on-failure",
    screenshot: "only-on-failure"
  },
  webServer: {
    command: "pnpm dev --host 127.0.0.1 --port 1420",
    url: "http://127.0.0.1:1420/?demo=1",
    reuseExistingServer: !process.env.CI,
    timeout: 120_000
  },
  projects: [
    {
      name: "desktop-chromium",
      use: {
        ...devices["Desktop Chrome"],
        ...(process.env.CI ? {} : { channel: "msedge" as const }),
        locale: "zh-CN",
        viewport: { width: 1360, height: 860 }
      }
    },
    {
      name: "desktop-webkit",
      timeout: 60_000,
      use: {
        ...devices["Desktop Safari"],
        locale: "zh-CN",
        viewport: { width: 1360, height: 860 }
      }
    }
  ]
});
