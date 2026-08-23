import { beforeEach, describe, expect, it, vi } from "vitest";
import { enUS } from "./locales/en-US";
import { zhCN } from "./locales/zh-CN";
import { getResolvedLanguage, setLanguage, t } from "./index";
import { resolveLanguage } from "./language";

function keys(value: unknown, prefix = ""): string[] {
  if (!value || typeof value !== "object") return [prefix];
  return Object.entries(value).flatMap(([key, child]) => keys(child, prefix ? `${prefix}.${key}` : key));
}

describe("i18n", () => {
  beforeEach(() => setLanguage("en-US"));

  it("keeps locale keys identical", () => expect(keys(zhCN).sort()).toEqual(keys(enUS).sort()));
  it("maps Chinese system locales to zh-CN", () => {
    expect(resolveLanguage("system", "zh")).toBe("zh-CN");
    expect(resolveLanguage("system", "zh-Hans-CN")).toBe("zh-CN");
  });
  it("maps English and unknown system locales to en-US", () => {
    expect(resolveLanguage("system", "en-GB")).toBe("en-US");
    expect(resolveLanguage("system", "fr-FR")).toBe("en-US");
  });
  it("lets a manual selection override the system", () => {
    setLanguage("zh-CN");
    expect(getResolvedLanguage()).toBe("zh-CN");
  });
  it("interpolates parameters", () => expect(t("update.availableNotice", { version: "1.0.6" })).toContain("1.0.6"));
  it("throws for an unknown key in development", () => {
    vi.stubEnv("DEV", true);
    expect(() => t("unknown.key" as never)).toThrow(/Unknown translation key/);
    vi.unstubAllEnvs();
  });
});
