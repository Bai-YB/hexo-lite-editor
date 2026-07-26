import { beforeEach, describe, expect, it, vi } from "vitest";

const invokeMock = vi.hoisted(() => vi.fn());

vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));
vi.mock("@tauri-apps/plugin-clipboard-manager", () => ({ writeText: vi.fn() }));

import { platform } from "./tauri";

describe("WebDAV Tauri IPC contract", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      value: {},
      configurable: true
    });
  });

  it("uses the registered test command and camelCase request payload", async () => {
    invokeMock.mockResolvedValue({});
    const request = {
      projectId: "project-1",
      sessionGeneration: 7,
      endpoint: "https://dav.example.com/root",
      remoteDir: "hexo/blog",
      username: "writer",
      password: "candidate-password"
    };

    await platform.testWebDavContentSync(request);

    expect(invokeMock).toHaveBeenCalledWith("test_webdav_content_sync", { request });
  });

  it("uses the registered update command and exact connection payload", async () => {
    invokeMock.mockResolvedValue({});
    const request = {
      projectId: "project-1",
      sessionGeneration: 7,
      endpoint: "https://dav.example.com/root",
      remoteDir: "hexo/next-blog"
    };

    await platform.updateWebDavContentSync(request);

    expect(invokeMock).toHaveBeenCalledWith("update_webdav_content_sync", { request });
  });
});
