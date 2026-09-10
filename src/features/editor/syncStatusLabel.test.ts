import { describe, expect, it } from "vitest";
import { syncStatusLabel } from "./syncStatusLabel";

describe("sync status label", () => {
  it("shows actionable human labels and the actual conflict count", () => {
    const view = { enabled: true, provider: "github" as const, conflicts: ["a.md", "b.md"] };
    expect(syncStatusLabel({ ...view, status: "conflict" })).toBe("2 个文件需处理");
    expect(syncStatusLabel({ ...view, status: "remoteAhead" })).toBe("云端有更新");
    expect(syncStatusLabel({ ...view, status: "localPending" })).toBe("等待同步");
    expect(syncStatusLabel({ ...view, status: "error" })).toBe("失败待重试");
    expect(syncStatusLabel({ ...view, status: "authRequired" })).toBe("需登录");
    expect(syncStatusLabel({ ...view, status: "synced" })).toBe("已同步");
  });
});
