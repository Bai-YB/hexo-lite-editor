import { describe, expect, it } from "vitest";
import { assetKindLabel, nextLightboxIndex, shouldLoadCloudflare } from "./model";

describe("image bed model", () => {
  it("waits for project, config and credential before the first remote request", () => {
    expect(shouldLoadCloudflare({ sessionReady: true, credentialReady: false, credentialConfigured: true, apiUrl: "https://img.example.com" })).toBe(false);
    expect(shouldLoadCloudflare({ sessionReady: true, credentialReady: true, credentialConfigured: true, apiUrl: "https://img.example.com" })).toBe(true);
  });

  it("wraps lightbox navigation", () => {
    expect(nextLightboxIndex(0, -1, 4)).toBe(3);
    expect(nextLightboxIndex(3, 1, 4)).toBe(0);
    expect(nextLightboxIndex(0, 1, 0)).toBe(-1);
  });

  it("keeps file types explicit instead of rendering broken image placeholders", () => {
    expect(assetKindLabel("archive")).toBe("压缩包");
    expect(assetKindLabel("file")).toBe("文件");
    expect(assetKindLabel("folder")).toBe("文件夹");
  });
});
