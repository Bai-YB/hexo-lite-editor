import { describe, expect, it } from "vitest";
import { previewStateLabel } from "./previewModel";

describe("preview model", () => {
  it("maps every preview state to quiet status text", () => {
    expect(previewStateLabel("starting")).toBe("启动中");
    expect(previewStateLabel("running")).toBe("运行中");
    expect(previewStateLabel("stopping")).toBe("停止中");
    expect(previewStateLabel("error")).toBe("异常");
    expect(previewStateLabel("stopped")).toBe("已停止");
  });
});
