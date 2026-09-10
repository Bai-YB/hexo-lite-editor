import { describe, expect, it } from "vitest";
import { defaultConfig } from "$shared/types/app";
import { mergeSavedDraft, validateSettings } from "./settingsDraft";

describe("settings draft saves", () => {
  it("retains edits made while a save is pending", () => {
    const submitted = structuredClone(defaultConfig);
    const current = structuredClone(submitted);
    current.general.language = "en-US";
    const persisted = structuredClone(submitted);
    persisted.editor.fontSize = 18;
    const merged = mergeSavedDraft(submitted, current, persisted);
    expect(merged.general.language).toBe("en-US");
    expect(merged.editor.fontSize).toBe(18);
  });
  it("saves a connection field while retaining unrelated dirty sections", () => {
    const submitted = structuredClone(defaultConfig);
    submitted.imageBed.cloudflareApiUrl = "https://img.example.com";
    const current = structuredClone(submitted);
    current.general.autoSave = !submitted.general.autoSave;
    const merged = mergeSavedDraft(submitted, current, submitted);
    expect(merged.general.autoSave).toBe(current.general.autoSave);
    expect(merged.imageBed.cloudflareApiUrl).toBe(submitted.imageBed.cloudflareApiUrl);
  });
  it("rejects invalid numbers with a focusable field and section", () => {
    const config = structuredClone(defaultConfig);
    config.editor.fontSize = 40;
    expect(validateSettings(config)).toMatchObject({ section: "editing", field: "fontSize" });
    config.editor.fontSize = 15;
    config.general.autoSaveDelayMs = Number.NaN;
    expect(validateSettings(config)).toMatchObject({ section: "general", field: "autoSaveDelayMs" });
  });
});
