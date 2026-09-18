import { test as base } from "@playwright/test";
export { expect, type Page } from "@playwright/test";

const runningInCI = Boolean((globalThis as typeof globalThis & {
  process?: { env?: { CI?: string } };
}).process?.env?.CI);

export const test = base.extend({
  page: async ({ page }, use) => {
    // Demo photos must not depend on a third-party service's latency or availability.
    // Individual image tests register their own routes afterward and take priority.
    await page.route("https://picsum.photos/**", route => route.fulfill({
      contentType: "image/png", path: "static/favicon.png"
    }));
    // Hosted release runs must keep Vite HMR available while its dependency
    // optimizer settles. Otherwise a requested full reload can leave the page
    // on an invalidated Svelte module graph. Local runs still block reloads so
    // an unrelated edit cannot interrupt an interaction fixture.
    if (!runningInCI) await page.routeWebSocket(/.*/, socket => socket.close());
    await use(page);
  }
});
