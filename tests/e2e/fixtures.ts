import { test as base } from "@playwright/test";
export { expect, type Page } from "@playwright/test";

export const test = base.extend({
  page: async ({ page }, use) => {
    // Demo photos must not depend on a third-party service's latency or availability.
    // Individual image tests register their own routes afterward and take priority.
    await page.route("https://picsum.photos/**", route => route.fulfill({
      contentType: "image/png", path: "static/favicon.png"
    }));
    await page.routeWebSocket(/.*/, socket => socket.close());
    await use(page);
  }
});
