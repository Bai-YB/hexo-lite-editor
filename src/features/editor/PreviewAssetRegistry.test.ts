import { describe, expect, it } from "vitest";
import { PreviewAssetRegistry, replacePreviewAssetInPlace } from "./PreviewAssetRegistry";

describe("PreviewAssetRegistry", () => {
  it("tracks concurrent uploads independently", () => {
    const registry = new PreviewAssetRegistry();
    registry.register({ uploadId: "a", localPreviewUrl: "hlex-asset://a" });
    registry.register({ uploadId: "b", localPreviewUrl: "hlex-asset://b" });
    registry.markUploading("a"); registry.resolveRemote("b", "https://img/b.png");
    expect(registry.get("a")?.status).toBe("uploading");
    expect(registry.get("b")?.remoteUrl).toBe("https://img/b.png");
  });

  it("keeps the same DOM node and scroll position", () => {
    const container = document.createElement("div");
    container.innerHTML = '<img data-upload-id="a" data-image-state="uploading" src="hlex-asset://a">';
    const before = container.firstElementChild;
    container.scrollTop = 20;
    const result = replacePreviewAssetInPlace(container, "a", "https://img/a.png");
    expect(result).toBe(before);
    expect(result?.src).toBe("https://img/a.png");
    expect(container.scrollTop).toBe(20);
  });
});
